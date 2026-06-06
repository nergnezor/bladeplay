use std::sync::atomic::{AtomicU64, Ordering};
static ELAPSED_SECS: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
pub extern "C" fn set_elapsed(t: f32) {
    ELAPSED_SECS.store(t.to_bits() as u64, Ordering::Relaxed);
}

fn elapsed() -> f32 {
    f32::from_bits(ELAPSED_SECS.load(Ordering::Relaxed) as u32)
}

// --- Tweakable point-light intensity (hot-reload by saving this file) ---
// Light color is `obj.color * obj.emissive * POINT_LIGHT_INTENSITY`.
// Attenuation in the shader is `1/dist²`, so this number needs to be large
// to be visible at typical scene distances of a few meters.
const POINT_LIGHT_INTENSITY: f32 = 6.0;

// Sun illuminance at scene level (roughly: brdf-weighted irradiance on a unit Lambertian surface).
// The sun PointLight color is premultiplied by dist² so that 1/dist² cancels out,
// producing near-parallel light regardless of distance.
const SUN_ILLUMINANCE: f32 = 3.0;

#[no_mangle]
pub extern "C" fn point_light_intensity() -> f32 {
    POINT_LIGHT_INTENSITY
}

#[no_mangle]
pub extern "C" fn sun_illuminance() -> f32 {
    SUN_ILLUMINANCE
}

#[derive(Clone)]
pub struct Sun {
    pub pos: glam::Vec3,
    pub vel: glam::Vec3,  // vel.z = angular speed (rad/s) for circular orbit
    pub color: glam::Vec3,
}

// Gravitational coupling between suns. Tune for desired orbital period.
// At ~2000 m separation: a ≈ G/r² = 2e7/4e6 = 5 m/s² → T ≈ 2 min.
const G_BODY: f32 = 2.0e7;

#[no_mangle]
pub extern "C" fn make_suns(out: &mut [Sun; 4]) {
    // Positions kept at ~8000 m so illumination looks like parallel sunlight.
    // Velocities chosen so total momentum ≈ 0 and each sun has ~100 m/s speed,
    // giving chaotic ~2-minute orbits as they gravitationally interact.
    // All suns disabled — room is fully enclosed, sun rays are blocked by walls.
    // Re-enable by setting non-zero colors if the scene changes to outdoor.
    *out = [
        Sun { pos: glam::Vec3::new(0.0, -10.0, 1.0), vel: glam::Vec3::ZERO, color: glam::Vec3::ZERO },
        Sun { pos: glam::Vec3::new(0.0, -10.0, 1.0), vel: glam::Vec3::ZERO, color: glam::Vec3::ZERO },
        Sun { pos: glam::Vec3::new(0.0, -10.0, 1.0), vel: glam::Vec3::ZERO, color: glam::Vec3::ZERO },
        Sun { pos: glam::Vec3::new(0.0, -10.0, 1.0), vel: glam::Vec3::ZERO, color: glam::Vec3::ZERO },
    ];
}

#[no_mangle]
pub extern "C" fn step_suns(suns: &mut [Sun; 4], dt: f32) {
    // N-body gravity: accumulate pairwise accelerations then integrate.
    let mut accels = [glam::Vec3::ZERO; 4];
    for i in 0..4 {
        if suns[i].color.length_squared() < 1e-6 { continue; }
        for j in (i + 1)..4 {
            if suns[j].color.length_squared() < 1e-6 { continue; }
            let diff = suns[j].pos - suns[i].pos;
            // Softening at 400 m prevents divergence on close approaches.
            let r2 = diff.length_squared().max(400.0 * 400.0);
            let a_mag = G_BODY / r2;
            let dir = diff / r2.sqrt();
            accels[i] += dir * a_mag;
            accels[j] -= dir * a_mag;
        }
    }
    for i in 0..4 {
        if suns[i].color.length_squared() < 1e-6 { continue; }
        suns[i].vel += accels[i] * dt;
        suns[i].pos += suns[i].vel * dt;
    }
}

// ---------------------------------------------------------------------------
// Declarative scene — edit and save to hot-reload objects in the running app
// ---------------------------------------------------------------------------

/// One object in the scene. `id` is its stable identity across reloads.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ObjectDesc {
    pub id:         u64,
    pub model:      [u8; 32],  // glb filename, null-terminated, relative to data/
    pub pos:        [f32; 3],
    pub scale:      f32,
    pub color:      [f32; 3],  // RGB
    pub emissive:   f32,       // 0 = lit, >0 = glowing
    pub no_gravity: u32,       // 1 = static/kinematic, 0 = falls
}

impl ObjectDesc {
    pub fn model_str(&self) -> &str {
        let end = self.model.iter().position(|&b| b == 0).unwrap_or(32);
        std::str::from_utf8(&self.model[..end]).unwrap_or("sphere.glb")
    }
}

pub fn model(name: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let b = name.as_bytes();
    buf[..b.len().min(31)].copy_from_slice(&b[..b.len().min(31)]);
    buf
}

pub const MAX_SCENE_OBJECTS: usize = 64;

#[repr(C)]
pub struct SceneDesc {
    pub objects: [ObjectDesc; MAX_SCENE_OBJECTS],
    pub count:   u32,
}

impl SceneDesc {
    pub fn new() -> Self {
        Self {
            objects: [ObjectDesc {
                id: 0, model: model("sphere.glb"),
                pos: [0.0; 3], scale: 1.0,
                color: [1.0; 3], emissive: 0.0,
                no_gravity: 0,
            }; MAX_SCENE_OBJECTS],
            count: 0,
        }
    }
    pub fn push(&mut self, obj: ObjectDesc) {
        if (self.count as usize) < MAX_SCENE_OBJECTS {
            self.objects[self.count as usize] = obj;
            self.count += 1;
        }
    }
}

// Reserved IDs: 100=ground, 101-103=sun spheres, 200=room shell
// IDs 1-20: scene objects

/// Returns the desired scene for this frame.
/// Edit freely — objects are added/removed live on save.
#[no_mangle]
pub extern "C" fn scene_objects(out: &mut SceneDesc) {
    *out = SceneDesc::new();

    // Ground plane (y=0)
    out.push(ObjectDesc {
        id: 100, model: model("plane.glb"),
        pos: [0.0, 0.0, 0.0], scale: 1.0,
        color: [0.85, 0.80, 0.75], emissive: 0.0, no_gravity: 1,
    });

    // Room shell — 10×10×10 m box with inward-facing normals.
    // Centered at [0,5,0] so floor=y0, ceiling=y10, walls at x±5, z±5.
    out.push(ObjectDesc {
        id: 200, model: model("room.glb"),
        pos: [0.0, 5.0, 0.0], scale: 10.0,
        color: [0.90, 0.90, 0.90], emissive: 0.0, no_gravity: 1,
    });

    // 5 emissive spheres spread across the room — fewer lights = fewer NEE shadow rays per pixel.
    let lights: &[(u64, [f32; 3], [f32; 3])] = &[
        (1,  [ 0.0, 2.5,  0.0], [1.0, 0.15, 0.10]), // red   — center
        (3,  [-2.0, 4.0, -2.0], [1.0, 0.95, 0.10]), // yellow — left-back
        (5,  [ 2.0, 4.0, -2.0], [0.05, 0.6, 1.00]), // cyan   — right-back
        (7,  [ 3.0, 7.0,  2.0], [0.75, 0.1, 1.00]), // violet — right-front upper
        (10, [-3.0, 7.0,  2.0], [1.00, 0.9, 0.30]), // warm white — left-front upper
    ];
    // particle.glb has radius 0.08 m (256 triangles vs sphere.glb's 2304).
    // scale = 0.3 / 0.08 = 3.75 gives 0.3 m effective radius.
    for &(id, pos, color) in lights {
        out.push(ObjectDesc {
            id, model: model("particle.glb"),
            pos, scale: 3.75, color, emissive: 3.0, no_gravity: 1,
        });
    }

    // Large white sphere on floor (scale=1.0 → radius 1 m, center at y=1).
    out.push(ObjectDesc {
        id: 11, model: model("sphere.glb"),
        pos: [-2.0, 1.0, 1.5], scale: 1.0,
        color: [0.95, 0.95, 0.95], emissive: 2.0, no_gravity: 1,
    });

    // White cube on floor (scale=1.0 → 1×1×1 m, center at y=0.5).
    out.push(ObjectDesc {
        id: 12, model: model("cube.glb"),
        pos: [2.0, 0.5, -1.5], scale: 1.0,
        color: [0.95, 0.95, 0.95], emissive: 0.0, no_gravity: 1,
    });

    // Floating spheres — bob up and down, 2 of 4 are emissive to keep light count low.
    let t = elapsed();
    let floaters: &[(u64, [f32; 3], [f32; 3], f32, f32, f32)] = &[
        // (id, xz_pos, color, base_y, phase, emissive)
        (20, [-1.5, 0.0, -1.0], [1.00, 0.20, 0.05], 3.2, 0.0, 2.5),
        (21, [ 1.5, 0.0, -2.5], [0.15, 0.50, 1.00], 2.0, 1.3, 2.5),
        (22, [ 0.0, 0.0, -3.5], [0.60, 1.00, 0.10], 4.5, 2.6, 2.5),
        (23, [-0.5, 0.0,  0.5], [1.00, 0.80, 0.10], 2.5, 0.9, 2.5),
    ];
    for &(id, xz, color, base_y, phase, emissive) in floaters {
        let y = base_y + (t * 0.6 + phase).sin() * 0.8;
        out.push(ObjectDesc {
            id, model: model("sphere.glb"),
            pos: [xz[0], y, xz[2]],
            scale: 0.5,
            color, emissive, no_gravity: 1,
        });
    }

    // Falling + exploding spheres — non-emissive to keep shadow ray count low.
    let fallers: &[(u64, [f32; 3], [f32; 3])] = &[
        (30, [ 1.8, 9.5,  0.0], [1.00, 0.15, 0.05]),
        (31, [-1.0, 9.0, -1.5], [0.20, 0.40, 1.00]),
        (32, [ 0.2, 9.8,  1.0], [0.80, 1.00, 0.10]),
    ];
    for &(id, pos, color) in fallers {
        out.push(ObjectDesc {
            id, model: model("sphere.glb"),
            pos, scale: 0.55,
            color, emissive: 2.5, no_gravity: 0,
        });
    }
}


pub const ENV_W: u32 = 1024;
pub const ENV_H: u32 = 512;

#[no_mangle]
pub extern "C" fn make_env_pixels(suns: &[Sun; 4], out: *mut [f32; 3]) {
    let pixels = unsafe { std::slice::from_raw_parts_mut(out, (ENV_W * ENV_H) as usize) };
    compute_env_pixels(suns, pixels);
}

fn compute_env_pixels(_suns: &[Sun], pixels: &mut [[f32; 3]]) {
    // Black environment — all illumination comes from point lights via NEE
    for p in pixels.iter_mut() {
        *p = [0.0, 0.0, 0.0];
    }
}

#[no_mangle]
pub extern "C" fn write_env_hdr(suns: &[Sun; 4]) {
    use std::io::Write as _;
    const W: u32 = ENV_W;
    const H: u32 = ENV_H;
    let mut pixels = vec![[0f32; 3]; (W * H) as usize];
    compute_env_pixels(suns, &mut pixels);

    let mut data = Vec::new();
    write!(data, "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {} +X {}\n", H, W).unwrap();
    for y in 0..H {
        // New-style RLE scanline header: [2, 2, width_hi, width_lo]
        data.push(2);
        data.push(2);
        data.push((W >> 8) as u8);
        data.push((W & 0xFF) as u8);
        let mut rgbe_row = vec![[0u8; 4]; W as usize];
        for x in 0..W {
            let p = pixels[(y * W + x) as usize];
            rgbe_row[x as usize] = float_to_rgbe(p[0], p[1], p[2]);
        }
        // Write each of 4 channels as uncompressed literal runs (max 128 per run)
        for chan in 0..4usize {
            let bytes: Vec<u8> = rgbe_row.iter().map(|px| px[chan]).collect();
            let mut i = 0;
            while i < bytes.len() {
                let len = (bytes.len() - i).min(128);
                data.push(len as u8);
                data.extend_from_slice(&bytes[i..i + len]);
                i += len;
            }
        }
    }
    std::fs::write("data/env_suns.hdr", &data).expect("failed to write hdr");
}

fn float_to_rgbe(r: f32, g: f32, b: f32) -> [u8; 4] {
    let max = r.max(g).max(b);
    if max < 1e-32 {
        return [0, 0, 0, 0];
    }
    let (frac, exp) = frexp(max);
    let scale = frac * 256.0 / max;
    [(r * scale) as u8, (g * scale) as u8, (b * scale) as u8, (exp + 128) as u8]
}

fn frexp(x: f32) -> (f32, i32) {
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as i32 - 126;
    let frac = f32::from_bits((bits & 0x807FFFFF) | 0x3F000000);
    (frac, exp)
}

#[no_mangle]
pub extern "C" fn combined_light(
    suns: &[Sun; 4],
    out_dir: &mut [f32; 3],
    out_color: &mut [f32; 3],
) {
    let scene_center = glam::Vec3::new(0.0, 0.5, 0.0);
    let mut dir = glam::Vec3::ZERO;
    let mut color = glam::Vec3::ZERO;
    for sun in suns {
        let brightness = sun.color.length();
        let from_sun = (scene_center - sun.pos).normalize_or_zero();
        dir += from_sun * brightness;
        color += sun.color;
    }
    let dir = dir.normalize_or_zero();
    let color = color * 1.5;
    *out_dir = [dir.x, dir.y, dir.z];
    *out_color = [color.x, color.y, color.z];
}

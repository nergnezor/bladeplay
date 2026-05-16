pub const BALL_Y: f32 = 0.5;
pub const CUBE_Y: f32 = 0.5;
pub const DRAG_Y: f32 = 0.7;

// --- Tweakable sky/sun constants (hot-reload by saving this file) ---
const SKY_ZENITH: [f32; 3] = [0.02, 0.02, 0.5];   // color at top of sky
const SKY_HORIZON: [f32; 3] = [0.25, 0.08, 0.06];  // extra glow at horizon
const SKY_NADIR: [f32; 3] = [0.12, 0.06, 0.03];    // color at bottom (ground reflection)
const SUN_INTENSITY: f32 = 20.0;                    // brightness of sun blobs in env-map
const SUN_RADIUS: i32 = 6;                          // angular size of suns in env-map pixels

const G: f32 = 80.0;

#[derive(Clone)]
pub struct Sun {
    pub pos: glam::Vec3,
    pub vel: glam::Vec3,
    pub color: glam::Vec3,
    pub mass: f32,
}

#[no_mangle]
pub extern "C" fn make_suns(out: &mut [Sun; 3]) {
    *out = [
        Sun { pos: glam::Vec3::new(-8.0, 6.0, -10.0), vel: glam::Vec3::new(0.6, 0.05, 0.0),   color: glam::Vec3::new(0.6, 0.0, 1.0),  mass: 1.0 },
        Sun { pos: glam::Vec3::new( 2.0, 9.0, -12.0), vel: glam::Vec3::new(-0.4, -0.03, 0.1), color: glam::Vec3::new(1.0, 0.2, 0.6),  mass: 1.2 },
        Sun { pos: glam::Vec3::new( 8.0, 5.0,  -9.0), vel: glam::Vec3::new(-0.3, 0.04, -0.1), color: glam::Vec3::new(1.0, 0.5, 0.0),  mass: 0.8 },
    ];
}

#[no_mangle]
pub extern "C" fn step_suns(suns: &mut [Sun; 3], dt: f32) {
    let positions = [suns[0].pos, suns[1].pos, suns[2].pos];
    let masses = [suns[0].mass, suns[1].mass, suns[2].mass];

    for i in 0..3 {
        let mut acc = glam::Vec3::ZERO;
        for j in 0..3 {
            if i == j {
                continue;
            }
            let diff = positions[j] - positions[i];
            let dist_sq = diff.length_squared().max(0.5);
            acc += diff.normalize() * (G * masses[j] / dist_sq);
        }
        suns[i].vel += acc * dt;
    }
    for sun in suns.iter_mut() {
        sun.pos += sun.vel * dt;
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

// Reserved IDs: 100=ground, 101-103=sun spheres, 104=ball, 105=cube
// IDs 1-99 are free for user objects.

/// Returns the desired scene for this frame.
/// Edit freely — objects are added/removed live on save.
#[no_mangle]
pub extern "C" fn scene_objects(out: &mut SceneDesc) {
    *out = SceneDesc::new();

    // Ground
    out.push(ObjectDesc {
        id: 100, model: model("plane.glb"),
        pos: [0.0, 0.0, 0.0], scale: 1.0,
        color: [1.0, 1.0, 1.0], emissive: 0.0, no_gravity: 1,
    });

    // Sun spheres — emissive with their actual colors
    out.push(ObjectDesc {
        id: 101, model: model("sphere.glb"),
        pos: [-8.0, 6.0, -10.0], scale: 0.8,
        color: [0.6, 0.0, 1.0], emissive: 1.0, no_gravity: 1,
    });
    out.push(ObjectDesc {
        id: 102, model: model("sphere.glb"),
        pos: [2.0, 9.0, -12.0], scale: 0.8,
        color: [1.0, 0.2, 0.6], emissive: 1.0, no_gravity: 1,
    });
    out.push(ObjectDesc {
        id: 103, model: model("sphere.glb"),
        pos: [8.0, 5.0, -9.0], scale: 0.8,
        color: [1.0, 0.5, 0.0], emissive: 1.0, no_gravity: 1,
    });

    // Ball and cube
    out.push(ObjectDesc {
        id: 104, model: model("sphere.glb"),
        pos: [-2.0, 0.5, 0.0], scale: 1.0,
        color: [1.0, 1.0, 1.0], emissive: 0.0, no_gravity: 1,
    });
    out.push(ObjectDesc {
        id: 105, model: model("cube.glb"),
        pos: [2.0, 0.5, 0.0], scale: 1.0,
        color: [1.0, 1.0, 1.0], emissive: 0.0, no_gravity: 1,
    });

    // --- User objects below — edit freely ---
    out.push(ObjectDesc {
        id: 1, model: model("sphere.glb"),
        pos: [0.0, 5.0, 0.0], scale: 1.0,
        color: [0.0, 1.0, 0.4], emissive: 0.0, no_gravity: 0,
    });
    out.push(ObjectDesc {
        id: 2, model: model("torus.glb"),
        pos: [2.0, 4.0, 0.0], scale: 1.0,
        color: [1.0, 0.3, 0.8], emissive: 0.0, no_gravity: 0,
    });
    out.push(ObjectDesc {
        id: 3, model: model("star.glb"),
        pos: [-2.0, 6.0, 0.0], scale: 2.0,
        color: [1.0, 0.9, 0.1], emissive: 0.0, no_gravity: 0,
    });
}

pub const ENV_W: u32 = 512;
pub const ENV_H: u32 = 256;

#[no_mangle]
pub extern "C" fn make_env_pixels(suns: &[Sun; 3], out: *mut [f32; 3]) {
    let pixels = unsafe { std::slice::from_raw_parts_mut(out, (ENV_W * ENV_H) as usize) };
    compute_env_pixels(suns, pixels);
}

fn compute_env_pixels(suns: &[Sun; 3], pixels: &mut [[f32; 3]]) {
    const W: u32 = ENV_W;
    const H: u32 = ENV_H;

    // Sky gradient: zenith → horizon → nadir
    for y in 0..H {
        let t = y as f32 / H as f32;
        let horizon = (-((t - 0.5).abs() * 6.0)).exp();
        for x in 0..W {
            let r = t * SKY_NADIR[0] + horizon * SKY_HORIZON[0] + SKY_ZENITH[0] * (1.0 - t);
            let g = t * SKY_NADIR[1] + horizon * SKY_HORIZON[1] + SKY_ZENITH[1] * (1.0 - t);
            let b = t * SKY_NADIR[2] + horizon * SKY_HORIZON[2] + SKY_ZENITH[2] * (1.0 - t).powf(0.6);
            pixels[(y * W + x) as usize] = [r, g, b];
        }
    }

    // Paint each sun as a bright gaussian blob
    for sun in suns {
        let dir = sun.pos.normalize_or_zero();
        let u = (dir.x.atan2(dir.z) + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
        let v = (0.5 - dir.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI).clamp(0.0, 1.0);
        let cx = (u * W as f32) as i32;
        let cy = (v * H as f32) as i32;
        let radius = SUN_RADIUS;
        for dy in -radius * 4..=radius * 4 {
            for dx in -radius * 4..=radius * 4 {
                let px = ((cx + dx).rem_euclid(W as i32)) as u32;
                let py = (cy + dy).clamp(0, H as i32 - 1) as u32;
                let dist = ((dx * dx + dy * dy) as f32).sqrt() / radius as f32;
                let falloff = (-dist * dist * 0.5).exp();
                let intensity = falloff * SUN_INTENSITY;
                let idx = (py * W + px) as usize;
                pixels[idx][0] += sun.color.x * intensity;
                pixels[idx][1] += sun.color.y * intensity;
                pixels[idx][2] += sun.color.z * intensity;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn write_env_hdr(suns: &[Sun; 3]) {
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
    suns: &[Sun; 3],
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

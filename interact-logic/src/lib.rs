// --- Tweakable sky/sun constants (hot-reload by saving this file) ---
const SKY_ZENITH: [f32; 3] = [0.10, 0.15, 0.45];
const SKY_HORIZON: [f32; 3] = [0.80, 0.30, 0.05];
const SKY_NADIR: [f32; 3] = [0.05, 0.02, 0.01];
const SUN_INTENSITY: f32 = 2000.0;
const SUN_RADIUS: f32 = 0.5;

#[derive(Clone)]
pub struct Sun {
    pub pos: glam::Vec3,
    pub vel: glam::Vec3,  // vel.z = angular speed (rad/s) for circular orbit
    pub color: glam::Vec3,
}

const ORBIT_CENTER: glam::Vec3 = glam::Vec3::new(0.0, 0.0, 0.0);

#[no_mangle]
pub extern "C" fn make_suns(out: &mut [Sun; 4]) {
    *out = [
        Sun { pos: glam::Vec3::new(   0.0,  0.3, -120.0), vel: glam::Vec3::new(0.0, 0.0,  0.006), color: glam::Vec3::new(1.0, 0.50, 0.05) },
        Sun { pos: glam::Vec3::new(  70.0,  0.5, -100.0), vel: glam::Vec3::new(0.0, 0.0, -0.005), color: glam::Vec3::new(1.0, 0.15, 0.05) },
        Sun { pos: glam::Vec3::new( -65.0,  0.5, -105.0), vel: glam::Vec3::new(0.0, 0.0,  0.007), color: glam::Vec3::new(0.55, 0.05, 0.90) },
        Sun { pos: glam::Vec3::new(   0.0, -10.0,   1.0), vel: glam::Vec3::new(0.0, 0.0,  0.0),   color: glam::Vec3::new(0.0, 0.0, 0.0) },
    ];
}

#[no_mangle]
pub extern "C" fn step_suns(suns: &mut [Sun; 4], dt: f32) {
    for sun in suns.iter_mut() {
        let rel = sun.pos - ORBIT_CENTER;
        let r = (rel.x * rel.x + rel.z * rel.z).sqrt().max(0.1);
        let angle = rel.z.atan2(rel.x) + sun.vel.z * dt;
        sun.pos.x = ORBIT_CENTER.x + r * angle.cos();
        sun.pos.z = ORBIT_CENTER.z + r * angle.sin();
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

pub const ENV_W: u32 = 1024;
pub const ENV_H: u32 = 512;

#[no_mangle]
pub extern "C" fn make_env_pixels(suns: &[Sun; 4], out: *mut [f32; 3]) {
    let pixels = unsafe { std::slice::from_raw_parts_mut(out, (ENV_W * ENV_H) as usize) };
    compute_env_pixels(suns, pixels);
}

fn compute_env_pixels(suns: &[Sun], pixels: &mut [[f32; 3]]) {
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

    // Single pass per sun: Rayleigh + Mie + disk blended together, iterated over
    // a circle so no square boundary artefacts appear at any intensity level.
    const RAYLEIGH_RADIUS: f32 = 18.0;
    const RAYLEIGH_INTENSITY: f32 = 80.0;
    const MIE_RADIUS: f32 = 5.0;
    const MIE_INTENSITY: f32 = 3000.0;
    // Outer radius to iterate — driven by Rayleigh which is the widest term
    const ITER_RADIUS: f32 = RAYLEIGH_RADIUS * 2.5;

    for sun in suns {
        let dir = sun.pos.normalize_or_zero();
        let u = (dir.x.atan2(dir.z) + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
        let v = (0.5 - dir.y.clamp(-1.0, 1.0).asin() / std::f32::consts::PI).clamp(0.0, 1.0);
        let cx_f = u * W as f32;
        let cy_f = v * H as f32;
        let cx = cx_f as i32;
        let cy = cy_f as i32;
        let sub_x = cx_f - cx as f32;
        let sub_y = cy_f - cy as f32;

        let r_i = ITER_RADIUS as i32 + 1;
        for dy in -r_i..=r_i {
            for dx in -r_i..=r_i {
                let fdx = dx as f32 - sub_x;
                let fdy = dy as f32 - sub_y;
                let dist2_px = fdx * fdx + fdy * fdy;
                // Hard circular clip — no square edges at any level
                if dist2_px > ITER_RADIUS * ITER_RADIUS { continue; }

                let rayleigh = (-dist2_px / (2.0 * RAYLEIGH_RADIUS * RAYLEIGH_RADIUS)).exp() * RAYLEIGH_INTENSITY;
                let mie      = (-dist2_px / (2.0 * MIE_RADIUS      * MIE_RADIUS     )).exp() * MIE_INTENSITY;
                let disk     = (-dist2_px / (2.0 * SUN_RADIUS       * SUN_RADIUS     )).exp() * SUN_INTENSITY;

                let total = rayleigh + mie + disk;
                if total < 0.01 { continue; }

                let px = ((cx + dx).rem_euclid(W as i32)) as u32;
                let py = (cy + dy).clamp(0, H as i32 - 1) as u32;
                let idx = (py * W + px) as usize;
                // Rayleigh warms the glow (suppress blue); Mie+disk keep full color
                let warm_r = sun.color.x * rayleigh + sun.color.x * (mie + disk);
                let warm_g = sun.color.y * rayleigh * 0.55 + sun.color.y * (mie + disk);
                let warm_b = sun.color.z * rayleigh * 0.15 + sun.color.z * (mie + disk);
                pixels[idx][0] += warm_r;
                pixels[idx][1] += warm_g;
                pixels[idx][2] += warm_b;
            }
        }
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

// --- Tweakable point-light intensity (hot-reload by saving this file) ---
// Light color is `obj.color * obj.emissive * POINT_LIGHT_INTENSITY`.
// Attenuation in the shader is `1/dist²`, so this number needs to be large
// to be visible at typical scene distances of a few meters.
const POINT_LIGHT_INTENSITY: f32 = 8.0;

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
    *out = [
        Sun { pos: glam::Vec3::new(   0.0,  280.0, -8000.0), vel: glam::Vec3::new( 50.0, 0.0,  80.0), color: glam::Vec3::new(1.0, 0.45, 0.04) },
        Sun { pos: glam::Vec3::new(1800.0,  540.0, -7800.0), vel: glam::Vec3::new(-15.0, 0.0, 100.0), color: glam::Vec3::new(0.8, 0.5, 0.3) },
        Sun { pos: glam::Vec3::new(-2400.0, 820.0, -7600.0), vel: glam::Vec3::new(-35.0, 0.0,-180.0), color: glam::Vec3::new(0.8, 0.3, 0.4) },
        // Disabled
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

// Reserved IDs: 100=ground, 101-103=sun spheres, 104=ball, 105=cube
// IDs 1-99: soft body particles

// ---------------------------------------------------------------------------
// Soft body simulation — spring-mass lattice, entirely in the .so.
// State persists between frames via OnceLock; resets on hot-reload.
// ---------------------------------------------------------------------------
use std::sync::{Mutex, OnceLock};

const SB_N: usize = 3;          // 3×3×3 = 27 particles
const SB_SPACING: f32 = 0.85;   // rest spacing between particle centers (m)
const SB_K: f32 = 220.0;        // spring stiffness (N/m)
const SB_DAMPING: f32 = 0.9;    // linear velocity damping coefficient
const SB_RESTITUTION: f32 = 0.82;
const SB_START: glam::Vec3 = glam::Vec3::new(0.0, 3.0, 2.0);
const SB_PARTICLE_R: f32 = 0.8; // particle.glb has radius 0.08 m

struct SoftBody {
    pos:     Vec<glam::Vec3>,
    vel:     Vec<glam::Vec3>,
    springs: Vec<(usize, usize, f32)>, // (i, j, rest_length)
    last_ns: u64,
}

fn wall_clock_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

impl SoftBody {
    fn new() -> Self {
        let half = (SB_N as f32 - 1.0) / 2.0;
        let mut pos = Vec::with_capacity(SB_N * SB_N * SB_N);
        for ix in 0..SB_N {
            for iy in 0..SB_N {
                for iz in 0..SB_N {
                    pos.push(SB_START + glam::Vec3::new(
                        (ix as f32 - half) * SB_SPACING,
                        (iy as f32)        * SB_SPACING,
                        (iz as f32 - half) * SB_SPACING,
                    ));
                }
            }
        }
        // Connect every pair within √2 * SB_SPACING: includes nearest-neighbour
        // and face-diagonal springs, giving volume-preserving stiffness.
        let mut springs = Vec::new();
        for i in 0..pos.len() {
            for j in (i + 1)..pos.len() {
                let d = (pos[j] - pos[i]).length();
                if d <= SB_SPACING * 1.8 {
                    springs.push((i, j, d));
                }
            }
        }
        SoftBody {
            vel: vec![glam::Vec3::ZERO; pos.len()],
            pos,
            springs,
            last_ns: wall_clock_ns(),
        }
    }

    fn step(&mut self) {
        let now = wall_clock_ns();
        let dt = ((now.saturating_sub(self.last_ns)) as f32 * 1e-9).min(0.033);
        self.last_ns = now;
        if dt < 1e-7 { return; }

        let n = self.pos.len();
        let mut force: Vec<glam::Vec3> =
            (0..n).map(|_| glam::Vec3::new(0.0, -9.8, 0.0)).collect();

        for &(i, j, rest) in &self.springs {
            let d = self.pos[j] - self.pos[i];
            let len = d.length().max(1e-4);
            let f = d / len * (SB_K * (len - rest));
            force[i] += f;
            force[j] -= f;
        }

        for k in 0..n {
            self.vel[k] += force[k] * dt;
            self.vel[k] *= (1.0 - SB_DAMPING * dt).max(0.0);
            self.pos[k] += self.vel[k] * dt;
            if self.pos[k].y < SB_PARTICLE_R {
                self.pos[k].y = SB_PARTICLE_R;
                if self.vel[k].y < 0.0 {
                    self.vel[k].y = -self.vel[k].y * SB_RESTITUTION;
                }
            }
        }
    }
}

static SOFTBODY: OnceLock<Mutex<SoftBody>> = OnceLock::new();

fn softbody_step_and_read() -> Vec<glam::Vec3> {
    let guard = SOFTBODY.get_or_init(|| Mutex::new(SoftBody::new()));
    let mut sb = guard.lock().unwrap();
    sb.step();
    sb.pos.clone()
}

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

    // Cube — static reference object
    out.push(ObjectDesc {
        id: 105, model: model("cube.glb"),
        pos: [5.0, 0.5, 0.0], scale: 1.0,
        color: [1.0, 1.0, 1.0], emissive: 0.0, no_gravity: 1,
    });

    // Soft body blob: 3×3×3 sphere particles connected by springs.
    // no_gravity:1 so scene.rs skips gravity; we integrate it ourselves above.
    let positions = softbody_step_and_read();
    let total = positions.len() as f32;
    for (i, pos) in positions.iter().enumerate() {
        let t = i as f32 / (total - 1.0);
        // Warm orange → purple gradient matching the suns
        let color = [
            1.0 - t * 0.5,
            0.35 + t * 0.1,
            0.05 + t * 0.85,
        ];
        out.push(ObjectDesc {
            id: i as u64 + 1,
            model: model("particle.glb"),
            pos: (*pos).into(),
            scale: 4.0,
            color,
            emissive: 0.0, // glass signal: IOR = 1.0 + 0.03 * 20 = 1.6
            no_gravity: 1,
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

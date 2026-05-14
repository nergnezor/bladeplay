use std::path::PathBuf;

pub use interact_logic::{BALL_Y, CUBE_Y, DRAG_Y, Sun};

pub struct Scene {
    pub _ground: blade_engine::ObjectHandle,
    pub ball: blade_engine::ObjectHandle,
    pub cube: blade_engine::ObjectHandle,
    pub ball_pos: glam::Vec3,
    pub cube_pos: glam::Vec3,
    pub suns: [Sun; 3],
}

impl Scene {
    pub fn new(window: &winit::window::Window) -> (blade_engine::Engine, Self) {
        let data_path = PathBuf::from("data");

        let suns = [
            Sun {
                pos: glam::Vec3::new(-12.0, 3.0, -80.0),
                vel: glam::Vec3::new(0.6, 0.05, 0.0),
                color: glam::Vec3::new(0.6, 0.0, 1.0),
                mass: 1.0,
            },
            Sun {
                pos: glam::Vec3::new(4.0, 5.0, -90.0),
                vel: glam::Vec3::new(-0.4, -0.03, 0.1),
                color: glam::Vec3::new(1.0, 0.2, 0.6),
                mass: 1.2,
            },
            Sun {
                pos: glam::Vec3::new(14.0, 2.0, -75.0),
                vel: glam::Vec3::new(-0.3, 0.04, -0.1),
                color: glam::Vec3::new(1.0, 0.5, 0.0),
                mass: 0.8,
            },
        ];

        let mut engine = blade_engine::Engine::new(
            blade_engine::Presentation::Window(window),
            &blade_engine::config::Engine {
                shader_path: "../../blade/blade-render/code".to_string(),
                data_path: data_path.as_os_str().to_string_lossy().into_owned(),
                cache_path: "asset-cache".to_string(),
                time_step: 0.01,
                render_backend: blade_engine::config::RenderBackend::RayTracer,
                gui_enabled: cfg!(debug_assertions),
            },
        );

        let _ground = engine.add_object(
            &blade_engine::config::Object {
                name: "ground".to_string(),
                visuals: vec![blade_engine::config::Visual {
                    model: "plane.glb".to_string(),
                    ..Default::default()
                }],
                colliders: vec![],
                additional_mass: None,
            },
            blade_engine::Transform::default(),
            blade_engine::DynamicInput::Empty,
        );

        let ball_pos = glam::Vec3::new(-2.0, BALL_Y, 0.0);
        let ball = engine.add_object(
            &blade_engine::config::Object {
                name: "ball".to_string(),
                visuals: vec![blade_engine::config::Visual {
                    model: "sphere.glb".to_string(),
                    ..Default::default()
                }],
                colliders: vec![],
                additional_mass: None,
            },
            blade_engine::Transform {
                position: ball_pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            },
            blade_engine::DynamicInput::SetPosition,
        );

        let cube_pos = glam::Vec3::new(2.0, CUBE_Y, 0.0);
        let cube = engine.add_object(
            &blade_engine::config::Object {
                name: "cube".to_string(),
                visuals: vec![blade_engine::config::Visual {
                    model: "cube.glb".to_string(),
                    ..Default::default()
                }],
                colliders: vec![],
                additional_mass: None,
            },
            blade_engine::Transform {
                position: cube_pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            },
            blade_engine::DynamicInput::SetPosition,
        );

        let scene = Self { _ground, ball, cube, ball_pos, cube_pos, suns };
        (engine, scene)
    }

    pub fn write_env_hdr(&self) {
        use std::io::Write as _;
        const W: u32 = 512;
        const H: u32 = 256;
        let mut pixels = vec![[0f32; 3]; (W * H) as usize];

        // Twilight sky: deep blue zenith, warm horizon
        for y in 0..H {
            let t = (y as f32 / H as f32).powf(0.4);
            for x in 0..W {
                pixels[(y * W + x) as usize] = [t * 0.08, t * 0.04, (1.0 - t) * 0.15 + t * 0.04];
            }
        }

        // Paint each sun as a bright gaussian blob
        for sun in &self.suns {
            let dir = sun.pos.normalize_or_zero();
            let u = (dir.z.atan2(dir.x) + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
            let v = dir.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
            let cx = (u * W as f32) as i32;
            let cy = (v * H as f32) as i32;
            let radius = 24i32;
            for dy in -radius * 3..=radius * 3 {
                for dx in -radius * 3..=radius * 3 {
                    let px = ((cx + dx).rem_euclid(W as i32)) as u32;
                    let py = (cy + dy).clamp(0, H as i32 - 1) as u32;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt() / radius as f32;
                    let falloff = (-dist * dist * 0.5).exp();
                    let intensity = falloff * 6.0;
                    let idx = (py * W + px) as usize;
                    pixels[idx][0] += sun.color.x * intensity;
                    pixels[idx][1] += sun.color.y * intensity;
                    pixels[idx][2] += sun.color.z * intensity;
                }
            }
        }

        let mut file = std::fs::File::create("data/env_suns.hdr").expect("failed to create hdr");
        write!(file, "#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {} +X {}\n", H, W).unwrap();
        for p in &pixels {
            file.write_all(&float_to_rgbe(p[0], p[1], p[2])).unwrap();
        }
    }

    pub fn step_suns(&mut self, dt: f32) {
        crate::logic::step_suns(&mut self.suns, dt);
    }

    pub fn sync_to_engine(&self, engine: &mut blade_engine::Engine) {
        engine.teleport_object(
            self.ball,
            blade_engine::Transform {
                position: self.ball_pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            },
        );
        engine.teleport_object(
            self.cube,
            blade_engine::Transform {
                position: self.cube_pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            },
        );
    }
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

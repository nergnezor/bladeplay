use std::path::PathBuf;

pub use interact_logic::{BALL_Y, CUBE_Y, DRAG_Y, Sun};

pub struct Scene {
    pub _ground: blade_engine::ObjectHandle,
    pub ball: blade_engine::ObjectHandle,
    pub cube: blade_engine::ObjectHandle,
    pub sphere: blade_engine::ObjectHandle,
    pub sun_spheres: [blade_engine::ObjectHandle; 3],
    pub ball_pos: glam::Vec3,
    pub cube_pos: glam::Vec3,
    pub suns: [Sun; 3],
}

impl Scene {
    pub fn new(window: &winit::window::Window) -> (blade_engine::Engine, Self) {
        let data_path = PathBuf::from("data");

        let suns = interact_logic::make_suns();

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

        let sphere = engine.add_object(
            &blade_engine::config::Object {
                name: "sphere".to_string(),
                visuals: vec![blade_engine::config::Visual {
                    model: "sphere.glb".to_string(),
                    ..Default::default()
                }],
                colliders: vec![],
                additional_mass: None,
            },
            blade_engine::Transform {
                position: glam::Vec3::new(0.0, BALL_Y, 0.0).into(),
                orientation: glam::Quat::IDENTITY.into(),
            },
            blade_engine::DynamicInput::Empty,
        );

        let sun_colors = [
            glam::Vec3::new(0.6, 0.0, 1.0),
            glam::Vec3::new(1.0, 0.2, 0.6),
            glam::Vec3::new(1.0, 0.5, 0.0),
        ];
        let sun_spheres = std::array::from_fn(|i| {
            let s = engine.add_object(
                &blade_engine::config::Object {
                    name: format!("sun{i}"),
                    visuals: vec![blade_engine::config::Visual {
                        model: "sphere.glb".to_string(),
                        scale: 8.0,
                        ..Default::default()
                    }],
                    colliders: vec![],
                    additional_mass: None,
                },
                blade_engine::Transform {
                    position: suns[i].pos.into(),
                    orientation: glam::Quat::IDENTITY.into(),
                },
                blade_engine::DynamicInput::SetPosition,
            );
            let c = sun_colors[i];
            engine.set_color_tint(s, [c.x, c.y, c.z, 1.0]);
            s
        });

        let scene = Self { _ground, ball, cube, sphere, sun_spheres, ball_pos, cube_pos, suns };
        (engine, scene)
    }

    pub fn make_env_pixels(&self) -> Vec<[f32; 3]> {
        crate::hot_logic::make_env_pixels(&self.suns)
    }

    pub fn step_suns(&mut self, dt: f32) {
        crate::hot_logic::step_suns(&mut self.suns, dt);
    }

    pub fn sync_to_engine(&self, engine: &mut blade_engine::Engine) {
        engine.set_color_tint(self.sphere, crate::hot_logic::sphere_tint());
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
        for (i, &sphere) in self.sun_spheres.iter().enumerate() {
            engine.teleport_object(
                sphere,
                blade_engine::Transform {
                    position: self.suns[i].pos.into(),
                    orientation: glam::Quat::IDENTITY.into(),
                },
            );
        }
    }
}


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

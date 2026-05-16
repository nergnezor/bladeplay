use std::collections::HashMap;
use std::path::PathBuf;

pub use interact_logic::{BALL_Y, CUBE_Y, DRAG_Y, Sun};
use interact_logic::ObjectDesc;

const GRAVITY: f32 = -9.8;
const RESTITUTION: f32 = 0.75;

struct DynPhysics {
    handle: blade_engine::ObjectHandle,
    pos: glam::Vec3,
    vel: glam::Vec3,
    radius: f32,
    spawn_pos: glam::Vec3,
    dragged: bool,
    no_gravity: bool,
}

pub struct Scene {
    pub suns: [Sun; 3],
    dynamic: HashMap<u64, DynPhysics>,
}

impl Scene {
    pub fn new(window: &winit::window::Window) -> (blade_engine::Engine, Self) {
        let data_path = PathBuf::from("data");
        let mut suns = std::array::from_fn(|_| Sun {
            pos: glam::Vec3::ZERO, vel: glam::Vec3::ZERO,
            color: glam::Vec3::ONE, mass: 1.0,
        });
        interact_logic::make_suns(&mut suns);

        let engine = blade_engine::Engine::new(
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

        let scene = Self { suns, dynamic: HashMap::new() };
        (engine, scene)
    }

    pub fn handle_for(&self, id: u64) -> Option<blade_engine::ObjectHandle> {
        self.dynamic.get(&id).map(|p| p.handle)
    }

    pub fn reset_suns(&mut self) {
        crate::hot_logic::make_suns(&mut self.suns);
    }

    pub fn make_env_pixels(&self) -> Vec<[f32; 3]> {
        crate::hot_logic::make_env_pixels(&self.suns)
    }

    pub fn step_suns(&mut self, dt: f32) {
        crate::hot_logic::step_suns(&mut self.suns, dt);
    }

    pub fn sync_dynamic(&mut self, engine: &mut blade_engine::Engine, dt: f32) {
        let desc = crate::hot_logic::scene_objects();
        let wanted: HashMap<u64, ObjectDesc> = desc.objects[..desc.count as usize]
            .iter()
            .map(|o| (o.id, *o))
            .collect();

        self.dynamic.retain(|id, phys| {
            if wanted.contains_key(id) {
                true
            } else {
                engine.remove_object(phys.handle);
                false
            }
        });

        for (id, obj) in &wanted {
            if !self.dynamic.contains_key(id) {
                let pos = glam::Vec3::from(obj.pos);
                let handle = engine.add_object(
                    &blade_engine::config::Object {
                        name: format!("dyn_{id}"),
                        visuals: vec![blade_engine::config::Visual {
                            model: obj.model_str().to_string(),
                            scale: obj.scale,
                            ..Default::default()
                        }],
                        colliders: vec![],
                        additional_mass: None,
                    },
                    blade_engine::Transform {
                        position: pos.into(),
                        orientation: glam::Quat::IDENTITY.into(),
                    },
                    blade_engine::DynamicInput::SetPosition,
                );
                self.dynamic.insert(*id, DynPhysics {
                    handle, pos,
                    vel: glam::Vec3::ZERO,
                    radius: obj.scale * 0.5,
                    spawn_pos: pos,
                    dragged: false,
                    no_gravity: obj.no_gravity != 0,
                });
            }
        }

        for (id, phys) in &mut self.dynamic {
            let obj = &wanted[id];
            let declared_pos = glam::Vec3::from(obj.pos);
            if declared_pos != phys.spawn_pos {
                phys.pos = declared_pos;
                phys.vel = glam::Vec3::ZERO;
                phys.spawn_pos = declared_pos;
            }
            phys.no_gravity = obj.no_gravity != 0;

            if !phys.dragged && !phys.no_gravity {
                phys.vel.y += GRAVITY * dt;
                phys.pos += phys.vel * dt;
                let floor = phys.radius;
                if phys.pos.y < floor {
                    phys.pos.y = floor;
                    phys.vel.y = (-phys.vel.y * RESTITUTION).max(0.0);
                }
            }

            engine.teleport_object(phys.handle, blade_engine::Transform {
                position: phys.pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            });
            engine.set_color_tint(phys.handle, [obj.color[0], obj.color[1], obj.color[2], obj.emissive]);
        }
    }

    pub fn pick_dynamic_ray(&self, origin: glam::Vec3, dir: glam::Vec3, radius: f32) -> Option<(u64, f32)> {
        self.dynamic.iter()
            .filter(|(_, p)| !p.dragged)
            .filter_map(|(id, p)| {
                if dir.y.abs() < 1e-6 { return None; }
                let t = (p.pos.y - origin.y) / dir.y;
                if t < 0.0 { return None; }
                let hit = origin + dir * t;
                let d = glam::Vec2::new(p.pos.x - hit.x, p.pos.z - hit.z).length();
                if d < radius { Some((*id, p.pos.y, d)) } else { None }
            })
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(id, y, _)| (id, y))
    }

    pub fn start_drag(&mut self, id: u64) -> Option<f32> {
        let phys = self.dynamic.get_mut(&id)?;
        phys.dragged = true;
        phys.vel = glam::Vec3::ZERO;
        Some(phys.pos.y)
    }

    pub fn drag_to(&mut self, id: u64, target: glam::Vec3, dt: f32) {
        if let Some(phys) = self.dynamic.get_mut(&id) {
            if phys.dragged {
                let prev = phys.pos;
                phys.pos = target;
                phys.vel = (target - prev) / dt.max(0.001);
            }
        }
    }

    pub fn release_drag(&mut self, id: u64) {
        if let Some(phys) = self.dynamic.get_mut(&id) {
            phys.dragged = false;
        }
    }

    /// Teleport a no_gravity object (e.g. sun spheres) to a new position.
    pub fn set_pos(&mut self, id: u64, pos: glam::Vec3) {
        if let Some(phys) = self.dynamic.get_mut(&id) {
            phys.pos = pos;
            phys.spawn_pos = pos;
        }
    }
}

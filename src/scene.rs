use std::collections::HashMap;

pub use interact_logic::Sun;
use interact_logic::ObjectDesc;

const GRAVITY: f32 = -9.8;
const RESTITUTION: f32 = 0.75;

fn pack_snorm(x: f32, y: f32, z: f32) -> u32 {
    let p = |v: f32| (v.clamp(-1.0, 1.0) * 127.0).round() as i8 as u8;
    u32::from_le_bytes([p(x), p(y), p(z), 127])
}

fn vtx(pos: [f32; 3], n: [f32; 3], t: [f32; 3], uv: [f32; 2]) -> blade_render::Vertex {
    blade_render::Vertex {
        position: pos,
        bitangent_sign: 1.0,
        tex_coords: uv,
        normal: pack_snorm(n[0], n[1], n[2]),
        tangent: pack_snorm(t[0], t[1], t[2]),
    }
}

fn height(x: f32, z: f32) -> f32 {
    // Flat for shadow ray debugging
    return 0.0;
    // Several overlapping sine waves at different frequencies/angles
    let h0 = (x * 0.18 + z * 0.11).sin() * 0.28;
    let h1 = (x * 0.31 - z * 0.27).sin() * 0.15;
    let h2 = (x * 0.07 + z * 0.19).cos() * 0.35;
    let h3 = (x * 0.53 + z * 0.41).sin() * 0.07;
    // Hash-based high-frequency bumps
    let hx = (x * 1.7 + 13.7).sin() * 43758.5453;
    let hz = (z * 2.3 + 7.1).sin() * 21234.1234;
    let bump = ((hx + hz).sin() * 0.5 + 0.5) * 0.06;
    h0 + h1 + h2 + h3 + bump
}

fn make_plane() -> blade_render::ProceduralGeometry {
    let s = 20.0f32;
    let divs: u32 = 80;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for row in 0..=divs {
        for col in 0..=divs {
            let u = col as f32 / divs as f32;
            let v = row as f32 / divs as f32;
            let x = -s + u * 2.0 * s;
            let z = -s + v * 2.0 * s;
            let y = height(x, z);

            // Finite-difference normal
            let eps = 0.05;
            let dydx = (height(x + eps, z) - height(x - eps, z)) / (2.0 * eps);
            let dydz = (height(x, z + eps) - height(x, z - eps)) / (2.0 * eps);
            let len = (dydx * dydx + 1.0 + dydz * dydz).sqrt();
            let n = [-dydx / len, 1.0 / len, -dydz / len];
            let t = [1.0, dydx, 0.0];
            let tl = (t[0] * t[0] + t[1] * t[1]).sqrt();
            let t = [t[0] / tl, t[1] / tl, 0.0];

            vertices.push(vtx([x, y, z], n, t, [u, v]));
        }
    }

    let w = divs + 1;
    for row in 0..divs {
        for col in 0..divs {
            let i00 = row * w + col;
            let i01 = row * w + col + 1;
            let i10 = (row + 1) * w + col;
            let i11 = (row + 1) * w + col + 1;
            indices.extend_from_slice(&[i00, i10, i01, i10, i11, i01]);
        }
    }

    blade_render::ProceduralGeometry {
        name: "plane".to_string(),
        vertices,
        indices,
        base_color_factor: [1.0; 4],
    }
}

fn make_sphere(lat: u32, lon: u32, radius: f32) -> blade_render::ProceduralGeometry {
    use std::f32::consts::{PI, TAU};
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let w = lon + 1;

    for y in 0..=lat {
        let phi = PI * y as f32 / lat as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        for x in 0..=lon {
            let theta = TAU * x as f32 / lon as f32;
            let sin_t = theta.sin();
            let cos_t = theta.cos();
            let nx = sin_phi * cos_t;
            let ny = cos_phi;
            let nz = sin_phi * sin_t;
            let tx = -sin_t;
            let tz = cos_t;
            vertices.push(vtx(
                [nx * radius, ny * radius, nz * radius],
                [nx, ny, nz],
                [tx, 0.0, tz],
                [x as f32 / lon as f32, y as f32 / lat as f32],
            ));
        }
    }

    for y in 0..lat {
        for x in 0..lon {
            let v00 = y * w + x;
            let v01 = y * w + x + 1;
            let v10 = (y + 1) * w + x;
            let v11 = (y + 1) * w + x + 1;
            indices.extend_from_slice(&[v00, v01, v10, v10, v01, v11]);
        }
    }

    blade_render::ProceduralGeometry {
        name: "sphere".to_string(),
        vertices,
        indices,
        base_color_factor: [1.0; 4],
    }
}

fn make_cube() -> blade_render::ProceduralGeometry {
    let h = 0.5f32;
    // Each face: [normal, tangent, 4 vertices in CCW order when viewed from outside]
    // CCW test: cross(B-A, C-A) · N > 0
    let faces: [([f32; 3], [f32; 3], [[f32; 3]; 4]); 6] = [
        // +Y top
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [
            [-h, h, -h], [-h, h,  h], [ h, h,  h], [ h, h, -h],
        ]),
        // -Y bottom
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [
            [-h, -h,  h], [-h, -h, -h], [ h, -h, -h], [ h, -h,  h],
        ]),
        // +Z front
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [
            [-h, -h, h], [ h, -h, h], [ h,  h, h], [-h,  h, h],
        ]),
        // -Z back
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [
            [ h, -h, -h], [-h, -h, -h], [-h,  h, -h], [ h,  h, -h],
        ]),
        // +X right
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [
            [h, -h,  h], [h, -h, -h], [h,  h, -h], [h,  h,  h],
        ]),
        // -X left
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [
            [-h, -h, -h], [-h, -h,  h], [-h,  h,  h], [-h,  h, -h],
        ]),
    ];

    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (face_idx, (n, t, pts)) in faces.iter().enumerate() {
        let base = (face_idx * 4) as u32;
        for (i, p) in pts.iter().enumerate() {
            vertices.push(vtx(*p, *n, *t, uvs[i]));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    blade_render::ProceduralGeometry {
        name: "cube".to_string(),
        vertices,
        indices,
        base_color_factor: [1.0; 4],
    }
}

fn make_torus(maj: u32, min_segs: u32, r_maj: f32, r_min: f32) -> blade_render::ProceduralGeometry {
    use std::f32::consts::TAU;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..maj {
        let u = TAU * i as f32 / maj as f32;
        let cos_u = u.cos();
        let sin_u = u.sin();
        for j in 0..min_segs {
            let v = TAU * j as f32 / min_segs as f32;
            let cos_v = v.cos();
            let sin_v = v.sin();

            let cx = r_maj * cos_u;
            let cz = r_maj * sin_u;

            let px = cx + r_min * cos_v * cos_u;
            let py = r_min * sin_v;
            let pz = cz + r_min * cos_v * sin_u;

            // Normal points from tube center outward
            let nx = cos_v * cos_u;
            let ny = sin_v;
            let nz = cos_v * sin_u;

            // Tangent along major circle direction
            let tx = -sin_u;
            let tz = cos_u;

            vertices.push(vtx(
                [px, py, pz],
                [nx, ny, nz],
                [tx, 0.0, tz],
                [i as f32 / maj as f32, j as f32 / min_segs as f32],
            ));
        }
    }

    for i in 0..maj {
        let i_next = (i + 1) % maj;
        for j in 0..min_segs {
            let j_next = (j + 1) % min_segs;
            let v00 = i * min_segs + j;
            let v01 = i * min_segs + j_next;
            let v10 = i_next * min_segs + j;
            let v11 = i_next * min_segs + j_next;
            indices.extend_from_slice(&[v00, v01, v10, v10, v01, v11]);
        }
    }

    blade_render::ProceduralGeometry {
        name: "torus".to_string(),
        vertices,
        indices,
        base_color_factor: [1.0; 4],
    }
}

fn make_star(points: u32, r_outer: f32, r_inner: f32, thickness: f32) -> blade_render::ProceduralGeometry {
    use std::f32::consts::{PI, TAU};
    // Build top face vertices: center + alternating outer/inner ring points
    let n_ring = points * 2;
    let mut top_verts: Vec<[f32; 3]> = Vec::new();
    let mut bot_verts: Vec<[f32; 3]> = Vec::new();
    let hy = thickness * 0.5;

    top_verts.push([0.0, hy, 0.0]);
    bot_verts.push([0.0, -hy, 0.0]);

    for k in 0..n_ring {
        let angle = TAU * k as f32 / n_ring as f32 - PI * 0.5;
        let r = if k % 2 == 0 { r_outer } else { r_inner };
        top_verts.push([r * angle.cos(), hy, r * angle.sin()]);
        bot_verts.push([r * angle.cos(), -hy, r * angle.sin()]);
    }

    let n_top = [0.0, 1.0, 0.0];
    let n_bot = [0.0, -1.0, 0.0];
    let t_top = [1.0, 0.0, 0.0];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Top face: center=0, ring=1..n_ring
    let top_base = vertices.len() as u32;
    for p in &top_verts {
        vertices.push(vtx(*p, n_top, t_top, [0.5, 0.5]));
    }
    for k in 0..n_ring {
        let a = 1 + k;
        let b = 1 + (k + 1) % n_ring;
        indices.extend_from_slice(&[top_base, top_base + a, top_base + b]);
    }

    // Bottom face: same but flipped winding for outward normal
    let bot_base = vertices.len() as u32;
    for p in &bot_verts {
        vertices.push(vtx(*p, n_bot, t_top, [0.5, 0.5]));
    }
    for k in 0..n_ring {
        let a = 1 + k;
        let b = 1 + (k + 1) % n_ring;
        indices.extend_from_slice(&[bot_base, bot_base + b, bot_base + a]);
    }

    // Side walls
    for k in 0..n_ring {
        let k_next = (k + 1) % n_ring;
        let t0 = &top_verts[1 + k as usize];
        let t1 = &top_verts[1 + k_next as usize];
        let b0 = &bot_verts[1 + k as usize];
        let b1 = &bot_verts[1 + k_next as usize];

        let mid = [(t0[0] + t1[0]) * 0.5, 0.0, (t0[2] + t1[2]) * 0.5];
        let len = (mid[0] * mid[0] + mid[2] * mid[2]).sqrt().max(1e-6);
        let sn = [mid[0] / len, 0.0, mid[2] / len];
        let st = [-(t1[0] - t0[0]), 0.0, -(t1[2] - t0[2])];
        let st_len = (st[0] * st[0] + st[2] * st[2]).sqrt().max(1e-6);
        let st = [st[0] / st_len, 0.0, st[2] / st_len];

        let base = vertices.len() as u32;
        vertices.push(vtx(*t0, sn, st, [0.0, 0.0]));
        vertices.push(vtx(*t1, sn, st, [1.0, 0.0]));
        vertices.push(vtx(*b1, sn, st, [1.0, 1.0]));
        vertices.push(vtx(*b0, sn, st, [0.0, 1.0]));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    blade_render::ProceduralGeometry {
        name: "star".to_string(),
        vertices,
        indices,
        base_color_factor: [1.0; 4],
    }
}

type ModelHandle = blade_asset::Handle<blade_render::Model>;

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
    pub suns: [Sun; 4],
    dynamic: HashMap<u64, DynPhysics>,
    models: HashMap<&'static str, ModelHandle>,
    sun_handles: [Option<blade_engine::ObjectHandle>; 4],
}

impl Scene {
    pub fn new(window: &winit::window::Window) -> (blade_engine::Engine, Self) {
        let data_path = std::path::PathBuf::from("data");
        let mut suns = std::array::from_fn(|_| Sun {
            pos: glam::Vec3::ZERO,
            vel: glam::Vec3::ZERO,
            color: glam::Vec3::ONE,
        });
        interact_logic::make_suns(&mut suns);

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

        let models = Self::register_models(&mut engine);

        let scene = Self { suns, dynamic: HashMap::new(), models, sun_handles: [None; 4] };
        (engine, scene)
    }

    fn register_models(engine: &mut blade_engine::Engine) -> HashMap<&'static str, ModelHandle> {
        let mut m = HashMap::new();
        m.insert("plane.glb",      engine.create_model("plane",      vec![make_plane()]));
        m.insert("sphere.glb",     engine.create_model("sphere",     vec![make_sphere(24, 48, 1.0)]));
        m.insert("particle.glb",   engine.create_model("particle",   vec![make_sphere(8, 16, 0.08)]));
        m.insert("sun_sphere.glb", engine.create_model("sun_sphere", vec![make_sphere(24, 48, 150.0)]));
        m.insert("cube.glb",       engine.create_model("cube",       vec![make_cube()]));
        m.insert("torus.glb",      engine.create_model("torus",      vec![make_torus(48, 24, 1.0, 0.35)]));
        m.insert("star.glb",       engine.create_model("star",       vec![make_star(5, 1.0, 0.4, 0.2)]));
        m
    }

    pub fn reset_suns(&mut self) {
        crate::hot_logic::make_suns(&mut self.suns);
    }

    pub fn make_env_pixels(&self, draw_suns: bool) -> Vec<[f32; 3]> {
        crate::hot_logic::make_env_pixels(&self.suns, draw_suns)
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
                let model_name = obj.model_str();
                let handle = if let Some(&model) = self.models.get(model_name) {
                    engine.add_object_with_model(
                        &format!("dyn_{id}"),
                        model,
                        blade_engine::Transform {
                            position: pos.into(),
                            orientation: glam::Quat::IDENTITY.into(),
                        },
                        blade_engine::DynamicInput::SetPosition,
                    )
                } else {
                    engine.add_object(
                        &blade_engine::config::Object {
                            name: format!("dyn_{id}"),
                            visuals: vec![blade_engine::config::Visual {
                                model: model_name.to_string(),
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
                    )
                };
                self.dynamic.insert(*id, DynPhysics {
                    handle,
                    pos,
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
                let floor = height(phys.pos.x, phys.pos.z) + phys.radius;
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

        // Sync sun sphere geometry to current sun positions.
        let sun_model = self.models["sun_sphere.glb"];
        for (i, sun) in self.suns.iter().enumerate() {
            let enabled = sun.color.length_squared() > 1e-6;
            let transform = blade_engine::Transform {
                position: sun.pos.into(),
                orientation: glam::Quat::IDENTITY.into(),
            };
            match (self.sun_handles[i], enabled) {
                (Some(handle), true) => {
                    engine.teleport_object(handle, transform);
                    engine.set_color_tint(handle, [sun.color.x, sun.color.y, sun.color.z, 0.4]);
                }
                (None, true) => {
                    let handle = engine.add_object_with_model(
                        &format!("sun_{i}"),
                        sun_model,
                        transform,
                        blade_engine::DynamicInput::SetPosition,
                    );
                    engine.set_color_tint(handle, [sun.color.x, sun.color.y, sun.color.z, 0.4]);
                    self.sun_handles[i] = Some(handle);
                }
                (Some(handle), false) => {
                    engine.remove_object(handle);
                    self.sun_handles[i] = None;
                }
                (None, false) => {}
            }
        }

        // Upload emissive objects as point lights for ray-tracing NEE.
        // `radius` is the physical sphere radius (used to offset shadow rays
        // so they don't self-intersect the light's own geometry).
        // `color` is premultiplied by intensity to overcome 1/r² falloff.
        let intensity_scale = crate::hot_logic::point_light_intensity();
        let mut point_lights: Vec<blade_render::PointLight> = wanted.values()
            .filter(|o| o.emissive > 0.0)
            .filter_map(|o| {
                let phys = self.dynamic.get(&o.id)?;
                let intensity = intensity_scale * o.emissive;
                Some(blade_render::PointLight {
                    pos: phys.pos.into(),
                    // Sphere mesh is always unit-radius (add_object_with_model uses identity transform)
                    radius: 1.0,
                    color: [
                        o.color[0] * intensity,
                        o.color[1] * intensity,
                        o.color[2] * intensity,
                    ],
                    _pad: 0.0,
                })
            })
            .collect();

        // Suns as point lights: color premultiplied by dist² so 1/dist² cancels out,
        // producing near-parallel illumination regardless of how far away the sun is.
        let sun_illum = crate::hot_logic::sun_illuminance();
        for sun in self.suns.iter() {
            if sun.color.length_squared() < 1e-6 { continue; }
            let dist_sq = sun.pos.length_squared();
            let c = sun.color * (dist_sq * sun_illum);
            point_lights.push(blade_render::PointLight {
                pos: sun.pos.into(),
                radius: 201.0,
                color: c.into(),
                _pad: 0.0,
            });
        }

        engine.set_point_lights(&point_lights);
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

    pub fn start_drag(&mut self, id: u64) -> Option<(f32, f32)> {
        let phys = self.dynamic.get_mut(&id)?;
        phys.dragged = true;
        phys.vel = glam::Vec3::ZERO;
        Some((phys.pos.y, phys.pos.z))  // (y, z) — caller uses z for depth plane
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
}

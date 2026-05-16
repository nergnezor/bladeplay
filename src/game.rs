use crate::{hud, picking, scene::Scene};
use blade_helpers::{Camera, ControlledCamera};
use std::time;

#[derive(Debug)]
pub struct QuitEvent;

pub struct Game {
    pub engine: blade_engine::Engine,
    pub scene: Scene,
    pub camera: ControlledCamera,
    pub window: winit::window::Window,
    pub ball: blade_engine::ObjectHandle,
    pub cube: blade_engine::ObjectHandle,
    last_update: time::Instant,
    egui_state: egui_winit::State,
    egui_viewport_id: egui::ViewportId,
    // drag state
    dragging: Option<blade_engine::ObjectHandle>,
    drag_offset: glam::Vec3,
    mouse_pos: glam::Vec2,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    surface_ready: bool,
    frame_count: u32,
    logic_src_mtime: time::SystemTime,
    rebuild_process: Option<std::process::Child>,
}

impl Drop for Game {
    fn drop(&mut self) {
        self.engine.destroy();
    }
}

impl Game {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let window = event_loop
            .create_window(
                winit::window::Window::default_attributes().with_title("blade-interact"),
            )
            .unwrap();

        let camera = ControlledCamera {
            inner: Camera {
                pos: glam::Vec3::new(0.0, 8.0, 12.0).into(),
                rot: glam::Quat::from_rotation_x(-0.6).into(),
                fov_y: 1.0,
                depth: 100.0,
                fov: None,
            },
            fly_speed: 10.0,
        };

        let (engine, scene) = Scene::new(&window);

        let egui_context = egui::Context::default();
        let egui_viewport_id = egui_context.viewport_id();
        let egui_state =
            egui_winit::State::new(egui_context, egui_viewport_id, &window, None, None, None);

        let window_size = window.inner_size();
        let ball = scene.ball;
        let cube = scene.cube;

        Self {
            engine,
            scene,
            camera,
            window,
            ball,
            cube,
            last_update: time::Instant::now(),
            egui_state,
            egui_viewport_id,
            dragging: None,
            drag_offset: glam::Vec3::ZERO,
            mouse_pos: glam::Vec2::ZERO,
            window_size,
            surface_ready: false,
            frame_count: 0,
            logic_src_mtime: logic_src_mtime(),
            rebuild_process: None,
        }
    }

    pub fn on_event(
        &mut self,
        event: &winit::event::WindowEvent,
    ) -> Result<winit::event_loop::ControlFlow, QuitEvent> {
        let response = self.egui_state.on_window_event(&self.window, event);
        if response.repaint {
            self.window.request_redraw();
        }
        let egui_consumed = response.consumed;

        let delta = 0.016f32;

        match *event {
            winit::event::WindowEvent::CloseRequested => return Err(QuitEvent),
            winit::event::WindowEvent::Resized(size) => {
                self.window_size = size;
                if size.width > 0 && size.height > 0 {
                    self.surface_ready = true;
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = glam::Vec2::new(position.x as f32, position.y as f32);
                if let Some(handle) = self.dragging {
                    let (origin, dir) =
                        picking::screen_ray(&self.camera, self.window_size, self.mouse_pos);
                    if let Some(hit) = picking::ray_plane_hit(origin, dir, crate::scene::DRAG_Y) {
                        let world = hit + self.drag_offset;
                        if handle == self.ball {
                            self.scene.ball_pos =
                                glam::Vec3::new(world.x, crate::scene::BALL_Y, world.z);
                        } else {
                            self.scene.cube_pos =
                                glam::Vec3::new(world.x, crate::scene::CUBE_Y, world.z);
                        }
                    }
                }
            }
            _ if egui_consumed => {}
            winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if key_code == winit::keyboard::KeyCode::Escape {
                    return Err(QuitEvent);
                }
                self.camera.on_key(key_code, delta);
            }
            winit::event::WindowEvent::MouseWheel { delta, .. } => {
                self.camera.on_wheel(delta);
            }
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    if let Some((handle, offset)) = picking::pick_object(self, self.mouse_pos) {
                        self.dragging = Some(handle);
                        self.drag_offset = offset;
                    }
                }
                winit::event::ElementState::Released => {
                    self.dragging = None;
                }
            },
            winit::event::WindowEvent::RedrawRequested => {
                if self.surface_ready {
                    self.on_draw();
                }
                self.window.request_redraw();
                return Ok(winit::event_loop::ControlFlow::Wait);
            }
            _ => {}
        }

        Ok(winit::event_loop::ControlFlow::Poll)
    }

    fn on_draw(&mut self) {
        let dt = self.last_update.elapsed().as_secs_f32();
        self.last_update = time::Instant::now();

        self.check_hot_reload();

        self.frame_count += 1;
        // Upload env map directly to GPU every 30 frames so lighting tracks sun movement
        if self.frame_count <= 2 || self.frame_count % 30 == 0 {
            let pixels = self.scene.make_env_pixels();
            self.engine.set_environment_map_hdr_data(
                interact_logic::ENV_W,
                interact_logic::ENV_H,
                &pixels,
            );
        }

        self.engine.update(dt);
        self.scene.step_suns(dt);
        self.scene.sync_to_engine(&mut self.engine);

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_context = self.egui_state.egui_ctx().clone();
        let egui_output = egui_context.run_ui(raw_input, |egui_ctx| {
            let frame = hud::panel_frame(egui_ctx);
            egui::Panel::right("hud")
                .frame(frame)
                .show_inside(egui_ctx, |ui| hud::populate(self, ui));
        });

        self.egui_state
            .handle_platform_output(&self.window, egui_output.platform_output);

        let primitives = self
            .egui_state
            .egui_ctx()
            .tessellate(egui_output.shapes, egui_output.pixels_per_point);
        self.engine.render(
            &self.camera.inner.into(),
            &primitives,
            &egui_output.textures_delta,
            self.window.inner_size(),
            self.window.scale_factor() as f32,
        );

        let _ = egui_output.viewport_output[&self.egui_viewport_id].repaint_delay;
    }

    fn check_hot_reload(&mut self) {
        // Poll finished rebuild
        if let Some(ref mut child) = self.rebuild_process {
            match child.try_wait() {
                Ok(None) => {} // still building
                _ => { self.rebuild_process = None; } // done or already reaped
            }
        }

        // Check if source changed and trigger a rebuild
        let mtime = logic_src_mtime();
        if mtime != self.logic_src_mtime {
            self.logic_src_mtime = mtime;
            if self.rebuild_process.is_none() {
                eprintln!("[hot_logic] source changed, spawning cargo build...");
                self.rebuild_process = std::process::Command::new("cargo")
                    .args(["build", "-p", "interact-logic"])
                    .spawn()
                    .ok();
            }
        }

        // Always poll the .so file (handles external rebuilds too)
        crate::hot_logic::try_reload();
    }
}

fn logic_src_mtime() -> time::SystemTime {
    std::fs::metadata("interact-logic/src/lib.rs")
        .and_then(|m| m.modified())
        .unwrap_or(time::SystemTime::UNIX_EPOCH)
}

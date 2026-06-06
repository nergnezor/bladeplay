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
    last_update: time::Instant,
    start_time:  time::Instant,
    egui_state: egui_winit::State,
    egui_viewport_id: egui::ViewportId,
    dragging_dyn: Option<u64>,
    drag_z: f32,
    drag_offset: glam::Vec3,
    mouse_pos: glam::Vec2,
    pub window_size: winit::dpi::PhysicalSize<u32>,
    surface_ready: bool,
    logic_src_mtime: time::SystemTime,
    rebuild_process: Option<std::process::Child>,
    pub draw_suns: bool,
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
                winit::window::Window::default_attributes()
                    .with_title("blade-interact")
                    .with_inner_size(winit::dpi::LogicalSize::new(800u32, 533u32)),
            )
            .unwrap();

        // Inside the 10×10×10 m room, wide view of the whole space.
        let cam_pos = glam::Vec3::new(0.0, 2.5,  4.0);
        let target  = glam::Vec3::new(0.0, 2.5, -4.0);
        let forward = (target - cam_pos).normalize();
        let rot = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);
        let camera = ControlledCamera {
            inner: Camera {
                pos: cam_pos.into(),
                rot: rot.into(),
                fov_y: 1.2,
                depth: 200.0,
                fov: None,
            },
            fly_speed: 4.0,
        };

        let (mut engine, scene) = Scene::new(&window);
        engine.set_render_scale(0.12);

        let egui_context = egui::Context::default();
        let egui_viewport_id = egui_context.viewport_id();
        let egui_state =
            egui_winit::State::new(egui_context, egui_viewport_id, &window, None, None, None);

        let window_size = window.inner_size();

        Self {
            engine, scene, camera, window,
            last_update: time::Instant::now(),
            start_time:  time::Instant::now(),
            egui_state, egui_viewport_id,
            dragging_dyn: None,
            drag_z: 0.0,
            drag_offset: glam::Vec3::ZERO,
            mouse_pos: glam::Vec2::ZERO,
            window_size,
            surface_ready: false,
            logic_src_mtime: logic_src_mtime(),
            rebuild_process: None,
            draw_suns: true,
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
                if let Some(id) = self.dragging_dyn {
                    let (origin, dir) =
                        picking::screen_ray(&self.camera, self.window_size, self.mouse_pos);
                    if let Some(hit) = picking::ray_z_plane_hit(origin, dir, self.drag_z) {
                        let target = hit + self.drag_offset;
                        self.scene.drag_to(id, glam::Vec3::new(target.x, target.y, self.drag_z), delta);
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
                if self.dragging_dyn.is_some() {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.01,
                    };
                    self.drag_z -= scroll * 0.5;
                } else {
                    self.camera.on_wheel(delta);
                }
            }
            winit::event::WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    let (origin, dir) =
                        picking::screen_ray(&self.camera, self.window_size, self.mouse_pos);
                    if let Some((id, _y)) = self.scene.pick_dynamic_ray(origin, dir, picking::PICK_RADIUS) {
                        if let Some((_y, drag_z)) = self.scene.start_drag(id) {
                            self.drag_z = drag_z;
                            self.drag_offset = glam::Vec3::ZERO;
                            self.dragging_dyn = Some(id);
                        }
                    }
                }
                winit::event::ElementState::Released => {
                    if let Some(id) = self.dragging_dyn.take() {
                        self.scene.release_drag(id);
                    }
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
        let frame_start = time::Instant::now();
        let dt = self.last_update.elapsed().as_secs_f32();
        self.last_update = frame_start;

        crate::hot_logic::set_elapsed(self.start_time.elapsed().as_secs_f32());
        self.check_hot_reload();

        self.engine.update(dt);
        self.scene.step_suns(dt);

        self.scene.sync_dynamic(&mut self.engine, dt);

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
        let pre_render = time::Instant::now();
        self.engine.render(
            &self.camera.inner.into(),
            &primitives,
            &egui_output.textures_delta,
            self.window.inner_size(),
            self.window.scale_factor() as f32,
        );
        let render_ms = pre_render.elapsed().as_secs_f32() * 1000.0;

        let _ = egui_output.viewport_output[&self.egui_viewport_id].repaint_delay;

        // Track frame times, report avg/max every 300 frames (excl. pauses >200ms).
        let frame_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        if frame_ms < 200.0 {
            static COUNT:      std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            static SUM_FRAME:  std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static MAX_FRAME:  std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            static SUM_RENDER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            static MAX_RENDER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            SUM_FRAME.fetch_add((frame_ms * 100.0) as u64, std::sync::atomic::Ordering::Relaxed);
            MAX_FRAME.fetch_max(frame_ms.to_bits(), std::sync::atomic::Ordering::Relaxed);
            SUM_RENDER.fetch_add((render_ms * 100.0) as u64, std::sync::atomic::Ordering::Relaxed);
            MAX_RENDER.fetch_max(render_ms.to_bits(), std::sync::atomic::Ordering::Relaxed);
            if count % 300 == 299 {
                let sf  = SUM_FRAME.swap(0, std::sync::atomic::Ordering::Relaxed);
                let mf  = f32::from_bits(MAX_FRAME.swap(0, std::sync::atomic::Ordering::Relaxed));
                let sr  = SUM_RENDER.swap(0, std::sync::atomic::Ordering::Relaxed);
                let mr  = f32::from_bits(MAX_RENDER.swap(0, std::sync::atomic::Ordering::Relaxed));
                let avg_f = sf as f32 / 100.0 / 300.0;
                let avg_r = sr as f32 / 100.0 / 300.0;
                eprintln!("[perf] frame avg={:.1}ms max={:.1}ms | render avg={:.1}ms max={:.1}ms",
                    avg_f, mf, avg_r, mr);
                for (name, dur) in self.engine.gpu_pass_timings() {
                    eprintln!("  [gpu] {}: {:.2}ms", name, dur.as_secs_f32() * 1000.0);
                }
            }
        }
    }

    fn check_hot_reload(&mut self) {
        if let Some(ref mut child) = self.rebuild_process {
            match child.try_wait() {
                Ok(None) => {}
                _ => { self.rebuild_process = None; }
            }
        }

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

        crate::hot_logic::try_reload();
        if crate::hot_logic::take_reloaded() {
            self.scene.reset_suns();
            self.engine.reset_accumulation();
        }

    }
}

fn logic_src_mtime() -> time::SystemTime {
    std::fs::metadata("interact-logic/src/lib.rs")
        .and_then(|m| m.modified())
        .unwrap_or(time::SystemTime::UNIX_EPOCH)
}

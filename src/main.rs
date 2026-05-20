#[cfg(not(target_os = "android"))]
mod game;
#[cfg(not(target_os = "android"))]
mod hot_logic;
#[cfg(not(target_os = "android"))]
mod hud;
#[cfg(not(target_os = "android"))]
mod picking;
#[cfg(not(target_os = "android"))]
mod scene;

#[cfg(not(target_os = "android"))]
use game::{Game, QuitEvent};

#[cfg(not(target_os = "android"))]
struct App {
    game: Option<Game>,
}

#[cfg(not(target_os = "android"))]
impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.game = Some(Game::new(event_loop));
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(game) = &self.game {
            game.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let game = self.game.as_mut().unwrap();
        match game.on_event(&event) {
            Ok(cf) => event_loop.set_control_flow(cf),
            Err(QuitEvent) => event_loop.exit(),
        }
    }
}

#[cfg(not(target_os = "android"))]
fn main() {
    // Prevent SIGCHLD from being delivered when cargo-build subprocesses exit.
    // POSIX: with SIG_IGN, children are auto-reaped with no signal to the parent.
    #[cfg(unix)]
    unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN); }

    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = App { game: None };
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(target_os = "android")]
fn main() {
    // Android runtime integration is not wired up in this crate yet.
}

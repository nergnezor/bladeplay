mod game;
mod hud;
mod picking;
mod scene;

use game::{Game, QuitEvent};

struct App {
    game: Option<Game>,
}

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

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = App { game: None };
    event_loop.run_app(&mut app).unwrap();
}

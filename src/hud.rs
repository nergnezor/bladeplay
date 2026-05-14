use crate::game::Game;

pub fn populate(game: &mut Game, ui: &mut egui::Ui) {
    use blade_helpers::ExposeHud as _;

    egui::CollapsingHeader::new("Camera")
        .default_open(true)
        .show(ui, |ui| game.camera.populate_hud(ui));

    game.engine.populate_hud(ui);
}

pub fn panel_frame(ctx: &egui::Context) -> egui::Frame {
    let mut frame = egui::Frame::side_top_panel(&ctx.global_style());
    let mut fill = frame.fill.to_array();
    for f in fill.iter_mut() {
        *f = (*f as u32 * 7 / 8) as u8;
    }
    frame.fill = egui::Color32::from_rgba_premultiplied(fill[0], fill[1], fill[2], fill[3]);
    frame
}

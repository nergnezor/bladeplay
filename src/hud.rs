use crate::game::Game;

pub fn populate(game: &mut Game, ui: &mut egui::Ui) {
    use blade_helpers::ExposeHud as _;

    egui::CollapsingHeader::new("Camera")
        .default_open(true)
        .show(ui, |ui| game.camera.populate_hud(ui));

    egui::CollapsingHeader::new("Light")
        .default_open(true)
        .show(ui, |ui| {
            let light = &mut game.scene.light;
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.label("Direction:");
                changed |= ui
                    .add(egui::DragValue::new(&mut light.light_dir.x).speed(0.01))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut light.light_dir.y).speed(0.01))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut light.light_dir.z).speed(0.01))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Color:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut light.light_color.x)
                            .speed(0.05)
                            .range(0.0..=10.0),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut light.light_color.y)
                            .speed(0.05)
                            .range(0.0..=10.0),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut light.light_color.z)
                            .speed(0.05)
                            .range(0.0..=10.0),
                    )
                    .changed();
            });
            if changed {
                game.engine.set_raster_config(*light);
            }
        });

    game.engine.populate_hud(ui);
}

/// Build the right-side panel frame with a slightly darker fill than the default.
pub fn panel_frame(ctx: &egui::Context) -> egui::Frame {
    let mut frame = egui::Frame::side_top_panel(&ctx.global_style());
    let mut fill = frame.fill.to_array();
    for f in fill.iter_mut() {
        *f = (*f as u32 * 7 / 8) as u8;
    }
    frame.fill = egui::Color32::from_rgba_premultiplied(fill[0], fill[1], fill[2], fill[3]);
    frame
}

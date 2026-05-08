use crate::game::Game;
use blade_helpers::ControlledCamera;

pub const PICK_RADIUS: f32 = 1.2;

/// Unproject screen coords to a world-space ray (origin + normalized direction).
pub fn screen_ray(
    camera: &ControlledCamera,
    size: winit::dpi::PhysicalSize<u32>,
    screen: glam::Vec2,
) -> (glam::Vec3, glam::Vec3) {
    let w = size.width as f32;
    let h = size.height as f32;
    let aspect = w / h;

    let ndc_x = (screen.x / w) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen.y / h) * 2.0;

    // Match the rasterizer's projection (near=0.01).
    let proj = glam::Mat4::perspective_rh(camera.inner.fov_y, aspect, 0.01, camera.inner.depth);
    let view = camera.get_view_matrix();
    let inv = (proj * view).inverse();

    let near = inv.project_point3(glam::Vec3::new(ndc_x, ndc_y, -1.0));
    let far = inv.project_point3(glam::Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far - near).normalize();
    (near, dir)
}

/// Intersect ray with horizontal plane at y = height.
pub fn ray_plane_hit(origin: glam::Vec3, dir: glam::Vec3, y: f32) -> Option<glam::Vec3> {
    if dir.y.abs() < 1e-6 {
        return None;
    }
    let t = (y - origin.y) / dir.y;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

/// Returns the closest pickable object under the cursor and the offset from
/// the plane-hit to its center (used to keep grab-relative position while dragging).
pub fn pick_object(
    game: &Game,
    screen: glam::Vec2,
) -> Option<(blade_engine::ObjectHandle, glam::Vec3)> {
    let (origin, dir) = screen_ray(&game.camera, game.window_size, screen);
    let hit = ray_plane_hit(origin, dir, crate::scene::DRAG_Y)?;

    let candidates = [game.ball, game.cube];
    let mut best: Option<(blade_engine::ObjectHandle, f32)> = None;
    for handle in candidates {
        let pos: glam::Vec3 = game.engine.get_object_position(handle).into();
        let dist = (hit - glam::Vec3::new(pos.x, crate::scene::DRAG_Y, pos.z)).length();
        if dist < PICK_RADIUS && best.map_or(true, |(_, d)| dist < d) {
            best = Some((handle, dist));
        }
    }
    best.map(|(h, _)| {
        let pos: glam::Vec3 = game.engine.get_object_position(h).into();
        let offset = glam::Vec3::new(pos.x, crate::scene::DRAG_Y, pos.z) - hit;
        (h, offset)
    })
}

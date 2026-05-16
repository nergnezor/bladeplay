use blade_helpers::ControlledCamera;

pub const PICK_RADIUS: f32 = 1.2;

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

    let proj = glam::Mat4::perspective_rh(camera.inner.fov_y, aspect, 0.01, camera.inner.depth);
    let view = camera.get_view_matrix();
    let inv = (proj * view).inverse();

    let near = inv.project_point3(glam::Vec3::new(ndc_x, ndc_y, -1.0));
    let far  = inv.project_point3(glam::Vec3::new(ndc_x, ndc_y,  1.0));
    (near, (far - near).normalize())
}

pub fn ray_plane_hit(origin: glam::Vec3, dir: glam::Vec3, y: f32) -> Option<glam::Vec3> {
    if dir.y.abs() < 1e-6 { return None; }
    let t = (y - origin.y) / dir.y;
    if t < 0.0 { return None; }
    Some(origin + dir * t)
}

// Intersect ray with vertical plane at constant z, returning world-space hit.
pub fn ray_z_plane_hit(origin: glam::Vec3, dir: glam::Vec3, z: f32) -> Option<glam::Vec3> {
    if dir.z.abs() < 1e-6 { return None; }
    let t = (z - origin.z) / dir.z;
    if t < 0.0 { return None; }
    Some(origin + dir * t)
}

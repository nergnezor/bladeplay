pub const BALL_Y: f32 = 0.5;
pub const CUBE_Y: f32 = 0.5;
pub const DRAG_Y: f32 = 0.7;

const G: f32 = 80.0;

#[derive(Clone)]
pub struct Sun {
    pub pos: glam::Vec3,
    pub vel: glam::Vec3,
    pub color: glam::Vec3,
    pub mass: f32,
}

pub fn make_suns() -> [Sun; 3] {
    [
        Sun { pos: glam::Vec3::new(-12.0, 3.0, -80.0), vel: glam::Vec3::new(0.6, 0.05, 0.0),   color: glam::Vec3::new(0.6, 0.0, 1.0),  mass: 1.0 },
        Sun { pos: glam::Vec3::new(  4.0, 5.0, -90.0), vel: glam::Vec3::new(-0.4, -0.03, 0.1), color: glam::Vec3::new(1.0, 0.2, 0.6),  mass: 1.2 },
        Sun { pos: glam::Vec3::new( 14.0, 2.0, -75.0), vel: glam::Vec3::new(-0.3, 0.04, -0.1), color: glam::Vec3::new(1.0, 0.5, 0.0),  mass: 0.8 },
    ]
}

#[no_mangle]
pub extern "C" fn step_suns(suns: &mut [Sun; 3], dt: f32) {
    let positions = [suns[0].pos, suns[1].pos, suns[2].pos];
    let masses = [suns[0].mass, suns[1].mass, suns[2].mass];

    for i in 0..3 {
        let mut acc = glam::Vec3::ZERO;
        for j in 0..3 {
            if i == j {
                continue;
            }
            let diff = positions[j] - positions[i];
            let dist_sq = diff.length_squared().max(0.5);
            acc += diff.normalize() * (G * masses[j] / dist_sq);
        }
        suns[i].vel += acc * dt;
    }
    for sun in suns.iter_mut() {
        sun.pos += sun.vel * dt;
    }
}

#[no_mangle]
pub extern "C" fn combined_light(
    suns: &[Sun; 3],
    out_dir: &mut [f32; 3],
    out_color: &mut [f32; 3],
) {
    let scene_center = glam::Vec3::new(0.0, 0.5, 0.0);
    let mut dir = glam::Vec3::ZERO;
    let mut color = glam::Vec3::ZERO;
    for sun in suns {
        let brightness = sun.color.length();
        let from_sun = (scene_center - sun.pos).normalize_or_zero();
        dir += from_sun * brightness;
        color += sun.color;
    }
    let dir = dir.normalize_or_zero();
    let color = color * 1.5;
    *out_dir = [dir.x, dir.y, dir.z];
    *out_color = [color.x, color.y, color.z];
}

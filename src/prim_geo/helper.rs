use bevy::prelude::{Quat, Vec3};

#[inline]
pub fn cal_ref_axis(v: &Vec3) -> Vec3 {
    let v = v.normalize();
    let a = v.x.abs();
    let b = v.y.abs();
    let c = v.z.abs();
    let mut dx = Vec3::new(1.0f32, 0.0, 0.0);
    if b <= a && b <= c {
        dx = Vec3::new(-v.z as f32, 0.0, v.x);
    } else if a <= b && a <= c {
        dx = Vec3::new(0.0, -v.z, v.y);
    } else {
        dx = Vec3::new(-v.y, v.x, 0.0);
    }
    dx
}

///针对torus的角度求解
pub fn rotate_from_vec3_to_vec3(dir: Vec3, from: Vec3, to: Vec3) -> Quat {
    let mut angle = from.angle_between(to);
    if angle.abs() < 1.0e-3 || (angle.abs() - std::f32::consts::PI).abs() < 1.0e-3 {
        let mut rotation_angle = angle;
        let mut ref_axis = dir;
        Quat::from_axis_angle(
            from.cross(ref_axis).normalize(),
            rotation_angle,
        )
    } else {
        // 不平行
        let a1 = dir.angle_between(from);
        let a2 = dir.angle_between(to);

        let mut z_dir = from.cross(to).normalize();
        if ((a1 + a2) - angle).abs() > 1.0e-3 {
            angle =  std::f32::consts::TAU - angle;
            z_dir = -z_dir;
        }
        Quat::from_axis_angle(z_dir, angle)
    }
}


pub fn quad_indices(indices: &mut Vec<usize>, l: &mut usize, o: usize, v0: usize, v1: usize, v2: usize, v3: usize){
    indices.push(o + v0);
    indices.push(o + v1);
    indices.push(o + v2);
    indices.push(o + v2);
    indices.push(o + v3);
    indices.push(o + v0);
    *l += 6;
}
use glam::{Quat, Vec3};

///解析expression到direction
pub fn parse_expr_to_dir(expr: &str) -> Vec3{
    // if let Ok((_, res)) = parse_rotation_struct(expr){
    //     // //////dbg!(&res);
    //     let mut axis = res.origin_axis;
    //     if res.rot1.is_some() {
    //         let rot1 = res.rot1.as_ref().unwrap();
    //         let target_axis = axis.cross(rot1.axis);
    //         let mut quat1 = Quat::from_axis_angle(target_axis, rot1.angle.to_radians());
    //         axis = quat1*axis;
    //         if res.rot2.is_some() {
    //             let rot2 = res.rot2.as_ref().unwrap();
    //             let target_axis = axis.cross(rot2.axis);
    //             let mut quat2 = Quat::from_axis_angle(target_axis, rot2.angle.to_radians());
    //             axis = quat2 * axis;
    //         }
    //     }
    //     return axis;
    // }
    Vec3::ZERO
}

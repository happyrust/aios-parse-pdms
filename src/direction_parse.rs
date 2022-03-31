use glam::*;
use nom::number::complete:: float;

use nom::*;

use static_init::{dynamic};
use std::collections::HashMap;

#[dynamic]
static AXISES_MAP: HashMap<&'static str, Vec3> = vec![("X", Vec3::X),
                                                      ("Y", Vec3::Y),
                                                      ("Z", Vec3::Z),
                                                      ("-X", Vec3::new(-1.0, 0.0, 0.0)),
                                                      ("-Y", Vec3::new(0.0, -1.0, 0.0)),
                                                      ("-Z", Vec3::new(0.0, 0.0, -1.0)), ].into_iter().collect();

#[derive(Debug, Default)]
struct Rotation {
    axis: Vec3,
    angle: f32,
}

#[derive(Debug, Default)]
struct RotationStruct {
    origin_axis: Vec3,
    rot1: Option<Rotation>,
    rot2: Option<Rotation>,
}
named!(parse_rotation_struct<&str, RotationStruct>, do_parse!(
    axis: recognize!(signed_axis) >>
    rot1: opt!(complete!(parse_axis_rotation)) >>
    rot2: opt!(complete!(parse_axis_rotation)) >>
    (RotationStruct{
        origin_axis: *AXISES_MAP.get(axis).unwrap(),
        rot1,
        rot2,
    })
));
named!(signed_axis<&str, (Option<&str>, &str)>,
    pair!(
        opt!(tag!("-")),  // maybe sign?
        alt!(tag!("X") | tag!("Y") | tag!("Z"))
    )
);
named!(parse_angle<&str, f32>,
    alt!(
        float |
        delimited!( tag!("("), float, tag!(")") )
    )
);
//recognize!(pair!(parse_angle, signed_axis))
named!(parse_axis_rotation<&str, Rotation>, do_parse!(
    angle: parse_angle >>
    axis: recognize!(signed_axis) >>
    (Rotation{
        axis: *AXISES_MAP.get(axis).unwrap(),
        angle,   //need panic here
    })
));

///解析expression到direction
pub fn parse_expr_to_dir(expr: &str) -> Vec3 {
    if let Ok((_, res)) = parse_rotation_struct(expr) {
        // //////dbg!(&res);
        let mut axis = res.origin_axis;
        if res.rot1.is_some() {
            let rot1 = res.rot1.as_ref().unwrap();
            let target_axis = axis.cross(rot1.axis);
            let quat1 = Quat::from_axis_angle(target_axis, rot1.angle.to_radians());
            axis = quat1 * axis;
            if res.rot2.is_some() {
                let rot2 = res.rot2.as_ref().unwrap();
                let target_axis = axis.cross(rot2.axis);
                let quat2 = Quat::from_axis_angle(target_axis, rot2.angle.to_radians());
                axis = quat2 * axis;
            }
        }
        return axis;
    }
    Vec3::ZERO
}

#[test]
fn test_parse_vector() {
    let test_str = "X30Y";
    let dir = parse_expr_to_dir(test_str);
    dbg!(dir);
    // let dir = parse_rotation_struct(test_str);
    //////dbg!(&dir);
    // //////dbg!(AXISES_MAP.get("-X"));
    // let mut  org_vec = Vec3::X;
    // let mut quat1 = Quat::from_axis_angle(Vec3::Y, 30.0f32.to_radians());
    // let mut quat2 = Quat::from_axis_angle(Vec3::Z, 30.0f32.to_radians());
    //
    // // let mut vec1 = quat2 * quat1 * org_vec;
    // let mut vec1 = quat2 * quat1 * org_vec;
    // //////dbg!(vec1);
    // let test_str = "-X(59)Y";
    // let res= parse_axis(test_str);
    // println!("{:?}",res);
}

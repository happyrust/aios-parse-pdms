use glam::{TransformSRT, Vec3};
use bevy::math::Quat;
use std::default::default;
use aios_core::prim_geo::category::CateBrepShape;
use aios_core::prim_geo::cylinder::SCylinder;

#[derive(Default, Debug, Clone)]
pub struct PdmsTubing{
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub bore: f32,
    pub finished: bool,  //完整的一个tubing信息
}


impl PdmsTubing{
    pub fn convert_to_shape(&self) -> CateBrepShape{
        let dir = (self.end_pt - self.start_pt).normalize();
        let mut cylinder = SCylinder{
            pdis: 0.0,
            phei: self.start_pt.distance(self.end_pt),
            pdia: self.bore,
            ..default()
        };

        CateBrepShape{
            brep_shape: Box::new(cylinder),
            transform: TransformSRT{
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation: (self.start_pt + self.end_pt) / 2.0,
                scale: Vec3::ONE,
            },
            visible: true,
            is_tubing: true,
        }
    }
}
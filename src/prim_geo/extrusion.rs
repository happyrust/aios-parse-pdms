use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hasher, Hash};
use bevy::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
// use bevy_inspector_egui::Inspectable;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use fixed::types::I24F8;
use log::kv::Source;
use crate::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::pdms_shape::{BrepMathTrait, BrepShape, hash_vec3, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect)]
#[reflect(Component)]
pub struct Extrusion {
    pub paax_expr: String,
    pub paax_pt: Vec3,   //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,   //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction, extru direction

    pub loop_verts: Vec<Vec3>, //loop vertex
    pub height: f32,
}



impl Default for Extrusion {
    fn default() -> Self {
        Self {
            paax_expr: "X".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::X,

            pbax_expr: "Z".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::Z,

            loop_verts: vec![Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 1.0), Vec3::new(0.0, 1.0, 1.0),  Vec3::new(0.0, 1.0, 2.0),  Vec3::new(0.0, 0.0, 2.0)],
            height: 5.0,
        }
    }
}

impl VerifiedShape for Extrusion {
    fn check_valid(&self) -> bool{
        self.height > std::f32::EPSILON
    }
}

impl BrepShape for Extrusion {

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        self.loop_verts.iter().for_each(|v|  {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        // hash_vec3::<DefaultHasher>(&self.pbax_dir, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        let unit = Self{
            loop_verts: self.loop_verts.clone(),
            height: 1.0,
            ..Default::default()
        };
        unit.gen_mesh(None)
    }

    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::new(1.0, 1.0, self.height)
    }


    fn gen_brep(& self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let mut wire = Wire::new();
        let ll = self.loop_verts.len();
        let mut verts: Vec<_> = self.loop_verts.iter().map(|x| builder::vertex(x.point3())).collect();
        for i in 0..ll {
            let cur_v = &verts[i];
            let next_v = &verts[(i+1)%ll];
            wire.push_back(builder::line(&cur_v, &next_v));
        }
        if let Ok(mut face) = builder::try_attach_plane(&[wire]){
            if let Surface::Plane(plane) = face.get_surface(){
                // let extrude_dir = self.pbax_dir.cross(self.paax_dir).normalize().vector3();
                let extrude_dir = self.pbax_dir.normalize().vector3();
                if plane.normal().dot(extrude_dir) < 0.0{
                    face = face.inverse();
                }
                let mut s = builder::tsweep(&face, extrude_dir * self.height as f64).into_boundaries();
                return s.pop();
            }
        }
        None
    }
}

impl From<AttrMap> for Extrusion {
    fn from(m: AttrMap) -> Self {
        Default::default()
    }
}
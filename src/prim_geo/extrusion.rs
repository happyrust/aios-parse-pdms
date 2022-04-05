use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
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
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};
use crate::tool::hash_tool::{hash_f32, hash_vec3};

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
    pub fradius_vec: Vec<f32>,
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

            loop_verts: vec![],
            fradius_vec: vec![],
            height: 1.0,
        }
    }
}

impl VerifiedShape for Extrusion {
    fn check_valid(&self) -> bool{
        self.height > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Extrusion {
    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let mut wire = Wire::new();
        let ll = self.loop_verts.len();
        let mut verts: Vec<_> = self.loop_verts.iter().map(|x| builder::vertex(x.point3())).collect();
        if ll < 3 {
            return None;
        }
        let mut pre_radius = 0.0;
        // let mut same_circle_pts = vec![];
        // dbg!(self);
        let mut i = 0;
        while i < ll {
            let cur_v = &verts[i];
            let next_v = &verts[(i+1)%ll];
            let fradius = self.fradius_vec[i];
            let next_fradius = self.fradius_vec[(i+1)%ll];
            let mut step = 1;
            if abs_diff_eq!(fradius, 0.0) && abs_diff_eq!(next_fradius, 0.0){
                wire.push_back(builder::line(cur_v, next_v));
                // dbg!(format!("Line from {:?} to {:?}", cur_v, next_v));
            }else if i <= ll - 2 {
                //最后一个点也就是回到原来的点
                //处理最后一个点和起点相同的情况
                let mut nnext_fradius = self.fradius_vec[(i+step)%ll];
                while abs_diff_eq!(nnext_fradius, next_fradius) && (i+step <= ll) {
                    step += 1;
                    nnext_fradius = self.fradius_vec[(i+step)%ll];
                };
                // dbg!(step);
                if step == (ll + 1) || step == ll{ //首尾相接的情况， 特殊处理
                    let j = if abs_diff_ne!(fradius, next_fradius){
                        1
                    }else{
                        0
                    };
                    let c_num = ll / 2 + j;
                    // dbg!(c_num);
                    wire.push_back(builder::circle_arc(&verts[j], &verts[c_num], verts[c_num-1].get_point()));
                    // dbg!(format!("Circle from {:?} to {:?}",&verts[j], &verts[c_num]));
                    wire.push_back(builder::circle_arc(&verts[c_num], &verts[j], verts[c_num+1].get_point()));
                    // dbg!(format!("Circle from {:?} to {:?}", &verts[c_num], &verts[j]));
                }else{
                    let k = (i+step)%ll;
                    let mut nnext_v = &verts[k];
                    let v1 = self.loop_verts[i];
                    let v2 = self.loop_verts[k];
                    let v3 = self.loop_verts[(i+k)/2].normalize();
                    let transit_pt = v3 * next_fradius;
                    let angle = v1.angle_between(v2);
                    // dbg!(angle);
                    // dbg!(&nnext_v);
                    // dbg!(transit_pt);
                    wire.push_back(builder::circle_arc(cur_v, nnext_v, transit_pt.point3()));
                    // wire.push_back(builder::circle_arc_with_center(
                    //     Point3::new(0.0, 0.0, 0.0), cur_v, nnext_v, Vec3::Z.vector3(), Rad(angle as f64)));
                    // dbg!(format!("Circle from {:?} to {:?}", cur_v, nnext_v));
                }
            }
            i += step;
        }
        // builder::try_attach_plane(&[wire.clone()]).unwrap();
        if let Ok(mut face) = builder::try_attach_plane(&[wire]){
            if let Surface::Plane(plane) = face.get_surface(){
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

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        self.loop_verts.iter().for_each(|v|  {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        self.fradius_vec.iter().for_each(|v|  {
            hash_f32::<DefaultHasher>(v, &mut hasher);
        });
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        let unit = Self{
            loop_verts: self.loop_verts.clone(),
            height: 10.0,   //开放一点大小，不然三角化出来的不对
            fradius_vec: self.fradius_vec.clone(),
            ..Default::default()
        };
        unit.gen_mesh(Some(0.0005))
    }


    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3{
        // Vec3::new(1.0, 1.0, self.height)
        Vec3::new(1.0, 1.0, self.height/10.0)
    }
}

impl From<AttrMap> for Extrusion {
    fn from(m: AttrMap) -> Self {
        Default::default()
    }
}
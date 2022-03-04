use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;
use std::hash::Hash;
use bevy::prelude::*;
use truck_modeling::{builder, Shell};
// use bevy_inspector_egui::Inspectable;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use fixed::types::I24F8;
use log::kv::Source;
use crate::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, rotate_from_vec3_to_vec3};
use crate::shape::pdms_shape::{BrepMathTrait, BrepShape, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, Clone,  Reflect)]
#[reflect(Component)]
pub struct SRTorus {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pheig: f32,
    pub pdia: f32,

}


impl Default for SRTorus {
    fn default() -> Self {
        Self {
            paax_expr: "X".to_string(),
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::X,

            pbax_expr: "Y".to_string(),
            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::Y,
            pheig: 2.0,
            pdia: 2.0,

            // center: Default::default(),
            // angle: 0.0,
            // rot_axis: Default::default(),
            // radius: 0.0,
        }
    }
}

#[derive(Default)]
struct TorusInfo{
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl SRTorus {
    fn cal_torus(&self) -> Option<TorusInfo> {
        let mut torus_info = TorusInfo::default();
        let pa_dir = Vec3::new(self.paax_dir.x, self.paax_dir.y, self.paax_dir.z).normalize();
        let pb_dir = Vec3::new(self.pbax_dir.x, self.pbax_dir.y, self.pbax_dir.z).normalize();
        let x_dir = (self.pbax_pt - self.paax_pt).normalize();
        // let dir = (x_dir);
        let quat = rotate_from_vec3_to_vec3(x_dir, -pa_dir, pb_dir);
        let (mut axis_z, angle) = quat.to_axis_angle();
        torus_info.rot_axis = axis_z;
        torus_info.angle = angle.to_degrees();
        // godot_dbg!(self.angle);
        let mid_pt = (self.paax_pt + self.pbax_pt) / 2.0;
        let x_len = x_dir.length();
        if x_len < 1.0e-3 {
            return None;
        }
        if (torus_info.angle - std::f32::consts::PI).abs() < 1.0e-3 {
            torus_info.center = mid_pt;
            torus_info.radius = x_len / 2.0;
        } else {
            let mut y_dir = torus_info.rot_axis.cross(x_dir);
            let ref_dir = torus_info.rot_axis.cross(self.pbax_dir.normalize());
            let p = self.pbax_pt - mid_pt;
            let px = p.dot(x_dir);
            let _py = p.dot(y_dir);
            if px < 1.0e-3 {
                return None;
            }
            let beta = torus_info.angle.to_radians() / 2.0;
            torus_info.radius = px / beta.sin().abs();
            torus_info.center = self.pbax_pt + ref_dir * torus_info.radius;
        }
        return Some(torus_info);
    }
}

impl VerifiedShape for SRTorus {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShape for SRTorus {

    fn gen_brep(& self) -> Option<Shell> {
        if let Some(torus_info) = self.cal_torus(){
            use truck_modeling::*;
            let circle_origin = self.paax_pt.point3();
            let z_axis = self.paax_dir.normalize().vector3();
            let y_axis = torus_info.rot_axis.vector3();
            let x_axis = z_axis.cross(y_axis);
            let h = self.pheig as f64;
            let d = self.pdia as f64;
            let p0 = self.paax_pt.point3() - y_axis * h / 2.0 - x_axis * d / 2.0;
            let v = builder::vertex(p0);
            let e = builder::tsweep(&v, y_axis * h as f64);
            let f = builder::tsweep(&e, x_axis * d as f64);
            let center = torus_info.center.point3();
            let mut solid = builder::rsweep(&f, center, -y_axis, Rad(torus_info.angle.to_radians() as f64)).into_boundaries();
            return solid.pop();
        }
        None
    }
}

impl From<AttrMap> for SRTorus {
    fn from(_: AttrMap) -> Self {
        Default::default()
    }
}

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect)]
pub struct RTorus {
    pub rins: f32,   //内圆半径
    pub rout: f32,  //外圆半径
    pub height: f32,
    pub angle: f32,  //旋转角度
}

impl Default for RTorus {
    fn default() -> Self {
        Self{
            rins: 0.5,
            rout: 1.0,
            height: 1.0,
            angle: 90.0,
        }
    }
}

impl VerifiedShape for RTorus {
    #[inline]
    fn check_valid(&self) -> bool {
        self.rout > 0.0 && self.angle.abs() > 0.0 && (self.rout - self.rins) > EPSILON && self.height > EPSILON
    }
}

impl BrepShape for RTorus {

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        let rins = I24F8::from_num(self.rins / self.rout);
        let beta = I24F8::from_num(self.angle);
        rins.hash(&mut hasher);
        beta.hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        let rins = self.rins / self.rout;
        let unit = Self{
            rins,
            rout: 1.0,
            height: 1.0,
            angle: self.angle
        };
        unit.gen_mesh(Some(0.002))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::new(self.rout, self.rout, self.height)
    }

    fn gen_brep(& self) -> Option<Shell> {
        use truck_modeling::*;

        let h = self.height as f64;
        let d = (self.rout - self.rins) as f64;
        let p0 = Point3::new(self.rins as f64, 0.0, 0.0);
        let v = builder::vertex(p0);
        let e = builder::tsweep(&v, Vector3::new(0.0, 0.0, h));
        let f = builder::tsweep(&e,  Vector3::new(d, 0.0, 0.0));

        let mut solid = builder::rsweep(&f, Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0), Rad(self.angle.to_radians() as f64)).into_boundaries();
        return solid.pop();
    }
}

impl From<&AttrMap> for RTorus {
    fn from(m: &AttrMap) -> Self {
        let rins = m.get_f32("RINS").unwrap();
        let rout = m.get_f32("ROUT").unwrap() ;
        let height = m.get_f32("HEIG").unwrap();
        let angle = m.get_f32("ANGL").unwrap();
        RTorus {
            rins,
            rout,
            height,
            angle,
        }
    }
}

impl From<AttrMap> for RTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


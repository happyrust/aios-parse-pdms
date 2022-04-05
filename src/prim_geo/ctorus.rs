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
use nalgebra_glm::normalize;
use crate::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, rotate_from_vec3_to_vec3};
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct SCTorus {
    pub paax_expr: String,
    pub paax_pt: Vec3,   //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,   //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pdia: f32,
}

#[derive(Default)]
struct TorusInfo{
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl SCTorus {
    fn cal_torus(&self) -> Option<TorusInfo> {
        let mut torus_info = TorusInfo::default();
        let pa_dir = Vec3::new(self.paax_dir.x, self.paax_dir.y, self.paax_dir.z).normalize();
        let pb_dir = Vec3::new(self.pbax_dir.x, self.pbax_dir.y, self.pbax_dir.z).normalize();
        let x_dir = (self.pbax_pt - self.paax_pt).normalize();
        let quat = rotate_from_vec3_to_vec3(x_dir, -pa_dir, pb_dir);
        let (mut axis_z, angle) = quat.to_axis_angle();
        torus_info.rot_axis = axis_z;
        torus_info.angle = angle.to_degrees();
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
            let ref_dir = torus_info.rot_axis.cross(self.pbax_dir.normalize()).normalize();
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

    pub fn convert_to_ctorus(&self) -> Option<(CTorus, glam::TransformSRT)>{
        if let Some(torus_info) = self.cal_torus(){
            let mut ctorus = CTorus::default();
            ctorus.angle = torus_info.angle;
            ctorus.rins = torus_info.radius - self.pdia/2.0;
            ctorus.rout = torus_info.radius + self.pdia/2.0;
            let z_axis = -torus_info.rot_axis.normalize();
            let x_axis = (self.pbax_pt - torus_info.center).normalize();
            let y_axis = z_axis.cross(x_axis).normalize();
            let mat = glam::TransformSRT{
                rotation: bevy::prelude::Quat::from_mat3(&bevy::prelude::Mat3::from_cols(
                    x_axis, y_axis, z_axis
                )),
                translation: torus_info.center,
                ..default()
            };

            return  Some((ctorus, mat));
        }

        None
    }

}


impl Default for SCTorus {
    fn default() -> Self {
        SCTorus {
            paax_expr: "X".to_string(),
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::new(1.0,0.0,0.0),//Down

            pbax_expr: "Y".to_string(),
            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::new(0.0,1.0,0.0), //UP
            pdia: 2.0,

        }
    }
}

impl VerifiedShape for SCTorus {
    fn check_valid(&self) -> bool {
        true
    }
}

impl BrepShapeTrait for SCTorus {

    fn gen_brep_shell(& self) -> Option<Shell> {
        use truck_modeling::*;
        if let Some(torus_info) = self.cal_torus(){
            let circle_origin = self.paax_pt.point3();
            let pt_0 = self.paax_pt + torus_info.rot_axis * self.pdia / 2.0;
            let v = builder::vertex(pt_0.point3());
            let rot_axis = torus_info.rot_axis.vector3();

            let w = builder::rsweep(
                &v,
                circle_origin,
                -self.paax_dir.normalize().vector3(),
                Rad(7.0),
            );
            if let Ok(disk) = builder::try_attach_plane(&vec![w]) {
                let center = torus_info.center.point3();
                let mut solid = builder::rsweep(&disk, center, rot_axis, Rad(torus_info.angle.to_radians() as f64)).into_boundaries();
                return solid.pop()
            }
        }
        None
    }
}

impl From<AttrMap> for SCTorus {
    fn from(m: AttrMap) -> Self {
       Default::default()
    }
}



#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect)]
pub struct CTorus {
    pub rins: f32,   //内圆半径
    pub rout: f32,  //外圆半径
    pub angle: f32,  //旋转角度
}

impl Default for CTorus {
    fn default() -> Self {
        Self{
            rins: 0.5,
            rout: 1.0,
            angle: 90.0,
        }
    }
}

impl VerifiedShape for CTorus {
    fn check_valid(&self) -> bool {
        self.rout > 0.0 && self.angle.abs() > 0.0 && (self.rout - self.rins) > EPSILON
    }
}

impl BrepShapeTrait for CTorus {
    fn gen_brep_shell(& self) -> Option<Shell> {
        use truck_modeling::*;

        let radius = ((self.rout - self.rins) /2.0) as f64;
        if radius <= 0.0 { return None; }
        let circle_origin = Point3::new(self.rins as f64 + radius, 0.0, 0.0);
        let v = builder::vertex(Point3::new(self.rout as f64, 0.0, 0.0));
        let w = builder::rsweep(
            &v,
            circle_origin,
            Vector3::new(0.0, 1.0, 0.0),
            Rad(7.0),
        );
        if let Ok(disk) = builder::try_attach_plane(&vec![w]) {
            let mut solid = builder::rsweep(&disk, Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0), Rad(self.angle.to_radians() as f64)).into_boundaries();
            return solid.pop()
        }
        None
    }

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        let rins = I24F8::from_num(self.rins / self.rout);
        let angle = I24F8::from_num(self.angle);
        // let rout = I24F8::from_num(self.rout);
        rins.hash(&mut hasher);
        angle.hash(&mut hasher);
        // rout.hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        let rins = self.rins / self.rout;
        let unit = Self{
            rins,
            rout: 1.0,
            angle: self.angle
        };
        unit.gen_mesh(Some(0.001))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::splat(self.rout)
    }
}

impl From<&AttrMap> for CTorus {
    fn from(m: &AttrMap) -> Self {
        let r_i = m.get_val("RINS").unwrap().double_value().unwrap() as f32 ;
        let r_o = m.get_val("ROUT").unwrap().double_value().unwrap() as f32 ;
        let angle = m.get_val("ANGL").unwrap().double_value().unwrap() as f32 ;
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
        }
    }
}

impl From<AttrMap> for CTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}
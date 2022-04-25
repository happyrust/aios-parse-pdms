use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;
use bevy::prelude::*;
use truck_modeling::{builder, Shell};
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use crate::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};
use crate::tool::hash_tool::{hash_f32, hash_vec3};

#[derive(Component, Debug, /*Inspectable,*/ Reflect, Clone, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct SlopeCylinder {
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub shear_x_top: f32,
    pub shear_x_bottom: f32,

    pub shear_y_top: f32,
    pub shear_y_bottom: f32,

    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32, //diameter
}

impl Default for SlopeCylinder {
    fn default() -> Self {
        Self {
            paxi_dir: Vec3::Z,
            shear_x_top: 0.0,
            shear_x_bottom: 0.0,
            shear_y_top: 0.0,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            shear_y_bottom: 0.0,
        }
    }
}

impl VerifiedShape for SlopeCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for SlopeCylinder {
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        let c_pt =  self.paxi_pt;
        let origin = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.phei;
        let mut ext_dir = dir.vector3();
        let mut reverse_dir = false;
        if ext_len < 0.0 {
            reverse_dir = true;
        }
        let v = builder::vertex(pt0.point3());
        let w_origin = builder::rsweep(&v, origin, ext_dir, Rad(7.0));

        //先变换出top 的 w_t
        let angle_top_x = Rad(self.shear_x_top.to_radians() as f64);
        let angle_top_y = Rad(self.shear_y_top.to_radians() as f64);
        let mat0 = Matrix4::from_translation(-origin.to_vec());
        let mat1 = Matrix4::from_axis_angle(Vector3::unit_x(), angle_top_y) * Matrix4::from_axis_angle(Vector3::unit_y(), angle_top_x);
        let mat2 = Matrix4::from_translation(-origin.to_vec() + Vector3::unit_z() * self.phei as f64);
        let w_t = builder::transformed(&w_origin, mat2 * mat1 * mat0);

        //buttom
        let angle_btm_x = Rad(self.shear_x_bottom.to_radians() as f64);
        let angle_btm_y = Rad(self.shear_y_bottom.to_radians() as f64);
        let mat0 = Matrix4::from_translation(-origin.to_vec());
        let mat1 = Matrix4::from_axis_angle(Vector3::unit_x(), angle_btm_y) * Matrix4::from_axis_angle(Vector3::unit_y(), angle_btm_x);
        let mat2 = Matrix4::from_translation(-origin.to_vec());
        let w_b = builder::transformed(&w_origin, mat2 * mat1 * mat0);

        let mut wt_clone = w_t.clone();
        let mut wb_clone = w_b.clone();

        let new_wire_1 = wb_clone.split_off((0.5 * wb_clone.len() as f32) as usize);
        let new_wire_2 = wt_clone.split_off((0.5 * wt_clone.len() as f32) as usize);
        let face1 = builder::homotopy(new_wire_1.front().unwrap(), &new_wire_2.front().unwrap());
        let face2 = builder::homotopy(wb_clone.front().unwrap(), &wt_clone.front().unwrap());

        let disk1 = builder::try_attach_plane(&vec![w_b.inverse()]).unwrap();
        let disk2 = builder::try_attach_plane(&vec![w_t]).unwrap();
        let shell = Shell::from(vec![disk1, face1, face2, disk2]);

        Some(shell)
    }

    fn hash_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_f32(self.shear_x_bottom, &mut hasher);
        hash_f32(self.shear_y_bottom, &mut hasher);
        hash_f32(self.shear_x_top, &mut hasher);
        hash_f32(self.shear_y_top, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh {
        Self {
            shear_x_top: self.shear_x_top,
            shear_x_bottom: self.shear_x_bottom,
            shear_y_top: self.shear_y_top,
            shear_y_bottom: self.shear_y_bottom,
            phei: 1.0,
            pdia: 1.0,
            ..default()
        }.gen_mesh(Some(0.001))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, self.phei.abs())
    }
}

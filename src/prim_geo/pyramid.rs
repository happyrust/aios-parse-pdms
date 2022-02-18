use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hasher, Hash};
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use truck_modeling::{builder, Shell, Surface, Wire};
// use bevy_inspector_egui::Inspectable;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use fixed::types::I24F8;
use glam::Vec3;
use log::kv::Source;
use truck_modeling::builder::try_attach_plane;
use crate::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::pdms_shape::{BrepMathTrait, BrepShape, hash_f32, hash_vec3, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct LPyramid {
    pub pbax_expr: String,
    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pcax_expr: String,
    pub pcax_pt: Vec3,
    //B Axis point
    pub pcax_dir: Vec3,   //B Axis Direction

    pub paax_expr: String,
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction


    pub pbtp: f32,
    pub pctp: f32,

    pub pbbt: f32,
    pub pcbt: f32,

    pub ptdi: f32,
    pub pbdi: f32,

    pub pbof: f32,
    pub pcof: f32,
}

impl Default for LPyramid {
    fn default() -> Self {
        Self {
            pbax_expr: "X".to_string(),  //todo 方位都想方法设法还原到原点坐标系
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_expr: "Y".to_string(),
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: 1.0,
            pctp: 1.0,
            pbbt: 1.0,
            pcbt: 1.0,
            ptdi: 1.0,
            pbdi: 0.0,
            pbof: 0.0,
            pcof: 0.0,
        }
    }
}

impl VerifiedShape for LPyramid {
    fn check_valid(&self) -> bool { true }
}

impl BrepShape for LPyramid {
    fn hash_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let r = vec![self.pbtp,
                     self.pctp,
                     self.pbbt,
                     self.pcbt,
                     self.ptdi,
                     self.pbdi,
                     self.pbof, ];
        for v in r {
            hash_f32::<DefaultHasher>(&v, &mut hasher);
        }
        hasher.finish()
    }

    //暂时不做可拉伸
    fn gen_unit_shape(&self) -> PdmsMesh {
        self.gen_mesh(None)
    }

    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    fn gen_brep(&self) -> Option<Shell> {
        use truck_modeling::*;
        let x_dir = self.pbax_dir.normalize().vector3();
        //暂时没用到 x y 的点信息
        let z_dir = self.paax_dir.normalize().vector3();
        let y_dir = z_dir.cross(x_dir);
        let z_pt = self.paax_pt.point3();

        let t_x = self.pbtp as f64 / 2.0;
        let t_y = self.pctp as f64 / 2.0;
        if t_x * t_y <= f64::EPSILON {
            return None;
        }
        let b_x = self.pbbt as f64 / 2.0;
        let b_y = self.pcbt as f64 / 2.0;
        //todo 暂时不考虑这种情况, 退化成一条边和一点的情况
        if b_x * b_y <= f64::EPSILON {
            return None;
        }
        let btm_center = z_pt + z_dir * self.pbdi as f64;
        let top_center = z_pt + z_dir * self.ptdi as f64 + x_dir * self.pbof as f64 + y_dir * self.pcof as f64;
        let len = btm_center.distance(top_center);
        let b1 = x_dir * b_x;
        let b2 = y_dir * b_y;
        //bottom points
        let mut bts = Vec::with_capacity(4);
        let mut ebs = Vec::with_capacity(4);
        bts.push(builder::vertex(btm_center - b1 - b2));   //if b1 = 0 && b2 = 0
        if b_x.abs() >= f64::EPSILON {
            bts.push(builder::vertex(btm_center + b1 - b2));
            ebs.push(builder::line(&bts[0], &bts[1]));
        }
        if b_y.abs() >= f64::EPSILON {
            bts.push(builder::vertex(btm_center + b1 + b2));
            bts.push(builder::vertex(btm_center - b1 + b2));

            ebs.push(builder::line(&bts[1], &bts[2]));
            ebs.push(builder::line(&bts[2], &bts[3]));
            ebs.push(builder::line(&bts[3], &bts[0]));
        }

        let t1 = x_dir * t_x;
        let t2 = y_dir * t_y;
        //top points
        let mut tts = Vec::with_capacity(4);
        let mut ets = Vec::with_capacity(4);
        tts.push(builder::vertex(top_center - t1 - t2));
        if t_x.abs() >= f64::EPSILON {
            tts.push(builder::vertex(top_center + t1 - t2));
            ets.push(builder::line(&tts[0], &tts[1]));
        }
        if t_y.abs() >= f64::EPSILON {
            tts.push(builder::vertex(top_center + t1 + t2));
            tts.push(builder::vertex(top_center - t1 + t2));

            ets.push(builder::line(&tts[1], &tts[2]));
            ets.push(builder::line(&tts[2], &tts[3]));
            ets.push(builder::line(&tts[3], &tts[0]));
        }


        let mut wires = vec![];

        if ebs.len() == 4 {
            wires.push(Wire::from_iter(&ebs));
        }
        if ets.len() == 4 {
            wires.push(Wire::from_iter(&ets).inverse());
        }

        let mut shell: Shell = wires.into_iter().map(|w| try_attach_plane(&[w]).unwrap()).collect();
        if ebs.len() == 4 && ets.len() == 4 {
            shell.push(builder::homotopy(&ebs[0], &ets[0]));
            shell.push(builder::homotopy(&ebs[1], &ets[1]));
            shell.push(builder::homotopy(&ebs[2], &ets[2]));
            shell.push(builder::homotopy(&ebs[3], &ets[3]));
        }
        //
        // else if ebs.len() == 2 && ets.len() == 4 {
        //     shell.push(builder::homotopy(&ebs[0], &ets[0]));
        //     shell.push(builder::homotopy(&ebs[0], &ets[2]));
        // }

        Some(shell)
    }
}

impl From<&AttrMap> for LPyramid {
    fn from(m: &AttrMap) -> Self {
        let xbot = m.get_val("XBOT").unwrap().f32_value().unwrap_or_default();
        let ybot = m.get_val("YBOT").unwrap().f32_value().unwrap_or_default();

        let xtop = m.get_val("XTOP").unwrap().f32_value().unwrap_or_default();
        let ytop = m.get_val("YTOP").unwrap().f32_value().unwrap_or_default();

        let xoff = m.get_val("XOFF").unwrap().f32_value().unwrap_or_default();
        let yoff = m.get_val("YOFF").unwrap().f32_value().unwrap_or_default();

        let height = m.get_val("HEIG").unwrap().f32_value().unwrap_or_default();


        LPyramid {
            pbax_expr: "X".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_expr: "Y".to_string(),
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: xtop,
            pctp: ytop,
            pbbt: xbot,
            pcbt: ybot,
            ptdi: height/2.0,
            pbdi: -height/2.0,
            pbof: xoff,
            pcof: yoff,
        }
    }
}

impl From<AttrMap> for LPyramid {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

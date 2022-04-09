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

use crate::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, RotateInfo};
use crate::shape::pdms_shape::{BevyMathTrait, BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};
use crate::tool::hash_tool::{hash_f32, hash_vec3};

#[derive(Component, Debug, /*Inspectable,*/ Clone, Reflect)]
#[reflect(Component)]
pub struct Extrusion {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction, extru direction

    pub loop_verts: Vec<Vec3>,
    //loop vertex
    pub fradius_vec: Vec<f32>,
    pub height: f32,
}

fn get_vec3_hash(v: &Vec3) -> u64{
    let mut hasher = DefaultHasher::new();
    hash_vec3::<DefaultHasher>(v, &mut hasher);
    hasher.finish()
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
    fn check_valid(&self) -> bool {
        self.height > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Extrusion {
    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let mut wire = Wire::new();
        if self.loop_verts.len() < 3 {
            return None;
        }
        //dbg!(self.loop_verts.len());
        let mut new_verts = self.loop_verts.iter().map(|x| Vec3::new(x.x, x.y, 0.0)).collect::<Vec<_>>();
        let mut pre_hash = 0;
        if get_vec3_hash(&new_verts[0]) == get_vec3_hash(new_verts.last().unwrap()) {
            //dbg!("Start end is the same, neet to puge the last");
            new_verts.remove(new_verts.len() - 1);
        }
        new_verts.retain(|x|{
            let hash = get_vec3_hash(x);
            let retain = pre_hash != hash;
            pre_hash = hash;
            retain
        });
        // //dbg!(new_verts.len());
        // //dbg!(self);
        let ll = new_verts.len();
        // let mut verts: Vec<_> = self.loop_verts.iter().map(|x| builder::vertex(x.point3())).collect();
        if ll < 3 {
            return None;
        }
        let mut pre_radius = 0.0;
        let mut i = 1;
        let r = self.fradius_vec[0];
        let origin_vert = if abs_diff_eq!(r, 0.0) {
            builder::vertex(new_verts[0].point3())
        } else {
            let v = &new_verts;
            let pbax_dir = (v[1] - v[0]).normalize();
            let paax_dir = (v[ll - 1] - v[0]).normalize();
            let angle = paax_dir.angle_between(pbax_dir) / 2.0;
            if abs_diff_eq!(angle, 0.0) { return None; }
            let b_len = r / angle.tan();
            // //dbg!(b_len);
            let pbax_pt = v[0] + pbax_dir * b_len;
            builder::vertex(pbax_pt.point3())
        };
        // //dbg!(&origin_vert);
        let mut pre_vert = origin_vert.clone();
        //从下一个点开始
        for i in 1..=ll {
            let cur_pt = &new_verts[i % ll];
            //如果点重合了，需要跳过
            if pre_vert.get_point().vec3().distance(*cur_pt) <= 0.01 {
                continue;
            }
            let fradius = self.fradius_vec[i % ll];
            if abs_diff_eq!(fradius, 0.0){
                let cur_vert = if i != ll { builder::vertex(cur_pt.point3()) } else { origin_vert.clone() };
                if pre_vert.get_point().distance(cur_vert.get_point()) > 0.01 {
                    wire.push_back(builder::line(&pre_vert, &cur_vert));
                    // //dbg!(format!("Line from {:?} to {:?}", pre_vert, cur_vert));
                    pre_vert = cur_vert.clone();
                }
            } else {
                //实现fillet 绘制
                let r = fradius;
                let pre_i = i - 1;
                let n_i = (i + 1) % ll;
                let pre_pt = new_verts[pre_i];
                let cur_pt = new_verts[i % ll];
                let next_pt = new_verts[n_i];


                let pa_dist = pre_pt.distance(cur_pt);
                let pb_dist = next_pt.distance(cur_pt);
                let paax_dir = (pre_pt - cur_pt).normalize();
                let pbax_dir = (next_pt - cur_pt).normalize();
                let angle = paax_dir.angle_between(pbax_dir) / 2.0;
                // //dbg!(angle);
                if abs_diff_eq!(angle, 0.0) { return None; }
                let b_len = r / angle.tan();
                // //dbg!(b_len);

                //不能超过最小的长度，不然这个fillet肯定不合适
                // //dbg!(pa_dist);
                // //dbg!(pb_dist);
                // //dbg!(cur_pt);
                //
                // //dbg!(r);
                if b_len - pa_dist.min(pb_dist) > 0.01 {
                    let cur_vert = if i != ll { builder::vertex(cur_pt.point3()) } else { origin_vert.clone() };
                    wire.push_back(builder::line(&pre_vert, &cur_vert));
                    // //dbg!(format!("Line from {:?} to {:?}", pre_vert, cur_vert));
                    pre_vert = cur_vert.clone();
                    continue;
                }


                let paax_pt = cur_pt + paax_dir * b_len;
                let pbax_pt = cur_pt + pbax_dir * b_len;

                // //dbg!(paax_dir);
                // //dbg!(pbax_dir);
                // //dbg!(paax_pt);
                // //dbg!(pbax_pt);

                let mut t_va = pre_vert.clone();
                let mut va = builder::vertex(paax_pt.point3());
                let mut t_vb = builder::vertex(pbax_pt.point3());
                // //dbg!(&pre_pt);
                // //dbg!(paax_pt);
                if paax_pt.distance(pre_vert.get_point().vec3()) >= 0.01 {
                    // //dbg!(paax_pt.distance(pre_pt));
                    t_va = va.clone();
                    wire.push_back(builder::line(&pre_vert, &t_va));
                    // //dbg!(format!("Line from {:?} to {:?}", &pre_vert, &t_va));
                }

                let origin_dist = pbax_pt.distance(origin_vert.get_point().vec3());
                if origin_dist < 0.01 {
                    t_vb = origin_vert.clone();
                }
                //连成圆弧
                let dot_value = paax_dir.cross(pbax_dir).dot(Vec3::Z);
                let rot_angle = if dot_value > 0.0 { angle } else { -angle };
                let c_dir = Quat::from_rotation_z(rot_angle).mul_vec3(paax_dir).normalize();
                // //dbg!(c_dir);
                // //dbg!(cur_pt);
                let center = cur_pt +  c_dir * (r / angle.sin());
                // //dbg!(center);
                let beta = (PI - 2.0 * angle);
                // //dbg!(beta);
                wire.push_back(builder::circle_arc_with_center(
                    center.point3(), &t_va, &t_vb, Vec3::Z.vector3(), Rad(beta as f64)));
                // //dbg!(format!("Circle from {:?} to {:?}", &t_va, &t_vb));

                //提前结束
                if origin_dist < 0.01 {
                    break;
                }
                if i == ll {
                    if origin_dist >= 0.01 {
                        wire.push_back(builder::line(&t_vb, &origin_vert));
                        // //dbg!(format!("Line from {:?} to {:?}", &t_vb, &origin_vert));
                    }
                }
                pre_vert = t_vb.clone();
            }
        }
        // builder::try_attach_plane(&[wire.clone()]).unwrap();
        if let Ok(mut face) = builder::try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = face.get_surface() {
                let extrude_dir = self.pbax_dir.normalize().vector3();
                if plane.normal().dot(extrude_dir) < 0.0 {
                    face = face.inverse();
                }
                let mut s = builder::tsweep(&face, extrude_dir * self.height as f64).into_boundaries();
                return s.pop();
            }
        } else {
            //dbg!(self);
        }
        None
    }



    fn hash_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.loop_verts.iter().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        self.fradius_vec.iter().for_each(|v| {
            hash_f32::<DefaultHasher>(v, &mut hasher);
        });
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh {
        let unit = Self {
            loop_verts: self.loop_verts.clone(),
            height: 10.0,   //开放一点大小，不然三角化出来的不对
            fradius_vec: self.fradius_vec.clone(),
            ..Default::default()
        };
        unit.gen_mesh(Some(0.0005))
    }


    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3 {
        // Vec3::new(1.0, 1.0, self.height)
        Vec3::new(1.0, 1.0, self.height / 10.0)
    }
}

impl From<AttrMap> for Extrusion {
    fn from(m: AttrMap) -> Self {
        Default::default()
    }
}
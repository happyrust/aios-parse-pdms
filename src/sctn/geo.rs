use std::f32::EPSILON;
use crate::{AttrMap, GeomsInfo};
use crate::parsed_data::CateProfileParam;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::GeoData;
use crate::prim_geo::loft::SctnSolid;
use crate::shape::pdms_shape::BrepShapeTrait;
use std::vec::Vec;
use bevy::prelude::Transform;
use glam::{TransformSRT, Vec3};
use crate::data_interface::PdmsDataInterface;
use crate::prim_geo::category::CateBrepShape;

pub async fn create_geos<T: PdmsDataInterface>(att: &AttrMap, geom_info: &GeomsInfo, interface: &T) -> Vec<CateBrepShape>  {
    let mut brep_shapes = vec![];
    let geoms = &geom_info.geometries;
    if geoms.len() == 0 { return brep_shapes; }

    let type_name = att.get_type();
    let arc_path = if type_name == "GENSEC" {
        let parent_pos = interface.get_ele_world_transform_async(att.get_refno().unwrap()).await.translation;
        dbg!(parent_pos);
        let children_hash = interface.get_ele_children_refs_async(att.get_refno().unwrap()).await;
        let mut res = None;
        for x in children_hash {
            let refs = interface.get_ele_children_refs_async(x).await;
            if refs.len() >= 3 {
                res = Some((
                    interface.get_ele_world_transform_async(refs[0]).await.translation - parent_pos,
                    interface.get_ele_world_transform_async(refs[1]).await.translation - parent_pos,
                    interface.get_ele_world_transform_async(refs[2]).await.translation - parent_pos,
                ));
            }
        }
        if res.is_some() {
            dbg!(&res);
        }
        res
    } else { None };

    let mut height = 0.0;
    if let Some(poss) = att.get_poss() {
        if let Some(pose) = att.get_pose() {
            height = pose.distance(poss);
        }
    }
    let drns = att.get_vec3("DRNS").unwrap_or_default();
    let drne = att.get_vec3("DRNE").unwrap_or_default();
    //rotate the profile
    for (i, geom) in geoms.iter().enumerate() {
        if let CateGeoParam::Profile(profile) = geom{
            let loft = SctnSolid {
                profile: profile.clone(),
                drns,
                drne,
                height,
                arc_path,
            };
            brep_shapes.push(CateBrepShape{
                brep_shape: Box::new(loft),
                transform: TransformSRT::IDENTITY,
                visible: true,
                is_tubing: false
            });
        }
    }

    brep_shapes
}
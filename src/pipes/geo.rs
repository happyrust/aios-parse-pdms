use std::f32::EPSILON;
use crate::{AttrMap, GeomsInfo};
use crate::parsed_data::CateProfileParam;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::GeoData;

//sctn 的hash 函数，需要涵盖截面的旋转


pub fn create_geo(att: &AttrMap, geom_info: &GeomsInfo) -> Option<GeoData> {

    let geoms = &geom_info.geometries;
    if geoms.len() < 2 { return None; }

    if let Some(poss) = att.get_poss() {
        if let Some(pose) = att.get_pose() {

            let height = pose.distance(poss);
            if height < EPSILON { return None; }

            let ns = att.get_vec3("DRNS");
            let ne = att.get_vec3("DRNE");

            //rotate the profile
            if let CateGeoParam::Profile(profile_s) = &geoms[0]{
                if let CateGeoParam::Profile(profile_e) = &geoms[1] {
                    match (profile_s, profile_e) {
                        (CateProfileParam::SANN(p_s), CateProfileParam::SANN(p_e)) =>{

                        }
                        (CateProfileParam::SPRO(p_s), CateProfileParam::SPRO(p_e)) =>{

                        }
                        (_, _) => {}
                    }
                }
            }



        }
    }



    // if let Some(geoms) = crate::query_cata::resolve_desi_comp(&refno, self).await {
    //     dbg!(&geoms);
    //     if geoms.geometries.len() == 0 { return None; }
    //     if let Some(poss) = desi_att.get_poss() {
    //         if let Some(pose) = desi_att.get_pose() {
    //             let height = pose.distance(poss);
    //             //这里需要加入一个旋转调整
    //             if let CateGeoParam::Profile(CateProfileParam::SPRO(profile)) = &geoms.geometries[0] {
    //                 let loop_verts = profile.iter().map(|x| Vec3::new(x[0], x[1], 0.0)).collect();
    //                 if height.abs() >= f32::EPSILON {
    //                     let extrusion = Box::new(Extrusion {
    //                         loop_verts,
    //                         height,
    //                         ..Default::default()
    //                     });
    //                     if extrusion.check_valid() {
    //                         let r = cached_mesh_mgr.get_pdms_mesh_hash_key(extrusion);
    //                         return Some((GeoData::Primitive(r)));
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    None

}
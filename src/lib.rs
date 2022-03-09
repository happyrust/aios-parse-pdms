#![feature(type_ascription)]
#![feature(array_methods)]
#![feature(slice_pattern)]
#![feature(core_intrinsics)]
#![feature(associated_type_bounds)]
#![feature(once_cell)]
#![feature(async_closure)]
#![feature(generic_const_exprs)]
#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]

#[macro_use]
extern crate serde;
#[macro_use]
extern crate approx;
#[macro_use]
extern crate hash32_derive;
extern crate hash32;

use mongodb::Client;
use mongodb::bson::doc;
use std::collections::HashSet;
use std::error::Error;
use crate::parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, EleNode, PdmsRefno, RefU64};
use futures::stream::TryStreamExt;
// use crate::interface::pdms_interface::PdmsInterface;


pub mod pdms_types;
pub mod db_tool;
pub use db_tool::*;

pub mod parse_explict_tools;
// pub mod query;
pub mod query_cata;
pub mod helper;
pub mod parsed_data;
pub mod pdms_data;
pub mod resolve_helper;
pub mod polish_notation;
pub mod direction_parse;
pub mod axis_param;
pub mod parse;
pub mod data_interface;
pub mod tool;

pub use parse::parse_pdms_dir;

pub mod consts;
pub mod shape;
pub mod prim_geo;
pub mod sctn;
pub mod pipes;
pub mod grpc;

pub mod local_db;

pub use parse::{parse_db, parse_file};
use std::time::Instant;

pub mod interface;
pub mod mesh_helper;
pub mod test_cases;

pub use test_cases::read_attr_info_config;
use crate::db_tool::db1_hash;
use crate::local_db::bonsaidb_local::{AiosDBManager, DbOption};

pub mod notify_file_change;

const ATT_PAXI: i32 = 0xB146F;
const ATT_PAAX: i32 = 0xF543D;
const ATT_PBAX: i32 = 0xF5458;
const ATT_PCAX: i32 = 0xF5473;
const ATT_PLAX: i32 = db1_hash("PLAX") as i32;

const ATT_PX: i32 = 0xFFF7E177u32 as i32;
const ATT_PY: i32 = 0xFFF7E15Cu32 as i32;
const ATT_PZ: i32 = 0xFFF7E141u32 as i32;
const ATT_PDIA: i32 = 0xFFF77D0Fu32 as i32;
const ATT_PHEI: i32 = 0xFFF520EFu32 as i32;
const ATT_PDIS: i32 = 0xFFF21519u32 as i32;
const ATT_PCON: i32 = 0xFFF3848Du32 as i32;
const ATT_PBOR: i32 = 0xFFF2511Cu32 as i32;
const ATT_PPRO: i32 = 0xFFF32DC0u32 as i32;
const ATT_DPRO: i32 = 0xFFF32DCCu32 as i32;
const ATT_BTHK: i32 = 0xFFF47D68u32 as i32;
const ATT_BDIA: i32 = 0xFFF77D1Du32 as i32;
const ATT_PTDI: i32 = 0xFFF52284u32 as i32;
const ATT_PBDI: i32 = 0xFFF5246Au32 as i32;
const ATT_PBTP: i32 = 0xFFF2DCA5u32 as i32;
const ATT_PCTP: i32 = 0xFFF2DC8Au32 as i32;
const ATT_PBBT: i32 = 0xFFF1DC5Bu32 as i32;
const ATT_PCBT: i32 = 0xFFF1DC40u32 as i32;
const ATT_PXLE: i32 = 0xFFF63EDCu32 as i32;
const ATT_PYLE: i32 = 0xFFF63EC1u32 as i32;
const ATT_PZLE: i32 = 0xFFF63EA6u32 as i32;
const ATT_PTDM: i32 = 0xFFF3EEF8u32 as i32;
const ATT_PBDM: i32 = 0xFFF3F0DEu32 as i32;
const ATT_POFF: i32 = 0xFFF60402u32 as i32;
const ATT_PTCDI: i32 = 0x95A34;
const ATT_DX: i32 = 0xFFF7E183u32 as i32;
const ATT_DY: i32 = 0xFFF7E168u32 as i32;
const ATT_PXTS: i32 = 0xFFF1F3AAu32 as i32;
const ATT_PYTS: i32 = 0xFFF1F38Fu32 as i32;
const ATT_PXBS: i32 = 0xFFF226ECu32 as i32;
const ATT_PYBS: i32 = 0xFFF226D1u32 as i32;
const ATT_PRAD : i32 = db1_hash("PRAD") as i32;
const ATT_DRAD : i32 = db1_hash("DRAD") as i32;
const ATT_PWID : i32 = db1_hash("PWID") as i32;

const ATT_PANG: i32 = 0xA5E2F;
const IMP_PAXI: i32 = 0xB146F;
const IMP_PCON: i32 = 0xC7B73;
const IMP_PDIS: i32 = 0xDEAE7;
const IMP_PBOR: i32 = 0xDAEE4;
const IMP_PDIA: i32 = 0x882F1;
const IMP_PHEI: i32 = 0xADF11;
const IMP_PTDI: i32 = 0xADD7C;
const IMP_PTDM: i32 = 0xC1108;
const IMP_PBDI: i32 = 0xADB96;
const IMP_PBDM: i32 = 0xC0F22;
const IMP_PPRO: i32 = 0xCD240;
const IMP_PRAD: i32 = 0x9544C;
const IMP_PX: i32 = 0x81E89;
const IMP_PY: i32 = 0x81EA4;
const IMP_PZ: i32 = 0x81EBF;
const IMP_PXLE: i32 = 0x9C124;
const IMP_PYLE: i32 = 0x9C13F;
const IMP_PZLE: i32 = 0x9C15A;
const IMP_PBTP: i32 = 0xD235B;
const IMP_PCTP: i32 = 0xD2376;
const IMP_PCBT: i32 = 0xE23C0;
const IMP_PBBT: i32 = 0xE23A5;
const IMP_PBOF: i32 = 0xA1440;
const IMP_PCOF: i32 = 0xA145B;
const IMP_PTCDI: i32 = 0x95A34;
const IMP_POFF: i32 = 0x9FBFE;
const IMP_DX: i32 = 0x81E7D;
const IMP_DY: i32 = 0x81E98;
const IMP_PLAX: i32 = 0xF5566;
const IMP_PXTS: i32 = 0xE0C56;
const IMP_PYTS: i32 = 0xE0C71;
const IMP_PXBS: i32 = 0xDD914;
const IMP_PYBS: i32 = 0xDD92F;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref EXPR_ATT_SET: HashSet<i32> = {
        let mut s = HashSet::new();
        s.insert(ATT_PAXI);s.insert(ATT_PAAX);s.insert(ATT_PBAX);s.insert(ATT_PCAX);
        s.insert(ATT_PLAX);
        s.insert(ATT_PX);s.insert(ATT_PY);s.insert(ATT_PZ);s.insert(ATT_PDIA);
        s.insert(ATT_PHEI);s.insert(ATT_PDIS);s.insert(ATT_PCON);s.insert(ATT_PBOR);
        s.insert(ATT_PPRO);s.insert(ATT_DPRO);s.insert(ATT_BTHK);s.insert(ATT_BDIA);
        s.insert(ATT_PTDI);s.insert(ATT_PBDI);s.insert(ATT_PBTP);s.insert(ATT_PCTP);
        s.insert(ATT_PBBT);s.insert(ATT_PCBT);s.insert(ATT_PXLE);s.insert(ATT_PYLE);
        s.insert(ATT_PZLE);s.insert(ATT_PTDM);s.insert(ATT_PBDM);s.insert(ATT_PTCDI);
        s.insert(ATT_POFF);s.insert(ATT_DX);s.insert(ATT_DY);s.insert(ATT_DY);
        s.insert(ATT_PXTS);s.insert(ATT_PYTS);s.insert(ATT_PXBS);s.insert(ATT_PYBS);
        s.insert(ATT_PRAD);s.insert(ATT_PWID);s.insert(ATT_DRAD);

        s.insert(IMP_PAXI);s.insert(IMP_PCON);s.insert(IMP_PDIS);s.insert(IMP_PBOR);
        s.insert(IMP_PDIA);s.insert(IMP_PHEI);s.insert(IMP_PTDI);s.insert(IMP_PTDM);
        s.insert(IMP_PBDI);s.insert(IMP_PBDM);s.insert(IMP_PPRO);s.insert(IMP_PRAD);
        s.insert(IMP_PX);s.insert(IMP_PY);s.insert(IMP_PZ);s.insert(IMP_PXLE);
        s.insert(IMP_PYLE);s.insert(IMP_PZLE);s.insert(IMP_PCTP);s.insert(IMP_PCBT);
        s.insert(IMP_PBBT);s.insert(IMP_PBOF);s.insert(IMP_PCOF);s.insert(IMP_PBTP);
        s.insert(IMP_PTCDI);s.insert(IMP_POFF);s.insert(IMP_DX);s.insert(IMP_DY);
        s.insert(IMP_PLAX);s.insert(IMP_PXTS);s.insert(IMP_PYTS);s.insert(IMP_PXBS);
        s.insert(IMP_PYBS);s.insert(ATT_PANG);
        s
    };
}

//cached functions to get value
//todo 数据分层，尽可能的用缓存

pub async fn init_pdms_db(db_option: Option<DbOption>) -> anyhow::Result<AiosDBManager> {

    let mut time = Instant::now();
    // #[cfg(target_arch = "arch64")]
    let path = "../Projects";
    let path = "G:/12.1SP4Projects";
    let path = "/Volumes/DPC/aba";
    // /Volumes/[C] Windows 11/AVEVA/Plant/Projects12.1.SP4
    // #[cfg(target_arch = "arch64")]
    // let path = "C:/AVEVA/Plant/Projects12.1.SP4";

    let mut db_manager = AiosDBManager::init(path,
                                             vec!["ABA".to_string()/*, "GDP".to_string()*/],
                                             "ABA",
                                             db_option).await.unwrap();

    dbg!(time.elapsed().as_millis());
    // let result = db_manager.cache_geos_data(11, "ABA").await?;
    // // db_manager.build_collision_world(7200).await?;
    // let refno = RefU64::from_two_nums(23584, 6615);
    let refno = RefU64::from_two_nums(23584, 5575);
    let refno = RefU64::from_two_nums(23584, 7040);
    let refno = RefU64::from_two_nums(23584, 8544);
    let refno = RefU64::from_two_nums(8193, 45580);
    let refno = RefU64::from_two_nums(8193, 4363);
    let refno = RefU64::from_two_nums(16395, 39039);
    let refno = RefU64::from_two_nums(16395, 39308);
    // let refno = RefU64(101292508714382);
    // let refno = RefU64(101292508716175);
    // //15213/499930
    // let refno = RefU64::from_two_nums(15213, 499930);
    // let refno = RefU64::from_two_nums(15192, 113114);

    //cached 一些常用的取值操作
    // let refno_info = db_manager.get_refno_info(&refno).await;
    // dbg!(&refno_info);

    // dbg!(refno_info);
    // dbg!(db_manager.get_project_of_refno(&refno).await);
    // dbg!(db_manager.get_pretty_attr(&refno).await);
    // dbg!(refno.to_refno_str());
    // dbg!(db_manager.get_pretty_attr(&refno).await);
    // dbg!(db_manager.get_world_transform(&refno).await);
    // dbg!(db_manager.get_children(&refno).await);
    // dbg!(db_manager.get_db_of_refno(&refno).await);

    // let mut  cache_mgr = CachedMeshes::default();
    // let attr = db_manager.get_dehashed_attr(&refno).await.unwrap().unwrap();
    // dbg!(attr.get_type());
    // if let Some(spre) = attr.get_foreign_refno("SPRE"){
    //     let geoms = db_manager.get_design_geoms(&refno, &mut cache_mgr).await;
    //     // dbg!(&geoms);
    // }



    // let refno = RefU64::from_two_nums(15207, 8922);
    //
    // //cached 一些常用的取值操作
    // let refno_info = db_manager.get_refno_info(&refno).await;
    //
    // dbg!(refno_info);
    // dbg!(db_manager.get_project_of_refno(&refno).await);
    // dbg!(db_manager.get_attr(&refno).await);

    //
    // let children = db.get_children(&refno).await?;
    // dbg!(&children);

    // let refno = RefU64::from_two_nums(23584, 9900);
    // let mut mgr = CachedMeshes::default();
    // let attr = db.get_attr(&refno).await.unwrap();
    // let ctorus: CTorus = (&attr).into();
    // if ctorus.check_valid() {
    //     dbg!(&ctorus);
    //     dbg!(attr.is_visible(None));
    //     let r = mgr.get_pdms_mesh_hash_key(Box::new(ctorus));
    //     let geo = Some(GeoData::Primitive(r));
    // }
    //
    // let trans = db.get_world_transform(&refno).await;
    // dbg!(&trans);

    // let refnos = vec![RefU64::from_two_nums(23584, 2705),
    //                   RefU64::from_two_nums(23584, 2706),
    //                   /*RefU64::from_two_nums(23584, 9008)*/];
    //
    // for refno in refnos {
    //     let attr = db.get_attr(&refno).await;
    //     let attr = attr.unwrap();
    //     dbg!(attr.is_visible(None));
    //     let mut dish: CTorus = attr.into();
    //     dbg!(dish.hash_mesh_params());
    //
    //     let idx = mgr.get_pdms_mesh_hash_key(&dish);
    //     dbg!(idx);
    // }

    // dbg!(mgr.meshes.len());

    // let refno = RefU64::from_two_nums(23584, 8839);
    // let attr = db.get_attr(&refno).await.unwrap();
    // dbg!(attr);
    // let trans = db.get_world_transform(&refno).await;
    // dbg!(&trans);
    // let mat3: glam::f32::Mat3 = glam::f32::Mat3::from_quat(trans.rotation);
    // dbg!(mat3);

    // let result = db.get_world_transform(&refno).await;
    // dbg!(trans.rotation.to_euler(glam::EulerRot::XYZ));
    // // dbg!(trans.rotation.to_scaled_axis());
    //
    // let matrix = db.get_world_matrix(&refno).await;
    // let quat = glam::Quat::from_affine3(&matrix);
    // dbg!(quat.to_euler(glam::EulerRot::XYZ));

    // let six_and_third = I24F8::from_num(19.23424);
// four decimal digits for 12 binary digits
//     dbg!(six_and_third);
//     assert_eq!(six_and_third.to_string(), "6.3333");


    // bonsaidb_local::cache_equip_geos_data().await;

    // #[cfg(feature = "sled")]{

    // sled_local::save_local().await;
        // sled_local::cache_room_geos_data().await;
    // }

    return Ok(db_manager);
}









#![feature(array_methods)]
#![feature(type_ascription)]

#[allow(dead_code, unused_imports)]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate serde;


use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::str::from_utf8;
use phf::phf_map;
use dashmap::DashMap;
use itertools::Itertools;
use rayon::iter::ParallelIterator;
use mongodb::bson::{doc, Document};
use std::{fs};
use std::env::current_dir;
use std::ffi::OsStr;
use std::option::Option::Some;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde::Serializer;

extern crate clap;

use clap::clap_app;
use log::LevelFilter;
use log::info;
use log::kv::Source;
use mongodb::options::{ClientOptions, FindOneAndReplaceOptions, FindOneAndUpdateOptions, FindOneOptions};
// use mysql::Pool;
// use mysql::prelude::Queryable;
use rayon::prelude::IntoParallelRefIterator;
use simplelog::{CombinedLogger, WriteLogger};
// use mysql::*;
// use mysql::prelude::*;
// use mysql::time::{Instant, parse};
use mongodb::IndexModel;
use mongodb::options::IndexOptions;
use parse_pdms_db::db_tool;
use parse_pdms_db::db_tool::{convert_to_hash, db1_dehash, decode_chars_data};
use parse_pdms_db::parse::*;
use parse_pdms_db::parse_explict_tools::*;
use parse_pdms_db::pdms_types::*;
use parse_pdms_db::pdms_types::AttrVal::*;
use std::ffi::OsString;
use fixed::types::{I20F12, I24F8};
use futures::TryStreamExt;
use id_tree::Tree;
use nalgebra_glm::Mat3;
use smol_str::SmolStr;
use parse_pdms_db::local_db::{bonsaidb_local, sled_local};
use parse_pdms_db::local_db::bonsaidb_local::{AiosDBManager, DbOption};
// use parse_pdms_db::local_db::sled_local::{cache_geos_data, save_local};
use parse_pdms_db::notify_file_change::notify_file;
use parse_pdms_db::prim_geo::ctorus::CTorus;
use parse_pdms_db::prim_geo::dish::Dish;
use parse_pdms_db::prim_geo::pdms_shape::{BrepShape, VerifiedShape};
use parse_pdms_db::test_cases::test_database::test_column;

const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;
type AiosDbError = core::result::Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn test() -> core::result::Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
#[test]
pub fn test_hash_noun(){
    dbg!(db1_dehash(0xE5461));
    dbg!(db1_dehash(0x95A34));
    dbg!(db1_dehash(0xC89B3));
    dbg!(db1_dehash(0x9298B));
    dbg!(db1_dehash(0x9CAF3));
    dbg!(db1_dehash(0x9BBDAC));

    dbg!(db1_dehash(convert_to_hash([0xFF, 0xF6, 0x94, 0x65].as_slice())));
}

fn main_1() {
    notify_file();
}

#[tokio::main]
async fn main() -> AiosDbError {
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), File::create("parse_pdms_db.log").unwrap()),
        ]
    ).unwrap();

    run().await;

    return Ok(());
}


pub async fn run() -> AiosDbError {

    let mut time = Instant::now();
    // #[cfg(target_arch = "arch64")]
    let path = "../Projects";
    // let path = "G:/12.1SP4Projects";
    // /Volumes/[C] Windows 11/AVEVA/Plant/Projects12.1.SP4
    // #[cfg(target_arch = "arch64")]
    // let path = "C:/AVEVA/Plant/Projects12.1.SP4";

    let mut db_manager = AiosDBManager::init(path,
                                             vec!["Sample".to_string(), "Master".to_string()],
                                             "Sample",
                                             Some(DbOption {
                                                 total_sync: false,
                                                 incr_sync: false,
                                             })).await.unwrap();

    dbg!(time.elapsed().as_millis());
    // let result = db_manager.cache_geos_data(7200).await?;
    // db_manager.build_collision_world(7200).await?;
    let refno = RefU64::from_two_nums(23584, 8537);
    // let refno = RefU64::from_two_nums(15192, 113114);

    //cached 一些常用的取值操作
    // let refno_info = db_manager.get_refno_info(&refno).await;
    // dbg!(&refno_info);


    //
    // dbg!(refno_info);
    // dbg!(db_manager.get_project_of_refno(&refno).await);
    // dbg!(db_manager.get_pretty_attr(&refno).await);
    dbg!(db_manager.get_pretty_attr(&refno).await);
    dbg!(db_manager.get_world_transform(&refno).await);
    // dbg!(db_manager.get_children(&refno).await);
    // dbg!(db_manager.get_db_of_refno(&refno).await);

    let mut  cache_mgr = CachedMeshes::default();
    let attr = db_manager.get_dehashed_attr(&refno).await.unwrap().unwrap();
    dbg!(&attr);
    if let Some(spre) = attr.get_foreign_refno("SPRE"){
        let geoms = db_manager.get_design_geoms(&refno, &mut cache_mgr).await;
        // dbg!(&geoms);
    }



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

    return Ok(());
}



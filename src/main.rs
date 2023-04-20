#![feature(array_methods)]
#![feature(type_ascription)]

#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]
#[macro_use]
extern crate nom;
#[macro_use]
extern crate serde;
extern crate clap;


use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::str::from_utf8;
use phf::phf_map;
use dashmap::DashMap;
use itertools::Itertools;
use rayon::iter::ParallelIterator;
use std::fs;
use std::env::current_dir;
use std::ffi::OsStr;
use std::option::Option::Some;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde::Serializer;



use clap::clap_app;
use log::LevelFilter;
use log::info;
use rayon::prelude::IntoParallelRefIterator;
use simplelog::{CombinedLogger, WriteLogger};
use parse_pdms_db::parse::*;
use parse_pdms_db::parse_explict_tools::*;
use std::ffi::OsString;
use aios_core::options::DbOption;
use aios_core::pdms_types::{AiosStr, PdmsCachedAttrMap, RefI32Tuple, };
use aios_core::tool::db_tool::{db1_hash};
use anyhow::anyhow;
use futures::TryStreamExt;
use id_tree::Tree;
use lazy_static::lazy_static;
use nalgebra_glm::Mat3;
use smol_str::SmolStr;


const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;

type AiosDbError = core::result::Result<(), Box<dyn std::error::Error>>;


fn main() -> anyhow::Result<()> {
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), std::fs::File::create("parse_pdms_db.log").unwrap()),
        ]
    ).unwrap();

    use config::{Config, ConfigError, Environment, File};
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()?;
    let db_option: DbOption = s.try_deserialize().unwrap();
    dbg!(&db_option);
    let mut time = Instant::now();
    // let mut mgr = AiosDBManager::init(&db_option).unwrap();
    // let v = mgr.get_attr(RefI32Tuple((23584, 205)).into())?;
    println!("初始化数据库时间: {} ms", time.elapsed().as_millis());
    // let refno = RefU64::from_two_nums(15192, 77134);
    // dbg!(mgr.get_attr(refno).unwrap().unwrap().to_string_hashmap());
    // let refno = RefU64::from_two_nums(15192, 77135);
    // dbg!(mgr.get_attr(refno).unwrap().unwrap().to_string_hashmap());
    // cache_viewer_data(&mut mgr, &db_option);
    // mgr.build_collision_world(db_option.project_name.as_str(), db_option.main_db_code);
    // mgr.set_ssc_room_tree(db_option.project_name.as_str(), db_option.main_db_code);
    return Ok(());
}


// pub fn cache_viewer_data(mgr: &mut AiosDBManager, db_option: &DbOption) -> anyhow::Result<bool> {

    // let r = mgr.cache_geos_data(db_option.main_db_code, db_option.project_name.as_str());
    // match r {
    //     Ok(_) => {}
    //     Err(err) => {
    //         println!("{:?}", err);
    //         return Err(err);
    //     }
    // }
    // return Ok(true);
    // let mut string_lookup = StringLookupTable::default();
    // let mut cached_attr_map: PdmsCachedAttrMap = PdmsCachedAttrMap::default();
    // let db_no = db_option.main_db_code;
    // // let tree = mgr.get_pdms_tree(db_option.project_name.as_str(), db_no).unwrap_or_default();
    //
    // let tree = mgr.get_pdms_tree(db_option.project_name.as_str(), db_no).ok_or(anyhow!("pdms tree not found"))?;
    // tree.serialize_to_bin_file(db_no);
    // let tree = tree.0;
    //
    // let mut proj_db = mgr.project_map.get_mut(&AiosStr(db_option.project_name.clone().into()).get_u32_hash())
    //     .ok_or(anyhow!("pdms project not found"))?;
    //
    // let node_id = tree.root_node_id().ok_or(anyhow!("root node not exist.".to_string()))?;
    // if let Ok(mut nodes) = tree.traverse_level_order_ids(node_id) {
    //     while let Some(mut cur_node_id) = nodes.next() {
    //         let cur_node = tree.get(&cur_node_id).unwrap();
    //         let d = cur_node.data();
    //         {
    //             if let Some(s) = proj_db.value_mut().get_string(d.name_hash).unwrap() {
    //                 string_lookup.lookup.insert(d.name_hash, s);
    //             }
    //         }
    //         cached_attr_map.0.insert(d.refno, proj_db.get_attr(d.refno, db_no)?);
    //     }
    // }
    // string_lookup.serialize_to_bin_file(db_no);
    // cached_attr_map.serialize_to_bin_file(db_option.main_db_code);
    //
//     Ok(true)
// }



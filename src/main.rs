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
use aios_core::pdms_types::{AiosStr, PdmsCachedAttrMap, RefI32Tuple, };
use aios_core::tool::db_tool::{db1_hash, read_attr_info_config};
use anyhow::anyhow;
use futures::TryStreamExt;
use id_tree::Tree;
use nalgebra_glm::Mat3;
use skytable::actions::Actions;
use skytable::{Connection, Element};
use skytable::ddl::{Ddl, Keymap, KeymapType};
use skytable::types::RawString;
use smol_str::SmolStr;
use parse_pdms_db::notify_file_change::notify_file;

const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;

type AiosDbError = core::result::Result<(), Box<dyn std::error::Error>>;

#[test]
pub fn test_hash_noun() {
    //dbg!(db1_dehash(0xE5461));
    //dbg!(db1_dehash(0x95A34));
    //dbg!(db1_dehash(0xC89B3));
    //dbg!(db1_dehash(0x9298B));
    //dbg!(db1_dehash(0x9CAF3));
    //dbg!(db1_dehash(0x9BBDAC));

    //dbg!(db1_dehash(convert_to_hash([0xFF, 0xF6, 0x94, 0x65].as_slice())));
}

// #[tokio::main]
// async
fn main() -> anyhow::Result<()> {
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), std::fs::File::create("parse_pdms_db.log").unwrap()),
        ]
    ).unwrap();
    return Ok(());
}


// pub fn cache_viewer_data(mgr: &mut AiosDBManager, db_option: &DbOption) -> anyhow::Result<bool> {
    //todo 可以用多线程去并发tree，获取节点下面，然后并发

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

// 修改 all_attr_info_bin 文件的属性的默认值
#[test]
fn change_info_bin_data() {
    // let mut config = read_attr_info_config("all_attr_info.bin");
    // let att = config.noun_attr_info_map.clone();
    // if let Some(value) = att.get(&(db1_hash("DB") as i32)){
    //     if let Some(mut v) = value.value().get_mut(&865153){
    //         v.att_type = aios_core::pdms_types::DbAttributeType::INTEGER;
    //         v.default_val = aios_core::pdms_types::AttrVal::IntegerType(1);
    //     }
    // };
    // config.noun_attr_info_map = att;
    // let mut file = File::create("all_attr_info_new.bin").unwrap();
    // file.write(&bincode::serialize(&config).unwrap());

    // 查看是否修改成功
    let att = read_attr_info_config("all_attr_info_new.bin").noun_attr_info_map;
    if let Some(value) = att.get(&(db1_hash("DB") as i32)) {
        if let Some(mut v) = value.value().get(&865153) {
            println!("v={:?}", v.value());
        }
    };
}

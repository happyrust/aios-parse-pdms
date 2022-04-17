#![feature(array_methods)]
#![feature(type_ascription)]

#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]
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
use std::fs;
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

// use mysql::Pool;
// use mysql::prelude::Queryable;
use rayon::prelude::IntoParallelRefIterator;
use simplelog::{CombinedLogger, WriteLogger};
// use mysql::*;
// use mysql::prelude::*;
// use mysql::time::{Instant, parse};
use parse_pdms_db::db_tool;
use parse_pdms_db::db_tool::{convert_to_hash, db1_dehash, decode_chars_data};
use parse_pdms_db::parse::*;
use parse_pdms_db::parse_explict_tools::*;
use parse_pdms_db::pdms_types::*;
use parse_pdms_db::pdms_types::AttrVal::*;
use std::ffi::OsString;
use anyhow::anyhow;
use fixed::types::{I20F12, I24F8};
use futures::TryStreamExt;
use id_tree::Tree;
use nalgebra_glm::Mat3;
use smol_str::SmolStr;
use parse_pdms_db::data_interface::PdmsDataInterface;
use parse_pdms_db::local_db::bonsaidb_local::AiosDBManager;
use parse_pdms_db::local_db::DbOption;
// use parse_pdms_db::local_db::sled_local::{cache_geos_data, save_local};
use parse_pdms_db::notify_file_change::notify_file;
use parse_pdms_db::prim_geo::ctorus::CTorus;
use parse_pdms_db::prim_geo::dish::Dish;
use parse_pdms_db::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
// use parse_pdms_db::test_cases::test_database::test_column;

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

fn main_1() {
    notify_file();
}

// #[tokio::main]
//async
fn main() -> AiosDbError {
    // CombinedLogger::init(
    //     vec![
    //         WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), File::create("parse_pdms_db.log").unwrap()),
    //     ]
    // ).unwrap();
    //
    let mut db_option = DbOption {
        total_sync: true,
        incr_sync: false,
        // project_path: "/Volumes/DPC/aba".to_string(),
        project_path: "D:/aba".to_string(),
        included_projects: vec!["ABA".to_owned(), "GDP".to_owned()],
        // included_db_files: Some(vec!["aba0117_0001".to_string()]),
        included_db_files: None,
        mdb_name: "ABA".to_string(),
        project_name: "ABA".to_string(),
        main_db_code: 117
    };

    let mut time = Instant::now();
    let mut mgr = AiosDBManager::init(&db_option).unwrap();

    println!("初始化数据库时间: {} ms", time.elapsed().as_millis());

    cache_viewer_data(&mut mgr, &db_option);

    return Ok(());
}


pub fn cache_viewer_data(mgr: &mut AiosDBManager, db_option: &DbOption) -> anyhow::Result<bool>{

    //todo 可以用多线程去并发tree，获取节点下面，然后并发
    let r = mgr.cache_geos_data(db_option.main_db_code, db_option.project_name.as_str());
    match r {
        Ok(_) => {}
        Err(err) => {
            println!("{:?}", err);
            return Err(err);
        }
    }
    // return Ok(true);
    let mut string_lookup = StringLookupTable::default();
    let mut cached_attr_map: PdmsCachedAttrMap = PdmsCachedAttrMap::default();
    let db_no = db_option.main_db_code;
    let tree = mgr.get_tree(db_option.project_name.as_str(), db_no).unwrap_or_default();
    tree.serialize_to_bin_file(db_no);
    let tree = &tree.0;

    if let Some(proj_db) = mgr.project_map.get(&AiosStr(db_option.project_name.clone().into()).get_u32_hash()){
        let node_id = tree.root_node_id().ok_or(anyhow!("root node not exist.".to_string()))?;
        if let Ok(mut nodes) = tree.traverse_level_order_ids(node_id) {
            while let Some(mut cur_node_id) = nodes.next() {
                let cur_node = tree.get(&cur_node_id).unwrap();
                let d = cur_node.data();
                if let Some(s) = proj_db.get_string(d.name_hash).unwrap(){
                    string_lookup.lookup.insert(d.name_hash, s);
                }
                cached_attr_map.0.insert(d.refno, mgr.get_stringfied_attr(d.refno).unwrap().unwrap());
            }
        }
        string_lookup.serialize_to_bin_file(db_no);
        cached_attr_map.serialize_to_bin_file(db_option.main_db_code);
    }


    Ok(true)
}

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
use mongodb::bson::{doc, Document};
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
use parse_pdms_db::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use parse_pdms_db::test_cases::test_database::test_column;

const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;
type AiosDbError = core::result::Result<(), Box<dyn std::error::Error>>;

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

    parse_pdms_db::init_pdms_db(&DbOption{
        total_sync: true,
        incr_sync: true,
        project_path: "/Volumes/DPC/aba".to_owned(),
        included_projects: vec!["ABA".to_string()/*, "GDP".to_string()*/],
        included_db_files: None,
    }).await;

    return Ok(());
}



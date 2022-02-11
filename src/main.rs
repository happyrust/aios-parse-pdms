#![feature(array_methods)]
#![feature(type_ascription)]

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
use parse_pdms_db::db_tool::{db1_dehash, decode_chars_data};
use parse_pdms_db::parse::*;
use parse_pdms_db::parse_explict_tools::*;
use parse_pdms_db::pdms_types::*;
use parse_pdms_db::pdms_types::AttrVal::*;
use std::ffi::OsString;
use futures::TryStreamExt;
use id_tree::Tree;
use smol_str::SmolStr;
use parse_pdms_db::local_db::{bonsaidb_local, sled_local};
use parse_pdms_db::local_db::bonsaidb_local::AiosDBManager;
// use parse_pdms_db::local_db::sled_local::{cache_geos_data, save_local};
use parse_pdms_db::notify_file_change::notify_file;

const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;

#[tokio::test]
async fn test() -> core::result::Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn main_1() {
    notify_file();
}

#[tokio::main]
async fn main() -> core::result::Result<(), Box<dyn std::error::Error>> {
    CombinedLogger::init(
        vec![
            WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), File::create("parse_pdms_db.log").unwrap()),
        ]
    ).unwrap();

    let mut db_manager = AiosDBManager::init("D:/AVEVA/Plant/Projects12.1.SP4",
                                             vec!["Sample".to_string()/*, "Master".to_string()*/], true).await.unwrap();
    let mut db = db_manager.db_map.get_mut("Sample").unwrap();
    db.cache_equip_geos_data().await;
    // bonsaidb_local::cache_equip_geos_data().await;

    #[cfg(feature = "sled")]{
        sled_local::save_local().await;
        sled_local::cache_room_geos_data().await;
    }

    return Ok(());
}



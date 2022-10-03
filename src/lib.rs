#![feature(type_ascription)]
#![feature(array_methods)]
#![feature(slice_pattern)]
#![feature(core_intrinsics)]
#![feature(associated_type_bounds)]
#![feature(once_cell)]
#![feature(async_closure)]
#![feature(generic_const_exprs)]
#![feature(default_free_fn)]
#![feature(exclusive_range_pattern)]
#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]
#[allow(unused_mut)]
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate approx;
#[macro_use]
extern crate hash32_derive;
extern crate hash32;

use std::collections::HashSet;
use std::error::Error;
use crate::parsed_data::GeomsInfo;
use futures::stream::TryStreamExt;
pub use parse::parse_pdms_dir;

pub mod error_types;
pub mod test_cases;
pub mod parse_explict_tools;
pub mod parsed_data;
pub mod parse;
pub mod consts;
pub mod shape;
pub mod grpc;

pub use parse::{parse_db, parse_file};
use std::time::Instant;
use aios_core::tool::db_tool::db1_hash;

pub mod options;

pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;


#[macro_use]
extern crate lazy_static;
extern crate core;


//cached functions to get value
//todo 数据分层，尽可能的用缓存

//todo wasm need use feature
// pub fn init_pdms_db(db_option: &DbOption) -> anyhow::Result<AiosDBManager> {
//     let mut time = Instant::now();
//     let mut db_manager = AiosDBManager::init(db_option).unwrap();
//
//     println!("初始化数据库时间: {} ms", time.elapsed().as_millis());
//     let refno = RefU64::from_two_nums(16395, 39308);
//     //cached 一些常用的取值操作
//     let refno_info = db_manager.get_refno_info(refno);
//     //dbg!(&refno_info);
//
//     return Ok(db_manager);
// }









#![feature(array_methods)]
#![feature(type_ascription)]

#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]
#[macro_use]
extern crate nom;
#[macro_use]
extern crate serde;
extern crate clap;
use std::convert::TryInto;
use std::time::Instant;
use serde::Serializer;
use log::LevelFilter;
use simplelog::{CombinedLogger, WriteLogger};
use aios_core::options::DbOption;
const ATT_MDB: i32 = 0x8221C;
const ATT_DB: i32 = 0x81C2B;
type AiosDbError = Result<(), Box<dyn std::error::Error>>;


fn main() -> anyhow::Result<()> {
    // CombinedLogger::init(
    //     vec![
    //         WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), std::fs::File::create("parse_pdms_db.log").unwrap()),
    //     ]
    // ).unwrap();

    use config::{Config, File};
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()?;
    let db_option: DbOption = s.try_deserialize().unwrap();
    dbg!(&db_option);
    let mut time = Instant::now();
    println!("初始化数据库时间: {} ms", time.elapsed().as_millis());
    return Ok(());
}




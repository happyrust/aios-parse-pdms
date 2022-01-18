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
use parse_pdms_db::parse::{DbInfo, get_dbname, get_dbname_from_dbnumber, get_numberdb, get_project_name_from_filename, parse_db, parse_db_name, parse_file, save_type_hash_file};
use parse_pdms_db::parse_explict_tools::{get_explicit_attr_type, get_expression_attr, parse_expression_attr, parse_axis_explicit_value_00, parse_axis_explicit_value_40, parse_axis_explicit_value_ff, print_refno_expression_data, times_keep_f32_two_decimal_place};
use parse_pdms_db::pdms_types::*;
use parse_pdms_db::pdms_types::AttrVal::*;
use std::ffi::OsString;
use futures::TryStreamExt;
use id_tree::Tree;
use smol_str::SmolStr;
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

    let matches = clap_app!(myapp =>
        (version: "1.0")
        (author: "dpc")
        (about: "获取PDMS的一些属性信息")
        (@arg DIR: -d --dir +takes_value  "PDMS文件夹路径")
        (@arg SAVE_TO_MONGODB: --save_mongdb  "是否保存到mongodb")
        (@arg SAVE_TO_MYSQL: --save_sql   "是否保存到mysql")
        (@arg SAVE_SYS: --save_sys "是否为sys文件")
        (@arg PARSE_NOUN_HASH_REFNO_MAP:  -s --save_pasre_hash_ref  "保存 noun_hash->refno map")
        (@arg FILENAME: -f --files +takes_value "PDMS文件名(格式为\"xx,xx,xx\")")
        (@arg CONFIG:  --attrconfig +takes_value "配置文件名")
        (@arg SERVER_IP: -i  --ip +takes_value "服务器ip地址")
        (@arg COUNT: --count +takes_value "读取的数据个数")
        (@arg DEBUG_REFNO: --debug_refno  +takes_value "打印参考号对应信息")
        (@arg TARGET_REFNO: --target_refno  +takes_value "指定开始参考号，从这个参考号开始读取")
        (@arg LOG : -l --log "是否保存log日志")
        (@arg EXP : -e --exp "是否只输出表达式")
    ).get_matches();

    //(@arg END_REFNO: -r  --refno +takes_value "结束的参考号")

    let dir = matches.value_of("DIR").unwrap_or(".");
    println!("PDMS文件夹路径为: {}", dir);
    let filter_files_str = matches.value_of("FILENAME").unwrap_or_default();
    let mut filter_file_names = vec![];
    if !filter_files_str.is_empty() {
        filter_file_names = filter_files_str.split(',').map(|x| x.trim().to_string()).collect();
    }
    println!("PDMS文件为: {:?}", filter_file_names);
    let config_path = matches.value_of("CONFIG").unwrap_or("");
    println!("配置文件路径为: {}", config_path);

    let server_ip = matches.value_of("SERVER_IP").unwrap_or("localhost");  //10.30.230.146
    println!("服务器ip为: {}", server_ip);
    let mongodb_url = format!("mongodb://{}:27017", server_ip);

    let print_refno_str = matches.value_of("DEBUG_REFNO").unwrap_or("");
    if !print_refno_str.is_empty() {
        println!("需要打印的参考号为: {}", print_refno_str);
    }

    let target_refno_str = matches.value_of("TARGET_REFNO").unwrap_or("");
    if !target_refno_str.is_empty() {
        println!("开始目标参考号为: {}", target_refno_str);
    }

    let limited_count_str = matches.value_of("COUNT").unwrap_or("unset");
    let limited_count = limited_count_str.parse::<u64>().unwrap_or(u64::MAX);  //i32::max_value
    dbg!(limited_count);

    // SERVER_IP
    let b_run_save_hash_ref = matches.occurrences_of("PARSE_NOUN_HASH_REFNO_MAP") == 1;
    let b_save_to_mongodb = matches.occurrences_of("SAVE_TO_MONGODB") == 1;
    let b_save_sys = matches.occurrences_of("SAVE_SYS") == 1;
    let b_save_to_log = matches.occurrences_of("LOG") == 1;
    let b_save_to_exp = matches.occurrences_of("EXP") == 1;
    dbg!(b_save_to_mongodb);
    let b_save_to_mysql = matches.occurrences_of("SAVE_TO_MYSQL") == 1;

    let mut target_files = Vec::new();
    if !filter_file_names.is_empty() && !b_run_save_hash_ref {
        for file_name in filter_file_names {
            let file_name = format!("{}/{}", dir, file_name);
            if Path::exists(file_name.as_ref()) {
                dbg!(&file_name);
                target_files.push(PathBuf::from(file_name));
            }
        }
    } else {
        target_files = fs::read_dir(dir)?.into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();
    }

    if b_run_save_hash_ref {
        dbg!("save_type_hash_file");
        save_type_hash_file(dir, "noun_hash_ref.map");
        return Ok(());
    }


    let mut database_info = None;
    if let Ok(mut file) = File::open(config_path) {
        let mut attr_buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut attr_buf);
        database_info = bincode::deserialize(&attr_buf).ok();
    }


    if database_info.is_none() {
        return Ok(());
    }

    target_files.sort_by(|a, b|
        fs::metadata(b).unwrap().len()
            .partial_cmp(&fs::metadata(a).unwrap().len()).unwrap());
    // let mut dbinfos = Vec::new();

    let parent_files = fs::read_dir(dir)?.into_iter().map(|entry| {
        let entry = entry.unwrap();
        entry.path()
    }).collect::<Vec<PathBuf>>();
    let mut pdms_refnos = vec![];
    let mut pdms_tree = vec![];
    let mut pdms_attrs = vec![];
    let mut project_name = SmolStr::new("");
    for path in parent_files {
        if let Some(file_name) = path.file_name().unwrap().to_str() {
            if file_name.to_string().ends_with("sys") {
                project_name = SmolStr::from(parse_db_name(file_name).unwrap().1);
                let mut client_options = ClientOptions::parse(&mongodb_url).await?;
                client_options.app_name = Some("AIOS".to_string());
                let client = mongodb::Client::with_options(client_options.clone())?;

                let mut file = File::open(&path).unwrap();
                let mut buf = vec![0u8; 36];
                file.read_exact(&mut buf)?;
                let db_type_bytes = &buf[32..36];
                let db_no_bytes = &buf[8..12];
                let db_no = i32::from_be_bytes(db_no_bytes.try_into().unwrap());
                let mut db_info = PDMSDBInfo::default();
                let eles_data_map = parse_file(&path, &database_info, limited_count as u32, b_save_to_log, print_refno_str, target_refno_str);
                pdms_refnos.push(eles_data_map.type_ele_map);
                pdms_tree.push(EleNodeDb::new(file_name, eles_data_map.ele_id_tree));
                pdms_attrs.push(eles_data_map.all_attr_map);
            }
        }
    };

    for path in target_files {
        let mut file = File::open(&path).unwrap();
        let mut buf = vec![0u8; 36];
        file.read_exact(&mut buf)?;
        let db_type_bytes = &buf[32..36];
        let db_no_bytes = &buf[8..12];
        let db_no = i32::from_be_bytes(db_no_bytes.try_into().unwrap());
        let mut db_info = PDMSDBInfo::default();
        println!("path={:?}", &path);
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let eles_data_map = parse_file(&path, &database_info, limited_count as u32, b_save_to_log, print_refno_str, target_refno_str);
        pdms_refnos.push(eles_data_map.type_ele_map);
        pdms_tree.push(EleNodeDb::new(file_name, eles_data_map.ele_id_tree));
        pdms_attrs.push(eles_data_map.all_attr_map);
        // if b_save_to_mongodb {
        //     let mut db_raw_name = path.file_name().unwrap().to_string_lossy().to_string();
        //     if let Some(name) = db_info_map.get(&db_no) {
        //         db_raw_name = name.to_string();
        //     }
        //     if db_raw_name == "" {
        //         continue;
        //     }
        //     if let Ok((_,v)) = get_dbname(db_raw_name.as_bytes(), &db_name_map){
        //         db_raw_name = v;
        //     }
        //     let mut db_name = db_raw_name[1..].replace('*', "").replace('/', "_");
        //     db_info.name = db_name.clone();
        //     db_info.db_no = db_no;
        //     db_info.db_type = db1_dehash(u32::from_be_bytes(db_type_bytes.try_into().unwrap_or_default()));
        //     dbinfos.push(db_info);
        //     dbg!(&db_name);
        //
        //     let mut client_options = ClientOptions::parse(&mongodb_url).await?;
        //     client_options.app_name = Some("AIOS".to_string());
        //     let client = mongodb::Client::with_options(client_options.clone())?;
        //     let db = client.database(&db_name);
        //     let db_name_clone = db_name.clone();
        //     let db_tree_name = format!("{}_tree", &db_name);
        //     let tree_db = client.database(&db_tree_name);
        //     // 存放所有的refno对应的db_name和type_name
        //     let table_db = client.database("PdmsRefnoDB");
        //     for (noun_hash, mut ele_data_vec) in eles_data_map.clone() {
        //         let type_name = db1_dehash(noun_hash as u32);
        //         println!("Curren {} elements len = {}", &type_name, ele_data_vec.len());
        //         let mut ele_table = Vec::new();
        //         let mut ele_nodes = Vec::new();
        //         for e in &ele_data_vec {
        //             ele_nodes.push(EleDataNode {
        //                 ref_no: e.ref_no.clone(),
        //                 children: vec![],
        //                 owner: e.owner.clone(),
        //                 name: e.name.clone(),
        //                 order: e.order,
        //                 db_name: db_name.clone(),
        //                 type_name: e.noun_name.clone(),
        //             });
        //             ele_table.push(PdmsRefno {
        //                 ref_no: e.ref_no.clone(),
        //                 db: db_name.clone(),
        //                 type_name: e.noun_name.clone(),
        //             });
        //         }
        //         // 属性值
        //         let collection = db.collection::<ElementData>(&type_name);
        //         collection.create_index(
        //             IndexModel::builder()
        //                 .keys(doc! {"ref_no":1})
        //                 .options(IndexOptions::builder().unique(true).build())
        //                 .build(),
        //             None,
        //         ).await?;
        //         for chunk in ele_data_vec.chunks(10000) {
        //             collection.insert_many(
        //                 chunk.to_owned(), None,
        //             ).await?;
        //         }
        //         // 参考号的tree
        //         let tree_collection = tree_db.collection::<EleDataNode>("PdmsTreeNode");
        //         collection.create_index(
        //             IndexModel::builder()
        //                 .keys(doc! {"ref_no":1})
        //                 .options(IndexOptions::builder().unique(true).build())
        //                 .build(),
        //             None,
        //         ).await?;
        //         for tree_chunk in ele_nodes.chunks(10000) {
        //             tree_collection.insert_many(
        //                 tree_chunk.to_owned(), None,
        //             ).await?;
        //
        //         }
        //         // 所有refno的dbname和typename
        //         let table_collection = table_db.collection::<PdmsRefno>("PdmsRefno");
        //         // collection.create_index(
        //         //     IndexModel::builder()
        //         //         .keys(doc! {"ref_no":1})
        //         //         .options(IndexOptions::builder().unique(true).build())
        //         //         .build(),
        //         //     None,
        //         // ).await?;
        //         for table_chunk in ele_table.chunks(10000) {
        //             table_collection.insert_many(
        //                 table_chunk.to_owned(), None,
        //             ).await?;
        //         }
        //     }
        //
        //     //commit db infos data
        //     let client = mongodb::Client::with_options(client_options)?;
        //     let db = client.database("PDMSDbInfos");
        //     let collection = db.collection::<PDMSDBInfo>("PDMSDbInfos");
        //     // collection.create_index(
        //     //     IndexModel::builder()
        //     //         .keys(doc! {"ref_no":1})
        //     //         .options(IndexOptions::builder().unique(true).build())
        //     //         .build(),
        //     //     None,
        //     // ).await?;
        //     collection.insert_many(
        //         dbinfos.to_owned(), None,
        //     ).await?;
        //     println!("Save {:?} to db ok", &path);
        // }


        // if b_save_to_exp {
        //     // let mut file = File::open("E:/AVEVA/Plant/PDMS12.0.SP4/expression_test.json").unwrap();
        //     // let reader = BufReader::new(file);
        //     // let database_info: DashMap<String, Vec<(String, String)>> = serde_json::from_reader(reader).unwrap();
        //     let exp_type = HashSet::from(["SSPH".to_string(), "SCTO".to_string(), "PTAX".to_string(), "LINE".to_string(),
        //         "SCYL".to_string(), "SCTO".to_string(), "LSNO".to_string(), "LCYL".to_string(), "PTCA".to_string(), "SDSH".to_string(),
        //         "BLTP".to_string(), "SSPH".to_string(), "SBOX".to_string(), "SCON".to_string(), "SSLC".to_string(), "NSSL".to_string()]);
        //     //let  exp_type = HashSet::from(["DATA".to_string()]);
        //     let mut parse_and_pdms_expression = DashMap::new();
        //     for (key, ele_data_vec) in eles_data_map {
        //         let table_name = db1_dehash(key as u32);
        //         if exp_type.contains(&table_name) {
        //             for ele in ele_data_vec {
        //                 let refno = ele.ref_no;
        //                 // if let Some(map) = database_info.get(&refno) {
        //                     let value = ele.attr_data_map;
        //                     //let result = map.clone();
        //                     let result=vec![];
        //                     let new_result = print_refno_expression_data(value, result);
        //                     parse_and_pdms_expression.insert(refno, new_result);
        //                 // }
        //             }
        //         }
        //     }
        //     let encoded: String = serde_json::to_string_pretty(&parse_and_pdms_expression).unwrap();
        //     let file_name = "expression.json";
        //     let mut file = OpenOptions::new()
        //         .write(true)
        //         .create(true)
        //         .truncate(true)
        //         .open(file_name)
        //         .unwrap();
        //     file.write_all(encoded.as_bytes());
        // }
        //
        //
    }

    if b_save_to_mongodb {
        let pdms_nodes = PdmsNode::new(pdms_refnos, pdms_tree, pdms_attrs);
        let pdms_refnos = pdms_nodes.type_ele_map;
        let pdms_tree = pdms_nodes.ele_id_tree;
        let pdms_attrs = pdms_nodes.all_attr_map;
        let client = mongodb::Client::with_uri_str("mongodb://localhost:27017").await?;
        let db = client.database(&format!("{}Project", project_name));
        let t_refnos = db.collection::< DashMap<SmolStr, Vec<SmolStr>> >("PdmsRefnos");

        let t_tree = db.collection::<EleNodeDb>("PdmsTree");
        let t_attrs = db.collection::< DashMap<SmolStr, AttrMap> >("PdmsAttrs");

        t_refnos.insert_one(pdms_refnos, None).await?;
        for table_chunk in pdms_tree.chunks(10000) {
            t_tree.insert_many(
                table_chunk.to_owned(), None,
            ).await?;
        }
        t_attrs.insert_one(pdms_attrs, None ).await?;
    }
    Ok(())
}



use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use bonsaidb::core::connection::{Connection, StorageConnection};
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::core::transaction;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::AsyncDatabase;
use itertools::Itertools;
use memchr::memmem::{find_iter, rfind_iter};
use smol_str::SmolStr;
use crate::helper::{parse_to_i32, parse_to_u16, parse_to_u32};
use crate::parse::{NOUN_TYPES_MAP, parse_ele_data, parse_file_basic_info, PdmsMongoDbInfo};
use crate::pdms_types::{AiosStr, DbnoVersion, EleNodeMongoDb, PdmsDatabaseInfo, RefU64Vec, StringLookupTable};
use crate::{AttrMap};
use crate::parse_increment_data::NewDataState;
use std::vec::Vec;
use anyhow::anyhow;
use bonsaidb::core::connection::AsyncStorageConnection;
use bonsaidb::core::connection::AsyncLowLevelConnection;
use bonsaidb::core::connection::*;
use crate::local_db::bonsaidb_local::AiosPdmsProject;


/// 检测新增数据是增删改中的哪个操作
pub fn check_increase_operate(input: &[u8], pos: usize, refno: &[u8]) -> Option<NewDataState> {
    let owner = &input[pos + 12..pos + 20];
    // 若能找到 则证明是增加
    if let Some(_owner_pos) = find_iter(&input[pos - 300..pos], owner).next() {
        let owner_data = &input[pos - 300..pos];
        let mut owner_iter = rfind_iter(owner_data, owner);
        while let Some(owner_pos) = owner_iter.next() {
            if &owner_data[owner_pos - 4..owner_pos - 2] == &[0, 2] {
                let children_len = parse_to_u16(&owner_data[owner_pos - 2..owner_pos]) as usize * 4;
                let children = &owner_data[owner_pos - 4..owner_pos - 4 + children_len];
                // 在children 中查找是否有该参考号，有就是增加
                if let Some(_) = find_iter(children, refno).next() {
                    return Some(NewDataState::Increase);
                }
                break;
            }
        }
    } else {
        // 往上找两个page(0x1000)，若不存在该参考号，则说明是删除
        if let Some(_) = find_iter(&input[pos - 0x1000..pos], refno).next() {
            if &input[pos - 4..pos] == &[0, 0, 0, 7] {
                return Some(NewDataState::Modify);
            }
        } else {
            return Some(NewDataState::Delete);
        }
    }
    None
}

/// 将修改数据保存到数据库
pub fn modify_data_to_db(input: &[u8], pdms_database_info: &PdmsDatabaseInfo, dbno: u64, dbs: &AiosPdmsProject) -> anyhow::Result<()> {
    let mut string_lookup = StringLookupTable::new();
    let ele_data = parse_ele_data(input, &pdms_database_info.noun_attr_info_map, &mut string_lookup,).unwrap_or_default();
    // 修改attrmap的数据
    let mut attr_db = dbs.storage.database::<AttrMap>(&dbno.to_string())?;
    let mut tx = Transaction::default();
    tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
        ele_data.refno.get_u32_hash(),
        &ele_data.attr_data_map,
    ).unwrap());
    attr_db.apply_transaction(tx)?;
    // 插入 StringLookUp数据
    for chunk in &string_lookup.lookup.iter().chunks(400000usize) {
        let mut tx = Transaction::default();
        for kv in chunk {
            tx.push(transaction::Operation::overwrite_serialized::<AiosStr>(
                kv.key().clone(),
                kv.value(),
            ).unwrap());
        }
        dbs.get_string_database().apply_transaction(tx)?;
    }

    println!("数据增量修改成功");
    Ok(())
}

pub fn increment_data_to_db(input: &[u8], pdms_database_info: &PdmsDatabaseInfo, dbno: u64, dbs: &AiosPdmsProject) -> anyhow::Result<()> {
    let mut string_lookup = StringLookupTable::new();
    let ele_data = parse_ele_data(input, &pdms_database_info.noun_attr_info_map, &mut string_lookup,).unwrap_or_default();
    let noun = ele_data.noun as u64;
    // todo 插入到tree中，先把 refnoinfo 加上 nodeid 再加上该功能
    // 修改 types_db中的参考号
    let mut v = RefU64Vec::get(noun, dbs.get_type_refs_database())?.unwrap();
    v.contents.push(ele_data.refno);
    let mut tx = Transaction::default();
    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
        noun,
        &v.contents,
    ).unwrap());
    dbs.get_type_refs_database().apply_transaction(tx)?;
    // 新增 refno的attmap
    let mut attr_db = dbs.storage.database::<AttrMap>(&dbno.to_string())?;
    let mut tx = Transaction::default();
    tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
        ele_data.refno.get_u32_hash(),
        &ele_data.attr_data_map,
    ).unwrap());
    attr_db.apply_transaction(tx)?;
    // 插入 StringLookUp数据
    for chunk in &string_lookup.lookup.iter().chunks(400000usize) {
        let mut tx = Transaction::default();
        for kv in chunk {
            tx.push(transaction::Operation::overwrite_serialized::<AiosStr>(
                kv.key().clone(),
                kv.value(),
            ).unwrap());
        }
        dbs.get_string_database().apply_transaction(tx)?;
    }

    println!("数据增量增加成功");
    Ok(())
}

// delete需要调整 ， 这个返回的是delete 的node的owner，只需获得删除的refno和owner的refno就好了
pub fn delete_data_to_db(input: &[u8], pdms_database_info: &PdmsDatabaseInfo, dbno: u64, dbs: &AiosPdmsProject) -> anyhow::Result<()> {
    let mut string_lookup = StringLookupTable::new();
    let data = parse_ele_data(input, &pdms_database_info.noun_attr_info_map, &mut string_lookup,).unwrap_or_default();
    let noun = data.noun as u64;
    // 修改 types_db中的参考号
    let mut v = RefU64Vec::get(noun, dbs.get_type_refs_database())?.unwrap();
    v.contents.0.retain(|x| { *x != data.refno });
    let mut tx = Transaction::default();
    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
        noun,
        &v.contents,
    ).unwrap());
    dbs.get_type_refs_database().apply_transaction(tx)?;
    // 删除att_map
    let attr_db = dbs.storage.database::<AttrMap>(&dbno.to_string())?;
    if let Some(doc) = AttrMap::get(data.refno.get_u32_hash(), &attr_db)? {
        attr_db.collection::<AttrMap>().delete(&doc)?;
    }
    Ok(())
}
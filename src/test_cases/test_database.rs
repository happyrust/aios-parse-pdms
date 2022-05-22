use std::fs::File;
use std::io::Read;
use aios_core::pdms_types::{AiosStr, AttrMap, Integer, PdmsTree, RefI32Tuple, RefnoInfo, RefU64, RefU64Vec, };
use aios_core::tool::db_tool::db1_hash;
use bevy::prelude::In;
use id_tree::Tree;
use skytable::actions::Actions;
use smol_str::SmolStr;
use crate::consts::ATT_ROOM;
use crate::parse::RoomCode;
// use crate::pdms_types::{AiosStr, AiosStrHash, Integer, PdmsTree, RefI32Tuple, RefU64Vec, StringLookupTable};
// use crate::{AttrMap, db1_hash, RefU64};

#[test]
fn query_room_code() {
    let db = sled::open("AIOS_DBS/APS/room.sled").expect("not found");
    let string_db = sled::open("AIOS_DBS/APS/names.sled").expect("not found");
    let vals = db.iter();
    for val in vals {
        if let Ok((_k,v)) = val {
            let v = bincode::deserialize::<RoomCode>(&v.to_vec()).unwrap();
            let name_bytes = string_db.get(v.name_hash.to_be_bytes()).unwrap().unwrap().to_vec();
            let room_name = bincode::deserialize::<AiosStr>(&name_bytes).unwrap();
            println!("refno={:?} room={:?}",v.refno.to_refno_str(),room_name);
        }
    }
}

#[test]
fn query_attr() {
    let db = sled::open("AIOS_DBS/SCB/attr.sled").expect("not found");
    let refno : RefU64 = RefI32Tuple((14195,6)).into();
    let val = db.get(refno.to_be_bytes()).unwrap().unwrap();
    let val = bincode::deserialize::<AttrMap>(&val.to_vec()).unwrap();
    println!("val={:?}",val.to_string_hashmap());
}

#[test]
fn test_tree() {
    let mut file = File::open("ssc_sample_250204.bin").unwrap();
    let mut buf = vec![];
    file.read_to_end(&mut buf).unwrap();
    let tree = bincode::deserialize::<PdmsTree>(&buf).unwrap();
    let tree_id = tree.0.root_node_id().unwrap();
    let children = tree.0.children(tree_id).unwrap();
    for child in children {
        println!("val={:?}",child.data());
    }
}

#[test]
fn test_name_hash() {
    let mut file = File::open("StringLookupTable_250204.bin").unwrap();
    let mut buf = vec![];
    file.read_to_end(&mut buf).unwrap();
    let string_map = bincode::deserialize::<StringLookupTable>(&buf).unwrap();
    if let Some(val) = string_map.lookup.get(&3696643365){
        println!("v={:?}",val.value());
    };
}

#[test]
fn test_project_type_refnos() {
    let db = sled::open("AIOS_DBS/APS/type_eles.sled").expect("not found");
    let attr_db = sled::open("AIOS_DBS/APS/attr.sled").expect("not found");

    if let Ok(Some(val)) = db.get(db1_hash("DB").to_be_bytes()) {
        let refnos = bincode::deserialize::<RefU64Vec>(&val.to_vec()).unwrap();
        for refno in refnos {
            if let Ok(Some(attr)) = attr_db.get(&refno.to_be_bytes()){
                let attr = bincode::deserialize::<AttrMap>(&attr.to_vec()).unwrap();
                println!("attr={:?}",attr.to_string_hashmap());
            }
        }
    }
}

#[test]
fn test_aios_hash() {
    let aios_str = AiosStr(SmolStr::new("SCB"));
    println!("r={}",aios_str.get_u32_hash());
}

use skytable::{Connection, SkyResult};
use skytable::ddl::Ddl;

#[test]
fn test_sky_table_att() {


    let mut con = Connection::new("127.0.0.1", 2003).unwrap();
    con.switch("ABA:attrs").unwrap();
    let key :RefU64 = RefI32Tuple((16476,57)).into();
    let v:AttrMap = con.get(&key).unwrap();
    println!("v={:?}",v);
}

#[test]
fn test_sky_table_type_refnos() {
    let mut con = Connection::new("127.0.0.1", 2003).unwrap();
    let noun:Integer = Integer(db1_hash("MDB"));
    let v:RefU64Vec = con.get(&noun).unwrap();
    println!("v={:?}",v);
}

#[test]
fn test_sky_table_refno_info(){
    let mut con = Connection::new("127.0.0.1", 2003).unwrap();
    con.switch("ref_infos:infos").unwrap();
    let refno:RefU64 = RefI32Tuple((23584,69)).into();
    let v :SkyResult<RefnoInfo>= con.get(&Integer(refno.get_0()));
    println!("v={:?}",v);
}
use std::error::Error;
use std::fs;
use std::path::Path;
use std::ptr::eq;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
use glam::Mat4;
use nom::AsBytes;
use smol_str::SmolStr;
use crate::{AttrMap, db1_dehash, parse_pdms_dir};
use crate::parse::PdmsDbData;
use crate::pdms_types::{EleGeoData, Refi32Tuple, RefU64, RefU64Vec};
// sanakirja 不支持动态大小
// use sanakirja::*;

#[derive(Default, Debug)]
pub struct PdmsConfig{
    pub data_dir: String,   //pdms的数据文件夹
    pub project_name: String,
    pub all_projects: Vec<String>,
    pub mdb_name: String,
}


pub async fn save_local() -> Result<(), sled::Error> {
    let pdms_config = PdmsConfig {
        // dir: "D:/AVEVA/Projects/E3D2.1/AvevaPlantSample/aps000".to_string(),
        // project_name: "aps000".to_string()
        data_dir: "D:/AVEVA/Plant/Projects12.1.SP4".to_string(),    //sam7200_0001
        project_name: "SAM".to_string(),
        all_projects: vec!["Sample".to_string(), "Master".to_string()],   //配置所有需要读取的project
        mdb_name: "SAMPLE".to_string(),
    };

    let mut data_dir = Path::new(&pdms_config.data_dir);
    for project in &pdms_config.all_projects {
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();
        dbg!(&target_dir);
        //todo have a test on versioned database, make a custom version
        if let Ok(attr_db) = sled::open(format!("./{project}/attrs.db")){
            if let Ok(type_refs_db) = sled::open(format!("./{project}/type_refs.db")) {
                if let Ok(mut r) = parse_pdms_dir(target_dir.as_os_str().to_str().unwrap(), None) {
                    fs::create_dir_all(project).unwrap();
                    for (k, PdmsDbData {
                        all_attr_map,
                        ele_id_tree,
                        type_ele_map,
                        db_name,
                        db_no,
                        refno_info_map,
                        ..
                    }) in r {
                        dbg!(all_attr_map.len());
                        let mut file_name = &k;
                        dbg!(&db_name);
                        //属性全部插入
                        for (refno, v) in all_attr_map {
                            let bytes = bincode::serialize(&v).unwrap();
                            attr_db.insert(refno.0.to_be_bytes(), bytes.as_bytes());
                        }
                        for (type_noun, v) in type_ele_map {
                            //和db_code 组合一个
                            let type_name = db1_dehash(type_noun);
                            let format_str = format!("{type_name}_{db_no}");
                            let bytes = bincode::serialize(&v).unwrap();
                            type_refs_db.insert(format_str.as_bytes(), bytes.as_bytes());
                        }

                        for (k, v) in refno_info_map {
                            let refno = k.0;
                            let format_str = format!("{refno}_children");
                            let bytes = bincode::serialize(&v).unwrap();
                            type_refs_db.insert(format_str.as_bytes(), bytes.as_bytes());
                        }

                        //cache the mesh attributes first

                    }
                }
            }
        }
    }
    Ok(())
}

#[inline]
pub fn get_attr(refno: &RefU64, attr_db: &sled::Db) -> Option<AttrMap>{
    if let Some(d) = attr_db.get(refno.0.to_be_bytes()).unwrap() {
        return bincode::deserialize::<AttrMap>(&d).ok();
    }
    None
}

pub fn get_ancestors_attrs(refno: &RefU64, attr_db: &sled::Db) -> Vec<AttrMap>{
    let mut attrs = vec![];
    let mut cur_refno = *refno;
    while let Some(attr) = get_attr(&cur_refno, attr_db){
        if let Some(owner) = attr.get_owner(){
            cur_refno = owner;
            attrs.push(attr);
        }else{
            break;
        }
    }
    attrs
}

///获取子孙后代的几何节点
pub fn get_descendant_attrs(refno: &RefU64, attr_db: &sled::Db) -> Vec<EleGeoData>{
    let mut geoms = vec![];
    //从一个节点开始层级遍历
    let mut cur_refno = *refno;
    while let Some(attr) = get_attr(&cur_refno, attr_db){
        // if let Some(owner) = attr.get_owner(){
        //     cur_refno = owner;
        //     attrs.push(attr);
        // }else{
        //     break;
        // }
    }
    geoms
}

pub fn get_world_matrix(refno: &RefU64, attr_db: &sled::Db) -> Mat4{
    let mut world_mat = Mat4::IDENTITY;
    let mut cur_refno = *refno;
    while let Some(attr) = get_attr(&cur_refno, attr_db){
        if let Some(owner) = attr.get_owner(){
            cur_refno = owner;
            world_mat = world_mat * attr.get_mat4();
        }else{
            break;
        }
    }
    world_mat
}


//test the equip data, default cache all the element
pub async fn cache_room_geos_data() -> Result<(), sled::Error> {
    let db_code = 7200;
    let equip_type = format!("EQUI_{db_code}");
    let project = "Sample";
    if let Ok(attr_db) = sled::open(format!("./{project}/attrs.db")) {
        dbg!(attr_db.len());
        if let Ok(type_refs_db) = sled::open(format!("./{project}/type_refs.db")) {
            dbg!(type_refs_db.len());
            //get the equip refnos
            if let Some(d) = type_refs_db.get(equip_type).unwrap(){
                let refnos = bincode::deserialize::<RefU64Vec>(&d).unwrap();
                dbg!(refnos.0.len());
                // dbg!(refnos.0.iter().map(|x| Refi32Tuple::from(x)).collect::<Vec<_>>());
                // dbg!(refnos.0.iter().map(|x| get_attr(x, &attr_db)).collect::<Vec<_>>());
                let first = refnos.0.first().unwrap();
                let ancestors = get_ancestors_attrs(first, &attr_db);
                dbg!(&ancestors);

                let world_mat = get_world_matrix(first,&attr_db);
                dbg!(world_mat);

                //get the equip's geoms node

            }
        }
    }
    Ok(())
}
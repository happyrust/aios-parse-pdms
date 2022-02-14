use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::ptr::eq;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::connection::{Connection, StorageConnection};
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::core::transaction;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::{Database, Storage};
use glam::Mat4;
use id_tree::NodeId;
use nom::AsBytes;
use smol_str::SmolStr;
use crate::{AttrMap, db1_dehash, parse_pdms_dir};
use crate::db_tool::db1_hash;
use crate::local_db::helper::combine_to_u64;
use crate::parse::{PdmsDbData};
use crate::pdms_types::{EleGeoData, GeoData, PdmsTree, Refi32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom};
use crate::prim_geo::pdms_shape::ScaledShape;
use crate::prim_geo::sbox::SBox;

pub const ATT_DB_NAME: &'static str = "attr";
pub const REFS_DB_NAME: &'static str = "refs";
pub const TREE_DB_NAME: &'static str = "tree";
pub const INFO_DB_NAME: &'static str = "info";
pub const GEOM_DB_NAME: &'static str = "geoms";

#[derive(Default, Debug)]
pub struct PdmsConfig {
    pub data_dir: String,
    //pdms的数据文件夹
    pub project_name: String,
    pub all_projects: Vec<String>,
    pub mdb_name: String,
}

///MDB数据库管理
#[derive(Default)]
pub struct AiosDBManager {
    pub db_map: HashMap<String, AiosDB>,
}

impl AiosDBManager {
    pub async fn init(dir: &str, projects: Vec<String>, sync: bool) ->  Result<AiosDBManager, bonsaidb::core::Error>{
        let mut adb_manager = AiosDBManager::default();
        for project in projects {
            let mut adb = AiosDB::init(project.as_str(), dir).await?;
            //如果已经保存过了，不需要重新保存
            if sync { adb.save().await?; }
            adb_manager.db_map.insert(project, adb);
        }
        Ok(adb_manager)
    }
}

/// DB 单个数据库管理
pub struct AiosDB {
    pub project: String,
    pub dir: String,
    //pdms data directory
    pub att_db: Database,
    pub refs_db: Database,
    pub tree_db: Database,
    pub info_db: Database,
    pub geom_db: Storage,
    pub mdb_name: Option<String>,
}

impl AiosDB {
    pub async fn init(project: &str, dir: &str) -> Result<Self, bonsaidb::core::Error> {
        Ok(Self {
            project: project.to_string(),
            dir: dir.to_string(),
            att_db: Database::open::<AttrMap>(StorageConfiguration::new(format!("./{project}/{ATT_DB_NAME}"))).await?,
            refs_db: Database::open::<RefU64Vec>(StorageConfiguration::new(format!("./{project}/{REFS_DB_NAME}"))).await?,
            tree_db: Database::open::<PdmsTree>(StorageConfiguration::new(format!("./{project}/{TREE_DB_NAME}"))).await?,
            info_db: Database::open::<RefnoInfo>(StorageConfiguration::new(format!("./{project}/{INFO_DB_NAME}"))).await?,
            geom_db: Storage::open(StorageConfiguration::new(format!("./{project}/{GEOM_DB_NAME}")).with_schema::<EleGeoData>()?).await?,
            mdb_name: None,
        })
    }

    #[inline]
    pub async fn get_attr(&self, refno: &RefU64) -> Option<AttrMap> {
        if let Ok(Some(d)) = AttrMap::get(refno.0, &self.att_db).await {
            return Some(d.contents);
        }
        None
    }

    #[inline]
    pub async fn get_node_id(&self, refno: &RefU64) -> Option<NodeId> {
        if let Ok(Some(d)) = RefnoInfo::get(refno.0, &self.info_db).await {
            return Some(d.contents.node_id);
        }
        None
    }

    ///获得世界坐标系
    pub async fn get_world_matrix(&self, refno: &RefU64) -> Mat4{
        let mut world_mat = Mat4::IDENTITY;
        let mut cur_refno = *refno;
        while let Some(attr) = self.get_attr(&cur_refno).await{
            if let Some(owner) = attr.get_owner(){
                cur_refno = owner;
                world_mat = world_mat * attr.get_mat4();
            }else{
                break;
            }
        }
        world_mat
    }

    pub async fn save(&mut self) -> Result<(), bonsaidb::core::Error> {
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;

        //todo 暂时全部删除，再创建
        // if Path::new(project).exists() {
        //     fs::remove_dir_all(project).unwrap();
        // } else {
        fs::create_dir_all(project).unwrap();
        // }
        dbg!("here");
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();
        //todo have a test on versioned database, make a custom version
        let mut tmp_set = HashSet::new();
        let mut tmp_set1 = HashSet::new();
        let mut tmp_set2 = HashSet::new();
        let mut tmp_set3 = HashSet::new();
        dbg!(&target_dir);
        if let Ok(mut r) = parse_pdms_dir(target_dir.as_os_str().to_str().unwrap(), None) {
            for (k, PdmsDbData {
                all_attr_map,
                ele_id_tree,
                type_ele_map,
                refno_info_map,
                db_name,
                db_no,
                filed_no,
                ..
            }) in r {
                dbg!(all_attr_map.len());
                let mut file_name = &k;
                dbg!(&db_name);
                //save the db tree
                let mut target_dbno = db_no as u64;
                if filed_no != 0 {
                    target_dbno = filed_no as u64;
                }
                // dbg!(self.tree_db.name());
                if !tmp_set.contains(&target_dbno) {
                    tmp_set.insert(target_dbno);
                    let mut tx = Transaction::default();
                    let pdms_tree = PdmsTree(ele_id_tree);
                    tx.push(transaction::Operation::insert_serialized::<PdmsTree>(
                        Some(target_dbno),
                        &pdms_tree,
                    ).unwrap());
                    
                    let mut file = File::create(format!("{target_dbno}.json")).unwrap();

                    let serialized = serde_json::to_string(&pdms_tree).unwrap();
                    file.write_all(serialized.as_bytes()).unwrap();

                    self.tree_db.apply_transaction(tx).await.unwrap();
                }

                //属性全部插入
                let mut tx = Transaction::default();
                for (refno, v) in all_attr_map {
                    if !tmp_set1.contains(&refno.0) {
                        tmp_set1.insert(refno.0);
                        tx.push(transaction::Operation::insert_serialized::<AttrMap>(
                            Some(refno.0),
                            &v,
                        ).unwrap());
                    }
                }
                self.att_db.apply_transaction(tx).await.unwrap();

                let mut tx = Transaction::default();
                for (type_noun, v) in type_ele_map {
                    if filed_no != 0 {
                        continue;
                    }
                    let k = combine_to_u64(type_noun, target_dbno as u32);
                    if !tmp_set2.contains(&k) {
                        tmp_set2.insert(k);
                        tx.push(transaction::Operation::insert_serialized::<RefU64Vec>(
                            Some(k),
                            &v,
                        ).unwrap());
                    }
                }
                self.refs_db.apply_transaction(tx).await.unwrap();

                let mut tx = Transaction::default();
                for (refno, v) in refno_info_map {
                    if !tmp_set3.contains(&refno.0) {
                        tmp_set3.insert(refno.0);
                        tx.push(transaction::Operation::insert_serialized::<RefnoInfo>(
                            Some(refno.0),
                            &v,
                        ).unwrap());
                    }
                }
                self.info_db.apply_transaction(tx).await.unwrap();
            }
        }
        Ok(())
    }


    //缓存设备得几何体
    pub async fn cache_equip_geos_data(&mut self) -> Result<HashMap<RefU64, EleGeoData>, bonsaidb::core::Error> {
        let db_code = 7200;
        let equip_hash = db1_hash("EQUI");
        dbg!(db1_dehash(equip_hash));
        let equip_key = combine_to_u64(equip_hash, db_code);
        let project = "Sample";

        let mut  geo_map = HashMap::new();

        //根据不同种类的几何体，做一下单独的聚集，这样再获取的时候能直接获取需要的几何形状
        // self.geom_db.create_database::<EleGeoData>("cubes", true).await?;
        // let cubes_db = storage.database::<Message>("cubes").await?;
        //for test
        // let test_refno: RefU64 = Refi32Tuple((23584, 10204)).into();
        // let attr = self.get_attr(&test_refno).await;
        if let Some(d) = RefU64Vec::get(equip_key, &self.refs_db).await? {
            // if let Some(d) = AttrMap::get(test_refno.0, &attr_db).await? {
            let refnos = d.contents;
            // dbg!(refnos.0.iter().map(|x| Refi32Tuple::from(x)).collect::<Vec<_>>());
            dbg!(refnos.len());

            if let Some(d) = PdmsTree::get(db_code as u64, &self.tree_db).await? {
                let tree = d.contents.0;
                dbg!(tree.height());
                let root_node_id = tree.root_node_id().unwrap();
                let mut cubes_tx = Transaction::default();
                for refno in refnos.0 {
                    let matrix = self.get_world_matrix(&refno).await;
                    dbg!(matrix);
                    if let Some(node_id) = self.get_node_id(&refno).await{
                        dbg!(&node_id);
                        //保存一个数据库，
                        if let Ok(mut nodes) = tree.traverse_level_order(&node_id){
                            while let Some(mut cur_node) = nodes.next() {
                                dbg!(&cur_node.data().name);
                                let d = cur_node.data();
                                //先针对box，生成模型
                                if d.noun == db1_hash("BOX"){
                                    let sbox: SBox = self.get_attr(&d.refno).await.unwrap().into();
                                    let geom_data = EleGeoData{
                                        geo: GeoData::Scaled(ScaledGeom::Box(sbox.get_scale_vec3())),
                                        global_transform: self.get_world_matrix(&d.refno).await           //todo 优化global matrix的计算，是否需要统一来一次计算
                                    };

                                    geo_map.insert(d.refno, geom_data);
                                    // tx.push(transaction::Operation::insert_serialized::<EleGeoData>(
                                    //     Some(d.refno.0),
                                    //     &geom_data,
                                    // ).unwrap());
                                }
                            }
                        }
                    }
                } //end for -equip
                // cubes_db.apply_transaction(tx).await.unwrap();
            }
        }
        Ok(geo_map)
    }

}

// pub const

pub fn get_children_attrs() -> Vec<AttrMap> {
    vec![]
}


//直接生成一个HashMap通过tonic返回给wasm, todo 数据库不用重复这样初始化多次
pub async fn get_just_cubes() -> Result<HashMap<RefU64, EleGeoData>, bonsaidb::core::Error>{
    let mut db_manager = AiosDBManager::init("D:/AVEVA/Plant/Projects12.1.SP4",
                                             vec!["Sample".to_string()/*, "Master".to_string()*/], true).await.unwrap();
    let mut db = db_manager.db_map.get_mut("Sample").unwrap();

    db.cache_equip_geos_data().await
}



// pub async fn get_ancestors_attrs(refno: &RefU64, attr_db: &Database) -> Vec<AttrMap> {
//     let mut attrs = vec![];
//     let mut cur_refno = *refno;
//     while let Some(attr) = get_attr(&cur_refno, attr_db).await {
//         if let Some(owner) = attr.get_owner() {
//             cur_refno = owner;
//             attrs.push(attr);
//         } else {
//             break;
//         }
//     }
//     attrs
// }



use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::ptr::eq;
use std::sync::Mutex;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::connection::{Connection, StorageConnection};
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::core::transaction;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::config::{Builder, Compression, StorageConfiguration};
use bonsaidb::local::{Database, Storage};
use glam::{Mat4, Quat, TransformRT, Vec3};
use id_tree::NodeId;
use itertools::Itertools;
use ncollide3d::world::CollisionWorld;
use nom::AsBytes;
use once_cell::sync::Lazy;
use smol_str::SmolStr;
use crate::{AttrMap, db1_dehash, parse_pdms_dir};
use crate::db_tool::db1_hash;
use crate::local_db::helper::combine_to_u64;
use crate::parse::{PdmsDbData};
use crate::pdms_types::{AiosStr, CachedMeshes, EleGeoData, GeoData, PdmsTree, RefI32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom, StringLookupTable};
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::pdms_shape::{BrepShape, VerifiedShape};
use crate::prim_geo::pyramid::LPyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::local_db::consts::*;
use crate::prim_geo::facet::{Contour, Facet, Polygon};

pub const ATT_DB_NAME: &'static str = "attr";
pub const REFS_DB_NAME: &'static str = "refs";
pub const CHILDREN_DB_NAME: &'static str = "children";
pub const TREE_DB_NAME: &'static str = "tree";
pub const INFO_DB_NAME: &'static str = "info";
pub const GEOM_DB_NAME: &'static str = "geoms";

///collision world  存储所属元件名称和GeoId
static GLOBAL_COLLISION_WORLD: Lazy<Mutex<CollisionWorld<f64, (String, RefU64)>>> = Lazy::new(|| {
    let mut world = CollisionWorld::<f64, (String, RefU64)>::new(0.001f64);
    Mutex::new(world)
});

static PRIM_HASH_NOUNS: Lazy<Vec<u32>> = Lazy::new(|| {
    vec![BOX_NOUN, CYLI_NOUN, SPHE_NOUN, CONE_NOUN, CTOR_NOUN, DISH_NOUN,
         LOOP_NOUN, PYRA_NOUN, RTOR_NOUN, REVO_NOUN, POHE_NOUN]
});


#[derive(Default, Debug)]
pub struct PdmsConfig {
    pub data_dir: String,
    //pdms的数据文件夹
    pub project_name: String,
    pub all_projects: Vec<String>,
    pub mdb_name: String,
}

#[derive(Debug, Clone)]
pub struct RefInoDatabase{
    pub db: Database,
    pub string_lookup: StringLookupTable,
}

impl Deref for RefInoDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl DerefMut for RefInoDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl RefInoDatabase {

    pub async fn init(path: &str, string_lookup: StringLookupTable) -> Self{
        Self{
            db: Database::open::<RefnoInfo>(StorageConfiguration::new(path)).await.expect("path not correct"),
            string_lookup
        }
    }

    ///获得refno的project 名称
    // #[inline]
    // pub async fn get_project_hash(&self, refno: &RefU64) -> Option<u32> {
    //     self.get_refno_info(refno).await.map(|x| x.project_hash)
    // }


    ///获得refno的project 名称
    // #[inline]
    // pub async fn get_project_name_of_refno(&self, refno: &RefU64) -> Option<SmolStr> {
    //     self.get_project_of_refno_by_hash(refno.get_u32_hash())
    // }

    ///获得refno的project 名称
    // #[inline]
    // pub async fn get_project_of_refno_by_hash(&self, hash: u32) -> Option<SmolStr> {
    //     self.get_refno_info_by_hash(hash).await.map(|x| x.project_hash).map(|x| self.string_lookup.get_string(x).unwrap())
    // }

    ///获得refno的project 名称
    #[inline]
    pub async fn get_refno_info(&self, refno: &RefU64) -> Result<Option<RefnoInfo>, bonsaidb::core::Error> {
        // self.get_refno_info_by_hash(refno.get_u32_hash())
        Ok(RefnoInfo::get(refno.get_0(), &self.db).await?.map(|x| x.contents))
    }


}


// #[derive(Debug, Clone)]
// pub struct StringDatabase(pub Database);

///MDB数据库管理
#[derive(Debug, Clone)]
pub struct AiosDBManager {
    pub db_map: HashMap<u32, AiosDB>, //project name hash -> Aios DB
    pub info_db: RefInoDatabase,      //管理所有refno info的db
    pub string_lookup: StringLookupTable,
}

#[derive(Debug, Default, Clone)]
pub struct DbOption {
    pub total_sync: bool,
    pub incr_sync: bool,
}

impl AiosDBManager {

    pub async fn init(dir: &str, projects: Vec<String>, mdb_name: &str, option: Option<DbOption>) -> Result<AiosDBManager, bonsaidb::core::Error> {
        let option = option.unwrap_or_default();
        let mut string_lookup = StringLookupTable::deserialize_from_default_json_file("AIOS").unwrap_or(StringLookupTable::new("AIOS"));
        let mut db_map = HashMap::default();
        let info_db = RefInoDatabase::init(format!("./AIOS_DBS/{INFO_DB_NAME}").as_str(), string_lookup.clone()).await;
        for project in projects {
            let mut adb = AiosDB::init(project.as_str(), dir, info_db.clone()).await?;
            //如果已经保存过了，不需要重新保存
            if option.total_sync { adb.sync_total(&mut string_lookup).await?; }  //完全更新
            if option.incr_sync {}    //todo 增量更新
            let project_str: SmolStr = project.into();
            db_map.insert(AiosStr(project_str).get_u32_hash(), adb);
        }

        // dbg!(&string_lookup);
        string_lookup.serialize_to_default_json_file();

        Ok(AiosDBManager{
            db_map,
            info_db,
            string_lookup
        })
    }


    ///获得refno的project 名称
    #[inline]
    pub async fn get_refno_info(&self, refno: &RefU64) -> Result<Option<RefnoInfo>, bonsaidb::core::Error> {
       self.info_db.get_refno_info(refno).await
    }

    ///获得refno的project 名称
    #[inline]
    pub async fn get_children(&self, refno: &RefU64) -> Result<Option<RefU64Vec>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await?{
            if let Some(db) = self.db_map.get(&ref_info.project_hash){
                return db.get_children(refno).await;
            }
        }
        Ok(Default::default())
    }


    ///获取attr 属性
    #[inline]
    pub async fn get_attr(&self, refno: &RefU64) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await?{
            if let Some(db) = self.db_map.get(&ref_info.project_hash){
                return db.get_attr(refno, ref_info.db_no).await;
            }
        }
        Ok(None)
    }

    ///string 被还原了的
    #[inline]
    pub async fn get_dehashed_attr(&self, refno: &RefU64) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await?{
            if let Some(db) = self.db_map.get(&ref_info.project_hash){
                return db.get_attr(refno, ref_info.db_no).await.map(|x| x.map( |x| x.dehash_value(&self.string_lookup)));
            }
        }
        Ok(None)
    }

    ///打印用
    #[inline]
    pub async fn get_pretty_attr(&self, refno: &RefU64) -> Result<HashMap<String, String>, bonsaidb::core::Error> {
        if let Some(attr) = self.get_attr(refno).await?{
            return Ok(attr.to_hashmap(&self.string_lookup));
        }
        Ok(Default::default())
    }



    ///获取世界坐标变换矩阵
    #[inline]
    pub async fn get_world_transform(&self, refno: &RefU64) -> Result<glam::TransformRT, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await?{
            if let Some(db) = self.db_map.get(&ref_info.project_hash){
                return db.get_world_transform(refno, ref_info.db_no).await;
            }
        }
        Ok(glam::TransformRT::IDENTITY)
    }

    ///缓存设备得几何体
    pub async fn cache_geos_data(&mut self, db_code: u32) -> Result<HashMap<SmolStr, EleGeoData>, bonsaidb::core::Error> {
        // let db_code = 7200;
        // let equip_hash = db1_hash("EQUI");
        // let equip_key = combine_to_u64(equip_hash, db_code);
        let project = AiosStr("Sample".into());
        let mut main_db = self.db_map.get_mut(&project.get_u32_hash()).expect("Not exist project");

        let mut cached_mesh_mgr = CachedMeshes::default();

        let mut geo_map = HashMap::new();

        if let Some(d) = PdmsTree::get(db_code as u64, &main_db.tree_db).await? {
            let tree = d.contents.0;
            let root_node_id = tree.root_node_id().unwrap();
            let mut cubes_tx = Transaction::default();
            let node_id = tree.root_node_id().unwrap();
            if let Ok(mut nodes) = tree.traverse_level_order(node_id) {
                while let Some(mut cur_node) = nodes.next() {
                    let d = cur_node.data();
                    let noun = d.noun;

                    // let mut db = self.get_db_of_refno(&d.refno).await.expect("Refno not exist.");
                    let attr = self.get_attr(&d.refno).await?.unwrap();

                    if PRIM_HASH_NOUNS.contains(&noun) {
                        let mut scaled = Vec3::ONE;

                        let mut tr = self.get_world_transform(&d.refno).await?;
                        let mut geo = None;
                        if noun == LOOP_NOUN {
                            let parent = attr.get_owner().unwrap();
                            let mut parent_att = self.get_attr(&parent).await?.unwrap();
                            let parent_noun = parent_att.get_type();
                            let mut loop_verts: Vec<Vec3> = vec![];
                            if let Some(children_refs) = self.get_children(&d.refno).await?{
                                for x in children_refs {
                                    let v = self.get_attr(&x).await?.unwrap().get_position();
                                    loop_verts.push(v);
                                }
                            }
                            //todo 旋转类型另外处理
                            if parent_noun != "REVO" && parent_noun != "NREV" {
                                if let Some(v) = parent_att.get_val("HEIG") {
                                    let height = v.f32_value().unwrap_or_default();
                                    if height >= f32::EPSILON {
                                        let extrusion = Box::new(Extrusion {
                                            loop_verts,
                                            height,
                                            ..Default::default()
                                        });
                                        if extrusion.check_valid() {
                                            let r = cached_mesh_mgr.get_pdms_mesh_hash_key(extrusion);
                                            geo = Some(GeoData::Primitive(r));
                                        }
                                    }
                                }
                            } else if parent_noun == "REVO" {
                                if let Some(v) = parent_att.get_val("ANGL") {
                                    let angle = v.f32_value().unwrap_or_default();
                                    // //dbg!(d.refno.to_refno_str());
                                    if angle >= f32::EPSILON {
                                        let revo = Box::new(Revolution {
                                            loop_verts,
                                            angle,
                                            ..Default::default()
                                        });
                                        // //dbg!(&revo);
                                        if revo.check_valid() {
                                            let r = cached_mesh_mgr.get_pdms_mesh_hash_key(revo);
                                            geo = Some(GeoData::Primitive(r));
                                        }
                                    }
                                }
                            }
                            //end of LOOP_NOUN
                        } else if noun == POHE_NOUN {  //多面体, try to save the leaf nodes in database
                            let children_hash = self.get_children(&d.refno).await?.unwrap_or_default();
                            let mut facet = Facet::default();
                            for x in children_hash {
                                let refs = self.get_children(&x).await?.unwrap_or_default();
                                let mut vertices: Vec<[f32; 3]> = vec![];
                                let mut tv = vec![];
                                let v_cnt = refs.len();
                                if v_cnt >= 3 {
                                    for x in refs {
                                        let mut contour = Contour::default();
                                        let v = self.get_attr(&x).await?.unwrap().get_position();
                                        vertices.push([v[0], v[1], v[2]]);
                                        if tv.len() < 3 {
                                            tv.push(v);
                                        }
                                    }
                                    let n = (tv[1] - tv[0]).cross(tv[2] - tv[1]).normalize();
                                    let mut polygon = Polygon {
                                        contours: vec![Contour {
                                            vertices,
                                            normals: vec![n.into(); v_cnt],
                                        }]
                                    };
                                    facet.polygons.push(polygon);
                                }
                            }
                            if facet.check_valid() {
                                let r = cached_mesh_mgr.get_pdms_mesh_hash_key(Box::new(facet));
                                geo = Some(GeoData::Primitive(r));
                            }


                        } else {
                            if let Some(brep_obj) = attr.create_brep_shape() {
                                if brep_obj.check_valid() {
                                    let r = cached_mesh_mgr.get_pdms_mesh_hash_key(brep_obj);
                                    geo = Some(GeoData::Primitive(r));
                                }
                            }
                        }
                        if let Some(geo) = geo {
                            let geom_data = EleGeoData {
                                geo,
                                global_transform: (tr.rotation, tr.translation),
                                visible: attr.is_visible(None),
                            };

                            geo_map.insert(d.refno.to_refno_str(), geom_data);
                        } // end of insert geo_map
                    }
                    else{
                        if let Some(spre) = attr.get_foreign_refno("SPRE"){
                            // let mut cate_db = self.get_db_of_refno(&spre).await.expect("DB not exist");
                            // //需要去取spref的key
                            // if let Some(cat_ref) = self.get_attr(&spre).await{
                            //     //dbg!(&cat_ref);
                            // }
                        }
                        // let tr = self.get_world_transform(&d.refno).await;
                    }

                }
            }
        }
        // }

        // D:/bevy_projects/web-aios
        let mut file = File::create(format!("../web-aios/{db_code}_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&geo_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();


        //dbg!(cached_mesh_mgr.meshes.len());
        cached_mesh_mgr.serialize_to_json_file();

        Ok(geo_map)
    }

}

/// DB 单个数据库管理
#[derive(Debug, Clone, )]
pub struct AiosDB {
    pub project: String,
    pub dir: String,
    //pdms data directory
    pub att_db_map: HashMap<u32, Database>,   //att map 需要做分库, db_number -> database
    // pub attr_db:  Database,   //att map 需要做分库, db_name -> database
    pub refs_db: Database,
    pub children_db: Database,
    pub tree_db: Database,
    pub info_db: RefInoDatabase,
    pub geom_db: Storage,
    pub mdb_name: Option<String>,
}


impl AiosDB {
    pub async fn create_att_database(path: &str) -> Result<Database, bonsaidb::local::Error> {
        //format!("./AIOS_DBS/{project}/{ATT_DB_NAME}")
        Database::open::<AttrMap>(StorageConfiguration::new(path)
                                  /*  .default_compression(Compression::Lz4)*/).await
    }

    pub async fn init(project: &str, dir: &str, info_db: RefInoDatabase) -> Result<Self, bonsaidb::core::Error> {

        // let info_db = RefInoDatabase::init(format!("./AIOS_DBS/{INFO_DB_NAME}").as_str()).await;

        //需要把所有的db number
        let mut att_db_map = HashMap::new();
        let cur_project = format!("./AIOS_DBS/{project}");
        let mut db_dir = Path::new(cur_project.as_str());
        if db_dir.exists() {
            let mut target_dir = fs::read_dir(db_dir).unwrap().into_iter().map(|entry| {
                let entry = entry.unwrap();
                entry.path()
            }).collect::<Vec<_>>();

            for x in target_dir {
                let extension = x.extension().unwrap_or_default().to_str().unwrap();
                if extension == "att" {
                    let db_code = x.file_stem().unwrap().to_str().unwrap().parse::<u32>().unwrap();
                    let db = Database::open::<AttrMap>(StorageConfiguration::new(x)).await.unwrap();
                    att_db_map.insert(db_code, db);
                }
            }
        }

        //dbg!(att_db_map.len());

        Ok(Self {
            project: project.to_string(),
            dir: dir.to_string(),
            att_db_map,
            // attr_db: Self::create_att_database(format!("./AIOS_DBS/{project}/{ATT_DB_NAME}").as_str()).await?,
            refs_db: Database::open::<RefU64Vec>(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{REFS_DB_NAME}"))/*.default_compression(Compression::Lz4)*/).await?,
            children_db: Database::open::<RefU64Vec>(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{CHILDREN_DB_NAME}"))/*.default_compression(Compression::Lz4)*/).await?,
            tree_db: Database::open::<PdmsTree>(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{TREE_DB_NAME}"))/*.default_compression(Compression::Lz4)*/).await?,
            info_db,
            geom_db: Storage::open(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{GEOM_DB_NAME}")).with_schema::<EleGeoData>()?).await?,
            mdb_name: None,
        })
    }

    //按需要打开database
    #[inline]
    pub async fn get_attr(&self, refno: &RefU64, db_no: u32) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(att_db) = self.att_db_map.get(&db_no){
            if let Ok(Some(mut d)) = AttrMap::get(refno.get_u32_hash(), att_db).await {
                return Ok(Some(d.contents));
            }
        }
        Ok(None)
    }

    #[inline]
    pub async fn get_children(&self, refno: &RefU64) -> Result<Option<RefU64Vec>, bonsaidb::core::Error> {
        if let Ok(Some(mut d)) = RefU64Vec::get(refno.0, &self.children_db).await {
            return Ok(Some(d.contents));
        }
        Ok(Default::default())
    }

    //按需要打开database
    // #[inline]
    // pub async fn get_attr_by_hash(&self, hash: u32, db_no: u32) -> Option<AttrMap> {
    //     if let Some(att_db) = self.att_db_map.get(&db_no){
    //         if let Ok(Some(d)) = AttrMap::get(hash, att_db).await {
    //             return Some(d.contents);
    //         }
    //     }
    //     None
    // }

    // #[inline]
    // pub async fn get_node_id(&self, refno: &RefU64) -> Option<NodeId> {
    //     if let Ok(Some(d)) = RefnoInfo::get(refno.0, &self.info_db).await {
    //         return Some(d.contents.node_id);
    //     }
    //     None
    // }

    //包含自己
    pub async fn get_ancestors_attrs(&self, refno: &RefU64, db_no: u32) -> Result<Vec<AttrMap>, bonsaidb::core::Error>{
        let mut cur_refno = *refno;
        let mut r = vec![];
        while let Some(attr) = self.get_attr(&cur_refno, db_no).await? {
            if let Some(owner) = attr.get_owner() {
                r.push(attr);
                cur_refno = owner;
            } else {
                break;
            }
        }
        Ok(r)
    }

    // ///获得世界坐标系
    pub async fn get_world_transform(&self, refno: &RefU64, db_no: u32) -> Result<glam::TransformRT, bonsaidb::core::Error> {
        let mut ancestors = self.get_ancestors_attrs(&refno, db_no).await?;
        ancestors.reverse();
        let mut rotation = Quat::IDENTITY;
        let mut translation = Vec3::ZERO;
        let mut parent: Option<Quat> = None;
        for attr in ancestors {
            let t = attr.get_rotation();
            translation = translation + rotation * attr.get_position();
            rotation = rotation * t;
        }
        Ok(glam::TransformRT {
            rotation,
            translation,
        })
    }

    //todo  infos 存储什么的问题，要不要存储dbno
    pub async fn sync_total(&mut self, whole_string_lookup: &mut StringLookupTable) -> Result<(), bonsaidb::core::Error> {
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();
        if let Ok(mut r) = parse_pdms_dir(target_dir.as_os_str().to_str().unwrap(), project.as_str(),None) {
            for (k, PdmsDbData {
                all_attr_map,
                ele_id_tree,
                type_ele_map,
                refno_info_map,
                db_name,
                db_no,
                field_no,
                string_lookup,
                children_map,
                ..
            }) in r {
                let target_dbno = if field_no == 0 {db_no} else{ field_no};
                let mut attr_db = Self::create_att_database(format!("./AIOS_DBS/{project}/{target_dbno}.att").as_str()).await?;

                whole_string_lookup.merge(&string_lookup);

                let mut tx = Transaction::default();
                let pdms_tree = PdmsTree(ele_id_tree);
                tx.push(transaction::Operation::overwrite_serialized::<PdmsTree>(
                    target_dbno as u64,
                    &pdms_tree,
                ).unwrap());
                self.tree_db.apply_transaction(tx).await.unwrap();

                // 属性全部插入
                dbg!(all_attr_map.len());
                for chunk in &all_attr_map.iter().chunks(400000usize) {
                    let mut tx = Transaction::default();
                    for kv in chunk {
                        tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
                            kv.key().get_u32_hash(),
                            kv.value(),
                        ).unwrap());
                    }
                    attr_db.apply_transaction(tx).await.unwrap();
                }
                self.att_db_map.insert(target_dbno, attr_db);

                let mut tx = Transaction::default();
                for (type_noun, v) in type_ele_map {
                    let k = combine_to_u64(type_noun, target_dbno as u32);
                    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
                        k,
                        &v,
                    ).unwrap());
                }
                self.refs_db.apply_transaction(tx).await.unwrap();


                let mut tx = Transaction::default();
                for (refno, v) in children_map {
                    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
                        refno.0,
                        &v,
                    ).unwrap());
                }
                self.children_db.apply_transaction(tx).await.unwrap();

                dbg!(refno_info_map.len());
                for chunk in &refno_info_map.iter().chunks(400000usize) {
                    let mut tx = Transaction::default();
                    for (k,v) in chunk {
                        tx.push(transaction::Operation::overwrite_serialized::<RefnoInfo>(
                            // kv.key().get_u32_hash(),
                            *k,
                            v,
                        ).unwrap());
                        // tx.push(transaction::Operation::overwrite_serialized::<RefnoInfo>(
                        //     // kv.key().get_u32_hash(),
                        //     k.get_u32_hash(),
                        //     v,
                        // ).unwrap());
                    }
                    self.info_db.apply_transaction(tx).await.unwrap();
                }


            }
        }
        Ok(())
    }

    //todo 基于元件库的模型也要生成
    //todo 房间号的算法移植

    pub async fn build_collision_world(&mut self) -> Result<(), bonsaidb::core::Error> {
        let mut world = GLOBAL_COLLISION_WORLD.lock().unwrap();
        // *world = CollisionWorld::<f64, (String, RefU64)>::new(0.001f64);
        Ok(())
    }



    ///获得下一个Element
    #[inline]
    pub async fn next(&mut self) -> Result<(), bonsaidb::core::Error> {
        Ok(())
    }



}






use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::mem::size_of;
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
use crate::pdms_types::{CachedMeshes, EleGeoData, GeoData, PdmsTree, Refi32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom};
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

///MDB数据库管理
#[derive(Debug, Clone)]
pub struct AiosDBManager {
    pub db_map: HashMap<String, AiosDB>,
    pub info_db: Database,
}

#[derive(Debug, Default, Clone)]
pub struct DbOption {
    pub total_sync: bool,
    pub incr_sync: bool,
}

impl AiosDBManager {

    pub async fn init(dir: &str, projects: Vec<String>, option: Option<DbOption>) -> Result<AiosDBManager, bonsaidb::core::Error> {
        let option = option.unwrap_or_default();
        let mut db_map = HashMap::default();
        let info_db = Database::open::<RefnoInfo>(
            StorageConfiguration::new(format!("./AIOS_DBS/{INFO_DB_NAME}")).default_compression(Compression::Lz4)).await?;
        for project in projects {
            let mut adb = AiosDB::init(project.as_str(), dir, info_db.clone()).await?;
            //如果已经保存过了，不需要重新保存
            if option.total_sync { adb.sync_total().await?; }  //完全更新
            if option.incr_sync {}    //todo 增量更新
            db_map.insert(project, adb);
        }

        Ok(AiosDBManager{
            db_map,
            info_db
        })
    }

    ///获得refno的project 名称
    #[inline]
    pub async fn get_project_of_refno(&self, refno: &RefU64) -> Option<SmolStr> {
        if let Ok(Some(d)) = RefnoInfo::get(refno.0, &self.info_db).await{
            Some(d.contents.project)
        }else{
            None
        }
    }

    /// 获得children
    #[inline]
    pub async fn get_children(&mut self, refno: &RefU64) -> Result<RefU64Vec, bonsaidb::core::Error> {
        if let Some(d) = RefnoInfo::get(refno.0, &self.info_db).await? {
            return Ok(d.contents.children);
        }
        Ok(RefU64Vec::default())
    }


    ///获得refno对应的db
    #[inline]
    pub async fn get_db_of_refno(&self, refno: &RefU64) -> Option<AiosDB> {
        if let Some(project) = self.get_project_of_refno(refno).await {
            self.db_map.get(project.as_str()).map(|x| (*x).clone())
        }else{
            None
        }
    }

    #[inline]
    pub async fn get_attr(&self, refno: &RefU64) -> Option<AttrMap> {
        // if let Some(db) = self.get_db_of_refno(refno).await {
        //     db.get_attr(refno).await
        // }else{
            None
        // }
    }

    //缓存设备得几何体
    pub async fn cache_geos_data(&mut self) -> Result<HashMap<SmolStr, EleGeoData>, bonsaidb::core::Error> {
        let db_code = 7200;
        // let equip_hash = db1_hash("EQUI");
        // let equip_key = combine_to_u64(equip_hash, db_code);
        let project = "Sample";
        let mut main_db = self.db_map.get_mut(project).expect("Not exist project");

        let mut cached_mesh_mgr = CachedMeshes::default();

        let mut geo_map = HashMap::new();
        let box_hash = db1_hash("BOX");
        let cylinder_hash = db1_hash("CYLI");
        let sphere_hash = db1_hash("SPHE");
        let cone_hash = db1_hash("CONE");
        let dish_hash = db1_hash("DISH");
        let ctorus_hash = db1_hash("CTOR");
        let loop_hash = db1_hash("LOOP");

        if let Some(d) = PdmsTree::get(db_code as u64, &main_db.tree_db).await? {
            let tree = d.contents.0;
            let root_node_id = tree.root_node_id().unwrap();
            let mut cubes_tx = Transaction::default();
            let node_id = tree.root_node_id().unwrap();
            if let Ok(mut nodes) = tree.traverse_level_order(node_id) {
                while let Some(mut cur_node) = nodes.next() {
                    let d = cur_node.data();
                    let noun = d.noun;

                    let mut db = self.get_db_of_refno(&d.refno).await.expect("Refno not exist.");
                    let attr = db.get_attr(&d.refno).await.unwrap();

                    if PRIM_HASH_NOUNS.contains(&noun) {
                        let mut scaled = Vec3::ONE;

                        let tr = db.get_world_transform(&d.refno).await;
                        let mut geo = None;
                        if noun == LOOP_NOUN {
                            let parent_node = tree.get(cur_node.parent().unwrap()).unwrap();
                            let parent_refno = parent_node.data().refno.to_refno_str();
                            let parent_noun = parent_node.data().noun;
                            let type_string = db1_dehash(parent_node.data().noun);
                            let mut loop_verts: Vec<Vec3> = vec![];
                            let children_refs = self.get_children(&d.refno).await?;
                            for x in children_refs.0 {
                                let v = db.get_attr(&x).await.unwrap().get_position();
                                loop_verts.push(v);
                            }
                            let mut parent_att = db.get_attr(&parent_node.data().refno).await.unwrap();
                            //todo 旋转类型另外处理
                            if parent_noun != REVO_NOUN && parent_noun != NREV_NOUN {
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
                            } else if parent_noun == REVO_NOUN {
                                if let Some(v) = parent_att.get_val("ANGL") {
                                    let angle = v.f32_value().unwrap_or_default();
                                    // dbg!(d.refno.to_refno_str());
                                    if angle >= f32::EPSILON {
                                        let revo = Box::new(Revolution {
                                            loop_verts,
                                            angle,
                                            ..Default::default()
                                        });
                                        // dbg!(&revo);
                                        if revo.check_valid() {
                                            let r = cached_mesh_mgr.get_pdms_mesh_hash_key(revo);
                                            geo = Some(GeoData::Primitive(r));
                                        }
                                    }
                                }
                            }
                            //end of LOOP_NOUN
                        } else if noun == POHE_NOUN {  //多面体, try to save the leaf nodes in database
                            let children_refs = self.get_children(&d.refno).await?;
                            let mut facet = Facet::default();
                            for x in children_refs {
                                let refs = self.get_children(&x).await?;
                                dbg!(&refs);
                                let mut vertices: Vec<[f32; 3]> = vec![];
                                let mut tv = vec![];
                                let v_cnt = refs.len();
                                if v_cnt >= 3 {
                                    for x in refs {
                                        let mut contour = Contour::default();
                                        let v = db.get_attr(&x).await.unwrap().get_position();
                                        vertices.push([v[0], v[1], v[2]]);
                                        if tv.len() < 3 {
                                            tv.push(v);
                                        }
                                    }
                                    let n = (tv[1] - tv[0]).cross(tv[2] - tv[1]).normalize();
                                    dbg!(&n);
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
                                dbg!(&facet);
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
                        if d.refno == RefU64::from_two_nums(23584, 9898) {
                            dbg!(&geo);
                        }
                        if let Some(geo) = geo {
                            let geom_data = EleGeoData {
                                geo,
                                global_transform: (tr.rotation, tr.translation),           //todo 优化global matrix的计算，是否需要统一来一次计算
                                visible: attr.is_visible(None),
                            };

                            geo_map.insert(d.refno.to_refno_str(), geom_data);
                        } // end of insert geo_map
                    }
                    else{
                        if let Some(spre) = attr.get_foreign_refno("SPRE"){
                            let mut cate_db = self.get_db_of_refno(&spre).await.expect("DB not exist");
                            //需要去取spref的key
                            if let Some(cat_ref) = self.get_attr(&spre).await{
                                dbg!(&cat_ref);
                            }
                        }
                        // let tr = self.get_world_transform(&d.refno).await;
                    }

                }
            }
        }
        // }

        // D:/bevy_projects/web-aios
        let mut file = File::create(format!("D:/bevy_projects/web-aios/{db_code}_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&geo_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();


        dbg!(cached_mesh_mgr.meshes.len());
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
    // pub att_db_map: HashMap<SmolStr, Database>,   //att map 需要做分库, db_name -> database
    pub attr_db:  Database,   //att map 需要做分库, db_name -> database
    pub refs_db: Database,
    pub tree_db: Database,
    pub info_db: Database,
    pub geom_db: Storage,
    pub mdb_name: Option<String>,
}


impl AiosDB {

    pub async fn create_att_database(path: &str) -> Result<Database, bonsaidb::local::Error> {
        //format!("./AIOS_DBS/{project}/{ATT_DB_NAME}")
        Database::open::<AttrMap>(StorageConfiguration::new(path)
            .default_compression(Compression::Lz4)).await
    }

    pub async fn init(project: &str, dir: &str, info_db: Database) -> Result<Self, bonsaidb::core::Error> {
        Ok(Self {
            project: project.to_string(),
            dir: dir.to_string(),
            attr_db: Self::create_att_database(format!("./AIOS_DBS/{project}/{ATT_DB_NAME}").as_str()).await?,
            refs_db: Database::open::<RefU64Vec>(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{REFS_DB_NAME}")).default_compression(Compression::Lz4)).await?,
            tree_db: Database::open::<PdmsTree>(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{TREE_DB_NAME}")).default_compression(Compression::Lz4)).await?,
            info_db,
            geom_db: Storage::open(StorageConfiguration::new(format!("./AIOS_DBS/{project}/{GEOM_DB_NAME}")).with_schema::<EleGeoData>()?).await?,
            mdb_name: None,
        })
    }

    #[inline]
    pub async fn get_attr(&self, refno: &RefU64) -> Option<AttrMap> {
        // if let Ok(Some(d)) = AttrMap::get(refno.0, &self.att_db).await {
        //     return Some(d.contents);
        // }
        None
    }

    #[inline]
    pub async fn get_node_id(&self, refno: &RefU64) -> Option<NodeId> {
        if let Ok(Some(d)) = RefnoInfo::get(refno.0, &self.info_db).await {
            return Some(d.contents.node_id);
        }
        None
    }

    /// 获得children
    #[inline]
    pub async fn get_children(&mut self, refno: &RefU64) -> Result<RefU64Vec, bonsaidb::core::Error> {
        if let Some(d) = RefnoInfo::get(refno.0, &self.info_db).await? {
            return Ok(d.contents.children);
        }
        Ok(RefU64Vec::default())
    }

    //包含自己
    pub async fn get_ancestors_attrs(&self, refno: &RefU64) -> Vec<AttrMap> {
        let mut cur_refno = *refno;
        let mut r = vec![];
        while let Some(attr) = self.get_attr(&cur_refno).await {
            if let Some(owner) = attr.get_owner() {
                r.push(attr);
                cur_refno = owner;
            } else {
                break;
            }
        }
        r
    }

    ///获得世界坐标系
    pub async fn get_world_transform(&self, refno: &RefU64) -> glam::TransformRT {
        let mut ancestors = self.get_ancestors_attrs(&refno).await;
        ancestors.reverse();
        let mut rotation = Quat::IDENTITY;
        let mut translation = Vec3::ZERO;
        let mut parent: Option<Quat> = None;
        for attr in ancestors {
            let t = attr.get_rotation();

            translation = translation + rotation * attr.get_position();
            rotation = rotation * t;
        }
        glam::TransformRT {
            rotation,
            translation,
        }
        // world_mat
    }

    pub async fn sync_total(&mut self) -> Result<(), bonsaidb::core::Error> {
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
                filed_no,
                ..
            }) in r {
                let mut attr_db = Self::create_att_database(format!("./AIOS_DBS/{project}/{db_no}.db").as_str()).await?;

                dbg!(all_attr_map.len());
                let mut file_name = &k;
                dbg!(&file_name);
                // continue;
                //save the db tree
                let mut target_dbno = db_no as u64;
                if filed_no != 0 {
                    target_dbno = filed_no as u64;
                }
                // dbg!(self.tree_db.name());
                let mut tx = Transaction::default();
                let pdms_tree = PdmsTree(ele_id_tree);
                tx.push(transaction::Operation::overwrite_serialized::<PdmsTree>(
                    target_dbno,
                    &pdms_tree,
                ).unwrap());
                self.tree_db.apply_transaction(tx).await.unwrap();
                // let mut file = File::create(format!("./{project}/{target_dbno}.json")).unwrap();

                // let serialized = serde_json::to_string(&pdms_tree).unwrap();
                // file.write_all(serialized.as_bytes()).unwrap();

                //属性全部插入
                let mut tran_cnt = 0;
                for chunk in &all_attr_map.iter().chunks(400000usize) {
                    let mut tx = Transaction::default();
                    for kv in chunk {
                        // tx.push(transaction::Operation::insert_serialized::<AttrMap>(
                        //     Some(kv.key().get_hash()),
                        //     kv.value(),
                        // ).unwrap());
                        tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
                            kv.key().get_hash(),
                            kv.value(),
                        ).unwrap());
                    }
                    attr_db.apply_transaction(tx).await.unwrap();
                }

                let mut tx = Transaction::default();
                for (type_noun, v) in type_ele_map {
                    if filed_no != 0 {
                        continue;
                    }
                    let k = combine_to_u64(type_noun, target_dbno as u32);
                    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
                        k,
                        &v,
                    ).unwrap());
                }
                self.refs_db.apply_transaction(tx).await.unwrap();

                // let mut tx = Transaction::default();
                // for (refno, v) in refno_info_map {
                //     tx.push(transaction::Operation::overwrite_serialized::<RefnoInfo>(
                //         refno.0,
                //         &v,
                //     ).unwrap());
                // }
                // self.info_db.apply_transaction(tx).await.unwrap();
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






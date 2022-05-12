use std::borrow::BorrowMut;
use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::default::default;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::ptr::eq;
use std::sync::{Arc, Mutex, RwLock};
use anyhow::anyhow;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::connection::{AsyncConnection, AsyncStorageConnection};
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::core::transaction;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::config::{Builder, Compression, StorageConfiguration};
use bonsaidb::local::{Database, Storage};
use glam::{Mat4, Quat, TransformRT, TransformSRT, Vec3};
use itertools::Itertools;
use ncollide3d::world::CollisionWorld;
use nom::AsBytes;
use once_cell::sync::Lazy;
use smol_str::SmolStr;
use crate::{ db1_dehash,parse_pdms_dir, read_attr_info_config, sctn};
use crate::data_interface::PdmsDataInterface;
use crate::db_tool::db1_hash;
use crate::local_db::helper::combine_to_u64;
use crate::parse::{get_dbnos_of_mdb, NOUN_TYPES_MAP, parse_file_basic_info, PdmsDbData, RoomCode};
// use crate::pdms_types::{AiosStr, AiosStrHash, CachedMeshesMgr, DbnoVersion, EleGeoInstData, GeoData, Integer, PdmsMeshMgr, PdmsNodeId, PdmsTree, RefI32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom, ShapeInstancesMgr, StringLookupTable};
// use crate::prim_geo::ctorus::{CTorus, SCTorus};
// use crate::prim_geo::extrusion::{CurveType, Extrusion};
// use crate::shape::pdms_shape::{ PdmsPrimShape};
// use crate::prim_geo::revolution::Revolution;
use crate::local_db::consts::*;
use crate::local_db::refno_info_database::RefInoDatabase;
use crate::local_db::string_database::StringDatabase;
// use crate::pdms_data::ScomInfo;
// use crate::pdms_types::AttrVal::{RefU64Type, StringHashType, StringType, WordType};
// use crate::prim_geo::facet::{Contour, Facet, Polygon};
use async_trait::async_trait;
use bevy::prelude::Transform;
use dashmap::DashMap;
use memchr::memmem::rfind_iter;
use ncollide3d::bounding_volume::AABB;
use ncollide3d::na as na;
use ncollide3d::na::{Isometry3, Translation3, UnitQuaternion};
use ncollide3d::pipeline::{CollisionGroups, GeometricQueryType};
use ncollide3d::query::{Ray, RayCast};
use ncollide3d::shape::{Cuboid, ShapeHandle};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use truck_polymesh::stl::IntoSTLIterator;
use crate::helper::{parse_to_i32, parse_to_u32};
// use crate::parse_increment_data::increment_modify::{check_increase_operate, increment_data_to_db, modify_data_to_db};
// use crate::parse_increment_data::NewDataState;
use crate::parsed_data::CateProfileParam;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::query_cata::resolve_desi_comp;
use clap::{Parser, ValueHint};
// use crate::prim_geo::category::{CateBrepShape, convert_to_brep_shapes};
use std::panic::catch_unwind;
use std::time::Instant;
use aios_core::pdms_types::{AiosStr, AiosStrHash, AttrMap, CachedMeshesMgr, EleGeoInstData, EleNode, Integer, PdmsMeshMgr, PdmsNodeId, PdmsTree, RefnoInfo, RefU64, RefU64Vec, ShapeInstancesMgr, StringLookupTable};
use aios_core::pdms_types::AttrVal::{RefU64Type, StringHashType, StringType, WordType};
use aios_core::prim_geo::category::{CateBrepShape, convert_to_brep_shapes};
use aios_core::prim_geo::extrusion::{CurveType, Extrusion};
use aios_core::prim_geo::facet::{Contour, Facet, Polygon};
use aios_core::prim_geo::revolution::Revolution;
use aios_core::prim_geo::tubing::PdmsTubing;
use aios_core::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy::ecs::schedule::ShouldRun::No;
use bevy::render::primitives::Aabb;
use bincode::deserialize;
// use crate::prim_geo::sphere::Sphere;
// use crate::prim_geo::tubing::PdmsTubing;
use bonsaidb::core::connection::StorageConnection;
use bonsaidb::core::connection::LowLevelConnection;
use calamine::{open_workbook, RangeDeserializerBuilder, Reader, Xlsx};
use futures::StreamExt;
use id_tree::InsertBehavior::{AsRoot, UnderNode};
use log::Level::Debug;
use sled::{Db, IVec};
use bevy::ecs::component::Component;
use id_tree::{Node, NodeId};
use skytable::actions::Actions;
use skytable::Connection;
use skytable::ddl::{Ddl, Keymap, KeymapType};
use skytable::types::IntoSkyhashBytes;
use crate::consts::ATT_ROOM;
use crate::error_types::AttError::AttNotExist;
use crate::local_db::DbOption;

pub const ATT_DB_NAME: &'static str = "attr";
pub const TYPES_DB_NAME: &'static str = "refs";
pub const CHILDREN_DB_NAME: &'static str = "children";
pub const TREE_DB_NAME: &'static str = "tree";
pub const INFO_DB_NAME: &'static str = "info";
pub const STR_DB_NAME: &'static str = "strs";
pub const GEOM_DB_NAME: &'static str = "geoms";
pub const DBNO_VERSIONS: &'static str = "vers";
pub const TUBI_TOL: f32 = 10.0f32;
//多少距离需要成为 tubi
///collision world  存储所属元件名称和GeoId
static GLOBAL_COLLISION_WORLD: Lazy<Mutex<CollisionWorld<f32, (RefU64, RefU64)>>> = Lazy::new(|| {
    let mut world = CollisionWorld::<f32, (RefU64, RefU64)>::new(0.001f32);
    Mutex::new(world)
});

static PRIM_HASH_NOUNS: Lazy<Vec<u32>> = Lazy::new(|| {
    vec![BOX_NOUN, CYLI_NOUN, SPHE_NOUN, CONE_NOUN, CTOR_NOUN, DISH_NOUN,
         LOOP_NOUN, PYRA_NOUN, RTOR_NOUN, REVO_NOUN, POHE_NOUN, PLOO_NOUN, SPINE_NOUN]
});

static GENRIC_NOUN_NAMES: Lazy<Vec<SmolStr>> = Lazy::new(|| {
    vec!["EQUI".into(), "PIPE".into(), "STRU".into(), "ROOM".into(), "STWALL".into(), "FLOOR".into()]
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
#[derive(Debug, Clone,Component)]
pub struct AiosDBManager {
    pub project_map: DashMap<u32, AiosPdmsProjectSled>,
    //project hash -> Project DBS
    //project name hash -> Aios DB
    pub info_db: sled::Db,

    pub projects: Vec<String>,

    pub needed_parse_files: Option<Vec<String>>,

    pub project_path: String,  //整个项目的路径
}

// #[async_trait]
impl PdmsDataInterface for AiosDBManager {
    fn sync_total_project(&self) -> anyhow::Result<bool> {
        self.sync_total_internal()
    }

    fn sync_incremental_project(&mut self) -> anyhow::Result<bool> {
        self.sync_incremental_internal()
    }

    #[inline]
    fn get_ele_attr(&self, refno: RefU64) -> anyhow::Result<AttrMap> {
        let att = self.get_stringfied_attr(refno)?;
        Ok(att.unwrap_or_default())
    }

    #[inline]
    fn get_ele_children_attrs(&self, refno: RefU64) -> Vec<AttrMap> {
        self.get_children_attrs(refno).unwrap_or_default()
    }

    #[inline]
    fn get_ele_children_refs(&self, refno: RefU64) -> RefU64Vec {
        self.get_children(refno).unwrap().unwrap_or_default()
    }

    fn get_ele_world_transform(&self, refno: RefU64) -> TransformRT {
        self.get_world_transform(refno).unwrap_or_default()
    }

    fn get_pdms_tree(&self, project_name: &str, db_no: u32) -> Option<PdmsTree> {
        if let Some(db) = self.project_map.get(&AiosStr(project_name.into()).get_u32_hash()) {
            db.get_tree(db_no).ok()?
        } else {
            None
        }
    }

    fn get_node_id(&self, refno: RefU64) -> Option<NodeId> {
        if let Ok(Some(ref_info)) = self.get_refno_info(refno) {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_node_id(refno).ok()?;
            }
        }
        None
    }

    fn get_name(&self, refno: RefU64) -> SmolStr {
        "unset".into()
    }

    fn get_name_by_hash(&self, refno: RefU64, name_hash: u32) -> Option<SmolStr> {
        if let Ok(Some(ref_info)) = self.get_refno_info(refno) {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                if let Ok(Some(name)) = db.get_string(name_hash) {
                    return Some(name.0);
                }
            }
        }
        None
    }

    fn get_refnos_by_type(&self, project_name: SmolStr, att_type: &str) -> Option<RefU64Vec> {
        if let Some(project_dbs) = self.project_map.get(&AiosStr(project_name).get_u32_hash()) {
            let type_db = project_dbs.types_db.clone();
            if let Ok(Some(v)) = type_db.get(&db1_hash(att_type).to_be_bytes()) {
                return Some(bincode::deserialize::<RefU64Vec>(&v.to_vec()).unwrap());
            }
        }
        None
    }

    // 将所有 dbno 的 tree 合并成一个 tree
    fn get_pdms_project_tree(&self, project: &str,main_db:u32) -> anyhow::Result<PdmsTree> {
        let mut tree_map = HashMap::new();
        let mut pdms_tree = PdmsTree::default();
        // 获取到所有的 tree
        if let Some(dbno_refnos) = self.get_refnos_by_type(SmolStr::new(project), "DB") {
            for dbno_refno in dbno_refnos {
                if let Ok(Some(dbno_info)) = self.get_refno_info(dbno_refno) {
                    if let Ok(Some(dbno_att)) = self.get_attr(dbno_refno) {
                        if let Some(dbno) = dbno_att.get_val("NUMBDB") {
                            let dbno = dbno.i32_value() as u32;
                            if let Some(tree) = self.get_pdms_tree_by_name_hash(dbno_info.project_hash,dbno) {
                                tree_map.entry(dbno).or_insert(tree);
                            } else {
                                for project in &self.projects {
                                    let project_hash = AiosStr(SmolStr::new(project)).get_u32_hash();
                                    if project_hash != dbno_info.project_hash {
                                        if let Some(tree) = self.get_pdms_tree_by_name_hash(project_hash,dbno) {
                                            tree_map.entry(dbno).or_insert(tree);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // 将所有的 tree 重新排序 ,重构成 pdms 的 tree
        if let Some(main_tree) = tree_map.remove(&main_db) {
            if let Some(root) = main_tree.0.root_node_id() {
                let root_data = main_tree.0.get(root)?.data().clone();
                let child = pdms_tree.0.insert(Node::new(root_data),AsRoot)?;
                let main_tree_order = main_tree.0.traverse_post_order(root)?;
                for main_node in main_tree_order {
                    pdms_tree.0.insert(Node::new(main_node.data().clone()),UnderNode(&child))?;
                }
                for (dbno,tree) in tree_map {
                    dbg!(dbno);
                    let root_id = tree.0.root_node_id().ok_or(anyhow!("it's a empty tree"))?;
                    let tree_order = tree.0.traverse_post_order(root_id)?;
                    for node in tree_order {
                        pdms_tree.0.insert(Node::new(node.data().clone()),UnderNode(&child))?;
                    }
                }
            }
        }
        Ok(pdms_tree)
    }
}

impl AiosDBManager {
    ///初始化
    pub fn init(option: &DbOption) -> anyhow::Result<AiosDBManager> {
        let dir = option.project_path.as_str();
        let info_db = sled::open("AIOS_DBS/ref_info.sled").expect("Create info_db file");
        let project_map = DashMap::new();
        for project in &option.included_projects {
            let mut proj = AiosPdmsProjectSled::init(project.as_str(), option.project_path.as_str(), info_db.clone())?;
            let project_str: SmolStr = project.into();
            project_map.insert(AiosStr(project_str).get_u32_hash(), proj);
        }
        let mut mgr = AiosDBManager {
            project_map,
            info_db,
            projects: option.included_projects.clone(),
            needed_parse_files: option.included_db_files.clone(),
            project_path: option.project_path.clone(),
        };

        if option.total_sync {
            mgr.sync_total_internal()?;
        }
        Ok(mgr)
    }

    fn sync_incremental_internal(&mut self) -> anyhow::Result<bool> {
        Ok(true)
    }

    /// 需要spawn a task to run
    ///内部实现同步所有，todo 添加部分同步
    fn sync_total_internal(&self) -> anyhow::Result<bool> {
        let time = std::time::Instant::now();
        println!("当前解析线程数量: {}", rayon::current_num_threads());
        for project in &self.project_map {
            //完全同步数据
            project.value().sync_total(&self.needed_parse_files)?;
        }
        println!("总共时间: {} ms", time.elapsed().as_millis());
        Ok(true)
    }

    ///获得refno的project 名称
    #[inline]
    pub fn get_refno_info(&self, refno: RefU64) -> anyhow::Result<Option<RefnoInfo>> {
        let bytes = self.info_db.get(&refno.get_0().to_be_bytes())
            .map_err(|_| anyhow!("get refno error".to_string()))?;
        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<RefnoInfo>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    /// 获得 children refno
    #[inline]
    pub fn get_children(&self, refno: RefU64) -> anyhow::Result<Option<RefU64Vec>> {
        if let Some(ref_info) = self.get_refno_info(refno)? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_children(refno);
            }
        }
        Ok(Default::default())
    }

    ///获得refno的project 名称
    #[inline]
    pub fn get_children_attrs(&self, refno: RefU64) -> anyhow::Result<Vec<AttrMap>> {
        let mut atts = vec![];
        let mut children = self.get_children(refno)?.unwrap_or_default();
        for child in children.drain(..) {
            atts.push(self.get_stringfied_attr(child)?.unwrap_or_default());
        }
        Ok(atts)
    }

    ///获取attr 属性
    #[inline]
    pub fn get_attr(&self, refno: RefU64) -> anyhow::Result<Option<AttrMap>> {
        if let Some(ref_info) = self.get_refno_info(refno)? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_attr(refno, ref_info.db_no);
            }
        }
        Ok(None)
    }

    #[inline]
    pub fn get_attr_with_project(&self, refno: RefU64, project: &str, db_no: u32) -> anyhow::Result<Option<AttrMap>> {
        if let Some(db) = self.project_map.get(&AiosStr(project.into()).get_u32_hash()) {
            return db.get_attr(refno, db_no);
        }
        Ok(None)
    }

    ///string 被还原了的 属性
    pub fn get_stringfied_attr(&self, refno: RefU64) -> anyhow::Result<Option<AttrMap>> {
        if let Some(ref_info) = self.get_refno_info(refno)? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                if let Some(mut attr) = db.get_attr(refno, ref_info.db_no)? {
                    for (_, val) in attr.iter_mut() {
                        if let StringHashType(h) = val {
                            *val = StringType(db.get_string(*h)?.unwrap_or_default().take());
                        }
                    }
                    return Ok(Some(attr));
                }
            }
        }
        Ok(None)
    }

    #[inline]
    pub fn get_pdms_tree_by_name_hash(&self, project_hash: AiosStrHash, db_no: u32) -> Option<PdmsTree> {
        if let Some(db) = self.project_map.get(&project_hash) {
            db.get_tree(db_no).ok()?
        } else {
            None
        }
    }

    ///打印用
    #[inline]
    pub fn get_pretty_attr(&self, refno: RefU64) -> anyhow::Result<HashMap<String, String>> {
        if let Some(attr) = self.get_stringfied_attr(refno)? {
            return Ok(attr.to_string_hashmap());
        }
        Ok(Default::default())
    }

    ///获取世界坐标变换矩阵
    #[inline]
    pub fn get_world_transform(&self, refno: RefU64) -> Option<glam::TransformRT> {
        if let Ok(Some(ref_info)) = self.get_refno_info(refno) {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_world_transform(refno, ref_info.db_no);
            }
        }
        Some(glam::TransformRT::IDENTITY)
    }

    #[inline]
    pub fn get_parent_att_by_type(&self, refno: RefU64, type_name: &str) -> anyhow::Result<Option<AttrMap>> {
        if let Some(ref_info) = self.get_refno_info(refno)? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_parent_att_by_type(refno, ref_info.db_no, type_name);
            }
        }
        Ok(None)
    }

    #[inline]
    pub fn get_cat_ref_in_desi(&self, refno: RefU64) -> Option<RefU64> {
        let att = self.get_attr(refno).ok()??;
        let spre = att.get_foreign_refno("SPRE")?;
        let att = self.get_attr(spre).ok()??;
        att.get_foreign_refno("CATR")
    }

    #[inline]
    pub fn get_cat_att_in_desi(&self, refno: RefU64) -> anyhow::Result<Option<AttrMap>> {
        if let Some(cat_ref) = self.get_cat_ref_in_desi(refno) {
            let att = self.get_attr(cat_ref)?;
            return Ok(att);
        }
        Ok(None)
    }

    ///返回geo data
    #[inline]
    pub fn get_design_geoms(&self, refno: RefU64, cached_mesh_mgr: &mut CachedMeshesMgr) -> anyhow::Result<HashMap<RefU64, Vec<CateBrepShape>>> {
        let mut result_map = HashMap::new();

        if let Some(desi_att) = self.get_attr(refno)? {
            let type_name = desi_att.get_type();
            let is_bran = type_name == "BRAN";
            if !is_bran {
                let geoms = resolve_desi_comp(refno, self).unwrap_or_default();
                // dbg!(&geoms);
                if type_name == "SCTN" || type_name == "STWALL" || type_name == "GENSEC" {
                    result_map.insert(refno, sctn::create_geos(&desi_att, &geoms, self));
                } else {
                    let mut result_shapes = vec![];
                    for geom in geoms.geometries {
                        if let Some(cate_shape) = convert_to_brep_shapes(&geom) {
                            result_shapes.push(cate_shape);
                        }
                    }
                    result_map.insert(refno, result_shapes);
                }
            } else {   //先暂时只让旋转用bran
                let bran_transform = self.get_world_transform(refno).unwrap_or_default();
                let bran_htube_pt = bran_transform.transform_point3(desi_att.get_vec3("HPOS").ok_or(anyhow!("HPOS not exist".to_string()))?);
                let bran_ttube_pt = bran_transform.transform_point3(desi_att.get_vec3("TPOS").ok_or(anyhow!("TPOS not exist".to_string()))?);
                let htube_ref = desi_att.get_foreign_refno("HSTU").unwrap_or_default();
                let mut bore = 0.0f32;
                if let Some(hstube_att) = self.get_attr(htube_ref)? {
                    let hstube_cat_att = self.get_attr(hstube_att.get_foreign_refno("CATR").unwrap_or_default())?.unwrap_or_default();
                    let params = hstube_cat_att.get_f64_vec("PARA").unwrap_or_default();
                    if params.len() >= 2 {
                        bore = params[1] as f32;
                    }
                }
                let mut current_tubing = PdmsTubing {
                    start_pt: bran_htube_pt,
                    end_pt: Vec3::ZERO,
                    bore,
                    finished: false,
                };
                let children = self.get_children(refno)?.unwrap_or_default();
                if children.len() == 0 {
                    if !current_tubing.finished && bran_ttube_pt.distance(current_tubing.start_pt) > TUBI_TOL {
                        current_tubing.end_pt = bran_ttube_pt;
                        current_tubing.finished = true;
                        result_map.insert(refno, vec![current_tubing.convert_to_shape()]);
                    }
                    return Ok(result_map);
                }
                //第一遍完成后，然后生成tubing
                let last_child = children.last().unwrap().clone();
                for child in children {
                    // if child != RefU64::from_two_nums(16501, 1460) {
                    //     continue;
                    // }
                    let world_trans = self.get_world_transform(child).unwrap_or_default();
                    let mut result_shapes = vec![];
                    let geoms = crate::query_cata::resolve_desi_comp(child, self).unwrap_or_default();
                    // dbg!(&geoms);
                    let attr = self.get_attr(child)?.unwrap_or_default();
                    if let Some(arrive) = attr.get_i32("ARRI") {
                        //todo 加入获取arrive position 的方法
                        if geoms.axis_map.contains_key(&arrive) {
                            let p = &geoms.axis_map[&arrive].pt;
                            let a_pos = world_trans.transform_point3(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
                            if !current_tubing.finished && a_pos.distance(current_tubing.start_pt) > TUBI_TOL {
                                current_tubing.end_pt = a_pos;
                                current_tubing.finished = true;
                                result_shapes.push(current_tubing.convert_to_shape());
                            }
                        }
                    }
                    if let Some(lstube) = attr.get_foreign_refno("LSTU") {
                        if let Some(lstube_att) = self.get_attr(lstube)? {
                            let lstube_cat_att = self.get_attr(lstube_att.get_foreign_refno("CATR").unwrap_or_default())?.unwrap_or_default();
                            let params = lstube_cat_att.get_f64_vec("PARA").unwrap_or_default();
                            if params.len() >= 2 {
                                current_tubing.bore = params[1] as f32;
                            }
                        }
                    }
                    if let Some(leave) = attr.get_i32("LEAV") {
                        //todo 加入获取leave position 的方法
                        // current_tubing
                        if geoms.axis_map.contains_key(&leave) {
                            let p = &geoms.axis_map[&leave].pt;
                            let l_pos = world_trans.transform_point3(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
                            current_tubing.start_pt = l_pos;
                            current_tubing.finished = false;
                        }
                    }
                    //管件的生成
                    for geom in geoms.geometries {
                        if let Some(cate_shape) = convert_to_brep_shapes(&geom) {
                            result_shapes.push(cate_shape);
                            // break;
                        }
                    } // end geoms.geometries
                    if child == last_child {
                        if !current_tubing.finished && bran_ttube_pt.distance(current_tubing.start_pt) > TUBI_TOL {
                            current_tubing.end_pt = bran_ttube_pt;
                            current_tubing.finished = true;
                            result_shapes.push(current_tubing.convert_to_shape());
                        }
                    }
                    result_map.insert(child, result_shapes);
                }
            }
        }
        Ok(result_map)
    }

    pub fn get_general_type_refno(&self, refno: RefU64) -> Option<(SmolStr, RefU64)> {
        let mut cur_refno = refno;
        while let Some(attr) = self.get_attr(cur_refno).ok()? {
            let noun_name = attr.get_type_cloned()?;
            if GENRIC_NOUN_NAMES.contains(&noun_name) {
                return Some((noun_name, cur_refno));
            }
            if let Some(owner) = attr.get_owner() {
                cur_refno = owner;
            } else {
                break;
            }
        }
        None
    }

    ///缓存所有几何体
    pub fn cache_geos_data(&mut self, db_code: u32, project: &str) -> anyhow::Result<PdmsMeshMgr> {
        let mut time = Instant::now();

        let project_hash = AiosStr(project.into()).get_u32_hash();
        let mut main_db = self.project_map.get(&project_hash).ok_or(anyhow!(format!("{project} not exist")))?;

        let mut cached_mesh_mgr = CachedMeshesMgr::default();
        let mut inst_map = HashMap::new();
        let mut level_shape_mgr = HashMap::new();
        let mut type_geom_refs_map = HashMap::new();
        let mut type_refs_map = HashMap::new();
        if let Some(tree) = main_db.get_tree(db_code)? {
            let tree = tree.0;
            let root_node_id = tree.root_node_id().unwrap();
            let node_id = tree.root_node_id().unwrap();

            if let Ok(mut nodes) = tree.traverse_level_order_ids(node_id) {
                while let Some(mut cur_node_id) = nodes.next() {
                    let cur_node = tree.get(&cur_node_id).unwrap();
                    let d = cur_node.data();
                    let noun = d.noun;
                    let attr = self.get_attr(d.refno)?;
                    if attr.is_none() { continue; }
                    let attr = attr.unwrap();
                    // if d.refno != RefU64::from_two_nums(23584, 6328)
                    // {
                    //     continue;
                    // }
                    let mut geo_hash = None;
                    let mut color_type = None;
                    let mut item_trans = glam::TransformSRT::IDENTITY;
                    let mut target_refno = d.refno;
                    let mut target_att = attr.clone();
                    let mut target_node_id = cur_node_id.clone();
                    if PRIM_HASH_NOUNS.contains(&noun) {
                        //获得类型和参考号
                        if let Some((noun_name, r)) = self.get_general_type_refno(d.refno) {
                            type_geom_refs_map.entry(r).or_insert(Vec::new()).push(d.refno);
                            // if type_geom_refs_map.contains_key() { }
                            type_refs_map.entry(noun_name.clone()).or_insert(HashSet::new()).insert(r);
                            color_type = Some(noun_name);
                        }
                        if noun == LOOP_NOUN || noun == PLOO_NOUN {
                            let parent = attr.get_owner().unwrap();
                            target_refno = parent;
                            target_node_id = cur_node.parent().unwrap().clone();
                            let mut parent_att = self.get_attr(parent)?.unwrap();
                            let parent_noun_name = parent_att.get_type();
                            let mut loop_verts: Vec<Vec3> = vec![];
                            let mut fradius_vec: Vec<f32> = vec![];
                            if let Some(children_refs) = self.get_children(d.refno)? {
                                for x in children_refs {
                                    if let Some(a) = self.get_attr(x)? {
                                        loop_verts.push(a.get_position().unwrap_or_default());
                                        fradius_vec.push(a.get_f32("FRAD").unwrap_or_default());
                                    } else {
                                        break;
                                    }
                                }
                            }
                            if parent_noun_name == "REVO" {
                                let angle = parent_att.get_f32("ANGL").unwrap_or_default();
                                if angle >= f32::EPSILON {
                                    let revo = Box::new(Revolution {
                                        loop_verts,
                                        angle,
                                        ..Default::default()
                                    });
                                    if revo.check_valid() {
                                        item_trans = revo.get_trans();
                                        let r = cached_mesh_mgr.get_pdms_mesh_hash_key(revo);
                                        geo_hash = Some(r);
                                    }
                                }
                            } else if parent_noun_name != "NXTR" && parent_noun_name != "NREV" && parent_noun_name != "SCREED" {
                                let mut height = attr.get_f32("HEIG").unwrap_or(parent_att.get_f32("HEIG").unwrap_or_default());
                                let extrusion = Box::new(Extrusion {
                                    verts: loop_verts,
                                    height,
                                    fradius_vec,
                                    ..Default::default()
                                });
                                if extrusion.check_valid() {
                                    item_trans = extrusion.get_trans();
                                    if noun == PLOO_NOUN {
                                        if let Some(sjus) = attr.get_string("SJUS") {
                                            if sjus.as_str() == "UTOP" || sjus.as_str() == "DTOP" {
                                                item_trans.translation = item_trans.translation + Vec3::new(0.0, 0.0, -height);
                                            }
                                        }
                                    }
                                    let r = cached_mesh_mgr.get_pdms_mesh_hash_key(extrusion);
                                    geo_hash = Some(r);
                                }
                            } //end of LOOP_NOUN
                            target_att = parent_att;
                        } else if noun == POHE_NOUN {  //多面体, try to save the leaf nodes in database
                            let children_hash = self.get_children(d.refno)?.unwrap_or_default();
                            let mut facet = Facet::default();
                            for x in children_hash {
                                let refs = self.get_children(x)?.unwrap_or_default();
                                let mut vertices: Vec<[f32; 3]> = vec![];
                                let mut tv = vec![];
                                let v_cnt = refs.len();
                                if v_cnt >= 3 {
                                    for x in refs {
                                        let mut contour = Contour::default();
                                        let v = self.get_attr(x)?.unwrap_or_default().get_position().unwrap_or_default();
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
                                item_trans = facet.get_trans();
                                let r = cached_mesh_mgr.get_pdms_mesh_hash_key(Box::new(facet));
                                geo_hash = Some(r);
                            }
                        } else if noun == SPINE_NOUN {
                            let parent = attr.get_owner().unwrap();
                            target_refno = parent;
                            target_node_id = cur_node.parent().unwrap().clone();
                            let mut parent_att = self.get_attr(parent)?.unwrap();
                            let parent_noun_name = parent_att.get_type();
                            //todo 假定圆心是 O
                            let center = Vec3::ZERO;
                            let params = parent_att.get_f64_vec("DESP").unwrap_or_default();
                            if params.len() >= 2 {
                                let thick = params[0] as f32;
                                let height = params[1] as f32;
                                if height >= f32::EPSILON && thick >= f32::EPSILON {
                                    let mut verts: Vec<Vec3> = vec![];
                                    let mut fradius_vec: Vec<f32> = vec![];
                                    if let Some(children_refs) = self.get_children(d.refno)? {
                                        for x in children_refs {
                                            if let Some(a) = self.get_attr(x)? {
                                                let p = a.get_position().unwrap_or_default();
                                                verts.push(p);
                                                let c_rad = a.get_f32("RADI").unwrap_or_default();
                                                if abs_diff_ne!(c_rad, 0.0) {
                                                    fradius_vec.push(c_rad);
                                                }
                                            }
                                        }
                                    }
                                    let extrusion = Box::new(Extrusion {
                                        verts,
                                        height,
                                        fradius_vec,
                                        cur_type: CurveType::Spine(thick),
                                        ..Default::default()
                                    });
                                    // dbg!(&extrusion);
                                    if extrusion.check_valid() {
                                        item_trans = extrusion.get_trans();
                                        let r = cached_mesh_mgr.get_pdms_mesh_hash_key(extrusion);
                                        geo_hash = Some(r);
                                    }
                                } // end height
                            }  //end params.len() >= 2
                            target_att = parent_att;
                        } else {
                            if let Some(brep_obj) = attr.create_brep_shape() {
                                if brep_obj.check_valid() {
                                    item_trans = brep_obj.get_trans();
                                    let r = cached_mesh_mgr.get_pdms_mesh_hash_key(brep_obj);
                                    geo_hash = Some(r);
                                }
                            }
                        }
                    } else {
                        continue;
                        let ele_type = attr.get_type();
                        let owner = self.get_attr(attr.get_owner().unwrap())?;
                        let has_catref = attr.get_foreign_refno("CATR").is_some() || attr.get_foreign_refno("SPRE").is_some();
                        //针对管道特殊处理
                        if ele_type == "BRAN" || (owner.is_some() && owner.unwrap().get_type() != "BRAN" && has_catref) {
                            let mut node_ids_map = HashMap::new();
                            for node_id in cur_node.children() {
                                let data = tree.get(node_id).unwrap().data();
                                node_ids_map.insert(data.refno, node_id.clone());
                            }
                            let brep_shapes = self.get_design_geoms(d.refno, &mut cached_mesh_mgr)?;
                            // dbg!(&brep_shapes);
                            for (cur_refno, shapes) in brep_shapes {
                                //记录对应的不同颜色类型
                                if let Some((noun_name, r)) = self.get_general_type_refno(d.refno) {
                                    type_geom_refs_map.entry(r).or_insert(Vec::new()).push(d.refno);
                                    type_refs_map.entry(noun_name.clone()).or_insert(HashSet::new()).insert(r);
                                    color_type = Some(noun_name);
                                }
                                //维护每个节点有那些几何实例
                                let ancestors = tree.ancestors(&cur_node_id).unwrap();
                                for ancestor in ancestors {
                                    let p_refno = ancestor.data().refno;
                                    level_shape_mgr.entry(p_refno).or_insert(RefU64Vec::default()).push(cur_refno);
                                }
                                //当前自身也要加进去
                                if d.refno != cur_refno {
                                    level_shape_mgr.entry(d.refno).or_insert(RefU64Vec::default()).push(cur_refno);
                                }
                                let desi_trans_origin = self.get_world_transform(cur_refno).unwrap_or_default();
                                for shape in shapes {
                                    let CateBrepShape {
                                        brep_shape,
                                        mut transform,
                                        visible,
                                        is_tubing,
                                    } = shape;
                                    if !visible || !brep_shape.check_valid() { continue; }
                                    item_trans = brep_shape.get_trans();
                                    if !brep_shape.check_valid() {
                                        continue;
                                    }
                                    let geo_hash = cached_mesh_mgr.get_pdms_mesh_hash_key(brep_shape);
                                    let mut desi_trans = desi_trans_origin.clone();
                                    if !is_tubing {
                                        desi_trans.translation = desi_trans.translation + desi_trans.rotation * transform.translation;
                                        desi_trans.rotation = desi_trans.rotation * transform.rotation;
                                    } else {
                                        desi_trans.translation = transform.translation;
                                        desi_trans.rotation = transform.rotation;
                                    }
                                    let mut bbox = cached_mesh_mgr.get_bbox(&geo_hash).unwrap();
                                    bbox.scaled(&item_trans.scale);
                                    let geom_data = EleGeoInstData {
                                        geo_hash,
                                        bbox,
                                        global_transform: (desi_trans.rotation, desi_trans.translation, item_trans.scale),
                                        visible: true,
                                        generic_type: color_type.clone().unwrap_or_default(),
                                        zone_refno: self.get_parent_att_by_type(cur_refno, "ZONE")?.map(|x| x.get_refno().unwrap_or_default()).unwrap_or_default(),
                                        node_id: node_ids_map.get(&cur_refno).map(|x| x.clone()).unwrap_or(cur_node_id.clone()),
                                    };
                                    inst_map.entry(cur_refno).or_insert(Vec::new()).push(geom_data);
                                }
                            }
                        }
                    }
                    //处理有几何体返回的情况，需要加入到几何列表里
                    if let Some(geo_hash) = geo_hash {
                        let mut ancestors = tree.ancestors(&cur_node_id).unwrap();
                        //维护每个节点有那些几何实例
                        for ancestor in ancestors {
                            let p_refno = ancestor.data().refno;
                            level_shape_mgr.entry(p_refno).or_insert(RefU64Vec::default()).push(target_refno);
                        }
                        let tr: TransformSRT = item_trans * self.get_world_transform(target_refno).unwrap_or_default();
                        // let xyz = tr.rotation.to_euler(glam::EulerRot::XYZ);
                        // //dbg!((xyz.0.to_degrees(), xyz.1.to_degrees(), xyz.2.to_degrees()));
                        // //dbg!(&tr);
                        let mut bbox = cached_mesh_mgr.get_bbox(&geo_hash).unwrap();
                        bbox.scaled(&tr.scale);
                        let geom_data = EleGeoInstData {
                            geo_hash,
                            bbox,
                            global_transform: (tr.rotation, tr.translation, tr.scale),
                            visible: target_att.is_visible_by_level(None).unwrap_or(true),
                            generic_type: color_type.unwrap_or_default(),
                            zone_refno: self.get_parent_att_by_type(target_refno, "ZONE")?.unwrap().get_refno().unwrap(),
                            node_id: target_node_id,
                        };
                        inst_map.entry(target_refno).or_insert(Vec::new()).push(geom_data);
                    } // end of insert geo_map
                }
            }
        }

        // let mut file = File::create(format!("{db_code}_geoms.json")).unwrap();
        // let serialized = serde_json::to_string(&inst_map).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();


        let mut file = File::create(format!("type_refs_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&type_geom_refs_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        let mut file = File::create(format!("type_refs.json")).unwrap();
        let serialized = serde_json::to_string(&type_refs_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        //需要把rooms单独标记出来
        // let mut file = File::create(format!("room_geoms.json")).unwrap();
        // let serialized = serde_json::to_string(&room_geom_refs_map).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();

        // cached_mesh_mgr.serialize_to_json_file();
        // cached_mesh_mgr.serialize_to_bin_file();
        let mgr = PdmsMeshMgr {
            inst_mgr: ShapeInstancesMgr {
                inst_map
            },
            cached_mesh_mgr,
            level_shape_mgr,
        };
        println!("cache all geoms costs: {}ms", time.elapsed().as_millis());
        mgr.serialize_to_bin_file(db_code);
        Ok(mgr)
    }

    //todo 房间号的算法移植
    pub fn build_collision_world(&mut self, project_str: &str, db_code: u32) -> anyhow::Result<()> {
        let mut world = GLOBAL_COLLISION_WORLD.lock().unwrap();
        // *world = CollisionWorld::<f32, (RefU64, RefU64)>::new(0.01f32);
        let query = GeometricQueryType::Proximity(0.0);
        let groups = CollisionGroups::new();
        let mut room_aabb_map = HashMap::new();
        //取得所有的rooms
        let mesh_mrg = PdmsMeshMgr::deserialize_from_bin_file(db_code).unwrap_or_default();

        let mut file = File::open(format!("type_refs_geoms.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        let type_geom_refs_map: HashMap<RefU64, Vec<RefU64>> = serde_json::from_slice(&buf).unwrap();

        let mut file = File::open(format!("type_refs.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        let type_refs_map: HashMap<SmolStr, Vec<RefU64>> = serde_json::from_slice(&buf).unwrap();
        let room_key = SmolStr::new("ROOM");
        if type_refs_map.contains_key(&room_key) {
            dbg!(&type_refs_map[&room_key]);
            for v in &type_refs_map[&room_key] {
                // dbg!(mesh_mrg.get_instants_data(*v));
                let geo_data_map = mesh_mrg.get_instants_data(*v);
                for (k, geo_data_vec) in geo_data_map {
                    for geo_data in geo_data_vec {
                        room_aabb_map.insert(*v, geo_data.clone());
                    }
                }
            }
        }

        //
        // //暂时用json，方便调试
        // let mut file = File::open(format!("{db_code}_geoms.json")).unwrap();
        // let mut buf: Vec<u8> = Vec::new();
        // file.read_to_end(&mut buf)?;
        // let geo_map: HashMap<SmolStr, EleGeoInstData> = serde_json::from_slice(&buf).unwrap();
        // let mesh_mgr: PdmsMeshMgr = PdmsMeshMgr::deserialize_from_bin_file(db_code)?;
        //
        // // 暂时找到所有的设备，在这里进行遍历，获得包围盒信息
        // let equip_hash = db1_hash("EQUI");
        // let equip_key = combine_to_u64(equip_hash, db_code);

        // //查询出所有的设备的几何体
        for (generic_ref, v) in type_geom_refs_map {
            for refno in v {
                // if refno != RefU64::from_two_nums(23584, 128) {
                //     continue;
                // }
                let geo_data_map = mesh_mrg.get_instants_data(refno);
                for (k, geo_data_vec) in geo_data_map {
                    // dbg!(k.to_refno_str());
                    //数量太多需要合并
                    // let aabb = Aabb::default();
                    //self.half_extents = /*t.rotation **/ Vec3A::new(s.x * h.x, s.y * h.y, s.z * h.z);
                    for geo_data in geo_data_vec {
                        if geo_data.generic_type != "ROOM" {
                            let extents = geo_data.bbox.get_half_extents();
                            let center = geo_data.bbox.get_center();
                            let (r, t, s) = geo_data.global_transform;
                            // dbg!(&s);
                            // dbg!(&geo_data.bbox);
                            let extents = na::Vector3::new(extents.x, extents.y, extents.z);
                            // dbg!(&extents);
                            let shape = ShapeHandle::new(Cuboid::new(extents));
                            let t = t + r * center;
                            let translation = na::Vector3::new(t.x, t.y, t.z);
                            // dbg!(&translation);
                            let (axis, angle) = r.to_axis_angle();
                            let axis_angle = na::Vector3::new(axis.x, axis.y, axis.z) * angle;
                            let iso = Isometry3::new(translation, axis_angle);
                            //需要用整体元件的来做为 AABB，而不是单个的，减少插入的个数,
                            //第二次细致检查的时候，需要改成用基本体一个个去判断
                            world.add(iso, shape.clone(), groups, query, (generic_ref, k));
                        }
                    }
                }
            }
        }

        world.update();
        dbg!("world update ok");
        let mut aabb_contained = HashMap::new();
        //不能用参考号作为参考了，需要用node id, 有可能同一个层级在不同的地方出现过了
        //node id -> geom refnos
        let mut ssc_nodeid_geom_refs = HashMap::new();
        let mut ssc_nodeid_map: HashMap<RefU64, NodeId> = HashMap::new();
        let mut ssc_tree = PdmsTree::default();
        let mut ele_id_tree = self.get_pdms_tree(project_str, db_code).ok_or(anyhow!("Tree not found".to_string()))?;
        let ele_root_id = ele_id_tree.0.root_node_id().unwrap();
        let ele_root_data = ele_id_tree.0.get(ele_root_id).unwrap().data().clone();
        let root_id: NodeId = ssc_tree.0.insert(Node::new(ele_root_data), AsRoot).unwrap();
        let mut room_final_contained = HashMap::new();
        // let mut room_geo_refs_map = HashMap::new();
        for (room_refno, room_geo) in room_aabb_map {
            // if k != RefU64::from_two_nums(23584, 65) {
            //     continue;
            // }
            // room_geo_refs_map.insert(k.to_refno_str(), room_geo_refnos.into_iter().map(|x| x.to_refno_str()).collect::<Vec<_>>());
            // dbg!(k.to_refno_str());
            let e = room_geo.bbox.get_half_extents();
            let c = room_geo.bbox.get_center();
            let (r, t, s) = room_geo.global_transform;
            let t = t + r * c;
            let aabb = AABB::from_half_extents(na::Point3::new(t.x, t.y, t.z), na::Vector3::new(e.x, e.y, e.z));
            let mesh_indx = room_geo.geo_hash.clone();
            // dbg!(&room_geo);
            let translation = na::Vector3::new(t.x, t.y, t.z);
            let (axis, angle) = r.to_axis_angle();
            let axis_angle = na::Vector3::new(axis.x, axis.y, axis.z) * angle;
            let room_iso = Isometry3::new(translation, axis_angle);
            let room_tri_mesh = mesh_mrg.cached_mesh_mgr.get_mesh(&mesh_indx).unwrap().get_tri_mesh(TransformSRT {
                rotation: r,
                translation: t,
                scale: s,
            });
            // dbg!(&aabb);
            let room_node_id = if ssc_nodeid_map.contains_key(&room_refno) {
                ssc_nodeid_map[&room_refno].clone()
            } else {
                let node_id = self.get_node_id(room_refno).unwrap();
                let node_data = ele_id_tree.0.get(&node_id).unwrap().data().clone();
                let room_node_id = ssc_tree.0.insert(Node::new(node_data), UnderNode(&root_id)).unwrap();
                ssc_nodeid_map.insert(room_refno, room_node_id.clone());
                room_node_id
            };

            let interferences = world.interferences_with_aabb(&aabb, &groups);
            for x in interferences {
                let generic_refno = x.1.data().0.clone();   //类型的参考号
                let geom_refno = x.1.data().1.clone();
                // dbg!(generic_refno.to_refno_str());
                //暂时做了两层的结构，需要按照原来的结构还原，剔除不属于room的构件
                let type_node_id = if ssc_nodeid_map.contains_key(&generic_refno) {
                    ssc_nodeid_map[&generic_refno].clone()
                } else {
                    let node_id = self.get_node_id(generic_refno).unwrap();
                    let node_data = ele_id_tree.0.get(&node_id).unwrap().data().clone();
                    let type_node_id = ssc_tree.0.insert(Node::new(node_data), UnderNode(&room_node_id)).unwrap();
                    ssc_nodeid_map.insert(generic_refno, type_node_id.clone());
                    type_node_id
                };

                let node_id = self.get_node_id(geom_refno).unwrap();
                let node_data = ele_id_tree.0.get(&node_id).unwrap().data().clone();
                let new_id = ssc_tree.0.insert(Node::new(node_data), UnderNode(&type_node_id)).unwrap();
                //todo 需要把层级移动过来
                ssc_nodeid_map.insert(geom_refno, new_id.clone());
                //存储这个索引关系

                ssc_nodeid_geom_refs.entry(type_node_id).or_insert(Vec::new()).push(geom_refno);
                ssc_nodeid_geom_refs.entry(new_id).or_insert(Vec::new()).push(geom_refno);
                ssc_nodeid_geom_refs.entry(room_node_id.clone()).or_insert(Vec::new()).push(room_refno);
                ssc_nodeid_geom_refs.entry(room_node_id.clone()).or_insert(Vec::new()).push(geom_refno);

                aabb_contained.entry(room_refno).or_insert(Vec::new()).push(geom_refno);
                let geo_data_map = mesh_mrg.get_instants_data(geom_refno);
                for (_, geo_data_vec) in geo_data_map {
                    for geo_data in geo_data_vec {
                        let mesh_indx = &geo_data.geo_hash;
                        let pt = geo_data.global_transform.1;
                        let first_pt = ncollide3d::na::Point3::new(pt.x, pt.y, pt.z);
                        let ray_x = Ray::new(first_pt, ncollide3d::na::Vector3::z());
                        let ray_neg_x = Ray::new(first_pt, -ncollide3d::na::Vector3::z());

                        let ray_x = room_tri_mesh.toi_with_ray(&Isometry3::identity(), &ray_x, std::f32::MAX, false);
                        let ray_neg_x = room_tri_mesh.toi_with_ray(&Isometry3::identity(), &ray_neg_x, std::f32::MAX, false);
                        if ray_x.is_some() && ray_neg_x.is_some() {
                            // aabb_contained.entry(k.to_refno_str()).or_insert(HashSet::new()).insert(generic_refno.to_refno_str());
                            //满足要求，插入到树中
                            room_final_contained.entry(room_refno.to_refno_str()).or_insert(HashSet::new()).insert(geom_refno.to_refno_str());
                        } else {
                            println!("{} exclude from trimesh check", geom_refno.to_refno_str());
                        }
                    }
                }
            }
        }
        dbg!(aabb_contained.len());
        dbg!(room_final_contained.len());
        //
        // let mut file = File::create(format!("../web-aios/room_contained_equips.json")).unwrap();
        // let serialized = serde_json::to_string(&aabb_contained).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();
        //
        let mut file = File::create(format!("room_contained.json")).unwrap();
        let serialized = serde_json::to_string(&aabb_contained).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        //ssc 需要用node id 去获取需要显示的 geoms
        let mut file = File::create(format!("ssc_nodeid_geom_refs.bin")).unwrap();
        let serialized = bincode::serialize(&ssc_nodeid_geom_refs).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        // ssc_nodeid_geom_refs
        // ssc_tree.serialize_to_bin_file_with_name("ssc_sample", db_code);
        //
        //
        // let mut file = File::create(format!("../web-aios/room_geo_refs_map.json")).unwrap();
        // let serialized = serde_json::to_string(&room_geo_refs_map).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();
        Ok(())
    }

    pub fn set_ssc_room_tree(&mut self, project_str: &str, db_code: u32) -> anyhow::Result<()> {
        let mut file = File::open(format!("StringLookupTable_{}.bin", db_code))?;
        let mut buf = vec![];
        file.read_to_end(&mut buf);
        let mut string_look_up = bincode::deserialize::<StringLookupTable>(&buf)?;

        let (mut ssc_tree, root_id) = set_ssc_tree(&mut string_look_up)?;
        let room_map = get_room_refnos(project_str, self.clone());

        let one = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("1层(-6.70m)"))?;
        let two = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("2层(-3.30m)"))?;
        let three = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("3层(0.00m)"))?;
        let four = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("4层(+3.60m)"))?;
        let five = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("5层(+7.5m)"))?;
        let six = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("6层(+13.50m)"))?;
        let seven = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("7层(+16.50m)"))?;
        let eight = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("8层(+22.00m及以上)"))?;
        let nine = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("9层(内穹顶)"))?;

        let mut room_node_map = HashMap::new(); // 存放所有房间名的 nodeid
        let room_info = get_room_info_from_excel()?;

        for (k, v) in room_info {
            match k.as_str() {
                "1" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&one), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "2" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&two), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "3" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&three), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "4" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&four), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "5" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&five), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "6" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&six), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "7" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&seven), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "8" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&eight), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                "9" => {
                    for name in v {
                        let node = insert_tree_node(&mut string_look_up, &mut ssc_tree, Some(&nine), name.clone())?;
                        room_node_map.entry(name).or_insert(node);
                    }
                }
                _ => {}
            }
        }

        let project_dbs = self.project_map.get(&AiosStr(SmolStr::new(project_str)).get_u32_hash()).ok_or(anyhow!("can not find this project"))?;
        let string_db = project_dbs.string_db.clone();

        for (k, v) in room_map {
            let mut room_name = bincode::deserialize::<AiosStr>(&string_db.get(k.to_be_bytes())?.ok_or(anyhow!("can not find string hash"))?)?.0;
            room_name = get_split_room_name(room_name);
            if let Some(node) = room_node_map.get(&room_name) {
                for refno in v {
                    if let Ok(Some(project_info)) = self.get_refno_info(refno) {
                        let mut tree = self.get_pdms_tree(project_str, project_info.db_no).ok_or(anyhow!("can not find tree"))?;
                        // 找到改参考号在pdms树中的 elenode
                        let refno_node_id = self.get_node_id(refno).ok_or(anyhow!("can not find node id in tree"))?;
                        let node_data = tree.0.get(&refno_node_id).unwrap().data().clone();
                        ssc_tree.0.insert(Node::new(node_data), UnderNode(&node)).unwrap();
                    }
                }
            }
        }

        ssc_tree.serialize_to_bin_file_with_name("ssc_sample", db_code);
        string_look_up.serialize_to_bin_file(db_code);
        dbg!("ssc_tree write ok");
        Ok(())
    }
}

/// 创建 ssc 树之前固定的层级
pub fn set_ssc_tree(mut look_up: &mut StringLookupTable) -> anyhow::Result<((PdmsTree, NodeId))> {
    let mut ssc_tree = PdmsTree::default();
    let root_id = insert_tree_node(&mut look_up, &mut ssc_tree, None, SmolStr::new(r"“华龙一号”标准SSC结构"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("土建子项"))?;

    let room_node = insert_tree_node(&mut look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("安装厂房"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("系统"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("设备"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&root_id), SmolStr::new("全局性信息"))?;

    let ni_node = insert_tree_node(&mut look_up, &mut ssc_tree, Some(&room_node), SmolStr::new("NI"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&room_node), SmolStr::new("CI"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&room_node), SmolStr::new("BOP"))?;

    let one_unit = insert_tree_node(&mut look_up, &mut ssc_tree, Some(&ni_node), SmolStr::new("一号机组"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&ni_node), SmolStr::new("二号机组"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&ni_node), SmolStr::new("双机组共用"))?;

    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1DX"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1DU"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1KA"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1KP"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1KY"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1LA"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1NH"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1PR"))?;

    let rx_node = insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1RX"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1SL"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1SR"))?;
    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&one_unit), SmolStr::new("1UR"))?;

    insert_tree_node(&mut look_up, &mut ssc_tree, Some(&rx_node), SmolStr::new("安装分区"))?;
    let install_level = insert_tree_node(&mut look_up, &mut ssc_tree, Some(&rx_node), SmolStr::new("安装层位"))?;

    Ok((ssc_tree, install_level))
}

/// 解析 excel 表单 ，找到每一层下面所有的房间号
fn get_room_info_from_excel() -> anyhow::Result<HashMap<String, Vec<SmolStr>>> {
    let mut r = HashMap::new();
    let mut workbook: Xlsx<_> = open_workbook("test.xlsx")?;
    let range = workbook.worksheet_range("Sheet1")
        .ok_or(anyhow!("Cannot find 'Sheet1'"))??;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    while let Some(result) = iter.next() {
        let v: RoomExcelData = result?;
        if v.安装厂房 == Some("RX".to_string()) {
            let room_name = SmolStr::new(v.房间代码.ok_or(anyhow!("房间代码 filed is empty"))?);
            r.entry(v.安装层位.ok_or(anyhow!("安装层位 filed is empty"))?).or_insert_with(Vec::new).push(room_name.clone());
        }
    }
    Ok(r)
}

fn insert_tree_node(look_up: &mut StringLookupTable, tree: &mut PdmsTree, node_id: Option<&NodeId>, name: SmolStr) -> anyhow::Result<NodeId> {
    let name_hash = AiosStr(name.clone()).get_u32_hash();
    look_up.lookup.entry(name_hash).or_insert(AiosStr(name));
    let node_id = if let Some(node_id) = node_id {
        tree.0.insert(Node::new(EleNode::set_default_name(name_hash)), UnderNode(node_id))?
    } else {
        tree.0.insert(Node::new(EleNode::set_default_name(name_hash)), AsRoot)?
    };
    Ok(node_id)
}

// 获得 房间下的所有refno
pub fn get_room_refnos(project_str: &str, db: AiosDBManager) -> DashMap<AiosStrHash, Vec<RefU64>> {
    let mut r = DashMap::new();
    if let Some(dbs) = db.project_map.get(&AiosStr(SmolStr::new(project_str)).get_u32_hash()) {
        let db = dbs.room_db.clone();
        for val_opt in db.iter() {
            if let Ok((_, v)) = val_opt {
                let room_code = bincode::deserialize::<RoomCode>(&v.to_vec()).unwrap();
                r.entry(room_code.name_hash).or_insert_with(Vec::new).push(room_code.refno);
            }
        }
    }
    r
}

/// 为 ssc 树结构手动创建 pdms 树节点
fn set_pdms_node_attr(db: Db, refno: RefU64, owner: RefU64, noun_name: SmolStr, mut attr: AttrMap) -> anyhow::Result<()> {
    attr.insert_by_att_name("OWNER", RefU64Type(owner));
    attr.insert_by_att_name("TYPE", WordType(noun_name));
    attr.insert_by_att_name("REFNO", RefU64Type(refno.into()));
    db.insert(refno.to_be_bytes(), bincode::serialize(&attr)?);
    Ok(())
}

/// 获得分割过的room name
fn get_split_room_name(room_name: SmolStr) -> SmolStr {
    if room_name.contains("RM") {
        let vals = room_name.split('-').collect::<Vec<_>>();
        if vals.len() > 2 {
            return SmolStr::new(vals[2]);
        }
    }
    room_name
}

// 房间信息 excel 字段
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoomExcelData {
    pub 房间代码: Option<String>,
    pub 所属机组: Option<u32>,
    pub 安装厂房: Option<String>,
    pub 区域: Option<String>,
    pub 安装层位: Option<String>,
    pub 厂房: Option<String>,
    pub 分区: Option<String>,
    pub 层位及标高: Option<String>,
    pub 序号: Option<u32>,
}

/// DB 单个数据库管理
#[derive(Debug, Clone)]
pub struct AiosPdmsProjectSled {
    pub project: String,
    pub dir: String,
    // pub storage: Storage,
    //pdms data directory
    // pub att_db_map: HashMap<u32, Database>,
    //att map 需要做分库, db_number -> database
    pub db_no_list: Vec<u32>,

    pub all_att_db: sled::Db,
    pub types_db: sled::Db,
    pub children_db: sled::Db,
    pub tree_db: sled::Db,
    pub info_db: sled::Db,
    pub string_db: sled::Db,
    pub mdb_name: Option<String>,
    pub version_db: sled::Db,
    pub room_db: sled::Db,
}


impl AiosPdmsProjectSled {
    ///获得children的数据库
    #[inline]
    pub fn get_children_database(&self) -> sled::Db {
        self.children_db.clone()
    }

    ///获得tree的数据库
    #[inline]
    pub fn get_info_database(&self) -> sled::Db {
        self.info_db.clone()
    }

    ///获得type refs的数据库
    #[inline]
    pub fn get_type_refs_database(&self) -> sled::Db {
        self.types_db.clone()
    }

    ///获得tree的数据库
    #[inline]
    pub fn get_tree_database(&self) -> sled::Db {
        self.tree_db.clone()
    }

    ///获得strings的数据库
    #[inline]
    pub fn get_string_database(&self) -> sled::Db {
        self.string_db.clone()
    }

    #[inline]
    pub fn get_room_database(&self) -> sled::Db { self.room_db.clone() }

    pub fn init(project: &str, dir: &str, info_db: Db) -> anyhow::Result<Self> {
        let cur_project = format!("./AIOS_DBS/{project}");

        let all_att_db = sled::open(format!("AIOS_DBS/{}/attr.sled", project)).expect("Create db file");
        let children_db = sled::open(format!("AIOS_DBS/{}/children.sled", project)).expect("Create db file");
        let types_db = sled::open(format!("AIOS_DBS/{}/type_eles.sled", project)).expect("Create db file");
        let string_db = sled::open(format!("AIOS_DBS/{}/names.sled", project)).expect("Create db file");
        let tree_db = sled::open(format!("AIOS_DBS/{}/tree.sled", project)).expect("Create db file");
        let version_db = sled::open(format!("AIOS_DBS/{}/version.sled", project)).expect("Create db file");
        let room_db = sled::open(format!("AIOS_DBS/{}/room.sled", project)).expect("Create db file");

        Ok(Self {
            project: project.to_string(),
            dir: dir.to_string(),
            // storage,
            db_no_list: vec![],
            all_att_db,
            types_db,
            children_db,
            tree_db,
            info_db,
            string_db,
            mdb_name: None,
            version_db,
            room_db,
        })
    }

    //按需要打开database
    #[inline]
    pub fn get_attr(&self, refno: RefU64, db_no: u32) -> anyhow::Result<Option<AttrMap>> {
        let bytes = self.all_att_db.get(&refno.get_sled_key())
            .map_err(|_| anyhow!("get attr error".to_string()))?;
        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<AttrMap>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    #[inline]
    pub fn get_string(&self, h: u32) -> anyhow::Result<Option<AiosStr>> {
        let bytes = self.get_string_database().get(&h.to_be_bytes())
            .map_err(|_| anyhow!("get string error".to_string()))?;
        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<AiosStr>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    #[inline]
    pub fn get_tree(&self, dbno: u32) -> anyhow::Result<Option<PdmsTree>> {
        let bytes = self.get_tree_database().get(&dbno.to_be_bytes())
            .map_err(|_| anyhow!("get string error".to_string()))?;
        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<PdmsTree>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    #[inline]
    pub fn get_node_id(&self, refno: RefU64) -> anyhow::Result<Option<NodeId>> {
        let bytes = self.get_tree_database().get(&refno.get_sled_key())
            .map_err(|_| anyhow!("get string error".to_string()))?;

        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<NodeId>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    #[inline]
    pub fn get_children(&self, refno: RefU64) -> anyhow::Result<Option<RefU64Vec>> {
        let bytes = self.get_children_database().get(&refno.get_sled_key())
            .map_err(|_| anyhow!("get string error".to_string()))?;

        match bytes {
            None => Ok(None),
            Some(d) => {
                Ok(Some(bincode::deserialize::<RefU64Vec>(&*d).map_err(|e| anyhow!(e.to_string()))?))
            }
        }
    }

    //包含自己
    pub fn get_ancestors_attrs(&self, refno: RefU64, db_no: u32) -> Vec<AttrMap> {
        let mut cur_refno = refno;
        let mut r = vec![];
        while let Ok(Some(attr)) = self.get_attr(cur_refno, db_no) {
            if let Some(owner) = attr.get_owner() {
                r.push(attr);
                cur_refno = owner;
            } else {
                break;
            }
        }
        r
    }

    pub fn get_parent_att_by_type(&self, refno: RefU64, db_no: u32, type_name: &str) -> anyhow::Result<Option<AttrMap>> {
        let mut cur_refno = refno;
        let mut r = None;
        while let Some(attr) = self.get_attr(cur_refno, db_no)? {
            if let Some(owner) = attr.get_owner() {
                if attr.get_type() == type_name {
                    r = Some(attr);
                    break;
                }
                cur_refno = owner;
            } else {
                break;
            }
        }
        Ok(r)
    }


    ///获得世界坐标系
    pub fn get_world_transform(&self, refno: RefU64, db_no: u32) -> Option<glam::TransformRT> {
        let mut ancestors = self.get_ancestors_attrs(refno, db_no);
        ancestors.reverse();
        let mut rotation = Quat::IDENTITY;
        let mut translation = Vec3::ZERO;
        let mut parent: Option<Quat> = None;
        for attr in ancestors {
            let t = if attr.get_type() == "SCTN" || attr.get_type() == "STWALL" {
                let tr = TransformRT {
                    rotation,
                    translation,
                };
                let mut final_rot = Quat::IDENTITY;
                let poss = attr.get_poss()?;
                let pose = attr.get_pose()?;
                let w_poss = tr.transform_point3(poss);
                let w_pose = tr.transform_point3(pose);
                let extru_dir: Vec3 = (pose - poss).normalize();
                let bangle = attr.get_f32("BANG").unwrap_or_default();
                //如果和Z轴平行，需要使用Y轴作为参考轴
                let d = extru_dir.dot(Vec3::Z).abs();

                let mut ref_axis = if abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else { Vec3::Z };

                let p_axis = ref_axis.cross(extru_dir).normalize();
                let y_axis = extru_dir.cross(p_axis).normalize();
                final_rot = Quat::from_mat3(&glam::f32::Mat3::from_cols_array_2d(
                    &[p_axis.to_array(), y_axis.to_array(), extru_dir.to_array()]
                )) * Quat::from_rotation_z(bangle.to_radians());
                final_rot
            } else {
                attr.get_rotation().unwrap_or_default()
            };
            translation = translation + rotation * attr.get_position().unwrap_or_default();
            rotation = rotation * t;
        }
        Some(glam::TransformRT {
            rotation,
            translation,
        })
    }

    // let all_att_db = sled::open(format!("AIOS_DBS/{}/attr.sled", project)).expect("Create db file");
    // let children_db = sled::open(format!("AIOS_DBS/{}/children.sled", project)).expect("Create db file");
    // let types_db = sled::open(format!("AIOS_DBS/{}/type_eles.sled", project)).expect("Create db file");
    // let string_db = sled::open(format!("AIOS_DBS/{}/names.sled", project)).expect("Create db file");
    // let tree_db = sled::open(format!("AIOS_DBS/{}/tree.sled", project)).expect("Create db file");
    // let version_db = sled::open(format!("AIOS_DBS/{}/version.sled", project)).expect("Create db file");
    // let room_db = sled::open(format!("AIOS_DBS/{}/room.sled", project)).expect("Create db file");
    fn create_tables(project: &str){

        Self::get_attr_conn(project, true);

        let table_name = format!("{project}:trees");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
        let table_name = format!("{project}:children");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
        let table_name = format!("{project}:names");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
        let table_name = format!("{project}:version");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
        let table_name = format!("{project}:rooms");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
        let table_name = format!("{project}:ref_infos");
        let mytable = Keymap::new(&table_name)
            .set_ktype(KeymapType::Binstr)
            .set_vtype(KeymapType::Binstr);
        con.create_table(mytable);
    }

    #[inline]
    fn get_attr_conn(project: &str, creat: bool) -> Connection{
        let mut con = Connection::new("127.0.0.1", 2003).unwrap();
        let table_name = format!("{project}:attrs");
        if creat {
            con.create_keyspace(project);
            let mytable = Keymap::new(&table_name)
                .set_ktype(KeymapType::Binstr)
                .set_vtype(KeymapType::Binstr);
            con.create_table(mytable);
        }
        con.switch(&table_name).unwrap();
        con
    }

    pub fn sync_total(&self, need_parsing_files: &Option<Vec<String>>) -> anyhow::Result<()> {
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(&project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();

        let mut children_files = fs::read_dir(target_dir)?.into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();

        // Self::create_tables(project.as_str());

        Self::get_attr_conn(project.as_str(), true);

        let versions_map = Arc::new(DashMap::new());
        children_files.par_iter().for_each(|path| {
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
            if !file_name.ends_with("com") && !file_name.ends_with("mis") {
                if need_parsing_files.is_none() || need_parsing_files.as_ref().unwrap().contains(&file_name) {
                    let file_name = file_name.as_str();
                    println!("path={:?}", file_name);
                    if let Ok(PdmsDbData {
                                  all_attr_map,
                                  ele_id_tree,
                                  type_ele_map,
                                  refno_node_id_map,
                                  string_lookup,
                                  refno_info_map,
                                  children_map,
                                  db_no,
                                  field_no,
                                  version,
                                  room_code_map,
                                  ..
                              }) = crate::parse_file(&path, &None, file_name, project, "") {
                        let versions_map = versions_map.clone();
                        let target_dbno = if field_no.0 == 0 { db_no } else { field_no };
                        versions_map.insert(target_dbno, version);

                        let mut con = Self::get_attr_conn(project.as_str(), false);
                        for (k, v) in all_attr_map {
                            // all_att_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                            con.set(&k,&v).unwrap();
                        }
                        // let table_name = format!("{project}:attrs");
                        // con.switch(&table_name).unwrap();
                        // con.set(&target_dbno,&ele_id_tree).unwrap();

                        // for (k, v) in refno_node_id_map {
                        //     // tree_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&k,&PdmsNodeId(v)).unwrap();
                        // }
                        // for (k, v) in type_ele_map {
                        //     // types_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&Integer(k),&v).unwrap();
                        // }
                        // let lookup = Arc::try_unwrap(string_lookup.lookup).unwrap();
                        // for (k, v) in lookup {
                        //     // string_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&Integer(k),&v);
                        // }
                        // for (k, v) in refno_info_map {
                        //     // info_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&k,&v);
                        // }
                        // for (k, v) in children_map {
                        //     // children_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&k,&v).unwrap();
                        // }
                        // for (k, v) in room_code_map {
                        //     // room_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        //     con.set(&k,&v).unwrap();
                        // }
                    }
                }
            }
        });
        let mut con = Connection::new("127.0.0.1", 2003)?;
        for kv in versions_map.as_ref() {
            // version_db.insert(&kv.key().0.to_be_bytes(), bincode::serialize(kv.value()).unwrap());
            con.set(kv.key(),kv.value());
        }
        Ok(())
    }

    pub fn inc_sync(&mut self, external_info_db: sled::Tree) -> anyhow::Result<()> {
        let project = &self.project;
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;
        let project_dir = data_dir.join(&project);

        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();
        let dir = PathBuf::from(target_dir);
        let mut children_files = fs::read_dir(dir)?.into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();


        // for path in children_files {
        //     let file_name = path.file_name().unwrap().to_str().unwrap();
        //     if !file_name.ends_with("com") && !file_name.ends_with("mis") {
        //         println!("path={:?}", &path);
        //         self.increment_parse(project, &path)?;
        //     }
        // };
        Ok(())
    }
    ///获得下一个Element
    #[inline]
    pub fn next(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    // pub fn increment_parse(&self, project: &str, path: &PathBuf) -> anyhow::Result<()> {
    //     let filename = SmolStr::new(path.file_name().unwrap().to_str().unwrap());
    //     let mut file = File::open(path)?;
    //     let mut buf: Vec<u8> = Vec::new();
    //     file.read_to_end(&mut buf)?;
    //     let input = &buf[..];
    //     let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    //
    //     let (db_type, file_version, mut dbno) = parse_file_basic_info(input);
    //     let db_no_str = dbno.to_string();
    //     let mut field_no = 0;
    //     if db_type.as_str() != "SYST" && !filename.contains(&db_no_str) {
    //         let _chars_len = db_no_str.len();
    //         let l = filename.len();
    //         // //dbg!(&filename);
    //         let end = filename.chars().position(|x| x == '_').unwrap_or(l);
    //         field_no = filename[project.len()..end].parse::<u32>().unwrap_or_default();
    //     }
    //     dbno = if field_no == 0 { dbno } else { field_no };
    //
    //     if let Some(dbno_version) = DbnoVersion::get(dbno, &self.version_db)? {
    //         let mut version = dbno_version.contents.version; // 从数据库中获取的 version
    //         if version != file_version && version < file_version { // 如果文件的版本和数据库中存储的版本对不上 就进行增量解析
    //             loop {
    //                 let mut v = vec![0u8, 0, 0, 3];
    //                 v.append(&mut version.to_be_bytes().to_vec());
    //
    //                 if let Some(pos) = rfind_iter(input, &v).next() {
    //                     version = parse_to_u32(&input[pos + 20..pos + 24]);
    //                     let b_version = parse_to_u32(&input[pos + 36..pos + 40]);
    //                     if b_version == version {
    //                         version -= 1;
    //                     }
    //                     let data_version = version - 4; // 参考号的版本是大版本-4 ,有遇到是-5的情况，若是-5则查不到对应的位置
    //                     if let Some(refno_pos) = rfind_iter(&input[..pos], &data_version.to_be_bytes()).next() { // 通过version找到他的参考号
    //                         let refno = &input[refno_pos - 8..refno_pos];
    //                         let mut iter = rfind_iter(&input[..refno_pos - 8], refno); // todo 调整为在某个范围内查询
    //                         while let Some(data_pos) = iter.next() { // 找到的参考号是文件里所有的
    //                             let attr_type = parse_to_i32(&input[data_pos + 8..data_pos + 12]);
    //                             if NOUN_TYPES_MAP.contains_key(&attr_type) {
    //                                 if let Some(state) = check_increase_operate(input, data_pos, refno) {
    //                                     match state {
    //                                         NewDataState::Modify => { modify_data_to_db(&input[data_pos - 4..data_pos - 4 + 0x800], &pdms_database_info, dbno as u64, &self)? }
    //                                         NewDataState::Increase => { increment_data_to_db(&input[data_pos - 4..data_pos - 4 + 0x800], &pdms_database_info, dbno as u64, &self)? }
    //                                         // NewDataState::Delete => { delete_data_to_db(&input[data_pos - 4..data_pos - 4 + 0x800],  &pdms_database_info, dbno as u64,&dbs)? }
    //                                         _ => {
    //                                             // //dbg!("todo delete");
    //                                             ()
    //                                         } // todo delete先不管，先把modify 和 increase跑通
    //                                     }
    //                                     // update_version_in_db(filename.clone(), version, &mut interface)?               ;
    //                                 }
    //                                 break;
    //                             }
    //                         }
    //                     }
    //                 } else {
    //                     break;
    //                 }
    //             }
    //         }
    //     }
    //     Ok(())
    // }
    //
}






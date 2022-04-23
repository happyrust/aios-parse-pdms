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
use id_tree::NodeId;
use itertools::Itertools;
use ncollide3d::world::CollisionWorld;
use nom::AsBytes;
use once_cell::sync::Lazy;
use smol_str::SmolStr;
use crate::{AttrMap, db1_dehash, EleNode, GeomsInfo, parse_pdms_dir, read_attr_info_config, sctn};
use crate::data_interface::PdmsDataInterface;
use crate::db_tool::db1_hash;
use crate::local_db::helper::combine_to_u64;
use crate::parse::{NOUN_TYPES_MAP, parse_file_basic_info, PdmsDbData};
use crate::pdms_types::{AiosStr, CachedMeshesMgr, DbnoVersion, EleGeoInstData, GeoData, PdmsMeshMgr, PdmsTree, RefI32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom, ShapeInstancesMgr, StringLookupTable};
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::extrusion::{CurveType, Extrusion};
use crate::shape::pdms_shape::{BrepShapeTrait, PdmsPrimShape, VerifiedShape};
use crate::prim_geo::revolution::Revolution;
use crate::local_db::consts::*;
use crate::local_db::refno_info_database::RefInoDatabase;
use crate::local_db::string_database::StringDatabase;
use crate::pdms_data::ScomInfo;
use crate::pdms_types::AttrVal::{StringHashType, StringType};
use crate::prim_geo::facet::{Contour, Facet, Polygon};
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
use crate::prim_geo::category::{CateBrepShape, convert_to_brep_shapes};
use std::panic::catch_unwind;
use std::time::Instant;
use bincode::deserialize;
use crate::prim_geo::sphere::Sphere;
use crate::prim_geo::tubing::PdmsTubing;
use bonsaidb::core::connection::StorageConnection;
use bonsaidb::core::connection::LowLevelConnection;
use futures::StreamExt;
use log::Level::Debug;
use sled::{Db, IVec};
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
#[derive(Debug, Clone)]
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

    fn get_tree(&self, project_name: &str, db_no: u32) -> Option<PdmsTree> {
        if let Some(db) = self.project_map.get(&AiosStr(project_name.into()).get_u32_hash()) {
            db.get_tree(db_no).ok()?
        }else{
            None
        }
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
    pub fn get_children_attrs(&self, refno: RefU64) -> anyhow::Result<Vec<AttrMap>>{
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
    //todo use anyhow
    ///返回geo data
    #[inline]
    pub fn get_design_geoms(&self, refno: RefU64, cached_mesh_mgr: &mut CachedMeshesMgr) -> anyhow::Result<HashMap<RefU64, Vec<CateBrepShape>>> {
        //todo，直接use type_refs里面的数据直接过滤出哪些有参考号，而不用一个个去找
        let mut result_map = HashMap::new();
        if let Some(desi_att) = self.get_attr(refno)? {
            let type_name = desi_att.get_type();
            let is_bran = type_name == "BRAN";
            if !is_bran {
                let geoms = resolve_desi_comp(refno, self).unwrap_or_default();
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
                // //dbg!(&current_tubing);
                let children = self.get_children(refno)?.unwrap_or_default();
                if children.len() == 0 {
                    return Ok(result_map);
                }
                //第一遍完成后，然后生成tubing
                let last_child = children.last().unwrap().clone();
                for child in children {
                    if child != RefU64::from_two_nums(16501, 1157) {
                        continue;
                    }
                    // dbg!(self.get_pretty_attr(child));
                    let world_trans = self.get_world_transform(child).unwrap_or_default();
                    // dbg!(&world_trans);
                    let mut result_shapes = vec![];
                    let geoms = crate::query_cata::resolve_desi_comp(child, self).unwrap_or_default();
                    // dbg!(&geoms);
                    let attr = self.get_attr(child)?.unwrap_or_default();
                    if let Some(arrive) = attr.get_i32("ARRI") {
                        //todo 加入获取arrive position 的方法
                        if geoms.axis_map.contains_key(&arrive) {
                            let p = &geoms.axis_map[&arrive].pt;
                            let a_pos = world_trans.transform_point3(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
                            //dbg!(&a_pos);
                            if !current_tubing.finished && a_pos.distance(current_tubing.start_pt) > f32::EPSILON {
                                current_tubing.end_pt = a_pos;
                                current_tubing.finished = true;
                                // result_shapes.push(current_tubing.convert_to_shape());
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
                        // //dbg!(leave);
                    }
                    //管件的生成
                    for geom in geoms.geometries {
                        if let Some(cate_shape) = convert_to_brep_shapes(&geom) {
                            result_shapes.push(cate_shape);
                        }
                    } // end geoms.geometries
                    if child == last_child {
                        if !current_tubing.finished && bran_ttube_pt.distance(current_tubing.start_pt) > f32::EPSILON {
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

    pub fn get_color_type_refno(&self, refno: RefU64) -> Option<(SmolStr, RefU64)> {
        let mut cur_refno = refno;
        while let Some(attr) = self.get_attr(cur_refno).ok()? {
            let noun_name = attr.get_type_cloned();
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
        if let Some(tree) = main_db.get_tree(db_code)? {
            let tree = tree.0;
            let root_node_id = tree.root_node_id().unwrap();
            let node_id = tree.root_node_id().unwrap();

            if let Ok(mut nodes) = tree.traverse_level_order_ids(node_id) {
                while let Some(mut cur_node_id) = nodes.next() {
                    let cur_node = tree.get(&cur_node_id).unwrap();
                    let d = cur_node.data();
                    let noun = d.noun;
                    let attr = self.get_attr(d.refno)?.ok_or(anyhow!("No attr map".to_string()))?;

                    // if d.owner != RefU64::from_two_nums(16501, 235)
                    if d.refno != RefU64::from_two_nums(16501, 1156)
                    /* && d.refno != RefU64::from_two_nums(8193, 46417)*/
                    // && d.refno != RefU64::from_two_nums(16501, 237)
                    {
                        continue;
                    }

                    let mut geo_hash = None;
                    let mut color_type = None;
                    let mut item_trans = glam::TransformSRT::IDENTITY;
                    let mut target_refno = d.refno;
                    let mut target_node_id = cur_node_id.clone();
                    if PRIM_HASH_NOUNS.contains(&noun) {
                        //获得类型和参考号
                        if let Some(e) = self.get_color_type_refno(d.refno) {
                            type_geom_refs_map.entry(e.1).or_insert(Vec::new()).push(d.refno);
                            color_type = Some(e.0.clone());
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
                                if height >= f32::EPSILON {
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
                                }
                            } //end of LOOP_NOUN
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
                        let ele_type = attr.get_type();
                        let owner = self.get_attr(attr.get_owner().unwrap())?;
                        let has_catref = attr.get_foreign_refno("CATR").is_some() || attr.get_foreign_refno("SPRE").is_some();
                        //todo fix these types
                        if ele_type == "PFIT" /*|| ele_type == "FITT"*/ {
                            continue;
                        }
                        //针对管道特殊处理
                        if ele_type == "BRAN" || (owner.is_some() && owner.unwrap().get_type() != "BRAN" && has_catref) {
                            let mut node_ids_map = HashMap::new();
                            for node_id in cur_node.children() {
                                let data = tree.get(node_id).unwrap().data();
                                node_ids_map.insert(data.refno, node_id.clone());
                            }
                            let brep_shapes = self.get_design_geoms(d.refno, &mut cached_mesh_mgr)?;
                            for (cur_refno, value_vec) in brep_shapes {
                                //记录对应的不同颜色类型
                                if let Some(e) = self.get_color_type_refno(d.refno) {
                                    type_geom_refs_map.entry(e.1).or_insert(Vec::new()).push(cur_refno);
                                    color_type = Some(e.0.clone());
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
                                for iter_val in value_vec {
                                    let CateBrepShape {
                                        brep_shape,
                                        mut transform,
                                        visible,
                                        is_tubing,
                                    } = iter_val;
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
                            visible: attr.is_visible_by_level(None).unwrap_or(false),
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

        //
        // let mut file = File::create(format!("type_geoms.json")).unwrap();
        // let serialized = serde_json::to_string(&type_geom_refs_map).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();

        // let mut file = File::create(format!("./AIOS_DBS/room_geoms.json")).unwrap();
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

    //todo 基于元件库的模型也要生成
    //todo 房间号的算法移植
    pub fn build_collision_world(&mut self, db_code: u32) -> anyhow::Result<()> {
        let mut world = GLOBAL_COLLISION_WORLD.lock().unwrap();
        // *world = CollisionWorld::<f32, (RefU64, RefU64)>::new(0.01f32);
        let query = GeometricQueryType::Proximity(0.0);
        let groups = CollisionGroups::new();

        let mut file = File::open(format!("./AIOS_DBS/type_geoms.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        let type_geom_refs_map: HashMap<RefU64, Vec<RefU64>> = serde_json::from_slice(&buf).unwrap();

        //暂时用json，方便调试
        let mut file = File::open(format!("../web-aios/{db_code}_geoms.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;
        let geo_map: HashMap<SmolStr, EleGeoInstData> = serde_json::from_slice(&buf).unwrap();

        // 暂时找到所有的设备，在这里进行遍历，获得包围盒信息
        let equip_hash = db1_hash("EQUI");
        let equip_key = combine_to_u64(equip_hash, db_code);
        let mut room_aabb_map = HashMap::new();
        //查询出所有的设备的几何体
        for (generic_ref, v) in type_geom_refs_map {
            for refno in &v {
                if let Some(geo_data) = geo_map.get(&refno.to_refno_str()) {
                    if geo_data.generic_type == "ROOM" {
                        room_aabb_map.insert(generic_ref, (v.clone(), geo_data.clone()));
                    } else {
                        let extents = geo_data.bbox.get_half_extents();
                        let center = geo_data.bbox.get_center();
                        let extents = na::Vector3::new(extents.x, extents.y, extents.z);
                        let shape = ShapeHandle::new(Cuboid::new(extents));
                        let (r, t, s) = geo_data.global_transform;
                        let t = /*t +*/ r * center;
                        let translation = na::Vector3::new(t.x, t.y, t.z);
                        let (axis, angle) = r.to_axis_angle();
                        let axisangle = na::Vector3::new(axis.x, axis.y, axis.z) * angle;
                        let iso = Isometry3::new(translation, axisangle);
                        world.add(iso, shape.clone(), groups, query, (generic_ref, *refno));
                    }
                }
            }
        }

        world.update();
        let mut aabb_contained = HashMap::new();
        let mut room_final_contained = HashMap::new();
        let mut room_geo_refs_map = HashMap::new();
        let mut cached_meshes = CachedMeshesMgr::deserialize_from_bin_file();
        for (k, (room_geo_refnos, room_geo)) in room_aabb_map {
            room_geo_refs_map.insert(k.to_refno_str(), room_geo_refnos.into_iter().map(|x| x.to_refno_str()).collect::<Vec<_>>());

            let e = room_geo.bbox.get_half_extents();
            let c = /*room_geo.global_transform.1 +*/ room_geo.bbox.get_center();
            let aabb = AABB::from_half_extents(na::Point3::new(c.x, c.y, c.z),
                                               na::Vector3::new(e.x, e.y, e.z));
            let mesh_indx = room_geo.geo_hash;
            let (r, t, s) = room_geo.global_transform;
            let translation = na::Vector3::new(t.x, t.y, t.z);
            let (axis, angle) = r.to_axis_angle();
            let axisangle = na::Vector3::new(axis.x, axis.y, axis.z) * angle;
            let room_iso = Isometry3::new(translation, axisangle);
            let room_tri_mesh = cached_meshes.meshes.get(&mesh_indx).unwrap().get_tri_mesh(TransformSRT {
                rotation: r,
                translation: t,
                scale: s,
            });
            let interferences = world.interferences_with_aabb(&aabb, &groups);
            for x in interferences {
                let generic_refno = x.1.data().0.clone();   //类型的参考号
                let geom_refno = x.1.data().1.clone();
                // //dbg!(geom_refno.to_refno_str());
                if let Some(geo_data) = geo_map.get(&geom_refno.to_refno_str()) {
                    let mesh_indx = &geo_data.geo_hash;
                    let pt = geo_data.global_transform.1;
                    let first_pt = ncollide3d::na::Point3::new(pt.x, pt.y, pt.z);
                    let ray_x = Ray::new(first_pt, ncollide3d::na::Vector3::z());
                    let ray_neg_x = Ray::new(first_pt, -ncollide3d::na::Vector3::z());

                    let ray_x = room_tri_mesh.toi_with_ray(&room_iso, &ray_x, std::f32::MAX, false);
                    let ray_neg_x = room_tri_mesh.toi_with_ray(&room_iso, &ray_neg_x, std::f32::MAX, false);
                    if ray_x.is_some() && ray_neg_x.is_some() {
                        aabb_contained.entry(k.to_refno_str()).or_insert(HashSet::new()).insert(generic_refno.to_refno_str());
                        room_final_contained.entry(k.to_refno_str()).or_insert(HashSet::new()).insert(geom_refno.to_refno_str());
                    } else {
                        println!("{} exclude from trimesh check", geom_refno.to_refno_str());
                    }
                }
            }
        }

        let mut file = File::create(format!("../web-aios/room_contained_equips.json")).unwrap();
        let serialized = serde_json::to_string(&aabb_contained).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        let mut file = File::create(format!("../web-aios/room_contained.json")).unwrap();
        let serialized = serde_json::to_string(&room_final_contained).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();


        let mut file = File::create(format!("../web-aios/room_geo_refs_map.json")).unwrap();
        let serialized = serde_json::to_string(&room_geo_refs_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        Ok(())
    }
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

    pub fn init(project: &str, dir: &str, info_db: Db) -> anyhow::Result<Self> {
        let cur_project = format!("./AIOS_DBS/{project}");

        let all_att_db = sled::open(format!("AIOS_DBS/{}/attr.sled", project)).expect("Create db file");
        let children_db = sled::open(format!("AIOS_DBS/{}/children.sled", project)).expect("Create db file");
        let types_db = sled::open(format!("AIOS_DBS/{}/type_eles.sled", project)).expect("Create db file");
        let string_db = sled::open(format!("AIOS_DBS/{}/names.sled", project)).expect("Create db file");
        let tree_db = sled::open(format!("AIOS_DBS/{}/tree.sled", project)).expect("Create db file");
        let version_db = sled::open(format!("AIOS_DBS/{}/version.sled", project)).expect("Create db file");

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

    //todo  infos 存储什么的问题，要不要存储dbno
    pub fn sync_total(&self, need_parsing_files: &Option<Vec<String>>) -> anyhow::Result<()> {
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();

        let mut children_files = fs::read_dir(target_dir)?.into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).collect::<Vec<PathBuf>>();
        let all_att_db = self.all_att_db.clone();
        let types_db = self.types_db.clone();
        let tree_db = self.tree_db.clone();
        let string_db = self.string_db.clone();
        let inofo_db = self.info_db.clone();
        let children_db = self.children_db.clone();
        let version_db = self.version_db.clone();
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
                                  string_lookup,
                                  refno_info_map,
                                  children_map,
                                  db_no,
                                  field_no,
                                  version,
                                  ..
                              }) = crate::parse_file(&path, &None, file_name, project, "") {

                        let versions_map = versions_map.clone();
                        let target_dbno = if field_no == 0 { db_no } else { field_no };
                        versions_map.insert(target_dbno, version);

                        for (k, v) in all_attr_map {
                            all_att_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        }
                        tree_db.insert(&target_dbno.to_be_bytes(), bincode::serialize(&ele_id_tree).unwrap());
                        for (k, v) in type_ele_map {
                            types_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        }
                        let lookup = Arc::try_unwrap(string_lookup.lookup).unwrap();
                        for (k, v) in lookup {
                            string_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        }
                        for (k, v) in refno_info_map {
                            inofo_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        }
                        for (k, v) in children_map {
                            children_db.insert(&k.to_be_bytes(), bincode::serialize(&v).unwrap());
                        }
                    }
                }
            }
        });

        for kv in versions_map.as_ref() {
            version_db.insert(&kv.key().to_be_bytes(), bincode::serialize(kv.value()).unwrap());
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






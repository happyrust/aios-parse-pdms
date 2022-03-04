use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::f32::EPSILON;
use std::fs::{self, File};
use std::io::{Read, Write};
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
use crate::{AttrMap, db1_dehash, GeomsInfo, parse_pdms_dir, pipes, sctn};
use crate::data_interface::PdmsDataInterface;
use crate::db_tool::db1_hash;
use crate::local_db::helper::combine_to_u64;
use crate::parse::{PdmsDbData};
use crate::pdms_types::{AiosStr, CachedMeshes, EleGeoData, GeoData, PdmsTree, RefI32Tuple, RefnoInfo, RefU64, RefU64Vec, ScaledGeom, StringLookupTable};
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::shape::pdms_shape::{BrepShape, PdmsPrimShape, VerifiedShape};
use crate::prim_geo::pyramid::LPyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::local_db::consts::*;
use crate::local_db::refno_info_database::RefInoDatabase;
use crate::local_db::string_database::StringDatabase;
use crate::pdms_data::ScomInfo;
use crate::pdms_types::AttrVal::{StringHashType, StringType};
use crate::prim_geo::facet::{Contour, Facet, Polygon};
use async_trait::async_trait;
use ncollide3d::bounding_volume::AABB;
use ncollide3d::na as na;
use ncollide3d::na::{Isometry3, Translation3, UnitQuaternion};
use ncollide3d::pipeline::{CollisionGroups, GeometricQueryType};
use ncollide3d::query::{Ray, RayCast};
use ncollide3d::shape::{Cuboid, ShapeHandle};
use truck_polymesh::stl::IntoSTLIterator;
use crate::parsed_data::CateProfileParam;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::query_cata::resolve_desi_comp;

pub const ATT_DB_NAME: &'static str = "attr";
pub const TYPES_DB_NAME: &'static str = "refs";
pub const CHILDREN_DB_NAME: &'static str = "children";
pub const TREE_DB_NAME: &'static str = "tree";
pub const INFO_DB_NAME: &'static str = "info";
pub const STR_DB_NAME: &'static str = "strs";
pub const GEOM_DB_NAME: &'static str = "geoms";

///collision world  存储所属元件名称和GeoId
static GLOBAL_COLLISION_WORLD: Lazy<Mutex<CollisionWorld<f32, (RefU64, RefU64)>>> = Lazy::new(|| {
    let mut world = CollisionWorld::<f32, (RefU64, RefU64)>::new(0.001f32);
    Mutex::new(world)
});

static PRIM_HASH_NOUNS: Lazy<Vec<u32>> = Lazy::new(|| {
    vec![BOX_NOUN, CYLI_NOUN, SPHE_NOUN, CONE_NOUN, CTOR_NOUN, DISH_NOUN,
         LOOP_NOUN, PYRA_NOUN, RTOR_NOUN, REVO_NOUN, POHE_NOUN]
});

static GENERIC_NOUN_NAMES: Lazy<Vec<SmolStr>> = Lazy::new(|| {
    vec!["EQUI".into(), "PIPE".into(), "STRU".into(), "ROOM".into()]
});


#[derive(Default, Debug)]
pub struct PdmsConfig {
    pub data_dir: String,
    //pdms的数据文件夹
    pub project_name: String,
    pub all_projects: Vec<String>,
    pub mdb_name: String,
}

#[derive(Debug, Default, Clone)]
pub struct DbOption {
    pub total_sync: bool,
    pub incr_sync: bool,
}

///MDB数据库管理
#[derive(Debug, Clone)]
pub struct AiosDBManager {
    pub project_map: HashMap<u32, AiosPdmsProject>,
    //project name hash -> Aios DB
    pub info_db: RefInoDatabase,
    //管理所有refno info的db
    pub storage: Storage,
}

#[async_trait]
impl PdmsDataInterface for AiosDBManager {
    #[inline]
    async fn get_ele_attr(&self, refno: &RefU64) -> Option<AttrMap> {
        self.get_attr(refno).await.unwrap()
    }

    #[inline]
    async fn get_ele_children_attrs(&self, refno: &RefU64) -> Vec<AttrMap> {
        self.get_children_attrs(refno).await.unwrap_or_default()
    }

    #[inline]
    async fn get_ele_children_refs(&self, refno: &RefU64) -> RefU64Vec {
        self.get_children(refno).await.unwrap().unwrap_or_default()
    }
}

impl AiosDBManager {
    ///初始化
    pub async fn init(dir: &str, projects: Vec<String>, mdb_name: &str, option: Option<DbOption>) -> Result<AiosDBManager, bonsaidb::core::Error> {
        let extra_storage_name = format!("./AIOS_DBS/AIOS_Extra");
        let storage = Storage::open(
            StorageConfiguration::new(extra_storage_name.as_str())
                .default_compression(Compression::Lz4)
                .with_schema::<AiosStr>()?
                .with_schema::<RefnoInfo>()?
        ).await?;

        storage.create_database::<RefnoInfo>(INFO_DB_NAME, true).await?;
        let mut info_db = RefInoDatabase {
            db: storage.database::<RefnoInfo>(INFO_DB_NAME).await?
        };

        let option = option.unwrap_or_default();
        let mut db_map = HashMap::default();
        for project in projects {
            let mut proj = AiosPdmsProject::init(project.as_str(), dir).await?;
            //增量保存数据
            if option.total_sync {
                proj.sync_total(&info_db).await?;
                info_db.merge(proj.get_info_database());
            }  //完全更新
            if option.incr_sync {}    //todo 增量更新
            let project_str: SmolStr = project.into();
            db_map.insert(AiosStr(project_str).get_u32_hash(), proj);
        }
        Ok(AiosDBManager {
            project_map: db_map,
            info_db,
            storage,
        })
    }

    ///获得refno的project 名称
    #[inline]
    pub async fn get_refno_info(&self, refno: &RefU64) -> Result<Option<RefnoInfo>, bonsaidb::core::Error> {
        self.info_db.get_refno_info(refno).await
    }

    /// 获得 children refno
    #[inline]
    pub async fn get_children(&self, refno: &RefU64) -> Result<Option<RefU64Vec>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_children(refno).await;
            }
        }
        Ok(Default::default())
    }

    ///获得refno的project 名称
    #[inline]
    pub async fn get_children_attrs(&self, refno: &RefU64) -> Result<Vec<AttrMap>, bonsaidb::core::Error> {
        let mut atts = vec![];
        let children = self.get_children(refno).await?.unwrap_or_default();
        for child in children {
            atts.push(self.get_dehashed_attr(&child).await?.unwrap_or_default());
        }
        Ok(atts)
    }

    ///获取attr 属性
    #[inline]
    pub async fn get_attr(&self, refno: &RefU64) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_attr(refno, ref_info.db_no).await;
            }
        }
        Ok(None)
    }

    ///string 被还原了的
    #[inline]
    pub async fn get_dehashed_attr(&self, refno: &RefU64) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                if let Some(mut attr) = db.get_attr(refno, ref_info.db_no).await? {
                    for (_, val) in attr.iter_mut() {
                        if let StringHashType(h) = val {
                            *val = StringType(db.get_string(*h).await?.unwrap_or_default().take());
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
    pub async fn get_pretty_attr(&self, refno: &RefU64) -> Result<HashMap<String, String>, bonsaidb::core::Error> {
        if let Some(attr) = self.get_dehashed_attr(refno).await? {
            return Ok(attr.to_string_hashmap());
        }
        Ok(Default::default())
    }

    ///获取世界坐标变换矩阵
    #[inline]
    pub async fn get_world_transform(&self, refno: &RefU64) -> Result<glam::TransformRT, bonsaidb::core::Error> {
        if let Some(ref_info) = self.get_refno_info(refno).await? {
            if let Some(db) = self.project_map.get(&ref_info.project_hash) {
                return db.get_world_transform(refno, ref_info.db_no).await;
            }
        }
        Ok(glam::TransformRT::IDENTITY)
    }

    #[inline]
    pub async fn get_cat_ref_in_desi(&self, refno: &RefU64) -> Option<RefU64> {
        if let Some(att) = self.get_attr(refno).await.unwrap() {
            if let Some(spre) = att.get_foreign_refno("SPRE") {
                if let Some(att) = self.get_attr(&spre).await.unwrap() {
                    if let Some(cat) = att.get_foreign_refno("CATR") {
                        return Some(cat);
                    }
                }
            }
        }
        None
    }

    #[inline]
    pub async fn get_cat_att_in_desi(&self, refno: &RefU64) -> Option<AttrMap> {
        if let Some(cat_ref) = self.get_cat_ref_in_desi(refno).await {
            if let Some(att) = self.get_attr(&cat_ref).await.unwrap() {
                return Some(att);
            }
        }
        None
    }

    ///返回geo data，
    #[inline]
    pub async fn get_design_geoms(&self, refno: &RefU64, cached_mesh_mgr: &mut CachedMeshes) -> Option<GeoData> {
        //todo，直接use type_refs里面的数据直接过滤出哪些有参考号，而不用一个个去找
        if let Some(desi_att) = self.get_attr(refno).await.unwrap() {
            //如果是SCTN，使用SCTN的方法创建GeoData
            if let Some(geoms) = crate::query_cata::resolve_desi_comp(&refno, self).await {
                dbg!(&geoms);
                let type_name = desi_att.get_type();
                if type_name == "SCTN" {
                    sctn::create_geo(&desi_att,&geoms);
                }
                //管件的生成
                //pipes::create_geo(&desi_att,&geoms);
            }
        }
        None
    }

    pub async fn get_generic_type_refno(&self, refno: &RefU64) -> Option<(SmolStr, RefU64)> {
        let mut cur_refno = *refno;
        while let Some(attr) = self.get_attr(&cur_refno).await.expect("Get attr failed") {
            if let Some(owner) = attr.get_owner() {
                let noun_name = attr.get_type_cloned();
                if GENERIC_NOUN_NAMES.contains(&noun_name) {
                    return Some((noun_name, cur_refno));
                }
                cur_refno = owner;
            } else {
                break;
            }
        }
        None
    }

    ///缓存所有几何体
    pub async fn cache_geos_data(&mut self, db_code: u32) -> Result<HashMap<SmolStr, EleGeoData>, bonsaidb::core::Error> {
        let project = AiosStr("Sample".into());
        let mut main_db = self.project_map.get_mut(&project.get_u32_hash()).expect("Not exist project");

        let mut cached_mesh_mgr = CachedMeshes::default();

        let mut geo_map = HashMap::new();
        let mut type_geom_refs_map = HashMap::new();
        if let Some(d) = PdmsTree::get(db_code as u64, main_db.get_tree_database()).await? {
            let tree = d.contents.0;
            let root_node_id = tree.root_node_id().unwrap();
            let node_id = tree.root_node_id().unwrap();
            if let Ok(mut nodes) = tree.traverse_level_order(node_id) {
                while let Some(mut cur_node) = nodes.next() {
                    let d = cur_node.data();
                    let noun = d.noun;
                    let attr = self.get_attr(&d.refno).await?.unwrap();
                    let mut geo = None;
                    let mut generic_type = None;
                    let mut scaled = Vec3::ONE;

                    let mut extra_rot = Quat::IDENTITY;
                    if PRIM_HASH_NOUNS.contains(&noun) {
                        //获得类型和参考号
                        if let Some(e) = self.get_generic_type_refno(&d.refno).await {
                            if e.0 == "ROOM" {
                                dbg!(&e);
                            }
                            type_geom_refs_map.entry(e.1).or_insert(Vec::new()).push(d.refno);
                            generic_type = Some(e.0.clone());
                        }
                        if noun == LOOP_NOUN {
                            let parent = attr.get_owner().unwrap();
                            let mut parent_att = self.get_attr(&parent).await?.unwrap();
                            let parent_noun = parent_att.get_type_cloned();
                            let mut loop_verts: Vec<Vec3> = vec![];
                            if let Some(children_refs) = self.get_children(&d.refno).await? {
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
                    } else {
                        //todo use known nouns to quick filter
                        if let Some(spre) = attr.get_foreign_refno("SPRE") {
                            geo = self.get_design_geoms(&d.refno, &mut cached_mesh_mgr).await;
                        }
                    }

                    //处理有几何体返回的情况，需要加入到几何列表里
                    if let Some(geo) = geo {
                        let GeoData::Primitive((hash, scaled)) = &geo;
                        let tr = self.get_world_transform(&d.refno).await?;
                        let mut bbox = cached_mesh_mgr.get_bbox(hash).unwrap();
                        bbox.scaled(scaled);
                        let geom_data = EleGeoData {
                            geo,
                            bbox,
                            global_transform: (tr.rotation, tr.translation),
                            visible: attr.is_visible(None),
                            generic_type: generic_type.unwrap_or_default(),
                        };
                        geo_map.insert(d.refno.to_refno_str(), geom_data);
                    } // end of insert geo_map
                }
            }
        }

        let mut file = File::create(format!("../web-aios/{db_code}_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&geo_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        let mut file = File::create(format!("./AIOS_DBS/type_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&type_geom_refs_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        // let mut file = File::create(format!("./AIOS_DBS/room_geoms.json")).unwrap();
        // let serialized = serde_json::to_string(&room_geom_refs_map).unwrap();
        // file.write_all(serialized.as_bytes()).unwrap();

        cached_mesh_mgr.serialize_to_json_file();
        cached_mesh_mgr.serialize_to_bin_file();
        Ok(geo_map)
    }

    //todo 基于元件库的模型也要生成
    //todo 房间号的算法移植

    pub async fn build_collision_world(&mut self, db_code: u32) -> Result<(), bonsaidb::core::Error> {
        let mut world = GLOBAL_COLLISION_WORLD.lock().unwrap();
        // *world = CollisionWorld::<f32, (RefU64, RefU64)>::new(0.01f32);
        let query = GeometricQueryType::Proximity(0.0);
        let groups = CollisionGroups::new();

        let mut file = File::open(format!("./AIOS_DBS/type_geoms.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        let type_geom_refs_map: HashMap<RefU64, Vec<RefU64>> = serde_json::from_slice(&buf).unwrap();

        //暂时用json，方便调试
        let mut file = File::open(format!("../web-aios/{db_code}_geoms.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        let geo_map: HashMap<SmolStr, EleGeoData> = serde_json::from_slice(&buf).unwrap();

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
                        let (r, t) = geo_data.global_transform;
                        let t = t + r * center;
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
        let mut cached_meshes = CachedMeshes::deserialize_from_bin_file();
        for (k, (room_geo_refnos, room_geo)) in room_aabb_map {
            room_geo_refs_map.insert(k.to_refno_str(), room_geo_refnos.into_iter().map(|x| x.to_refno_str()).collect::<Vec<_>>());

            let e = room_geo.bbox.get_half_extents();
            let c = room_geo.global_transform.1 + room_geo.bbox.get_center();
            let aabb = AABB::from_half_extents(na::Point3::new(c.x, c.y, c.z),
                                               na::Vector3::new(e.x, e.y, e.z));
            // println!("{} : {:?}, {:?}", k.to_refno_str(), &v, &c);
            let GeoData::Primitive((mesh_indx, scaled)) = room_geo.geo;
            let (r, t) = room_geo.global_transform;
            let translation = na::Vector3::new(t.x, t.y, t.z);
            let (axis, angle) = r.to_axis_angle();
            let axisangle = na::Vector3::new(axis.x, axis.y, axis.z) * angle;
            let room_iso = Isometry3::new(translation, axisangle);
            dbg!(&room_iso);

            let room_tri_mesh = cached_meshes.meshes.get(&mesh_indx).unwrap().get_tri_mesh(scaled);
            let interferences = world.interferences_with_aabb(&aabb, &groups);
            for x in interferences {
                let generic_refno = x.1.data().0.clone();   //类型的参考号
                let geom_refno = x.1.data().1.clone();
                dbg!(geom_refno.to_refno_str());
                if let Some(geo_data) = geo_map.get(&geom_refno.to_refno_str()) {
                    let GeoData::Primitive((mesh_indx, scaled)) = &geo_data.geo;
                    let tmp_mesh = cached_meshes.meshes.get(mesh_indx).unwrap();

                    // let pt: Vec3 = (*tmp_mesh.vertices.first().unwrap()).into();
                    // let pt = glam::TransformSRT{
                    //     rotation: geo_data.global_transform.0,
                    //     translation: geo_data.global_transform.1,
                    //     scale: *scaled,
                    // }.transform_vector3(pt);
                    let pt = geo_data.global_transform.1;
                    let first_pt = ncollide3d::na::Point3::new(pt.x, pt.y, pt.z);
                    let ray_x = Ray::new(first_pt, ncollide3d::na::Vector3::z());
                    let ray_neg_x = Ray::new(first_pt, -ncollide3d::na::Vector3::z());


                    let ray_x = room_tri_mesh.toi_with_ray(&room_iso, &ray_x, std::f32::MAX, false);
                    let ray_neg_x = room_tri_mesh.toi_with_ray(&room_iso, &ray_neg_x, std::f32::MAX, false);
                    if ray_x.is_some() && ray_neg_x.is_some() {
                        dbg!(geom_refno.to_refno_str());
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

        //room_geo_refs_map

        Ok(())
    }

    ///获得structure profile构件， 返回的是截面，这里生成拉伸Z方向的单元构件
    #[inline]
    pub async fn get_sprf_geom(&self, spre: &RefU64) -> Result<Option<GeoData>, bonsaidb::core::Error> {
        if let Some(spre_attr) = self.get_dehashed_attr(spre).await? {
            // dbg!(spre_attr.to_string_hashmap());
            if let Some(cat_ref) = spre_attr.get_foreign_refno("CATR") {
                if let Some(cat_attr) = self.get_dehashed_attr(&cat_ref).await? {
                    dbg!(cat_attr.to_string_hashmap());
                    if let Some(gms_ref) = cat_attr.get_foreign_refno("GSTR") {
                        if let Some(gms_attr) = self.get_dehashed_attr(&gms_ref).await? {
                            dbg!(gms_attr.to_string_hashmap());
                            let children = self.get_children(&gms_ref).await?.unwrap_or_default();
                            let mut loop_verts: Vec<Vec3> = vec![];
                            if children.len() > 0 {
                                let first_profile = children[0];
                                let children = self.get_children(&first_profile).await?.unwrap_or_default();
                                for x in children {
                                    let v = self.get_attr(&x).await?.unwrap().get_position();
                                    loop_verts.push(v);
                                }
                            }

                            dbg!(&loop_verts);
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

/// DB 单个数据库管理
#[derive(Debug, Clone, )]
pub struct AiosPdmsProject {
    pub project: String,
    pub dir: String,
    pub storage: Storage,
    //pdms data directory
    pub att_db_map: HashMap<u32, Database>,   //att map 需要做分库, db_number -> database

    pub types_db: Database,
    pub children_db: Database,
    pub tree_db: Database,
    pub info_db: RefInoDatabase,
    pub string_db: StringDatabase,
    pub mdb_name: Option<String>,
}


impl AiosPdmsProject {
    #[inline]
    pub fn get_attr_database(&self, db_no: u32) -> Option<Database> {
        if let Some(att_db) = self.att_db_map.get(&db_no) {
            return Some(att_db.clone());
        }
        None
    }

    ///获得children的数据库
    #[inline]
    pub fn get_children_database(&self) -> &Database {
        &self.children_db
    }


    ///获得tree的数据库
    #[inline]
    pub fn get_info_database(&self) -> &RefInoDatabase {
        &self.info_db
    }

    ///获得type refs的数据库
    #[inline]
    pub fn get_type_refs_database(&self) -> &Database {
        &self.types_db
    }

    ///获得tree的数据库
    #[inline]
    pub fn get_tree_database(&self) -> &Database {
        &self.tree_db
    }

    ///获得strings的数据库
    #[inline]
    pub fn get_string_database(&self) -> &StringDatabase {
        &self.string_db
    }

    pub async fn init(project: &str, dir: &str) -> Result<Self, bonsaidb::core::Error> {
        let cur_project = format!("./AIOS_DBS/{project}");
        let storage = Storage::open(
            StorageConfiguration::new(cur_project.as_str())
                .default_compression(Compression::Lz4)
                .with_schema::<AttrMap>()?
                .with_schema::<RefU64Vec>()?
                .with_schema::<PdmsTree>()?
                .with_schema::<AiosStr>()?
                .with_schema::<RefnoInfo>()?
        ).await?;

        //需要把所有的db number
        let mut att_db_map = HashMap::new();

        let dbs = storage.list_databases().await?;
        for x in dbs {
            if let Ok(num) = x.name.parse::<u32>() {
                let name = x.name.as_str();
                let db = storage.database::<AttrMap>(name).await?;
                att_db_map.insert(num, db);
            }
        }
        storage.create_database::<PdmsTree>(TREE_DB_NAME, true).await?;
        let tree_db = storage.database::<PdmsTree>(TREE_DB_NAME).await?;

        storage.create_database::<RefU64Vec>(CHILDREN_DB_NAME, true).await?;
        let children_db = storage.database::<RefU64Vec>(CHILDREN_DB_NAME).await?;

        storage.create_database::<RefU64Vec>(TYPES_DB_NAME, true).await?;
        let types_db = storage.database::<RefU64Vec>(TYPES_DB_NAME).await?;

        storage.create_database::<AiosStr>(STR_DB_NAME, true).await?;
        let string_db = storage.database::<AiosStr>(STR_DB_NAME).await?;

        storage.create_database::<RefnoInfo>(INFO_DB_NAME, true).await?;
        let info_db = storage.database::<RefnoInfo>(INFO_DB_NAME).await?;

        Ok(Self {
            project: project.to_string(),
            dir: dir.to_string(),
            storage,
            att_db_map,
            types_db,
            children_db,
            tree_db,
            info_db: RefInoDatabase { db: info_db },
            string_db: StringDatabase { db: string_db },
            mdb_name: None,
        })
    }

    //按需要打开database
    #[inline]
    pub async fn get_attr(&self, refno: &RefU64, db_no: u32) -> Result<Option<AttrMap>, bonsaidb::core::Error> {
        if let Some(att_db) = self.att_db_map.get(&db_no) {
            if let Ok(Some(mut d)) = AttrMap::get(refno.get_u32_hash(), att_db).await {
                return Ok(Some(d.contents));
            }
        }
        Ok(None)
    }

    #[inline]
    pub async fn get_string(&self, h: u32) -> Result<Option<AiosStr>, bonsaidb::core::Error> {
        self.get_string_database().get_string(h).await
    }

    #[inline]
    pub async fn get_children(&self, refno: &RefU64) -> Result<Option<RefU64Vec>, bonsaidb::core::Error> {
        if let Ok(Some(mut d)) = RefU64Vec::get(refno.0, self.get_children_database()).await {
            return Ok(Some(d.contents));
        }
        Ok(Default::default())
    }

    //包含自己
    pub async fn get_ancestors_attrs(&self, refno: &RefU64, db_no: u32) -> Result<Vec<AttrMap>, bonsaidb::core::Error> {
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

        //need check the type
        // let w_poss = parent_trans.transform_point3(poss);
        // dbg!(w_poss);
        // let w_pose = parent_trans.transform_point3(pose);
        // dbg!(w_pose);
        // let extru_dir: Vec3 = (w_pose - w_poss).normalize();
        // let bangle = desi_att.get_f32("BANG").unwrap_or_default();
        // dbg!(&bangle);
        // let quat = Quat::from_rotation_arc(Vec3::Z, extru_dir);
        for attr in ancestors {
            let t = if attr.get_type_cloned() == "SCTN" {
                let tr = TransformRT {
                    rotation,
                    translation,
                };
                let mut final_rot = Quat::IDENTITY;
                if let Some(poss) = attr.get_poss() {
                    if let Some(pose) = attr.get_pose() {
                        let w_poss = tr.transform_point3(poss);
                        dbg!(w_poss);
                        let w_pose = tr.transform_point3(pose);
                        dbg!(w_pose);
                        let extru_dir: Vec3 = (w_pose - w_poss).normalize();
                        let bangle = attr.get_f32("BANG").unwrap_or_default();
                        dbg!(&bangle);
                        //如果和Z轴平行，需要使用Y轴作为参考轴
                        // abs_diff_eq!(1.0, 1.0, epsilon = f32::EPSILON);
                        let d = extru_dir.dot(Vec3::Z).abs();
                        let mut ref_axis = if abs_diff_eq!(1.0, d, epsilon = f32::EPSILON) {
                            Vec3::Y
                        } else { Vec3::Z };
                        // dbg!(&ref_axis);
                        let p_axis = ref_axis.cross(extru_dir).normalize();
                        // dbg!(&p_axis);
                        let y_axis = extru_dir.cross(p_axis).normalize();
                        // dbg!(&y_axis);
                        final_rot = Quat::from_mat3(&glam::f32::Mat3::from_cols_array_2d(
                            &[p_axis.to_array(), y_axis.to_array(), extru_dir.to_array()]
                        )/*.transpose()*/) * Quat::from_rotation_z(bangle.to_radians());
                        let xyz = final_rot.to_euler(glam::EulerRot::XYZ);
                        // dbg!((xyz.0.to_degrees(), xyz.1.to_degrees(), xyz.2.to_degrees()));
                    }
                }
                final_rot
            } else {
                attr.get_rotation()
            };
            translation = translation + rotation * attr.get_position();
            rotation = rotation * t;
        }
        Ok(glam::TransformRT {
            rotation,
            translation,
        })
    }

    //todo  infos 存储什么的问题，要不要存储dbno
    pub async fn sync_total(&mut self, external_info_db: &Database) -> Result<(), bonsaidb::core::Error> {
        let mut data_dir = Path::new(&self.dir);
        let project = &self.project;
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();

        if let Ok(mut r) = parse_pdms_dir(target_dir.as_os_str().to_str().unwrap(), project.as_str(), None) {
            let mut total_lookup = StringLookupTable::default();
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
                let target_dbno = if field_no == 0 { db_no } else { field_no };
                total_lookup.merge(&string_lookup);

                let mut tx = Transaction::default();
                tx.push(transaction::Operation::overwrite_serialized::<PdmsTree>(
                    target_dbno as u64,
                    &PdmsTree(ele_id_tree),
                ).unwrap());
                self.get_tree_database().apply_transaction(tx).await?;

                // 属性全部插入
                dbg!(all_attr_map.len());
                let dbno_str = target_dbno.to_string();
                self.storage.create_database::<AttrMap>(dbno_str.as_str(), true).await?;
                let mut attr_db = self.storage.database::<AttrMap>(dbno_str.as_str()).await?;
                //todo use multi thread
                for chunk in &all_attr_map.iter().chunks(400000usize) {
                    let mut tx = Transaction::default();
                    for kv in chunk {
                        tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
                            kv.key().get_u32_hash(),
                            kv.value(),
                        ).unwrap());
                    }
                    attr_db.apply_transaction(tx).await?;
                }
                self.att_db_map.insert(target_dbno, attr_db);

                let mut tx = Transaction::default();
                for (type_noun, v) in type_ele_map {
                    // let k = combine_to_u64(type_noun, target_dbno as u32);
                    let k = type_noun as u64;
                    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
                        k,
                        &v,
                    ).unwrap());
                }
                self.get_type_refs_database().apply_transaction(tx).await?;


                let mut tx = Transaction::default();
                for (refno, v) in children_map {
                    tx.push(transaction::Operation::overwrite_serialized::<RefU64Vec>(
                        refno.0,
                        &v,
                    ).unwrap());
                }
                self.get_children_database().apply_transaction(tx).await?;


                dbg!(refno_info_map.len());
                for chunk in &refno_info_map.iter().chunks(400000usize) {
                    let mut tx = Transaction::default();
                    for (k, v) in chunk {
                        tx.push(transaction::Operation::overwrite_serialized::<RefnoInfo>(
                            *k,
                            v,
                        ).unwrap());
                    }
                    self.get_info_database().apply_transaction(tx.clone()).await?;
                    external_info_db.apply_transaction(tx).await?;
                }
            }

            for chunk in &total_lookup.lookup.iter().chunks(400000usize) {
                let mut tx = Transaction::default();
                for (k, v) in chunk {
                    tx.push(transaction::Operation::overwrite_serialized::<AiosStr>(
                        *k,
                        v,
                    ).unwrap());
                }
                self.get_string_database().apply_transaction(tx).await?;
            }
        }

        let dbs = self.storage.list_databases().await?;
        dbg!(&dbs.iter().map(|x| x.name.clone()).collect::<Vec<_>>());

        Ok(())
    }

    ///获得下一个Element
    #[inline]
    pub async fn next(&mut self) -> Result<(), bonsaidb::core::Error> {
        Ok(())
    }
}






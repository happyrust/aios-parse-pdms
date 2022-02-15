use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::ptr::eq;
use std::sync::Mutex;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::connection::{Connection, StorageConnection};
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::core::transaction;
use bonsaidb::core::transaction::Transaction;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::{Database, Storage};
use glam::{Mat4, Quat, TransformRT, Vec3};
use id_tree::NodeId;
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
use crate::prim_geo::extrude::Extrusion;
use crate::prim_geo::pdms_shape::{BrepShape, ScaledShape, VerifiedShape};
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;

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
    pub async fn init(dir: &str, projects: Vec<String>, sync: bool) -> Result<AiosDBManager, bonsaidb::core::Error> {
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
        glam::TransformRT{
            rotation,
            translation,
        }
        // world_mat
    }

    // pub async fn get_world_matrix(&self, refno: &RefU64) -> glam::f32::Affine3A {
    //     let mut world_mat = glam::f32::Affine3A::IDENTITY;
    //     let mut cur_refno = *refno;
    //     let mut mats = vec![];
    //     while let Some(attr) = self.get_attr(&cur_refno).await {
    //         if let Some(owner) = attr.get_owner() {
    //             let t = attr.get_matrix();
    //             // println!("{}: {:?}", cur_refno.to_refno_str(), &attr);
    //             // dbg!(t.rotation.to_euler(glam::EulerRot::XYZ));
    //             // world_mat = world_mat * t;
    //             mats.insert(0, t);
    //             // world_mat = attr.get_tansformRT() * world_mat;
    //             cur_refno = owner;
    //         } else {
    //             break;
    //         }
    //     }
    //     mats.iter().for_each(|&x| { world_mat = x * world_mat; });
    //     world_mat
    // }


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
                let mut tx = Transaction::default();
                let pdms_tree = PdmsTree(ele_id_tree);
                tx.push(transaction::Operation::overwrite_serialized::<PdmsTree>(
                    target_dbno,
                    &pdms_tree,
                ).unwrap());
                self.tree_db.apply_transaction(tx).await.unwrap();
                // let mut file = File::create(format!("./{project}/{target_dbno}.json")).unwrap();
                //
                // let serialized = serde_json::to_string(&pdms_tree).unwrap();
                // file.write_all(serialized.as_bytes()).unwrap();

                //属性全部插入
                let mut tx = Transaction::default();
                for (refno, v) in all_attr_map {
                    tx.push(transaction::Operation::overwrite_serialized::<AttrMap>(
                        refno.0,
                        &v,
                    ).unwrap());
                }
                self.att_db.apply_transaction(tx).await.unwrap();

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

                let mut tx = Transaction::default();
                for (refno, v) in refno_info_map {
                    tx.push(transaction::Operation::overwrite_serialized::<RefnoInfo>(
                        refno.0,
                        &v,
                    ).unwrap());
                }
                self.info_db.apply_transaction(tx).await.unwrap();
            }
        }
        Ok(())
    }

    //todo 基于元件库的模型也要生成
    //todo 房间号的算法移植

    pub async fn build_collison_world(&mut self) -> Result<(), bonsaidb::core::Error> {
        let mut world = GLOBAL_COLLISION_WORLD.lock().unwrap();
        // *world = CollisionWorld::<f64, (String, RefU64)>::new(0.001f64);
        Ok(())
    }


    /// 获得children
    #[inline]
    pub async fn get_children(&mut self, refno: &RefU64) -> Result<RefU64Vec, bonsaidb::core::Error>{
        if let Some(d) = RefnoInfo::get(refno.0, &self.info_db).await? {
            return Ok(d.contents.children);
        }
        Ok(RefU64Vec::default())
    }

    ///获得下一个Element
    #[inline]
    pub async fn next(&mut self) -> Result<(), bonsaidb::core::Error>{
        Ok(())
    }


    //缓存设备得几何体
    pub async fn cache_prim_geos_data(&mut self) -> Result<HashMap<SmolStr, EleGeoData>, bonsaidb::core::Error> {
        let db_code = 7200;
        let equip_hash = db1_hash("EQUI");
        dbg!(db1_dehash(equip_hash));
        let equip_key = combine_to_u64(equip_hash, db_code);
        let project = "Sample";

        let mut cached_mesh_mgr = CachedMeshes::default();

        let mut geo_map = HashMap::new();
        //根据不同种类的几何体，做一下单独的聚集，这样再获取的时候能直接获取需要的几何形状
        if let Some(d) = RefU64Vec::get(equip_key, &self.refs_db).await? {
            // if let Some(d) = AttrMap::get(test_refno.0, &attr_db).await? {
            let refnos = d.contents;
            // dbg!(refnos.0.iter().map(|x| Refi32Tuple::from(x)).collect::<Vec<_>>());
            dbg!(refnos.len());
            let box_hash = db1_hash("BOX");
            let cylinder_hash = db1_hash("CYLI");
            let sphere_hash = db1_hash("SPHE");
            let cone_hash = db1_hash("CONE");
            let dish_hash = db1_hash("DISH");
            let ctorus_hash = db1_hash("CTOR");
            let loop_hash = db1_hash("LOOP");

            if let Some(d) = PdmsTree::get(db_code as u64, &self.tree_db).await? {
                let tree = d.contents.0;
                let root_node_id = tree.root_node_id().unwrap();
                let mut cubes_tx = Transaction::default();
                // for refno in refnos.0 {
                // let matrix = self.get_world_matrix(&refno).await;
                // dbg!(refno.to_refno_str());
                // if let Some(node_id) = self.get_node_id(&refno).await{
                //     dbg!(&node_id);
                //保存一个数据库，
                let node_id = tree.root_node_id().unwrap();
                if let Ok(mut nodes) = tree.traverse_level_order(node_id) {
                    while let Some(mut cur_node) = nodes.next() {
                        // dbg!(&cur_node.data().name);
                        let d = cur_node.data();
                        //改成check是否是基本体，然后再继续
                        if d.noun == box_hash {
                            let attr = self.get_attr(&d.refno).await.unwrap();
                            let sbox: SBox = (&attr).into();
                            let tr = self.get_world_transform(&d.refno).await;
                            let geom_data = EleGeoData {
                                geo: GeoData::Scaled(ScaledGeom::Box(sbox.get_scale_vec3())),
                                global_transform: (tr.rotation, tr.translation),           //todo 优化global matrix的计算，是否需要统一来一次计算
                                visible: attr.is_visible(None)
                            };
                            geo_map.insert(d.refno.to_refno_str(), geom_data);
                            // tx.push(transaction::Operation::insert_serialized::<EleGeoData>(
                            //     Some(d.refno.0),
                            //     &geom_data,
                            // ).unwrap());
                        }
                        if d.noun == cylinder_hash {
                            let attr = self.get_attr(&d.refno).await.unwrap();
                            let cyli: SCylinder = (&attr).into();
                            let tr = self.get_world_transform(&d.refno).await;
                            let geom_data = EleGeoData {
                                geo: GeoData::Scaled(ScaledGeom::Cylinder(cyli.get_scale_vec3())),
                                global_transform: (tr.rotation, tr.translation),            //todo 优化global matrix的计算，是否需要统一来一次计算
                                visible: attr.is_visible(None)
                            };
                            geo_map.insert(d.refno.to_refno_str(), geom_data);
                        } else if d.noun == cone_hash {
                            let attr = self.get_attr(&d.refno).await.unwrap();
                            let snout: LSnout = (&attr).into();
                            if snout.check_valid() {
                                dbg!(d.refno.to_refno_str());
                                let tr = self.get_world_transform(&d.refno).await;
                                let result = cached_mesh_mgr.get_pdms_mesh_hash_key(&snout);
                                let geom_data = EleGeoData {
                                    geo: GeoData::Primitive(result),                            //todo use bin-code to transfer data
                                    global_transform: (tr.rotation, tr.translation),            //todo 优化global matrix的计算，是否需要统一来一次计算
                                    visible: attr.is_visible(None)
                                };
                                geo_map.insert(d.refno.to_refno_str(), geom_data);
                            }
                        } else if d.noun == dish_hash {
                            let attr = self.get_attr(&d.refno).await.unwrap();
                            let dish: Dish = (&attr).into();        //make this to dyn trait
                            if dish.check_valid() {
                                let tr = self.get_world_transform(&d.refno).await;
                                let result = cached_mesh_mgr.get_pdms_mesh_hash_key(&dish);
                                let geom_data = EleGeoData {
                                    geo: GeoData::Primitive(result),                            //todo use bin-code to transfer data
                                    global_transform: (tr.rotation, tr.translation),            //todo 优化global matrix的计算，是否需要统一来一次计算
                                    visible: attr.is_visible(None)
                                };
                                geo_map.insert(d.refno.to_refno_str(), geom_data);
                            }
                        }else if d.noun == ctorus_hash {
                            let attr = self.get_attr(&d.refno).await.unwrap();
                            let ctorus: CTorus = (&attr).into();        //make this to dyn trait
                            if ctorus.check_valid() {
                                let tr = self.get_world_transform(&d.refno).await;
                                let result = cached_mesh_mgr.get_pdms_mesh_hash_key(&ctorus);
                                let geom_data = EleGeoData {
                                    geo: GeoData::Primitive(result),                    //todo use bin-code to transfer data
                                    global_transform: (tr.rotation, tr.translation),    //todo 优化global matrix的计算，是否需要统一来一次计算
                                    visible: attr.is_visible(None)
                                };
                                geo_map.insert(d.refno.to_refno_str(), geom_data);
                            }
                        }else if d.noun == loop_hash {

                            let parent_node = tree.get(cur_node.parent().unwrap()).unwrap();
                            let parent_refno = parent_node.data().refno.to_refno_str();
                            // dbg!(&parent_refno);
                            let type_string = db1_dehash(parent_node.data().noun);
                            // dbg!(&type_string);
                            let mut loop_verts: Vec<Vec3> = vec![];
                            let children_refs = self.get_children(&d.refno).await?;
                            for x in children_refs.0 {
                                let v = self.get_attr(&x).await.unwrap().get_position();
                                loop_verts.push(v);
                            }
                            // dbg!(&loop_verts);
                            //todo 旋转类型另外处理
                            if type_string.as_str() != "REVO" && type_string.as_str() != "NREV" {
                                let mut att_map = self.get_attr(&parent_node.data().refno).await.unwrap();
                                if let Some(v) = att_map.get_val("HEIG") {
                                    let height = v.f32_value().unwrap_or_default();
                                    if height >= f32::EPSILON {
                                        let extrusion = Extrusion {
                                                                   loop_verts,
                                                                   height,
                                                                   ..Default::default()
                                                               };
                                        let tr = self.get_world_transform(&d.refno).await;
                                        // dbg!(&tr);
                                        let pdms_mesh = extrusion.gen_mesh(None);
                                        // let geom_data = EleGeoData {
                                        //     geo: GeoData::Primitive(pdms_mesh),   //todo use bin-code to transfer data
                                        //     global_transform: (tr.rotation, tr.translation),            //todo 优化global matrix的计算，是否需要统一来一次计算
                                        // };
                                        // geo_map.insert(d.refno.to_refno_str(), geom_data);
                                    }
                                } else { /*dbg!(&refno);*/ }
                                //判断是否为软碰撞
                                // if let Some(obstruction) = att_map.get_u32("OBST") {
                                //     is_obstruction = obstruction == 1;
                                // }
                            }

                            // let ctor:  = self.get_attr(&d.refno).await.unwrap().into();
                            // let tr = self.get_world_transform(&d.refno).await;
                            // let pdms_mesh = ctor.gen_mesh(None);
                            // let geom_data = EleGeoData {
                            //     geo: GeoData::Primitive(pdms_mesh),   //todo use bin-code to transfer data
                            //     global_transform: (tr.rotation, tr.translation),            //todo 优化global matrix的计算，是否需要统一来一次计算
                            // };
                            // geo_map.insert(d.refno.to_refno_str(), geom_data);
                        }



                    }
                    // }
                }
                // dbg!(dish_refnos);
                // } //end for -equip
                // cubes_db.apply_transaction(tx).await.unwrap();
            }
        }

        ///{project}
        let mut file = File::create(format!("./{db_code}_geoms.json")).unwrap();
        let serialized = serde_json::to_string(&geo_map).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();


        dbg!(cached_mesh_mgr.meshes.len());
        cached_mesh_mgr.serialize_to_json_file();

        Ok(geo_map)
    }
}






use crate::consts::UNSET_STR;
use crate::helper::get_attr_value_f64_vec;
use crate::pdms_types::AttrVal::{
    BoolArrayType, BoolType, DoubleArrayType, DoubleType, ElementType, IntArrayType, IntegerType,
    RefU64Type, StringArrayType, StringHashType, StringType, Vec3Type, WordType,
};
use bevy::prelude::*;
use bevy_inspector_egui::{widgets::InspectableButton, Inspectable, InspectorPlugin};
// use bonsaidb::core::schema::{
//     Collection, CollectionName, DefaultSerialization, Schematic, SerializedCollection,
// };
// use bonsaidb::core::Error;
use dashmap::DashMap;
use glam::TransformSRT;
use hash32::Hasher;
use highway::{HighwayHash, HighwayHasher, Key};
use id_tree::{NodeId, Tree, TreeBuilder};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::result::Iter;
use std::vec::IntoIter;
use anyhow::anyhow;
use bevy::render::primitives::Aabb;
use bevy_egui::egui;
use egui::Key::O;

pub const LEVEL_VISBLE: u32 = 6;

///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct RefI32Tuple(pub (i32, i32));

impl Into<SmolStr> for RefI32Tuple {
    fn into(self) -> SmolStr {
        SmolStr::from(format!("{}/{}", self.get_0(), self.get_1()))
    }
}

impl Into<String> for RefI32Tuple {
    fn into(self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
    }
}

impl From<&[u8]> for RefI32Tuple {
    fn from(input: &[u8]) -> Self {
        Self::new(
            i32::from_be_bytes(input[0..4].try_into().unwrap()),
            i32::from_be_bytes(input[4..8].try_into().unwrap()),
        )
    }
}

impl From<&str> for RefI32Tuple {
    fn from(s: &str) -> Self {
        let x: Vec<i32> = s
            .split('/')
            .map(|x| x.parse::<i32>().unwrap_or_default())
            .collect();
        Self::new(x[0], x[1])
    }
}

impl From<&RefU64> for RefI32Tuple {
    fn from(n: &RefU64) -> Self {
        let n = n.0.to_be_bytes();
        Self((
            i32::from_be_bytes(n[..4].try_into().unwrap()),
            i32::from_be_bytes(n[4..].try_into().unwrap()),
        ))
    }
}

impl RefI32Tuple {
    #[inline]
    pub fn new(ref_0: i32, ref_1: i32) -> Self {
        Self { 0: (ref_0, ref_1) }
    }

    #[inline]
    pub fn get_0(&self) -> i32 {
        self.0 .0
    }

    #[inline]
    pub fn get_1(&self) -> i32 {
        self.0 .1
    }
}

//把Refno当作u64
#[derive(Hash, Serialize, Deserialize, Clone, Copy, Default, Component, Eq, PartialEq, Hash32)]
pub struct RefU64(pub u64);

impl Inspectable for RefU64 {
    type Attributes = (u32, u32);

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        options: Self::Attributes,
        context: &mut bevy_inspector_egui::Context,
    ) -> bool {
        true
    }
}

impl Deref for RefU64 {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for RefU64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_refno_str().as_str())
    }
}

impl From<&RefI32Tuple> for RefU64 {
    fn from(n: &RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<RefI32Tuple> for RefU64 {
    fn from(n: RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<&[u8]> for RefU64 {
    fn from(input: &[u8]) -> Self {
        Self(u64::from_be_bytes(input[0..8].try_into().unwrap()))
    }
}

impl RefU64 {
    #[inline]
    pub fn get_0(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[0..4].try_into().unwrap())
    }

    #[inline]
    pub fn get_1(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[4..8].try_into().unwrap())
    }

    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hash, Hasher};
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish()
    }

    #[inline]
    pub fn to_refno_str(&self) -> SmolStr {
        let refno: RefI32Tuple = self.into();
        refno.into()
    }

    #[inline]
    pub fn from_two_nums(i: u32, j: u32) -> Self {
        let bytes: Vec<u8> = [i.to_be_bytes(), j.to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct RefU64Vec(pub Vec<RefU64>);

impl Deref for RefU64Vec {
    type Target = Vec<RefU64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RefU64Vec {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for RefU64Vec {
    type Item = RefU64;
    type IntoIter = IntoIter<RefU64>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

//存储children，也可以这么去存储
// impl Collection for RefU64Vec {
//     type PrimaryKey = u64;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "refnos")
//     }
//
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
//
// impl SerializedCollection for RefU64Vec {
//     type Format = transmog_bincode::Bincode;
//     type Contents = Self;
//
//     fn format() -> Self::Format {
//         // The bincode options can be set on this type
//         transmog_bincode::Bincode::default()
//     }
// }

impl RefU64Vec {
    #[inline]
    pub fn push(&mut self, v: RefU64) {
        self.0.push(v);
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug, Default, Component, Eq, Hash, PartialEq)]
#[derive(
Serialize,
Deserialize,
Clone,
Debug,
Default,
Component,
Reflect,
Inspectable,
Eq,
Hash,
PartialEq,
Ord,
PartialOrd,
)]
#[reflect(Component)]
pub struct NounHash(pub u32);

impl Deref for NounHash {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&SmolStr> for NounHash {
    fn from(s: &SmolStr) -> Self {
        Self(db1_hash(s.as_str()))
    }
}

impl From<SmolStr> for NounHash {
    fn from(s: SmolStr) -> Self {
        Self(db1_hash(s.as_str()))
    }
}

impl From<u32> for NounHash {
    fn from(n: u32) -> Self {
        Self(n)
    }
}

impl From<&str> for NounHash {
    fn from(s: &str) -> Self {
        Self(db1_hash(s))
    }
}

///PDMS的属性数据Map
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct AttrMap {
    pub map: bevy_utils::HashMap<NounHash, AttrVal>,
}

impl Inspectable for AttrMap {
    type Attributes = ();

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        options: Self::Attributes,
        context: &mut bevy_inspector_egui::Context,
    ) -> bool {
        let mut changed = false;
        ui.vertical_centered(|ui| {
            egui::Grid::new(context.id()).show(ui, |ui| {
                let sort_keys = self.map.keys().cloned().sorted_by_key(|x| db1_dehash(x.0));
                //need sort
                for sort_key in sort_keys {
                    ui.label(db1_dehash(sort_key.0));
                    let v = self.map.get_mut(&sort_key).unwrap();
                    ui.vertical(|ui| {
                        changed |= v.ui(ui, Default::default(), context);
                    });
                    ui.end_row();
                }
            });
        });
        changed
    }
}

impl Deref for AttrMap {
    type Target = bevy_utils::HashMap<NounHash, AttrVal>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for AttrMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl AttrMap {
    #[inline]
    pub fn insert(&mut self, k: NounHash, v: AttrVal) {
        self.map.insert(k, v);
    }

    #[inline]
    pub fn insert_by_att_name(&mut self, k: &str, v: AttrVal) {
        self.map.insert(k.into(), v);
    }

    #[inline]
    pub fn contains_attr_name(&self, name: &str) -> bool {
        self.map.contains_key(&name.into())
    }

    #[inline]
    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&(hash.into()))
    }

    // pub fn dehash_value(&self, m: &StringLookupTable) -> AttrMap{
    //     let mut attr = self.clone();
    //     for (k, v) in &self.map {
    //         if let StringHashType(h) = v{
    //             attr.insert(k.clone(), StringType(m.get_string(*h).unwrap()));
    //         }
    //     }
    //     attr
    // }

    pub fn to_string_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (k, v) in &self.map {
            map.insert(db1_dehash(k.0), format!("{:?}", v));
        }
        map
    }

    #[inline]
    pub fn get_name_hash(&self) -> AiosStrHash {
        if let Some(StringHashType(name_hash)) = self.get_val("NAME") {
            *name_hash
        } else {
            0
        }
    }

    //获取spref
    #[inline]
    pub fn get_foreign_refno(&self, key: &str) -> Option<RefU64> {
        if let Some(RefU64Type(d)) = self.map.get(&key.into()) {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_refno_as_string(&self) -> SmolStr {
        self.get_as_string("REFNO").unwrap_or(UNSET_STR.into())
    }

    pub fn get_obstruction(&self) -> Option<u32> {
        self.get_u32("OBST")
    }

    pub fn get_level(&self) -> Option<[u32; 2]> {
        if let Some(v) = self.get_i32_vec("LEVE") {
            if v.len() >= 2 {
                return Some([v[0] as u32, v[1] as u32]);
            }
        }
        None
    }

    ///判断构件是否可见
    pub fn is_visible(&self, level: Option<u32>) -> bool {
        let l = level.unwrap_or(LEVEL_VISBLE);
        if let Some(level) = self.get_level() {
            return level[1] >= l;
        }
        true
    }

    #[inline]
    pub fn get_refno(&self) -> Option<RefU64> {
        if let Some(RefU64Type(d)) = self.map.get(&"REFNO".into()) {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner(&self) -> Option<RefU64> {
        if let Some(RefU64Type(d)) = self.map.get(&"OWNER".into()) {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> SmolStr {
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_type(&self) -> &str {
        self.get_string("TYPE").unwrap().as_str()
    }

    #[inline]
    pub fn get_type_cloned(&self) -> SmolStr {
        self.get_string("TYPE").unwrap().clone()
    }

    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        if let Some(v) = self.map.get(&key.into()) {
            match v {
                IntegerType(d) => {
                    return Some(*d as u32);
                }
                _ => {}
            }
        }
        None
    }

    #[inline]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        if let Some(v) = self.map.get(&key.into()) {
            match v {
                IntegerType(d) => {
                    return Some(*d as i32);
                }
                _ => {}
            }
        }
        None
    }

    #[inline]
    pub fn get_string(&self, key: &str) -> Option<&SmolStr> {
        if let Some(v) = self.map.get(&key.into()) {
            match v {
                StringType(s) | WordType(s) | ElementType(s) => {
                    return Some(s);
                }
                _ => {}
            }
        }
        None
    }

    #[inline]
    pub fn get_as_string(&self, key: &str) -> Option<SmolStr> {
        if let Some(v) = self.map.get(&key.into()) {
            let s = match v {
                StringType(s) | WordType(s) | ElementType(s) => s.clone(),
                IntegerType(d) => d.to_string().into(),
                DoubleType(d) => d.to_string().into(),
                BoolType(d) => d.to_string().into(),
                DoubleArrayType(d) => d
                    .iter()
                    .map(|i| format!(" {}", i))
                    .collect::<String>()
                    .into(),
                StringArrayType(d) => d
                    .iter()
                    .map(|i| format!(" {}", i))
                    .collect::<String>()
                    .into(),
                IntArrayType(d) => d
                    .iter()
                    .map(|i| format!(" {}", i))
                    .collect::<String>()
                    .into(),
                BoolArrayType(d) => d
                    .iter()
                    .map(|i| format!(" {}", i))
                    .collect::<String>()
                    .into(),
                Vec3Type(d) => d
                    .iter()
                    .map(|i| format!(" {}", i))
                    .collect::<String>()
                    .into(),

                RefU64Type(d) => RefI32Tuple::from(d).into(),
                StringHashType(d) => format!("{d}").into(),

                _ => UNSET_STR.into(),
            };
            return Some(s);
        }
        None
    }

    #[inline]
    pub fn get_as_vec_string(&self, key: &str) -> Vec<SmolStr> {
        if let Some(v) = self.map.get(&key.into()) {
            return match v {
                StringArrayType(d) => d.clone(),
                _ => {
                    vec![]
                }
            };
        }
        vec![]
    }

    #[inline]
    pub fn get_as_vec_refnos(&self, key: &str) -> Vec<SmolStr> {
        if let Some(v) = self.map.get(&key.into()) {
            return match v {
                IntArrayType(d) => d
                    .chunks_exact(2)
                    .map(|x| format!("{}/{}", x[0], x[1]).into())
                    .collect(),
                _ => {
                    vec![]
                }
            };
        }
        vec![]
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> bool {
        if let Some(v) = self.map.get(&key.into()) {
            match v {
                BoolType(b) => *b,
                _ => false,
            }
        } else {
            false
        }
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&AttrVal> {
        if let Some(v) = self.map.get(&key.into()) {
            Some(v)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        if let Some(v) = self.map.get(&key.into()) {
            v.double_value()
        } else {
            None
        }
    }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        if let Some(v) = self.map.get(&key.into()) {
            v.double_value().map(|x| x as f32)
        } else {
            None
        }
    }

    #[inline]
    pub fn get_position(&self) -> Vec3 {
        if let Some(pos) = get_attr_value_f64_vec(self, "POS") {
            return glam::f32::Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        } else {
            //如果没有POS，就以POSS来尝试
            if let Some(poss) = self.get_poss() {
                return poss;
            }
        }
        Vec3::ZERO
    }

    #[inline]
    pub fn get_posse_dist(&self) -> f32 {
        self.get_pose()
            .unwrap_or_default()
            .distance(self.get_poss().unwrap_or_default())
    }

    #[inline]
    pub fn get_poss(&self) -> Option<Vec3> {
        if let Some(pos) = get_attr_value_f64_vec(self, "POSS") {
            return Some(glam::f32::Vec3::new(
                pos[0] as f32,
                pos[1] as f32,
                pos[2] as f32,
            ));
        }
        None
    }

    #[inline]
    pub fn get_pose(&self) -> Option<Vec3> {
        if let Some(pos) = get_attr_value_f64_vec(self, "POSE") {
            return Some(glam::f32::Vec3::new(
                pos[0] as f32,
                pos[1] as f32,
                pos[2] as f32,
            ));
        }
        None
    }

    #[inline]
    pub fn get_rotation(&self) -> Quat {
        if let Some(ang) = get_attr_value_f64_vec(self, "ORI") {
            // return Quat::from_euler(EulerRot::XYZ, ang[0].to_radians() as f32, ang[1].to_radians() as f32, ang[2].to_radians() as f32);
            let mat = (glam::f32::Mat3::from_rotation_z(ang[2].to_radians() as f32)
                * glam::f32::Mat3::from_rotation_y(ang[1].to_radians() as f32)
                * glam::f32::Mat3::from_rotation_x(ang[0].to_radians() as f32));
            return Quat::from_mat3(&mat);
        }
        Quat::IDENTITY
    }

    pub fn get_matrix(&self) -> glam::f32::Affine3A {
        let mut affine = glam::f32::Affine3A::IDENTITY;
        if let Some(pos) = get_attr_value_f64_vec(self, "POS") {
            affine.translation = glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        }
        if let Some(ang) = get_attr_value_f64_vec(self, "ORI") {
            affine.matrix3 = (glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32)
                * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)
                * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32));
        }
        affine
    }

    #[inline]
    pub fn get_mat4(&self) -> glam::f32::Mat4 {
        glam::f32::Mat4::from(self.get_matrix())
    }

    pub fn get_f64_vec(&self, key: &str) -> Option<Vec<f64>> {
        if let Some(val) = self.map.get(&key.into()) {
            match val {
                AttrVal::DoubleArrayType(data) => {
                    return Some(data.clone());
                }
                AttrVal::Vec3Type(data) => {
                    return Some(data.to_vec());
                }
                _ => {}
            }
        }
        None
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let Some(AttrVal::Vec3Type(d)) = self.map.get(&key.into()) {
            return Some(Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32));
        }
        None
    }

    pub fn get_i32_vec(&self, att: &str) -> Option<Vec<i32>> {
        if let Some(val) = self.map.get(&att.into()) {
            match val {
                AttrVal::IntArrayType(data) => {
                    return Some(data.clone());
                }
                _ => {}
            }
        }
        None
    }

    ///使用spref + params 混合成的meshid
    pub fn cal_des_mesh_id(&self) -> u64 {
        let key = Key([1, 2, 3, 4]);
        let mut hasher64 = HighwayHasher::new(key);
        if let Some(spref) = self.get_as_string("SPRE") {
            hasher64.append(spref.as_ref());
        }
        if let Some(para) = self.get_f64_vec("PARA") {
            let output: Vec<u8> = para.iter().flat_map(|val| val.to_be_bytes()).collect();
            hasher64.append(&output);
        }
        if let Some(d) = self.get_as_string("RADI") {
            hasher64.append(d.as_ref());
        }
        if let Some(d) = self.get_as_string("HEIG") {
            hasher64.append(d.as_ref());
        }
        if let Some(d) = self.get_as_string("ANGL") {
            hasher64.append(d.as_ref());
        }
        let id = hasher64.finalize64();
        id
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type_cloned();
        return match type_noun.as_str() {
            "BOX" => Some(Box::new(SBox::from(self))),
            "CYLI" => Some(Box::new(SCylinder::from(self))),
            // "SPHE" => Some(Box::new(Sphere::from(self))),
            "CONE" => Some(Box::new(LSnout::from(self))),
            "DISH" => Some(Box::new(Dish::from(self))),
            "CTOR" => Some(Box::new(CTorus::from(self))),
            "RTOR" => Some(Box::new(RTorus::from(self))),
            "PYRA" => Some(Box::new(LPyramid::from(self))),
            _ => None,
        };
    }
}

// impl Collection for AttrMap {
//     type PrimaryKey = u32;
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "attr")
//     }
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
//
// impl SerializedCollection for AttrMap {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct PdmsTree(pub Tree<EleNode>);

// impl Collection for PdmsTree {
//     type PrimaryKey = u64;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "tree")
//     }
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
// impl SerializedCollection for PdmsTree {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

/// 一个参考号是有可能重复的，project信息可以不用存储，获取信息时必须要带上 db_no
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoInfo {
    /// 参考号的ref0
    pub ref_0: u32, //只需要保存一个ref0的信息，就能知道这个数据在哪个位置
    /// 项目hash
    pub project_hash: u32,
    /// 对应db number
    pub db_no: u32,
}

// impl Collection for RefnoInfo {
//     type PrimaryKey = u32;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "info")
//     }
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
//
// impl SerializedCollection for RefnoInfo {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize, Clone, Debug, Component, Reflect)]
#[reflect(Component)]
pub enum AttrVal {
    InvalidType,
    IntegerType(i32),
    StringType(SmolStr),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<SmolStr>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(SmolStr),
    WordType(SmolStr),

    RefU64Type(RefU64),
    StringHashType(AiosStrHash),
}

impl Inspectable for AttrVal {
    type Attributes = ();

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        options: Self::Attributes,
        context: &mut bevy_inspector_egui::Context,
    ) -> bool {
        let mut changed = false;
        match self {
            StringType(s) | ElementType(s) | WordType(s) => {
                s.ui(ui, Default::default(), context);
            }
            IntegerType(d) => {
                d.ui(ui, Default::default(), context);
            }
            DoubleType(d) => {
                d.ui(ui, Default::default(), context);
            }
            RefU64Type(r) => {
                r.to_refno_str().ui(ui, Default::default(), context);
            }
            Vec3Type(r) => {
                Vec3::new(r[0] as f32, r[1] as f32, r[2] as f32).ui(
                    ui,
                    Default::default(),
                    context,
                );
            }
            BoolType(b) => {
                b.ui(ui, Default::default(), context);
            }
            BoolArrayType(bs) => {
                for b in bs {
                    b.ui(ui, Default::default(), context);
                    ui.end_row();
                }
            }
            DoubleArrayType(ds) => {
                for b in ds {
                    b.ui(ui, Default::default(), context);
                    ui.end_row();
                }
            }
            StringHashType(s) => {
                s.ui(ui, Default::default(), context);
            }
            _ => {}
        }
        changed
    }
}

impl Default for AttrVal {
    fn default() -> Self {
        Self::InvalidType
    }
}

impl AttrVal {
    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            IntegerType(v) => *v,
            _ => 0,
        };
    }

    #[inline]
    pub fn double_value(&self) -> Option<f64> {
        return match self {
            DoubleType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            DoubleType(v) => Some(*v as f32),
            _ => None,
        };
    }

    #[inline]
    pub fn dvec_value(&self) -> Option<Vec<f64>> {
        return match self {
            DoubleArrayType(v) => Some(v.to_vec()),
            _ => None,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PdmsDatabaseInfo {
    pub db_names_map: DashMap<i32, String>,
    // 第一个i32是refno ，第二个i32是type的hash
    pub noun_attr_info_map: DashMap<i32, DashMap<i32, AttrInfo>>,
}

///可以缩放的类型
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScaledGeom {
    Box(Vec3),
    Cylinder(Vec3),
    Sphere(f32),
}

//for json compatibility
pub type PdmsMeshIdx = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum GeoType {
    Box = 0,
    Cylinder,
    Dish,
    Sphere,
    Snout,
    CTorus,
    RTorus,
    Pyramid,
    Revo,
    Extru,
    Polyhedron,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiosMaterial {
    pub color: Vec4,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GeoData {
    Primitive(PdmsMeshIdx), //索引的哪个mesh,和对应的拉伸值， 先从dish开始判断相似性
                            // Raw(Mesh),          //原生的Mesh
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AiosAABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AiosAABB {
    #[inline]
    pub fn new(v1: Vec3, v2: Vec3) -> Self {
        Self { min: v1, max: v2 }
    }

    #[inline]
    pub fn scaled(&mut self, scale: &Vec3) {
        self.min = Vec3::new(
            self.min.x * scale.x,
            self.min.y * scale.y,
            self.min.z * scale.z,
        );
        self.max = Vec3::new(
            self.max.x * scale.x,
            self.max.y * scale.y,
            self.max.z * scale.z,
        );
    }

    #[inline]
    pub fn get_half_extents(&self) -> Vec3 {
        let center = (self.min + self.max) / 2.0;
        self.max - center
    }

    #[inline]
    pub fn get_center(&self) -> Vec3 {
        let center = (self.min + self.max) / 2.0;
        center
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsMeshMgr {
    pub inst_mgr: ShapeInstancesMgr,
    pub cached_mesh_mgr: CachedMeshesMgr,
    pub level_shape_mgr: HashMap<RefU64, RefU64Vec>   //每个非叶子节点都知道自己的所有shape refno
}

impl PdmsMeshMgr {
    #[inline]
    pub fn get_instants_data(&self, refno: RefU64) -> HashMap<RefU64, &Vec<EleGeoInstData>> {
        let mut results = HashMap::new();
        let inst_map = &self.inst_mgr.inst_map;
        if self.level_shape_mgr.contains_key(&refno) {
            for v in self.level_shape_mgr[&refno].iter() {
                if inst_map.contains_key(&v) {
                    results.insert(v.clone(),inst_map.get(&v).unwrap());
                }
            }
        }else{
            if inst_map.contains_key(&refno) {
                results.insert(refno.clone(), inst_map.get(&refno).unwrap());
            }
        }
        results
    }

    // #[inline]
    // pub fn get_bevy_mesh(&self, mesh_hash: &str) -> Option<Mesh> {
    //     if let Some(cached_msh) = self.get_mesh(mesh_hash) {
    //         let bevy_mesh = cached_msh.gen_bevy_mesh();
    //         return Some(bevy_mesh);
    //     }
    //     None
    // }

    pub fn serialize_to_bin_file(&self) -> bool {
        let mut file = File::create(format!("PdmsMeshMgr.bin")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr_{}.bin", db_code))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }

    pub fn serialize_to_json_file(&self) -> bool {
        let mut file = File::create(format!("PdmsMeshMgr.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_json_file() -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr.json"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        let r =  serde_json::from_slice::<Self>(&buf)?;
        Ok(r)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ShapeInstancesMgr {
    pub inst_map: HashMap<RefU64, Vec<EleGeoInstData>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CachedMeshesMgr {
    pub meshes: HashMap<String, PdmsMesh>, //世界坐标系的变换, 为了js兼容64位，暂时使用String
}

impl CachedMeshesMgr {
    //获得对应的id的 EleGeoDatas
    pub fn get_bevy_mesh(&self, mesh_hash: &str) -> Option<(Mesh, Aabb)> {
        if let Some(cached_msh) = self.get_mesh(mesh_hash) {
            let bevy_mesh = cached_msh.gen_bevy_mesh_with_aabb();
            return Some(bevy_mesh);
        }
        None
    }

    pub fn get_mesh(&self, mesh_hash: &str) -> Option<&PdmsMesh> {
        self.meshes.get(mesh_hash)
    }

    //get the mesh index, if not exist, try to create and insert, and return index
    pub fn get_pdms_mesh_hash_key(&mut self, m: Box<dyn BrepShapeTrait>) -> String {
        let hash = m.hash_mesh_params().to_string();
        if !self.meshes.contains_key(&hash) {
            let mesh = m.gen_unit_shape();
            self.meshes.insert(hash.clone(), mesh);
        }
        hash
    }

    pub fn get_bbox(&self, hash: &String) -> Option<AiosAABB> {
        if self.meshes.contains_key(hash) {
            let mesh = self.meshes.get(hash).unwrap();
            return Some(mesh.aabb.clone());
        }
        None
    }

    pub fn serialize_to_bin_file(&self) -> bool {
        let mut file = File::create(format!("cached_meshes.bin")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file() -> Self {
        let mut file = File::open(format!("cached_meshes.bin")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        bincode::deserialize(buf.as_slice()).unwrap()
    }

    pub fn serialize_to_json_file(&self) -> bool {
        let mut file = File::create(format!("cached_meshes.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_json_file() -> Self {
        let mut file = File::open(format!("cached_meshes.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        serde_json::from_slice(&buf).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EleGeoInstData {
    pub geo_hash: String,
    pub bbox: AiosAABB,
    pub global_transform: (Quat, Vec3, Vec3), //世界坐标系的变换, rot, translation, scale
    pub visible: bool,
    pub generic_type: SmolStr, //所属一般类型，ROOM、STRU、PIPE等
    pub zone_refno: RefU64, // 暂时用这个
    pub node_id: NodeId,
}

// impl Collection for EleGeoInstData {
//     type PrimaryKey = u64;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "geoms")
//     }
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
// impl SerializedCollection for EleGeoInstData {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

pub trait PdmsNodeTrait {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64::default()
    }

    #[inline]
    fn get_name_hash(&self) -> u32 {
        0
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }
}

//todo node 不需要多大，这些数据也不用缓存
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNode {
    pub refno: RefU64,
    pub owner: RefU64,
    pub name_hash: AiosStrHash,
    pub noun: u32,
    pub version: u32,
    // pub global_mat: Mat4,   //全局坐标系下的变换矩阵
}

/// 每个dbno对应的version
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DbnoVersion {
    pub dbno: u32,
    pub version: u32,
}

// impl Collection for DbnoVersion {
//     type PrimaryKey = u32;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "vers")
//     }
//
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
//
// impl SerializedCollection for DbnoVersion {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

impl PdmsNodeTrait for EleNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno
    }

    #[inline]
    fn get_name_hash(&self) -> u32 {
        self.name_hash
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        self.noun
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNodeMongoDb {
    pub file_name: SmolStr,
    /// 序列化后的 tree
    pub tree: Vec<u8>,
}

impl EleNodeMongoDb {
    pub fn new(db_name: &str, tree: Tree<EleNode>) -> Self {
        Self {
            file_name: SmolStr::from(db_name),
            tree: bincode::serialize(&tree).unwrap(),
        }
    }
}

impl EleNode {
    // pub fn name(&self) -> &str {
    //     self.name.as_str()
    // }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsMongoAttr {
    pub refno: SmolStr,
    pub attr: AttrMap,
}

#[test]
fn test_dashmap() {
    let mut dashmap_1 = DashMap::new();
    dashmap_1.insert("1", "hello");
    let mut dashmap_2 = DashMap::new();
    dashmap_2.insert("2", "world");
    let mut dashmap_3 = DashMap::new();
    dashmap_1.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dashmap_2.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dbg!(&dashmap_3);
}

#[test]
fn test_refu64() {
    let refno = RefU64::from(RefI32Tuple(((16477, 80))));
    println!("refno={}", refno.0);
}

// #[test]
// fn test_ref_i32_tuple(){
//     let refno:Refi32Tuple = RefU64(65326452626828).into();
//     println!("refno={:?}",refno);
// }


#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum DbAttributeType {
    INTEGER = 1,
    DOUBLE,
    BOOL,
    STRING,
    ELEMENT,
    WORD,
    DIRECTION,
    POSITION,
    ORIENTATION,
    DATETIME,
    DOUBLEVEC,
    INTVEC,
    FLOATVEC,
    TYPEX,
    Vec3Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    pub name: SmolStr,
    pub hash: i32,
    pub offset: u32,
    pub default_val: AttrVal,
    pub att_type: DbAttributeType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PDMSDBInfo {
    pub name: String,
    pub db_no: i32,
    pub db_type: String,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PdmsRefno {
    pub ref_no: String,
    pub db: String,
    pub type_name: String,
}

use crate::db1_dehash;
use crate::db_tool::db1_hash;
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::pyramid::LPyramid;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::shape::pdms_shape::{BrepShapeTrait, PdmsMesh, PdmsPrimShape};
use id_tree::InsertBehavior::*;
use itertools::Itertools;
use ncollide3d::bounding_volume::AABB;
use truck_polymesh::stl::IntoSTLIterator;

#[test]
fn test_id_tree() {
    let mut tree: Tree<i32> = TreeBuilder::new().with_node_capacity(5).build();

    //      0
    //     / \
    //    1   2
    //   / \
    //  3   4
    let root_id: NodeId = tree.insert(id_tree::Node::new(0), AsRoot).unwrap();
    let child_id: NodeId = tree
        .insert(id_tree::Node::new(1), UnderNode(&root_id))
        .unwrap();
    tree.insert(id_tree::Node::new(2), UnderNode(&root_id))
        .unwrap();
    tree.insert(id_tree::Node::new(3), UnderNode(&child_id))
        .unwrap();
    tree.insert(id_tree::Node::new(4), UnderNode(&child_id))
        .unwrap();

    println!("Pre-order:");
    for node in tree.children(&root_id).unwrap() {
        print!("{}, ", node.data());
    }
}

pub type AiosStrHash = u32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiosStr(pub SmolStr);

impl AiosStr {
    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hash, Hasher};
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish()
    }
    pub fn take(mut self) -> SmolStr {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for AiosStr {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl hash32::Hash for AiosStr {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write(self.0.as_str().as_bytes());
        state.write(&[0xff]);
    }
}

// impl Collection for AiosStr {
//     type PrimaryKey = u32;
//
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "strings")
//     }
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }
//
// impl SerializedCollection for AiosStr {
//     type Contents = Self;
//     type Format = transmog_bincode::Bincode;
//     fn format() -> Self::Format {
//         transmog_bincode::Bincode::default()
//     }
// }

//todo make it as database
#[derive(Component, Debug, Default, Clone, Serialize, Deserialize)]
pub struct StringLookupTable {
    pub lookup: HashMap<u32, AiosStr>,
}

impl StringLookupTable {
    pub fn new() -> Self {
        Self {
            lookup: HashMap::new(),
        }
    }

    pub fn get_string(&self, hash: u32) -> Option<SmolStr> {
        self.lookup.get(&hash).map(|x| x.0.clone())
    }

    pub fn add_str(&mut self, str_val: &str) -> u32 {
        use hash32::{FnvHasher, Hash, Hasher};
        let mut fnv = FnvHasher::default();
        str_val.hash(&mut fnv);
        let hash = fnv.finish();

        self.lookup.entry(hash).or_insert(AiosStr(str_val.into()));
        hash
    }

    pub fn merge(&mut self, other: &Self) -> bool {
        for (k, v) in &other.lookup {
            self.lookup.insert(*k, v.clone());
        }
        true
    }

    pub fn serialize_to_default_json_file(&self) -> bool {
        let mut file = File::create(format!("./AIOS_DBS/AIOS_name_lookup.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_default_json_file(name: &str) -> Option<Self> {
        if let Ok(mut file) = File::open(format!("./AIOS_DBS/AIOS_name_lookup.json")) {
            let mut bytes = vec![];
            file.read_to_end(&mut bytes);
            return serde_json::from_slice::<Self>(bytes.as_slice()).ok();
        }
        None
    }
}

#[test]
fn query_db() {

}
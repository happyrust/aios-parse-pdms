use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use dashmap::DashMap;
// use gdnative::prelude::{Transform, Vector3};
use highway::{HighwayHash, HighwayHasher, Key};
use id_tree::{NodeId, Tree, TreeBuilder};
use serde::{Serialize, Deserialize};
use smol_str::SmolStr;
use crate::consts::UNSET_STR;
use crate::pdms_types::AttrVal::{BoolArrayType, BoolType, DoubleArrayType, DoubleType, ElementType, IntArrayType, IntegerType, RefU64Type, StringArrayType, StringType, Vec3Type, WordType};
use crate::helper::get_attr_value_f64_vec;
// use bevy_inspector_egui::Inspectable;
use bevy::prelude::*;
use bonsaidb::core::Error;
use bonsaidb::core::schema::{Collection, CollectionName, DefaultSerialization, Schematic, SerializedCollection};
use glam::TransformSRT;


//todo wrap noun hash
pub struct NounHash(pub i32);

///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct Refi32Tuple(pub (i32, i32));

impl Into<SmolStr> for Refi32Tuple {
    fn into(self) -> SmolStr {
        SmolStr::from(format!("{}/{}", self.get_0(), self.get_1()))
    }
}

impl Into<String> for Refi32Tuple {
    fn into(self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
    }
}

impl From<&[u8]> for Refi32Tuple {
    fn from(input: &[u8]) -> Self {
        Self::new(i32::from_be_bytes(input[0..4].try_into().unwrap()), i32::from_be_bytes(input[4..8].try_into().unwrap()))
    }
}

impl From<&str> for Refi32Tuple {
    fn from(s: &str) -> Self {
        let x: Vec<i32> = s.split('/').map(|x| x.parse::<i32>().unwrap_or_default()).collect();
        Self::new(x[0], x[1])
    }
}

impl From<&RefU64> for Refi32Tuple {
    fn from(n: &RefU64) -> Self {
        let n = n.0.to_be_bytes();
        Self((
            i32::from_be_bytes(n[..4].try_into().unwrap()),
            i32::from_be_bytes(n[4..].try_into().unwrap())
        ))
    }
}

impl Refi32Tuple {

    #[inline]
    pub fn new(ref_0: i32, ref_1: i32) -> Self{
        Self{
            0: (ref_0, ref_1)
        }
    }

    #[inline]
    pub fn get_0(&self) -> i32 { self.0.0 }

    #[inline]
    pub fn get_1(&self) -> i32 { self.0.1 }
}




//把Refno当作u64
#[derive(Hash, Serialize, Deserialize, Clone, Copy, Debug, Default, Component, Eq, PartialEq)]
pub struct RefU64(pub u64);

impl Deref for RefU64{
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&Refi32Tuple> for RefU64 {
    fn from(n: &Refi32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<Refi32Tuple> for RefU64 {
    fn from(n: Refi32Tuple) -> Self {
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
    pub fn to_refno_str(&self) -> SmolStr{
        let refno: Refi32Tuple = self.into();
        refno.into()
    }

    #[inline]
    pub fn from_two_nums(i: u32, j: u32) -> Self{
        let bytes: Vec<u8> = [i.to_be_bytes(), j.to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct RefU64Vec(pub Vec<RefU64>);

impl Deref for RefU64Vec{
    type Target = Vec<RefU64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//存储children，也可以这么去存储
impl Collection for RefU64Vec {
    fn collection_name() -> CollectionName {
        CollectionName::new("aios", "refnos")
    }

    fn define_views(schema: &mut Schematic) -> Result<(), Error> {
        Ok(())
    }
}

impl SerializedCollection for RefU64Vec {
    type Format = transmog_bincode::Bincode;
    type Contents = Self;

    fn format() -> Self::Format {
        // The bincode options can be set on this type
        transmog_bincode::Bincode::default()
    }
}


impl RefU64Vec{
    #[inline]
    pub fn push(&mut self, v: RefU64){
        self.0.push(v);
    }
}

//parent可以存到一直到root


///PDMS的属性数据Map
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct AttrMap{
    pub map: HashMap<SmolStr, AttrVal>
}
impl AttrMap {

    #[inline]
    pub fn insert(&mut self, k: SmolStr, v: AttrVal){
        self.map.entry(k).or_insert(v);
    }

    #[inline]
    pub fn get_name(&self) -> SmolStr{
        self.get_as_string("NAME").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_refno_as_string(&self) -> SmolStr{
        self.get_as_string("REFNO").unwrap_or(UNSET_STR.into())
    }



    #[inline]
    pub fn get_refno(&self) -> Option<RefU64>{
        if let Some(RefU64Type(d)) = self.map.get("REFNO"){
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner(&self) -> Option<RefU64>{
        if let Some(RefU64Type(d)) = self.map.get("OWNER"){
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> SmolStr{
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_type(&self) -> SmolStr{
        self.get_as_string("TYPE").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32>{
        if let Some(v) = self.map.get(key){
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
    pub fn get_as_string(&self, key: &str) -> Option<SmolStr>{
        if let Some(v) = self.map.get(key){
            let s = match v {
                StringType(s) | WordType(s) | ElementType(s) => s.clone(),
                IntegerType(d)  => d.to_string().into(),
                DoubleType(d)  => d.to_string().into(),
                BoolType(d)  => d.to_string().into(),
                DoubleArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>().into(),
                StringArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>().into(),
                IntArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>().into(),
                BoolArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>().into(),
                Vec3Type(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>().into(),
                RefU64Type(d) => Refi32Tuple::from(d).into(),
                _ => { UNSET_STR.into() }
            };
            return Some(s);
        }
        None
    }

    #[inline]
    pub fn get_as_vec_string(&self, key: &str) -> Vec<SmolStr>{
        if let Some(v) = self.map.get(key){
            return match v {
                StringArrayType(d) => d.clone(),
                _ => { vec![] }
            };
        }
        vec![]
    }

    #[inline]
    pub fn get_as_vec_refnos(&self, key: &str) -> Vec<SmolStr>{
        if let Some(v) = self.map.get(key){
            return match v {
                IntArrayType(d) => d.chunks_exact(2).map(|x| format!("{}/{}", x[0], x[1]).into()).collect(),
                _ => { vec![] }
            };
        }
        vec![]
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> bool{
        if let Some(v) = self.map.get(key){
            match v {
                BoolType(b)  => *b,
                _ => false,
            }
        }else{
            false
        }
    }


    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&AttrVal>{
        if let Some(v) = self.map.get(key) {
            Some(v)
        }else{
            None
        }
    }

    #[inline]
    pub fn get_translation(&self) -> Vec3{
        if let Some(pos) = get_attr_value_f64_vec(self, "POS") {
            return glam::f32::Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        }

        Vec3::ZERO
    }

    #[inline]
    pub fn get_rotation(&self) -> Quat{
        if let Some(ang) = get_attr_value_f64_vec(self, "ORI"){
            let mat3 = Mat3::from_rotation_z(ang[2].to_radians() as f32)
                * Mat3::from_rotation_y(ang[1].to_radians() as f32)
                * Mat3::from_rotation_x(ang[0].to_radians() as f32);

            // let mat3 = Mat3::from_rotation_x(ang[0].to_radians() as f32)
            //     * Mat3::from_rotation_y(ang[1].to_radians() as f32)
            //     * Mat3::from_rotation_z(ang[2].to_radians() as f32);

            return Quat::from_mat3(&mat3);
        }

        Quat::IDENTITY
    }

    pub fn get_tansformRT(&self) -> glam::TransformRT{
        let mut tr = glam::TransformRT::IDENTITY;
        if let Some(pos) = get_attr_value_f64_vec(self, "POS") {
            tr.translation = glam::f32::Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        }
        if let Some(ang) = get_attr_value_f64_vec(self, "ORI"){
            tr.rotation = self.get_rotation();
        }
        tr
    }


    pub fn get_matrix(&self) -> glam::f32::Affine3A{
        let mut affine = glam::f32::Affine3A::IDENTITY;
        if let Some(pos) = get_attr_value_f64_vec(self, "POS") {
            affine.translation = glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        }
        if let Some(ang) = get_attr_value_f64_vec(self, "ORI"){
            affine.matrix3 = glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32) * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)  * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32);
        }
        affine
    }

    #[inline]
    pub fn get_mat4(&self) -> glam::f32::Mat4{
        glam::f32::Mat4::from(self.get_matrix())
    }
    //
    // pub fn get_transform(&self) -> Transform{
    //     let matrix = self.get_matrix();
    //     let x = &matrix.matrix3.col(0);
    //     let y = &matrix.matrix3.col(1);
    //     let z = &matrix.matrix3.col(2);
    //     let p = &matrix.translation;
    //     Transform::from_basis_origin(
    //         Vector3::new(x[0], x[1], x[2]),
    //         Vector3::new(y[0], y[1], y[2]),
    //         Vector3::new(z[0], z[1], z[2]),
    //         Vector3::new(p[0], p[1], p[2]))
    // }

    pub fn get_f64_vec(&self, att: &str) -> Option<Vec<f64>> {
        let mut v = vec![];
        if let Some(val) = self.map.get(att) {
            match val {
                AttrVal::DoubleArrayType(data) => {
                    v = data.clone();
                    return Some(v);
                }
                AttrVal::Vec3Type(data) => {
                    v = data.to_vec();
                    return Some(v);
                }
                _ => {}
            }
        }
        None
    }


    ///使用spref + params 混合成的meshid
    pub fn cal_des_mesh_id(&self) -> u64{
        let key = Key([1, 2, 3, 4]);
        let mut hasher64 = HighwayHasher::new(key);
        if let Some(spref) = self.get_as_string("SPRE"){
            hasher64.append(spref.as_ref());
        }
        if let Some(para) = self.get_f64_vec("PARA"){
            let output: Vec<u8> = para.iter().flat_map(|val| val.to_be_bytes()).collect();
            hasher64.append(&output);
        }
        if let Some(d) = self.get_as_string("RADI"){
            hasher64.append(d.as_ref());
        }
        if let Some(d) = self.get_as_string("HEIG"){
            hasher64.append(d.as_ref());
        }
        if let Some(d) = self.get_as_string("ANGL"){
            hasher64.append(d.as_ref());
        }

        let id = hasher64.finalize64();
        id
    }

}

impl Collection for AttrMap {
    fn collection_name() -> CollectionName {
        CollectionName::new("aios", "attrs")
    }
    fn define_views(schema: &mut Schematic) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializedCollection for AttrMap {
    type Contents = Self;
    type Format = transmog_bincode::Bincode;
    fn format() -> Self::Format {
        // The bincode options can be set on this type
        transmog_bincode::Bincode::default()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct PdmsTree(pub Tree<EleNode>);

impl Collection for PdmsTree {
    fn collection_name() -> CollectionName {
        CollectionName::new("aios", "tree")
    }
    fn define_views(schema: &mut Schematic) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializedCollection for PdmsTree {
    type Contents = Self;
    type Format = transmog_bincode::Bincode;
    fn format() -> Self::Format {
        transmog_bincode::Bincode::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoInfo {
    /// 参考号
    pub refno: RefU64,
    /// 文件名
    // pub file_name: SmolStr,         //todo if need, make it use index
    /// 参考号对应的node_id
    pub node_id: NodeId,
    /// 子节点
    pub children: RefU64Vec,
}

impl Collection for RefnoInfo {
    fn collection_name() -> CollectionName {
        CollectionName::new("aios", "info")
    }
    fn define_views(schema: &mut Schematic) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializedCollection for RefnoInfo {
    type Contents = Self;
    type Format = transmog_bincode::Bincode;
    fn format() -> Self::Format {
        transmog_bincode::Bincode::default()
    }
}




#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
}

impl AttrVal {

    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            IntegerType(v) => {
                *v
            }
            _ => {
                0
            }
        }
    }

    #[inline]
    pub fn double_value(&self) -> Option<f64> {
        return match self {
            DoubleType(v) => {
                Some(*v)
            }
            _ => { None }
        }
    }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            DoubleType(v) => {
                Some(*v as f32)
            }
            _ => { None }
        }
    }

    #[inline]
    pub fn dvec_value(&self) -> Option<Vec<f64>> {
        return match self {
            DoubleArrayType(v) => {
                Some(v.to_vec())
            }
            _ => { None }
        }
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
pub enum ScaledGeom{
    Box(Vec3),
    Cylinder(Vec3),
    Sphere(f32),
}

pub type PdmsMeshIdx = u64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GeoData{
    Scaled(ScaledGeom),  //可以用Unit Shape缩放的类型
    Primitive((PdmsMeshIdx, Vec3)),  //索引的哪个mesh,和对应的拉伸值， 先从dish开始判断相似性
    // Raw(Mesh),          //原生的Mesh
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CachedMeshes{
    pub meshes: HashMap<u64, PdmsMesh>,    //世界坐标系的变换
}

impl CachedMeshes {
    //get the mesh index, if not exist, try to create and insert, and return index
    pub fn get_pdms_mesh_hash_key<T: BrepShape>(&mut self, m: &T) -> (u64, Vec3){
        let hash = m.hash_mesh_params();
        if !self.meshes.contains_key(&hash) {
            let mesh = m.gen_unit_shape();
            self.meshes.insert(hash, mesh);
        }
        let scaled = m.get_scaled_vec3();
        (hash, scaled)
    }

    pub fn serialize_to_json_file(&self) -> bool{
        let mut file = File::create(format!("./cached_meshes.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EleGeoData{
    pub geo: GeoData,
    // pub bbox: AABB,
    pub global_transform: (Quat, Vec3),    //世界坐标系的变换
}

impl Collection for EleGeoData {
    fn collection_name() -> CollectionName {
        CollectionName::new("aios", "geoms")
    }
    fn define_views(schema: &mut Schematic) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializedCollection for EleGeoData {
    type Contents = Self;
    type Format = transmog_bincode::Bincode;
    fn format() -> Self::Format {
        transmog_bincode::Bincode::default()
    }
}


//todo node 不需要多大，这些数据也不用缓存
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNode {
    pub refno: RefU64,
    pub owner: RefU64,
    pub name: SmolStr,   //todo make it as a index of name table
    pub noun: u32,
    pub version: u32,
    // pub global_mat: Mat4,   //全局坐标系下的变换矩阵
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNodeMongoDb {
    pub file_name : SmolStr,
    /// 序列化后的 tree
    pub tree : Vec<u8>,
}

impl EleNodeMongoDb {
    pub fn new(db_name:&str,tree:Tree<EleNode>) -> Self {
        Self {
            file_name: SmolStr::from(db_name),
            tree: bincode::serialize(&tree).unwrap(),
        }
    }
}

impl EleNode {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl fmt::Display for EleNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name().fmt(f)
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsMongoAttr {
    pub refno:SmolStr,
    pub attr:AttrMap,
}

#[test]
fn test_dashmap() {
    let mut dashmap_1 = DashMap::new();
    dashmap_1.insert("1","hello");
    let mut dashmap_2 = DashMap::new();
    dashmap_2.insert("2","world");
    let mut dashmap_3=DashMap::new();
    dashmap_1.iter().for_each(|m|{ dashmap_3.insert(m.key().clone(),m.value().clone()); });
    dashmap_2.iter().for_each(|m|{ dashmap_3.insert(m.key().clone(),m.value().clone()); });
    dbg!(&dashmap_3);
}


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

use id_tree::InsertBehavior::*;
use itertools::Itertools;
use ncollide3d::bounding_volume::AABB;
use crate::prim_geo::pdms_shape::{BrepShape, PdmsMesh, PdmsPrimShape};

#[test]
fn test_id_tree() {


    let mut tree: Tree<i32> = TreeBuilder::new()
        .with_node_capacity(5)
        .build();

    //      0
    //     / \
    //    1   2
    //   / \
    //  3   4
    let root_id: NodeId = tree.insert(id_tree::Node::new(0), AsRoot).unwrap();
    let child_id: NodeId = tree.insert(id_tree::Node::new(1), UnderNode(&root_id)).unwrap();
    tree.insert(id_tree::Node::new(2), UnderNode(&root_id)).unwrap();
    tree.insert(id_tree::Node::new(3), UnderNode(&child_id)).unwrap();
    tree.insert(id_tree::Node::new(4), UnderNode(&child_id)).unwrap();

    println!("Pre-order:");
    for node in tree.children(&root_id).unwrap() {
        print!("{}, ", node.data());
    }
}
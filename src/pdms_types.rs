use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use crate::db_tool::db1_dehash;

pub type RefNoTuple = (i32, i32);



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AttrVal {
    InvalidType,
    IntegerType(i32),
    StringType(String),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),

}

#[derive(Serialize, Deserialize, Debug)]
pub struct PdmsDatabaseInfo {
    pub db_names_map: DashMap<i32, String>,
    // 第一个i32是refno ，第二个i32是type的hash
    pub noun_attr_info_map: DashMap<i32, DashMap<i32, AttrInfo>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ElementData {
    //当前节点
    pub ref_no: String,
    pub name: String,
    pub noun_name: String,
    pub noun_hash: i32,
    //子节点
    pub children: Vec<String>,
    //父节点
    pub owner: String,
    pub attr_data_map: DashMap<String, AttrVal>,
    pub order: i32,
}

impl ElementData {
    // pub fn get_type_name(&self) -> String {
    //     db1_dehash(self.noun_hash as u32)
    // }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleDataNode {
    pub ref_no: String,
    pub children: Vec<String>,
    pub owner: String,
    pub name: String,
    pub order: i32,
    pub db_name:String,
    pub type_name: String,
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
    FLOATVEC
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    pub name: String,
    pub hash: i32,
    pub offset: u32,
    pub default_val: AttrVal,
    pub att_type: DbAttributeType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PDMSDBInfo{
    pub name: String,
    pub db_no: i32,
    pub db_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Table{
    pub ref_no:String,
    pub db:String,
    pub type_name:String
}
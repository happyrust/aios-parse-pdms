use dashmap::DashMap;
use gdnative::prelude::{Transform, Vector3};
use highway::{HighwayHash, HighwayHasher, Key};
use serde::{Serialize, Deserialize};
use crate::pdms_types::AttrVal::{BoolArrayType, BoolType, DoubleArrayType, DoubleType, ElementType, IntArrayType, IntegerType, StringArrayType, StringType, Vec3Type, WordType};
use crate::helper::get_attr_value_f64_vec;


pub type RefNoTuple = (i32, i32);

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AttrMap{
    pub map: DashMap<String, AttrVal>
}

impl AttrMap {

    #[inline]
    pub fn get_name(&self) -> String{
        self.get_as_string("NAME").unwrap_or("unset".to_string())
    }

    #[inline]
    pub fn get_refno(&self) -> String{
        self.get_as_string("REFNO").unwrap_or("unset".to_string())
    }

    #[inline]
    pub fn get_owner(&self) -> String{
        self.get_as_string("OWNER").unwrap_or("unset".to_string())
    }

    #[inline]
    pub fn get_type(&self) -> String{
        self.get_as_string("TYPE").unwrap_or("unset".to_string())
    }

    #[inline]
    pub fn get_as_string(&self, key: &str) -> Option<String>{
        if let Some(v) = self.map.get(key){
            let s = match v.value() {
                StringType(s) | WordType(s) | ElementType(s) => s.trim().to_string(),
                IntegerType(d)  => d.to_string(),
                DoubleType(d)  => d.to_string(),
                BoolType(d)  => d.to_string(),
                DoubleArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>(),
                StringArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>(),
                IntArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>(),
                BoolArrayType(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>(),
                Vec3Type(d) => d.iter().map(|i| format!(" {}", i)).collect::<String>(),
                _ => { "unset".to_string() }
            };
            return Some(s);
        }
        None
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> bool{
        if let Some(v) = self.map.get(key){
            match v.value() {
                BoolType(b)  => *b,
                _ => false,
            }
        }else{
           false
        }
    }


    #[inline]
    pub fn get(&self, key: &str) -> Option<AttrVal>{
        if let Some(v) = self.map.get(key) {
            Some(v.value().clone())
        }else{
            None
        }
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

    pub fn get_transform(&self) -> Transform{
        let matrix = self.get_matrix();
        let x = &matrix.matrix3.col(0);
        let y = &matrix.matrix3.col(1);
        let z = &matrix.matrix3.col(2);
        let p = &matrix.translation;
        Transform::from_basis_origin(
            Vector3::new(x[0], x[1], x[2]),
            Vector3::new(y[0], y[1], y[2]),
            Vector3::new(z[0], z[1], z[2]),
            Vector3::new(p[0], p[1], p[2]))
    }

    pub fn get_f64_vec(&self, att: &str) -> Option<Vec<f64>> {
        let mut v = vec![];
        if let Some(val) = self.map.get(att) {
            match val.value() {
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

impl AttrVal {
    pub fn get_attrval_value_in_integer_type(&self) -> i32 {
        match self {
            IntegerType(v) => {
                return *v;
            }
            _ => {
                return 0;
            }
        }
    }
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
    pub version:u32,
    //子节点, 临时存储children
    // #[serde(skip_serializing)]
    pub children: Vec<RefNoTuple>,
    //父节点
    pub owner: String,
    pub attr_data_map: DashMap<String, AttrVal>,
    pub order: i32,
}

impl ElementData {}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleDataNode {
    pub ref_no: String,
    pub children: Vec<String>,
    pub owner: String,
    pub name: String,
    pub order: i32,
    pub db_name: String,
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
    FLOATVEC,
    TYPEX,
    Vec3Type,
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


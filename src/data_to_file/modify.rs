use std::fs;
use std::intrinsics::ceilf32;
use std::io::Read;
use bevy::ecs::schedule::ShouldRun::No;
use bevy::render::render_resource::std430::Std430;
use bitvec::prelude::*;
use dashmap::DashMap;
use itertools::concat;
use serde::{Serialize, Deserialize};
use memchr::memmem::{find_iter, rfind_iter};
use nalgebra::min;
use nalgebra_glm::e;
use nom::combinator::value;
use notify::Op;
use phf::phf_map;
use regex::Replacer;
use smol_str::SmolStr;
use crate::AttrVal::BoolType;
use crate::data_to_file::{DataPage, NewPage};
use crate::db_tool::db1_hash;
use crate::helper::{parse_to_i32, parse_to_u16, parse_to_u32};
use crate::interface::pdms_interface::{MResult, PdmsInterface};
use crate::pdms_types::{AttrInfo, AttrVal, PdmsDatabaseInfo, Refi32Tuple, RefU64};
use crate::read_attr_info_config;
use crate::test_cases::convert_str_to_bytes;

pub static PACKAGE_TYPE: phf::Map<&'static str, i32> = phf_map! {
    "CATE" => 0x9D572i32,
    "CATA" => 0x8A1E6i32,
    "SECT" => 0xE26D2i32,
    "WORL" => 0xBEB83i32,
    "SPWL" => 0xBF9D7i32,
    "PRTWLD" => 0x3DC5838i32,
    "TABGRO" => 0xD70673Ei32,
    "CTABLE" => 0x4B0C612i32,
    "PRTELE" => 0x4B1E2ADi32,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModifyNewData {
    pub refno: RefU64,
    pub attr_type: String,
    pub noun_type: String,
    pub data: AttrVal,
}

impl ModifyNewData {
    pub fn get_refno_bytes(&self) -> Vec<u8> {
        self.refno.0.to_be_bytes().to_vec()
    }

    pub fn get_refno_and_type_bytes(&self) -> Vec<u8> {
        let mut refno = self.get_refno_bytes();
        let mut attr_type = db1_hash(&self.attr_type).to_be_bytes()[..4].to_vec();
        refno.append(&mut attr_type);
        refno
    }

    pub fn get_type_hash_u32(&self) -> u32 {
        db1_hash(&self.attr_type)
    }

    pub fn get_noun_hash_u32(&self) -> u32 {
        db1_hash(&self.noun_type)
    }

    pub fn get_noun_hash_vec(&self) -> Vec<u8> {
        db1_hash(&self.noun_type).to_be_bytes()[..4].to_vec()
    }

    /// 将显示属性转换成pdms格式
    pub(crate) fn convert_explicit_data_to_bytes(mut noun_hash: Vec<u8>, mut type_len: Vec<u8>, len: Option<Vec<u8>>, mut data: Vec<u8>) -> Vec<u8> {
        noun_hash.append(&mut type_len);
        if let Some(mut len) = len {
            noun_hash.append(&mut len);
        }
        // 将数据位数补成4的倍数
        let r = 4 - data.len() % 4;
        noun_hash.append(&mut data);
        if r != 4 {
            for _ in 0..r {
                noun_hash.push(0);
            }
        }
        noun_hash
    }

    fn convert_implicit_data_to_bytes(len: Option<Vec<u8>>, data: Vec<u8>) -> Vec<u8> {
        if let Some(len) = len {
            [len, data].concat()
        } else {
            data
        }
    }

    pub fn convert_implicit_data_to_vec(&self,b_f64: bool) -> Vec<u8> {
        match self.data.clone() {
            AttrVal::Vec3Type(values) => {
                let mut value = vec![];
                if b_f64 {
                    for v in values {
                        if let [a, b, c, d, e, f, g, h] = v.to_be_bytes() {
                            value.push(vec![e, f, g, h, a, b, c, d]);
                        }
                    }
                } else {
                    for v in values {
                        value.push(v.to_be_bytes().to_vec());
                    }
                }
                let value = value.into_iter().flatten().collect::<Vec<u8>>();
                ModifyNewData::convert_implicit_data_to_bytes(Some(vec![0, 0, 0, 3]), value)
            }
            AttrVal::IntArrayType(values) => {
                let mut value = vec![];
                for v in values {
                    value.push(v.to_be_bytes().to_vec());
                }
                let len = (value.len() as u32).to_be_bytes().to_vec();
                let value = value.into_iter().flatten().collect::<Vec<u8>>();
                ModifyNewData::convert_implicit_data_to_bytes(Some(len), value)
            }
            AttrVal::WordType(v) => {
                let value = db1_hash(v.as_str()).to_be_bytes().to_vec();
                ModifyNewData::convert_implicit_data_to_bytes(None, value)
            }
            AttrVal::RefU64Type(v) => {
                let refno = Refi32Tuple::from(v).0;
                let value = [refno.0.to_be_bytes().to_vec(), refno.1.to_be_bytes().to_vec()].concat();
                ModifyNewData::convert_implicit_data_to_bytes(None, value)
            }

            _ => {
                vec![]
            }
        }
    }

    pub fn convert_explicit_data_to_vec(&self, b_f64: bool) -> Vec<u8> {
        let mut noun_hash = self.get_noun_hash_vec();
        match self.data.clone() {
            AttrVal::IntegerType(v) => {
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, vec![0xC, 0, 0, 1], None, v.to_be_bytes()[..4].to_vec())
            }
            AttrVal::WordType(v) => {
                let v = db1_hash(v.as_str()).to_be_bytes().to_vec();
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, vec![0xC, 0, 0, 1], None, v)
            }
            AttrVal::StringType(v) => {
                let v = v.as_bytes();
                let len = v.len() as f32;
                let mut l = [vec![0x3C, 0], (((len / 4.0).ceil() + 1.0) as u16).to_be_bytes().to_vec()].concat();
                let len = (len as u32).to_be_bytes().to_vec();
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, l, Some(len), v.to_vec())
            }
            AttrVal::BoolType(v) => {
                if v {
                    ModifyNewData::convert_explicit_data_to_bytes(noun_hash, vec![0x14, 0, 0, 1], None, vec![0, 0, 0, 1])
                } else {
                    ModifyNewData::convert_explicit_data_to_bytes(noun_hash, vec![0x14, 0, 0, 1], None, vec![0, 0, 0, 0])
                }
            }
            AttrVal::DoubleType(v) => {
                if let [a, b, c, d, e, f, g, h] = v.to_be_bytes() {
                    let value = vec![e, f, g, h, a, b, c, d];
                    ModifyNewData::convert_explicit_data_to_bytes(noun_hash, vec![8, 0, 0, 2], None, value)
                } else {
                    vec![]
                }
            }
            AttrVal::DoubleArrayType(values) => {
                let mut value = vec![];
                let mut l = vec![];
                if b_f64 {
                    for v in values {
                        if let [a, b, c, d, e, f, g, h] = v.to_be_bytes() {
                            value.push(vec![e, f, g, h, a, b, c, d]);
                        }
                    }
                    l = ((value.len() * 2 + 1) as u16).to_be_bytes()[..2].to_vec();
                } else {
                    for v in values {
                        value.push((v as f32).to_be_bytes().to_vec());
                    }
                    l = ((value.len() + 1) as u16).to_be_bytes()[..2].to_vec();
                }

                l = [vec![0x18, 0], l].concat();
                let len = (value.len() as u32).to_be_bytes()[..4].to_vec();
                let value = value.into_iter().flatten().collect();
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, l, Some(len), value)
            }
            AttrVal::Vec3Type(values) => {
                let mut value = vec![];
                let mut l = vec![];
                if b_f64 {
                    for v in values {
                        if let [a, b, c, d, e, f, g, h] = v.to_be_bytes() {
                            value.push(vec![e, f, g, h, a, b, c, d]);
                        }
                    }
                    l = ((value.len() * 2 + 1) as u16).to_be_bytes()[..2].to_vec();
                } else {
                    for v in values {
                        value.push((v as f32).to_be_bytes().to_vec());
                    }
                    l = ((value.len() + 1) as u16).to_be_bytes()[..2].to_vec();
                }

                l = [vec![0x18, 0], l].concat();
                let value = value.into_iter().flatten().collect();
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, l, Some(vec![0, 0, 0, 3]), value)
            }
            AttrVal::IntArrayType(values) => {
                let mut value = vec![];
                for v in values {
                    value.push(v.to_be_bytes().to_vec());
                }
                let len = (value.len() as u32).to_be_bytes().to_vec();
                let l = [vec![0x0, 0], ((value.len() + 1) as u16).to_be_bytes()[..2].to_vec()].concat(); // 还没找到pdms文件中的IntArrayType数据
                let value = value.into_iter().flatten().collect::<Vec<u8>>();
                ModifyNewData::convert_explicit_data_to_bytes(noun_hash, l, Some(len), value)
            }

            _ => {
                vec![]
            }
        }
    }
}

fn modify_bool_implicit_data(input:&[u8],offset: u32,value:bool) -> u32{
    let val_off = offset & 0xFFFFF;
    let index = ( val_off >> 0x14 ) as usize;
    println!("index={}",index);
    let pos = (val_off *4) as usize;
    let mut val = parse_to_u32(&input[pos..pos+4]);
    let mut bits = val.view_bits_mut::<Lsb0>();
    bits.set(index,value);
    bits.load_be()
}

#[test]
fn modify_bool_implicit_data_test() {
    let input = "00 00 00 00 00 00 00 00";
    let data = convert_str_to_bytes(input);
    let r= modify_bool_implicit_data(&data,1,true);
    println!("r={}",r);
}

#[test]
fn test_convert_explicit_data_to_vec() {
    let new_data = ModifyNewData {
        refno: Refi32Tuple((23984, 1046)).into(),
        attr_type: "SBOX".to_string(),
        noun_type: "NAPP".to_string(),
        data: AttrVal::RefU64Type(Refi32Tuple((23984, 1046)).into()),
    }.convert_explicit_data_to_vec(false);
    println!("new_data={:#4X?}", new_data);
}

#[test]
fn test_convert_implicit_data_to_vec() {
    let new_data = ModifyNewData {
        refno: Refi32Tuple((23984, 1046)).into(),
        attr_type: "SBOX".to_string(),
        noun_type: "NAPP".to_string(),
        data: AttrVal::RefU64Type(Refi32Tuple((23984, 1046)).into()),
    }.convert_implicit_data_to_vec(false);
    println!("new_data={:#4X?}", new_data);
}


/// 修改原文件的version,并将旧的version返回
pub fn change_origin_file(path: &str) -> (u32, Vec<u8>) {
    let mut file = fs::File::open(path).unwrap();
    let mut buf = Vec::new();
    file.read_to_end(&mut buf);
    let old_version = parse_to_u32(&buf[40..44]);
    let new_version_u32 = old_version + 6;
    let new_version = (old_version + 6).to_be_bytes()[..4].to_vec();
    buf.splice(40..44, new_version);
    (new_version_u32, buf)
}

#[tokio::test]
async fn convert_new_pdms_file_test() -> MResult<()> {
    let new_data = ModifyNewData {
        refno: Refi32Tuple((23984, 1046)).into(),
        attr_type: "SBOX".to_string(),
        noun_type: "LEVE".to_string(),
        data: AttrVal::IntArrayType(vec![1, 11]),
    };
    let path = r"E:\AVEVA\Plant\PDMS12.0.SP4\project\Sample\sam000\sam7600_0001";
    convert_new_pdms_file(path, new_data, "sam7600_0001_new").await?;
    Ok(())
}

pub async fn convert_new_pdms_file(path: &str, new_data: ModifyNewData, filename: &str) -> MResult<()> {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let (version, input) = change_origin_file(path);
    let second_version_refno = &[0x0u8, 0x0, 0x5D, 0xB0, 0x0, 0x0, 0x3, 0xB0];
    if let Some(origin_data_page) = find_data_in_origin_file(&new_data.get_refno_and_type_bytes(), &input) {
        let refno_bytes = new_data.get_refno_bytes();
        let refno = new_data.refno.clone();
        if let Some(new_data_page) = convert_new_data_page(origin_data_page, new_data, version) {
            if let Some(first_version_page) = convert_first_version_page(&input, &refno_bytes, version) {
                if let Some(second_version_page) = convert_second_version_page(&input, second_version_refno, version) {
                    if let Some((change_times_page, change_times)) = convert_change_times_page(&input, refno, &mut interface).await? {
                        if let Some(conversion_page) = convert_conversation_page(&input, version, &change_times) {
                            let new_file = NewPage {
                                origin_file: input,
                                data_page: new_data_page,
                                first_version_page,
                                second_version_page,
                                change_times_page,
                                conversion_page,
                            }.convert_into_one_page();
                            let path = filename;
                            fs::write(path, new_file)?;
                            println!("写入文件成功");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// 传入 refno + type 返回该数据在pdms文件中的位置
pub fn find_data_in_origin_file(input: &[u8], buf: &[u8]) -> Option<DataPage> {
    if let Some(pos) = rfind_iter(&buf, input).next() {
        let implicit_data_len = (u32::from_be_bytes(buf[pos - 4..pos].try_into().unwrap()) * 4 - 4) as usize;
        let implicit_data = [vec![0x0, 0x0, 0x0, 0x7], buf[pos - 4..pos + implicit_data_len].to_vec()].concat();
        let (children_data, explicit_data) = get_origin_children_and_explicit_data(&buf, pos + implicit_data_len);
        return Some(DataPage {
            implicit_data,
            children: children_data,
            explicit_data,
        });
    }
    None
}

/// 将修改的值写入到 DataPage中
fn convert_new_data_page(mut page: DataPage, data: ModifyNewData, version: u32) -> Option<Vec<u8>> {
    let mut new_data = vec![];
    let pdms_database_info = read_attr_info_config("all_attr_info.bin"); //todo 不应该每次调用这个方法都读取一边 先写在这里，后面再改
    let attr_type = data.get_type_hash_u32();
    let noun = data.get_noun_hash_u32();
    // 修改的内容为隐式属性
    if let Some((noun_pos,offset)) = check_b_implicit_data(&pdms_database_info.noun_attr_info_map, attr_type as i32, noun as i32) {
        return match data.data {
            BoolType(value) => {
                let mut r = modify_bool_implicit_data(&page.implicit_data, offset, data.data.bool_value().unwrap());
                new_data.append(&mut r.to_be_bytes().to_vec());
                Some(DataPage {
                    implicit_data: new_data,
                    children: page.children,
                    explicit_data: page.explicit_data,
                }.convert_new_data_page())
            }
            _ => {
                let data = data.convert_implicit_data_to_vec(true);
                let len = data.len();
                let origin_len = page.implicit_data.len();
                new_data = page.implicit_data[..noun_pos + 4].to_vec();
                new_data = [new_data, data].concat();
                if noun_pos + len < origin_len {
                    new_data = [new_data, page.implicit_data[len + noun_pos + 4..].to_vec()].concat(); // +4是因为 data前面还有个 007
                }
                let path = r"data_implicit_page.txt";
                fs::write(path, DataPage {
                    implicit_data: new_data.clone(),
                    children: page.children.clone(),
                    explicit_data: page.explicit_data.clone(),
                }.convert_new_data_page());
                Some(DataPage {
                    implicit_data: new_data,
                    children: page.children,
                    explicit_data: page.explicit_data,
                }.convert_new_data_page())
            }
        }
        // 修改的内容为显示属性
    } else {
        if let Some(pos) = find_iter(&page.explicit_data, &noun.to_be_bytes()[..]).next() {
            let data = data.convert_explicit_data_to_vec(true);
            let new_version = (version - 4).to_be_bytes()[..4].to_vec(); // 大版本 - 4
            page.implicit_data.splice(0x1C..0x20, new_version);
            // 未修改的属性直接复制到new_data中
            new_data = page.explicit_data[..pos].to_vec();
            new_data = [new_data, data].concat();
            let attr_len = (u16::from_be_bytes(page.explicit_data[pos + 6..pos + 8].try_into().unwrap()) * 4) as usize;
            // 修改的属性后面还有未改变的值，也直接复制过来
            if pos + 8 + attr_len < *&page.explicit_data.len() {
                new_data = [new_data, page.explicit_data[pos + 8 + attr_len..].to_vec()].concat();
            }
        }
        let len = (*&new_data.len() as u16 / 4).to_be_bytes();
        new_data.splice(2..4, len); // 修改显示属性 01 后的长度

        Some(DataPage {
            implicit_data: page.implicit_data,
            children: page.children,
            explicit_data: new_data,
        }.convert_new_data_page())
    }
}

#[inline]
fn check_b_implicit_data(map: &DashMap<i32, DashMap<i32, AttrInfo>>, attr_type: i32, noun_hash: i32) -> Option<(usize,u32)> {
    if let Some(info_map) = map.get(&attr_type) {
        // //dbg!(&info_map.value());
        if let Some(info) = info_map.get(&noun_hash) {
            if info.offset != 0 {
                return Some(((info.offset as usize) * 4,info.offset));
            }
        }
    }
    None
}

/// 获取该节点的 children 或者 显示属性
pub fn get_origin_children_and_explicit_data(input: &[u8], mut pos: usize) -> (Vec<u8>, Vec<u8>) {
    let mut children_data = vec![];
    let mut explicit_data = vec![];

    if &input[pos..pos + 2] == &[0x0, 0x2] {
        let data_len = (parse_to_u16(&input[pos + 2..pos + 4]) * 4) as usize;
        children_data = input[pos..pos + data_len].to_vec();
        pos = pos + 4 + data_len;
    }

    if &input[pos..pos + 2] == &[0x0, 0x1] {
        let data_len = (parse_to_u16(&input[pos + 2..pos + 4]) * 4) as usize;
        explicit_data = input[pos..pos + data_len].to_vec();
    }
    (children_data, explicit_data)
}

/// 修改第一个refno + version page 的版本号
pub fn convert_first_version_page(input: &[u8], refno: &[u8], version: u32) -> Option<Vec<u8>> {
    let version_start = &[0x0u8, 0x0, 0x0, 0x5, 0x0, 0xCC, 0x47, 0xDF, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x2];

    let mut iter = rfind_iter(input, version_start);
    while let Some(pos) = iter.next() {
        let mut version_page = vec![0u8; 0x800];
        let mut version_data = input[pos..pos + 0x800].to_vec();
        if let Some(r_pos) = find_iter(&input[pos..pos + 0x800], refno).next() {
            let new_version = (version - 4).to_be_bytes()[..4].to_vec(); // 在大版本 +5 的基础上 -4
            version_data.splice(r_pos + 8..r_pos + 8 + 4, new_version);
            version_page.splice(0..0x800, version_data);
            return Some(version_page);
        }
    }
    None
}

/// 修改第二个 参考号 + 版本号
pub fn convert_second_version_page(input: &[u8], refno: &[u8], version: u32) -> Option<Vec<u8>> {
    let version_start = &[0x0u8, 0x0, 0x0, 0x5, 0x0, 0xCC, 0x47, 0xDF, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x2];

    let mut iter = rfind_iter(input, version_start);
    while let Some(pos) = iter.next() {
        let mut version_page = vec![0u8; 0x800];
        let mut version_data = input[pos..pos + 0x800].to_vec();
        if let Some(r_pos) = find_iter(&input[pos..pos + 0x800], refno).next() {
            let new_version = (version - 3).to_be_bytes()[..4].to_vec(); // 在大版本 +5 的基础上 -3
            version_data.splice(r_pos + 8..r_pos + 8 + 4, new_version);
            version_page.splice(0..0x800, version_data);
            return Some(version_page);
        }
    }
    None
}

pub async fn convert_change_times_page(input: &[u8], refno: RefU64, interface: &mut PdmsInterface) -> MResult<Option<(Vec<u8>, Vec<u8>)>> {
    let start = &[0x0u8, 0x0, 0x0, 0x5, 0x0, 0x74, 0x3F, 0x49, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x2];
    if let Some(pos) = rfind_iter(input, start).next() {
        if let Some((project_name, filename)) = interface.get_project_name_and_filename(refno).await? {
            if let Some(tree) = interface.get_tree(filename.clone(), project_name.clone()).await? {
                if let Some(tree_id) = interface.get_node_id(filename, refno, project_name.clone()).await? {
                    let mut node = tree.get(&tree_id).unwrap();
                    loop {
                        if let Some(p_node_id) = node.parent() {
                            let p_node = tree.get(p_node_id).unwrap();
                            node = p_node;
                            if PACKAGE_TYPE.contains_key(node.data().attr_type.as_str()) {
                                let refno = &node.data().refno.0.to_be_bytes()[..8];
                                let mut change_times_page = input[pos..pos + 0x800].to_vec();
                                if let Some(ref_pos) = find_iter(&input[pos..pos + 0x800], refno).next() {
                                    let change_times = &(parse_to_i32(&input[pos + ref_pos + 12..pos + ref_pos + 16]) + 1).to_be_bytes()[..4];
                                    change_times_page.splice(ref_pos + 12..ref_pos + 16, change_times.to_vec());
                                    return Ok(Some((change_times_page, change_times.to_vec())));
                                }
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

pub fn convert_conversation_page(input: &[u8], version: u32, change_times: &[u8]) -> Option<Vec<u8>> {
    let v = &[0x0u8, 0x0, 0x0, 0x2, 0x61, 0x64, 0x6D, 0x69, 0x6E, 0x0, 0x0, 0x0];

    let mut new_version = version.to_be_bytes()[..4].to_vec();
    let mut new_version_reduce_2 = (version - 2).to_be_bytes()[..4].to_vec();
    let mut new_version_reduce_1 = (version - 1).to_be_bytes()[..4].to_vec();
    let mut old_version = (version - 6).to_be_bytes()[..4].to_vec();

    if let Some(pos) = rfind_iter(input, &v).next() {
        let mut old = vec![0, 0, 0, 3];
        old.append(&mut old_version);
        let mut times = vec![0, 0, 0, 1];
        times.append(&mut change_times.to_vec());
        let mut new = vec![0xFF, 0xFF, 0xFF, 0xFF];
        new.append(&mut new_version);
        let mut new_2 = vec![0, 0, 0, 1];
        new_2.append(&mut new_version_reduce_2);
        let mut new_1 = vec![0, 0, 0, 1];
        new_1.append(&mut new_version_reduce_1);

        let mut remain_data = input[pos - 80..pos - 80 + 0x7D8].to_vec();
        let l = remain_data.len();
        let new_page = (parse_to_i32(&remain_data[l - 16..l - 12]) + 1).to_be_bytes()[..4].to_vec();
        remain_data.splice(l - 16..l - 12, new_page);

        return Some([old, times, new, new_2, new_1, remain_data].concat());
    }
    None
}
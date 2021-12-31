use std::collections::HashSet;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use dashmap::DashMap;
use futures::TryFutureExt;
use memchr::memmem::rfind_iter;
use nom::IResult;
use nom::number::complete::{be_f64, be_i16, be_i32, be_u16, be_u32, be_u8};
use nom::sequence::tuple;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use phf::phf_map;
use crate::db_tool;
use crate::db_tool::{db1_dehash, decode_chars_data};
use crate::parse_explict_tools::{get_explicit_attr_type, get_expression_attr, parse_expression_attr, parse_axis_explicit_value_00, parse_axis_explicit_value_40, parse_axis_explicit_value_ff, times_keep_f32_two_decimal_place};
use crate::pdms_types::*;
use crate::pdms_types::AttrVal::*;
use crate::EXPR_ATT_SET;

pub fn parse_db(path: &PathBuf, database_info: &PdmsDatabaseInfo, limited_cnt: u32, b_save_to_log: bool, print_refno_str: &str, target_refno_str: &str) -> DashMap<i32, Vec<ElementData>> {
    let mut all_ref_no=HashSet::new();
    let time_start = std::time::Instant::now();
    let mut file = File::open(path).unwrap();
    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf);
    let input = &buf[..];
    let time = time_start.elapsed();
    println!("read file {:?} finished in {:?}", path, time);
    let (refno_table_map, world_refno) = gen_ref_type_pos_table(input);
    let elapsed = time_start.elapsed();

    let noun_type_ele_data_map = DashMap::new();
    let attr_info_map = &database_info.noun_attr_info_map;
    let ele_order_map = DashMap::new();

    let mut root_refno = world_refno;
    if !target_refno_str.is_empty() {
        let target_refno = convert_string_to_ref(target_refno_str);
        root_refno = target_refno;
    }

    let mut pending_refnos = vec![root_refno];
    let mut count = 0;
    while !pending_refnos.is_empty() {
        let refno = pending_refnos.pop().unwrap();
        if !refno_table_map.contains_key(&refno) {
            continue;
        }
        let entry = &*refno_table_map.get(&refno).unwrap();
        let pos = entry.pos;
        let type_hash = entry.noun_hash;
        // 判断反序列话的DashMap中有无对应的type
        if !attr_info_map.contains_key(&type_hash) {
            continue;
        }
        let mut ele_data = ElementData::default();
        ele_data.ref_no = convert_ref_to_string(&refno);
        ele_data.noun_hash = type_hash;
        ele_data.noun_name = db1_dehash(type_hash as u32);
        let attr_info_map = &*attr_info_map.get(&type_hash).unwrap();
        let implicit_attr_len = i32::from_be_bytes(input[pos - 4..pos].try_into().unwrap()) as usize * 4;
        let start = pos - 4;
        // owner: position+12
        let (_, owner) = parse_attr_owner(&input[pos + 12..pos + 20]).unwrap();
        ele_data.owner = owner.clone();

        //隐藏属性得数据切片
        let implicit_data = &input[start..start + implicit_attr_len];

        let membs_pos = start + implicit_attr_len;
        let membs_data = &input[membs_pos..];
        let maybe_refno_0 = i32::from_be_bytes(membs_data[4..8].try_into().unwrap());
        let maybe_refno_1 = i32::from_be_bytes(membs_data[8..12].try_into().unwrap());
        let mut memb_bytes_len = 0;

        if maybe_refno_0 == refno.0 && maybe_refno_1 == refno.1 {
            if &membs_data[0..2] == [0x0, 0x2].as_slice() {
                memb_bytes_len = u16::from_be_bytes(membs_data[2..4].try_into().unwrap()) as usize * 4;
                let merged_data = get_merged_data(membs_data, &mut memb_bytes_len);
                let (_, children) = parse_attr_members(&merged_data).unwrap();
                for child in &children {
                    if !pending_refnos.contains(child) {
                        pending_refnos.push(*child);
                    }
                }
                ele_data.children = children.into_iter().map(|x| convert_ref_to_string(&x)).collect::<Vec<_>>();
                for i in 0..ele_data.children.len() {
                    ele_order_map.insert(ele_data.children[i].clone(), i as i32);
                }
            }
        }
        let explicit_start = start + implicit_attr_len + memb_bytes_len;
        let explicit_data = &input[explicit_start..];
        let maybe_refno_0 = i32::from_be_bytes(membs_data[4..8].try_into().unwrap());
        let maybe_refno_1 = i32::from_be_bytes(membs_data[8..12].try_into().unwrap());
        let mut explicit_bytes_len = 0;
        let implicit_len = implicit_data.len();
        let mut sorted_offs = sort_offsets(attr_info_map.clone());
        sorted_offs.retain(|x| *x < implicit_len as u32);
        for (_, attr_info) in attr_info_map.clone() {
            if attr_info.offset != 0 && implicit_len > (attr_info.offset & 0xFFFFF) as usize  {
                let mut k = attr_info.offset as usize;
                if attr_info.att_type == DbAttributeType::BOOL {
                    k &= 0xFFFFF;
                } else if attr_info.att_type == DbAttributeType::STRING {
                    k -= 1;  //长度在前面
                }
                k *= 4;  //dword => byte
                // if attr_info.name.as_str() == "RADI"  {
                //     dbg!("test");
                // }
                let mut data_len = implicit_len as i32 - k as i32;
                if data_len < 0 { break; }
                if let Some(mut j) = sorted_offs.iter().position(|&x| x == attr_info.offset){
                    let next_offset_idx = j as i32 + 1;
                    if  next_offset_idx < sorted_offs.len() as i32 {
                        data_len = data_len.min((sorted_offs[next_offset_idx as usize] as i32 - attr_info.offset as i32) * 4);
                    }
                }
                if implicit_len > k as usize {
                    let att_val = parse_implicit_attr_value(&implicit_data[k..], &attr_info, data_len, refno, pos: usize)
                        .unwrap().1;
                    ele_data.attr_data_map.entry(attr_info.name.clone())
                        .or_insert(att_val);
                }
            } else {
                // 给未出现的显式属性赋值
                if attr_info.name.as_str() == "PARA" {
                    ele_data.attr_data_map.entry(attr_info.name.clone()).or_insert(StringType("0".to_string()));
                } else if attr_info.name.as_str() == "PTCDI"{  //PTCD 默认值为Y
                    ele_data.attr_data_map.entry(attr_info.name.clone()).or_insert(StringType("Y".to_string()));
                }else {
                    ele_data.attr_data_map.entry(attr_info.name.clone())
                        .or_insert(attr_info.default_val.clone());
                }
            }
        }
        if maybe_refno_0 == refno.0 && maybe_refno_1 == refno.1 {
            if &explicit_data[0..2] == [0x0, 0x1].as_slice() {
                explicit_bytes_len = u16::from_be_bytes(explicit_data[2..4].try_into().unwrap()) as usize * 4;
                let merged_data = get_merged_data(explicit_data, &mut explicit_bytes_len);
                let (_, explicit_attr_map) = parse_explict_attrs(&merged_data, &attr_info_map, ele_data.attr_data_map.clone(), refno, explicit_start).unwrap();
                for (key, val) in explicit_attr_map {
                    ele_data.attr_data_map.insert(key, val);
                }
            }
        }

        //添加遗漏的属性
        ele_data.attr_data_map.insert("OWNER".to_string(), ElementType(owner));
        ele_data.attr_data_map.insert("REFNO".to_string(), StringType(ele_data.ref_no.clone()));
        ele_data.attr_data_map.insert("TYPE".to_string(), WordType(db_tool::db1_dehash(type_hash as u32)));
        if !print_refno_str.is_empty() && print_refno_str == ele_data.ref_no {
            println!("查看的Refno {}的位置：{:#4X}\n, 属性配置参数为：{:#4X?}\n, 结果为: {:#4X?}\n", print_refno_str, start, &attr_info_map, &ele_data);
        }
        if !all_ref_no.contains(&ele_data.ref_no) {
            all_ref_no.insert(ele_data.ref_no.clone());
            noun_type_ele_data_map.entry(type_hash).or_insert_with(Vec::new).push(ele_data);
            count += 1;
            if count >= limited_cnt {
                break;
            }
        }
    }
    noun_type_ele_data_map.iter_mut().for_each(|mut eles| {
        for mut ele in eles.iter_mut() {
            if ele_order_map.contains_key(&ele.ref_no) {
                ele.order = *ele_order_map.get(&ele.ref_no).unwrap();
            }
            let name_val = &*ele.attr_data_map.get("NAME").unwrap();
            match name_val {
                AttrVal::StringType(name) => {
                    if name.as_str() == "unset" || name.as_str() == "" || name.as_str() == " " {
                        ele.name = format!("{} {}", &ele.noun_name, ele.order)
                    } else {
                        ele.name = name.clone()
                    }
                }
                _ => {}
            }
        }
    });

    println!("解析db所耗时间: {:?}", elapsed);
    noun_type_ele_data_map
}

const ATT_LEVE: i32 = 0x9DB99;
const ATT_PTS: i32 = 0x85438;

/// 获取隐式属性
#[inline]
pub fn parse_implicit_attr_value<'a>(input: &'a [u8], attr_info: &'a AttrInfo, data_len : i32, ref_no: RefNoTuple, pos: usize) -> IResult<&'a [u8], AttrVal> {
    let mut val = AttrVal::InvalidType;
    use nom::bytes::complete::take;
    let b_expr = check_is_expr(attr_info.hash);
    // println!("{:#4X?}", input);
    if b_expr {
        // 隐式属性的所有StringType的offset都给原本的值-1，表达式默认是StringType，但是他不需要-1,所以这里slice的时候再+1
        match attr_info.att_type {
            DbAttributeType::STRING => {
                let (_, attrval) = convert_to_implicit_axis_string(&input[4..])?;
                val = attrval;
            }
            _ => {
                let (_, attrval) = convert_to_implicit_axis_string(input)?;
                val = attrval;
            }
        }
        // log::info!("隐式属性 ref_no={:?} position={:#04X?} val={:?}",ref_no,pos,r);
    } else {
        // 隐式属性LEVEL 需要做特殊处理 map给定的是IntegerType 但其实是Vec<Int>
        if attr_info.hash == ATT_LEVE || attr_info.hash == ATT_PTS {
            let (mut tmp_input, length) = be_i32(input)?;
            let mut result = vec![];
            let mut length = length as usize;
            while tmp_input.len() >= 4 && length > 0 {
                let (input, value) = be_i32(tmp_input)?;
                result.push(value);
                tmp_input = input;
                length -= 1;
            }
            val = AttrVal::IntArrayType(result);
        } else {
            match attr_info.att_type {
                DbAttributeType::INTEGER => {
                    let (_, r) = be_i32(input)?;
                    val = AttrVal::IntegerType(r);
                }
                DbAttributeType::DOUBLE => {
                    if data_len == 4{
                        let d = ((f32::from_be_bytes(input[0..4].try_into().unwrap()) * 100.0).round() / 100.0) as f64;
                        val = AttrVal::DoubleType(d);
                    }else if data_len >= 8{    //允许是最后一个offset的情况，导致长度可能>8
                        if let [a, b, c, d, e, f, g, h] = input[0..8] {
                            let d = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                            val = AttrVal::DoubleType(d);
                        }
                    }
                }
                DbAttributeType::BOOL => {
                    let o = (attr_info.offset >> 0x14) as usize;
                    let (_, r) = be_u32(input)?;
                    let result = r >> o & 1;
                    val = AttrVal::BoolType(result == 1);
                }
                DbAttributeType::STRING => {
                    let (_, str_len) = be_i32(input)?;
                    let str_len = str_len as usize;
                    if str_len < input.len() && input.len() > 4 && str_len > 4 {
                        let (decode_string, _b_chi) = decode_chars_data(&input[4..str_len]);
                        val = AttrVal::StringType(decode_string);
                    } else {
                        val = AttrVal::StringType("unset".to_string());
                        //log::error!("字符串解析出错，数据为：{:#4X?}, 属性为：{:#4X?}", input, &attr_info);
                    }
                }
                DbAttributeType::ELEMENT => {
                    let (_, (ref_0, ref_1)) = tuple((
                        be_i32,
                        be_i32,
                    ))(input)?;
                    val = AttrVal::ElementType(convert_ref_to_string(&(ref_0, ref_1)));
                }
                DbAttributeType::WORD => {
                    let (_, v) = be_i32(input)?;
                    if v > 0x171FAD39 {
                        let n = db1_dehash(v as u32);
                        val = AttrVal::WordType(n);
                    } else {
                        val = AttrVal::IntegerType(v);
                    }
                }
                DbAttributeType::DIRECTION | DbAttributeType::POSITION | DbAttributeType::ORIENTATION | DbAttributeType::Vec3Type => {
                    let mut data = [0f64; 3];
                    let (input, cnt) = be_i32(input)?;
                    let l = input;
                    if input.len() >= 3 && cnt == 3{
                        let b_double = (data_len - 1) / cnt /4  == 2;
                        if b_double {
                            for i in 0..cnt as usize {
                                if let [a, b, c, d, e, f, g, h] = l[i * 8..i * 8 + 8] {
                                    data[i] = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                                }
                            }
                        } else {
                            for i in 0..cnt as usize {
                                data[i] = ((f32::from_be_bytes(l[i * 4..i * 4 + 4].try_into().unwrap()) * 100.0).round() / 100.0) as f64;
                            }
                        }
                    }
                    val = AttrVal::Vec3Type(data);
                }
                DbAttributeType::DATETIME => {}
                _ => {}
            }
        }
    }
    Ok((input, val))
}

/// 获取已知显式属性
pub fn parse_explict_attrs<'a>(input: &'a [u8], attr_info_map: &'a DashMap<i32, AttrInfo>, implicit_data: DashMap<String, AttrVal>, refno: RefNoTuple, pos: usize) -> IResult<&'a [u8], DashMap<String, AttrVal>> {
    let mut explict_attrs = DashMap::new();
    let mut residual = input;
    let total_len = input.len();
    while residual.len() >= 8 {
        let debug_pos = total_len - residual.len();
        let hash_val = i32::from_be_bytes(residual[..4].try_into().unwrap());
        if check_is_expr(hash_val) {
            let (input, (expression_type, value)) = parse_expression_attr(residual).unwrap();
            explict_attrs.insert(expression_type, StringType(value));
            residual = input;
        } else {
            let (l, (explict_num, attr_type_num, type_len)) = tuple((
                be_i32,
                be_u16,//这个是属性的类型
                be_u16,//这个就是一个属性的长度
            ))(&residual[..])?;
            let type_len = type_len as usize;
            if type_len * 4 <= l.len() {
                residual = &l[type_len * 4..];
                // 显式属性有可能他给了type但是超了01 后面得长度 所以还要做一层判断
                let tmp_input = &l[..type_len * 4];
                if attr_info_map.contains_key(&explict_num) {
                    let mut attr_info = attr_info_map.get(&explict_num).unwrap().value().clone();
                    if attr_type_num == 0x1800 {
                        attr_info.att_type = DbAttributeType::DOUBLEVEC;
                    } else if attr_type_num == 0x1C00 || attr_type_num == 0x2000 {
                        attr_info.att_type = DbAttributeType::INTVEC;
                    } else if attr_type_num == 0x0C00 {
                        attr_info.att_type = DbAttributeType::WORD;
                    }
                    // DESP 特殊处理
                    if explict_num == 0xD20C7 {
                        attr_info.att_type = DbAttributeType::INTVEC;
                    }
                    // 根据获取到的type hash值，拿到需要的类型
                    match attr_info.att_type {
                        DbAttributeType::INTEGER => {
                            let (_, val) = be_i32(tmp_input)?;
                            explict_attrs.insert(attr_info.name.clone(), IntegerType(val));
                        }
                        DbAttributeType::DOUBLE => {
                            let dou_len = tmp_input.len() / 4;
                            if dou_len == 1 {
                                let (_, val) = be_i32(tmp_input)?;
                                explict_attrs.insert(attr_info.name.clone(), IntegerType(val));
                            } else {
                                if let [a, b, c, d, e, f, g, h] = tmp_input[..8] {
                                    let val = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                                    explict_attrs.insert(attr_info.name.clone(), DoubleType(val));
                                }
                            }
                        }
                        DbAttributeType::BOOL => {
                            let (_, val) = be_u32(tmp_input)?;
                            explict_attrs.insert(attr_info.name.clone(), BoolType(val != 0));
                        }
                        DbAttributeType::STRING => {
                            let (_, a) = be_u32(tmp_input)?;
                            let len_a = a as usize;
                            if tmp_input.len() > 4 {
                                let (decode_string, _b_chi) = decode_chars_data(&tmp_input[4..4 + len_a]);
                                // dbg!(&decode_string);
                                explict_attrs.insert(attr_info.name.clone(), AttrVal::StringType(decode_string));
                            } else {
                                println!("len_a={:#04X?}", len_a);
                                println!("error refno={:?}", refno);
                                println!("error 显示 input={:#04X?}", tmp_input);
                            }
                        }
                        DbAttributeType::ELEMENT => {
                            let (_, (ref_0, ref_1)) = tuple((
                                be_i32,
                                be_i32,
                            ))(tmp_input)?;
                            explict_attrs.insert(attr_info.name.clone(), ElementType(convert_ref_to_string(&(ref_0, ref_1))));
                            //explict_attrs.entry(attr_info.name.clone()).or_insert( ElementType((ref_0, ref_1)));
                        }
                        DbAttributeType::WORD => {
                            let (_, val) = be_i32(tmp_input)?;
                            if val >= 0x81BF1 {
                                let val_word = db1_dehash(val as u32);
                                explict_attrs.insert(attr_info.name.clone(), WordType(val_word));
                            } else {
                                explict_attrs.insert(attr_info.name.clone(), IntegerType(val));
                            }
                        }
                        DbAttributeType::DIRECTION | DbAttributeType::POSITION | DbAttributeType::ORIENTATION => {
                            let (l, v) = be_i32(tmp_input)?;
                            let _len = v as usize;
                            let mut data = [0f64; 3];
                            for i in 0..3 {
                                if let [a, b, c, d, e, f, g, h] = l[i * 8..i * 8 + 8] {
                                    // 保留两位精度
                                    data[i] = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                                }
                            }
                            explict_attrs.insert(attr_info.name.clone(), Vec3Type(data));
                        }
                        DbAttributeType::DATETIME => {}

                        DbAttributeType::DOUBLEVEC => {
                            let array_len = tmp_input.len() / 4;
                            let (tmp_input, data_len) = be_i32(tmp_input)?;
                            let len = data_len as usize;
                            let double_or_float = array_len / len;
                            let mut tmp_input = tmp_input;

                            if double_or_float == 2 {
                                if tmp_input.len() >= 8 {
                                    let mut data = vec![];
                                    for _ in 0..len {
                                        if let [a, b, c, d, e, f, g, h] = tmp_input[..8] {
                                            data.push((f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 100.0).round() / 100.0);
                                            tmp_input = &tmp_input[8..];
                                        }
                                    }
                                    explict_attrs.insert(attr_info.name.clone(), DoubleArrayType(data));
                                } else {
                                    explict_attrs.insert(attr_info.name.clone(), DoubleArrayType(vec![0.0]));
                                }
                            } else if double_or_float == 1 {
                                if tmp_input.len() > 4 {
                                    let mut data = vec![];
                                    for _ in 0..len {
                                        if let [a, b, c, d] = tmp_input[..4] {
                                            let val = f32::from_be_bytes([a, b, c, d]) as f64;
                                            data.push((val * 100.0).round() / 100.0);
                                            tmp_input = &tmp_input[4..];
                                        }
                                    }
                                    explict_attrs.insert(attr_info.name.clone(), DoubleArrayType(data));
                                } else {
                                    explict_attrs.insert(attr_info.name.clone(), DoubleArrayType(vec![0.0]));
                                }
                            }
                        }
                        DbAttributeType::INTVEC => {
                            let (tmp_input, len) = be_u32(tmp_input)?;
                            let len = len as usize;
                            let mut tmp_input = tmp_input;
                            let mut data = vec![];
                            for _ in 0..len {
                                let (remain_input, val) = be_i32(tmp_input)?;
                                data.push(val);
                                tmp_input = remain_input;
                            }
                            explict_attrs.insert(attr_info.name.clone(), IntArrayType(data));
                        }

                        _ => {}
                    }
                } else {
                    // 这里的逻辑改了一下，先判断是否为表达式，所以之前在这里的表达式判断就注释掉了
                    // 如果DashMap没有对应属性的hash 则调用get_explicit_attr_type进行解析
                    if let Some(attr_type) = get_explicit_attr_type(attr_type_num, debug_pos + pos) {
                        let attr_name = db1_dehash(explict_num as u32);
                        // 根据获取到的type hash值，拿到需要的类型
                        match attr_type {
                            DbAttributeType::INTEGER => {
                                let (_, val) = be_i32(tmp_input)?;
                                explict_attrs.insert(attr_name, IntegerType(val));
                            }
                            DbAttributeType::DOUBLE => {
                                if tmp_input.len() >= 8 {
                                    if let [a, b, c, d, e, f, g, h] = tmp_input[..8] {
                                        let val = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                                        explict_attrs.insert(attr_name, DoubleType(val));
                                    }
                                } else {
                                    let (_, val) = be_i32(tmp_input)?;
                                    explict_attrs.insert(attr_name, IntegerType(val));
                                }
                            }
                            DbAttributeType::BOOL => {
                                let (_, val) = be_u32(tmp_input)?;
                                explict_attrs.insert(attr_name, BoolType(val != 0));
                            }
                            DbAttributeType::STRING => {
                                let (_, a) = be_u32(tmp_input)?;
                                let len_a = a as usize;
                                if tmp_input.len() > 4 {
                                    let (decode_string, _b_chi) = decode_chars_data(&tmp_input[4..4 + len_a]);
                                    explict_attrs.insert(attr_name, AttrVal::StringType(decode_string));
                                } else {
                                    println!("len_a={:#04X?}", len_a);
                                    println!("error refno={:?}", refno);
                                    println!("error 显示 input={:#04X?}", tmp_input);
                                }
                            }
                            DbAttributeType::ELEMENT => {
                                let (_, (ref_0, ref_1)) = tuple((
                                    be_i32,
                                    be_i32,
                                ))(tmp_input)?;
                                explict_attrs.insert(attr_name, ElementType(convert_ref_to_string(&(ref_0, ref_1))));
                                //explict_attrs.entry(attr_info.name.clone()).or_insert( ElementType((ref_0, ref_1)));
                            }
                            DbAttributeType::WORD => {
                                let (_, val) = be_i32(tmp_input)?;
                                if val >= 0x81BF1 {
                                    let val_word = db1_dehash(val as u32);
                                    explict_attrs.insert(attr_name, WordType(val_word));
                                } else {
                                    explict_attrs.insert(attr_name, IntegerType(val));
                                }
                            }
                            DbAttributeType::DIRECTION | DbAttributeType::POSITION | DbAttributeType::ORIENTATION => {
                                let (l, v) = be_i32(tmp_input)?;
                                let _len = v as usize;
                                let mut data = [0f64; 3];
                                for i in 0..3 {
                                    if let [a, b, c, d, e, f, g, h] = l[i * 8..i * 8 + 8] {
                                        // 保留两位精度
                                        data[i] = f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0;
                                    }
                                }
                                explict_attrs.insert(attr_name, Vec3Type(data));
                            }

                            DbAttributeType::DOUBLEVEC => {
                                let array_len = tmp_input.len() / 4;
                                let (mut tmp_input, data_len) = be_i32(tmp_input)?;
                                let len = data_len as usize;
                                let double_or_float = array_len / len;
                                // let mut tmp_input = tmp_input;
                                if double_or_float == 2 {
                                    let mut data = vec![];
                                    for _ in 0..len {
                                        if let [a, b, c, d, e, f, g, h] = tmp_input[..8] {
                                            data.push(f64::trunc(f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 10000.0) / 10000.0);
                                            tmp_input = &tmp_input[8..];
                                        }
                                    }
                                    explict_attrs.insert(attr_name, DoubleArrayType(data));
                                } else if double_or_float == 1 {
                                    let mut data = vec![];
                                    for _ in 0..len {
                                        if let [a, b, c, d] = tmp_input[..4] {
                                            let val = f32::from_be_bytes([a, b, c, d]) as f64;
                                            data.push(f64::trunc(val * 10000.0) / 10000.0);
                                            tmp_input = &tmp_input[4..];
                                        }
                                    }
                                    explict_attrs.insert(attr_name, DoubleArrayType(data));
                                }
                            }
                            DbAttributeType::INTVEC => {
                                let (tmp_input, len) = be_u32(tmp_input)?;
                                let len = len as usize;
                                let mut tmp_input = tmp_input;
                                let mut data = vec![];
                                for _ in 0..len {
                                    let (remain_input, val) = be_i32(tmp_input)?;
                                    data.push(val);
                                    tmp_input = remain_input;
                                }
                                explict_attrs.insert(attr_name, IntArrayType(data));
                            }
                            DbAttributeType::TYPEX => {
                                let (tmp_input, len) = be_u32(tmp_input)?;
                                if len == 1 {
                                    let (_, typex) = be_u32(&tmp_input[..4])?;
                                    let typex = db1_dehash(typex);
                                    explict_attrs.entry("TYPE".to_string()).or_insert(StringType(typex));
                                } else {
                                    println!("undefined TYPE len {} position={:#04X?}", len, pos);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                break;
            }
        }
    }
    Ok((input, explict_attrs))
}

/// 获取所有的members
pub fn parse_attr_members(input: &[u8]) -> IResult<&[u8], Vec<RefNoTuple>> {
    let mut members = vec![];
    let mut residual = input;
    while residual.len() > 4 {
        let (l, (ref0, ref1)) = tuple((
            be_i32,
            be_i32,
        ))(residual)?;
        residual = l;
        let ref_no = (ref0, ref1);
        members.push(ref_no);
    }
    Ok((input, members))
}

/// 获取该节点的owner
pub fn parse_attr_owner(input: &[u8]) -> IResult<&[u8], String> {
    let (_, (owner0, owner1)) = tuple((
        be_i32,
        be_i32,
    ))(input)?;
    let owner = convert_ref_to_string(&(owner0, owner1));
    Ok((input, owner))
}

#[inline]
pub fn trunc_f32_two(input:f32) -> f32 {
    ( input * 100.0 ).round() /100.0
}

/// 特殊处理AXIS隐式属性
pub fn convert_to_implicit_axis_string(input: &[u8]) -> IResult<&[u8], AttrVal> {
    // 目前都是以02开头，如果不是以02开头就记录下来
    let (tmp_input, signal) = be_u32(input)?;
    let mut val = AttrVal::StringType("".to_string());
    if signal == 2 {
        //这里改动了一下，给tmp_input截取了..8
        match &tmp_input[..8] {
            &[0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1] => { val = AttrVal::StringType("X".to_string()) }
            &[0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x2] => { val = AttrVal::StringType("Y".to_string()) }
            &[0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x3] => { val = AttrVal::StringType("Z".to_string()) }
            &[0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x1] => { val = AttrVal::StringType("-X".to_string()) }
            &[0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x2] => { val = AttrVal::StringType("-Y".to_string()) }
            &[0x0, 0x0, 0x0, 0x2, 0x0, 0x0, 0x0, 0x3] => { val = AttrVal::StringType("-Z".to_string()) }

            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x0] => { val = AttrVal::StringType("P0".to_string()) }
            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x1] => { val = AttrVal::StringType("P1".to_string()) }
            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x0, 0x2] => { val = AttrVal::StringType("P2".to_string()) }
            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x3, 0xE8] => { val = AttrVal::StringType("-P0".to_string()) }
            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x3, 0xE9] => { val = AttrVal::StringType("-P1".to_string()) }
            &[0x0, 0x0, 0x0, 0x3, 0x0, 0x0, 0x3, 0xEA] => { val = AttrVal::StringType("-P2".to_string()) }

            &_ => {
                match &tmp_input[..3] {
                    &[0xFF, 0xFF, 0xFF] => {
                        let (_, radius) = be_u32(&tmp_input[4..8])?;
                        let radius = radius / 100;
                        let mut result = String::new();
                        match &tmp_input[3..4] {
                            &[0xF4] => { result = format!("X {} Y", radius); }
                            &[0xD2] => { result = format!("-X {} -Z", radius); }
                            &[0xF0] => { result = format!("X {} -Z", radius); }
                            &[0xEB] => { result = format!("Y {} X", radius); }
                            &[0xE0] => { result = format!("Z {} Y", radius); }
                            &[0xE1] => { result = format!("Z {} X", radius); }
                            &_ => {}
                        }
                        val = AttrVal::StringType(result);
                    }
                    &_ => {}
                }
                match &tmp_input[..4] {
                    &[0x0, 0x0, 0x0, 0x3] => {
                        let (_, value) = be_u8(&tmp_input[7..8])?;
                        val = AttrVal::StringType(format!("P{}", value));
                        if value > 0xE8 {
                            let value = value - 0xE8;
                            val = AttrVal::StringType(format!("-P{}", value));
                        }
                    }
                    &[0x0, 0x0, 0x0, 0xC] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("X {} Y", value);
                        val = AttrVal::StringType(result);
                    }
                    &[0x0, 0x0, 0x0, 0xD] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("X {} Z", value);
                        val = AttrVal::StringType(result);
                    }
                    &[0x0, 0x0, 0x0, 0x15] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("Y {} X", value);
                        val = AttrVal::StringType(result);
                    }
                    &[0x0, 0x0, 0x0, 0x17] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("Y {} Z", value);
                        val = AttrVal::StringType(result);
                    }
                    &[0x0, 0x0, 0x0, 0x1F] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("Z {} X", value);
                        val = AttrVal::StringType(result);
                    }
                    &[0x0, 0x0, 0x0, 0x20] => {
                        let value = get_implicit_angle_expression(&tmp_input[4..8]);
                        let result = format!("Z {} Y", value);
                        val = AttrVal::StringType(result);
                    }
                    _ => {}
                }
            }
        }
    } else if signal == 4 {
        let mut val = String::new();
        // 目前的推论是 0x28代表符号部分 ，0x1代表数字部分
        match &tmp_input[..2] {
            &[0x0, 0x0] => {
                let (_, times) = be_i16(&tmp_input[2..4])?;
                let times = times_keep_f32_two_decimal_place(times as i32);
                if times == 1.0 {
                    val = "PARAM".to_string();
                } else if times == 0.0 {
                    val = "".to_string();
                } else {
                    val = format!("{} TIMES PARAM", times);
                }
            }
            &[0xFF, 0xFF] => {
                let (_, times) = be_i16(&tmp_input[2..4])?;
                if times > 0xFFFFu16 as i16 {
                    let mut times = ((0xFFFFu16 as i16) as f32 - times as f32 - 1.0) / 40.0f32;
                    times = (times * 100.0_f32).round() / 100.0;
                    val = format!("{} TIMES PARAM", times);
                } else {
                    let mut times = (times as f32 - (0xFFFFu16 as i16) as f32 - 1.0) / 40.0f32;
                    times = (times * 100.0_f32).round() / 100.0;
                    val = format!("{} TIMES PARAM", times);
                }
            }
            _ => {}
        }
        match &tmp_input[4..8] {
            &[0x0, 0x0, 0x0, 0x1] => {
                let (_, value) = be_i32(&tmp_input[8..12])?;
                if value >= 0x65 && value < 0x3E9 {
                    // PARAM 数值大于 0x65 就是 IPARAM
                    let value = value - 0x64;
                    val = format!("IPARAM {}", value);
                } else if value >= 0x3E9 {
                    let value = value - 0x3E8;
                    if value >= 0x65 {
                        let value = value - 0x64;
                        val = format!("- IPARAM {}", value);
                    } else {
                        val = format!("- {} {}", val, value);
                    }
                } else if value <= 0xFFFFFFFFu32 as i32 {
                    let (_, t) = be_i32(&tmp_input[..4])?;
                    // times是除以0x28的倍数
                    if t == 0x28 {
                        val = get_implicit_angle_expression(&tmp_input[8..12]);
                    } else {
                        let mut value="".to_string();
                        let v = t as f32 / 40.0;
                        let times= trunc_f32_two(v);
                        value = get_implicit_angle_expression(&tmp_input[8..12]);
                        if value == "" {
                            let v=i32::from_be_bytes(tmp_input[8..12].try_into().unwrap());
                            if v!=0 {
                                value = (-v as f32 / 10.0 ).to_string();
                            }
                        }
                        val = format!("{} TIMES {}", times, value);
                    }
                } else {
                    val = format!("{} {}", val, value);
                }
            }
            &[0x0, 0x0, 0x0, 0x2] => {
                let (_, value) = be_i32(&tmp_input[8..12])?;
                if value > 1000 {
                    let value = value - 1000;
                    match &tmp_input[12..16] {
                        &[0xFF, 0xFF, 0xFF, 0xFB] => {
                            val = format!("TANF - {} {} DDHEIGHT", val, value);
                        }
                        &[0xFF, 0xFF, 0xFF, 0xFC] => {
                            val = format!("TANF - {} {} DDANGLE", val, value);
                        }
                        _ => {}
                    }
                } else {
                    match &tmp_input[12..16] {
                        &[0xFF, 0xFF, 0xFF, 0xFB] => {
                            val = format!("TANF {} {} DDHEIGHT", val, value);
                        }
                        &[0xFF, 0xFF, 0xFF, 0xFC] => {
                            val = format!("TANF {} {} DDANGLE", val, value);
                        }
                        _ => {}
                    }
                }
            }
            &[0x0, 0x0, 0x0, 0x3] => {
                let (_, times) = be_i32(&tmp_input[..4])?;
                let times = times_keep_f32_two_decimal_place(times);
                let (_, (value1, value2)) = tuple((
                    be_i32,
                    be_i32,
                ))(&tmp_input[8..16])?;
                let mut result = String::from("PARAM");
                if value1 >= 0x65 && value1 < 0x1F5 {
                    let value1 = f32::trunc(((value1 - 0x64) as f32 + 0.005) * 100.0).ceil() / 100.0;
                    val = format!("IPARAM {}", value1);
                } else if value1 >= 0x1F5 {
                    let value1 = value1 - 0x1F4;
                    val = format!("TWICE PATAM {}", value1);
                } else if value1 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value1) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    val = value.to_string();
                } else {
                    val = format!("PARAM {}", value1);
                }
                if value2 >= 0x65 && value2 < 0x1F5 {
                    let value2 = value2 - 0x64;
                    result = format!("IPARAM {}", value2);
                } else if value2 >= 0x1F5 {
                    let value2 = value2 - 0x1F4;
                    result = format!("TWICE PATAM {}", value2);
                } else if value2 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value2) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    result = value.to_string();
                } else {
                    result = format!("PARAM {}", value2);
                }
                if times != 1.0 {
                    val = format!("{} TIMES DIFFERENCE {} {}", times, val, result);
                } else {
                    val = format!("DIFFERENCE {} {}", val, result);
                }
            }

            &[0x0, 0x0, 0x0, 0x4] => {
                let (_, times) = be_i32(&tmp_input[..4])?;
                let times = times_keep_f32_two_decimal_place(times);
                let (_, (value1, value2)) = tuple((
                    be_i32,
                    be_i32,
                ))(&tmp_input[8..16])?;
                let mut result = String::from("PARAM");
                if value1 >= 0x65 && value1 < 0x1F5 {
                    let value1 = value1 - 0x64;
                    val = format!("IPARAM {}", value1);
                } else if value1 >= 0x1F5 {
                    let value1 = value1 - 0x1F4;
                    val = format!("TWICE PATAM {}", value1);
                } else if value1 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value1) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    val = value.to_string();
                } else {
                    val = format!("PARAM {}", value1);
                }
                if value2 >= 0x65 {
                    let value2 = value2 - 0x64;
                    result = format!("IPARAM {}", value2);
                } else if value2 >= 0x1F5 {
                    let value2 = value2 - 0x1F4;
                    val = format!("TWICE PATAM {}", value2);
                } else if value2 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value2) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    result = value.to_string();
                } else {
                    result = format!("PARAM {}", value2);
                }
                if times != 1.0 {
                    val = format!("{} TIMES SUM {} {}", times, val, result);
                } else {
                    val = format!("SUM {} {}", val, result);
                }
            }
            &[0x0, 0x0, 0x0, 0x7] => {
                return Ok((input, StringType(String::new())));
            }
            &[0x0, 0x0, 0x0, 0x8] => {
                let (_, times) = be_i32(&tmp_input[..4])?;
                let times = times_keep_f32_two_decimal_place(times);
                let (_, (value1, value2)) = tuple((
                    be_i32,
                    be_i32,
                ))(&tmp_input[8..16])?;
                let mut result = String::from("PARAM");
                if value1 >= 0x65 {
                    let value1 = value1 - 0x64;
                    val = format!("IPARAM {}", value1);
                } else if value1 >= 0x1F5 && value1 < 0x1F5 {
                    let value1 = value1 - 0x1F4;
                    val = format!("TWICE PATAM {}", value1);
                } else if value1 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value1) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    val = value.to_string();
                } else {
                    val = format!("PARAM {}", value1);
                }
                if value2 >= 0x65 && value2 < 0x1F5 {
                    let value2 = value2 - 0x64;
                    result = format!("IPARAM {}", value2);
                } else if value2 >= 0x1F5 {
                    let value2 = value2 - 0x1F4;
                    result = format!("TWICE PATAM {}", value2);
                } else if value2 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value2) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    result = value.to_string();
                } else {
                    result = format!("PARAM {}", value2);
                }
                if times != 1.0 {
                    val = format!("{} TIMES SUM {} {}", times, val, result);
                } else {
                    val = format!("MULT {} {}", val, result);
                }
            }
            &[0x0, 0x0, 0x0, 0x9] => {
                let (_, times) = be_i32(&tmp_input[..4])?;
                let times = times_keep_f32_two_decimal_place(times);
                let (_, (value1, value2)) = tuple((
                    be_i32,
                    be_i32,
                ))(&tmp_input[8..16])?;
                let mut result = String::from("PARAM");
                if value1 >= 0x65 && value1 < 0x1F5 {
                    let value1 = value1 - 0x64;
                    val = format!("IPARAM {}", value1);
                } else if value1 >= 0x1F5 {
                    let value1 = value1 - 0x1F4;
                    val = format!("TWICE PATAM {}", value2);
                } else if value1 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value1) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    val = value.to_string();
                } else {
                    val = format!("PARAM {}", value1);
                }
                if value2 >= 0x65 && value2 < 0x1F5 {
                    let value2 = value2 - 0x64;
                    result = format!("IPARAM {}", value2);
                } else if value2 >= 0x1F5 {
                    let value2 = value2 - 0x1F4;
                    result = format!("TWICE PATAM {}", value2);
                } else if value2 <= 0xFFFFFFFFu32 as i32 {
                    let value = f32::trunc(((0xFFFFFFFFu32 as i32 - value2) as f32 / 0xA as f32 + 0.1) * 10.0) / 10.0;
                    result = value.to_string();
                } else {
                    result = format!("PARAM {}", value2);
                }
                if times != 1.0 {
                    val = format!("{} TIMES SUM {} {}", times, val, result);
                } else {
                    val = format!("DIV {} {}", val, result);
                }
            }

            _ => {}
        }
        if tmp_input.len() > 24 {
            let value = get_implicit_angle_expression(&tmp_input[16..20]);
            val = format!("{} {}", val, value);
        }
        return Ok((input, StringType(val)));
    }
    Ok((input, val))
}

/// 特殊处理AXIS显式属性
pub fn convert_to_explicit_axis_string(input: &[u8]) -> IResult<&[u8], AttrVal> {
    let mut result = AttrVal::StringType("".to_string());
    if input.len() < 20 {
        let (_, val) = convert_to_implicit_axis_string(input)?;
        result = val;
    } else {
        // 检测是否以 1A 1A 05 02 17 开头
        let (tmp_input, (a, b, c, d, e)) = tuple((
            be_u32,
            be_u32,
            be_u32,
            be_u32,
            be_u32,
        ))(input)?;
        if [a, b, c, d, e] == [0x1A, 0x1A, 0x5, 0x2, 0x17] {
            match &tmp_input[24..26] {
                &[0x0, 0x0] => {
                    let (tmp_input, first) = be_u32(tmp_input)?;
                    let first = match_explicit_attribute_to_string(first);

                    let data = &tmp_input[16..28];
                    let (_, first_data) = parse_axis_explicit_value_00(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (tmp_input, second) = be_u32(tmp_input)?;
                    let second = match_explicit_attribute_to_string(second);

                    let data = &tmp_input[16..28];
                    let (_, second_data) = parse_axis_explicit_value_00(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (_tmp_input, third) = be_u32(tmp_input)?;
                    let third = match_explicit_attribute_to_string(third);
                    let combine_result = format!("{} ( {} ) {} ( {} ) {}", first, first_data, second, second_data, third);
                    result = AttrVal::StringType(combine_result);
                }
                &[0x40, 0x0] => {
                    let (tmp_input, first) = be_u32(tmp_input)?;
                    let first = match_explicit_attribute_to_string(first);

                    let data = &tmp_input[16..28];
                    let (_, first_data) = parse_axis_explicit_value_40(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (tmp_input, second) = be_u32(tmp_input)?;
                    let second = match_explicit_attribute_to_string(second);

                    let data = &tmp_input[16..28];
                    let (_, second_data) = parse_axis_explicit_value_40(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (_tmp_input, third) = be_u32(tmp_input)?;
                    let third = match_explicit_attribute_to_string(third);
                    let combine_result = format!("{} ( {} ) {} ( {} ) {}", first, first_data, second, second_data, third);
                    result = AttrVal::StringType(combine_result);
                }
                &[0xFF, 0xFF] => {
                    let (tmp_input, first) = be_u32(tmp_input)?;
                    let first = match_explicit_attribute_to_string(first);

                    let data = &tmp_input[16..28];
                    let (_, first_data) = parse_axis_explicit_value_ff(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (tmp_input, second) = be_u32(tmp_input)?;
                    let second = match_explicit_attribute_to_string(second);

                    let data = &tmp_input[16..28];
                    let (_, second_data) = parse_axis_explicit_value_ff(data)?;

                    let tmp_input = &tmp_input[36..];
                    let (_tmp_input, third) = be_u32(tmp_input)?;
                    let third = match_explicit_attribute_to_string(third);
                    let combine_result = format!("{} ( {} ) {} ( {} ) {}", first, first_data, second, second_data, third);
                    result = AttrVal::StringType(combine_result);
                }
                _ => {}
            }
        } else {
            match &tmp_input[..8] {
                &[0x0, 0x0, 0x0, 0xB, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("X".to_string()) }
                &[0x0, 0x0, 0x0, 0xC, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("-X".to_string()) }
                &[0x0, 0x0, 0x0, 0xD, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("Y".to_string()) }
                &[0x0, 0x0, 0x0, 0xE, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("-Y".to_string()) }
                &[0x0, 0x0, 0x0, 0xF, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("Z".to_string()) }
                &[0x0, 0x0, 0x0, 0x10, 0x0, 0x0, 0x0, 0x3D] => { result = StringType("-Z".to_string()) }
                _ => {}
            }
        }
    }
    Ok((input, result))
}

/// match AXIS显式属性对应的值
#[inline]
pub fn match_explicit_attribute_to_string(key: u32) -> String {
    match key {
        0x10 => { "-Z".to_string() }
        0xF => { "Z".to_string() }
        0xE => { "-Y".to_string() }
        0xD => { "Y".to_string() }
        0xC => { "-X".to_string() }
        0xB => { "X".to_string() }
        _ => {
            " ".to_string()
        }
    }
}

/// 检查是否是Axis属性
#[inline]
pub fn check_is_expr(input: i32) -> bool {
    if EXPR_ATT_SET.contains(&input) {
        true
    } else {
        false
    }
}

/// 获取所有的DB对应的参考号、name和numberDb
pub fn get_sys_db_ref_no(db_eles_data_map: &DashMap<i32, Vec<ElementData>>) -> DashMap<String, (AttrVal, String)> {
    let mut result = DashMap::new();
    if let Some(db_data_map) = db_eles_data_map.get(&0x81C2B) {
        for ele in db_data_map.value() {
            if let Some(number_db) = ele.attr_data_map.get("NUMBDB") {
                let value = number_db.value();
                result.entry(ele.ref_no.clone()).or_insert((value.clone(), ele.name.clone()));
            }
        }
    }
    result
}

/// 隐式表达式解析，给一个字符串返回DDHEIGHT这种表达式
#[inline]
pub fn get_implicit_angle_expression(input: &[u8]) -> String {
    let mut val = String::new();
    match input {
        &[0xFF, 0xFF, 0xFF, 0xFB] => {
            val = "DDHEIGHT".to_string();
        }
        &[0xFF, 0xFF, 0xFF, 0xFC] => {
            val = "DDANGLE".to_string();
        }
        &[0xFF, 0xFF, 0xFF, 0xFD] => {
            val = "DDRADIUS".to_string();
        }
        _ => { }
    }
    val
}

#[inline]
pub fn is_desi_noun(bytes: &[u8]) -> bool {
    bytes == [0x0, 0xB, 0x6, 0x92].as_slice()
}

#[inline]
pub fn is_cata_noun(bytes: &[u8]) -> bool {
    bytes == [0x0, 0x8, 0xA1, 0xE6].as_slice()
}

///保存 noun_hash->refno  map
pub fn save_type_hash_file(dir: &str, out_name: &str) -> core::result::Result<(), Box<dyn std::error::Error>> {
    let mut unique_hash_refno_map = DashMap::new();
    let mut path_buf = fs::read_dir(dir)?.into_iter().map(|entry| {
        let entry = entry.unwrap();
        entry.path()
    }).collect::<Vec<PathBuf>>();
    path_buf.sort_by(|a, b| fs::metadata(b).unwrap().len().partial_cmp(&fs::metadata(a).unwrap().len()).unwrap());

    for path in path_buf {
        dbg!(&path);
        let mut file = File::open(&path).unwrap();
        let mut buf = vec![0u8; 36];
        file.read_exact(&mut buf)?;
        let _input = &buf[32..36];
        //if is_cata_noun(input) || is_desi_noun(input) {
        let start = Instant::now();
        println!("path={:?}", path);
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf);
        let input = &buf[..];
        let time = start.elapsed();
        println!("read {:?} finished in {:?}", path, time);
        let (_input, _) = process_type_hash(input, &mut unique_hash_refno_map, &path).unwrap_or_default();
        println!("noun_hash_refnos len = {:?}", unique_hash_refno_map.len());
        let encode = bincode::serialize(&unique_hash_refno_map).unwrap();
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(out_name)
            .unwrap();
        file.write(&encode);
        //}
    }
    Ok(())
}

pub fn process_type_hash<'a>(input: &'a [u8], type_hash: &'a mut DashMap<i32, (RefNoTuple, String)>, path: &PathBuf) -> IResult<&'a [u8], ()> {
    let refno_0_set = get_total_refno_0s(input);
    let path = Path::new(path);
    let file_name = path.file_name().unwrap().to_owned().to_string_lossy().to_string();
    refno_0_set.par_iter().for_each(|ref_0| {
        let pos_iter = rfind_iter(&input, ref_0);
        for p in pos_iter {
            let (_, refno_entry) = get_refno_entry(input, p).unwrap_or_default();
            type_hash.entry(refno_entry.1.noun_hash).or_insert((refno_entry.0, file_name.clone()));
        }
    });
    Ok((input, ()))
}

///获取所有不同的 refno_0
pub fn get_total_refno_0s(input: &[u8]) -> HashSet<&[u8]> {
    let mut refno_0_set = HashSet::new();
    let mut pos_iter = rfind_iter(&input, [0x00, 0xCC, 0x47, 0xDF, 0x00, 0x00, 0x00, 0x00].as_slice());
    while let Some(i) = pos_iter.next() {
        let mut j = i + 0x6 * 4;   //偏移6 dword
        let mut d = &input[j..j + 4];
        while d != [0, 0, 0, 0].as_slice() {
            refno_0_set.insert(d);
            j += 0x4 * 4;
            d = &input[j..j + 4];
        }
    }
    //dbg!(&refno_0_set);
    refno_0_set
}

#[inline]
fn get_refno_entry(input: &[u8], offset: usize) -> IResult<&[u8], (RefNoTuple, EleDataEntry)> {
    let (_, ref_type) = be_i32(&input[offset + 8..offset + 12])?;
    let mut refno_entry = ((0, 0), EleDataEntry::default());
    if NOUN_TYPES_MAP.contains_key(&ref_type) {
        let (_, (refno_0, refno_1, noun_hash)) = tuple((
            be_i32,
            be_i32,
            be_i32, //type hash
        ))(&input[offset..offset + 12])?;
        refno_entry = ((refno_0, refno_1), EleDataEntry {
            pos: offset as usize,
            noun_hash,
        });
    }
    Ok((input, refno_entry))
}

/// map中将所有offset不为0的值进行排序
pub fn sort_offsets(map: DashMap<i32, AttrInfo>) -> Vec<u32> {
    let mut offsets = vec![];
    for (_, v) in map {
        if v.offset != 0 {
            offsets.push(v.offset);
        }
    }
    offsets.sort_by(|a, b| a.cmp(b));
    offsets
}

///通过offset获取某个隐式属性的长度
pub fn get_implicit_len_by_offset(count: &Vec<u32>, offset: u32) -> usize {
    if let Some(index) = count.iter().position(|o| *o == offset) {
        return (count[index + 1] - count[index]) as usize;
    }
    0
}

#[derive(Default, Debug)]
pub struct EleDataEntry {
    pub pos: usize,
    pub noun_hash: i32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DbInfo {
    pub numb_db: AttrVal,
    pub db_name: String,
}

/// 获取 ref_no + type 的索引位置表  和 world的参考号
/// 根据get_last_index_position返回的hashset获取所有的ref_no + type的位置
/// 返回值是hashmap k:所有的ref_no v:(ref_no的position,type的hash)
pub fn gen_ref_type_pos_table(input: &[u8]) -> (DashMap<RefNoTuple, EleDataEntry>, RefNoTuple) {
    let refno_0_set = get_total_refno_0s(input);
    let mut world_refno = Arc::new(Mutex::new((0i32, 0)));
    let mut refno_table = DashMap::new();
    refno_0_set.par_iter().for_each(|ref_0| {
        let pos_iter = rfind_iter(&input, ref_0);
        let world_refno_clone = world_refno.clone();
        let mut w_refno = world_refno_clone.lock().unwrap();
        for p in pos_iter {
            let (_, refno_entry) = get_refno_entry(input, p).unwrap_or_default();
            //check if world element
            if w_refno.0 == 0 && refno_entry.1.noun_hash == 0xBEB83 {
                w_refno.0 = refno_entry.0.0;
                w_refno.1 = refno_entry.0.1;
            }
            //取得 ref_0
            if refno_entry.0.0 != 0 {
                refno_table.entry(refno_entry.0).or_insert(refno_entry.1);
            }
        }
    });
    let lock = Arc::try_unwrap(world_refno).expect("Lock still has multiple owners");
    (refno_table, lock.into_inner().expect("Mutex cannot be locked"))
}

#[inline]
fn convert_ref_to_string(refno: &RefNoTuple) -> String {
    format!("{}/{}", refno.0, refno.1)
}

#[inline]
fn convert_string_to_ref(refno: &str) -> RefNoTuple {
    let x: Vec<i32> = refno.split('/').map(|x| x.parse::<i32>().unwrap_or_default()).collect();
    (x[0], x[1])
}

fn get_merged_data(input: &[u8], len: &mut usize) -> Vec<u8> {
    let mut data = input[20..*len].to_vec();
    if *len + 4 > input.len() {
        return data;
    }
    let mut tmp_offset = *len;
    while &input[tmp_offset..tmp_offset + 4] == &[0x0, 0x0, 0x0, 0x7] {
        // println!("{:#4X?}", input[tmp_offset..tmp_offset + 4].to_vec());
        let seg_len = u16::from_be_bytes(input[tmp_offset + 6..tmp_offset + 8].try_into().unwrap()) as usize * 4;
        let mut seg_offset = tmp_offset + 16;
        let mut i = 0;
        while &input[seg_offset..seg_offset + 4] == &[0x0, 0x0, 0x0, 0x0] {
            seg_offset += 4;
            i += 1;
            if i == 2 { break; }   //暂时最多允许2个0的DWORD
        }
        let next_seg = &input[seg_offset..tmp_offset + seg_len + 4];
        data.extend_from_slice(next_seg);
        tmp_offset += seg_len + 4;
        // println!("fn get_merged_data: {:#4X?}", input[tmp_offset..tmp_offset + 4].to_vec());
    }
    *len = tmp_offset;
    data
}

static NOUN_TYPES_MAP: phf::Map<i32, &'static str> = phf_map! {

0xCC12Di32 => "SPLOAD",
0xE2C6Bi32 => "UDET",
0xA9A9F4Fi32 => "CLNTIL",
0xCD82Bi32 => "SRTORUS",
0x89E42i32 => "PTRACK",
0x1CC376Fi32 => "WTHTAB",
0xE2873i32 => "DUCTING",
0xB15DBi32 => "BOXING",
0xE55C0i32 => "RRST",
0xC10C9i32 => "GRDMODEL",
0x8D80Ai32 => "LALB",
0x11A118D0i32 => "HPRNOT",
0xAB9A3i32 => "SDSH",
0xC7A33i32 => "TRNN",
0x10169B35i32 => "FMEXTR",
0xBD94Bi32 => "CELL",
0xB9FFCi32 => "TASK",
0xBF8C8i32 => "RFWL",
0xE40D2i32 => "FILTER",
0x4FE43C1i32 => "TABQUESTION",
0x1672C1i32 => "TATTA",
0x9F801i32 => "UDEFINITION",
0xF7C27i32 => "ABOX",
0x7DBE404i32 => "HYSBDI",
0xD115C20i32 => "SUPNFO",
0x9D572i32 => "CATEGORY",
0x14BD9A20i32 => "TESTEXPRESSION",
0xB551474i32 => "ASITEM",
0xAE47FF7i32 => "CYMWRL",
0x112C974Bi32 => "EXTDAT",
0x11865E5Ci32 => "STRFLT",
0x12EFAF34i32 => "VMSUBV",
0x9076Di32 => "TRACING",
0x4E1225i32 => "GLYPH",
0x1CC21C9i32 => "PDATAB",
0x149355Ei32 => "HICPLA",
0x54E18Bi32 => "GRPLI",
0x17B3F44i32 => "CTSTRA",
0xE551Ei32 => "RLST",
0xB0F2Bi32 => "REVISION",
0xC0E7676i32 => "GIPPAN",
0xBE3AEi32 => "PVOLUME",
0x9E2963Ai32 => "HYTANK",
0x11E75A8Di32 => "TOPEXT",
0xB3F26i32 => "PALJOINT",
0x4EDDA5Ci32 => "HRKPSE",
0x787A326i32 => "MRESTH",
0x410426Ei32 => "MTPGSD",
0x97BC1i32 => "SNODE",
0x4C0DB04i32 => "WLPANEL",
0xB070E47i32 => "GLOCWL",
0xCB14440i32 => "LDRRUN",
0x11C16D9Fi32 => "SCINSTRUMENT",
0x9CA1Fi32 => "TAPER",
0x95743i32 => "SSBDOCU",
0xB9FEFi32 => "GASKET",
0x34F774i32 => "SPINE",
0x83E57i32 => "DBL",
0xF563Ei32 => "PTAXIS",
0xA7D77F0i32 => "HOLDFL",
0xF9BA5CCi32 => "TRUSER",
0x1220244Ci32 => "CTREDU",
0x2C3704i32 => "AWELD",
0xAF3C0i32 => "SOLID",
0x8499Fi32 => "CAP",
0x3208DE3i32 => "SPMSPC",
0x88CC0i32 => "PPLANE",
0xF8ECEF6i32 => "RLADDR",
0xAAD58D2i32 => "STWALL",
0xC1D97i32 => "RDIMENSION",
0x10A52720i32 => "REVLKS",
0xDC369DDi32 => "COLMAP",
0x6E2493i32 => "HRSOL",
0xCC0A81Ci32 => "DSXOWN",
0x47CD642i32 => "CSCREED",
0xF7A4B26i32 => "SETPARAMETER",
0x88B61i32 => "PCLAMP",
0x2B0F8Ci32 => "OBJHD",
0x13713FE8i32 => "GFCURV",
0xD2786i32 => "COUPLING",
0x1C9F20Di32 => "MPTLAB",
0x9D299i32 => "CASE",
0xC5FECi32 => "PLENUM",
0x9C469i32 => "PANEL",
0xAB2ADi32 => "SSPHERE",
0x11C801E4i32 => "BPFITTING",
0xCF4FEFBi32 => "HYDACO",
0xBEACDi32 => "CIRLIST",
0x11BABD8i32 => "SPMZFA",
0xE4CFDi32 => "PPPT",
0xB3C6351i32 => "HYCSBM",
0x8BAB0i32 => "DTABLE",
0x1CC4AB8i32 => "SNOTAB",
0xAC63DCEi32 => "ATTCOLUMN",
0xAE474i32 => "REGISTRY",
0x3974CE8i32 => "LSWIDD",
0x8F3A6i32 => "FTUBE",
0xE2D3Di32 => "OLET",
0xE556Di32 => "POST",
0xBFE23i32 => "LCYLINDER",
0xBE488i32 => "RCPL",
0xCA761i32 => "COCO",
0xAAD58C2i32 => "CTWALL",
0x11C17783i32 => "MPLNST",
0xC839D3Di32 => "GBRAPN",
0xC87C8i32 => "NLSNOUT",
0xCB2D2i32 => "POGON",
0xACBF2D3i32 => "RUTVOLUME",
0x67A1421i32 => "STRLNG",
0xAF28Di32 => "IDLIST",
0xE71A4i32 => "CMBU",
0x3DC3FB8i32 => "HBLWLD",
0x12A0B565i32 => "ASTATU",
0xDA7D6i32 => "SPLR",
0x98D7E13i32 => "GBLOCK",
0xEC7A4i32 => "NREVOLUTION",
0xB04D3i32 => "PORI",
0xA783DA0i32 => "CPANEL",
0xA94F5EAi32 => "SLRAIL",
0x2C9E50i32 => "LCOMD",
0xDB1DBi32 => "SCPROPERTY",
0xF76A635i32 => "APPDAREA",
0xF9AAE0Di32 => "TROPERATION",
0x1821637i32 => "SPMPSA",
0xE5461i32 => "RESTRAINT",
0x9C1FEi32 => "REMENTRY",
0x4E55936i32 => "SCCORE",
0x8738Bi32 => "PTCAR",
0x28E713i32 => "HBEAD",
0xC89B3i32 => "SCTN",
0x1049D9Ai32 => "SYSCDA",
0xC3F08i32 => "TXTM",
0xC0E5171i32 => "GICPAN",
0x90B48i32 => "HACC",
0xBBA1DA3i32 => "HRTERM",
0x674709i32 => "FMWSK",
0xAC7B826i32 => "HPRHOL",
0x553BD39i32 => "MBRDEF",
0x12898F6Ci32 => "ASREQU",
0x8A1E7i32 => "DATA",
0xE518Ci32 => "VERTEX",
0xB0C33ADi32 => "DBSTWL",
0x1E9E0Bi32 => "GLYTB",
0x25292Ei32 => "ASSOC",
0xE768Di32 => "REDUCER",
0x10154095i32 => "STRSTR",
0xE2963i32 => "ACDT",
0xDEAE5i32 => "NDISH",
0x89EC9i32 => "PYRAMID",
0x11C80096i32 => "SCFITTING",
0x14984104i32 => "GRIDAXIS",
0x4E7730i32 => "HYFRH",
0x977E4i32 => "BEND",
0xAF299CAi32 => "POLPTLIST",
0xA967CD1i32 => "ATTFILTER",
0x9E5AD64i32 => "STALNK",
0x13245E26i32 => "ARCHIV",
0x4F1699Ci32 => "HRGATE",
0xBF1CFi32 => "OUTLINE",
0x10E3D433i32 => "TRMESSAGE",
0x4B1B247i32 => "SRCELEMENT",
0xF4336i32 => "DBVW",
0x98D7E19i32 => "MBLOCK",
0x140209i32 => "MRPLA",
0x1C9E715i32 => "MTPLAB",
0xAD37022i32 => "PBSTPL",
0xC7DC96Di32 => "REGION",
0xC63FC31i32 => "GRIDLN",
0x8A30Ei32 => "BLTABLE",
0x4EDE56Fi32 => "HOOPSE",
0xD224Bi32 => "NSSPHERE",
0x106513D9i32 => "HYLOCS",
0x852D116i32 => "RESTRIC",
0x4975D8Ci32 => "POLYHEDRON",
0x11E75F0Ei32 => "LDREXT",
0x8738Ei32 => "STCATEGORY",
0x112C8F97i32 => "DERDAT",
0x10152770i32 => "HYISTR",
0xAC7B821i32 => "CPRHOL",
0x54ED8Fi32 => "EXTLI",
0xE22B5i32 => "STATUS",
0xD70673Ei32 => "TABGROUP",
0xC1D86i32 => "ADIMENSION",
0xCEE9Bi32 => "LOAPOINT",
0xCF8F40Ei32 => "TRINCOMMAND",
0xCA78Ci32 => "SPCOMPONENT",
0xA783DA5i32 => "HPANEL",
0xFE7B70Fi32 => "CABCORE",
0x346DE1i32 => "HHOLE",
0xE4B8C99i32 => "SYSGRP",
0x4B6A987i32 => "ACRULE",
0x3DC2AE1i32 => "STDWLD",
0xDFCC8i32 => "CLOSURE",
0x55D6111i32 => "HSTIFF",
0xF60C0i32 => "FLEXIBLE",
0x7F4725i32 => "MBURN",
0xC2EE419i32 => "BPOPEN",
0x8A21Ci32 => "CCTABLE",
0xB0658A3i32 => "AREAWLD",
0xE579Ai32 => "FITTING",
0x88CC3i32 => "SPLANE",
0xFA8B6i32 => "NSCYLINDER",
0xDF12C20i32 => "REVCGP",
0x7FC770i32 => "XCLTN",
0x3DC38F4i32 => "DSIWLD",
0x4FCC2B2i32 => "VVALUE",
0x3D60DEEi32 => "REVBLD",
0xAC78CB2i32 => "HICHOL",
0x34481Ai32 => "CABLE",
0xD943Ai32 => "USER",
0xF139Ci32 => "VIEW",
0xAAD2A4i32 => "GSTAT",
0xE4011i32 => "BBOLT",
0x8A1E6i32 => "CATALOGUE",
0x112CBBE7i32 => "HTFEAT",
0x5C7CD3Ai32 => "HYBMSF",
0x3DC4A19i32 => "SSOWLD",
0xEC08A23i32 => "SEGSEQ",
0x6D0B2Ai32 => "CWALL",
0x112CBB1Ei32 => "WLFEAT",
0x11977344i32 => "IJOINT",
0x4C1A264i32 => "FMEDNE",
0xC67825Ei32 => "FMBPLN",
0xE5599EAi32 => "OPENSPACE",
0xC2E93i32 => "SCOMPONENT",
0x84B6Ei32 => "GRP",
0xD2099i32 => "LCSPHERICAL",
0x9D1B1i32 => "NSREVOLUTION",
0x112CBBF1i32 => "RTFEAT",
0x4E947F2i32 => "AREASET",
0x8DA99i32 => "SYLB",
0x117D59D1i32 => "HBRCKT",
0x9E3B8i32 => "LAYER",
0xE21DAi32 => "PLATE",
0x1655E79i32 => "HYWAPA",
0x557440i32 => "GENNI",
0x161383i32 => "SPMSA",
0xF2AF3i32 => "COMW",
0xC7C2A77i32 => "FMWCON",
0x9AB88i32 => "SHEET",
0x66F4662i32 => "INSCMG",
0xA1217i32 => "CINF",
0x2C72F59i32 => "GLYRECT",
0x7EC3536i32 => "HVACFITTING",
0xE4B9A25i32 => "DSXGRP",
0x55D6110i32 => "GSTIFF",
0xD122Ei32 => "TANPOINT",
0x320775i32 => "RNODE",
0xE4C9Fi32 => "CMPTYPE",
0xAD375E5i32 => "FCUTPLANE",
0x6711797i32 => "EXTIMG",
0xCEF26i32 => "PTAPPING",
0xAF4BEi32 => "CYLINDER",
0xF96F243i32 => "LADDER",
0xD1679i32 => "LOOP",
0xDBEEFi32 => "SSTRESS",
0xE4628i32 => "VENT",
0x2237278i32 => "DRTMLB",
0x9D99006i32 => "DRSYLK",
0xC6B50i32 => "PLINE",
0xE4CF1i32 => "DPPT",
0x9BF25i32 => "RELEASE",
0x84F85i32 => "ACR",
0xDB0BDi32 => "CTORUS",
0x10BC8026i32 => "SCOINSTRUMENT",
0x320766i32 => "CNODE",
0x3DC5614i32 => "HYSWLD",
0x2C4233i32 => "BUILDING",
0xAFBC1i32 => "PJOINT",
0xCEE6Ci32 => "SMAP",
0xBFA36i32 => "FONTWORLD",
0xCEEF4i32 => "TRAP",
0x4E1EA6Ci32 => "MNRCRE",
0xE358ADEi32 => "POLOOP",
0xEC09AC8i32 => "NAMSEQ",
0xE05B1i32 => "PORSET",
0x9C802i32 => "SHOE",
0x9D42Fi32 => "DPSET",
0xB6EF78i32 => "SPBOU",
0x886AB6i32 => "AREVOLUTION",
0x9D65Ai32 => "SITE",
0x982DDi32 => "CARD",
0x5538EBCi32 => "STADEF",
0xCC10Di32 => "NOLOAD",
0xE26D1i32 => "RECTANGLE",
0xDF63EA8i32 => "RESTGP",
0x10843670i32 => "HYCKGS",
0x142EE321i32 => "WINDOW",
0x11C801E8i32 => "FPFITTING",
0x3DC50D4i32 => "NBRWLD",
0x112C72E6i32 => "CCHDAT",
0x3E695Ei32 => "CSURF",
0x112C96DBi32 => "ATTDAT",
0xE55A534i32 => "POINSP",
0xEBEADBi32 => "SPMBAA",
0xE554Bi32 => "INSTRUMENT",
0xBE598i32 => "TMPLATE",
0x676C46Bi32 => "DBRANG",
0x2CDA24i32 => "SCIND",
0x4F70D29i32 => "HPATTERN",
0xCBFE9i32 => "SDLOAD",
0x3DC3E08i32 => "HMKWLD",
0xDB38Ei32 => "VSPR",
0x3E6F451i32 => "FMBEND",
0xC274Fi32 => "VOLMODEL",
0x9D4A7i32 => "PTSET",
0x4F01D0i32 => "RPATH",
0xD1203E1i32 => "GSUPFO",
0x8221Ci32 => "MDB",
0x380A7B2i32 => "HYLOAD",
0xB0AE071i32 => "SYGPWL",
0x4EB2F47i32 => "RUNGSET",
0xDFD41i32 => "PPOS",
0xE629Fi32 => "SEXTRUSION",
0x60F9892i32 => "SCDIAGRAM",
0x9D5D3i32 => "SDTEXT",
0x6E1904i32 => "SPOOL",
0x926DAi32 => "SSLCYLINDER",
0x88CB6i32 => "FPLANE",
0xFEDE3FEi32 => "SEQWOR",
0xAFC67i32 => "TPOISSON",
0xB1C6B97i32 => "LBSTYL",
0x88B64i32 => "SCLAMP",
0xF30B122i32 => "MNRNSQ",
0xB711E3i32 => "ASNOUT",
0x11C4BACEi32 => "HYHYST",
0xD8914i32 => "BVAREA",
0xD9546i32 => "SBFRAMEWORK",
0x3DC4A18i32 => "RSOWLD",
0xE49F4i32 => "VNOTE",
0xC2E94i32 => "TCOMPONENT",
0x17F623Ci32 => "SPMGSA",
0x4E232C1i32 => "HYPDRE",
0x17660DBi32 => "HANDRA",
0xAD910i32 => "RECIPIENT",
0xE55B6i32 => "HRST",
0x3DC63ABi32 => "DSXWLD",
0x3E72234i32 => "HPREND",
0x856061i32 => "HIBLO",
0x1575B697i32 => "INSLAY",
0x128A46B2i32 => "TABHQUESTION",
0x25DA268i32 => "SPMRSB",
0x8DC790i32 => "STAMP",
0xF37EAi32 => "ACRW",
0xC6BA1i32 => "POINT",
0x468C7ABi32 => "PPIECE",
0xA7B2280i32 => "STRWELL",
0xC551Ci32 => "BRANCH",
0x862B8E1i32 => "HBRSTI",
0x10140C8Di32 => "CPROTR",
0xCDC7Di32 => "REVOLUTION",
0x117776Ei32 => "SPMLFA",
0xAFC66i32 => "SPOISSON",
0x3D79E3i32 => "MPROF",
0x10B33F76i32 => "AITEMS",
0x2C358Fi32 => "FIELD",
0xAE398C7i32 => "CTMTRL",
0x3778A4i32 => "CURVE",
0x11C1D0B8i32 => "HYPOST",
0xC7C26D2i32 => "REVCON",
0x9D13Ai32 => "CORE",
0x82AC9i32 => "TEE",
0xCF0C10Ei32 => "CPANBO",
0x17E2EB0i32 => "SPMCSA",
0xC5F17i32 => "SDENSITY",
0xF7C34i32 => "NBOX",
0x334936Ei32 => "LNDESC",
0xE3803i32 => "SFITTING",
0xB054Fi32 => "ETRIANGLE",
0x12A08721i32 => "JLDATUM",
0xE355969i32 => "REVNOP",
0xDCCD4i32 => "LPYRAMID",
0xF8ECEF7i32 => "SLADDR",
0x4F3D045i32 => "ENGITE",
0x11C8017Bi32 => "ELFITTING",
0x3D9C8D2i32 => "LNFOLD",
0x13E497i32 => "HIFLA",
0x8703Ai32 => "DPBA",
0x4F177A8i32 => "MPLATE",
0xB0932i32 => "ACTI",
0xCD152i32 => "UGROUP",
0xAFB3BD6i32 => "CRERULES",
0x8219495i32 => "HYTRLI",
0x10188151i32 => "GENCUR",
0xFD22CC0i32 => "HPILLR",
0x256660i32 => "GENPC",
0x9E5E79Bi32 => "REVLNK",
0xDB051i32 => "CPORT",
0xA79A155i32 => "TMRRELEMENT",
0x54F991i32 => "ACYLINDER",
0xBC174i32 => "BVCL",
0xE4B7B73i32 => "CYMGRP",
0xE52FEi32 => "NSRTORUS",
0xA5A3570i32 => "STAVAL",
0x10AA32ADi32 => "MTPBLS",
0xAD1596Bi32 => "SHTMPL",
0xB3F95i32 => "SELJOINT",
0x912EEi32 => "VSECTION",
0xFA85Bi32 => "DPCYLINDRICAL",
0xBF45Ai32 => "RRULE",
0x81DB5i32 => "TP",
0x3DC32BCi32 => "ENGWLD",
0xAB9B8i32 => "MESH",
0x144A2A3Ei32 => "CPINRW",
0x986AAC5i32 => "FMSSBK",
0xF3A28E9i32 => "MRESTQ",
0x81F4Bi32 => "UDA",
0x3D6F83Ci32 => "FMWELD",
0xD2F14i32 => "TEXTPRIMITIVE",
0x11C21AC2i32 => "HYOPST",
0xE97FEi32 => "TYOUNG",
0x1014D202i32 => "HYFRTR",
0x6FE3111i32 => "HNOTCH",
0x17DE1CDi32 => "SPMBSA",
0x45D624i32 => "GENPG",
0xDFD48i32 => "WPOS",
0x13D81D37i32 => "APPLDWORLD",
0xB725Ai32 => "BACKINGSHEET",
0x24825Fi32 => "LCOMC",
0x34F708i32 => "SLINE",
0xE5479i32 => "OFST",
0xBFE2Ai32 => "SCYLINDER",
0xEC37BEi32 => "SPMCAA",
0xAF254i32 => "FBLIND",
0x54BC3Ai32 => "LOCLI",
0xAC0189i32 => "DBSET",
0x87313i32 => "DPCARTESIAN",
0xD0B67i32 => "MARKPRIMITIVE",
0xB1488i32 => "NBOXING",
0xC1F0Fi32 => "PRIM",
0x57503Ci32 => "HISTI",
0xA0699F8i32 => "CPRMRK",
0xAD08F4Fi32 => "KICKPL",
0x1093817Bi32 => "STAHIS",
0xEC0FB43i32 => "HYSTEQ",
0xB1C6CB8i32 => "DMSTYL",
0x14116FAEi32 => "STLNKW",
0xD9485i32 => "OVERLAY",
0x116CF9A5i32 => "HYCCIT",
0x121E3833i32 => "HYGZCU",
0x4C0CFFCi32 => "GPLANE",
0x11EF67i32 => "HISEA",
0xCFA2299i32 => "HYGRCO",
0x9D2EBi32 => "DDSE",
0x11CE72B5i32 => "HPRCUT",
0xFA248i32 => "OLAYER",
0xFA4CDi32 => "LIBY",
0xAD15A6Ai32 => "DRTMPL",
0xE55F4i32 => "PTST",
0x9EC031i32 => "FLOOR",
0x1465E7Ai32 => "HBRFLA",
0xF7A283Di32 => "SYGPAR",
0xF9D398Ai32 => "VLAYER",
0xBD8F3i32 => "WALL",
0x10AF7620i32 => "HYCTLS",
0x1074F4F5i32 => "OLINESTYLE",
0xA708F98i32 => "FEMODL",
0x1575B385i32 => "FLRLAY",
0xF01F45i32 => "SPMPAA",
0x9CAF3i32 => "PIPE",
0x121BC5B7i32 => "HYCRCU",
0x4EDEDD0i32 => "TMRPSET",
0x16052A61i32 => "GRIDSYSTEM",
0x4B1AB18i32 => "PDAELE",
0x127D1E20i32 => "SCGROUP",
0x3E6962i32 => "GSURF",
0x90736i32 => "SPACER",
0x128E1317i32 => "MNSTQU",
0x907CDi32 => "HVAC",
0x13C18B5Ei32 => "MPLRAW",
0x98D7E0Ei32 => "BBLOCK",
0x97C22i32 => "HROD",
0xEEEBB9i32 => "SPMLAA",
0x3DC5847i32 => "DSTWLD",
0xE4B6936i32 => "ENGGRP",
0x9B855i32 => "BVIEW",
0xBD063i32 => "RAIL",
0x1CC4262i32 => "RPLTAB",
0x1495703i32 => "HDOPLA",
0x3C080FAi32 => "UVALID",
0xDB1A5i32 => "SAPROPERTY",
0xEA001i32 => "STRUCTURE",
0x67B3B61i32 => "CLNPNGRID",
0xAF361i32 => "ELLIPSE",
0xA099AAAi32 => "MNRWRK",
0xA1E48i32 => "SPRFILE",
0x856060i32 => "GIBLO",
0x9D499i32 => "BTSET",
0x222D41Ei32 => "TASKLB",
0xE56BEi32 => "BATTERY",
0xE55B1i32 => "CRST",
0xC164D5Bi32 => "PBSOBN",
0x9E5D007i32 => "CYMLNK",
0xD8874i32 => "DPAREA",
0xDB1A6i32 => "TAPROPERTY",
0xE4723i32 => "CONTYPE",
0xDBEF0i32 => "TSTRESS",
0x553C068i32 => "RESDEF",
0x97418i32 => "BWLD",
0x506E2DAi32 => "HCURVE",
0x6980B4Bi32 => "SPLDRG",
0x9BF1Bi32 => "HELEMENT",
0x84618i32 => "RUNDECK",
0xE4B8D47i32 => "DETGRP",
0xC4E0E13i32 => "CEILIN",
0xBF71168i32 => "DESSYMBOL",
0x11C0E0D3i32 => "TRMLST",
0x2B7CDB9i32 => "TRSUCCESS",
0x553D10Ci32 => "LAYDEF",
0xF2829i32 => "ROLWL",
0x97247i32 => "WELD",
0x11BE9919i32 => "DSXDST",
0xD330BBi32 => "TRDAY",
0xC4E0DEDi32 => "SCILINE",
0xF3A8Fi32 => "CASWORLD",
0x4EF2467i32 => "POSTSE",
0x11757BE9i32 => "MWLDJT",
0x707ACAi32 => "GTMWL",
0xB6F462i32 => "HIDOU",
0x3F1FC65i32 => "REVNOD",
0xCF94A2Di32 => "HYLOCO",
0xE53E6i32 => "CASTYPE",
0x112C6D79i32 => "REFDAT",
0x1CC4A82i32 => "SLOTAB",
0xC40CCi32 => "MNUM",
0xEC09F10i32 => "CONSEQ",
0xDF1A3i32 => "LNKSET",
0xC2FD6i32 => "ROOM",
0x85897i32 => "AHU",
0x175DA10i32 => "GSTBRA",
0xCD546i32 => "GRSO",
0xED6B4Ai32 => "SPMGAA",
0xD245Bi32 => "BLTP",
0xD0F45i32 => "DAMPER",
0x912EDi32 => "USECTION",
0x9D2EAi32 => "CDSE",
0xDF525i32 => "STLS",
0xE3800i32 => "PFITTING",
0xBEA29i32 => "ACRL",
0xF3D72i32 => "MATWORLD",
0xCF0C113i32 => "HPANBO",
0x9A831i32 => "ADDENTRY",
0x9A45Di32 => "TUBE",
0xED9D0i32 => "VALVE",
0xA4676i32 => "RSEG",
0xDB1C0i32 => "SBPROPERTY",
0xA7E5BE0i32 => "MPKGFL",
0xDB1DCi32 => "TCPROPERTY",
0x6E044Di32 => "HIHOL",
0x11519566i32 => "VLYSET",
0xE62A0i32 => "TEXT",
0xF770D02i32 => "TRYEAR",
0x119773E4i32 => "GPOINT",
0x9D4AAi32 => "STSECTION",
0xAEF165Ei32 => "TSTDTL",
0x41C0889i32 => "REVSTD",
0xCFB1F59i32 => "TROUCOMMAND",
0x8AA62DFi32 => "MOGOBJ",
0xF7D2C1i32 => "HYCOBA",
0xDCCD6i32 => "NPYRAMID",
0x429676i32 => "SCSEGMENT",
0x366844i32 => "PCDSE",
0xF2AB23Bi32 => "INSURQ",
0xAF25Bi32 => "MBLIST",
0x8F311i32 => "SNUB",
0x13F282FDi32 => "TRMSGW",
0xCCA3480i32 => "PBSTXN",
0xE26D2i32 => "SECTION",
0xB696AA1i32 => "HYAGHM",
0xC3021BBi32 => "SCSTENCIL",
0x632DD4Ai32 => "ISOREGI",
0xAFC5Ci32 => "IPOINT",
0x112CBB17i32 => "PLFEAT",
0xAFB7308i32 => "LAYRUL",
0xBBCF760i32 => "HYFORM",
0x11B5802Ci32 => "MWPART",
0xB072184i32 => "REVCWL",
0xF89CB08i32 => "CPINCR",
0xE4B9458i32 => "DRVGRP",
0xF817EFDi32 => "ASSMBR",
0xB09D57Fi32 => "REVLWL",
0x11B62049i32 => "PBSCRT",
0x88CC2i32 => "RPLANE",
0xC505F71i32 => "COATING",
0xAB956i32 => "WASH",
0x2A30046i32 => "POLFACE",
0x180E2ABi32 => "SPMLSA",
0x10153EDEi32 => "LDRSTR",
0x32076Bi32 => "HNODE",
0x9545Fi32 => "HSADDLE",
0x553CA63i32 => "HSVDEF",
0xC64046Ci32 => "HOLDLN",
0xDF1E0F0i32 => "ASDFGP",
0xBFA40i32 => "PTWLD",
0xBFE25i32 => "NCYLINDER",
0x10EF433Ei32 => "LOOPTS",
0xB1C6BA7i32 => "ACSTYLE",
0x18AACECi32 => "SSBRTA",
0xEC03E23i32 => "CNGREQ",
0x8AC85i32 => "VTWAY",
0xAC9CCFAi32 => "HSPOOL",
0xE531Ei32 => "STRT",
0x67A1927i32 => "INTLNG",
0x171C3AD8i32 => "SCNOZZLE",
0x553C2C7i32 => "DATDEF",
0x1068E6C0i32 => "TREADSET",
0x1C1B38i32 => "ISOLB",
0x978BEi32 => "DIAMOND",
0x453C5Ei32 => "GENNG",
0x1288E44Bi32 => "MPLCQU",
0xBF44Fi32 => "GRULE",
0xCD696i32 => "SCTORUS",
0xBE197i32 => "UBOLT",
0xAF252i32 => "DBLI",
0x251537i32 => "TRLOCATION",
0xBBC76i32 => "TABLE",
0xA069FB4i32 => "MPTMRK",
0x3DC43EBi32 => "COMWLD",
0x1110544i32 => "SCAREA",
0xD21F0i32 => "DPSPHERICAL",
0xE0779i32 => "MESS",
0x4EBD447i32 => "CTRISE",
0x9D6F7i32 => "NOTE",
0xE2847i32 => "NSCTORUS",
0x8D937i32 => "PLLB",
0x11A8BE1Ai32 => "HYCMPT",
0x9ECF0Bi32 => "ARTORUS",
0xCED087Bi32 => "HBRABO",
0xDF4BFA2i32 => "ASSOGP",
0xBBAACC2i32 => "HYPGRM",
0x10E51FFFi32 => "REVISS",
0x10C72FA1i32 => "CTCROS",
0x51A03A5i32 => "DSLAYER",
0x2B2DDEi32 => "ATTHD",
0x5A90473i32 => "WLPROF",
0x48B9135i32 => "HYDMGE",
0xE5DFDF8i32 => "HYCOTP",
0xD8730i32 => "DDAREA",
0x4C0DB97i32 => "HRPANE",
0x182DB9Ci32 => "HYASSA",
0xB03332i32 => "ACRST",
0xD9489i32 => "SVERTEX",
0x4B1D777i32 => "HTPELE",
0x112CBB75i32 => "BPFEAT",
0x11C0DCF9i32 => "FILLSTYLE",
0xB70AA07i32 => "FMEDIM",
0xCF62D24i32 => "FACECODE",
0xBFFF5i32 => "STYLE",
0xA5A14i32 => "RPLGROUP",
0xBF9CBi32 => "GPWLD",
0x112CBBB1i32 => "HRFEAT",
0x11E765DFi32 => "BOTEXT",
0x11BFF717i32 => "PPLIST",
0x346DDCi32 => "CHOLE",
0x7D7EED2i32 => "SCOPCI",
0xAB41BEi32 => "RSECT",
0x7AE30EBi32 => "NPOLYHEDRON",
0x115155B8i32 => "ACCSET",
0x10AA3DA5i32 => "MPTBLS",
0x11A95002i32 => "SDAOPT",
0xE5570i32 => "SOST",
0x6261F2i32 => "BLOCK",
0x1EDBCAi32 => "SCTUBING",
0x4B1BD1Ai32 => "IMGELE",
0xB6EEB0i32 => "HIBOU",
0x81C2Bi32 => "DB",
0x8D9A5i32 => "RPLB",
0x861E0i32 => "BOX",
0x14D9A8D2i32 => "INCFIX",
0x46FA4D3i32 => "AREADEF",
0xAF3CCi32 => "DPLINE",
0x6793D58i32 => "FIXING",
0x267ECC7i32 => "LSTYTB",
0xAF0C5i32 => "LNKITEM",
0x1496D84i32 => "RAWPLA",
0x9D3E1i32 => "GMSET",
0x10173Di32 => "MNOZZLE",
0xEA3A3i32 => "DATUM",
0x9A821i32 => "LCDESCRIPTOR",
0xE29B5i32 => "BFDT",
0x6D08F4i32 => "DBALL",
0x25A54A7i32 => "SPMGSB",
0x1353AF77i32 => "FLRCOV",
0xB074AB6i32 => "GRIDWLD",
0xC7B76i32 => "SCONE",
0x66BB82i32 => "HMARK",
0x4EBC877i32 => "CPNISE",
0xC3F22i32 => "SYTM",
0xAF279i32 => "PCLIP",
0x71E452i32 => "CSEAM",
0xBFA21i32 => "LINESTYLEWORLD",
0x8628EE6i32 => "GICSTI",
0xCD826i32 => "NRTORUS",
0x3D71DA4i32 => "XPIFLD",
0x9577Ai32 => "TUBDATA",
0xA798Fi32 => "DRWG",
0xB0A2EAi32 => "HICUT",
0xCD243i32 => "SPROFILE",
0x11B5D534i32 => "SSSBRT",
0x251377i32 => "DBLOC",
0xE558Bi32 => "SPST",
0xB1C6BAAi32 => "DCSTYLE",
0xA5D81Ei32 => "XCELS",
0xF32023i32 => "SPMZAA",
0xD9805i32 => "TAGRULE",
0x4F3C0B8i32 => "TABITEM",
0x7AE30DEi32 => "APOLYHEDRON",
0x20F56Di32 => "HYSAC",
0x9BF92i32 => "SILENCER",
0x8D9DDi32 => "TRLB",
0x11BB180Ai32 => "SRFTRT",
0xB0D03i32 => "FLUID",
0xC4EF866i32 => "WLJOIN",
0xAFBC4i32 => "SJOINT",
0xC4E00C8i32 => "BNDLIN",
0x3D79D9i32 => "CPROF",
0xDB1C1i32 => "TBPROPERTY",
0xBA2BBF4i32 => "DSXHOM",
0xC77701Ai32 => "ELCONN",
0x97BBEi32 => "PNODE",
0x1013A5FBi32 => "POINTR",
0x39E2567i32 => "SCREED",
0xA783DA4i32 => "GPANEL",
0xC6BB4i32 => "HPIN",
0xE0911i32 => "PTSSET",
0xCC729i32 => "LSNOUT",
0xADCF3i32 => "NODISPLACEMENT",
0x10EE9AF5i32 => "WLJNTS",
0x11991FFEi32 => "HYCONT",
0x4659508i32 => "SCSUBEQUIPMENT",
0x10E070Ai32 => "TABHEADER",
0x1851715i32 => "SPMZSA",
0xAC9D7DCi32 => "MNTOOL",
0x6FDFE52i32 => "DSXSCH",
0x1199280Ei32 => "TTFONT",
0x71E457i32 => "HSEAM",
0xBF6EB88i32 => "AXESYMBOL",
0xC1D91i32 => "LDIMENSION",
0x43866F3i32 => "CNGFXD",
0xC53DEi32 => "HFAN",
0x4B1E2ADi32 => "PRTELE",
0xE9734i32 => "GROUP",
0x175DA11i32 => "HSTBRA",
0xC15B9EDi32 => "THUMBN",
0x112F4763i32 => "INSMAT",
0xF2B47i32 => "FRMWORK",
0x118AAFAi32 => "SPMPFA",
0xBF9ACi32 => "COWL",
0xF7C39i32 => "SBOX",
0x3DC53AFi32 => "PBSWLD",
0x1036ACi32 => "NOZZLE",
0xC83F477i32 => "HSUBPN",
0xA136Ai32 => "RUNFILE",
0x4104D66i32 => "MPTGSD",
0x9DCC9i32 => "SPVERT",
0xAF39265i32 => "TWRSTL",
0x4C0CFF8i32 => "CPLANE",
0xC2EE3C2i32 => "WLOPEN",
0x782E8AFi32 => "MPLCTH",
0x11C16F40i32 => "DSINST",
0x119773E5i32 => "HPOINT",
0xE74B4i32 => "DOCU",
0xFD41679i32 => "POSRLR",
0xF3348i32 => "CMPWORLD",
0x11C36749i32 => "DSXTST",
0xC06C6i32 => "IDAM",
0x88F3Bi32 => "CMMA",
0x9D395i32 => "LJSE",
0x3DC32DFi32 => "MOGWLD",
0xBF96Ai32 => "RLWLD",
0xC830033i32 => "HYPZON",
0x112EEC07i32 => "CLNLATTICE",
0xE4B58D0i32 => "STAGRP",
0x15CBD87Di32 => "ASMBLY",
0x10BD696Ci32 => "MAPLNS",
0xF6185i32 => "NSEXTRUSION",
0xC0E7F01i32 => "GISPAN",
0x133ACC5Ci32 => "SCVALV",
0xFA7FEi32 => "SLCYLINDER",
0xE4BC8i32 => "DEPT",
0xDF69Ai32 => "NGMSET",
0xFABFB88i32 => "UDETGR",
0x3E7222Fi32 => "CPREND",
0xAE98FBi32 => "PAINT",
0xCA7D8i32 => "NSCONE",
0x11BEC87Ci32 => "LINESTYLE",
0xCEF1Ei32 => "HTAP",
0x1CC51FFi32 => "SBRTAB",
0xA734Ai32 => "SLUG",
0xE97FDi32 => "SYOUNG",
0xFB75215i32 => "GLYCIRCLE",
0x788177Bi32 => "MNSTTH",
0xC2EE3BBi32 => "PLOPEN",
0x7CF1291i32 => "HPANBI",
0x98E58EDi32 => "HYGRCK",
0xA799064i32 => "COLRELATION",
0xAE264i32 => "CMFITTING",
0xC1D95i32 => "PDIMENSION",
0x8A3E5i32 => "ATTACHMENT",
0x8E9027i32 => "PEROP",
0x4ED4ED3i32 => "HRPNSE",
0x2C370Ci32 => "IWELD",
0x3BF275Bi32 => "ULOGID",
0xB24CBi32 => "SUBJOINT",
0xBF987i32 => "TEAMWORLD",
0xE35899Di32 => "SCLOOP",
0x9D6C6i32 => "SMTEXT",
0xB07D654i32 => "ASDFWL",
0x408310Ci32 => "PLTGRD",
0xFEB6EB4i32 => "CFLOOR",
0x2A31726i32 => "MPTFAC",
0xD163Ai32 => "CMOP",
0x11C42C6Ci32 => "HYLWST",
0x119773E0i32 => "CPOINT",
0x3DC41F4i32 => "MWLWLD",
0x83CCAi32 => "LNK",
0x4F177A3i32 => "HPLATE",
0x8A143i32 => "BVSAREA",
0xAFC65i32 => "RPOINT",
0xAC0306i32 => "GPSET",
0xC665E0i32 => "GROLWL",
0xE38DDi32 => "UNIT",
0xE2DF8i32 => "MSET",
0x3DC38B7i32 => "XPIWLD",
0x3DC5838i32 => "PRTWLD",
0x9CFBFi32 => "BAREA",
0x55D610Ci32 => "CSTIFF",
0xF0919i32 => "DRAWING",
0xBE18Fi32 => "MBOLT",
0xE21CC25i32 => "INSCMP",
0x8518F42i32 => "GENPRI",
0x4B1C482i32 => "OBJELE",
0x912D0i32 => "SRECTANGLE",
0x95779i32 => "SUBDOCU",
0x97E6Fi32 => "CMPDATA",
0x11515450i32 => "SPBSET",
0xFAD1253i32 => "DBVWGROUP",
0x9A45Ci32 => "SUBEQUIPMENT",
0x114D1Ei32 => "PIPCARTESIAN",
0xA75121Bi32 => "SPMCEL",
0xCC72Bi32 => "NSNOUT",
0x712865i32 => "HSTYLE",
0x3DC42E0i32 => "FEMWLD",
0xDFAA2i32 => "TRNS",
0x11996A88i32 => "ACCPNT",
0x9E4E565i32 => "LNLINK",
0xCF84D4Ei32 => "SCELCONNECTION",
0xE5AFCi32 => "HNUT",
0x8754D9i32 => "MBPRO",
0x871BCi32 => "LCCARTESIAN",
0x10720717i32 => "SCPDESTINATION",
0x18B3148i32 => "REVSTA",
0x6D0B2Ei32 => "GWALL",
0xCC3A5i32 => "CMMO",
0x267ECC1i32 => "FSTYTB",
0x2326298i32 => "NOMINB",
0x557CB70i32 => "HRDREF",
0xCD547i32 => "HRSO",
0x12A08727i32 => "PLDATUM",
0xEC6F7i32 => "CLEVIS",
0x28E8CFi32 => "TREAD",
0x10A5A602i32 => "STLNKS",
0x89E45i32 => "STRAIGHT",
0xB0AB506i32 => "ASSOWL",
0xA61F9EBi32 => "LAYTBL",
0x9129Ai32 => "SPECIFICATION",
0xE5561i32 => "DOST",
0x4F16904i32 => "RLGATE",
0x37D0EFFi32 => "SPMCAD",
0x4F01230i32 => "DBVWSET",
0xBEC59i32 => "UWRLD",
0xC941E3Ci32 => "MTPBRN",
0xD3E151i32 => "ASLCYLINDER",
0x9C5EDi32 => "ZONE",
0x487F263i32 => "RLCAGE",
0x25AA18Ai32 => "SPMHSB",
0x67101A8i32 => "FEMIMG",
0xF2E7Di32 => "RUNWORLD",
0x65A2406i32 => "CPINJG",
0x2C75C2Ci32 => "GENSEC",
0x98567i32 => "EYRD",
0x11A8DDF4i32 => "HCOMPT",
0x8628EE7i32 => "HICSTI",
0x9ECD76i32 => "ACTORUS",
0xC6B133i32 => "LCOMW",
0x8DD72i32 => "SYMBOL",
0x11700A47i32 => "ULIMIT",
0xE4CEEi32 => "APPT",
0xB0D22E1i32 => "DBVWWLD",
0xCA439i32 => "ELBOW",
0xBF450i32 => "HRULE",
0x1147690i32 => "SPBPFA",
0x6331179i32 => "CAGSEG",
0x9C53Di32 => "LINE",
0xD0731A1i32 => "EXTGEO",
0x10B85F4i32 => "HYSZDA",
0xBBA6A1Bi32 => "INTFRM",
0x8BAB8i32 => "LTABLE",
0xAF435i32 => "ATLIST",
0xEA22Ei32 => "INSULATION",
0x1141BA87i32 => "SCDUCT",
0x11C8018Di32 => "WLFITTING",
0xAD15A85i32 => "DSTMPL",
0x18DB92i32 => "BRTAB",
0x11CE72B0i32 => "CPRCUT",
0xE4CFFi32 => "RPPT",
0x14D9C561i32 => "COMFIXING",
0x4B6A98Ai32 => "DCRULE",
0x483C414i32 => "OPENFE",
0x9D08Ei32 => "THREEWAY",
0x1431DBF9i32 => "UNKNOWN",
0x54D2E7i32 => "LNKLI",
0xF2DCCi32 => "CONWORLD",
0xFCCFEi32 => "NLPYRAMID",
0xBF9D8i32 => "TPWLD",
0xFA6FADCi32 => "CLNCGR",
0x9C033i32 => "ROLE",
0x9554Di32 => "CABDATA",
0x18B2B97i32 => "SETSTATUS",
0x1C728CAi32 => "SCMCABLE",
0xDEBB1i32 => "BLIST",
0xAF38Bi32 => "TMLI",
0x3F49D4i32 => "INSUF",
0xCCB67i32 => "REPORT",
0xA0694BCi32 => "MTPMRK",
0x2C3710i32 => "MWELD",
0xAC9CD02i32 => "PSPOOL",
0xA783D9Fi32 => "BPANEL",
0xBD223i32 => "GRILLE",
0x1151824Ci32 => "STRSET",
0x3DC42ACi32 => "HCMWLD",
0x4D8542Di32 => "PRTYPE",
0xBBA6B25i32 => "EXTFRM",
0xBF979i32 => "FMWLD",
0x9D104i32 => "CMRE",
0x7A238Ci32 => "RBRAN",
0x11C1CFFBi32 => "HRPOST",
0xFD40812i32 => "ANNRLR",
0xDBF68i32 => "EXTRUSION",
0xBF9D7i32 => "SPWLD",
0xC942934i32 => "MPTBRN",
0xBEB83i32 => "WORLD",
0xAFC57i32 => "DPOINT",
0x9541Di32 => "WPAD",
0xC4EF92Ai32 => "CTJOIN",
0x5A9049Fi32 => "MNPROF",
0xC7867i32 => "SANNULUS",
0xF0A81i32 => "MDBW",
0xE26D389i32 => "SPLTMP",
0xD709C56i32 => "DSTGRO",
0xDD0FE02i32 => "CGRDCP",
0xC06ECi32 => "TEAM",
0xC5F18i32 => "TDENSITY",
0x8F584Bi32 => "FMGRP",
0x9D49Bi32 => "DTSET",
0xD9DF5i32 => "ADIRECTION",
0x11BFF760i32 => "HSLIST",
0x4EE7DECi32 => "CCORSET",
0xCD684i32 => "ACTO",
0x11C0F9FDi32 => "INVLST",
0xA0513i32 => "STIFFENER",
0xC2E90i32 => "PCOMPONENT",
0xAB7D17Ei32 => "LCTIML",
0x11993BC7i32 => "TRMONTH",
0x112CBBE2i32 => "CTFEAT",
0x9C5D6i32 => "CONE",
0x11D211ACi32 => "HCTOUT",
0x8ADBBi32 => "HEXAGON",
0xC547Ei32 => "FLANGE",
0x3DC2256i32 => "STAWLD",
0x2C3715i32 => "RWELD",
0x87AB03Ai32 => "MPLRWI",
0x3D79DDi32 => "GPROF",
0x11C0A05Bi32 => "MARKSTYLE",
0x2C6A07i32 => "STWLD",
0xAD7E9i32 => "TUBING",
0xB0D89i32 => "EQUIPMENT",
0x11C2DB20i32 => "FMBSST",
0x10EB3A95i32 => "HYCCTS",
0x3E6F50Bi32 => "CTBEND",
0x11E74F28i32 => "SOLEXT",
0xA783DAAi32 => "MPANEL",
0x1666475i32 => "HYGEPA",
0x3DC232Fi32 => "TABWLD",
0xBFA2Ai32 => "USERWORLD",
0x24CC9Ai32 => "GENNC",
0x267ECC8i32 => "MSTYTB",
0x4B1AF11i32 => "GOBELE",
0xEC7A9i32 => "SREVOLUTION",
0x114C373i32 => "SPMCFA",
0x117071AFi32 => "FURNIT",
0x11D220DBi32 => "GLYOUTLINE",
0x114A894Di32 => "HYLWDT",
0x1071E346i32 => "COCDES",
0x7429BC3i32 => "OBJELH",
0x6D7DD8i32 => "LCOML",
0xC6B9Bi32 => "JOINT",
0x3E03E26i32 => "LCTIMD",
0xCA7B1i32 => "BRCO",
0xF818D3Di32 => "DSXMBR",
0xDE12EA3i32 => "ISODEPT",
0x11C0F1E9i32 => "TRSLST",
0x2A3033Di32 => "SPMFAC",
0x149355Di32 => "GICPLA",
0xBFA16i32 => "ASWLD",
0xC6BAFi32 => "CPIN",
0xE4833i32 => "EYENUT",
0xD88C3i32 => "BSAREA",
0x55C742i32 => "HIPOI",
0x4ECA60i32 => "ADISH",
0x11C2CE7Ei32 => "LAYRST",
0x19FB0Ai32 => "SPMEB",
0x82663i32 => "ARC",
0xE3F888Ai32 => "CTSUPP",
0x1CC451Ci32 => "NOMTAB",
0xDEAEAi32 => "SDISK",
0x17580FEi32 => "TKPARA",
0xA755BFFi32 => "HOLDEL",
0x553C1D1i32 => "ASSDEF",
0x1672B5i32 => "HATTA",
0xF8C29i32 => "VRTX",
0xA9671DDi32 => "EXPFILTER",
0x11A118CBi32 => "CPRNOT",
0xC89A1i32 => "ACTN",
0xCD691i32 => "NCTORUS",
0xC4E21DCi32 => "SCPLINE",
0x3D7CEDi32 => "HRSOF",
0xBE195i32 => "SBOLT",
0x4E156E4i32 => "RSTAREA",
0xAE312EEi32 => "ATTRRL",
0x10140C91i32 => "GPROTR",
0x11C18899i32 => "MPRNST",
0xCC949i32 => "PLOOP",
0x926D5i32 => "NSLCYLINDER",
0xC87A39i32 => "SPMSW",
0xFA7F9i32 => "NLCYLINDER",
0x7825A17i32 => "MNPATH",
0xB551429i32 => "GPITEM",
0x76178Di32 => "XGEOMETRY",
0x115F6FFi32 => "SPMGFA",
0xC8DBFi32 => "BOUNDARY",
0xABA1Bi32 => "DISH",
0xA5E27i32 => "HANGER",
0x858A9i32 => "SHU",
0x15163AD0i32 => "CSURPX",
0x676B3C5i32 => "HFLANG",
0xA23B7i32 => "FONTFILE",
0xAEF4EBi32 => "HINOT",
0x71E456i32 => "GSEAM",
0x9DB31i32 => "PAVERT",
0xC0EE9D4i32 => "CWBRAN",
0x7A1E8Di32 => "HIPAN",
0xD707D03i32 => "DSIGRO",
0x8764Ei32 => "USDA",
0x12A11772i32 => "SCACTUATOR",
0x788CB4Ei32 => "MRAWTH",
0xE20F6i32 => "DDATA",
0x6D1487i32 => "XCELL",
0x7D4FD8Ai32 => "MPTFCI",
0x8B9E7i32 => "SLABEL",
0x8CB181i32 => "HCLIP",
0x8F3AEi32 => "NTUBE",
0x140111i32 => "HIPLA",
0xB023F9i32 => "STLST",
0x18A9A5i32 => "SCCABLE",
0x9DBA0i32 => "SEVERT",
0x4F177A2i32 => "GPLATE",
0x828FCi32 => "ROD",
0x15E8BAi32 => "APYRAMID",
0xA1835i32 => "CMPFITTING",
0xE4B8E40i32 => "JNTGRP",
0x3DC584Ai32 => "GSTWLD",
0x8DF8Fi32 => "TRNB",
0x9B4CDi32 => "POHEDRON",
0x154B05i32 => "TEXPANSION",
0xD358BEi32 => "CTRAY",
0xDD8C6i32 => "SUBSTRUCTURE",
0x350719i32 => "ACONE",
0x8B9DBi32 => "GLABEL",
0xFA704i32 => "LCCYLINDRICAL",
0xAE14Bi32 => "SBFITTING",
0xDD408i32 => "TCASE",
0x662E66Fi32 => "MANPKG",
0xE96D4i32 => "SNOUT",
0x26187CDi32 => "MWLDTB",
0x1079E78i32 => "SYSMDA",
0x115184E0i32 => "DRSSET",
0xC2A15i32 => "COMM",
0xDFA40i32 => "CONSTRAINT",
0x11C801B5i32 => "INFITTING",
0xBF9ADi32 => "DOWLD",
0xC0EE7C8i32 => "SCBRANCH",
0xA6DFDDi32 => "PTPOS",
0x9BF26i32 => "SELEC",
0x4EE81CCi32 => "WLPRSE",
0xD2F13i32 => "SEXPANSION",
0x557F7C7i32 => "SFTREF",
0x4C0DA01i32 => "GCPANE",
0xC83F476i32 => "GSUBPN",
0xAE29Ai32 => "COFITTING",
0x4B0C612i32 => "CTABLE",
0x3DC5DDEi32 => "DRVWLD",
0xB47E7i32 => "PCOJOINT",
0xE25392Bi32 => "VMCOMP",
0x4881676i32 => "SCPAGE",
0xCC94Ci32 => "SLOOP",
0x1672BFi32 => "RATTA",
0x35A0F1i32 => "SCOPE",
0x8D8CEi32 => "SHLB",
0xE084Bi32 => "GMSSET",
0xD337Bi32 => "MTYPE",
0x102DE14Bi32 => "STRTWR",
0x7C83C35i32 => "HYGRAI",
0x1155EF38i32 => "MPKGFT",
0x8D92Ai32 => "CLLB",
0xB1C6DD8i32 => "VWSTYL",
0x8AB0Bi32 => "VFWAY",
0xAFB74Bi32 => "GPART",
0xE55C2i32 => "TRST",
0xB1C6DF1i32 => "TXSTYL",
0x77B5ABi32 => "ISOTM",
0x4EC9F5Di32 => "RAILSET",
0xA94D461i32 => "TRFAILURE",
0xF2F71i32 => "SCOWL",
0x339FC89i32 => "HYDWSC",
0xE4B6087i32 => "WLDGRP",
0x4B1C0BEi32 => "WTHELE",
0xA7AAFAFi32 => "BLEVEL",
0xBBA69ECi32 => "PLTFRM",
0x754EEEi32 => "SVOLMODEL",
0xDDE3Di32 => "NSDSH",
0x4F1779Ei32 => "CPLATE",
0x4B1E7ECi32 => "INVELEMENT",
0xE19F97Bi32 => "CGRDLP",
0xCEA0FA0i32 => "GXTRAO",
0xB09723Ai32 => "LINKWLD",
0x128EC6EAi32 => "MRAWQU",
0xA0597Fi32 => "AEXTRUSION",
0x101DB949i32 => "FIXTUR",
0xE253911i32 => "WLCOMPONENTS",
0x1CC573Ai32 => "HYSTAB",
0xDB037i32 => "DOOR",
0x23D3C17i32 => "HYPROB",
0xC7B71i32 => "NCONE",
0x2A7C1D1i32 => "SCHVAC",
0x2E761D7i32 => "HYCRIC",
0x7F1FC08i32 => "SCHVFITTING",
0x560E06i32 => "GENPI",
0xEE047i32 => "CINVENTORY",
0xD9EC4i32 => "SKIR",
0x936D2i32 => "CIRCLE",
0x11BFF768i32 => "PSLIST",
0xB09DFA0i32 => "STYLWL",
0xCF99D54i32 => "SCOPCO",
0xD8833i32 => "TMAREA",
0xC0E5172i32 => "HICPAN",
0x11C5C192i32 => "SETATTRIBUTE",
0x1074805i32 => "APPLDATA",
0xBEEBFi32 => "NSSLCYLINDER",
0xDB0CCi32 => "RTORUS",
0xDC3799Ei32 => "MBRMAP",
0xB0C0C16i32 => "UDETWL",
0xFA365i32 => "CWAY",
0x1053DA88i32 => "LNCLAS",
0x14019Di32 => "MNPLA",
0x86A162Di32 => "SCEQUIPMENT",
0xDFD6Ai32 => "CROSS",
0xC839D3Ei32 => "HBRAPN",
0xAF71Di32 => "PTMIX",
0xB47EAi32 => "SCOJOINT",
0x11C0CCE4i32 => "TRFLST",
0xBFA4Di32 => "BUWLD",
0xDBF71i32 => "NXTRUSION",
0x326328i32 => "CTTEE",
0xB70DD49i32 => "FMWDIM",
0x506E2D5i32 => "CCURVE",
0xE2267A5i32 => "SCTEMPLATE",
0x15A685i32 => "HIBRA",
0x4B1D3D1i32 => "SLOELE",
0xCB86Ei32 => "UNION",
0x2D2B40Ci32 => "HYGCGC",
0xCA4FFi32 => "NSBOX",
0xD8C19i32 => "SWBR",
0x11A8DDF3i32 => "GCOMPT",
0x506E2D9i32 => "GCURVE",
0x60DF332i32 => "LDRCAGE",
0xA06B08i32 => "HICUR",
0x6FA34Ei32 => "DBSTL",
0xAC632DAi32 => "EXPCOLUMN",
0x6C4DC3i32 => "HIPIL",
0x8261Di32 => "LOC",
0x189E86Fi32 => "SSNOTA",
0x42964Bi32 => "CBSEGMENT",
0xE9580i32 => "CBOUNDARY",
0xB70A17Ci32 => "FMBDIM",
0x239E4E3i32 => "SPMGOB",
0xA7347i32 => "PLUG",
0xE2DECi32 => "ASET",
0x8D92Bi32 => "DLLB",
0xB55143Ai32 => "XPITEM",
};
use std::fs::File;
use std::io::{BufReader, Read};
use dashmap::DashMap;
use nalgebra_glm::exp;
use nom::IResult;
use nom::number::complete::{be_i32, be_u16, be_i16, be_u32};
use nom::sequence::tuple;
use smol_str::SmolStr;
use crate::db_tool::{convert_to_hash, db1_dehash};
use crate::parse::{convert_to_explicit_axis_string, convert_to_implicit_axis_string};
use crate::pdms_types::AttrVal::*;
use crate::pdms_types::{AttrVal, DbAttributeType};
use crate::pdms_types::DbAttributeType::*;


const ATT_PX: i32 = 0xFFF7E177u32 as i32;
const ATT_PY: i32 = 0xFFF7E15Cu32 as i32;
const ATT_PZ: i32 = 0xFFF7E141u32 as i32;
const ATT_PDIA: i32 = 0xFFF77D0Fu32 as i32;
const ATT_PHEI: i32 = 0xFFF520EFu32 as i32;
const ATT_PDIS: i32 = 0xFFF21519u32 as i32;
const ATT_PCON: i32 = 0xFFF3848Du32 as i32;
const ATT_PBOR: i32 = 0xFFF2511Cu32 as i32;
const ATT_PPRO: i32 = 0xFFF32DC0u32 as i32;
const ATT_DPRO: i32 = 0xFFF32DCCu32 as i32;
const ATT_BTHK: i32 = 0xFFF47D68u32 as i32;
const ATT_PTCDI: i32 = 0x95A34;


#[inline]
/// 若在反序列化给定的map集合中未找到该属性对应的hash ，则用该方法直接获取到该属性的数据类型 ，然后进行解析
pub fn get_explicit_attr_type(input: u16) -> Option<DbAttributeType> {
    match input {
        // 2800 这个应该是个引用，数据给的是一个参考号 ，但是e3d没有这个属性值 ，但是他的类型不难看出是string   类型: 2C F2 AE D3
        0x3C00 | 0x2800 => { Some(STRING) }
        0x1800 => { Some(DOUBLEVEC) }
        0x1C00 | 0x2000 => { Some(INTVEC) }
        0x4000 | 0x1000 => { Some(ELEMENT) }
        0x0C00 => { Some(INTEGER) }
        0x1400 => { Some(BOOL) }
        0x0800 => { Some(DOUBLE) }
        0x3800 => { Some(TYPEX) }
        0x0000 => { None }
        _ => {
            // println!("failed to find explicit attr type {:#04X?} position={:#04X?}", input, pos);
            None
        }
    }
}

/// 解析表达式
pub fn get_expression_attr(explict_num: i32, input: &[u8]) -> IResult<&[u8], (String, String)> {
    let mut key = db1_dehash(explict_num.abs() as u32);
    let mut expression_data = &input[8..];
    // 表达式都是以0x0 0 0 1开头的
    let _expression_start = &expression_data[..4];
    expression_data = &expression_data[4..];
    // 这是表达式数字的起始标志
    let mut result_stack = vec![];
    while (&expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] || &expression_data[..4] == &[0x0, 0x0, 0x0, 0x6A] || &expression_data[..3] == &[0x0, 0x0, 0x3]) && expression_data.len() >= 8 {
        if &expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] {
            expression_data = &expression_data[8..];
            // 表达式的值
            let expression_data_value = &expression_data[..12];
            let mut dst_data = expression_data_value[..8].to_vec();
            let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
            dst_data[0] = dst_first;
            dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
            let value_tmp = f64::from_be_bytes(dst_data.try_into().unwrap());
            let value = (f64::trunc(value_tmp * 100.0) / 100.0).to_string();
            result_stack.push(value);
            expression_data = &expression_data[12..];
            // 表达式 值的结束位  这里是个结束位，但是没什么用，后期判断当表达式的值特别大的时候是否有用（目前遇到的值都是三位）
            let _expression_data_value_end = &expression_data[..8];
            expression_data = &expression_data[8..];
        }
        //若后面是6A 则代表该值没完
        while &expression_data[..4] == &[0x0u8, 0x0, 0x0, 0x6A][..] {
            // 跳6A 和02
            expression_data = &expression_data[8..];
            match &expression_data[..16] {
                &[0x0, 0x8, 0x9C, 0x41, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB PARA";
                    let value = result_stack.pop().unwrap();
                    let value = format!("{}[{}]", expression, value);
                    result_stack.push(value);
                }
                &[0x0, 0xD, 0x88, 0x79, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB IPAR";
                    let value = result_stack.pop().unwrap();
                    let value = format!("{}[{}]", expression, value);
                    result_stack.push(value);
                }
                &[0x0, 0xD, 0xDF, 0x8A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0, 0x0, 0x0, 0x0] => {
                    let expression="ATTRIB WDESP";
                    let value = result_stack.pop().unwrap();
                    let value = format!("{}[{}]", expression, value);
                    result_stack.push(value);
                }
                &[0x0, 0xB, 0xCB, 0xFF, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB ANGL".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xB, 0x20, 0x9F] => {
                    let expression = "ATTRIB RPRO DIAJ".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xA, 0x5E, 0x97] => {
                    let expression = "ATTRIB RPRO LENG".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x8, 0x1C, 0x3] => {
                    let expression = "ATTRIB RPRO R".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xA, 0x50, 0x56] => {
                    let expression = "ATTRIB RPRO HEIG".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xE, 0x2A, 0x1B] => {
                    let expression = "ATTRIB RPRO WIDT".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x8, 0x44, 0x59] => {
                    let expression = "ATTRIB RPRO CEN".to_string();
                    result_stack.push(expression);
                }
                _ => {}
            }

            // 又跳过20个bytes不知道干啥的，但是是这个if里面才有的(还有上面那个match)
            expression_data = &expression_data[24..];
        }

        // 这里表达式是结束了，但是可能会有后序表达式的运算符
        while &expression_data[..4] != &[0x0, 0x0, 0x0, 0x65] && expression_data.len() >= 4 && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x6A] {
            let mut symbol = "+".to_string();
            match &expression_data[..4] {
                &[0x0, 0x0, 0x3, 0x21] => {
                    // 这是负号
                    let value = result_stack.pop().unwrap();
                    symbol = format!("(-{})", value);
                }
                &[0x0, 0x0, 0x3, 0x22] => {
                    let value2 = result_stack.pop().unwrap();
                    let value1 = result_stack.pop().unwrap();
                    symbol = format!("({}+{})", value1, value2);
                }
                &[0x0, 0x0, 0x3, 0x23] => {
                    let value2 = result_stack.pop().unwrap();
                    let value1 = result_stack.pop().unwrap();
                    symbol = format!("({}-{})", value1, value2);
                }
                &[0x0, 0x0, 0x3, 0x24] => {
                    let value2 = result_stack.pop().unwrap();
                    let value1 = result_stack.pop().unwrap();
                    symbol = format!("{}*{}", value1, value2);
                }
                &[0x0, 0x0, 0x3, 0x25] => {
                    let value2 = result_stack.pop().unwrap();
                    let value1 = result_stack.pop().unwrap();
                    symbol = format!("{}/{}", value1, value2);
                }
                &[0x0, 0x0, 0x3, 0xE9] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("SQRT({})", value);
                }
                &[0x0, 0x0, 0x3, 0x85] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("SIN({})", value);
                }
                &[0x0, 0x0, 0x3, 0x86] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("COS({})", value);
                }
                &[0x0, 0x0, 0x3, 0x87] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("TAN({})", value);
                }
                &[0x0, 0x0, 0x3, 0x88] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("ASIN({})", value);
                }
                &[0x0, 0x0, 0x3, 0x89] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("ACOS({})", value);
                }
                &[0x0, 0x0, 0x3, 0x8A] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("ATAN({})", value);
                }
                &[0x0, 0x0, 0x3, 0x8B] => {  //这个ATAN有两个值
                    let value1 = result_stack.pop().unwrap();
                    let value2 = result_stack.pop().unwrap();
                    symbol = format!("ATAN({},{})", value2, value1);
                }
                &[0x0, 0x0, 0x3, 0xEA] => {
                    let value1 = result_stack.pop().unwrap();
                    let value2 = result_stack.pop().unwrap();
                    symbol = format!("POW({},{})", value2, value1);
                }
                &[0x0, 0x0, 0x3, 0xEB] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("LOG({})", value);
                }
                &[0x0, 0x0, 0x3, 0xEC] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("ALOG({})", value);
                }
                &[0x0, 0x0, 0x3, 0xED] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("INT({})", value);
                }
                &[0x0, 0x0, 0x3, 0xEE] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("NINT({})", value);
                }
                &[0x0, 0x0, 0x3, 0xEF] => {
                    let value = result_stack.pop().unwrap();
                    symbol = format!("ABS({})", value);
                }
                &[0x0, 0x0, 0x3, 0xF0] => {
                    let value1 = result_stack.pop().unwrap();
                    let value2 = result_stack.pop().unwrap();
                    let mut max_array = format!("{},{}", value2, value1);
                    while expression_data.len() > 36 && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF0] {
                        let expression_data_value = &expression_data[12..24];
                        let mut dst_data = expression_data_value[..8].to_vec();
                        let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                        dst_data[0] = dst_first;
                        dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                        max_array = format!("{},{}", max_array, value);
                        expression_data = &expression_data[32..];
                    }
                    symbol = format!("MAX({})", max_array);
                }
                &[0x0, 0x0, 0x3, 0xF1] => {
                    let value1 = result_stack.pop().unwrap();
                    let value2 = result_stack.pop().unwrap();
                    let mut max_array = format!("{},{}", value2, value1);
                    while expression_data.len() > 36 && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF1] {
                        let expression_data_value = &expression_data[12..24];
                        let mut dst_data = expression_data_value[..8].to_vec();
                        let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                        dst_data[0] = dst_first;
                        dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                        max_array = format!("{},{}", max_array, value);
                        expression_data = &expression_data[32..];
                    }
                    symbol = format!("MIN({})", max_array);
                }
                _ => {}
            }
            result_stack.push(symbol);
            if expression_data.len() > 4 {
                expression_data = &expression_data[4..];
            } else {
                let result = format!("({})", result_stack[0]);
                return Ok((input, (key, result)));
            }
        }
    }
    let result = format!("({})", result_stack[0]);
    Ok((input, (key, result)))
}

#[test]
fn get_expression_attr_test() {
    let mut file = File::open("BDIA").unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    let (_, (types, result)) = parse_expression_attr(&attr_buf).unwrap();
    println!("type={},result={}", types, result);
}

pub fn parse_expression_attr(input: &[u8]) -> IResult<&[u8], (String, SmolStr)> {
    let hash_val = &input[..4];
    let expression_type = db1_dehash(convert_to_hash(hash_val));
    // let expression_type = db1_dehash();
    if expression_type == "PTCD" {
        let (_, expression_length) = be_u16(&input[6..8])?;
        // 显式属性的length后有8个byte没用的，直接跳过了
        let expression_data = &input[8..(expression_length * 4) as usize + 8];
        let input = &input[(expression_length * 4) as usize + 8..];
        let (_, axis) = convert_to_explicit_axis_string(expression_data)?;
        let mut result:SmolStr = "".into();
        match axis {
            StringType(value) => {
                result = value;
            }
            _ => {}
        }
        Ok((input, (expression_type, result)))
    } else {
        let (_, expression_length) = be_u16(&input[6..8])?;
        // 显式属性的length后有8个byte没用的，直接跳过了
        let mut expression_data = &input[16..(expression_length * 4) as usize + 8];
        let input = &input[(expression_length * 4) as usize + 8..];
        // 表达式都是以0x0 0 0 1开头的
        let _expression_start = &expression_data[..4];
        expression_data = &expression_data[4..];
        // 这是表达式数字的起始标志
        let mut result_stack = vec!["".to_string()];
        while expression_data.len() >= 8 && (&expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] || &expression_data[..4] == &[0x0, 0x0, 0x0, 0x6A] || &expression_data[..3] == &[0x0, 0x0, 0x3] || &expression_data[..4] == &[0x0, 0x0, 0x0, 0x3]) {
            if &expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] {
                expression_data = &expression_data[8..];
                // 表达式的值
                match &expression_data[8..10] {
                    &[0x0, 0x0] => {
                        let (_, times) = be_i16(&expression_data[10..12])?;
                        let times = 2_f32.powf((5i16 - times) as f32) as f64;
                        let (_, a) = be_i32(&expression_data[..4])?;
                        let (_, b) = be_i32(&expression_data[4..8])?;
                        let value = (((a as f64 / 0x400 as f64) + (b as f64 / 0x20000000 as f64)) / times * 100.0).round() / 100.0;
                        result_stack.push(value.to_string());
                        expression_data = &expression_data[20..];
                    }

                    &[0x40, 0x0] => {
                        let expression_data_value = &expression_data[..12];
                        let mut dst_data = expression_data_value[..8].to_vec();
                        let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                        dst_data[0] = dst_first;
                        dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                        let value_tmp = f64::from_be_bytes(dst_data.try_into().unwrap());
                        let value = ((value_tmp * 100.0).round() / 100.0).to_string();
                        result_stack.push(value);
                        expression_data = &expression_data[12..];
                        // 表达式 值的结束位  这里是个结束位，但是没什么用，后期判断当表达式的值特别大的时候是否有用（目前遇到的值都是三位）
                        let _expression_data_value_end = &expression_data[..8];
                        expression_data = &expression_data[8..];
                    }
                    &[0xFF, 0xFF] => {
                        let (_, time) = be_u16(&expression_data[10..12])?;
                        let time = (time - 0xFFFB) as f32 * 2.0;
                        let (_, value1) = be_i32(&expression_data[..4])?;
                        let value1 = value1 as f32 * 0.000001f32 * time;
                        let (_, value2) = be_i32(&expression_data[4..8])?;
                        let value2 = value2 as f32 / (0x6680 as f32 / time) * 0.000001;
                        let value = ((value1 + value2) * 100.0).round() / 100.0;
                        result_stack.push(value.to_string());
                        expression_data = &expression_data[20..];
                    }
                    _ => {}
                }
            }
            //若后面是6A 则代表该值没完
            while expression_data.len() > 4 && &expression_data[..4] == &[0x0u8, 0x0, 0x0, 0x6A][..] {
                // 跳6A
                expression_data=&expression_data[4..];
                match &expression_data[..8] {
                    &[0x0, 0x0, 0x0, 0x1, 0x0, 0xE, 0x95, 0xA5] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB MCOU ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB MCOU ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0x8, 0x9C, 0x41] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB PARA";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{}[{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let (_, val) = be_i32(&expression_data[8..12])?;
                            let expression = format!("ATTRIB PARA [{}]", val.to_string());
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xD, 0x88, 0x79] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB IPAR";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{}[{}] ", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = "ATTRIB IPAR ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xD, 0x20, 0xC7] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB DESP";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{} [{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = format!("ATTRIB DESP ");
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xD, 0xDF, 0x77] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB DDESP";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{} [{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = format!("ATTRIB DDESP ");
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x17, 0xEF, 0x4B, 0x61] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB :HXYsize";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{} [{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = format!("ATTRIB :HXYsize ");
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x6, 0x0, 0xD, 0x88, 0x87] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB WPAR";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{} [{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = format!("ATTRIB WPAR ");
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x6, 0x0, 0xD, 0xDF, 0x8A] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB WDESP";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{} [{}]", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = format!("ATTRIB WDESP ");
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x4, 0x0, 0xD, 0xCA, 0x5F ] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB DTXR ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB DTXR ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x4, 0x0, 0xC, 0x2C, 0xA0] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB FLNM ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB FLNM ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x4, 0x0, 0x8, 0x82, 0xE3] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB BDIA ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB BDIA ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x4, 0x0, 0xD, 0x33, 0x70] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB BTYP ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB BTYP ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xB, 0xCB, 0xFF ] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB ANGL ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB ANGL ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xC, 0xD2, 0x42] => {
                        match &expression_data[16..20] {
                            &[0x0, 0xB, 0x20, 0x9F] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO DIAJ ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO DIAJ ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0xA, 0x5E, 0x97] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO LENG ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO LENG ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0xA, 0xBD, 0x47] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO FLTH ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO FLTH ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0x8, 0x1C, 0x03] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO R ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO R ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0xA, 0x50, 0x56] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO HEIG".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO HEIG".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0xE, 0x2A, 0x1B] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO WIDT ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO WIDT ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0x7, 0x44, 0x59] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO CNE ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO CNE ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            &[0x0, 0x8, 0x82, 0xE3] => {
                                if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                                    let expression = "ATTRIB RPRO BDIA ".to_string();
                                    result_stack.pop().unwrap();
                                    result_stack.push(expression);
                                } else {
                                    let expression = "ATTRIB RPRO BDIA ".to_string();
                                    result_stack.push(expression);
                                }
                            }
                            _ => {}
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x4, 0x0, 0xF, 0xAD, 0x95] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB SKEY ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB SKEY ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x2, 0x0, 0xD, 0x88, 0x73] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB CPAR";
                            let value = result_stack.pop().unwrap();
                            let value = format!("{}[{}] ", expression, value);
                            result_stack.push(value);
                        } else {
                            let expression = "ATTRIB CPAR ".to_string();
                            result_stack.push(expression);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x5, 0x0, 0xD, 0xBC, 0xF9] => {
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let expression = "ATTRIB CATR ".to_string();
                            result_stack.pop().unwrap();
                            result_stack.push(expression);
                        } else {
                            let expression = "ATTRIB CATR ".to_string();
                            result_stack.push(expression);
                        }
                    }

                    _ => {
                        expression_data = &expression_data[4..];
                        let (_,n)=get_expression_func_name(&expression_data[..8])?;
                        if &expression_data[8..16] == &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF] {
                            let value = result_stack.pop().unwrap();
                            let value = format!("{}[{}] ", n, value);
                            result_stack.push(value);
                        } else {
                            result_stack.push(n);
                        }
                    }
                }
                // OF = 类型表达式解析，目前推测是这样 但不肯定
                if &expression_data[20..24] == &[0x0, 0x0, 0x6, 0x42] {
                    let (tmp_input, length) = be_i32(&expression_data[24..])?;
                    let length = length as usize * 4;
                    let mut expression_input = &tmp_input[4..length];
                    while expression_input.len() > 4 {
                        let expression = get_expression_of_func(&expression_input[..4]);
                        if expression != "".to_string() {
                            let func = result_stack.pop().unwrap();
                            let result = format!("{} OF {} ", func, expression);
                            result_stack.push(result);
                            if expression_input.len() < 20 {
                                break;
                            }
                            expression_input = &expression_input[20..];
                        }
                        let (expression_tmp, (refno0, refno1, )) = tuple((
                            be_i32,
                            be_i32,
                        ))(&expression_input[..])?;
                        expression_input = &expression_tmp[..];
                        if refno0 == 0 {
                            let expression = get_expression_of_func(&expression_input[..4]);
                            let func = result_stack.pop().unwrap();
                            let result = format!("{} OF {} ", func, expression);
                            result_stack.push(result);
                            //func后面有3个word的数据不知道是干什么的
                            expression_input = &expression_input[16..];
                        } else {
                            let refno = format!("{}/{}", refno0, refno1);
                            let func = result_stack.pop().unwrap();
                            let result = format!("( {} OF = {} )", func, refno);
                            return Ok((input, (expression_type, result.into())));
                        }
                    }
                    expression_data = &expression_data[length..];
                } else {
                    // 跳过普通表达式的结束位
                    expression_data = &expression_data[24..];
                }
            }
            // 这里表达式是结束了，但是可能会有后序表达式的运算符
            while expression_data.len() >= 4 && &expression_data[..2] == &[0x0, 0x0] && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x65] && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x6A] && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x2] {
                let mut symbol = "".to_string();
                match &expression_data[..4] {
                    &[0x0, 0x0, 0x3, 0x21] => {
                        // 这是负号
                        let value = result_stack.pop().unwrap();
                        symbol = format!("( - {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x22] => {
                        if result_stack.len() > 1 {
                            let value2 = result_stack.pop().unwrap();
                            let value1 = result_stack.pop().unwrap();
                            symbol = format!("( {} + {} )", value1, value2);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0x23] => {
                        if result_stack.len() > 1 {
                            let value2 = result_stack.pop().unwrap();
                            let value1 = result_stack.pop().unwrap();
                            symbol = format!("( {} - {} )", value1, value2);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0x24] => {
                        if result_stack.len() > 1 {
                            let value2 = result_stack.pop().unwrap();
                            let value1 = result_stack.pop().unwrap();
                            symbol = format!("{} * {}", value1, value2);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0x25] => {
                        if result_stack.len() > 1 {
                            let value2 = result_stack.pop().unwrap();
                            let value1 = result_stack.pop().unwrap();
                            symbol = format!("{}/{}", value1, value2);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0xE9] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("SQRT ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x85] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("SIN ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x86] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("COS ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x87] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("TAN ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x88] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("ASIN ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x89] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("ACOS ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x8A] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("ATAN ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0x8B] => {  //这个ATAN有两个值
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap();
                            let value2 = result_stack.pop().unwrap();
                            symbol = format!("ATAN ( {} , {} )", value2, value1);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0xEA] => {
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap();
                            let value2 = result_stack.pop().unwrap();
                            symbol = format!("POW ( {} , {} )", value2, value1);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0xEB] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("LOG ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0xEC] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("ALOG ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0xED] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("INT ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0xEE] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("NINT ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0xEF] => {
                        let value = result_stack.pop().unwrap();
                        symbol = format!("ABS ( {} )", value);
                    }
                    &[0x0, 0x0, 0x3, 0xF0] => {
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap();
                            let value2 = result_stack.pop().unwrap();
                            let mut max_array = format!("{},{}", value2, value1);
                            while expression_data.len() > 36 && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF0] {
                                let expression_data_value = &expression_data[12..24];
                                let mut dst_data = expression_data_value[..8].to_vec();
                                let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                                dst_data[0] = dst_first;
                                dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                                let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                                max_array = format!("{},{}", max_array, value);
                                expression_data = &expression_data[32..];
                            }
                            symbol = format!("MAX ( {} )", max_array);
                        }
                    }
                    &[0x0, 0x0, 0x3, 0xF1] => {
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap();
                            let value2 = result_stack.pop().unwrap();
                            let mut max_array = format!("{},{}", value2, value1);
                            while expression_data.len() > 36 && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF1] {
                                let expression_data_value = &expression_data[12..24];
                                let mut dst_data = expression_data_value[..8].to_vec();
                                let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                                dst_data[0] = dst_first;
                                dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                                let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                                max_array = format!("{},{}", max_array, value);
                                expression_data = &expression_data[32..];
                            }
                            symbol = format!("MIN ( {} )", max_array);
                        }
                    }
                    &[0x0, 0x0, 0x0, 0x3] => {
                        if &expression_data[4..8] == &[0x0, 0x0, 0x6, 0xA5] {
                            if result_stack.len() > 1 {
                                let value1 = result_stack.pop().unwrap();
                                let value2 = result_stack.pop().unwrap();
                                symbol = format!("{} OF = {}", value2, value1);
                            }
                        }
                    }
                    _ => {}
                }
                if symbol != "" {
                    result_stack.push(symbol);
                }
                if expression_data.len() > 4 {
                    expression_data = &expression_data[4..];
                } else {
                    let result = format!("( {} )", result_stack.pop().unwrap());
                    return Ok((input, (expression_type, result.into())));
                }
            }
        }
        let result = format!("( {} )", result_stack.pop().unwrap());
        Ok((input, (expression_type, result.into())))
    }
}

/// 返回ATTRIB PARA类的函数名
pub fn get_expression_func_name(input:&[u8]) -> IResult<&[u8],String> {
    let mut result="".to_string();
    let (_,v)=be_u32(&input[4..8])?;
    if v > 0x81BF1 {
        result=format!("ATTRIB {}",db1_dehash(v));
    }
    Ok((input,result))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_axis_explicit_value_00(data: &[u8]) -> IResult<&[u8], f64> {
    let (_, times) = be_i16(&data[10..12])?;
    let times = 2_f32.powf((5i16 - times) as f32) as f64;
    let (_, a) = be_i32(&data[..4])?;
    let (_, b) = be_i32(&data[4..8])?;
    let value = f64::trunc(((a as f64 / 0x400 as f64) + (b as f64 / 0x20000000 as f64)) / times * 100.0) / 100.0;
    Ok((data, value))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_axis_explicit_value_40(data: &[u8]) -> IResult<&[u8], f64> {
    let mut dst_data = data[..8].to_vec();
    let dst_first = (data[10] & 0xF).checked_shl(4).unwrap() + (data[11] & 0xF0).checked_shr(4).unwrap();
    dst_data[0] = dst_first;
    dst_data[1] = (data[11] & 0xF).checked_shl(4).unwrap() + (data[1] & 0xF);
    let value = f64::from_be_bytes(dst_data.try_into().unwrap());
    Ok((data, value))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_axis_explicit_value_ff(data: &[u8]) -> IResult<&[u8], f64> {
    let (_, time) = be_u16(&data[10..12])?;
    let time = (time - 0xFFFB) as f32 * 2.0;
    let (_, value1) = be_i32(&data[..4])?;
    let value1 = value1 as f32 * 0.000001f32 * time;
    let (_, value2) = be_i32(&data[4..8])?;
    let value2 = value2 as f32 / (0x6680 as f32 / time) * 0.000001;
    let value = ((value1 + value2) * 100.0).round() / 100.0;
    Ok((data, value as f64))
}

#[inline]
pub fn get_expression_of_func(input: &[u8]) -> String {
    let mut result = "".to_string();
    match input {
        &[0x0, 0xA, 0x1D, 0xCB] => {
            result = "BLRF NUM 1".to_string();
        }
        &[0x0, 0xD, 0xBC, 0xF9] => {
            result = "CATR".to_string();
        }
        &[0x0, 0xD, 0x24, 0x5B] => {
            result = "BLTP 1".to_string();
        }
        _ => {}
    }
    result
}

#[inline]
pub fn times_keep_f32_two_decimal_place(input: i32) -> f32 {
    //let times=(((times as f32)/40.0f32 ) * 100.0 ).round() / 100.0;
    let input = input as f32;
    let result = input / 40.0f32 * 100.0;
    let b_seven = result as i32 % 10 == 7 && result < 100.0;
    let mut result = result;
    if b_seven {
        result = f32::trunc(result) / 100.0;
    } else {
        result = result.round() / 100.0;
    }
    result
}


#[test]
fn ceil_test() {
    let value1 = 15.999999f32;
    let value2 = 18.1000f32;
    let value1 = f32::trunc((value1 + 0.000001) * 100.0) / 100.0;
    let value2 = f32::trunc((value2 + 0.000001) * 100.0) / 100.0;
    let value3 = -2.4001_f32.round();
    let value4 = times_keep_f32_two_decimal_place(19);
    println!("value1={}", value1);
    println!("value2={}", value2);
    println!("value3={}", value3);
    println!("value4={}", value4);
}

#[test]
fn pow_test() {
    let times = 6;
    let times = 2_f32.powf((5 - times) as f32) as f64;
    println!("times={}", times);
    let value = (0x4E00 / 0x400) as f64 / times;
    println!("value={}", value);
}

#[test]
fn read_deseralize_file() {
    let file = File::open("E:/AVEVA/Plant/PDMS12.0.SP4/expression_test.json").unwrap();
    let reader = BufReader::new(file);
    let database_info: DashMap<String, Vec<(String, String)>> = serde_json::from_reader(reader).unwrap();
    println!("value={:?}", database_info);
}





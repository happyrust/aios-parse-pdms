use std::fs::File;
use std::io::Read;
use log::kv::ToKey;
use memchr::memmem::find_iter;
use nom::error::Error;
use nom::IResult;
use nom::number::complete::{be_f64, be_u16, be_u32};
use crate::{convert_to_implicit_axis_string, DbAttributeType};
use crate::pdms_types::DbAttributeType::*;

const ADD: &str = "+";
const SUBTRACT: &str = "-";
const MULTIPLICATION: &str = "*";
const DIVISION: &str = "/";
const SQRT: &str = "SQRT";
const SIN : &str = "SIN";
const COS: &str = "COS";

#[inline]
/// 若在反序列化给定的map集合中未找到该属性对应的hash ，则用该方法直接获取到该属性的数据类型 ，然后进行解析
pub fn get_explicit_attr_type(input: u16, pos: usize) -> Option<DbAttributeType> {
    match input {
        // 2800 这个应该是个引用，数据给的是一个参考号 ，但是e3d没有这个属性值 ，但是他的类型不难看出是string   类型: 2C F2 AE D3
        0x3C00 | 0x2800 => { Some(STRING) }
        0x1800 => { Some(DOUBLEVEC) }
        0x1C00 => { Some(INTVEC) }
        0x4000 => { Some(ELEMENT) }
        0x0C00 => { Some(INTEGER) }
        0x1400 => { Some(BOOL) }
        0x0800 => { Some(DOUBLE) }
        0x3800 => { Some(TYPEX) }
        _ => {
            println!("failed to find explicit attr type {:#04X?} position={:#04X?}", input, pos);
            None
        }
    }
}

/// 解析表达式 Px Py Pz
pub fn get_expression_attr(input: &[u8]) -> IResult<&[u8], (String,String)> {
    let expression_type_input = &input[..4];
    let mut expression_type = "PX".to_string();
    match expression_type_input {
        &[0xFF, 0xF7, 0xE1, 0x77] => { expression_type = "PX".to_string(); }
        &[0xFF, 0xF7, 0xE1, 0x5C] => { expression_type = "PY".to_string(); }
        &[0xFF, 0xF7, 0xE1, 0x41] => { expression_type = "PZ".to_string(); }
        _ => {}
    }
    let (_, expression_length) = be_u16(&input[6..8])?;
    // 显式属性的length后有8个byte没用的，直接跳过了
    let mut expression_data = &input[16..(expression_length * 4) as usize + 8];
    let input=&input[(expression_length * 4) as usize + 8 ..];
    // 表达式都是以0x0 0 0 1开头的
    let expression_start = &expression_data[..4];
    expression_data = &expression_data[4..];
    // 这是表达式数字的起始标志
    // let expression_data_start = &expression_data[..8];
    let mut result_stack = vec![];
    while (&expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] || &expression_data[..4] == &[0x0, 0x0, 0x0, 0x6A] || &expression_data[..3] == &[0x0,0x0,0x3]) && expression_data.len() >= 8 {
        if &expression_data[..8] == &[0x0, 0x0, 0x0, 0x65, 0x0, 0x0, 0x0, 0x6] {
            expression_data = &expression_data[8..];
            // 表达式的值
            let expression_data_value = &expression_data[..12];
            let mut dst_data = expression_data_value[..8].to_vec();
            let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
            dst_data[0] = dst_first;
            dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
            let value_tmp = f64::from_be_bytes(dst_data.try_into().unwrap());
            let value = (f64::trunc(value_tmp * 100.0) / 100.0 ).to_string();
            result_stack.push(value);
            expression_data = &expression_data[12..];
            // 表达式 值的结束位  这里是个结束位，但是没什么用，后期判断当表达式的值特别大的时候是否有用（目前遇到的值都是三位）
            let expression_data_value_end = &expression_data[..8];
            expression_data = &expression_data[8..];
        }
        //若后面是6A 则代表该值没完
        while &expression_data[..4] == &[0x0u8, 0x0, 0x0, 0x6A][..] {
            // 跳6A 和02
            expression_data = &expression_data[8..];
            match &expression_data[..16] {
                &[0x0, 0x8, 0x9C, 0x41, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB PARA";
                    let value=result_stack.pop().unwrap();
                    let value = format!("{}[{}]", expression, value);
                    result_stack.push(value);
                }
                &[0x0, 0xD, 0x88, 0x79, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB IPAR";
                    let value=result_stack.pop().unwrap();
                    let value = format!("{}[{}]", expression, value);
                    result_stack.push(value);
                }
                &[0x0, 0xB, 0xCB, 0xFF, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0] => {
                    let expression = "ATTRIB ANGL".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xB, 0x20, 0x9F] => {
                    let expression="ATTRIB RPRO DIAJ".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xA, 0x5E, 0x97] => {
                    let expression="ATTRIB RPRO LENG".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x8, 0x1C, 0x3] => {
                    let expression="ATTRIB RPRO R".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xA, 0x50, 0x56] => {
                    let expression="ATTRIB RPRO HEIG".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0xE, 0x2A, 0x1B] => {
                    let expression="ATTRIB RPRO WIDT".to_string();
                    result_stack.push(expression);
                }
                &[0x0, 0xC, 0xD2, 0x42, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0x0, 0x8, 0x44, 0x59] => {
                    let expression="ATTRIB RPRO CEN".to_string();
                    result_stack.push(expression);
                }
                _ => {}
            }

            // 又跳过20个bytes不知道干啥的，但是是这个if里面才有的(还有上面那个match)
            expression_data = &expression_data[24..];
        }

        // 这里表达式是结束了，但是可能会有后序表达式的运算符
        while &expression_data[..4] != &[0x0, 0x0, 0x0, 0x65] && expression_data.len() >= 4 &&&expression_data[..4] != &[0x0, 0x0, 0x0, 0x6A]{
            let mut symbol = "+".to_string();
            match &expression_data[..4] {
                &[0x0, 0x0, 0x3, 0x21] => {
                    // 这是负号
                    let value = result_stack.pop().unwrap();
                    symbol = format!("(-{})", value);
                }
                &[0x0, 0x0, 0x3, 0x22] => {
                    let value2=result_stack.pop().unwrap();
                    let value1=result_stack.pop().unwrap();
                    symbol = format!("({}+{})",value1,value2);
                }
                &[0x0, 0x0, 0x3, 0x23] => {
                    let value2=result_stack.pop().unwrap();
                    let value1=result_stack.pop().unwrap();
                    symbol = format!("({}-{})",value1,value2);
                }
                &[0x0, 0x0, 0x3, 0x24] => {
                    let value2=result_stack.pop().unwrap();
                    let value1=result_stack.pop().unwrap();
                    symbol = format!("{}*{}",value1,value2);
                }
                &[0x0, 0x0, 0x3, 0x25] => {
                    let value2=result_stack.pop().unwrap();
                    let value1=result_stack.pop().unwrap();
                    symbol = format!("{}/{}",value1,value2);
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
                    let value=result_stack.pop().unwrap();
                    symbol = format!("ATAN({})",value);
                }
                &[0x0, 0x0, 0x3, 0x8B] => {  //这个ATAN有两个值
                    let value1=result_stack.pop().unwrap();
                    let value2=result_stack.pop().unwrap();
                    symbol = format!("ATAN({},{})",value2,value1);
                }
                &[0x0, 0x0, 0x3, 0xEA] => {
                    let value1 = result_stack.pop().unwrap();
                    let value2=result_stack.pop().unwrap();
                    symbol = format!("POW({},{})", value2,value1);
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
                    let value2=result_stack.pop().unwrap();
                    let mut max_array=format!("{},{}",value2,value1);
                    while expression_data.len()>36 && &expression_data[32..36] == &[0x0,0x0,0x3,0xF0]{
                        let expression_data_value = &expression_data[12..24];
                        let mut dst_data = expression_data_value[..8].to_vec();
                        let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                        dst_data[0] = dst_first;
                        dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                        max_array=format!("{},{}",max_array,value);
                        expression_data=&expression_data[32..];
                    }
                    symbol=format!("MAX({})",max_array);
                }
                &[0x0, 0x0, 0x3, 0xF1] => {
                    let value1 = result_stack.pop().unwrap();
                    let value2=result_stack.pop().unwrap();
                    let mut max_array=format!("{},{}",value2,value1);
                    while expression_data.len()>36 && &expression_data[32..36] == &[0x0,0x0,0x3,0xF1]{
                        let expression_data_value = &expression_data[12..24];
                        let mut dst_data = expression_data_value[..8].to_vec();
                        let dst_first = (expression_data_value[10] & 0xF).checked_shl(4).unwrap() + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                        dst_data[0] = dst_first;
                        dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap() + (expression_data_value[1] & 0xF);
                        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                        max_array=format!("{},{}",max_array,value);
                        expression_data=&expression_data[32..];
                    }
                    symbol=format!("MIN({})",max_array);
                }
                _ => {}
            }
            result_stack.push(symbol);
            if expression_data.len() > 4 {
                expression_data = &expression_data[4..];
            } else {
                let result=format!("({})",result_stack[0]);
                return Ok((input,(expression_type,result)))
            }
        }
    }
    let result=format!("({})",result_stack[0]);
    Ok((input, (expression_type,result)))
}

#[test]
fn get_expression_attr_test() {
    let mut file = File::open("Untitled13").unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    let (_, (types,result)) = get_expression_attr(&attr_buf).unwrap();
    println!("type={},result={}", types,result);
}

#[test]
fn attr_test() {
    let expression_data_value_old = &[0x00u8, 0x12, 0x0, 0, 0, 0, 0, 0, ][..];
    let expression_data_value_explict = &[0x4, 0x12][..];
    let explict_first_value = expression_data_value_explict[0] & 0xF;
    let explict_second_value = expression_data_value_explict[1] >> 4;
    let explict_third_value = expression_data_value_explict[1] & 0xF;
    let explict_expression_value = expression_data_value_old[1] & 0xF;
    let result1 = explict_first_value << 0x4 | explict_second_value as u8;
    let result2 = explict_third_value << 0x4 | explict_expression_value as u8;
    let replace_array = &[result1, result2][..];
    let result_array = [replace_array, &expression_data_value_old[2..]].concat();
}

#[test]
fn paxis_attr_implicit_test(){
    let mut file = File::open("X90Y").unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    let (_,result)=convert_to_implicit_axis_string(&attr_buf).unwrap();
    println!("result={:?}",result);
}

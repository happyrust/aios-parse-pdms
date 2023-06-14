use std::fs::File;
use std::io::{BufReader, Read, Write};
use aios_core::helper::{parse_to_i16, parse_to_i32, parse_to_u16, parse_to_u32};
use aios_core::pdms_types::{DbAttributeType, RefI32Tuple};
use aios_core::pdms_types::AttrVal::StringType;
use aios_core::pdms_types::DbAttributeType::{BOOL, DOUBLE, DOUBLEVEC, ELEMENT, INTEGER, INTVEC, STRING, TYPEX};
use aios_core::tool::db_tool::{convert_to_hash, db1_dehash};
use aios_core::tool::float_tool::f64_round_3;
use dashmap::DashMap;
use dynfmt::{Format, SimpleCurlyFormat};
use itertools::Itertools;
use nom::combinator::value;
use nom::IResult;
use nom::number::complete::{be_i32, be_u16, be_i16, be_u32};
use nom::sequence::tuple;
use smol_str::SmolStr;
use crate::BHashMap;
use crate::parse::{convert_to_explicit_axis_string, match_explicit_attribute_to_string, parse_to_expression};

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

// pub static STRING_LOOKUP: Lazy<Mutex<StringLookupTable>> = Lazy::new(|| {
//     Mutex::new(StringLookupTable::default())
// });
lazy_static! {
    pub static ref MATH_OPERATORS_MAP: BHashMap<i32, &'static str> = {
        let mut s = BHashMap::new();
        s.insert(0x321, "(-{})");
        s.insert(0x322, "({}+{})");
        s.insert(0x323, "({}-{})");
        s.insert(0x324, "{}*{}");
        s.insert(0x325, "{}/{}");  //need a bracket
        s.insert(0x3E9, "SQRT({})");
        s.insert(0x385, "SIN({})");
        s.insert(0x386, "COS({})");
        s.insert(0x387, "TAN({})");
        s.insert(0x388, "ASIN({})");
        s.insert(0x389, "ACOS({})");
        s.insert(0x38A, "ATAN({})");
        s.insert(0x38B, "ATANT({},{})");
        s.insert(0x3EA, "POW({},{})");
        s.insert(0x3EB, "LOG({})");
        s.insert(0x3EC, "ALOG({})");
        s.insert(0x3ED, "INT({})");
        s.insert(0x3EE, "NINT({})");
        s.insert(0x3EF, "ABS({})");
        // s.insert(0x3F0, "MAX ({},{})");   //特殊处理
        // s.insert(0x3F1, "MIN ({},{})");
        s
    };
}


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
            None
        }
    }
}


/// 解析表达式
pub fn parse_expression_attr(input: &[u8], refno: RefI32Tuple) -> IResult<&[u8], (String, String)> {
    let hash_val = &input[..4];
    let expression_type = db1_dehash(convert_to_hash(hash_val));
    if expression_type == "PTCDI" || expression_type == "PTCD" {
        let (_, expression_length) = be_u16(&input[6..8])?;
        // 显式属性的length后有8个byte没用的，直接跳过了
        let expression_data = &input[8..(expression_length * 4) as usize + 8];
        let input = &input[(expression_length * 4) as usize + 8..];
        let (_, axis) = convert_to_explicit_axis_string(expression_data, refno)?;
        let mut result: String = "".into();
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
        if (expression_length as usize * 4 + 8) > input.len() {
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        }
        let mut expression_data = &input[16..(expression_length * 4) as usize + 8];
        let input = &input[(expression_length * 4) as usize + 8..];
        // 表达式都是以0x0 0 0 1开头的
        let _expression_start = &expression_data[..4];
        expression_data = &expression_data[4..];
        let result = parse_expression_func(expression_data, refno)?.1;
        Ok((input, (expression_type, result.into())))
    }
}

pub fn parse_expression_func(input: &[u8], refno: RefI32Tuple) -> IResult<&[u8], String> {
    if input.len() < 8 {
        return Ok((input, "".to_string()));
    }
    // 表达式都是以0x0 0 0 1开头的
    let mut expression_data = &input[..];
    // 这是表达式数字的起始标志
    let mut result_stack = vec![];
    let mut check_val1 = parse_to_i32(&expression_data[..4]);
    let mut check_val2 = parse_to_i32(&expression_data[4..8]);
    let mut number_flag = check_val1 == 0x65;
    while expression_data.len() >= 8 && (number_flag || check_val1 == 0x6A || check_val2 == 3 || &expression_data[..3] == &[0x0, 0x0, 0x3] || check_val2 == 0x65) {
        let expression_const = parse_expression_const(&expression_data[..4]);
        if expression_const != "" {
            result_stack.push(expression_const);
            expression_data = &expression_data[4..];
            number_flag = true;
        }
        //解析数值
        if number_flag {
            expression_data = &expression_data[8..];
            if expression_data.len() < 12 {
                return Ok((input, "".to_string())); //todo 检查这种情况
            }
            let num_flag = parse_to_i16(&expression_data[8..10]);
            let value = if num_flag == 0i16 {
                parse_explicit_num_00(&expression_data[..12])?.1
            } else if num_flag == 0x4000i16 {
                parse_explicit_num_40(&expression_data[..12])?.1
            } else if num_flag == -1i16 {
                parse_explicit_num_ff(&expression_data[..12])?.1
            } else { 0.0 };
            // 表达式的值
            result_stack.push(value.to_string());
            expression_data = &expression_data[12..];
            // 表达式 值的结束位  这里是个结束位 结束位 00 00 00 00 00 00 00 06
            // 这里可能会出现没有结束位就结束的情况，所以加了一个长度判断
            if expression_data.len() > 8 {
                expression_data = &expression_data[8..];
            }
        }
        //若后面是6A 则代表该值没完
        while expression_data.len() > 4 && &expression_data[..4] == &[0x0u8, 0x0, 0x0, 0x6A][..] {
            // 跳6A
            expression_data = &expression_data[4..];
            let num = u32::from_be_bytes(expression_data[..4].try_into().unwrap());
            let att_name = db1_dehash(u32::from_be_bytes(expression_data[4..8].try_into().unwrap()));
            let flags = (parse_to_i32(&expression_data[8..12]), parse_to_i32(&expression_data[12..16]));
            let mut rpro_name = String::new();
            let s_value = u32::from_be_bytes(expression_data[16..20].try_into().unwrap());
            if att_name.as_str() == "RPRO" && s_value != 0 {
                rpro_name = db1_dehash(s_value);
                if !rpro_name.is_empty() {
                    rpro_name.insert(0, ' ');
                }
            }
            let mut expression;
            if flags == (-1, -1) {
                let v = result_stack.pop().unwrap_or_default();
                expression = format!("{att_name}[{v}]{rpro_name}");
            } else {
                let num = flags.1;
                if s_value == 0 {
                    if num == 1 && &att_name != "PARA" {
                        expression = format!("{att_name}");
                    } else {
                        expression = format!("{att_name}[{num}]");
                    }
                } else {
                    expression = format!("{att_name}{rpro_name}");
                }
            }
            result_stack.push(expression);

            // OF = 类型表达式解析，目前推测是这样 但不肯定
            if &expression_data[20..24] == &[0x0, 0x0, 0x6, 0x42] {
                let (tmp_input, length) = be_i32(&expression_data[24..])?;
                let length = length as usize * 4;
                let mut expression_input = &tmp_input[4..length];
                while expression_input.len() > 4 {
                    let expression = get_expression_of_func(&expression_input[..4]);
                    if expression != "".to_string() {
                        let func = result_stack.pop().unwrap_or_default();
                        let result = format!("{} OF {} ", func, expression);
                        result_stack.push(result);
                        if expression_input.len() < 20 {
                            // OF 后面可能还有其他表达式
                            break;
                        }
                        expression_input = &expression_input[20..];
                    }
                    let (expression_tmp, (refno0, refno1, )) = tuple((
                        be_i32,
                        be_i32,
                    ))(&expression_input[..])?;
                    expression_input = &expression_tmp[..];
                    if &expression != "" || refno0 == 0 || refno1 == 1701 { // 1701 是 0x 06 A5 代表表达式的结束
                        let expression = get_expression_of_func(&expression_input[..4]);
                        let func = result_stack.pop().unwrap_or_default();
                        let result = format!("{} OF {} ", func, expression);
                        result_stack.push(result);
                        //func后面有3个word的数据不知道是干什么的
                        expression_input = &expression_input[16..];
                    } else {
                        let refno = format!("{}/{}", refno0, refno1);
                        let func = result_stack.pop().unwrap_or_default();
                        let result = format!("({} OF = {})", func, refno);
                        // result_stack.push(result);
                        return Ok((input, result.into()));
                    }
                }
                if expression_data.len() > 28 + length {
                    expression_data = &expression_data[24 + length + 4..];
                } else {
                    expression_data = &expression_data[length..];
                }
            } else {
                // 跳过普通表达式的结束位
                expression_data = &expression_data[24..];
            }
        }
        // 这里表达式是结束了，但是可能会有后序表达式的运算符
        while expression_data.len() >= 4 && &expression_data[..2] == &[0x0, 0x0] && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x65]
            && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x6A] && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x2] {
            let mut symbol = String::new();

            let op_key = parse_to_i32(&expression_data[..4]);
            if MATH_OPERATORS_MAP.contains_key(&op_key) {
                let op_str = MATH_OPERATORS_MAP[&op_key];
                let cnt = op_str.matches("{}").count();
                let len = result_stack.len();
                if len >= cnt {
                    symbol = dynfmt::SimpleCurlyFormat.format(op_str, &result_stack[len - cnt..]).unwrap_or_default().to_string();
                    result_stack.drain(len - cnt..);
                }
            }
            match &expression_data[..4] {
                &[0x0, 0x0, 0x3, 0xF0] => {
                    if result_stack.len() > 1 {
                        let value1 = result_stack.pop().unwrap_or_default();
                        let value2 = result_stack.pop().unwrap_or_default();
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
                        symbol = format!("MAX ({})", max_array);
                    }
                }
                &[0x0, 0x0, 0x3, 0xF1] => {
                    if result_stack.len() > 1 {
                        let value1 = result_stack.pop().unwrap_or_default();
                        let value2 = result_stack.pop().unwrap_or_default();
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
                        symbol = format!("MIN ({})", max_array);
                    }
                }
                &[0x0, 0x0, 0x0, 0x3] => {
                    if expression_data.len() >= 8 && &expression_data[4..8] == &[0x0, 0x0, 0x6, 0xA5] {
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap_or_default();
                            let value2 = result_stack.pop().unwrap_or_default();
                            symbol = format!("{} OF = {}", value2, value1);
                        }
                    }
                }
                &[0, 0, 0, 0x6F] => {
                    symbol = "PI".to_string();
                }
                _ => {}
            }
            if symbol != "" {
                result_stack.push(symbol);
            }
            if expression_data.len() > 4 {
                expression_data = &expression_data[4..];
            } else {
                let result = format!("{}", result_stack.pop().unwrap_or_default());
                return Ok((input, result.into()));
            }
        }
        if expression_data.len() >= 8 {
            check_val1 = parse_to_i32(&expression_data[..4]);
            check_val2 = parse_to_i32(&expression_data[4..8]);
            number_flag = check_val1 == 0x65;
        } else {
            break;
        }
    }
    let result = format!("{}", result_stack.pop().unwrap_or("".to_string()).trim());
    return Ok((input, result));
}

/// 返回 X () Y () Z 表达式 的 其中一个 坐标 + data 例如： X ()
pub fn parse_xyz_data(input: &[u8], refno: RefI32Tuple) -> IResult<&[u8], String> {
    let coordinate = match_explicit_attribute_to_string(parse_to_u32(&input[..4]));
    let data_len = parse_to_u32(&input[4..8]) as usize;
    let data = parse_expression_func(&input[12..(data_len + 1) * 4], refno)?.1;
    Ok((&input[data_len * 4 + 4..], format!("{} ({}) ", coordinate, data)))
}

/// 返回PARA类的函数名
pub fn get_expression_func_name(input: &[u8]) -> IResult<&[u8], String> {
    let mut result = "".to_string();
    let (_, v) = be_u32(&input[4..8])?;
    if v > 0x81BF1 {
        result = format!("{}", db1_dehash(v));
    }
    Ok((input, result))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_explicit_num_00(data: &[u8]) -> IResult<&[u8], f64> {
    let (_, times) = be_i16(&data[10..12])?;
    let times = 2_f32.powf((5i16 - times) as f32) as f64;
    let (_, a) = be_i32(&data[..4])?;
    let (_, b) = be_i32(&data[4..8])?;
    let value = (((a as f64 / 0x400 as f64) + (b as f64 / 0x20000000 as f64)) / times * 1000.0).round() / 1000.0;
    let value = f64_round_3(value);
    Ok((data, value))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_explicit_num_40(data: &[u8]) -> IResult<&[u8], f64> {
    let mut dst_data = data[..8].to_vec();
    let dst_first = (data[10] & 0xF).checked_shl(4).unwrap() + (data[11] & 0xF0).checked_shr(4).unwrap();
    dst_data[0] = dst_first;
    dst_data[1] = (data[11] & 0xF).checked_shl(4).unwrap() + (data[1] & 0xF);
    let value = if data[0] == 0x40 {
        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
        -value
    } else {
        f64::from_be_bytes(dst_data.try_into().unwrap())
    };
    let value = f64_round_3(value);
    Ok((data, value))
}

pub fn parse_expression_const(input: &[u8]) -> String {
    match input {
        &[0, 0, 0, 0x6F] => { "PI".to_string() }
        &_ => { "".to_string() }
    }
}

#[test]
fn test_parse_explicit_num_40() {
    // let data = [0x40u8, 0x06, 0x80, 0x0, 0x0, 0, 0, 0, 0x40, 0, 4, 4];
    let data = [0x0u8, 0x06, 0x80, 0x0, 0x0, 0, 0, 0, 0x40, 0, 4, 4];
    let data = parse_explicit_num_40(&data[..]).unwrap().1;
    println!("data={:?}", data);
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_explicit_num_ff(data: &[u8]) -> IResult<&[u8], f64> {
    let a = parse_to_i32(&data[..4]);
    //40 00 00 00 代表 0.5
    let b = parse_to_i32(&data[4..8]);
    let v = (a as f64 / 0x10024 as f64) + b as f64 / 0x40000000 as f64 * 0.5;
    let c = 0xFFFFu32 - parse_to_u16(&data[10..12]) as u32;   //parse like 0xFF FE
    let div_times = 2_i32.pow(c);
    let v = (v * 1000.0).round() / (div_times as f64) / 1000.0;
    let value = f64_round_3(v);
    Ok((data, value))
}

#[inline]
pub fn get_expression_of_func(input: &[u8]) -> String {
    let mut result = "".to_string();
    match input {
        &[0, 0, 0, 0xA] => {
            result = "PREV".to_string()
        }
        &[0, 0, 0, 0xB] => {
            result = "NEXT".to_string()
        }
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

#[inline]
pub fn times_keep_f32_three_decimal_place(input: i32) -> f32 {
    let input = input as f32;
    let result = input / 40.0f32 * 1000.0;
    let b_seven = result as i32 % 10 == 7 && result < 100.0;
    let mut result = result;
    if b_seven {
        result = f32::trunc(result) / 1000.0;
    } else {
        result = result.round() / 1000.0;
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





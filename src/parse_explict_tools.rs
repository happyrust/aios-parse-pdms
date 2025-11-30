use crate::parse::{convert_to_explicit_axis_string, match_axis};
use crate::parser::attribute::explicit::get_explicit_attr_type;
use crate::BHashMap;
use aios_core::helper::{parse_to_i16, parse_to_i32, parse_to_u16, parse_to_u32, parse_to_f64};
use aios_core::pdms_types::DbAttributeType::*;
use aios_core::pdms_types::{DbAttributeType, RefI32Tuple};
use aios_core::tool::db_tool::{convert_to_hash, db1_dehash, is_uda};
use aios_core::tool::float_tool::f64_round_3;
use aios_core::{AttrVal::*, RefU64};
use dashmap::DashMap;
use dynfmt::Format;
use nom::multi::count;
use nom::number::complete::{be_i16, be_i32, be_u16, be_u32, be_u8};
use nom::sequence::tuple;
use nom::IResult;
use nom::Parser;
use std::fs::File;
use std::io::BufReader;
use aios_core::bin_data::convert_str_to_bytes;
use log::error;
use pretty_hex::pretty_hex;
use tokio::count;

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
        s.insert(0x191,"{} EQ {}");
        s.insert(0x1F5,"{} NEQ {}");
        s.insert(0x259, "{} GT {}");
        // 通过该 000 文件的二进制数据 25A 也是 GT ， 不知道是不是这两个数字都代表 GT ，下同
        s.insert(0x25B, "{} LT {}");
        s.insert(0x25D, "{} GE {}");
        s.insert(0x25F, "{} LE {}");
        s.insert(0x321, "( -{} )");
s.insert(0x322, "( {} + {} )");
s.insert(0x323, "( {} - {} )");
s.insert(0x324, "( {} * {} )");
s.insert(0x325, "( {} / {} )");
s.insert(0x3E9, "SQRT( {} )");
s.insert(0x385, "SIN( {} )");
s.insert(0x386, "COS( {} )");
s.insert(0x387, "TAN( {} )");
s.insert(0x388, "ASIN( {} )");
s.insert(0x389, "ACOS( {} )");
s.insert(0x38A, "ATAN( {} )");
s.insert(0x38B, "ATANT( {}, {} )");
s.insert(0x3EA, "POW( {}, {} )");
s.insert(0x3EB, "LOG( {} )");
s.insert(0x3EC, "ALOG( {} )");
s.insert(0x3ED, "INT( {} )");
s.insert(0x3EE, "NINT( {} )");
s.insert(0x3EF, "ABS( {} )");
s.insert(0x515, "LEN( '{}' )");
s.insert(0x51C, "MAT( {}, '{}' )");
s.insert(0x522, "TRIM( {} )");
s.insert(0x529, "OCCUR( '{}', '{}' )");
s.insert(0x579, "REAL( '{}' )");
s.insert(0x582, "STR( {} )");
        // s.insert(0x3F0, "MAX ({},{})");   //特殊处理
        // s.insert(0x3F1, "MIN ({},{})");
        s
    };
}

// get_explicit_attr_type 已迁移到 parser::attribute::explicit 模块

/// 解析轴向表达式   
/// 轴向表达式处理总长度0x （4 * 4）= 16 字节： 1C 00 00 03 00 00 00 02 00 00 00 01 00 00 00 02
/// input[0..4]: 1C 00 00 03 => 标识轴向表达式
/// input[4..8]: 00 00 00 02 => 正负轴的表达式类型
/// input[8..12]: 00 00 00 01 => 正负标志（1为正，2为负）
/// input[12..16]: 00 00 00 02 => 轴索引（1=X, 2=Y, 3=Z）
pub fn parse_axis_expression(input: &[u8], expression_type: String) -> IResult<&[u8], (String, String)> {
    if input.len() < 16 {
        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
    }
    
    // 解析正负标志 (input[8..12])
    let positive_flag = parse_to_i32(&input[8..12]);
    // 解析轴索引 (input[12..16])
    let axis_index = parse_to_i32(&input[12..16]);
    
    let axis = match axis_index {
        1 => "X",
        2 => "Y", 
        3 => "Z",
        _ => "UNKNOWN",
    };
    
    let axis_result = if positive_flag == 1 {
        axis.to_string()
    } else if positive_flag == 2 {
        format!("-{}", axis)
    } else {
        axis.to_string()
    };
    
    let remaining = &input[16..];
    Ok((remaining, (expression_type, axis_result)))
}

/// 解析其他类型的表达式（原有逻辑）
pub fn parse_other_expression(input: &[u8], expression_type: String, refno: RefU64) -> IResult<&[u8], (String, String)> {
    //临时处理，后面需要总结规律
    if input.len() <= 4 * 5 {
        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
    }

    let (_, flag) = be_i32(&input[4 * 4..4 * 5])?;

    //string type
    if flag == 0x66 {
        let (_, str_len) = be_i32(&input[4 * 5..4 * 6])?;
        let (input, chars) = count(be_i32, str_len as usize).parse(&input[4 * 6..])?;
        let string = format!("'{}'", chars.iter().map(|c| *c as u8 as char).collect::<String>());
        return Ok((input, (expression_type, string)));
    }

    if expression_type == "PTCDI" || expression_type == "PTCD" {
        let (_, expression_length) = be_u16(&input[2..4])?;

        // 显式属性的length后有4个byte没用的，直接跳过了
        let end = (expression_length * 4) as usize + 4;

        //暂时跳过这个问题，长度问题
        if end > input.len() {
            error!("{refno}: parse_expression_attr PTCDI or PTCD {end} > {} input.len()",  input.len());
            error!("{:#4X?}", input);
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        }
        let expression_data = &input[4..end];
        let input = &input[end..];

        let (_, axis) = convert_to_explicit_axis_string(expression_data, refno)?;
        let result = match axis {
            StringType(value) => value,
            _ => "".to_string(),
        };

        Ok((input, (expression_type, result)))
    } else {
        let (_, expression_length) = be_u16(&input[2..4])?;

        // 显式属性的length后有4个byte没用的，直接跳过了
        if (expression_length as usize * 4 + 4) > input.len() {
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        }
        let end = (expression_length * 4) as usize + 4;
        //todo 需要检查
        if end <= 12 {
            // dbg!("Found expression length less than 12 bytes, skipping...{refno}");
            // println!("Debug expression data {:#4X?}", &input[12..]);
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        }
        let expression_data = &input[12..(expression_length * 4) as usize + 4];
        let flag1 = parse_to_i32(&input[4..8]);
        let flag2 = parse_to_i32(&input[8..12]);
        let flag3 = parse_to_i32(&expression_data[..4]);
        let input = &input[(expression_length * 4) as usize + 4..];

        if flag1 == 2 && flag2 == 1 {
            let axis = match flag3 {
                1 => "X",
                2 => "Y",
                3 => "Z",
                _ => "",
            };
            return Ok((input, (expression_type, axis.into())));
        }

        let expr_data = &expression_data[4..];

        if flag2 == 0x28 && expr_data.len() == 2 * 4 {
            let u32_value = parse_to_i32(&expr_data[..4]).abs() / 10;
            return Ok((input, (expression_type, u32_value.to_string())));
        }

        let result = parse_expression_func(expr_data, refno)?.1;
        Ok((input, (expression_type, result.into())))
    }
}

/// 判断是否为轴向表达式
fn is_axis_expression(input: &[u8]) -> bool {
    // 检查是否有足够的数据
    if input.len() < 16 {
        return false;
    }
    
    // 检查轴向表达式的特征
    // 根据注释，轴向表达式有特定的格式
    // input[0..4]: 1C 00 00 03 标识轴向表达式
    let identifier = parse_to_i32(&input[0..4]);
    if identifier != 0x1C000003u32 as i32 {
        return false;
    }
    
    // 检查轴索引是否有效 (input[12..16])
    let axis_index = parse_to_i32(&input[12..16]);
    axis_index >= 1 && axis_index <= 3
}

/// 解析表达式
pub fn parse_expression_attr(input: &[u8], refno: RefU64) -> IResult<&[u8], (String, String)> {
    let hash_val = &input[..4];
    let expression_type = db1_dehash(convert_to_hash(hash_val).abs() as _);
    let input = &input[4..];
    
    // 根据表达式类型和数据特征分发到不同的处理函数
    if is_axis_expression(input) {
        parse_axis_expression(input, expression_type)
    } else {
        parse_other_expression(input, expression_type, refno)
    }
}

pub fn parse_expression_func(input: &[u8], refno: RefU64) -> IResult<&[u8], String> {
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
    // 0x76 就是以字符串的标志
    while expression_data.len() >= 8
        && (number_flag
        || check_val1 == 0x6A
        || check_val2 == 3
        || &expression_data[..3] == &[0x0, 0x0, 0x3]
        || check_val2 == 0x65
        || check_val1 == 0x76)
    {
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
                parse_explicit_f64_40(&expression_data[..12])?.1
            } else if num_flag == -1i16 {
                parse_explicit_num_ff(&expression_data[..12])?.1
            } else {
                0.0
            };
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
            let hash_num = u32::from_be_bytes(expression_data[4..8].try_into().unwrap());
            let att_name = if is_uda(hash_num as _) {
                let uda_name = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        if let Some(uda_name) = aios_core::get_uda_name(hash_num as _).await {
                            // dbg!(hash_val, uda_refno);
                            format!(":{uda_name}")
                            //要加UDA:表达区分
                        } else {
                            db1_dehash(hash_num)
                        }
                    })
                });
                uda_name
            } else {
                db1_dehash(hash_num)
            };
            let new_att_name = format!("ATTRIB {att_name}");
            // let att_name = db1_dehash(hash_num);
            // if is_debug {
            //     // dbg!((&att_name, hash_num));
            // }
            let flags = (
                parse_to_i32(&expression_data[8..12]),
                parse_to_i32(&expression_data[12..16]),
            );
            let mut rpro_name = String::new();
            let s_value = u32::from_be_bytes(expression_data[16..20].try_into().unwrap());
            if att_name.as_str() == "RPRO" && s_value != 0 {
                rpro_name = db1_dehash(s_value);
                if !rpro_name.is_empty() {
                    rpro_name.insert(0, ' ');
                }
            }
            let expression;
            if flags == (-1, -1) {
                let v = result_stack.pop().unwrap_or_default();
                expression = format!("{new_att_name}[{v} ]{rpro_name}");
            } else {
                let num = flags.1;
                if s_value == 0 {
                    if num == 1 && &att_name != "PARA" {
                        expression = format!("{new_att_name}");
                    } else {
                        expression = format!("{new_att_name}[{num} ]");
                    }
                } else {
                    expression = format!("{new_att_name}{rpro_name}");
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
                        let result = format!(" ( {} OF {} ) ", func, expression);
                        result_stack.push(result);
                        if expression_input.len() < 20 {
                            // OF 后面可能还有其他表达式
                            break;
                        }
                        expression_input = &expression_input[20..];
                    }
                    let (expression_tmp, (refno0, refno1)) =
                        tuple((be_i32, be_i32))(&expression_input[..])?;
                    expression_input = &expression_tmp[..];
                    if &expression != "" || refno0 == 0 || refno1 == 1701 {
                        // 1701 是 0x 06 A5 代表表达式的结束
                        let expression = get_expression_of_func(&expression_input[..4]);
                        let func = result_stack.pop().unwrap_or_default();
                        let result = format!(" ( {} OF {} ) ", func, expression);
                        result_stack.push(result);
                        //func后面有3个word的数据不知道是干什么的
                        expression_input = &expression_input[16..];
                    } else {
                        let refno = format!(" {} / {} ", refno0, refno1);
                        let func = result_stack.pop().unwrap_or_default();
                        let result = format!(" ( {} OF = {} ) ", func, refno);
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
        while expression_data.len() >= 4
            && &expression_data[..2] == &[0x0, 0x0]
            && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x65]
            && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x6A]
            && &expression_data[..4] != &[0x0, 0x0, 0x0, 0x2]
        {
            let mut symbol = String::new();
            let op_key = parse_to_i32(&expression_data[..4]);
            if MATH_OPERATORS_MAP.contains_key(&op_key) {
                let op_str = MATH_OPERATORS_MAP[&op_key];
                let cnt = op_str.matches("{}").count();
                let len = result_stack.len();
                if len >= cnt {
                    symbol = dynfmt::SimpleCurlyFormat
                        .format(op_str, &result_stack[len - cnt..])
                        .unwrap_or_default()
                        .to_string();
                    result_stack.drain(len - cnt..);
                }
            }
            // println!("{:#4X?}",&expression_data[..4]);
            match &expression_data[..4] {
                &[0x0, 0x0, 0x0, 0x3] => {
                    if expression_data.len() >= 8
                        && &expression_data[4..8] == &[0x0, 0x0, 0x6, 0xA5]
                    {
                        if result_stack.len() > 1 {
                            let value1 = result_stack.pop().unwrap_or_default();
                            let value2 = result_stack.pop().unwrap_or_default();
                            symbol = format!(" ( {} OF = {} ) ", value2, value1);
                        }
                    }
                }
                &[0, 0, 0, 0x6F] => {
                    symbol = "PI".to_string();
                }
                // 字符串
                &[0x0, 0x0, 0x0, 0x76] => {
                    if expression_data.len() > 4 {
                        // 第一个是字符串的长度
                        let len = parse_to_u32(&expression_data[4..8]);
                        let mut chars = vec![];
                        if expression_data.len() >= (len * 4 + 4) as usize {
                            for i in 0..len {
                                let start = (8 + i * 4) as usize;
                                let c = parse_to_u32(&expression_data[start..start + 4]) as u8;
                                chars.push(c);
                            }
                        }
                        let end = (4 + len * 4) as usize;
                        expression_data = &expression_data[end..];
                        symbol = String::from_utf8_lossy(&chars).to_string();
                    }
                }
                &[0x0, 0x0, 0x3, 0xF0] => {
                    if result_stack.len() > 1 {
                        let value1 = result_stack.pop().unwrap_or_default();
                        let value2 = result_stack.pop().unwrap_or_default();
                        let mut max_array = format!("{},{}", value2, value1);
                        while expression_data.len() > 36
                            && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF0]
                        {
                            let expression_data_value = &expression_data[12..24];
                            let mut dst_data = expression_data_value[..8].to_vec();
                            let dst_first =
                                (expression_data_value[10] & 0xF).checked_shl(4).unwrap()
                                    + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                            dst_data[0] = dst_first;
                            dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap()
                                + (expression_data_value[1] & 0xF);
                            let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                            max_array = format!("{},{}", max_array, value);
                            expression_data = &expression_data[32..];
                        }
                        symbol = format!(" ( MAX ({}) ) ", max_array);
                    }
                }
                &[0x0, 0x0, 0x3, 0xF1] => {
                    if result_stack.len() > 1 {
                        let value1 = result_stack.pop().unwrap_or_default();
                        let value2 = result_stack.pop().unwrap_or_default();
                        let mut max_array = format!("{},{}", value2, value1);
                        while expression_data.len() > 36
                            && &expression_data[32..36] == &[0x0, 0x0, 0x3, 0xF1]
                        {
                            let expression_data_value = &expression_data[12..24];
                            let mut dst_data = expression_data_value[..8].to_vec();
                            let dst_first =
                                (expression_data_value[10] & 0xF).checked_shl(4).unwrap()
                                    + (expression_data_value[11] & 0xF0).checked_shr(4).unwrap();
                            dst_data[0] = dst_first;
                            dst_data[1] = (expression_data_value[11] & 0xF).checked_shl(4).unwrap()
                                + (expression_data_value[1] & 0xF);
                            let value = f64::from_be_bytes(dst_data.try_into().unwrap());
                            max_array = format!("{},{}", max_array, value);
                            expression_data = &expression_data[32..];
                        }
                        symbol = format!(" ( MIN ( {} ) ) ", max_array);
                    }
                }
                &[0x0, 0x0, 0x7, 0x1E] => {
                    if result_stack.len() > 2 {
                        let value1 = result_stack.pop().unwrap_or_default();
                        let value2 = result_stack.pop().unwrap_or_default();
                        let value3 = result_stack.pop().unwrap_or_default();
                        symbol = format!(" ( IFTRUE ( {} , {} , {} ) ) ", value3, value2, value1);
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
pub fn parse_xyz_data(input: &[u8], refno: RefU64, convert: bool) -> IResult<&[u8], String> {
    // println!("input bytes: {}", pretty_hex(&input));
    let axis = match_axis(parse_to_u32(&input[..4]));
    let data_len = parse_to_u32(&input[4..8]) as usize;
    let data = parse_expression_func(&input[12..(data_len + 1) * 4], refno)?.1;
    let result = if convert && axis.starts_with("-") {
        format!("{} (NEG ( {} )) ", &axis[1..], data)
    } else {
        format!("{} ( {} ) ", axis, data)
    };
    Ok((
        &input[data_len * 4 + 4..],
        result
    ))
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
    let value = (((a as f64 / 0x400 as f64) + (b as f64 / 0x20000000 as f64)) / times * 1000.0)
        .round()
        / 1000.0;
    let value = f64_round_3(value);
    Ok((data, value))
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_explicit_f64_40(data: &[u8]) -> IResult<&[u8], f64> {
    let mut dst_data = data[..8].to_vec();
    let dst_first =
        (data[10] & 0xF).checked_shl(4).unwrap() + (data[11] & 0xF0).checked_shr(4).unwrap();
    dst_data[0] = dst_first;
    dst_data[1] = (data[11] & 0xF).checked_shl(4).unwrap() + (data[1] & 0xF);
    // println!("data={:?}", pretty_hex(&dst_data));
    let value = if data[0] == 0x40 {
        let value = f64::from_be_bytes(dst_data.try_into().unwrap());
        -value
    } else {
        f64::from_be_bytes(dst_data.try_into().unwrap())
    };
    let value = f64_round_3(value);
    Ok((data, value))
}

#[test]
fn parse_axis_f32() {
    let data_str = "40 14 00 00 00 00 00 00 40 00 04 03";
    let bytes = convert_str_to_bytes(data_str);
    let (_, value) = parse_explicit_f64_40(&bytes).unwrap();
    dbg!(value);
}

pub fn parse_expression_const(input: &[u8]) -> String {
    match input {
        &[0, 0, 0, 0x6F] => "PI".to_string(),
        &_ => "".to_string(),
    }
}

#[test]
fn test_parse_explicit_num_40() {
    // let data = [0x40u8, 0x06, 0x80, 0x0, 0x0, 0, 0, 0, 0x40, 0, 4, 4];
    let data = [0x0u8, 0x06, 0x80, 0x0, 0x0, 0, 0, 0, 0x40, 0, 4, 4];
    let data = parse_explicit_f64_40(&data[..]).unwrap().1;
    println!("data={:?}", data);
}

/// 解析axis显式属性的值，分为00 40 FF三种
pub fn parse_explicit_num_ff(data: &[u8]) -> IResult<&[u8], f64> {
    let a = parse_to_i32(&data[..4]);
    //40 00 00 00 代表 0.5
    let b = parse_to_i32(&data[4..8]);
    let v = (a as f64 / 0x10024 as f64) + b as f64 / 0x40000000 as f64 * 0.5;
    let c = 0xFFFFu32 - parse_to_u16(&data[10..12]) as u32; //parse like 0xFF FE
    let div_times = 2_i32.pow(c);
    let v = (v * 1000.0).round() / (div_times as f64) / 1000.0;
    let value = f64_round_3(v);
    Ok((data, value))
}

#[inline]
pub fn get_expression_of_func(input: &[u8]) -> String {
    let mut result = "".to_string();
    match input {
        &[0, 0, 0, 0xA] => result = "PREV".to_string(),
        &[0, 0, 0, 0xB] => result = "NEXT".to_string(),
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
    let database_info: DashMap<String, Vec<(String, String)>> =
        serde_json::from_reader(reader).unwrap();
    println!("value={:?}", database_info);
}


//! 表达式解析器
//!
//! 提供 PDMS 表达式属性的解析功能，包括：
//! - 数学运算表达式
//! - 函数调用表达式
//! - 属性引用表达式
//! - 常量表达式

use crate::parser::numeric::{parse_explicit_f64_40, parse_explicit_num_00, parse_explicit_num_ff};
use aios_core::helper::parse_to_i16;
use aios_core::tool::db_tool::{convert_to_hash, db1_dehash};
use nom::bytes::complete::take;
use nom::number::complete::be_i32;
use nom::IResult;
use nom::Parser;
use std::collections::HashMap;

use super::axis::{is_axis_expression, parse_axis_expression_str};

/// 数学运算符映射表
///
/// 将 PDMS 内部操作码映射为可读的表达式格式
pub fn get_math_operators() -> HashMap<i32, &'static str> {
    let mut map = HashMap::new();
    // 比较运算符
    map.insert(0x191, "{} EQ {}");
    map.insert(0x1F5, "{} NEQ {}");
    map.insert(0x259, "{} GT {}");
    map.insert(0x25B, "{} LT {}");
    map.insert(0x25D, "{} GE {}");
    map.insert(0x25F, "{} LE {}");
    // 算术运算符
    map.insert(0x321, "( -{} )");
    map.insert(0x322, "( {} + {} )");
    map.insert(0x323, "( {} - {} )");
    map.insert(0x324, "( {} * {} )");
    map.insert(0x325, "( {} / {} )");
    // 数学函数
    map.insert(0x3E9, "SQRT( {} )");
    map.insert(0x385, "SIN( {} )");
    map.insert(0x386, "COS( {} )");
    map.insert(0x387, "TAN( {} )");
    map.insert(0x388, "ASIN( {} )");
    map.insert(0x389, "ACOS( {} )");
    map.insert(0x38A, "ATAN( {} )");
    map.insert(0x38B, "ATANT( {}, {} )");
    map.insert(0x3EA, "POW( {}, {} )");
    map.insert(0x3EB, "LOG( {} )");
    map.insert(0x3EC, "ALOG( {} )");
    map.insert(0x3ED, "INT( {} )");
    map.insert(0x3EE, "NINT( {} )");
    map.insert(0x3EF, "ABS( {} )");
    // 字符串函数
    map.insert(0x515, "LEN( '{}' )");
    map.insert(0x51C, "MAT( {}, '{}' )");
    map.insert(0x522, "TRIM( {} )");
    map.insert(0x529, "OCCUR( '{}', '{}' )");
    map.insert(0x579, "REAL( '{}' )");
    map.insert(0x582, "STR( {} )");
    map
}

/// 特殊常量映射
pub fn parse_expression_const(input: &[u8]) -> &'static str {
    if input.len() < 4 {
        return "";
    }
    match input[..4] {
        [0, 0, 0, 0x6F] => "PI",
        _ => "",
    }
}

/// 特殊函数表达式
pub fn get_expression_of_func(input: &[u8]) -> &'static str {
    if input.len() < 4 {
        return "";
    }
    match input[..4] {
        [0, 0, 0, 0xA] => "PREV",
        [0, 0, 0, 0xB] => "NEXT",
        [0x0, 0xA, 0x1D, 0xCB] => "BLRF NUM 1",
        [0x0, 0xD, 0xBC, 0xF9] => "CATR",
        [0x0, 0xD, 0x24, 0x5B] => "BLTP 1",
        _ => "",
    }
}

/// 解析表达式属性
///
/// # 格式
/// - input[0..4]: hash 值，确定表达式类型
/// - input[4..]: 表达式具体数据
///
/// # 返回
/// `(表达式类型名称, 表达式值字符串)`
pub fn parse_expression_attr(input: &[u8], refno: u64) -> IResult<&[u8], (String, String)> {
    let (input, hash_val) = take(4usize).parse(input)?;
    let expression_type = db1_dehash(convert_to_hash(hash_val).unsigned_abs());

    // 判断是否为轴向表达式
    if is_axis_expression(input)? {
        parse_axis_expression_str(input, expression_type)
    } else {
        parse_other_expression(input, expression_type, refno)
    }
}

/// 解析非轴向表达式
///
/// 支持的类型：
/// - 字符串类型 (flag == 0x66)
/// - PTCDI/PTCD 类型
/// - 通用表达式类型
pub fn parse_other_expression(
    input: &[u8],
    expression_type: String,
    _refno: u64,
) -> IResult<&[u8], (String, String)> {
    // 检查最小长度
    if input.len() < 20 {
        return Ok((input, (expression_type, String::new())));
    }

    // 检查是否为字符串类型 (flag at offset 16 == 0x66)
    let (_, flag) = be_i32(&input[16..20])?;
    if flag == 0x66 {
        return parse_string_expression(input, expression_type);
    }

    // PTCDI/PTCD 类型处理
    if expression_type == "PTCDI" || expression_type == "PTCD" {
        return parse_ptcd_expression(input, expression_type);
    }

    // 通用表达式处理
    parse_general_expression(input, expression_type)
}

/// 解析字符串表达式
fn parse_string_expression(
    input: &[u8],
    expression_type: String,
) -> IResult<&[u8], (String, String)> {
    if input.len() < 24 {
        return Ok((input, (expression_type, String::new())));
    }

    let (_, str_len) = be_i32(&input[20..24])?;
    let str_len = str_len as usize;

    if input.len() < 24 + str_len * 4 {
        return Ok((input, (expression_type, String::new())));
    }

    let string: String = input[24..24 + str_len * 4]
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                let val = i32::from_be_bytes(chunk.try_into().unwrap());
                Some(val as u8 as char)
            } else {
                None
            }
        })
        .collect();

    let result = format!("'{}'", string);
    Ok((&input[24 + str_len * 4..], (expression_type, result)))
}

/// 解析 PTCD/PTCDI 表达式
fn parse_ptcd_expression(
    input: &[u8],
    expression_type: String,
) -> IResult<&[u8], (String, String)> {
    if input.len() < 8 {
        return Ok((input, (expression_type, String::new())));
    }

    // 读取表达式长度
    let (_, expression_length) = nom::number::complete::be_u16(&input[2..4])?;
    let data_size = (expression_length as usize) * 4;

    if input.len() < 8 + data_size {
        return Ok((input, (expression_type, String::new())));
    }

    // 跳过头部，解析轴向数据
    let expression_data = &input[8..8 + data_size];
    let axis_result = parse_axis_data(expression_data);

    Ok((&input[8 + data_size..], (expression_type, axis_result)))
}

/// 解析通用表达式
fn parse_general_expression(
    input: &[u8],
    expression_type: String,
) -> IResult<&[u8], (String, String)> {
    if input.len() < 16 {
        return Ok((input, (expression_type, String::new())));
    }

    // 读取控制标志
    let (_, flag1) = be_i32(&input[4..8])?;
    let (_, flag2) = be_i32(&input[8..12])?;

    // 轴向快速模式
    if flag1 == 2 && flag2 == 1 {
        let axis_index = if input.len() >= 16 {
            i32::from_be_bytes(input[12..16].try_into().unwrap_or([0; 4]))
        } else {
            0
        };
        let axis = match axis_index {
            1 => "X",
            2 => "Y",
            3 => "Z",
            _ => "",
        };
        return Ok((input, (expression_type, axis.to_string())));
    }

    // 数值模式
    if flag2 == 0x28 && input.len() >= 24 {
        let num_flag = parse_to_i16(&input[20..22]);
        let value = match num_flag {
            0 => parse_explicit_num_00(&input[12..24]).map(|(_, v)| v).unwrap_or(0.0),
            0x4000 => parse_explicit_f64_40(&input[12..24]).map(|(_, v)| v).unwrap_or(0.0),
            -1 => parse_explicit_num_ff(&input[12..24]).map(|(_, v)| v).unwrap_or(0.0),
            _ => 0.0,
        };
        return Ok((input, (expression_type, value.to_string())));
    }

    // 默认返回空字符串
    Ok((input, (expression_type, String::new())))
}

/// 解析轴向数据
fn parse_axis_data(data: &[u8]) -> String {
    let mut result = String::new();

    if data.len() >= 12 {
        // 尝试解析 X/Y/Z 值
        let values: Vec<f64> = data
            .chunks(4)
            .take(3)
            .filter_map(|chunk| {
                if chunk.len() == 4 {
                    Some(f32::from_be_bytes(chunk.try_into().unwrap()) as f64)
                } else {
                    None
                }
            })
            .collect();

        if values.len() == 3 {
            let parts: Vec<String> = ["X", "Y", "Z"]
                .iter()
                .zip(values.iter())
                .filter(|(_, v)| v.abs() > 1e-10)
                .map(|(&axis, &v)| format!("{}{}", axis, v))
                .collect();
            result = parts.join(" ");
        }
    }

    result
}

/// 解析表达式中的数值
///
/// 根据标志位选择合适的解析器
pub fn parse_expression_number(input: &[u8]) -> IResult<&[u8], f64> {
    if input.len() < 12 {
        return Err(nom::Err::Incomplete(nom::Needed::new(12 - input.len())));
    }

    let num_flag = parse_to_i16(&input[8..10]);
    match num_flag {
        0 => parse_explicit_num_00(input),
        0x4000 => parse_explicit_f64_40(input),
        -1 => parse_explicit_num_ff(input),
        _ => Ok((input, 0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_math_operators() {
        let ops = get_math_operators();
        assert_eq!(ops.get(&0x322), Some(&"( {} + {} )"));
        assert_eq!(ops.get(&0x324), Some(&"( {} * {} )"));
        assert_eq!(ops.get(&0x385), Some(&"SIN( {} )"));
    }

    #[test]
    fn test_parse_expression_const() {
        assert_eq!(parse_expression_const(&[0, 0, 0, 0x6F]), "PI");
        assert_eq!(parse_expression_const(&[0, 0, 0, 0]), "");
        assert_eq!(parse_expression_const(&[0, 0]), "");
    }

    #[test]
    fn test_get_expression_of_func() {
        assert_eq!(get_expression_of_func(&[0, 0, 0, 0xA]), "PREV");
        assert_eq!(get_expression_of_func(&[0, 0, 0, 0xB]), "NEXT");
        assert_eq!(get_expression_of_func(&[0, 0, 0, 0]), "");
    }

    #[test]
    fn test_parse_axis_data() {
        // X=1.0, Y=0, Z=0
        let data = [
            0x3F, 0x80, 0x00, 0x00, // 1.0 as f32
            0x00, 0x00, 0x00, 0x00, // 0.0
            0x00, 0x00, 0x00, 0x00, // 0.0
        ];
        let result = parse_axis_data(&data);
        assert!(result.contains("X"));
    }
}

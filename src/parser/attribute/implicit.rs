//! 隐式属性解析器
//!
//! 提供 PDMS 隐式属性的解析功能，包括：
//! - 基于偏移量的属性定位
//! - 多种数据类型解析（整数、浮点、字符串、引用号等）
//! - 特殊属性处理（LEVEL、PTS、表达式等）
//! - f32/f64 混合模式支持

use aios_core::pdms_types::DbAttributeType;
use aios_core::types::NamedAttrValue;
use glam::Vec3;
use nom::IResult;

/// 隐式属性偏移信息
#[derive(Debug, Clone)]
pub struct ImplicitAttrOffset {
    /// 属性名称
    pub name: String,
    /// 偏移量（低16位为word位置，高12位为特殊标志）
    pub offset: u32,
    /// 属性类型
    pub attr_type: DbAttributeType,
}

/// 解析隐式属性值
///
/// # 参数
/// - `input`: 隐式数据块（已分段合并）
/// - `attr_info`: 属性元数据信息
/// - `is_f32`: 是否使用 f32 模式
/// - `f32_neg_offset`: f32 模式的负偏移量
/// - `step`: 当前解析步骤（用于调试）
///
/// # 返回
/// `IResult<&[u8], NamedAttrValue>` - 解析后的属性值
pub fn parse_implicit_attr_value<'a>(
    input: &'a [u8],
    attr_info: &ImplicitAttrOffset,
    is_f32: bool,
    f32_neg_offset: usize,
    _step: usize,
) -> IResult<&'a [u8], NamedAttrValue> {
    // 计算实际偏移位置
    let offset_words = (attr_info.offset & 0xFFFF) as usize;
    let offset_bytes = offset_words * 4;

    // 应用 f32 模式偏移调整
    let actual_offset = if is_f32 {
        offset_bytes.saturating_sub(f32_neg_offset * 4)
    } else {
        offset_bytes
    };

    // 确保数据范围有效
    if actual_offset >= input.len() {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let data = &input[actual_offset..];

    // 根据属性类型解析值
    match attr_info.attr_type {
        DbAttributeType::INTEGER => parse_integer(data),
        DbAttributeType::DOUBLE => parse_double(data, is_f32),
        DbAttributeType::BOOL => parse_bool(data, attr_info.offset),
        DbAttributeType::STRING => parse_string(data),
        DbAttributeType::ELEMENT => parse_refno(data),
        DbAttributeType::WORD => parse_word(data),
        DbAttributeType::Vec3Type => parse_vec3(data, is_f32),
        DbAttributeType::DOUBLEVEC => parse_double_array(data, is_f32),
        DbAttributeType::INTVEC => parse_int_array(data, &attr_info.name),
        _ => {
            // 未知类型，返回空值
            Ok((input, NamedAttrValue::InvalidType))
        }
    }
}

/// 解析整数类型
fn parse_integer(input: &[u8]) -> IResult<&[u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    let value = i32::from_be_bytes(input[..4].try_into().unwrap());
    Ok((input, NamedAttrValue::IntegerType(value)))
}

/// 解析浮点类型
fn parse_double(input: &[u8], is_f32: bool) -> IResult<&[u8], NamedAttrValue> {
    if is_f32 {
        if input.len() < 4 {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
        let value = f32::from_be_bytes(input[..4].try_into().unwrap());
        Ok((input, NamedAttrValue::F32Type(value)))
    } else {
        if input.len() < 8 {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
        let value = f64::from_be_bytes(input[..8].try_into().unwrap()) as f32;
        Ok((input, NamedAttrValue::F32Type(value)))
    }
}

/// 解析布尔类型（从 offset 高位提取位信息）
fn parse_bool(input: &[u8], offset: u32) -> IResult<&[u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    // 高12位用于位操作
    let bit_position = (offset >> 20) as u32;
    let value = i32::from_be_bytes(input[..4].try_into().unwrap());
    let bool_value = (value >> bit_position) & 1 == 1;

    Ok((input, NamedAttrValue::BoolType(bool_value)))
}

/// 解析字符串类型
fn parse_string(input: &[u8]) -> IResult<&[u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let len = i32::from_be_bytes(input[..4].try_into().unwrap()) as usize;
    let data_start = 4;
    let data_end = data_start + len * 4;

    if input.len() < data_end {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let string: String = input[data_start..data_end]
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

    Ok((input, NamedAttrValue::StringType(string)))
}

/// 解析 RefU64 类型
fn parse_refno(input: &[u8]) -> IResult<&[u8], NamedAttrValue> {
    use aios_core::types::RefU64;

    if input.len() < 8 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let refno = RefU64::from(&input[..8]);
    Ok((input, NamedAttrValue::RefU64Type(refno)))
}

/// 解析 Word 类型
fn parse_word(input: &[u8]) -> IResult<&[u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    use aios_core::tool::db_tool::db1_dehash;

    let value = i32::from_be_bytes(input[..4].try_into().unwrap());
    let word_str = db1_dehash(value.unsigned_abs());
    Ok((input, NamedAttrValue::WordType(word_str)))
}

/// 解析 Vec3 类型
fn parse_vec3(input: &[u8], is_f32: bool) -> IResult<&[u8], NamedAttrValue> {
    if is_f32 {
        if input.len() < 16 {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
        // 长度 + 3个 f32 值
        let _len = i32::from_be_bytes(input[..4].try_into().unwrap());
        let x = f32::from_be_bytes(input[4..8].try_into().unwrap());
        let y = f32::from_be_bytes(input[8..12].try_into().unwrap());
        let z = f32::from_be_bytes(input[12..16].try_into().unwrap());
        Ok((input, NamedAttrValue::Vec3Type(Vec3::new(x, y, z))))
    } else {
        if input.len() < 28 {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
        // 长度 + 3个 f64 值
        let _len = i32::from_be_bytes(input[..4].try_into().unwrap());
        let x = f64::from_be_bytes(input[4..12].try_into().unwrap()) as f32;
        let y = f64::from_be_bytes(input[12..20].try_into().unwrap()) as f32;
        let z = f64::from_be_bytes(input[20..28].try_into().unwrap()) as f32;
        Ok((input, NamedAttrValue::Vec3Type(Vec3::new(x, y, z))))
    }
}

/// 解析浮点数组类型
fn parse_double_array(input: &[u8], is_f32: bool) -> IResult<&[u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let count = i32::from_be_bytes(input[..4].try_into().unwrap()) as usize;
    let data_start = 4;

    if is_f32 {
        let data_end = data_start + count * 4;
        if input.len() < data_end {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        let values: Vec<f32> = input[data_start..data_end]
            .chunks(4)
            .filter_map(|chunk| {
                if chunk.len() == 4 {
                    Some(f32::from_be_bytes(chunk.try_into().unwrap()))
                } else {
                    None
                }
            })
            .collect();

        Ok((input, NamedAttrValue::F32VecType(values)))
    } else {
        let data_end = data_start + count * 8;
        if input.len() < data_end {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        let values: Vec<f32> = input[data_start..data_end]
            .chunks(8)
            .filter_map(|chunk| {
                if chunk.len() == 8 {
                    Some(f64::from_be_bytes(chunk.try_into().unwrap()) as f32)
                } else {
                    None
                }
            })
            .collect();

        Ok((input, NamedAttrValue::F32VecType(values)))
    }
}

/// 解析整数数组类型（特殊处理 LEVEL/PTS）
fn parse_int_array<'a>(input: &'a [u8], attr_name: &str) -> IResult<&'a [u8], NamedAttrValue> {
    if input.len() < 4 {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    // LEVEL 和 PTS 需要特殊处理计数
    let count = if attr_name == "LEVEL" || attr_name == "PTS" {
        // 从数据中读取实际的元素数量
        i32::from_be_bytes(input[..4].try_into().unwrap()) as usize
    } else {
        1
    };

    let data_start = 4;
    let data_end = data_start + count * 4;

    if input.len() < data_end {
        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }

    let values: Vec<i32> = input[data_start..data_end]
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                Some(i32::from_be_bytes(chunk.try_into().unwrap()))
            } else {
                None
            }
        })
        .collect();

    Ok((input, NamedAttrValue::IntArrayType(values)))
}

/// 检查属性是否为表达式类型
pub fn check_is_expr(hash: i32) -> bool {
    use aios_core::consts::EXPR_ATT_SET;
    EXPR_ATT_SET.contains(&hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer() {
        let data = [0x00, 0x00, 0x00, 0x2A]; // 42
        let (_, result) = parse_integer(&data).unwrap();
        assert!(matches!(result, NamedAttrValue::IntegerType(42)));
    }

    #[test]
    fn test_parse_double_f32() {
        let data = [0x40, 0x49, 0x0F, 0xDB]; // 3.14159 as f32
        let (_, result) = parse_double(&data, true).unwrap();
        match result {
            NamedAttrValue::F32Type(v) => {
                assert!((v - 3.14159).abs() < 0.001);
            }
            _ => panic!("Expected F32Type"),
        }
    }

    #[test]
    fn test_parse_bool() {
        let data = [0x00, 0x00, 0x00, 0x04]; // 第2位为1
        let offset = 2 << 20; // 位位置=2
        let (_, result) = parse_bool(&data, offset).unwrap();
        assert!(matches!(result, NamedAttrValue::BoolType(true)));
    }

    #[test]
    fn test_check_is_expr() {
        // 这需要实际的表达式哈希值来测试
        // 暂时跳过具体测试
    }
}

//! 新 parser 模块使用示例
//!
//! 本示例展示如何使用新的模块化解析器

use parse_pdms_db::parser::attribute::implicit::{
    check_is_expr, parse_implicit_attr_value, ImplicitAttrOffset,
};
use parse_pdms_db::parser::combinator::collect_segmented_payload;

use aios_core::pdms_types::DbAttributeType;
use aios_core::types::NamedAttrValue;

fn main() {
    println!("=== 新 Parser 模块使用示例 ===\n");

    // 示例 1: 解析整数属性
    example_parse_integer();

    // 示例 2: 解析浮点属性 (f32 模式)
    example_parse_f32();

    // 示例 3: 解析 Vec3 属性
    example_parse_vec3();

    // 示例 4: 解析字符串属性
    example_parse_string();

    // 示例 5: 检查表达式
    example_check_expression();

    // 示例 6: 处理 0x07 分段数据
    example_collect_segmented_payload();
}

/// 示例 1: 解析整数属性
fn example_parse_integer() {
    println!("示例 1: 解析整数属性");

    // 模拟数据: 整数 42 (big-endian)
    let data = vec![
        0x00, 0x00, 0x00, 0x2A, // 42
    ];

    // 创建属性偏移信息
    let attr_offset = ImplicitAttrOffset {
        name: "EXAMPLE_INT".to_string(),
        offset: 0, // 从字节 0 开始
        attr_type: DbAttributeType::INTEGER,
    };

    // 解析
    match parse_implicit_attr_value(&data, &attr_offset, false, 0, 1) {
        Ok((_, value)) => match value {
            NamedAttrValue::IntegerType(v) => {
                println!("  ✓ 解析成功: {}", v);
                assert_eq!(v, 42);
            }
            _ => println!("  ✗ 错误的类型"),
        },
        Err(e) => println!("  ✗ 解析失败: {:?}", e),
    }
    println!();
}

/// 示例 2: 解析浮点属性 (f32 模式)
fn example_parse_f32() {
    println!("示例 2: 解析浮点属性 (f32 模式)");

    // 模拟数据: f32 3.14159
    let data = vec![
        0x40, 0x49, 0x0F, 0xDB, // 3.14159 as f32
    ];

    let attr_offset = ImplicitAttrOffset {
        name: "EXAMPLE_FLOAT".to_string(),
        offset: 0,
        attr_type: DbAttributeType::DOUBLE,
    };

    // 使用 f32 模式
    match parse_implicit_attr_value(&data, &attr_offset, true, 0, 1) {
        Ok((_, value)) => match value {
            NamedAttrValue::F32Type(v) => {
                println!("  ✓ 解析成功: {}", v);
                assert!((v - 3.14159).abs() < 0.001);
            }
            _ => println!("  ✗ 错误的类型"),
        },
        Err(e) => println!("  ✗ 解析失败: {:?}", e),
    }
    println!();
}

/// 示例 3: 解析 Vec3 属性
fn example_parse_vec3() {
    println!("示例 3: 解析 Vec3 属性");

    // 模拟数据: 长度 + 3个 f32 值
    let data = vec![
        0x00, 0x00, 0x00, 0x03, // 长度 = 3
        0x3F, 0x80, 0x00, 0x00, // x = 1.0
        0x40, 0x00, 0x00, 0x00, // y = 2.0
        0x40, 0x40, 0x00, 0x00, // z = 3.0
    ];

    let attr_offset = ImplicitAttrOffset {
        name: "EXAMPLE_VEC3".to_string(),
        offset: 0,
        attr_type: DbAttributeType::Vec3Type,
    };

    match parse_implicit_attr_value(&data, &attr_offset, true, 0, 4) {
        Ok((_, value)) => match value {
            NamedAttrValue::Vec3Type(v) => {
                println!("  ✓ 解析成功: {:?}", v);
                assert_eq!(v.x, 1.0);
                assert_eq!(v.y, 2.0);
                assert_eq!(v.z, 3.0);
            }
            _ => println!("  ✗ 错误的类型"),
        },
        Err(e) => println!("  ✗ 解析失败: {:?}", e),
    }
    println!();
}

/// 示例 4: 解析字符串属性
fn example_parse_string() {
    println!("示例 4: 解析字符串属性");

    // 模拟数据: 长度 + 字符串 "HELLO"
    let data = vec![
        0x00, 0x00, 0x00, 0x05, // 长度 = 5
        0x00, 0x00, 0x00, b'H', // H
        0x00, 0x00, 0x00, b'E', // E
        0x00, 0x00, 0x00, b'L', // L
        0x00, 0x00, 0x00, b'L', // L
        0x00, 0x00, 0x00, b'O', // O
    ];

    let attr_offset = ImplicitAttrOffset {
        name: "EXAMPLE_STRING".to_string(),
        offset: 0,
        attr_type: DbAttributeType::STRING,
    };

    match parse_implicit_attr_value(&data, &attr_offset, false, 0, 6) {
        Ok((_, value)) => match value {
            NamedAttrValue::StringType(s) => {
                println!("  ✓ 解析成功: {}", s);
                assert_eq!(s, "HELLO");
            }
            _ => println!("  ✗ 错误的类型"),
        },
        Err(e) => println!("  ✗ 解析失败: {:?}", e),
    }
    println!();
}

/// 示例 5: 检查表达式
fn example_check_expression() {
    println!("示例 5: 检查表达式");

    // 一些常见的表达式属性哈希值
    let expr_hashes = vec![
        0x0095A34, // PTCDI
        0x00D245B, // BLTP
    ];

    for hash in expr_hashes {
        let is_expr = check_is_expr(hash);
        println!("  哈希 {:#08X}: {}", hash, if is_expr { "是表达式" } else { "非表达式" });
    }
    println!();
}

/// 示例 6: 处理 0x07 分段数据
fn example_collect_segmented_payload() {
    println!("示例 6: 处理 0x07 分段数据");

    // 模拟主段 + 追加段
    let data = vec![
        // 主段 (20 字节)
        0x00, 0x02, // flag = 0x0002
        0x00, 0x05, // len = 5 words (20 bytes)
        0x00, 0x00, 0x00, 0x01, // refno high
        0x00, 0x00, 0x00, 0x02, // refno low
        0xAA, 0xBB, 0xCC, 0xDD, // payload 前4字节
        0x11, 0x22, 0x33, 0x44, // payload 后4字节
        // 追加段 (28 字节)
        0x00, 0x00, 0x00, 0x07, // 0x07 marker
        0x00, // padding
        0x02, // flag = 0x02
        0x00, 0x07, // len = 7 words (28 bytes)
        0x00, 0x00, 0x00, 0x01, // refno high
        0x00, 0x00, 0x00, 0x02, // refno low
        0x00, 0x00, 0x00, 0x00, // reserved
        0x00, 0x00, 0x00, 0x00, // reserved
        0x55, 0x66, 0x77, 0x88, // payload 前4字节
        0x99, 0xAA, 0xBB, 0xCC, // payload 后4字节
    ];

    // 收集分段 payload
    match collect_segmented_payload(&data, 20, 0x02) {
        Ok((_, merged)) => {
            println!("  ✓ 合并成功!");
            println!("  主段 payload 长度: {}", 8);
            println!("  追加段 payload 长度: {}", 8);
            println!("  合并后总长度: {}", merged.len());
            println!("  合并后数据: {:02X?}", &merged[..16.min(merged.len())]);

            assert_eq!(merged.len(), 16); // 8 + 8
            assert_eq!(&merged[0..4], &[0xAA, 0xBB, 0xCC, 0xDD]);
            assert_eq!(&merged[4..8], &[0x11, 0x22, 0x33, 0x44]);
            assert_eq!(&merged[8..12], &[0x55, 0x66, 0x77, 0x88]);
            assert_eq!(&merged[12..16], &[0x99, 0xAA, 0xBB, 0xCC]);
        }
        Err(e) => println!("  ✗ 合并失败: {:?}", e),
    }
    println!();
}

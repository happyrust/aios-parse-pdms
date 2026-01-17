//! 分析 ams 1112 数据文件的 db type
//! 
//! 这个示例程序读取 ams7330_0001 文件并分析其数据库类型

use std::fs::File;
use std::io::Read;
use anyhow::Result;

// 直接使用内部模块
use parse_pdms_db::parser::database::header::{extract_db_type, extract_db_no, extract_field_no, DbType};

fn main() -> Result<()> {
    // 分析 ams7330_0001 文件
    let file_path = "test-files/ams7330_0001";
    println!("🔍 分析文件: {}", file_path);
    
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("📁 文件大小: {} bytes", buffer.len());
    
    // 提取数据库头部信息
    if let Some(db_no) = extract_db_no(&buffer) {
        println!("📊 数据库编号 (db_no): {}", db_no);
    } else {
        println!("❌ 无法提取数据库编号");
    }
    
    if let Some(field_no) = extract_field_no(&buffer) {
        println!("📋 字段编号 (field_no): {}", field_no);
    } else {
        println!("❌ 无法提取字段编号");
    }
    
    if let Some(db_type) = extract_db_type(&buffer) {
        println!("🗄️  数据库类型 (db_type): {:?}", db_type);
        println!("📝 类型字符串: {}", db_type.as_str());
        println!("✅ 类型是否有效: {}", db_type.is_valid());
        
        // 分析类型是否正确
        match db_type {
            DbType::Design => println!("💡 这是设计数据库 (DESI)"),
            DbType::Catalog => println!("💡 这是目录数据库 (CATA)"),
            DbType::Dictionary => println!("💡 这是字典数据库 (DICT)"),
            DbType::System => println!("💡 这是系统数据库 (SYST)"),
            DbType::Global => println!("💡 这是全局数据库 (GLOB)"),
            DbType::Unknown => println!("⚠️  这是未知数据库类型"),
        }
    } else {
        println!("❌ 无法提取数据库类型");
    }
    
    // 显示文件头部的前64字节用于调试
    println!("\n📋 文件头部前64字节:");
    for (i, chunk) in buffer.chunks(16).take(4).enumerate() {
        let offset = i * 16;
        print!("{:04x}: ", offset);
        for byte in chunk {
            print!("{:02x} ", byte);
        }
        println!();
    }
    
    // 分析类型哈希值
    if buffer.len() >= 36 {
        let type_hash = i32::from_be_bytes(buffer[32..36].try_into().unwrap());
        println!("\n🔍 类型哈希值: {} (0x{:08x})", type_hash, type_hash as u32);
        
        // 使用 db_tool 的反哈希函数
        use aios_core::tool::db_tool::db1_dehash;
        let type_name = db1_dehash(type_hash as u32);
        println!("📝 反哈希类型名称: '{}'", type_name);
    }
    
    Ok(())
}

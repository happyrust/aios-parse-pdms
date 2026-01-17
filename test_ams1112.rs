use std::fs::File;
use std::io::Read;
use aios_parse_pdms_fork::parser::database::header::{extract_db_type, extract_db_no, extract_field_no};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 读取 ams7330_0001 文件（假设这就是 ams 1112 的数据文件）
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
        println!("🗄️  数据库类型 (db_type): {}", db_type);
        println!("📝 类型字符串: {}", db_type.as_str());
        println!("✅ 类型是否有效: {}", db_type.is_valid());
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
    
    Ok(())
}

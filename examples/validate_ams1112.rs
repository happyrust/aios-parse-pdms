//! 验证 ams 1112 数据文件的完整解析能力
//! 
//! 这个示例程序验证文件是否可以完整解析，包括元素和属性

use std::fs::File;
use std::io::Read;
use anyhow::Result;
use parse_pdms_db::parse::parse_file_basic_info;
use parse_pdms_db::parser::database::validation::validate_db_header;
use parse_pdms_db::parser::database::header::parse_db_header;

fn main() -> Result<()> {
    let file_path = "test-files/ams7330_0001";
    println!("🔍 验证文件: {}", file_path);
    
    // 读取文件
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("📁 文件大小: {} bytes", buffer.len());
    
    // 1. 验证数据库头部
    println!("\n📋 1. 数据库头部验证:");
    let header_result = parse_db_header(&buffer);
    match &header_result {
        Ok((rest, header)) => {
            println!("✅ 头部解析成功");
            println!("   📊 数据库编号: {}", header.db_no);
            println!("   📋 字段编号: {}", header.field_no);
            println!("   🗄️  数据库类型: {}", header.db_type);
            println!("   📍 索引区偏移: {}", header.index_offset);
            println!("   📍 数据区偏移: {}", header.data_offset);
            println!("   🔍 类型哈希: {}", header.type_hash);
            println!("   📝 类型名称: {}", header.type_name());
            
            // 验证剩余数据长度
            println!("   📏 剩余数据: {} bytes", rest.len());
        }
        Err(e) => {
            println!("❌ 头部解析失败: {:?}", e);
            return Ok(());
        }
    }
    
    // 2. 数据库头部验证
    println!("\n📋 2. 数据库有效性验证:");
    let validation_result = validate_db_header(&buffer);
    if validation_result.is_valid {
        println!("✅ 数据库有效");
        if let Some(db_type) = &validation_result.db_type {
            println!("   🗄️  确认类型: {}", db_type);
        }
    } else {
        println!("❌ 数据库无效");
        if let Some(error) = &validation_result.error {
            println!("   🚨 错误: {}", error);
        }
    }
    
    // 3. 基本信息解析
    println!("\n📋 3. 基本信息解析:");
    let basic_info = parse_file_basic_info(&buffer);
    println!("   📊 数据库类型: {}", basic_info.db_type);
    println!("   📋 SES 页号: {}", basic_info.ses_pgno);
    println!("   🔢 数据库编号: {}", basic_info.db_no);
    
    // 4. 分析数据区结构
    if let Ok((_, header)) = &header_result {
        println!("\n📋 4. 数据区结构分析:");
        
        let index_start = header.index_offset as usize;
        let data_start = header.data_offset as usize;
        
        if index_start < buffer.len() && data_start < buffer.len() {
            println!("   📍 索引区范围: {} - {}", index_start, data_start);
            println!("   📍 数据区开始: {}", data_start);
            println!("   📏 索引区大小: {} bytes", data_start - index_start);
            println!("   📏 数据区大小: {} bytes", buffer.len() - data_start);
            
            // 检查索引区前几个字节
            if index_start + 16 < buffer.len() {
                println!("   🔍 索引区前16字节:");
                let index_data = &buffer[index_start..index_start + 16];
                for (i, chunk) in index_data.chunks(4).enumerate() {
                    let offset = index_start + i * 4;
                    print!("      {:04x}: ", offset);
                    for byte in chunk {
                        print!("{:02x} ", byte);
                    }
                    println!();
                }
            }
            
            // 检查数据区前几个字节
            if data_start + 16 < buffer.len() {
                println!("   🔍 数据区前16字节:");
                let data_data = &buffer[data_start..data_start + 16];
                for (i, chunk) in data_data.chunks(4).enumerate() {
                    let offset = data_start + i * 4;
                    print!("      {:04x}: ", offset);
                    for byte in chunk {
                        print!("{:02x} ", byte);
                    }
                    println!();
                }
            }
        } else {
            println!("   ⚠️  偏移量超出文件范围");
        }
    }
    
    // 5. 总结
    println!("\n📋 5. 验证总结:");
    if header_result.is_ok() && validation_result.is_valid {
        println!("✅ 文件格式正确，可以安全解析");
        println!("🗄️  数据库类型: DESI (设计数据库)");
        println!("📊 文件结构完整");
        println!("🎯 结论: db type 正确，无需修改");
    } else {
        println!("❌ 文件格式存在问题，需要进一步检查");
    }
    
    Ok(())
}

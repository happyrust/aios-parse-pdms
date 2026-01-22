//! 解析 ams7330_0001 文件并进行数据统计
//! 
//! 这个示例程序完整解析文件内容，统计元素和属性信息

use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use anyhow::Result;
use parse_pdms_db::parse::parse_ele_data;
use parse_pdms_db::parser::database::header::{extract_db_type, extract_db_no, extract_field_no};

#[derive(Debug, Default)]
struct Statistics {
    // 基本统计
    total_elements: usize,
    total_attributes: usize,
    total_explicit_attributes: usize,
    
    // 按类型统计
    elements_by_type: HashMap<String, usize>,
    attributes_by_name: HashMap<String, usize>,
    
    // 解析统计
    successful_parses: usize,
    failed_parses: usize,
    parse_errors: Vec<String>,
    
    // 性能统计
    total_parse_time_us: u64,
    avg_parse_time_us: f64,
}

impl std::fmt::Display for Statistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n╔══════════════════════════════════════════════════════════════╗")?;
        writeln!(f, "║                    AMS7330_0001 数据统计报告                    ║")?;
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        
        // 基本统计
        writeln!(f, "║ 📊 基本统计                                                    ║")?;
        writeln!(f, "║   总元素数量:        {:>12}                              ║", self.total_elements)?;
        writeln!(f, "║   总属性数量:        {:>12}                              ║", self.total_attributes)?;
        writeln!(f, "║   显式属性数量:      {:>12}                              ║", self.total_explicit_attributes)?;
        writeln!(f, "║   隐式属性数量:      {:>12}                              ║", 
                self.total_attributes - self.total_explicit_attributes)?;
        
        // 解析统计
        writeln!(f, "║                                                              ║")?;
        writeln!(f, "║ 🔍 解析统计                                                    ║")?;
        writeln!(f, "║   成功解析:          {:>12}                              ║", self.successful_parses)?;
        writeln!(f, "║   解析失败:          {:>12}                              ║", self.failed_parses)?;
        if self.successful_parses > 0 {
            writeln!(f, "║   成功率:            {:>12.1}%                            ║", 
                    (self.successful_parses as f64 / (self.successful_parses + self.failed_parses) as f64) * 100.0)?;
        }
        
        // 性能统计
        writeln!(f, "║                                                              ║")?;
        writeln!(f, "║ ⚡ 性能统计                                                    ║")?;
        writeln!(f, "║   总解析时间:        {:>12.2} ms                          ║", self.total_parse_time_us as f64 / 1000.0)?;
        if self.successful_parses > 0 {
            writeln!(f, "║   平均解析时间:      {:>12.2} μs                          ║", self.avg_parse_time_us)?;
            writeln!(f, "║   解析速度:          {:>12.2} 元素/秒                      ║", 
                    self.successful_parses as f64 / (self.total_parse_time_us as f64 / 1_000_000.0))?;
        }
        
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        writeln!(f, "║ 🗄️  元素类型分布 (Top 10)                                     ║")?;
        
        let mut type_counts: Vec<_> = self.elements_by_type.iter().collect();
        type_counts.sort_by(|a, b| b.1.cmp(a.1));
        
        for (i, (elem_type, count)) in type_counts.iter().take(10).enumerate() {
            writeln!(f, "║   {:>2}. {:<12} : {:>6} ({:>5.1}%)                    ║", 
                    i + 1, elem_type, count, 
                    (**count as f64 / self.total_elements as f64) * 100.0)?;
        }
        
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        writeln!(f, "║ 📋 属性名称分布 (Top 10)                                     ║")?;
        
        let mut attr_counts: Vec<_> = self.attributes_by_name.iter().collect();
        attr_counts.sort_by(|a, b| b.1.cmp(a.1));
        
        for (i, (attr_name, count)) in attr_counts.iter().take(10).enumerate() {
            writeln!(f, "║   {:>2}. {:<12} : {:>6} ({:>5.1}%)                    ║", 
                    i + 1, attr_name, count,
                    (**count as f64 / self.total_attributes as f64) * 100.0)?;
        }
        
        if !self.parse_errors.is_empty() {
            writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
            writeln!(f, "║ 🚨 解析错误 (前5个)                                          ║")?;
            for (i, error) in self.parse_errors.iter().take(5).enumerate() {
                writeln!(f, "║   {}. {}                                    ║", i + 1, error)?;
            }
        }
        
        writeln!(f, "╚══════════════════════════════════════════════════════════════╝")?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let file_path = "test-files/ams7330_0001";
    println!("🔍 解析文件: {}", file_path);
    
    // 读取文件
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("📁 文件大小: {} bytes", buffer.len());
    
    // 显示文件基本信息
    if let Some(dbnum) = extract_db_no(&buffer) {
        println!("📊 数据库编号: {}", dbnum);
    }
    if let Some(field_no) = extract_field_no(&buffer) {
        println!("📋 字段编号: {}", field_no);
    }
    if let Some(db_type) = extract_db_type(&buffer) {
        println!("🗄️  数据库类型: {}", db_type);
    }
    
    let mut stats = Statistics::default();
    let start_time = std::time::Instant::now();
    
    // 尝试解析整个文件
    println!("\n🔍 开始解析文件内容...");
    
    let mut offset = 0;
    let mut element_index = 0;
    let max_elements = 1000; // 限制解析数量以避免过长时间
    
    while offset < buffer.len() && element_index < max_elements {
        let element_start = std::time::Instant::now();
        
        // 尝试解析一个元素
        match parse_ele_data(&buffer[offset..]).await {
            Ok(ele_data) => {
                let parse_time = element_start.elapsed().as_micros() as u64;
                stats.total_parse_time_us += parse_time;
                stats.successful_parses += 1;
                
                // 统计元素信息
                let noun = ele_data.whole_attmap.attmap
                    .get_as_string("TYPE")
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                
                *stats.elements_by_type.entry(noun.clone()).or_insert(0) += 1;
                
                let attr_count = ele_data.whole_attmap.attmap.len();
                let explicit_attr_count = ele_data.whole_attmap.explicit_attmap.len();
                
                stats.total_attributes += attr_count;
                stats.total_explicit_attributes += explicit_attr_count;
                stats.total_elements += 1;
                
                // 统计属性名称
                for attr_name in ele_data.whole_attmap.attmap.keys() {
                    *stats.attributes_by_name.entry(attr_name.clone()).or_insert(0usize) += 1;
                }
                for attr_name in ele_data.whole_attmap.explicit_attmap.keys() {
                    *stats.attributes_by_name.entry(format!("{}*", attr_name)).or_insert(0usize) += 1;
                }
                
                // 显示进度
                if element_index < 10 || element_index % 100 == 0 {
                    println!("  📌 元素 #{}: {} (属性: {}, 显式: {}, 耗时: {}μs)",
                        element_index, 
                        ele_data.refno.to_string(),
                        attr_count,
                        explicit_attr_count,
                        parse_time
                    );
                }
                
                element_index += 1;
                
                // 估算下一个元素的偏移量
                // 这里简化处理，假设每个元素至少占用64字节
                offset += 64;
                
                // 如果解析的数据量很小，可能需要调整偏移量
                if offset >= buffer.len() {
                    break;
                }
            }
            Err(e) => {
                stats.failed_parses += 1;
                let error_msg = format!("元素 #{} (offset={}): {}", element_index, offset, e);
                stats.parse_errors.push(error_msg);
                
                // 如果连续失败多次，尝试跳过一些字节
                if stats.failed_parses > 10 {
                    println!("⚠️  连续解析失败，尝试跳过...");
                    offset += 64;
                    stats.failed_parses = 0;
                } else {
                    offset += 1;
                }
                
                if offset >= buffer.len() {
                    break;
                }
            }
        }
    }
    
    let total_time = start_time.elapsed();
    
    if stats.successful_parses > 0 {
        stats.avg_parse_time_us = stats.total_parse_time_us as f64 / stats.successful_parses as f64;
    }
    
    println!("\n⏱️  总解析时间: {:?}", total_time);
    println!("📊 解析统计: 成功 {}, 失败 {}", stats.successful_parses, stats.failed_parses);
    
    // 显示统计报告
    println!("{}", stats);
    
    Ok(())
}

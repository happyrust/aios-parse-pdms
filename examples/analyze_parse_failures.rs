//! 分析解析失败的情况
//! 
//! 专门分析 ams7330_0001 文件中解析失败的类型和数据

use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use anyhow::Result;
use parse_pdms_db::parse::parse_ele_data;

#[derive(Debug, Default)]
struct FailureAnalysis {
    // 失败统计
    total_attempts: usize,
    successful_parses: usize,
    failed_parses: usize,
    
    // 失败类型统计
    failure_types: HashMap<String, usize>,
    failure_offsets: Vec<usize>,
    failure_contexts: Vec<String>,
    
    // 成功解析的元素类型
    successful_types: HashMap<String, usize>,
    
    // 详细错误信息
    detailed_errors: Vec<String>,
}

impl FailureAnalysis {
    fn add_failure(&mut self, offset: usize, error: &str, context: &[u8]) {
        self.failed_parses += 1;
        self.total_attempts += 1;
        
        // 统计错误类型
        let error_type = if error.contains("not exist in attr_info_map") {
            "属性映射缺失".to_string()
        } else if error.contains("parse") {
            "解析错误".to_string()
        } else if error.contains("invalid") {
            "无效数据".to_string()
        } else {
            "其他错误".to_string()
        };
        
        *self.failure_types.entry(error_type).or_insert(0) += 1;
        self.failure_offsets.push(offset);
        
        // 保存上下文信息
        let context_hex = context.iter()
            .take(32)
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        self.failure_contexts.push(format!("offset {}: {}", offset, context_hex));
        
        // 保存详细错误
        self.detailed_errors.push(format!("offset {}: {}", offset, error));
    }
    
    fn add_success(&mut self, element_type: String) {
        self.successful_parses += 1;
        self.total_attempts += 1;
        *self.successful_types.entry(element_type).or_insert(0) += 1;
    }
}

impl std::fmt::Display for FailureAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n╔══════════════════════════════════════════════════════════════╗")?;
        writeln!(f, "║                    解析失败分析报告                              ║")?;
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        
        // 基本统计
        writeln!(f, "║ 📊 解析统计                                                    ║")?;
        writeln!(f, "║   总尝试次数:        {:>12}                              ║", self.total_attempts)?;
        writeln!(f, "║   成功解析:          {:>12}                              ║", self.successful_parses)?;
        writeln!(f, "║   解析失败:          {:>12}                              ║", self.failed_parses)?;
        if self.total_attempts > 0 {
            writeln!(f, "║   成功率:            {:>12.1}%                            ║", 
                    (self.successful_parses as f64 / self.total_attempts as f64) * 100.0)?;
        }
        
        // 失败类型分析
        writeln!(f, "║                                                              ║")?;
        writeln!(f, "║ 🚨 失败类型分析                                                ║")?;
        for (error_type, count) in &self.failure_types {
            writeln!(f, "║   {:<12}: {:>6} ({:>5.1}%)                        ║", 
                    error_type, count, 
                    (*count as f64 / self.failed_parses as f64) * 100.0)?;
        }
        
        // 成功解析的类型
        writeln!(f, "║                                                              ║")?;
        writeln!(f, "║ ✅ 成功解析的类型 (Top 10)                                    ║")?;
        let mut success_types: Vec<_> = self.successful_types.iter().collect();
        success_types.sort_by(|a, b| b.1.cmp(a.1));
        
        for (i, (elem_type, count)) in success_types.iter().take(10).enumerate() {
            writeln!(f, "║   {:>2}. {:<12} : {:>6} ({:>5.1}%)                    ║", 
                    i + 1, elem_type, count,
                    (**count as f64 / self.successful_parses as f64) * 100.0)?;
        }
        
        // 失败偏移量分析
        if !self.failure_offsets.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 📍 失败偏移量分析                                            ║")?;
            writeln!(f, "║   失败偏移量数量: {}                                      ║", self.failure_offsets.len())?;
            
            // 分析偏移量分布
            let mut ranges = HashMap::new();
            for &offset in &self.failure_offsets {
                let range = (offset / 1024) * 1024; // 按 1KB 分组
                *ranges.entry(range).or_insert(0) += 1;
            }
            
            writeln!(f, "║   偏移量分布 (按 1KB 分组):                                 ║")?;
            let mut ranges_vec: Vec<_> = ranges.iter().collect();
            ranges_vec.sort_by_key(|&(range, _)| range);
            
            for (range, count) in ranges_vec.iter().take(10) {
                writeln!(f, "║     {:06x}-{:06x}: {:>4} 次                        ║", 
                        *range, *range + 1023, count)?;
            }
        }
        
        // 详细错误信息
        if !self.detailed_errors.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 🔍 详细错误信息 (前10个)                                    ║")?;
            for (i, error) in self.detailed_errors.iter().take(10).enumerate() {
                writeln!(f, "║   {}. {}                                    ║", i + 1, error)?;
            }
        }
        
        // 失败上下文
        if !self.failure_contexts.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 📋 失败上下文 (前5个)                                       ║")?;
            for (i, context) in self.failure_contexts.iter().take(5).enumerate() {
                writeln!(f, "║   {}. {}                                    ║", i + 1, context)?;
            }
        }
        
        writeln!(f, "╚══════════════════════════════════════════════════════════════╝")?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let file_path = "test-files/ams7330_0001";
    println!("🔍 分析解析失败情况: {}", file_path);
    
    // 读取文件
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("📁 文件大小: {} bytes", buffer.len());
    
    let mut analysis = FailureAnalysis::default();
    let mut offset = 0;
    let mut element_index = 0;
    let max_elements = 2000; // 增加解析数量以找到更多失败情况
    
    while offset < buffer.len() && element_index < max_elements {
        // 尝试解析一个元素
        match parse_ele_data(&buffer[offset..]).await {
            Ok(ele_data) => {
                // 成功解析
                let noun = ele_data.whole_attmap.attmap
                    .get_as_string("TYPE")
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                
                analysis.add_success(noun.clone());
                
                // 显示成功解析的进度
                if element_index < 10 || element_index % 500 == 0 {
                    println!("✅ 元素 #{}: {} (offset: {})", 
                        element_index, noun, offset);
                }
                
                element_index += 1;
                
                // 估算下一个元素的偏移量
                offset += 64;
                
                // 如果到达文件末尾，停止
                if offset >= buffer.len() {
                    break;
                }
            }
            Err(e) => {
                // 解析失败
                let error_msg = e.to_string();
                
                // 获取失败位置的上下文
                let context_start = if offset >= 16 { offset - 16 } else { 0 };
                let context_end = std::cmp::min(offset + 32, buffer.len());
                let context = &buffer[context_start..context_end];
                
                analysis.add_failure(offset, &error_msg, context);
                
                // 显示失败信息
                if analysis.failed_parses <= 10 {
                    println!("❌ 解析失败 #{} (offset: {}): {}", 
                        analysis.failed_parses, offset, error_msg);
                }
                
                // 尝试跳过一些字节继续解析
                offset += 1;
                
                // 如果连续失败多次，尝试跳过更大的块
                if analysis.failed_parses % 20 == 0 {
                    println!("⚠️  连续失败 {} 次，跳过 64 字节", analysis.failed_parses);
                    offset += 63; // 因为后面还会 +1
                }
                
                if offset >= buffer.len() {
                    break;
                }
            }
        }
    }
    
    println!("\n📊 分析完成:");
    println!("   总尝试: {}", analysis.total_attempts);
    println!("   成功: {}", analysis.successful_parses);
    println!("   失败: {}", analysis.failed_parses);
    
    // 显示分析报告
    println!("{}", analysis);
    
    // 生成建议
    println!("\n💡 分析建议:");
    if analysis.failed_parses == 0 {
        println!("✅ 没有发现解析失败，数据格式完全正确");
    } else if analysis.failed_parses < analysis.successful_parses / 10 {
        println!("✅ 解析失败率较低 (<10%)，数据质量良好");
    } else {
        println!("⚠️  解析失败率较高，建议检查数据格式或解析器");
    }
    
    if analysis.failure_types.contains_key("属性映射缺失") {
        println!("📋 主要问题是属性映射缺失，建议完善 attr_info_map");
    }
    
    Ok(())
}

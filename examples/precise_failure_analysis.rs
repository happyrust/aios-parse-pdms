//! 精确分析解析失败的情况
//! 
//! 使用更智能的方法来定位真正的元素边界，避免在非元素位置尝试解析

use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use anyhow::Result;
use parse_pdms_db::parse::parse_ele_data;

#[derive(Debug, Default)]
struct PreciseFailureAnalysis {
    // 成功解析的元素
    successful_elements: Vec<ElementInfo>,
    
    // 失败分析
    genuine_failures: Vec<GenuineFailure>,
    non_element_data: Vec<NonElementData>,
    
    // 统计
    total_scanned: usize,
    element_boundaries_found: usize,
}

#[derive(Debug)]
struct ElementInfo {
    offset: usize,
    element_type: String,
    refno: String,
    attribute_count: usize,
    parse_time_us: u64,
}

#[derive(Debug)]
struct GenuineFailure {
    offset: usize,
    error: String,
    context: Vec<u8>,
}

#[derive(Debug)]
struct NonElementData {
    offset: usize,
    data_type: String,
    size: usize,
}

impl PreciseFailureAnalysis {
    fn add_success(&mut self, info: ElementInfo) {
        self.successful_elements.push(info);
        self.element_boundaries_found += 1;
    }
    
    fn add_genuine_failure(&mut self, offset: usize, error: &str, context: &[u8]) {
        self.genuine_failures.push(GenuineFailure {
            offset,
            error: error.to_string(),
            context: context.to_vec(),
        });
    }
    
    fn add_non_element_data(&mut self, offset: usize, data_type: &str, size: usize) {
        self.non_element_data.push(NonElementData {
            offset,
            data_type: data_type.to_string(),
            size,
        });
    }
}

impl std::fmt::Display for PreciseFailureAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n╔══════════════════════════════════════════════════════════════╗")?;
        writeln!(f, "║                    精确解析失败分析报告                          ║")?;
        writeln!(f, "╠══════════════════════════════════════════════════════════════╣")?;
        
        // 基本统计
        writeln!(f, "║ 📊 扫描统计                                                    ║")?;
        writeln!(f, "║   总扫描位置:        {:>12}                              ║", self.total_scanned)?;
        writeln!(f, "║   成功元素:          {:>12}                              ║", self.successful_elements.len())?;
        writeln!(f, "║   真正失败:          {:>12}                              ║", self.genuine_failures.len())?;
        writeln!(f, "║   非元素数据:        {:>12}                              ║", self.non_element_data.len())?;
        
        if self.element_boundaries_found > 0 {
            writeln!(f, "║   元素边界识别率:    {:>12.1}%                            ║", 
                    (self.element_boundaries_found as f64 / self.total_scanned as f64) * 100.0)?;
        }
        
        // 成功元素类型分析
        if !self.successful_elements.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ ✅ 成功解析的元素类型                                        ║")?;
            
            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for element in &self.successful_elements {
                *type_counts.entry(element.element_type.clone()).or_insert(0) += 1;
            }
            
            let mut sorted_types: Vec<_> = type_counts.iter().collect();
            sorted_types.sort_by(|a, b| b.1.cmp(a.1));
            
            for (i, (elem_type, count)) in sorted_types.iter().take(10).enumerate() {
                writeln!(f, "║   {:>2}. {:<12} : {:>6} ({:>5.1}%)                    ║", 
                        i + 1, elem_type, count,
                        (**count as f64 / self.successful_elements.len() as f64) * 100.0)?;
            }
        }
        
        // 真正失败分析
        if !self.genuine_failures.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 🚨 真正解析失败分析                                            ║")?;
            
            let mut failure_types: HashMap<String, usize> = HashMap::new();
            for failure in &self.genuine_failures {
                let error_type = if failure.error.contains("not exist in attr_info_map") {
                    "属性映射缺失"
                } else if failure.error.contains("impl_len") {
                    "数据长度错误"
                } else if failure.error.contains("parse") {
                    "解析错误"
                } else {
                    "其他错误"
                };
                *failure_types.entry(error_type.to_string()).or_insert(0) += 1;
            }
            
            for (error_type, count) in &failure_types {
                writeln!(f, "║   {:<12}: {:>6} ({:>5.1}%)                        ║", 
                        error_type, count,
                        (*count as f64 / self.genuine_failures.len() as f64) * 100.0)?;
            }
            
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 🔍 详细失败信息 (前5个)                                     ║")?;
            for (i, failure) in self.genuine_failures.iter().take(5).enumerate() {
                writeln!(f, "║   {}. offset {}: {}                    ║", 
                        i + 1, failure.offset, failure.error)?;
            }
        }
        
        // 非元素数据分析
        if !self.non_element_data.is_empty() {
            writeln!(f, "║                                                              ║")?;
            writeln!(f, "║ 📋 非元素数据分析                                              ║")?;
            
            let mut data_type_counts: HashMap<String, usize> = HashMap::new();
            for data in &self.non_element_data {
                *data_type_counts.entry(data.data_type.clone()).or_insert(0) += 1;
            }
            
            for (data_type, count) in &data_type_counts {
                writeln!(f, "║   {:<12}: {:>6} 次                        ║", 
                        data_type, count)?;
            }
        }
        
        writeln!(f, "╚══════════════════════════════════════════════════════════════╝")?;
        Ok(())
    }
}

/// 检查是否可能是元素开始位置
fn is_potential_element_start(data: &[u8], offset: usize) -> bool {
    if offset + 64 > data.len() {
        return false;
    }
    
    let chunk = &data[offset..offset + 64];
    
    // 检查是否看起来像元素头部
    // 元素通常以特定的模式开始
    let has_valid_header = chunk[0] == 0 && chunk[1] == 0 && // 通常前两个字节为0
                           chunk[4] != 0 && chunk[5] != 0; // 后面有非零数据
    
    // 检查是否有合理的长度信息
    let potential_length = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    let has_reasonable_length = potential_length > 0 && potential_length < 1000000;
    
    has_valid_header && has_reasonable_length
}

#[tokio::main]
async fn main() -> Result<()> {
    let file_path = "test-files/ams7330_0001";
    println!("🔍 精确分析解析失败情况: {}", file_path);
    
    // 读取文件
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    println!("📁 文件大小: {} bytes", buffer.len());
    
    let mut analysis = PreciseFailureAnalysis::default();
    
    // 跳过文件头部，从数据区开始
    let mut offset = 643; // 从数据区偏移开始
    let scan_step = 64; // 扫描步长
    
    while offset < buffer.len() - scan_step {
        analysis.total_scanned += 1;
        
        // 检查是否可能是元素开始
        if is_potential_element_start(&buffer, offset) {
            // 尝试解析
            match parse_ele_data(&buffer[offset..]).await {
                Ok(ele_data) => {
                    // 成功解析
                    let element_type = ele_data.whole_attmap.attmap
                        .get_as_string("TYPE")
                        .unwrap_or_else(|| "UNKNOWN".to_string());
                    
                    let element_info = ElementInfo {
                        offset,
                        element_type: element_type.clone(),
                        refno: ele_data.refno.to_string(),
                        attribute_count: ele_data.whole_attmap.attmap.len(),
                        parse_time_us: 0, // 不测量时间以提高速度
                    };
                    
                    analysis.add_success(element_info);
                    
                    // 显示进度
                    if analysis.successful_elements.len() % 100 == 0 {
                        println!("✅ 发现元素 #{}: {} (offset: {})", 
                            analysis.successful_elements.len(), element_type, offset);
                    }
                    
                    // 跳过这个元素，寻找下一个
                    offset += scan_step;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    
                    // 判断是真正的失败还是非元素数据
                    if error_msg.contains("not exist in attr_info_map") || 
                       error_msg.contains("impl_len") {
                        // 这可能是真正的元素但解析失败
                        let context_start = if offset >= 16 { offset - 16 } else { 0 };
                        let context_end = std::cmp::min(offset + 32, buffer.len());
                        let context = &buffer[context_start..context_end];
                        
                        analysis.add_genuine_failure(offset, &error_msg, context);
                        
                        println!("❌ 真正失败 offset {}: {}", offset, error_msg);
                    } else {
                        // 这可能不是元素数据
                        analysis.add_non_element_data(offset, "未知数据", scan_step);
                    }
                    
                    offset += 1; // 逐字节前进以找到真正的元素开始
                }
            }
        } else {
            // 不是元素开始，标记为非元素数据
            analysis.add_non_element_data(offset, "非元素头部", scan_step);
            offset += scan_step;
        }
        
        // 避免无限循环
        if offset > buffer.len() - 1000 {
            break;
        }
    }
    
    println!("\n📊 精确分析完成:");
    println!("   总扫描位置: {}", analysis.total_scanned);
    println!("   成功元素: {}", analysis.successful_elements.len());
    println!("   真正失败: {}", analysis.genuine_failures.len());
    println!("   非元素数据: {}", analysis.non_element_data.len());
    
    // 显示分析报告
    println!("{}", analysis);
    
    // 生成建议
    println!("\n💡 精确分析建议:");
    if analysis.genuine_failures.is_empty() {
        println!("✅ 没有发现真正的解析失败，所有元素都能正确解析");
    } else {
        println!("⚠️  发现 {} 个真正的解析失败", analysis.genuine_failures.len());
        
        let attr_map_failures = analysis.genuine_failures.iter()
            .filter(|f| f.error.contains("not exist in attr_info_map"))
            .count();
        
        if attr_map_failures > 0 {
            println!("📋 其中 {} 个是由于属性映射缺失", attr_map_failures);
            println!("🔧 建议完善 attr_info_map 以提高解析成功率");
        }
    }
    
    println!("📈 元素边界识别率: {:.1}%", 
        (analysis.element_boundaries_found as f64 / analysis.total_scanned as f64) * 100.0);
    
    Ok(())
}

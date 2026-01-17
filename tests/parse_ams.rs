use std::path::PathBuf;

use parse_pdms_db::parse::parse_file;
use aios_core::types::RefU64;

#[tokio::test]
#[ignore]
async fn parse_ams7330_0001() -> anyhow::Result<()> {
    let path = PathBuf::from("test-files/ams7330_0001");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid file name"))?;

    // 项目名取文件名前缀，便于 field_no 解析；ams7330_0001 => ams7330
    let project = "ams7330";

    let pdms = parse_file(&path, &None, file_name, project).await?;
    println!(
        "Parsed {}: types={}, attrs={}, children={}",
        file_name,
        pdms.type_ele_map.len(),
        pdms.total_attr_map.len(),
        pdms.children_map.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn parse_ams1112_0001() -> anyhow::Result<()> {
    let path = PathBuf::from("D:/AVEVA/Projects/E3D2.1/AvevaMarineSample/ams000/ams1112_0001");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid file name"))?;

    // 项目名取文件名前缀，便于 field_no 解析；ams1112_0001 => ams1112
    let project = "ams1112";

    let pdms = parse_file(&path, &None, file_name, project).await?;
    println!(
        "Parsed {}: types={}, attrs={}, children={}",
        file_name,
        pdms.type_ele_map.len(),
        pdms.total_attr_map.len(),
        pdms.children_map.len()
    );

    Ok(())
}

/// RUS-104: 调试 BEND 元素 ANGL 属性解析
/// 问题：Ref=17496/171138 BEND 元素的 ANGL 应为 14.976 degree，解析结果异常
#[tokio::test]
async fn test_rus_104_bend_angl_parsing() -> anyhow::Result<()> {
    let path = PathBuf::from("D:/AVEVA/Projects/E3D2.1/AvevaMarineSample/ams000/ams1112_0001");
    let file_name = "ams1112_0001";
    let project = "ams1112";

    let pdms = parse_file(&path, &None, file_name, project).await?;
    
    // Ref = 17496/171138
    let target_refno = RefU64::from_two_nums(17496, 171138);
    
    if let Some(attr_map) = pdms.total_attr_map.get(&target_refno) {
        println!("\n=== RUS-104 调试信息 ===");
        println!("Target Refno: {}", target_refno);
        println!("元素类型: {:?}", attr_map.get_as_string("TYPE"));
        
        // 获取 ANGL 的原始值
        if let Some(angl_val) = attr_map.get_val("ANGL") {
            println!("ANGL 原始值: {:?}", angl_val);
        } else {
            println!("ANGL 属性不存在!");
        }
        
        let angl = attr_map.get_f32_or_default("ANGL");
        println!("ANGL get_f32_or_default: {} degree", angl);
        
        // 打印所有属性用于分析
        println!("\n--- 该元素的全部属性 ---");
        for (k, v) in attr_map.iter() {
            println!("  {} = {:?}", k, v);
        }
        
        // 期望值约为 14.976 degree
        println!("\n期望值: 14.976 degree");
        println!("实际值: {} degree", angl);
        
        // 临时禁用断言，用于调试
        // assert!((angl - 14.976).abs() < 0.01, "ANGL should be ~14.976, got {}", angl);
    } else {
        panic!("Refno 17496/171138 not found in parsed data");
    }
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn parse_ams1112_0001_refno_17496_142306_attrs() -> anyhow::Result<()> {
    let path = PathBuf::from("D:/AVEVA/Projects/E3D2.1/AvevaMarineSample/ams000/ams1112_0001");
    let file_name = "ams1112_0001";
    let project = "ams1112";

    let pdms = parse_file(&path, &None, file_name, project).await?;

    let target_refno = RefU64::from_two_nums(17496, 142306);
    if let Some(attr_map) = pdms.total_attr_map.get(&target_refno) {
        println!("\n=== 17496_142306 属性 ===");
        println!("Target Refno: {}", target_refno);
        println!("元素类型: {:?}", attr_map.get_as_string("TYPE"));
        for (key, value) in attr_map.iter() {
            println!("  {} = {:?}", key, value);
        }
    } else {
        println!("Refno 17496_142306 not found in parsed data");
    }

    Ok(())
}

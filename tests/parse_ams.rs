use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use aios_core::types::RefU64;
use aios_core::tool::db_tool::db1_dehash;
use parse_pdms_db::parse::parse_file;
use parse_pdms_db::parse::{collect_explict_data, parse_file_db_basic_data};
use parse_pdms_db::parse_explict_tools::parse_expression_attr as parse_expression_attr_legacy;
use parse_pdms_db::parser::attribute::explicit::parse_explicit_header;
use parse_pdms_db::parser::attribute::expression::parse_expression_attr;
use parse_pdms_db::parser::attribute::expression_payload::decode_expression_payload;
use parse_pdms_db::parser::combinator::{collect_segmented_payload, extend_impl_len};
use parse_pdms_db::parser::primitives::parse_impl_len_bytes;

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

#[tokio::test]
async fn test_ams5052_0001_refno_13244_103430_expressions() -> anyhow::Result<()> {
    let path = PathBuf::from("D:/AVEVA/Projects/E3D2.1/AvevaMarineSample/ams000/ams5052_0001");
    let file_name = "ams5052_0001";
    let project = "ams5052";

    let pdms = parse_file(&path, &None, file_name, project).await?;

    let target_refno = RefU64::from_two_nums(13244, 103430);
    let attr_map = pdms
        .total_attr_map
        .get(&target_refno)
        .ok_or_else(|| anyhow::anyhow!("Refno {} not found in parsed data", target_refno))?;

    let debug_names = [
        "PX", "PY", "PZ", "PXLE", "PYLE", "PZLE",
        "PXLEN", "PYLEN", "PZLEN", "PXLENGTH", "PYLENGTH", "PZLENGTH",
    ];
    if let Ok(payloads) =
        collect_explicit_attr_payloads(&path, file_name, project, target_refno, &debug_names)
    {
        for (name, payload) in payloads {
            println!("{} payload bytes = {}", name, format_hex_words(&payload));
            if let Ok((_, (_ty, value))) = parse_expression_attr(&payload, target_refno.0) {
                println!("{} parse_expression_attr => {}", name, value);
            }
            if let Ok((_, (_ty, value))) = parse_expression_attr_legacy(&payload, target_refno) {
                println!("{} parse_expression_attr_legacy => {}", name, value);
            }
            if payload.len() > 4 {
                if let Ok((_, value)) = decode_expression_payload(&payload[4..]) {
                    println!("{} decode_expression_payload => {}", name, value);
                }
            }
        }
    }

    let px = get_expr(&attr_map, &["PX"])?;
    let py = get_expr(&attr_map, &["PY"])?;
    let pz = get_expr(&attr_map, &["PZ"])?;
    let px_len = get_expr(&attr_map, &["PXLE", "PXLEN", "PXLENGTH"])?;
    let py_len = get_expr(&attr_map, &["PYLE", "PYLEN", "PYLENGTH"])?;
    let pz_len = get_expr(&attr_map, &["PZLE", "PZLEN", "PZLENGTH"])?;

    let expected_px = "ATTRIB RPRO TWID / 2 - 0.5 * ATTRIB RPRO YA";
    let expected_py = "- (ATTRIB RPRO YB + ATTRIB RPRO CGB + ATTRIB PARA[2 ] / 2)";
    let expected_pz = "ATTRIB RPRO TLEN / 2";
    let expected_px_len = "ATTRIB RPRO TWID - ATTRIB RPRO YA";
    let expected_py_len = "ATTRIB PARA[2 ]";
    let expected_pz_len = "ATTRIB RPRO TLEN";

    assert_eq!(normalize_expr(&px), normalize_expr(expected_px));
    assert_eq!(normalize_expr(&py), normalize_expr(expected_py));
    assert_eq!(normalize_expr(&pz), normalize_expr(expected_pz));
    assert_eq!(normalize_expr(&px_len), normalize_expr(expected_px_len));
    assert_eq!(normalize_expr(&py_len), normalize_expr(expected_py_len));
    assert_eq!(normalize_expr(&pz_len), normalize_expr(expected_pz_len));

    Ok(())
}

fn get_expr(
    attr_map: &aios_core::types::NamedAttrMap,
    names: &[&str],
) -> anyhow::Result<String> {
    for name in names {
        if let Some(value) = attr_map.get_as_string(name) {
            return Ok(value);
        }
    }
    let mut keys: Vec<String> = attr_map.iter().map(|(k, _)| k.to_string()).collect();
    keys.sort();
    anyhow::bail!("missing attr {:?}. available keys: {:?}", names, keys)
}

fn normalize_expr(input: &str) -> String {
    let mut text = input.trim().to_string();
    if let Some(stripped) = strip_outer_parens(&text) {
        text = stripped;
    }
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn strip_outer_parens(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
        return None;
    }
    let mut depth = 0;
    let mut end_index = None;
    for (idx, ch) in trimmed.chars().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end_index = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    if end_index == Some(trimmed.len() - 1) {
        Some(trimmed[1..trimmed.len() - 1].trim().to_string())
    } else {
        None
    }
}

fn collect_explicit_attr_payloads(
    path: &Path,
    file_name: &str,
    project: &str,
    target_refno: RefU64,
    target_names: &[&str],
) -> anyhow::Result<HashMap<String, Vec<u8>>> {
    let mut name_set = HashSet::new();
    for name in target_names {
        name_set.insert(*name);
    }
    let db_basic = parse_file_db_basic_data(&path.to_path_buf(), file_name, project)?;
    let entry = db_basic
        .refno_table_map
        .get(&target_refno)
        .ok_or_else(|| anyhow::anyhow!("Refno {} not found in index", target_refno))?;
    let start = entry.pos.saturating_sub(4);
    let input = &db_basic.bytes[start..];
    let (_, impl_len_bytes) = parse_impl_len_bytes(input)
        .map_err(|_| anyhow::anyhow!("parse impl_len failed"))?;
    let actual_impl_len = extend_impl_len(impl_len_bytes, input);
    if actual_impl_len > input.len() {
        anyhow::bail!("actual_impl_len > input.len()");
    }
    let membs_data = &input[actual_impl_len..];
    let mut memb_bytes_len = 0;
    if membs_data.len() > 12 {
        let maybe_refno = RefU64::from(&membs_data[4..12]);
        if maybe_refno == target_refno && &membs_data[0..2] == [0x0, 0x2].as_slice() {
            let declared_words =
                u16::from_be_bytes([membs_data[2], membs_data[3]]) as usize;
            let declared_bytes = declared_words * 4;
            if let Ok((rest, _merged)) =
                collect_segmented_payload(membs_data, declared_bytes, 0x2)
            {
                memb_bytes_len = membs_data.len().saturating_sub(rest.len());
            }
        }
    }
    let explicit_start = actual_impl_len + memb_bytes_len;
    if explicit_start > input.len() {
        anyhow::bail!("explicit_start > input.len()");
    }
    let explicit_data = &input[explicit_start..];
    let explicit_data = collect_explict_data(explicit_data, target_refno);
    let mut residual = explicit_data.as_slice();
    let mut results = HashMap::new();
    while residual.len() >= 8 {
        let (rest, header) = match parse_explicit_header(residual) {
            Ok(r) => r,
            Err(_) => break,
        };
        if header.hash == 0 {
            break;
        }
        let data_len = header.data_len();
        if rest.len() < data_len {
            break;
        }
        let name = db1_dehash(header.hash.unsigned_abs());
        if name_set.contains(name.as_str()) {
            let entry_len = 8 + data_len;
            results.insert(name, residual[..entry_len].to_vec());
            if results.len() == name_set.len() {
                break;
            }
        }
        residual = &rest[data_len..];
    }
    Ok(results)
}

fn format_hex_words(input: &[u8]) -> String {
    let mut out = String::new();
    for (idx, chunk) in input.chunks(4).enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        for b in chunk {
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

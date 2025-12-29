use std::path::PathBuf;

use parse_pdms_db::parse::parse_file;

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

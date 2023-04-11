use aios_core::pdms_types::RefU64;
use aios_core::tool::db_tool::db1_dehash;

#[test]
fn test_uda_dehash() {
    let hash = db1_dehash(0x2902D6E0);
    assert_eq!(":CNPEspco".to_string(),hash);
    let hash = db1_dehash(0xE473396C);
    let refno = RefU64(0xE473396C);
    dbg!(refno);
    assert_eq!(":3D_SJZT".to_string(),hash);
    let hash = db1_dehash(642951949);
    assert_eq!(":3D_SJRY".to_string(),hash);
}
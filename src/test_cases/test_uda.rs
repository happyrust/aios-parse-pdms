use aios_core::get_default_pdms_db_info;
use aios_core::pdms_types::RefU64;
use aios_core::tool::db_tool::{db1_dehash, read_attr_info_config_from_json};
use crate::parse::parse_ele_data;
use crate::test_cases::convert_str_to_bytes;

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

// ams desi 24381/48631
// 当前值  :4WO
// 期望值  :3D_SJRY
#[test]
fn test_parse_uda_data_24381_48631() {
    let data_str = "
    00 00 00 29 00 00 5C 20 00 00 15 D4 00 08 F3 A6
00 00 5C 20 00 00 15 D1 00 00 26 F5 00 05 40 01
00 00 00 00 00 00 00 00 20 08 C0 00 00 00 00 03
00 00 00 00 40 C1 FA 80 00 00 00 00 40 C8 06 00
00 00 00 00 40 8A 54 00 00 00 00 03 00 00 00 00
00 00 00 00 00 00 00 00 C0 56 80 00 00 00 00 00
00 00 00 00 00 00 00 0C 00 00 3B 58 00 03 80 68
00 00 3B 58 00 03 80 27 00 00 00 01 00 00 00 02
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 02 2F B0 E7 4C
80 00 00 01 00 01 00 28 00 00 5C 20 00 00 15 D4
00 00 00 00 00 00 00 00 00 0A AF CA 14 00 00 01
00 00 00 00 00 09 2E A7 0C 00 00 01 FF FF FF FF
00 0B C6 C0 14 00 00 01 00 00 00 01 06 A0 26 04
0C 00 00 01 00 0D F3 17 10 71 D1 20 08 00 00 02
00 00 00 00 00 00 00 00 10 71 D1 2B 08 00 00 02
00 00 00 00 00 00 00 00 00 0D FD 22 14 00 00 01
00 00 00 00 00 CC 6B 3F 38 00 00 02 00 00 00 01
00 08 F3 A6 00 08 DF C1 1C 00 00 02 00 00 00 01
00 00 00 00 2C F2 AE D5 28 00 00 02 00 00 00 04
74 65 73 74 00 00 00 00 00 00 00 00
    ";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = get_default_pdms_db_info();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map).unwrap();
    for (key,value) in ele_data.whole_attmap.explicit_attmap.map {
        dbg!(&key);
        dbg!(&value);
    }
}
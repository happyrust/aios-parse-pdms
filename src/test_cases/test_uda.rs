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

#[test]
fn test_parse_uda_data_24381_48631() {
    let data_str = "00 00 00 1C 00 00 5F 3D 00 00 BD F7 00 09 D6 5A
00 00 3F 3D 00 00 00 00 00 00 22 58 00 0A 40 01
00 00 22 58 00 07 60 01 20 07 40 12 00 08 1C F2
00 00 00 00 00 00 00 00 00 00 00 03 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 02 00 17 00 00 5F 3D 00 00 BD F7 00 00 00 00
00 00 00 00 00 00 5F 3D 00 00 BD F8 00 00 5F 3D
00 00 BF 31 00 00 5F 3D 00 00 C1 74 00 00 5F 3D
00 00 DB A3 00 00 5F 3D 00 00 DD 52 00 00 5F 3D
00 00 DD 73 00 00 5F 3D 00 00 DE BD 00 00 5F 3D
00 00 DF 50 00 00 5F 3D 00 00 E0 24 00 01 00 22
00 00 5F 3D 00 00 BD F7 00 00 00 00 00 00 00 00
00 CC 6B 3F 38 00 00 02 00 00 00 01 00 09 D6 5A
00 09 C1 8E 3C 00 00 04 00 00 00 0C 2F 31 43 41
56 2D 48 56 41 43 48 42 00 09 39 40 28 00 00 07
00 00 00 18 E7 8E AF E5 BD A2 E7 A9 BA E9 97 B4
E9 80 9A E9 A3 8E E7 B3 BB E7 BB 9F 00 09 2C B5
28 00 00 02 00 00 00 04 48 56 41 43 26 52 AB 0D
28 00 00 04 00 00 00 0B 7A 68 61 6E 67 73 68 75
61 69 61 00";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config_from_json("all_attr_info.json");
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map).unwrap();
    dbg!(&ele_data.whole_attmap.explicit_attmap);
}
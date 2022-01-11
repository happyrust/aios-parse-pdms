use crate::parse::parse_ele_data;
use crate::test_cases::{convert_str_to_bytes, read_attr_info_config};

#[test]
fn test_spine_aba_32769_21909() {
    let data_str = "
00 00 00 17 00 00 80 01 00 00 55 95 00 34 F7 74
00 00 80 01 00 00 55 94 00 00 00 00 00 00 00 00
00 01 C7 13 00 25 C0 01 00 00 00 06 00 00 00 03
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 00 3F 80 00 00 00 02 00 0B
00 00 80 01 00 00 55 95 00 00 00 00 00 00 00 00
00 00 80 01 00 00 55 96 00 00 80 01 00 00 55 97
00 00 80 01 00 00 55 98 00";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0x34F774i32) {
        dbg!(map.value());
    }
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map);
    dbg!(&ele_data);
}



//15192,
//     238890,
#[test]
fn test_sample_15192_238890() {
    let data_str = "
00 00 00 1A 00 00 3B 58 00 03 A5 2A 00 09 12 9A
00 00 3B 58 00 00 EA B1 00 00 85 6B 00 27 60 01
00 00 85 6B 00 26 80 01 20 07 C0 02 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
3F F0 00 00 00 00 00 00 00 09 CC A7 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 02 00 07 00 00 3B 58
00 03 A5 2A 00 00 00 00 00 00 00 00 00 00 3B 58
00 03 A5 2B 00 01 00 24 00 00 3B 58 00 03 A5 2A
00 00 00 00 00 00 00 00 05 57 E4 C6 40 00 00 02
00 00 00 00 00 00 00 00 00 3A 79 30 40 00 00 02
00 00 00 00 00 00 00 00 05 57 CA 1B 40 00 00 02
00 00 00 00 00 00 00 00 04 FF 0E BA 28 00 00 02
00 00 00 02 2D 31 00 00 11 D2 4E 19 28 00 00 03
00 00 00 07 55 6E 6B 6E 6F 77 6E 00 00 09 C1 8E
3C 00 00 04 00 00 00 0C 2F 44 52 30 37 43 5F 42
4F 4C 54 53 00 09 F8 00 28 00 00 02 00 00 00 04
4E 4F 4E 45";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0x34F774i32) {
        dbg!(map.value());
    }
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map);
    dbg!(&ele_data);
}
use aios_core::pdms_types::StringLookupTable;
use crate::parse::parse_ele_data;
// use crate::pdms_types::StringLookupTable;
use crate::read_attr_info_config;
use crate::test_cases::convert_str_to_bytes;

#[test]
fn test_sample_2013286748_1428() {
    // issue :https://gitee.com/happydpc/aios-parse-pdms/issues/I4QBEC
    let data_str = "00 00 00 2D 78 00 51 5C 00 00 05 94 00 0E 55 4B
78 00 51 5C 00 00 05 76 00 00 04 81 00 05 C0 01
00 00 00 00 00 00 00 00 20 06 C0 00 00 00 00 03
92 93 3E 18 C0 CA E3 02 32 FE CE EC 40 91 E5 BF
2B BD 07 40 40 8D 04 5A 00 00 00 03 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
40 66 80 00 00 00 00 0C 00 00 3B 58 00 03 86 75
00 00 3B 58 00 03 80 23 00 00 00 01 00 00 00 02
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 40 56 80 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 02 35 A5 11 77
80 00 00 01 00 01 00 20 78 00 51 5C 00 00 05 94
00 00 00 00 00 00 00 00 00 09 2E A7 0C 00 00 01
FF FF FF FF 00 0B C6 C0 14 00 00 01 00 00 00 01
06 A0 26 04 0C 00 00 01 00 0D F3 17 00 CC 6B 3F
38 00 00 02 00 00 00 01 00 0E 55 4B 00 09 C1 8E
3C 00 00 05 00 00 00 0F 2F 43 6F 70 79 2D 6F 66
2D 46 56 2D 31 31 35 00 29 02 D6 DA 28 00 00 05
00 00 00 0D 31 41 52 2D 52 4D 30 36 2D 41 36 32
35 00 00 00
";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0x85897i32) {
    //     //dbg!(map.value());
    // }
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    if let Some(value) = lookup.lookup.get(&1433536923){
        println!("string={:?}",value.value());
    }
    dbg!(&ele_data.attr_data_map.to_string_hashmap());
}

#[test]
fn test_24575_228_sample() {
    let data_str ="
00 00 00 0D 00 00 5F FF 00 00 00 E4 00 0C C3 A5
00 00 5F FF 00 00 00 DB 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 20 00 00 00 00 00 00 01
FF FF FF FF 31 41 52 2D 52 4D 30 36 2D 41 36 32
35 00 00 00";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xCC3A5) {
        dbg!(map.value());
    }
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    println!("ele_data={:?}",ele_data.attr_data_map.to_string_hashmap());
    // if let Some(value) = lookup.lookup.get(&1433536923){
    //     println!("string={:?}",value.value());
    // }
    // dbg!(&ele_data.attr_data_map.to_string_hashmap());
}
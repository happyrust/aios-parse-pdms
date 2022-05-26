use std::fs::File;
use std::io::Write;
use aios_core::pdms_types::{PdmsDatabaseInfo, };
use crate::parse::parse_ele_data;
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
    if let Some(map) = pdms_database_info.noun_attr_info_map.get(&(db1_hash("SECT") as i32)) {
        dbg!(map.value());
    };
    // let mut lookup = StringLookupTable::default();
    // let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    // if let Some(value) = lookup.lookup.get(&1433536923){
    //     println!("string={:?}",value.value());
    // }
    // dbg!(&ele_data.attr_data_map.to_string_hashmap());
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
    // let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let pdms_database_info = read_attr_info_config_json("all_attr_info.json");
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xCC3A5) {
    //     dbg!(map.value());
    // }
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    println!("ele_data={:?}",ele_data.whole_attmap);
    // if let Some(value) = lookup.lookup.get(&1433536923){
    //     println!("string={:?}",value.value());
    // }
    // dbg!(&ele_data.attr_data_map.to_string_hashmap());
}

#[test]
fn test_sample_mdb() {
    let data_str = "00 00 00 0B 00 00 5F FF 00 00 02 15 00 08 22 1C
00 00 5F FF 00 00 00 07 00 00 02 1F 00 1C 60 01
00 00 02 1F 00 11 80 01 20 2B C0 52 00 02 00 57
00 00 5F FF 00 00 02 15 00 00 00 00 00 00 00 00
00 00 5F FF 00 00 01 E7 00 00 5F FF 00 00 02 09
00 00 5F FF 00 00 01 F9 00 00 5F FF 00 00 01 F8
00 00 5F FF 00 00 02 01 00 00 5F FF 00 00 01 F4
00 00 5F FF 00 00 02 08 00 00 5F FF 00 00 01 E8
00 00 5F FF 00 00 02 0B 00 00 5F FF 00 00 02 0A
00 00 5F FF 00 00 02 0E 00 00 5F FF 00 00 02 0F
00 00 5F FF 00 00 02 10 00 00 5F FF 00 00 02 11
00 00 5F FF 00 00 02 12 00 00 5F FF 00 00 02 13
00 00 5F FF 00 00 02 14 00 00 5F FF 00 00 02 0C
00 00 5F FF 00 00 02 05 00 00 5F FF 00 00 01 E9
00 00 5F FF 00 00 01 D4 00 00 5F FF 00 00 01 E5
00 00 5F FF 00 00 01 D6 00 00 5F FF 00 00 01 D7
00 00 5F FF 00 00 01 D8 00 00 5F FF 00 00 01 DD
00 00 5F FF 00 00 01 DC 00 00 5F FF 00 00 01 D9
00 00 5F FF 00 00 01 DA 00 00 5F FF 00 00 01 E6
00 00 5F FF 00 00 01 DB 00 00 5F FF 00 00 01 DE
00 00 5F FF 00 00 01 E3 00 00 5F FF 00 00 02 0D
00 00 5F FF 00 00 02 07 00 00 5F FF 00 00 01 DF
00 00 5F FF 00 00 02 06 00 00 5F FF 00 00 01 E0
00 00 5F FF 00 00 01 E1 00 00 5F FF 00 00 01 E4
00 00 5F FF 00 00 01 E2 00 01 00 B4 00 00 5F FF
00 00 02 15 00 00 00 00 00 00 00 00 00 09 C1 8E
3C 00 00 03 00 00 00 07 2F 53 41 4D 50 4C 45 00
00 0D F3 30 20 00 00 53 00 00 00 29 00 00 5F FF
00 00 01 E2 00 00 5F FF 00 00 01 E4 00 00 5F FF
00 00 01 E1 00 00 5F FF 00 00 01 E0 00 00 5F FF
00 00 02 06 00 00 5F FF 00 00 01 DF 00 00 5F FF
00 00 02 07 00 00 5F FF 00 00 02 0D 00 00 5F FF
00 00 01 E3 00 00 5F FF 00 00 01 DE 00 00 5F FF
00 00 01 DB 00 00 5F FF 00 00 01 E6 00 00 5F FF
00 00 01 DA 00 00 5F FF 00 00 01 D9 00 00 5F FF
00 00 01 DC 00 00 5F FF 00 00 01 DD 00 00 5F FF
00 00 01 D8 00 00 5F FF 00 00 01 D7 00 00 5F FF
00 00 01 D6 00 00 5F FF 00 00 01 E5 00 00 5F FF
00 00 01 D4 00 00 5F FF 00 00 01 E9 00 00 5F FF
00 00 02 05 00 00 5F FF 00 00 02 0C 00 00 5F FF
00 00 02 14 00 00 5F FF 00 00 02 13 00 00 5F FF
00 00 02 12 00 00 5F FF 00 00 02 11 00 00 5F FF
00 00 02 10 00 00 5F FF 00 00 02 0F 00 00 5F FF
00 00 02 0E 00 00 5F FF 00 00 02 0A 00 00 5F FF
00 00 02 0B 00 00 5F FF 00 00 01 E8 00 00 5F FF
00 00 02 08 00 00 5F FF 00 00 01 F4 00 00 5F FF
00 00 02 01 00 00 5F FF 00 00 01 F8 00 00 5F FF
00 00 01 F9 00 00 5F FF 00 00 02 09 00 00 5F FF
00 00 01 E7 00 09 84 F9 20 00 00 53 00 00 00 29
00 00 5F FF 00 00 01 E7 00 00 5F FF 00 00 02 09
00 00 5F FF 00 00 01 F9 00 00 5F FF 00 00 01 F8
00 00 5F FF 00 00 02 01 00 00 5F FF 00 00 01 F4
00 00 5F FF 00 00 02 08 00 00 5F FF 00 00 01 E8
00 00 5F FF 00 00 02 0B 00 00 5F FF 00 00 02 0A
00 00 5F FF 00 00 02 0E 00 00 5F FF 00 00 02 0F
00 00 5F FF 00 00 02 10 00 00 5F FF 00 00 02 11
00 00 5F FF 00 00 02 12 00 00 5F FF 00 00 02 13
00 00 5F FF 00 00 02 14 00 00 5F FF 00 00 02 0C
00 00 5F FF 00 00 02 05 00 00 5F FF 00 00 01 E9
00 00 5F FF 00 00 01 D4 00 00 5F FF 00 00 01 E5
00 00 5F FF 00 00 01 D6 00 00 5F FF 00 00 01 D7
00 00 5F FF 00 00 01 D8 00 00 5F FF 00 00 01 DD
00 00 5F FF 00 00 01 DC 00 00 5F FF 00 00 01 D9
00 00 5F FF 00 00 01 DA 00 00 5F FF 00 00 01 E6
00 00 5F FF 00 00 01 DB 00 00 5F FF 00 00 01 DE
00 00 5F FF 00 00 01 E3 00 00 5F FF 00 00 02 0D
00 00 5F FF 00 00 02 07 00 00 5F FF 00 00 01 DF
00 00 5F FF 00 00 02 06 00 00 5F FF 00 00 01 E0
00 00 5F FF 00 00 01 E1 00 00 5F FF 00 00 01 E4
00 00 5F FF 00 00 01 E2 ";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config_json("all_attr_info.json");
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xCC3A5) {
    //     dbg!(map.value());
    // }
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    println!("ele_data={:?}",ele_data.whole_attmap);
}

#[test]
fn change_info_bin_file() {
    let info = bincode::deserialize::<PdmsDatabaseInfo>(include_bytes!("../../all_attr_info.bin")).unwrap();
    let mut file = File::create("all_attr_info_new.json").unwrap();
    let v = serde_json::to_string(&info).unwrap();
    file.write(v.as_bytes()).unwrap();
}

#[test]
fn test_room_code_sample() {
    let data_str = "00 00 00 29 78 00 51 5C 00 00 21 65 00 08 F3 A6
78 00 51 5C 00 00 21 64 00 00 09 56 00 15 00 01
00 00 00 00 00 00 00 00 20 04 80 00 00 00 00 03
7A E1 47 A0 40 C9 B0 74 47 AE 14 80 C0 CF 71 81
00 00 00 00 C0 A1 F8 00 00 00 00 03 00 00 00 00
40 56 80 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 0C 00 00 3B 6D 00 03 EF F2
00 00 3B 6D 00 03 EF E8 00 00 00 01 00 00 00 02
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 40 9F 40 00 00 00 00 02 11 E3 A0 D7
80 00 00 01 00 01 00 17 78 00 51 5C 00 00 21 65
00 00 00 00 00 00 00 00 00 09 2E A7 0C 00 00 01
FF FF FF FF 00 0B C6 C0 14 00 00 01 00 00 00 01
06 A0 26 04 0C 00 00 01 00 36 7E CC 00 CC 6B 3F
38 00 00 02 00 00 00 01 00 08 F3 A6 29 02 D6 DA
28 00 00 03 00 00 00 05 31 52 31 30 31 00 00 00 ";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config_json("all_attr_info.json");
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xCC3A5) {
    //     dbg!(map.value());
    // }
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup).unwrap();
    println!("ele_data={:?}",ele_data.whole_attmap);
}
use crate::parse::parse_ele_data;
use crate::test_cases::read_attr_info_config;
use crate::test_cases::test_branchs::convert_str_to_bytes;

#[test]
fn test_spine_aba_32769_21909() {
    let data_str="
00 00 00 17 00 00 80 01 00 00 55 95 00 34 F7 74
00 00 80 01 00 00 55 94 00 00 00 00 00 00 00 00
00 01 C7 13 00 25 C0 01 00 00 00 06 00 00 00 03
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 00 3F 80 00 00 00 02 00 0B
00 00 80 01 00 00 55 95 00 00 00 00 00 00 00 00
00 00 80 01 00 00 55 96 00 00 80 01 00 00 55 97
00 00 80 01 00 00 55 98 00";
    let data=convert_str_to_bytes(data_str);
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    if let Some(map)=pdms_database_info.noun_attr_info_map.get(&0x34F774i32){
        dbg!(map.value());
    }
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map);
    dbg!(&ele_data);
}
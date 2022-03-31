use crate::parse::parse_ele_data;
use crate::pdms_types::{AttrVal, StringLookupTable};
use crate::test_cases::{convert_str_to_bytes, read_attr_info_config};

#[test]
fn test_gdp_15194_134() {
    let data_str="
00 00 00 19 00 00 3B 5A 00 00 00 86 00 08 A1 E7
00 00 3B 5A 00 00 00 85 00 00 01 D9 00 0C 80 01
00 00 00 00 00 00 00 00 00 0C 00 00 00 0E 49 B8
00 0E 41 95 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 0E 49 B8
00 00 00 00 00 01 00 35 00 00 3B 5A 00 00 00 86
00 00 00 00 00 00 00 00 00 09 C1 8E 3C 00 00 09
00 00 00 1E 2F 43 41 44 43 48 56 41 43 43 41 54
41 2D 44 54 53 45 2D 43 4F 4D 4D 4F 4E 2D 50 4C
4F 54 00 00 00 0E 39 6E 28 00 00 03 00 00 00 08
50 6C 6F 74 66 69 6C 65 FF F3 2D C0 1C 00 00 10
00 00 00 0F 00 00 00 0F 00 00 00 01 00 00 00 6A
00 00 00 04 00 0C 2C A0 00 00 00 01 00 00 00 01
00 00 00 00 00 00 06 42 00 00 00 05 00 00 00 02
00 0D BC F9 00 00 00 01 00 00 00 05 00 00 06 A5
FF F3 2D CC 1C 00 00 0C 00 00 00 0B 00 00 00 0B
00 00 00 01 00 00 00 66 00 00 00 07 00 00 00 48
00 00 00 56 00 00 00 41 00 00 00 43 00 00 00 41
00 00 00 44 00 00 00 56 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PPRO") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( ATTRIB FLNM OF CATR  )",result);
}

#[test]
fn test_dbp_5194_136() {
    let data_str="
00 00 00 19 00 00 3B 5A 00 00 00 88 00 08 A1 E7
00 00 3B 5A 00 00 00 85 00 00 01 D9 00 1F 00 01
00 00 00 00 00 00 00 00 00 09 C0 00 00 0C ED E5
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 0D 20 C7
00 00 00 28 00 01 00 2C 00 00 3B 5A 00 00 00 88
00 00 00 00 00 00 00 00 00 09 C1 8E 3C 00 00 09
00 00 00 1E 2F 43 41 44 43 48 56 41 43 43 41 54
41 2D 44 54 53 45 2D 43 4F 4D 4D 4F 4E 2D 53 48
41 50 00 00 00 0E 39 6E 28 00 00 03 00 00 00 05
53 48 41 50 45 00 00 00 FF F3 2D C0 1C 00 00 12
00 00 00 11 00 00 00 11 00 00 00 01 00 00 00 65
00 00 00 06 00 00 50 00 00 00 00 00 00 00 00 06
00 00 00 00 00 00 00 06 00 00 00 6A 00 00 00 06
00 0D DF 8A FF FF FF FF FF FF FF FF 00 00 00 00
00 00 06 41 00 00 06 A5 00 09 D4 C4 0C 00 00 01
00 00 00 01 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PPRO") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( ATTRIB WDESP[40] )",result);
}

#[test]
fn test_gdp_15194_223() {
    let data_str="
00 00 00 21 00 00 3B 5A 00 00 00 DF 00 0F 56 3E
00 00 3B 5A 00 00 00 DE 00 00 01 E5 00 20 80 01
00 00 00 00 00 00 00 00 00 0E 00 00 00 00 00 01
00 00 00 04 00 00 00 28 00 00 00 07 00 00 03 9F
00 00 01 B9 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 02
00 00 00 02 00 00 00 03 00 00 00 00 00 00 00 00
00 00 00 00 00 01 00 3D 00 00 3B 5A 00 00 00 DF
00 00 00 00 00 00 00 00 00 09 C1 8E 3C 00 00 04
00 00 00 0B 2F 52 54 55 42 45 31 2D 50 41 31 00
00 09 6B 9B 1C 00 00 05 00 00 00 04 00 00 00 00
00 00 00 01 00 00 00 00 00 00 00 00 00 0A DF 11
1C 00 00 05 00 00 00 04 00 00 00 00 00 00 00 01
00 00 00 00 00 00 00 00 FF F2 51 1C 1C 00 00 22
00 00 00 21 00 00 00 21 00 00 00 01 00 00 00 65
00 00 00 06 00 00 40 00 00 00 00 00 00 00 00 02
00 00 00 00 00 00 00 06 00 00 00 6A 00 00 00 02
00 0D DF 77 FF FF FF FF FF FF FF FF 00 00 00 00
00 00 06 41 00 00 06 A5 00 00 00 65 00 00 00 06
00 00 60 00 00 00 00 00 00 00 00 02 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D DF 77
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 03 22 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    // dbg!(ele_data);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PPRO") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( ATTRIB WDESP[40] )",result);
}

#[test]
fn test_gdp_15194_287() {
    let data_str="
00 00 00 1D 00 00 3B 5A 00 00 01 15 00 0B 15 DB
00 00 3B 5A 00 00 01 14 00 00 01 EC 00 2A 40 01
00 00 00 00 00 00 00 00 00 0B C0 00 00 00 00 02
00 00 00 02 00 00 00 00 00 00 00 0A 00 00 00 0F
00 00 00 02 00 00 00 03 00 00 00 02 00 00 00 04
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 04 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 01 00 34 00 00 3B 5A 00 00 01 15
00 00 00 00 00 00 00 00 00 09 C1 8E 3C 00 00 05
00 00 00 0D 2F 48 52 54 55 42 45 32 2D 42 4F 58
49 00 00 00 FF F6 3E DC 1C 00 00 12 00 00 00 11
00 00 00 11 00 00 00 01 00 00 00 65 00 00 00 06
00 00 40 00 00 00 00 00 00 00 00 01 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 17 EF 4B 61
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 FF F6 3E A6 1C 00 00 12 00 00 00 11
00 00 00 11 00 00 00 01 00 00 00 65 00 00 00 06
00 00 40 00 00 00 00 00 00 00 00 02 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 17 EF 4B 61
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    // dbg!(ele_data);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PZLE") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( ATTRIB :HXYsize[2] )",result);
}

#[test]
fn test_gdp_15194_8039(){
    let data_str="
00 00 00 2E 00 00 3B 5A 00 00 1F 67 00 0F 7C 39
00 00 3B 5A 00 00 1F 66 00 00 09 69 00 15 60 01
00 00 09 69 00 14 80 01 00 09 40 02 00 00 00 02
00 00 00 02 00 00 00 00 00 00 00 0A 00 00 00 03
00 00 00 04 00 00 00 00 00 00 00 01 00 00 00 00
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 01
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 04
00 00 00 28 00 00 00 01 00 00 00 34 00 00 00 00
00 00 00 04 00 00 00 28 00 00 00 01 00 00 00 35
00 00 00 00 00 00 00 04 00 00 00 28 00 00 00 01
00 00 00 38 00 00 00 00 00 02 00 07 00 00 3B 5A
00 00 1F 67 00 00 00 00 00 00 00 00 00 00 3B 5A
00 00 1F 68 00 01 00 2A 00 00 3B 5A 00 00 1F 67
00 00 00 00 00 00 00 00 00 0D 17 DA 0C 00 00 01
00 00 00 02 00 09 C1 8E 3C 00 00 04 00 00 00 0C
2F 55 52 53 54 52 41 2D 42 4F 44 59 FF F7 E1 41
1C 00 00 1A 00 00 00 19 00 00 00 19 00 00 00 01
00 00 00 65 00 00 00 06 00 00 60 00 00 00 00 00
00 00 00 03 00 00 00 00 00 00 00 06 00 00 00 6A
00 00 00 02 00 0D 20 C7 FF FF FF FF FF FF FF FF
00 00 00 00 00 00 06 41 00 00 06 A5 00 00 00 65
00 00 00 06 00 00 40 00 00 00 00 00 00 00 00 02
00 00 00 00 00 00 00 06 00 00 03 25 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xF7C39i32) {
    //     dbg!(map.value());
    // }
    // dbg!(ele_data);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PZLE") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("DESIGN PARAM 6",result);
}

#[test]
fn test_gdp_15194_8204() {
    let data_str="
00 00 00 2E 00 00 3B 5A 00 00 20 0C 00 0F 7C 39
00 00 3B 5A 00 00 1F DD 00 00 09 7F 00 31 80 01
00 00 09 7F 00 30 A0 01 00 30 C0 02 00 00 00 02
00 00 00 02 00 00 00 06 00 00 00 0A 00 00 00 03
00 00 00 04 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 01
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 04
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 04 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 02 00 07 00 00 3B 5A
00 00 20 0C 00 00 00 00 00 00 00 00 00 00 3B 5A
00 00 20 0D 00 01 00 74 00 00 3B 5A 00 00 20 0C
00 00 09 80 00 00 20 01 00 09 C1 8E 3C 00 00 05
00 00 00 0E 2F 55 52 53 4F 46 46 53 2D 4C 45 41
4A 32 00 00 FF F7 E1 77 1C 00 00 12 00 00 00 11
00 00 00 11 00 00 00 01 00 00 00 65 00 00 00 06
00 00 50 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 FF F7 E1 41 1C 00 00 2A 00 00 00 29
00 00 00 29 00 00 00 01 00 00 00 65 00 00 00 06
00 00 60 00 00 00 00 00 00 00 00 03 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 00 65 00 00 00 06 00 00 64 00
00 00 00 00 00 00 00 05 00 00 00 00 00 00 00 06
00 00 00 6A 00 00 00 02 00 0D 20 C7 FF FF FF FF
FF FF FF FF 00 00 00 00 00 00 06 41 00 00 06 A5
00 00 00 65 00 00 00 06 00 00 40 00 00 00 00 00
00 00 00 02 00 00 00 00 00 00 00 06 00 00 03 25
00 00 03 23 FF F6 3E DC 1C 00 00 32 00 00 00 31
00 00 00 31 00 00 00 01 00 00 00 65 00 00 00 06
00 00 40 00 00 00 00 00 00 00 00 03 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 00 65 00 00 00 06 00 00 68 00
00 00 00 00 00 00 00 05 00 00 00 00 00 00 00 06
00 00 00 6A 00 00 00 02 00 0D 20 C7 FF FF FF FF
FF FF FF FF 00 00 00 00 00 00 06 41 00 00 06 A5
00 00 03 22 00 00 00 65 00 00 00 06 00 00 68 00
00 00 00 00 00 00 00 07 00 01 00 59 00 00 3B 5A 00 00 20 0C
00 00 00 00 00 00 00 00 00 00 00 05 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 03 22 FF F6 3E C1 1C 00 00 32
00 00 00 31 00 00 00 31 00 00 00 01 00 00 00 65
00 00 00 06 00 00 50 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 06 00 00 00 6A 00 00 00 02
00 0D 20 C7 FF FF FF FF FF FF FF FF 00 00 00 00
00 00 06 41 00 00 06 A5 00 00 00 65 00 00 00 06
00 00 68 00 00 00 00 00 00 00 00 05 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 03 22 00 00 00 65 00 00 00 06
00 00 68 00 00 00 00 00 00 00 00 05 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0D 20 C7
FF FF FF FF FF FF FF FF 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 03 22 FF F6 3E A6 1C 00 00 12
00 00 00 11 00 00 00 11 00 00 00 01 00 00 00 65
00 00 00 06 00 00 64 00 00 00 00 00 00 00 00 05
00 00 00 00 00 00 00 06 00 00 00 6A 00 00 00 02
00 0D 20 C7 FF FF FF FF FF FF FF FF 00 00 00 00
00 00 06 41 00 00 06 A5 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PZ") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( ( ATTRIB DESP[6] - ATTRIB DESP[25]/2 ) )",result);
}

#[test]
fn test_gdp_15194_5814() {
    let data_str="
00 00 00 27 00 00 3B 5A 00 00 16 B6 00 0B EE BF
00 00 3B 5A 00 00 16 B5 00 00 08 3D 00 2C 60 01
00 00 00 00 00 00 00 00 00 1F 80 00 00 00 00 00
00 00 00 02 00 00 00 09 00 00 00 0A 00 00 00 0F
00 00 00 04 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 02
00 00 00 03 00 00 03 ED 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 FF FF FF FF 00 01 00 83
00 00 3B 5A 00 00 16 B6 00 00 00 00 00 00 00 00
00 09 C1 8E 3C 00 00 05 00 00 00 10 2F 43 35 53
45 47 42 2D 4E 49 4E 53 53 45 47 4C FF F7 7D 0F
1C 00 00 1A 00 00 00 19 00 00 00 19 00 00 00 01
00 00 00 65 00 00 00 06 00 00 40 00 00 00 00 00
00 00 00 02 00 00 00 00 00 00 00 06 00 00 00 6A
00 00 00 02 00 0D 20 C7 FF FF FF FF FF FF FF FF
00 00 00 00 00 00 06 41 00 00 06 A5 00 00 00 65
00 00 00 06 00 00 50 00 00 00 00 00 00 00 00 03
00 00 00 00 00 00 00 06 00 00 03 23 FF F5 20 EF
1C 00 00 3C 00 00 00 3B 00 00 00 3B 00 00 00 01
00 00 00 65 00 00 00 06 00 00 40 00 00 00 00 00
00 00 00 02 00 00 00 00 00 00 00 06 00 00 00 6A
00 00 00 02 00 0D 20 C7 FF FF FF FF FF FF FF FF
00 00 00 00 00 00 06 41 00 00 06 A5 00 00 00 65
00 00 00 06 00 00 40 00 00 00 00 00 00 00 00 02
00 00 00 00 00 00 00 06 00 00 03 25 00 00 00 65
00 00 00 06 00 00 48 00 00 00 00 00 00 00 00 04
00 00 00 00 00 00 00 06 00 00 00 6A 00 00 00 02
00 0D 20 C7 FF FF FF FF FF FF FF FF 00 00 00 00
00 00 06 41 00 00 06 A5 00 00 03 22 00 00 00 6A
00 00 00 02 00 0B CB FF 00 00 00 01 00 00 00 01
00 00 00 00 00 00 06 41 00 00 06 A5 00 00 00 65
00 00 00 06 00 00 40 00 00 00 00 00 00 00 00 04
00 00 00 00 00 00 00 06 00 00 03 25 00 00 03 87
00 00 03 24 FF F1 F3 8F 1C 00 00 1B 00 00 00 1A
00 00 00 1A 00 00 00 01 00 00 00 65 00 00 00 06
FF FF C0 00 00 00 00 00 00 00 00 01 00 00 00 00
00 00 00 06 00 00 00 6A 00 00 00 02 00 0B CB FF
00 00 00 01 00 00 00 01 00 00 00 00 00 00 06 41
00 00 06 A5 00 00 03 24 00 00 00 65 00 00 00 06
00 00 40 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 06 00 00 03 25 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    // if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xBEEBFi32) {
    //     dbg!(map.value());
    // }
    // dbg!(ele_data);
    let mut result="".to_string();
    if let Some(r)=ele_data.attr_data_map.get_val("PYTS") {
        match r {
            AttrVal::StringType(v) => {
                result=v.to_string();
            }
            _ => {}
        }
    }
    assert_eq!("( -1 * ATTRIB ANGL/8 )",result);
}

#[test]
fn test_sample_23984_1066() {
    let data_str="
00 00 00 27 00 00 5D B0 00 00 04 2A 00 0B EE BF
00 00 5D B0 00 00 04 29 00 00 03 60 00 10 E0 01
00 00 00 00 00 00 00 00 20 07 40 00 00 00 00 02
00 00 00 02 00 00 00 00 00 00 00 0A 00 00 00 0C
00 00 00 04 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02
00 00 00 01 00 00 00 02 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 FF FF FF FF 00 01 00 22
00 00 5D B0 00 00 04 2A 00 00 00 00 00 00 00 00
FF F1 F3 AA 1C 00 00 1B 00 00 00 1A 00 00 00 1A
00 00 00 01 00 00 00 65 00 00 00 06 40 10 00 00
00 00 00 00 40 00 03 FF 00 00 00 00 00 00 00 06
00 00 00 6A 00 00 00 02 00 0B CB FF 00 00 00 01
00 00 00 01 00 00 00 00 00 00 06 41 00 00 06 A5
00 00 03 24 00 00 00 65 00 00 00 06 00 00 00 00
00 00 00 00 40 00 04 02 00 00 00 00 00 00 00 06
00 00 03 25 ";
    let data=convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    dbg!(ele_data);
}


#[test]
fn test_sample_15213_499928_12_1() {
    let data_str="
00 00 00 41 00 00 3B 6D 00 07 A0 D8 00 0D CC D4
00 00 3B 6D 00 07 A0 D6 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 20 00 00 00 00 00 00 02
00 00 00 02 00 00 00 00 00 00 00 0A 00 00 00 0F
00 00 00 02 00 00 00 01 00 00 00 01 00 00 00 02
00 00 00 01 00 00 00 03 00 00 00 02 00 00 00 03
00 00 03 E9 00 00 00 04 00 00 00 00 00 00 00 01
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 00
00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 04
00 00 00 28 00 00 00 03 00 00 00 3F 00 00 00 3E
00 00 00 04 00 00 00 28 00 00 00 01 00 00 00 3D
00 00 00 00 00 00 00 04 00 00 00 00 00 00 00 01
00 00 00 00 00 00 00 00 00 00 00 04 00 00 00 28
00 00 00 01 00 00 00 37 00 00 00 00 00 00 00 04
00 00 00 28 00 00 00 01 00 00 00 34 00 00 00 00
00 00 00 04 00 00 00 28 00 00 00 01 00 00 00 35
00 00 00 00 00 00 00 00 10 10 00 00 00 10 00 01
00 00 00 07 00 00 00 41 00 00 3B 6D 00 07 A0 D9";
    let data = convert_str_to_bytes(data_str);
    let pdms_database_info = read_attr_info_config("all_attr_info.bin");
    if let Some(map) = pdms_database_info.noun_attr_info_map.get(&0xDCCD4) {
        dbg!(map.value());
    };
    let mut lookup = StringLookupTable::default();
    let ele_data = parse_ele_data(data.as_slice(), &pdms_database_info.noun_attr_info_map,&mut lookup,0);
    dbg!(ele_data.attr_data_map.to_string_hashmap());
}
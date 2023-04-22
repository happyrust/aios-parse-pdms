use std::fs::File;
use std::io::Read;
use aios_core::pdms_types::PdmsDatabaseInfo;
use aios_core::tool::db_tool::read_attr_info_config_from_json;


pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str.trim().split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}


#[cfg(test)]
mod test_expression;
mod test_data_new;
mod test_nom;
mod test_chinese;
mod test_uda;
mod test_parse_element;
// pub mod test_database;


use std::fs::File;
use std::io::Read;
// use crate::pdms_types::PdmsDatabaseInfo;

pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str.trim().split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}


#[cfg(test)]
mod test_expression;
mod test_data_new;
mod test_nom;
// pub mod test_database;

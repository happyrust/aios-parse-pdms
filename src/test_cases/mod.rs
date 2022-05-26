use std::fs::File;
use std::io::Read;
// use crate::pdms_types::PdmsDatabaseInfo;

pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str.trim().split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}

// #[cfg(test)]
// mod test_branchs;
// #[cfg(test)]
// mod test_double_or_float;
// #[cfg(test)]
// mod test_data;
#[cfg(test)]
mod test_expression;

mod test_data_new;

mod test_nom;


// mod test_string_lookup;
pub mod test_database;

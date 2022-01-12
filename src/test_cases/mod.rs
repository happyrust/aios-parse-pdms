use std::fs::File;
use std::io::Read;
use crate::pdms_types::PdmsDatabaseInfo;

pub fn convert_str_to_bytes(data_str: &str) -> Vec<u8> {
    data_str.trim().split_whitespace().map(|s| u8::from_str_radix(s, 16).unwrap())
        .collect()
}

#[cfg(test)]
mod test_branchs;
#[cfg(test)]
mod test_double_or_float;
#[cfg(test)]
mod test_data;

mod test_nom;


fn read_attr_info_config(config_path: &str) -> PdmsDatabaseInfo{
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    bincode::deserialize(&attr_buf).unwrap()
}
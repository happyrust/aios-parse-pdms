use std::fs::File;
use std::io::Read;
use crate::pdms_types::PdmsDatabaseInfo;


#[cfg(test)]
mod test_branchs;


fn read_attr_info_config(config_path: &str) -> PdmsDatabaseInfo{
    let mut file = File::open(config_path).unwrap();
    let mut attr_buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut attr_buf);
    bincode::deserialize(&attr_buf).unwrap()
}
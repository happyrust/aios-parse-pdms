


#[inline]
pub fn combine_to_u64(a: u32, b: u32) -> u64{
    let bytes: Vec<u8> = [a.to_be_bytes(), b.to_be_bytes()].concat();
    u64::from_be_bytes(bytes[..8].try_into().unwrap())
}

// #[inline]
// pub fn split_to_two_u32(a: u32, b: u32) -> u64{
//     let bytes: Vec<u8> = [b.to_be_bytes(), b.to_be_bytes()].concat();
//     let k = u64::from_be_bytes(bytes[..8].try_into().unwrap());
// }
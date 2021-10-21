#[inline]
pub fn db1_dehash(hash: u32) -> String{
    let mut result = String::new();
    if hash > 0x171FAD39 { // UDA的情况
        let mut v8 = ((hash - 0x171FAD39) % 0x1000000) as i32;
        result.push(':');
        for i in 0..6 {
            if v8 <= 0{
                break;
            }
            result.push(((v8 & 0x3F) + 32) as u8 as char);
            v8 /= 64;
        }
    } else {
        let mut v6 = (hash - 0x81BF1) as i32;
        while v6 > 0 {
            result.push((v6 % 27 + 64) as u8  as char);
            v6 /= 27;
        }
    }
    result
}

#[test]
fn db1_dehash_test(){
    let name=db1_dehash(0xC551C);
    println!("name={}",name);
}
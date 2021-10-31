use crate::DbAttributeType;
use crate::pdms_types::DbAttributeType::*;

#[inline]
/// 若在反序列化给定的map集合中未找到该属性对应的hash ，则用该方法直接获取到该属性的数据类型 ，然后进行解析
pub fn get_explicit_attr_type(input: &[u8]) -> Option<DbAttributeType> {
    match input {
        &[0x3C] => { Some(STRING) }
        &[0x18] => { Some(DOUBLEVEC) }
        &[0x1C] => { Some(INTVEC) }
        &[0x40] => { Some(ELEMENT) }
        &[0x0C] => { Some(INTEGER) }
        &[0x14] => { Some(BOOL) }
        &[0x08] => { Some(DOUBLE) }
        &_ => {
            println!("failed to find explicit attr type {:#04X?}",input);
            None
        }
    }
}
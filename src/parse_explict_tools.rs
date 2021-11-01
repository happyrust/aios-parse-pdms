use crate::DbAttributeType;
use crate::pdms_types::DbAttributeType::*;

#[inline]
/// 若在反序列化给定的map集合中未找到该属性对应的hash ，则用该方法直接获取到该属性的数据类型 ，然后进行解析
pub fn get_explicit_attr_type(input: u16,pos:usize) -> Option<DbAttributeType> {
    match input {
        // 2800 这个应该是个引用，数据给的是一个参考号 ，但是e3d没有这个属性值 ，但是他的类型不难看出是string   类型: 2C F2 AE D3
        0x3C00 | 0x2800=> { Some(STRING) }
        0x1800 => { Some(DOUBLEVEC) }
        0x1C00 => { Some(INTVEC) }
        0x4000 => { Some(ELEMENT) }
        0x0C00 => { Some(INTEGER) }
        0x1400 => { Some(BOOL) }
        0x0800 => { Some(DOUBLE) }
        0x3800 => { Some(TYPEX) }
        _ => {
            println!("failed to find explicit attr type {:#04X?} position={:#04X?}",input,pos);
            None
        }
    }
}
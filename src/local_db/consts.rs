use crate::db_tool::db1_hash;

pub const BOX_NOUN: u32 = db1_hash("BOX");
pub const CYLI_NOUN: u32 = db1_hash("CYLI");
pub const SPHE_NOUN: u32 = db1_hash("SPHE");
pub const CONE_NOUN: u32 = db1_hash("CONE");
pub const DISH_NOUN: u32 = db1_hash("DISH");
pub const CTOR_NOUN: u32 = db1_hash("CTOR");
pub const RTOR_NOUN: u32 = db1_hash("RTOR");
pub const PYRA_NOUN: u32 = db1_hash("PYRA");
pub const LOOP_NOUN: u32 = db1_hash("LOOP");
pub const PLOO_NOUN: u32 = db1_hash("PLOO");
pub const SPINE_NOUN: u32 = db1_hash("SPINE");
pub const GENSEC_NOUN: u32 = db1_hash("GENSEC");
pub const POHE_NOUN: u32 = db1_hash("POHE");    //多边形的处理
pub const REVO_NOUN: u32 = db1_hash("REVO");
pub const NREV_NOUN: u32 = db1_hash("NREV");   //todo 负实体，后面需要加入

pub struct ArrayPair<T, const N: usize> {
    pub left: [T; N],
    // right: [T; N],
}

pub const T1: ArrayPair<u32, 3> = ArrayPair::<u32, 3>{
    left: [0u32; 3],
};

pub const T2: ArrayPair<u32, 2> = ArrayPair::<u32, 2>{
    left: [0u32; 2],
};

// fn foo<const N: usize>() {}
//
// fn bar<T, const M: usize>() {
//     foo::<M>(); // ok: `M` is a const parameter
//     foo::<2021>(); // ok: `2021` is a literal
//     foo::<{20 * 100 + 20 * 10 + 1}>(); // ok: const expression contains no generic parameters
//
//     foo::<{ M + 1 }>(); // error: const expression contains the generic parameter `M`
//     foo::<{ std::mem::size_of::<T>() }>(); // error: const expression contains the generic parameter `T`
//
//     let _: [u8; M]; // ok: `M` is a const parameter
//     // let _: [(); std::mem::size_of::<T>()]; // error: const expression contains the generic parameter `T`
// }
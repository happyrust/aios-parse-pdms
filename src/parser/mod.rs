//! PDMS 解析器模块
//!
//! 本模块提供 PDMS 数据库文件的解析功能，采用 nom 组合子库实现。
//!
//! # 模块结构
//!
//! - `primitives` - 基础类型解析器（RefU64, Hash, String 等）
//! - `numeric` - 数值解析器（f32/f64/带标志位数值）
//! - `combinator` - 自定义组合子

pub mod combinator;
pub mod numeric;
pub mod primitives;

// 重新导出常用类型和函数
pub use combinator::*;
pub use numeric::*;
pub use primitives::*;

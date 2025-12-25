#![feature(type_ascription)]
#![feature(array_methods)]
#![feature(slice_pattern)]
#![feature(associated_type_bounds)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#[macro_use]
extern crate approx;
#[allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused,
    missing_docs,
    unused_results,
    unused_must_use
)]
#[allow(unused_mut)]
#[macro_use]
extern crate bitflags;
extern crate core;
#[macro_use]
extern crate derivative;
extern crate hash32;
#[macro_use]
extern crate hash32_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;


pub use parse::parse_pdms_dir;
pub use parse::{parse_db, parse_file};

pub mod consts;
pub mod error_types;
pub mod parse;
pub mod parse_explict_tools;
pub mod parser;
#[cfg(test)]
pub mod test_cases;

pub type BHashMap<K, V> = std::collections::HashMap<K, V>;

#![feature(type_ascription)]
#![feature(array_methods)]
#![feature(slice_pattern)]
#![feature(core_intrinsics)]
#![feature(associated_type_bounds)]
#![feature(once_cell)]
#![feature(async_closure)]
#![feature(generic_const_exprs)]
#![feature(default_free_fn)]
#![feature(exclusive_range_pattern)]
#[allow(dead_code, unused_imports, unused_variables, unused_imports, unused, missing_docs, unused_results, unused_must_use)]
#[allow(unused_mut)]
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate approx;
#[macro_use]
extern crate hash32_derive;
extern crate hash32;

use std::collections::HashSet;
use std::error::Error;
use crate::parsed_data::GeomsInfo;
use futures::stream::TryStreamExt;
pub use parse::parse_pdms_dir;

pub mod error_types;
pub mod test_cases;
pub mod parse_explict_tools;
pub mod parsed_data;
pub mod parse;
pub mod consts;
pub mod shape;
pub mod grpc;

pub use parse::{parse_db, parse_file};
use std::time::Instant;
use aios_core::tool::db_tool::db1_hash;

pub mod options;

pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;

const ATT_PAXI: i32 = 0xB146F;
const ATT_PAAX: i32 = 0xF543D;
const ATT_PBAX: i32 = 0xF5458;
const ATT_PCAX: i32 = 0xF5473;
const ATT_PLAX: i32 = db1_hash("PLAX") as i32;

const ATT_PX: i32 = 0xFFF7E177u32 as i32;
const ATT_PY: i32 = 0xFFF7E15Cu32 as i32;
const ATT_PZ: i32 = 0xFFF7E141u32 as i32;
const ATT_PDIA: i32 = 0xFFF77D0Fu32 as i32;
const ATT_PHEI: i32 = 0xFFF520EFu32 as i32;
const ATT_PDIS: i32 = 0xFFF21519u32 as i32;
const ATT_PCON: i32 = 0xFFF3848Du32 as i32;
const ATT_PBOR: i32 = 0xFFF2511Cu32 as i32;
const ATT_PPRO: i32 = 0xFFF32DC0u32 as i32;
const ATT_DPRO: i32 = 0xFFF32DCCu32 as i32;
const ATT_BTHK: i32 = 0xFFF47D68u32 as i32;
const ATT_BDIA: i32 = 0xFFF77D1Du32 as i32;
const ATT_PTDI: i32 = 0xFFF52284u32 as i32;
const ATT_PBDI: i32 = 0xFFF5246Au32 as i32;
const ATT_PBTP: i32 = 0xFFF2DCA5u32 as i32;
const ATT_PCTP: i32 = 0xFFF2DC8Au32 as i32;
const ATT_PBBT: i32 = 0xFFF1DC5Bu32 as i32;
const ATT_PCBT: i32 = 0xFFF1DC40u32 as i32;
const ATT_PXLE: i32 = 0xFFF63EDCu32 as i32;
const ATT_PYLE: i32 = 0xFFF63EC1u32 as i32;
const ATT_PZLE: i32 = 0xFFF63EA6u32 as i32;
const ATT_PTDM: i32 = 0xFFF3EEF8u32 as i32;
const ATT_PBDM: i32 = 0xFFF3F0DEu32 as i32;
const ATT_POFF: i32 = 0xFFF60402u32 as i32;
const ATT_PTCDI: i32 = 0x95A34;
const ATT_DX: i32 = 0xFFF7E183u32 as i32;
const ATT_DY: i32 = 0xFFF7E168u32 as i32;
const ATT_PXTS: i32 = 0xFFF1F3AAu32 as i32;
const ATT_PYTS: i32 = 0xFFF1F38Fu32 as i32;
const ATT_PXBS: i32 = 0xFFF226ECu32 as i32;
const ATT_PYBS: i32 = 0xFFF226D1u32 as i32;
const ATT_PRAD: i32 = db1_hash("PRAD") as i32;
const ATT_DRAD: i32 = db1_hash("DRAD") as i32;
const ATT_PWID: i32 = db1_hash("PWID") as i32;

const ATT_PANG: i32 = 0xA5E2F;
const IMP_PAXI: i32 = 0xB146F;
const IMP_PCON: i32 = 0xC7B73;
const IMP_PDIS: i32 = 0xDEAE7;
const IMP_PBOR: i32 = 0xDAEE4;
const IMP_PDIA: i32 = 0x882F1;
const IMP_PHEI: i32 = 0xADF11;
const IMP_PTDI: i32 = 0xADD7C;
const IMP_PTDM: i32 = 0xC1108;
const IMP_PBDI: i32 = 0xADB96;
const IMP_PBDM: i32 = 0xC0F22;
const IMP_PPRO: i32 = 0xCD240;
const IMP_PRAD: i32 = 0x9544C;
const IMP_PX: i32 = 0x81E89;
const IMP_PY: i32 = 0x81EA4;
const IMP_PZ: i32 = 0x81EBF;
const IMP_PXLE: i32 = 0x9C124;
const IMP_PYLE: i32 = 0x9C13F;
const IMP_PZLE: i32 = 0x9C15A;
const IMP_PBTP: i32 = 0xD235B;
const IMP_PCTP: i32 = 0xD2376;
const IMP_PCBT: i32 = 0xE23C0;
const IMP_PBBT: i32 = 0xE23A5;
const IMP_PBOF: i32 = 0xA1440;
const IMP_PCOF: i32 = 0xA145B;
const IMP_PTCDI: i32 = 0x95A34;
const IMP_POFF: i32 = 0x9FBFE;
const IMP_DX: i32 = 0x81E7D;
const IMP_DY: i32 = 0x81E98;
const IMP_PLAX: i32 = 0xF5566;
const IMP_PXTS: i32 = 0xE0C56;
const IMP_PYTS: i32 = 0xE0C71;
const IMP_PXBS: i32 = 0xDD914;
const IMP_PYBS: i32 = 0xDD92F;
const IMP_DPRO: i32 = 0xCD234;

#[macro_use]
extern crate lazy_static;
extern crate core;

lazy_static! {
    pub static ref EXPR_ATT_SET: HashSet<i32> = {
        let mut s = HashSet::new();
        s.insert(ATT_PAXI);s.insert(ATT_PAAX);s.insert(ATT_PBAX);s.insert(ATT_PCAX);
        s.insert(ATT_PLAX);
        s.insert(ATT_PX);s.insert(ATT_PY);s.insert(ATT_PZ);s.insert(ATT_PDIA);
        s.insert(ATT_PHEI);s.insert(ATT_PDIS);s.insert(ATT_PCON);s.insert(ATT_PBOR);
        s.insert(ATT_PPRO);s.insert(ATT_DPRO);s.insert(ATT_BTHK);s.insert(ATT_BDIA);
        s.insert(ATT_PTDI);s.insert(ATT_PBDI);s.insert(ATT_PBTP);s.insert(ATT_PCTP);
        s.insert(ATT_PBBT);s.insert(ATT_PCBT);s.insert(ATT_PXLE);s.insert(ATT_PYLE);
        s.insert(ATT_PZLE);s.insert(ATT_PTDM);s.insert(ATT_PBDM);s.insert(ATT_PTCDI);
        s.insert(ATT_POFF);s.insert(ATT_DX);s.insert(ATT_DY);s.insert(ATT_DY);
        s.insert(ATT_PXTS);s.insert(ATT_PYTS);s.insert(ATT_PXBS);s.insert(ATT_PYBS);
        s.insert(ATT_PRAD);s.insert(ATT_PWID);s.insert(ATT_DRAD);

        s.insert(IMP_PAXI);s.insert(IMP_PCON);s.insert(IMP_PDIS);s.insert(IMP_PBOR);
        s.insert(IMP_PDIA);s.insert(IMP_PHEI);s.insert(IMP_PTDI);s.insert(IMP_PTDM);
        s.insert(IMP_PBDI);s.insert(IMP_PBDM);s.insert(IMP_PPRO);s.insert(IMP_PRAD);
        s.insert(IMP_PX);s.insert(IMP_PY);s.insert(IMP_PZ);s.insert(IMP_PXLE);
        s.insert(IMP_PYLE);s.insert(IMP_PZLE);s.insert(IMP_PCTP);s.insert(IMP_PCBT);
        s.insert(IMP_PBBT);s.insert(IMP_PBOF);s.insert(IMP_PCOF);s.insert(IMP_PBTP);
        s.insert(IMP_PTCDI);s.insert(IMP_POFF);s.insert(IMP_DX);s.insert(IMP_DY);
        s.insert(IMP_PLAX);s.insert(IMP_PXTS);s.insert(IMP_PYTS);s.insert(IMP_PXBS);
        s.insert(IMP_PYBS);s.insert(ATT_PANG);s.insert(IMP_DPRO);
        s
    };
}

//cached functions to get value
//todo 数据分层，尽可能的用缓存

//todo wasm need use feature
// pub fn init_pdms_db(db_option: &DbOption) -> anyhow::Result<AiosDBManager> {
//     let mut time = Instant::now();
//     let mut db_manager = AiosDBManager::init(db_option).unwrap();
//
//     println!("初始化数据库时间: {} ms", time.elapsed().as_millis());
//     let refno = RefU64::from_two_nums(16395, 39308);
//     //cached 一些常用的取值操作
//     let refno_info = db_manager.get_refno_info(refno);
//     //dbg!(&refno_info);
//
//     return Ok(db_manager);
// }









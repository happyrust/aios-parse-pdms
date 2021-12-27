#![feature(type_ascription)]
#![feature(array_methods)]

use mongodb::Client;
use mongodb::bson::doc;
use std::collections::HashSet;
use std::error::Error;
use crate::query_scom::{query_descomp_info, resolve_cata_comp_attrs};
use crate::pdms_parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, EleDataNode, ElementData, PdmsRefno};
use futures::stream::TryStreamExt;

pub mod pdms_types;
pub mod db_tool;
pub mod parse_explict_tools;
pub mod query_scom;
pub mod get_attr_tool;
pub mod pdms_parsed_data;
pub mod pdms_origin_data;
pub mod param_parse;
pub mod polish_notation;
pub mod direction_parse;
pub mod parse_data_impl;
pub mod parse_data_to_db;
pub mod interface;

const ATT_PAXI: i32 = 0xB146F;
const ATT_PAAX: i32 = 0xF543D;
const ATT_PBAX: i32 = 0xF5458;
const ATT_PCAX: i32 = 0xF5473;
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
const ATT_PTCDI: i32 = 0x95A34;

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

#[macro_use]
extern crate lazy_static;

lazy_static! {
    pub static ref EXPRESSION: HashSet<i32> = {
        let mut s = HashSet::new();
        s.insert(ATT_PAXI);s.insert(ATT_PAAX);s.insert(ATT_PBAX);s.insert(ATT_PCAX);
        s.insert(ATT_PX);s.insert(ATT_PY);s.insert(ATT_PZ);s.insert(ATT_PDIA);
        s.insert(ATT_PHEI);s.insert(ATT_PDIS);s.insert(ATT_PCON);s.insert(ATT_PBOR);
        s.insert(ATT_PPRO);s.insert(ATT_DPRO);s.insert(ATT_BTHK);s.insert(ATT_BDIA);
        s.insert(ATT_PTDI);s.insert(ATT_PBDI);s.insert(ATT_PBTP);s.insert(ATT_PCTP);
        s.insert(ATT_PBBT);s.insert(ATT_PCBT);s.insert(ATT_PXLE);s.insert(ATT_PYLE);
        s.insert(ATT_PZLE);s.insert(ATT_PTDM);s.insert(ATT_PBDM);s.insert(ATT_PTCDI);

        s.insert(IMP_PAXI);s.insert(IMP_PCON);s.insert(IMP_PDIS);s.insert(IMP_PBOR);
        s.insert(IMP_PDIA);s.insert(IMP_PHEI);s.insert(IMP_PTDI);s.insert(IMP_PTDM);
        s.insert(IMP_PBDI);s.insert(IMP_PBDM);s.insert(IMP_PPRO);s.insert(IMP_PRAD);
        s.insert(IMP_PX);s.insert(IMP_PY);s.insert(IMP_PZ);s.insert(IMP_PXLE);
        s.insert(IMP_PYLE);s.insert(IMP_PZLE);s.insert(IMP_PCTP);s.insert(IMP_PCBT);
        s.insert(IMP_PBBT);s.insert(IMP_PBOF);s.insert(IMP_PCOF);s.insert(IMP_PBTP);
        s.insert(IMP_PTCDI);
        s
    };
}


use std::collections::{BTreeMap, HashMap};
use std::ops::Neg;
use dashmap::DashMap;
use itertools::Itertools;
use mongodb::{Database, bson::doc, Client};
use smol_str::SmolStr;
use crate::AttrMap;
use crate::db_tool::db1_dehash;
use crate::resolve_helper::{eval_str_to_f64, resolve_dir_and_pos, parse_str_axis_to_vec3, resolve_to_cate_geo_params};
use crate::pdms_data::{AxisParam, GmParam, ScomInfo};
use crate::parsed_data::{CateAxisParam, GmseParamData};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::{AttrVal, EleNode};
use crate::query_cata::{DDANGLE_STR, DDHEIGHT_STR, DDRADIUS_STR};

pub fn get_attr_double_as_dehash_string(ele: &DashMap<String, AttrVal>, attr: &str) -> String {
    if let Some(value) = ele.get(attr) {
        match value.value() {
            AttrVal::DoubleType(d) => {
                return db1_dehash(*d as u32);
            }
            _ => {}
        }
    };
    "unset".to_string()
}


pub fn get_attr_value_f64_vec(attr_map: &AttrMap, att: &str) -> Option<Vec<f64>> {
    let mut v = vec![];
    if let Some(val) = attr_map.get_val(att) {
        match val {
            AttrVal::DoubleArrayType(data) => {
                v = data.clone();
                return Some(v);
            }
            AttrVal::Vec3Type(data) => {
                v = data.to_vec();
                return Some(v);
            }
            _ => {}
        }
    }
    None
}

pub fn get_attr_value_int(ele: &AttrMap, attr: &str) -> i32 {
    let mut value = 0;
    if let Some(ele_value) = ele.get_val(attr) {
        match ele_value {
            AttrVal::IntegerType(data) => {
                value = *data;
            }
            _ => {}
        }
    }
    value
}

pub fn get_attr_value_int_vec(ele: &AttrMap, attr: &str) -> Vec<i32> {
    let mut value = vec![];
    if let Some(ele_value) = ele.get_val(attr) {
        match ele_value {
            AttrVal::IntArrayType(data) => {
                value = data.to_vec();
            }
            AttrVal::DoubleArrayType(data) => {
                value = data.iter().map(|v| {
                    *v as i32 })
                    .collect::<Vec<i32>>();
            }
            _ => {}
        }
    }
    value
}

pub fn get_world_matrix_f64_db(ele: &AttrMap) -> Vec<f64> {
    let mut pos = get_attr_value_f64_vec(ele, "POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    vec![
        1.0f64, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, pos[0], pos[1], pos[2],
    ]
}

pub fn get_attr_strings_db(ele: &AttrMap, attrs: &[&str]) -> Vec<SmolStr> {
    let mut results = vec![];
    for &attr_name in attrs {
        if let Some(result) = ele.get_val(attr_name) {
            match result {
                AttrVal::StringType(value) => {
                    if value != "" {
                        results.push(value.trim_matches('\0').to_owned().clone().into());
                    }
                }
                _ => {}
            }
        }
    }
    results
}

/// 求解axis的数值, 得到 {num:  }
pub fn resolve_axis_params(
    scom: &ScomInfo,
    context: &HashMap<SmolStr, SmolStr>,
) -> BTreeMap<i32, CateAxisParam> {
    let mut map = BTreeMap::new();
    for i in 0..scom.axis_params.len() {
        if let Some(axis) = resolve_axis_param(&scom.axis_params[i], scom, context) {
            map.insert(scom.axis_param_numbers[i], axis);
        }
    }
    map
}

pub fn resolve_gms(
    gmse_strs: &[GmParam],
    context: &HashMap<SmolStr, SmolStr>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
    ddangle: Option<f64>,
) -> Vec<CateGeoParam> {
    gmse_strs
        .iter()
        .filter_map(|gmse_str| {
            resolve_paragon_gm_params(&gmse_str, context, axis_params)
        })
        .collect::<Vec<CateGeoParam>>()
}

/// 解析gmes的参数
pub fn resolve_paragon_gm_params(
    gm_param: &GmParam,
    context: &HashMap<SmolStr, SmolStr>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
) -> Option<CateGeoParam> {
    dbg!(&gm_param);
    if let Some(gm_data) = resolve_gmse_params(gm_param, context, axis_params) {
        if let Ok(s) = std::panic::catch_unwind(move || unsafe {
            dbg!(&gm_data);
            return resolve_to_cate_geo_params(gm_data);
        }){
            return s;
        }
    }
    None
}

pub fn resolve_gmse_params(
    gm: &GmParam,
    context: &HashMap<SmolStr, SmolStr>,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
) -> Option<GmseParamData> {
    // dbg!(gm.refno.to_refno_str());
    let angle = context[DDANGLE_STR].parse::<f64>().unwrap_or(0.0f64).to_radians();
    let radius = context[DDRADIUS_STR].parse::<f64>().unwrap_or(0.0f64);
    let height = context[DDHEIGHT_STR].parse::<f64>().unwrap_or(0.0f64);
    let diameters = gm.diameters
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();
    // dbg!(&gm.diameters);
    let distances = gm.distances
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let verts = gm.verts
        .iter()
        .map(|exp| [eval_str_to_f64(exp[0].as_str(), context).unwrap_or_default() as f32,
            eval_str_to_f64(exp[1].as_str(), context).unwrap_or_default() as f32])
        .collect::<Vec<[f32; 2]>>();

    let phei = eval_str_to_f64(&gm.phei, context).unwrap_or_default();
    let offset = eval_str_to_f64(&gm.offset, context).unwrap_or_default();

    let pang = eval_str_to_f64(&gm.pang, context).unwrap_or_default();
    let prad = eval_str_to_f64(&gm.prad, context).unwrap_or_default();
    let pwid = eval_str_to_f64(&gm.pwid, context).unwrap_or_default();
    let drad = eval_str_to_f64(&gm.drad, context).unwrap_or_default();
    let dwid = eval_str_to_f64(&gm.dwid, context).unwrap_or_default();

    let dxy = gm.dxy
        .iter()
        .map(|exp| [eval_str_to_f64(exp[0].as_str(), context).unwrap_or_default() as f32,
            eval_str_to_f64(exp[1].as_str(), context).unwrap_or_default() as f32])
        .collect::<Vec<[f32; 2]>>();


    let box_lengths = gm.box_lengths
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let xyz = gm.xyz
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let mut paxises: Vec<CateAxisParam> = Vec::new();
    for name in gm.paxises.iter() {
        if name != "" {
            let (is_negative, name) = if name.starts_with('-') {
                (true, &name[1..])
            } else {
                (false, &name[..])
            };
            match &name[0..1] {
                "P" => {
                    if let Ok(index) = name.trim()[1..].parse::<i32>() {
                        if index == 0 {
                            //todo
                            paxises.push(CateAxisParam::zero());
                        } else {
                            if axis_param_map.contains_key(&index) {
                                paxises.push(if is_negative {
                                    axis_param_map[&index].clone().neg()
                                } else {
                                    axis_param_map[&index].clone()
                                });
                            } else {
                                return None;
                            }
                        }
                    }
                }
                "T" => {}
                _ => {
                    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(0.0f64);
                    let dir = parse_str_axis_to_vec3(name, ddangle);
                    let axis = CateAxisParam {
                        pt: vec![0.0f64, 0.0, 0.0],
                        dir: dir.to_vec(),
                        pconnect: "".to_string(),
                        pbore: 0.0,
                    };
                    paxises.push(if is_negative { axis.neg() } else { axis });
                }
            }
        }
    }
    let type_name = gm.gm_type.clone();
    Some(GmseParamData {
        type_name,
        radius,
        angle,
        height,
        pwid,
        prad,
        pang,
        diameters,
        distances,
        phei,
        offset,
        verts,
        dxy,
        drad,
        dwid,
        box_lengths,
        xyz,
        paxises,
        centre_line_flag: gm.centre_line_flag,
        tube_flag: gm.visible_flag,
    })
}

pub fn resolve_axis_param(
    axis_param: &AxisParam,
    scom: &ScomInfo,
    context: &HashMap<SmolStr, SmolStr>,
) -> Option<CateAxisParam> {
    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(0.0f64);
    let key: SmolStr = axis_param.pconnect.replace("\n", "").replace(" ", "").into();
    let pconnect = if context.contains_key(&key) {
        let tmp = context[&key].parse::<u32>().unwrap_or(0u32);
        db1_dehash(tmp)
    } else {
        "".to_string()
    };
    let pbore = eval_str_to_f64(&axis_param.pbore, &context).unwrap_or_default();
    match axis_param.attr_map.get_type_cloned().as_str() {
        "PTAX" => {
            let d = eval_str_to_f64(&axis_param.distance, &context).unwrap_or_default();
            let (dir, pos) = resolve_dir_and_pos(axis_param, ddangle, scom, context);
            Some(CateAxisParam {
                pt: vec![d * dir[0] + pos[0], d * dir[1] + pos[1], d * dir[2] + pos[2]],
                dir: dir.to_vec(),
                pconnect,
                pbore,
            })
        }
        "PTCA" | "PTMI" => {
            let x = eval_str_to_f64(&axis_param.x, &context).unwrap_or_default();
            let y = eval_str_to_f64(&axis_param.y, &context).unwrap_or_default();
            let z = eval_str_to_f64(&axis_param.z, &context).unwrap_or_default();
            // dbg!(axis_param.attr_map.to_string_hashmap());
            let (dir, pos) = resolve_dir_and_pos(axis_param, ddangle, scom, context);
            Some(CateAxisParam { pt: vec![pos[0] + x, pos[1] + y, pos[2] + z], dir: dir.to_vec(), pconnect, pbore })
        }
        "PTPOS" => {
            let (dir, pos) = resolve_dir_and_pos(axis_param, ddangle, scom, context);
            let pnt_index_str = axis_param.attr_map.get_as_string("PTCPOS").unwrap_or_default();
            let paras = pnt_index_str.split_whitespace().map(|x| x.trim().to_owned()).collect::<Vec<_>>();
            if paras.len() == 2 {
                let pnt_index = paras[1].parse::<i32>().unwrap_or(i32::MAX);
                if let Some(indx) = scom.axis_param_numbers.iter().position(|&x| x == pnt_index) {
                    if let Some(axis) = resolve_axis_param(&scom.axis_params[indx], scom, context) {
                        Some(CateAxisParam { pt: axis.pt, dir: dir.to_vec(), pconnect, pbore })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None
    }
}

pub fn convert_to_context_key(expr: &str, i: &mut usize, strs: &Vec<SmolStr>) -> Option<SmolStr> {
    match expr {
        "PARA" | "PARAM" | "CPAR" => {
            *i += 1;
            Some(format!("PARAM{}", strs[*i]).into())
        }
        "ANGL" => {
            Some("ANGL".into())
        }
        "IPAR" | "IPARAM" => {
            *i += 1;
            //先忽略保温层厚度
            Some(format!("IPARAM{}", strs[*i]).into())
        }
        "DESP" | "DDESP"  => {
            *i += 1;
            Some(format!("DESP{}", strs[*i]).into())
        }
        "DESIGN PARAM" => {
            *i += 2;
            Some(format!("DESP{}", strs[*i]).into())
        }
        _ => {
            Some("".into())
        }
    }
}

#[inline]
pub fn parse_to_u16(input: &[u8]) -> u16 {
    u16::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i32(input: &[u8]) -> i32 {
    i32::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_u32(input: &[u8]) -> u32 {
    u32::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_f32(input: &[u8]) -> f32 {
    (f32::from_be_bytes(input.try_into().unwrap()) * 100.0).round() / 100.0
}

#[inline]
pub fn parse_to_f64(input: &[u8]) -> f64 {
    if let [a, b, c, d, e, f, g, h] = input[..8] {
        return (f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 100.0).round() / 100.0;
    } else {
        return 0.0;
    }
}


#[inline]
pub fn convert_u32_to_noun(input: &[u8]) -> SmolStr {
    db1_dehash(parse_to_u32(input.try_into().unwrap())).into()
}

#[inline]
pub fn parse_to_f64_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f64(&input[i * 8..i * 8 + 8]);
    }
    data
}

#[inline]
pub fn parse_to_f32_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f32(&input[i * 4..i * 4 + 4]) as f64;
    }
    data
}
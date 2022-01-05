use std::collections::{BTreeMap, HashMap};
use std::ops::Neg;
use dashmap::DashMap;
use itertools::Itertools;
use mongodb::{Database, bson::doc, Client};
use crate::AttrMap;
use crate::db_tool::db1_dehash;
use crate::resolve_helper::{eval_str_to_f64, resolve_dir_and_pos, parse_str_axis_to_vec3, resolve_to_cate_geo_params};
use crate::pdms_data::{AxisParam, GmseParam, ScomInfo};
use crate::parsed_data::{CateAxisParam, GeoParamsData, GmseParamData};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::{AttrVal, EleDataNode, ElementData};


// pub async fn get_attr_string_db(ele: EleDataNode, db: &Database, _db_tree: &Database) -> mongodb::error::Result<ElementData> {
//     let table = db.collection::<ElementData>(&ele.type_name);
//     let value = table.find_one(doc! { "ref_no":ele.ref_no }, None).await?;
//     if let Some(value) = value {
//         Ok(value)
//     } else {
//         Ok(ElementData::default())
//     }
// }

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
    if let Some(val) = attr_map.get(att) {
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
    if let Some(ele_value) = ele.get(attr) {
        match ele_value {
            AttrVal::IntegerType(data) => {
                value = data;
            }
            _ => {}
        }
    }
    value
}

pub fn get_attr_value_int_vec(ele: &AttrMap, attr: &str) -> Vec<i32> {
    let mut value = vec![];
    if let Some(ele_value) = ele.get(attr) {
        match ele_value {
            AttrVal::IntArrayType(data) => {
                value = data.to_vec();
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

pub fn get_attr_strings_db(ele: &AttrMap, attrs: &[&str]) -> Vec<String> {
    let mut results = vec![];
    for &attr_name in attrs {
        if let Some(result) = ele.get(attr_name) {
            match result {
                AttrVal::StringType(value) => {
                    if value != "" {
                        results.push(value.trim_matches('\0').to_owned().clone());
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
    context: &HashMap<String, String>,
) -> BTreeMap<i32, CateAxisParam> {
    let mut map = BTreeMap::new();
    for i in 0..scom.axis_params.len() {
        if let Some(axis) = resolve_axis_param(&scom.axis_params[i], scom, context) {
            map.insert(scom.axis_param_numbers[i], axis);
        }
    }
    map
}

pub fn resolve_gmses(
    gmse_strs: &[GmseParam],
    context: &HashMap<String, String>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
    ddangle: Option<f64>,
) -> Vec<CateGeoParam> {
    gmse_strs
        .iter()
        .filter_map(|gmse_str| {
            parse_paragon_gmse_params(&gmse_str, context, axis_params)
        })
        .collect::<Vec<CateGeoParam>>()
}

/// 解析gmes的参数
pub fn parse_paragon_gmse_params(
    gmse_param: &GmseParam,
    context: &HashMap<String, String>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
) -> Option<CateGeoParam> {
    // dbg!(&gmse_param);
    if let Some(gmse_data) = resolve_gmse_params(gmse_param, context, axis_params) {
        let d = resolve_to_cate_geo_params(gmse_data);
        // dbg!(&d);
        return d;
    }
    None
}

pub fn resolve_gmse_params(
    gmse: &GmseParam,
    context: &HashMap<String, String>,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
) -> Option<GmseParamData> {
    let radius = eval_str_to_f64(&gmse.radius, context).unwrap_or(10.0f64);
    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(90.0f64);
    let angle = ddangle.to_radians();
    let diameters = gmse.diameters
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let distances = gmse.distances
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let height = eval_str_to_f64(&gmse.height, context).unwrap_or(10.0);
    let offset = eval_str_to_f64(&gmse.offset, context).unwrap_or_default();

    let box_lengths = gmse.box_lengths
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let xyz = gmse.xyz
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let mut paxises: Vec<CateAxisParam> = Vec::new();
    for name in gmse.paxises.iter() {
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
                    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(90.0f64);
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
    let attr_map = &gmse.attr_map;
    Some(GmseParamData {
        name: attr_map.get_name(),
        refno: attr_map.get_refno(),
        owner: attr_map.get_owner(),
        type_name: attr_map.get_type(),
        radius,
        angle,
        diameters,
        distances,
        height,
        offset,
        box_lengths,
        xyz,
        paxises,
        centre_line_flag: gmse.centre_line_flag,
        tube_flag: gmse.tube_flag,
    })
}

pub fn resolve_axis_param(
    axis_param: &AxisParam,
    scom: &ScomInfo,
    context: &HashMap<String, String>,
) -> Option<CateAxisParam> {
    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(0.0f64);
    let key = &axis_param.pconnect.replace("\n", "").replace(" ", "");
    let pconnect = if context.contains_key(key) {
        let tmp = context[key].parse::<u32>().unwrap_or(0u32);
        db1_dehash(tmp)
    } else {
        "".to_string()
    };
    let pbore = eval_str_to_f64(&axis_param.pbore, &context).unwrap_or_default();
    match axis_param.attr_map.get_type().as_str() {
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

pub fn convert_to_context_key(expr: &str, i: &mut usize, strs: &Vec<String>) -> Option<String> {
    match expr {
        "PARA" | "PARAM" => {
            *i += 1;
            Some(format!("PARAM{}", strs[*i]))
        }
        "ANGL" => {
            Some("DDANGLE".to_string())
        }
        "IPAR" | "IPARAM" => {
            *i += 1;
            //先忽略保温层厚度
            Some(format!("IPARAM{}", strs[*i]))
        }
        "DESP" => {
            *i += 1;
            Some(format!("DESP{}", strs[*i]))
        }
        _ => {
            Some("".to_string())
        }
    }
}

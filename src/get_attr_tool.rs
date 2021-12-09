use std::collections::{BTreeMap, HashMap};
use std::ops::Neg;
use dashmap::DashMap;
use mongodb::{Database,bson::doc};
use crate::param_parse::{eval_str_to_f64, get_dir_and_pos, parse_gmse_param_to_cate_geo_params, parse_str_axis_to_vec3};
use crate::pdms_origin_data::{AxisParam, GmseParam, ScomParamStr};
use crate::pdms_parsed_data::{CateAxisParam, GeoParamsData, GmseParamData};
use crate::pdms_types::{AttrVal, EleDataNode, ElementData};

pub async fn get_attr_string_db(ele: EleDataNode, db: &Database, db_tree: &Database) -> mongodb::error::Result<ElementData> {
    let table = db.collection::<ElementData>(&ele.type_name);
    let value = table.find_one(
        doc! {
            "ref_no":ele.ref_no
        },
        None,
    ).await?;
    if let Some(value) = value {
        Ok(value)
    } else {
        Ok(ElementData::default())
    }
}

pub fn get_attr_value_as_string(ele: &DashMap<String, AttrVal>, types: &str) -> String {
    let mut result = "".to_string();
    if let Some(value) = ele.get(types) {
        match value.value() {
            AttrVal::StringType(str_val) | AttrVal::WordType(str_val) | AttrVal::ElementType(str_val) => {
                result = str_val.clone();
            }
            AttrVal::IntegerType(int_str) => {
                result = int_str.to_string();
            }
            AttrVal::DoubleType(dou_str) => {
                result = dou_str.to_string();
            }
            AttrVal::BoolType(bool_str) => {
                result = bool_str.to_string();
            }
            _ => {}
        }
    };
    result
}

pub fn get_attr_value_dou_vec(ele: &DashMap<String, AttrVal>, types: &str) -> Vec<f64> {
    let mut value = vec![0.0,0.0,0.0];
    if let Some(ele_value) = ele.get(types) {
        match ele_value.value() {
            AttrVal::DoubleArrayType(data) => {
                value = data.clone();
            }
            AttrVal::Vec3Type(data) => {
                value = data.to_vec();
            }
            _ => {}
        }
    }
    value
}

pub fn get_attr_value_int(ele: &DashMap<String, AttrVal>, types: &str) -> i32 {
    let mut value = 0;
    if let Some(ele_value) = ele.get(types) {
        match ele_value.value() {
            AttrVal::IntegerType(data) => {
                value = *data;
            }
            _ => {}
        }
    }
    value
}


pub fn get_world_matrix_f64_db(ele:&DashMap<String, AttrVal>) -> Vec<f64> {
    let mut pos=get_attr_value_dou_vec(ele,"POS");
    vec![
        1.0f64, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, pos[0], pos[1], pos[2],
    ]
}

pub fn get_attr_strings_db(ele: &DashMap<String, AttrVal>, attrs: &[&str]) -> Vec<String> {
    let mut results = vec![];
    for &attr_name in attrs {
        if let Some(result) = ele.get(attr_name) {
            match result.value() {
                AttrVal::StringType(value) => {
                    if value != "" {
                        results.push(value.clone());
                    }
                }
                _ => {}
            }
        }
    }
    results
}

pub fn parse_axis_params(
    scom: &ScomParamStr,
    context: &HashMap<String, String>,
    data: &ElementData
) -> BTreeMap<i32, CateAxisParam> {
    let mut cate_axis_params_map = BTreeMap::new();
    for i in 0..scom.axis_param_strs.len(){
        if let Some(axis) = parse_axis_param(&scom.axis_param_strs[i], scom, context,data) {
            cate_axis_params_map.insert(scom.axis_param_number[i], axis);
        }
    }
    cate_axis_params_map
}

pub fn parse_gmses(
    gmse_strs: &[GmseParam],
    context: &HashMap<String, String>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
    ddangle: Option<f64>,
) -> Vec<GeoParamsData> {
    gmse_strs
        .iter()
        .map(|gmse_str| {
            parse_paragon_gmse_params(&gmse_str, context, axis_params).unwrap_or(GeoParamsData{
                cate_geo_params: None
            })
        })
        .collect::<Vec<GeoParamsData>>()
}

/// 解析gmes的参数
pub fn parse_paragon_gmse_params(
    gmse_str: &GmseParam,
    context: &HashMap<String, String>,
    axis_params: &BTreeMap<i32, CateAxisParam>,
) -> Option<GeoParamsData> {
    if let Some(gmse) =
    parse_gmse_params(gmse_str, context, axis_params){
        return Some(parse_gmse_param_to_cate_geo_params(gmse));
    }
    None
}

pub fn parse_gmse_params(
    gmse_str: &GmseParam,
    context: &HashMap<String, String>,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
) -> Option<GmseParamData> {
    let radius = eval_str_to_f64(&gmse_str.radius, context).unwrap_or_default();
    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(0.0f64);
    let angle = ddangle * std::f64::consts::PI / 180.0;
    let diameters = gmse_str.diameters
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let distances = gmse_str.distances
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let height = eval_str_to_f64(&gmse_str.height, context).unwrap_or_default();
    let offset = eval_str_to_f64(&gmse_str.offset, context).unwrap_or_default();

    let box_lengths = gmse_str.box_lengths
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let xyz = gmse_str.xyz
        .iter()
        .map(|exp| eval_str_to_f64(&exp, context).unwrap_or_default())
        .collect::<Vec<f64>>();

    let mut paxises: Vec<CateAxisParam> = Vec::new();
    for name in gmse_str.paxises.iter() {
        if name != "" {
            //////dbg!(&name);
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
                            }else{
                                // ////dbg!(&index);
                                // ////dbg!(&axis_param_map);
                                // log::info!("当前axis key是{:?}, axis_param_map是{:?}", index, &axis_param_map);
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
                        pbore: 0.0
                    };
                    paxises.push(if is_negative { axis.neg() } else { axis });
                }
            }
        }
    }

    Some(GmseParamData {
        name: gmse_str.name.clone(),
        refno: gmse_str.refno.clone(),
        owner: gmse_str.owner.clone(),
        self_type: gmse_str.self_type.clone(),
        radius,
        angle,
        diameters,
        distances,
        height,
        offset,
        box_lengths,
        xyz,
        paxises,
        centre_line_flag: gmse_str.centre_line_flag,
        tube_flag: gmse_str.tube_flag,
    })
}

pub fn parse_axis_param(
    axis_param: &AxisParam,
    scom: &ScomParamStr,
    context: &HashMap<String, String>,
    data:&ElementData
) -> Option<CateAxisParam> {
    let ddangle = context["DDANGLE"].parse::<f64>().unwrap_or(0.0f64);
    let key = &axis_param.pconnect.replace("\n", "").replace(" ", "");
    let pconnect = if context.contains_key(key) {
        context[key].clone()
    } else {
        "".to_string()
    };
    ////dbg!(&axis_param);
    let pbore = eval_str_to_f64(&axis_param.pbore, &context).unwrap_or_default();
    match &axis_param.self_type[..] {
        "PTAX" => {
            let d = eval_str_to_f64(&axis_param.distance, &context).unwrap_or_default();
            let (dir, pos) = get_dir_and_pos(axis_param, ddangle, scom, context,data);
            // ////dbg!(&dir);
            Some(CateAxisParam {
                pt: vec![d * dir[0] + pos[0], d * dir[1] + pos[1], d * dir[2] + pos[2]],
                dir: dir.to_vec(),
                pconnect,
                pbore
            })
        }
        "PTCA" | "PTMI" => {
            let x = eval_str_to_f64(&axis_param.x, &context).unwrap_or_default();
            ////dbg!(x);
            let y = eval_str_to_f64(&axis_param.y, &context).unwrap_or_default();
            ////dbg!(y);
            let z = eval_str_to_f64(&axis_param.z, &context).unwrap_or_default();
            ////dbg!(z);
            let (dir, pos) = get_dir_and_pos(axis_param, ddangle, scom, context,data);
            ////dbg!(&pos);
            Some(CateAxisParam { pt: vec![pos[0]+x, pos[1]+y, pos[2]+z], dir: dir.to_vec(), pconnect, pbore })
        }
        "PTPOS" => {
            let (dir, pos) = get_dir_and_pos(axis_param, ddangle, scom, context,data);
            // let ele = DbElement::from_refno_str(&axis_param.refno);
            let ele=&data.ref_no;
            let data_map=&data.attr_data_map;
            // let pnt_index_str = crate::get_attr_string!(ele, PTCPOS);
            let pnt_index_str=get_attr_value_as_string(data_map,"PTCPOS");
            let paras = pnt_index_str.split_whitespace().map(|x| x.trim().to_owned()).collect::<Vec<_>>();
            // ////dbg!(pnt_index_str);
            if paras.len() ==2 {
                let pnt_index = paras[1].parse::<i32>().unwrap_or(i32::MAX);
                if let Some(indx) = scom.axis_param_number.iter().position(|&x| x == pnt_index){
                    // ////dbg!(indx);
                    if let Some(axis) = parse_axis_param(&scom.axis_param_strs[indx], scom, context,data){
                        Some(CateAxisParam { pt: axis.pt, dir: dir.to_vec(), pconnect, pbore })
                    }else{
                        None
                    }
                }else{
                    None
                }
            }else{
                None
            }
        }
        _ => None
    }
}

pub fn convert_to_context_key(expr: &str, i: &mut usize, strs: &Vec<String>) -> Option<String>{
    match expr {
        "PARA" | "PARAM" => {
            *i += 1;
            Some(format!("PARAM{}", strs[*i]))
        },
        "ANGL" =>{
            Some("DDANGLE".to_string())
        },
        "IPAR" | "IPARAM" =>{
            *i += 1;
            //先忽略保温层厚度
            Some(format!("IPARAM{}", strs[*i]))
        },
        "DESP" => {
            *i += 1;
            Some(format!("DESP{}", strs[*i]))
        },
        _ =>{
            // let log_str = format!("PARA 或 DESP 参数解析错误！表达式是 {}, 当前key是 {}", exp, s);
            // println!("{}",&log_str);
            // info!("{}, 上下文是 {:?}",&log_str, context);
            Some("".to_string())
        }
    }
}


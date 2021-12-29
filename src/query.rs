use crate::db_tool::db1_dehash;
use crate::helper::*;
use crate::interface::pdms_interface::PdmsInterface;
use crate::pdms_data::{AxisParam, DesCompInfo, GmseParam, ScomInfo};
use crate::parsed_data::geo_params_data::CateGeoParam::TubeImplied;
use crate::parsed_data::{CateTubeImpliedParam, GeoParamsData, GeomsInfo, SLoo};
use crate::pdms_types::AttrVal::IntArrayType;
use crate::pdms_types::{AttrVal, EleDataNode, ElementData, PdmsRefno};
use crate::AttrMap;
use dashmap::DashMap;
use mongodb::{bson::doc, Client, Database};
use nalgebra_glm::{DMat4, DVec4};
use std::collections::{BTreeMap, HashMap};

const DDHEIGHT_STR: &'static str = "DDHEIGHT";
const DDRADIUS_STR: &'static str = "DDRADIUS";
const DDANGLE_STR: &'static str = "DDANGLE";


///求解design component
pub async fn resolve_desi_comp(
    // attr_map: &AttrMap,
    refno: &str,
    interface: &mut PdmsInterface,
) -> mongodb::error::Result<Option<GeomsInfo>> {

    let attr_map = interface.get_ele_attr_map_async(refno).await?;
    if attr_map.is_none() { return Ok(None); }
    let attr_map = attr_map.unwrap();
    let desp = get_attr_value_int_vec(&attr_map, "DESP");
    let spre_ref = attr_map.get_as_string("SPRE");
    let mut scom_ref = "unset".to_string();
    if let Some(spre) = interface.get_ele_attr_map_async(spre_ref.as_str()).await?{
        scom_ref = spre.get_as_string("CATR");
    }
    let scom_info = interface.get_scom_info_async(scom_ref.as_str()).await?;
    // dbg!(&scom_info);
    if scom_info.is_none() { return Ok(None); }
    let mut context = HashMap::new();
    context.insert(DDHEIGHT_STR.to_string(), attr_map.get_as_string("HEIG"));
    context.insert(DDANGLE_STR.to_string(), attr_map.get_as_string("ANGL"));
    context.insert(DDRADIUS_STR.to_string(), attr_map.get_as_string("RADI"));
    let desparams = get_attr_value_f64_vec(&attr_map, "PARA").unwrap_or_default();
    for i in 0..desparams.len() {
        context.insert(
            format!("DESP{}", i + 1),
            desparams[i].to_string(),
        );
    }
    let mut geom_info = resolve_cata_comp_async(scom_info.as_ref().unwrap(), interface, Some(context)).await?;
    Ok(Some(geom_info))
}

///整合SCOM对应的临时数据
pub async fn query_scom_info(
    refno: &str,
    interface: &mut PdmsInterface,
) -> mongodb::error::Result<Option<ScomInfo>> {
    if let Some(attr_map) = interface.get_ele_attr_map_async(refno).await? {
        let ptre_refno = attr_map.get_as_string("PTRE");
        let mut axis_params = vec![];
        let mut axis_param_numbers = vec![];
        if let Some(ptre_am) = interface
            .get_ele_attr_map_async(ptre_refno.as_str())
            .await?
        {
            let axis_param_map = query_axis_params(&ptre_am, interface).await?;
            axis_params = axis_param_map.values().cloned().collect::<Vec<_>>();
            axis_param_numbers = axis_param_map.keys().cloned().collect::<Vec<_>>();
        }

        let gmset_refno = attr_map.get_as_string("GMRE");
        let mut gmse_params = vec![];
        if let Some(gmse_am) = interface
            .get_ele_attr_map_async(gmset_refno.as_str())
            .await?
        {
            gmse_params = query_gmse_params(&gmse_am, interface).await?;
        }

        return Ok(Some(ScomInfo {
            name: attr_map.get_name(),
            gtype: attr_map.get_as_string("GTYPE"),
            dtse_params: vec![],
            gmse_params,
            axis_params,
            params: attr_map
                .get_as_string("PARA")
                .replace("\n", " ")
                .replace("  ", " "),
            axis_param_numbers,
            attr_map,
        }));
    }
    Ok(None)
}

pub async fn query_axis_params(
    attr_map: &AttrMap,
    interface: &mut PdmsInterface,
) -> mongodb::error::Result<BTreeMap<i32, AxisParam>> {
    // 查找ptse
    let mut map = BTreeMap::new();
    let refno = attr_map.get_refno();
    let children = interface
        .get_children_attr_map_async(refno.as_str())
        .await?;
    for child in children {
        let number = child.get_as_string("NUMB").parse::<i32>().unwrap_or(-1);
        map.entry(number).or_insert(get_axis_param(&child));
    }
    Ok(map)
}

///查询gmse的参数
pub async fn query_gmse_params(
    attr_map: &AttrMap,
    interface: &mut PdmsInterface,
) -> mongodb::error::Result<Vec<GmseParam>> {
    let mut gmses = vec![];
    let refno = attr_map.get_refno();
    let children = interface
        .get_children_attr_map_async(refno.as_str())
        .await?;
    for child in children {
        gmses.push(query_gmse_param(&child));
    }
    Ok(gmses)
}


///对元件库的SCOM Element进行求值计算
pub async fn resolve_cata_comp_async(
    scomp_info: &ScomInfo,
    interface: &mut PdmsInterface,
    context: Option<HashMap<String, String>>
) -> mongodb::error::Result<GeomsInfo> {
    let mut cur_context = HashMap::new();
    if context.is_some(){
        cur_context = context.unwrap();
    }
    cur_context
        .entry(DDHEIGHT_STR.to_string())
        .or_insert("0.0".to_string());
    cur_context
        .entry(DDRADIUS_STR.to_string())
        .or_insert("0.0".to_string());
    cur_context
        .entry(DDANGLE_STR.to_string())
        .or_insert("0.0".to_string());
    //获取DTSE的expression
    query_dtse_params(&scomp_info.attr_map, interface, &mut cur_context).await;
    cur_context.insert("IPARAM0".to_string(), "0".to_string());
    let params = get_attr_value_f64_vec(&scomp_info.attr_map, "PARA").unwrap_or_default();
    for i in 0..params.len() {
        cur_context.insert(format!("PARAM{}", i + 1), params[i].to_string());
        cur_context.insert(format!("IPARAM{}", i + 1), "0".to_string());
    }
    //求解AXIS的数据
    let axis_map = resolve_axis_params(scomp_info, &cur_context);
    //求解子节点几何模型的数据
    let geometries = resolve_gmses(&scomp_info.gmse_params, &cur_context, &axis_map, None);
    Ok(GeomsInfo {
        geometries,
        axis_map,
    })
}

///获得AxisParam
pub fn get_axis_param(attr_map: &AttrMap) -> AxisParam {
    let type_name = attr_map.get_as_string("TYPE");
    let pconnect = attr_map.get_as_string("PCON");
    let pbore = attr_map.get_as_string("PBOR");
    let refno = attr_map.get_refno();
    let pos = get_attr_value_f64_vec(attr_map, "POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    match type_name.as_ref() {
        "PTAX" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".to_string(),
            y: "".to_string(),
            z: "".to_string(),
            distance: attr_map.get_as_string("PDIS"),
            direction: attr_map.get_as_string("PAXI"),
            pconnect,
            pbore,
        },
        "PTCA" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX"),
            y: attr_map.get_as_string("PY"),
            z: attr_map.get_as_string("PZ"),
            distance: "".to_string(),
            direction: attr_map.get_as_string("PTCDI"),
            pconnect,
            pbore,
        },
        "PTMI" => AxisParam {
            attr_map: attr_map.clone(),
            x: attr_map.get_as_string("PX"),
            y: attr_map.get_as_string("PY"),
            z: attr_map.get_as_string("PZ"),
            distance: "".to_string(),
            direction: attr_map.get_as_string("PAXI"),
            pconnect,
            pbore,
        },
        "PTPOS" => AxisParam {
            attr_map: attr_map.clone(),
            x: "".to_string(),
            y: "".to_string(),
            z: "".to_string(),
            distance: attr_map.get_as_string("PTCPOS"),
            direction: attr_map.get_as_string("PTCD"),
            pconnect,
            pbore,
        },
        _ => AxisParam {
            attr_map: attr_map.clone(),
            x: "".to_string(),
            y: "".to_string(),
            z: "".to_string(),
            distance: "".to_string(),
            direction: "".to_string(),
            pconnect,
            pbore,
        },
    }
}

///获得gmse的params
pub fn query_gmse_param(attr_map: &AttrMap) -> GmseParam {
    let mut paxises = get_attr_strings_db(attr_map, &["PAXI", "PAAX", "PBAX", "PCAX"]);
    if let Some(val) = attr_map.get("PTS") {
        match val {
            IntArrayType(v) => {
                for s in v {
                    paxises.push(s.to_string());
                }
            }
            _ => {}
        }
    }
    let centre_line_flag = attr_map.get_bool("CLFL");
    let tube_flag = attr_map.get_bool("TUFL");
    GmseParam {
        attr_map: attr_map.clone(),
        radius: attr_map.get_as_string("PRAD"),
        diameters: get_attr_strings_db(attr_map, &["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: get_attr_strings_db(attr_map, &["PDIS", "PBDI", "PTDI"]),
        height: attr_map.get_as_string("PHEI"),
        offset: attr_map.get_as_string("POFF"),
        // box_lengths: get_attr_strings_db(ele_map, &["PXEL", "PYEL", "PZEL"]),
        box_lengths: get_attr_strings_db(attr_map, &["PXLE", "PYLE", "PZLE"]),
        xyz: get_attr_strings_db(
            attr_map,
            &[
                "PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF",
            ],
        ),
        paxises, // 先pa_axis, 后pb_axis
        centre_line_flag,
        tube_flag,
    }
}

///获得dtse的参数信息
pub async fn query_dtse_params(
    attr_map: &AttrMap,
    interface: &mut PdmsInterface,
    context: &mut HashMap<String, String>,
) -> mongodb::error::Result<()> {

    let dtre_refno = attr_map.get_as_string("DTRE");
    let children = interface
        .get_children_attr_map_async(dtre_refno.as_str())
        .await?;
    for child in children {
        // let number = child.get_as_string("NUMB").parse::<i32>().unwrap_or(-1);
        // map.entry(number).or_insert(get_axis_param(&child));
        let key = child.get_as_string("DKEY");
        let exp = child.get_as_string("PPRO");
        let default_key = format!("{}_default_expr", key);
        let default_expr = child.get_as_string("DPRO"); 
        context.insert(key, exp);
        context.insert(default_key, default_expr);
    }

    Ok(())
}

use std::collections::{BTreeMap, HashMap};
use dashmap::DashMap;
use crate::pdms_types::{AttrVal, EleDataNode, ElementData, PdmsRefno};
use mongodb::{Client, Database, bson::doc};
use nalgebra_glm::{DMat4, DVec4};
use crate::AttrMap;
use crate::db_tool::db1_dehash;
use crate::get_attr_tool::*;
use crate::interface::pdms_interface::PdmsInterface;
use crate::pdms_origin_data::{AxisParam, DesCompInfo, GmseParam, ScomInfo};
use crate::pdms_parsed_data::{CateTubeImpliedParam, GeomsInfo, GeoParamsData, SLoo};
use crate::pdms_parsed_data::geo_params_data::CateGeoParams::TubeImplied;
use crate::pdms_types::AttrVal::IntArrayType;


pub async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    // let client_uri = "mongodb://localhost:27017".to_string();
    // let client = Client::with_uri_str(&client_uri).await?;
    // let refno = "15192/222818";
    // let refno_db = client.database("PdmsRefnoDB");
    // let refno_table = refno_db.collection::<PdmsRefno>("PdmsRefno");
    // let db_name_opt = refno_table.find_one(doc! {"ref_no":refno}, None).await?;
    // if let Some(value) = db_name_opt {
    //     let db_name = value.db;
    //     let db_name_tree = format!("{}_tree", db_name);
    //     let db = client.database(&db_name);
    //     let db_tree = client.database(&db_name_tree);
    //     let (refno, sloo) = query_design_component_by_refno_str_db(refno, &db, &db_tree).await.unwrap();
    //     // dbg!(&refno);
    //     let scom = resolve_cata_comp_attrs(refno, sloo, &db, &db_tree).await?;
    //     // let scom=resolve_desi_comp_attrs(refno,&db,&db_tree).await?;
    //     dbg!(&scom);
    // }
    Ok(())
}

// pub async fn query_design_component_by_refno_str_db(refno: &str, db: &Database, db_tree: &Database) -> mongodb::error::Result<DesignComponentData> {
//     let tree_table = db_tree.collection::<EleDataNode>("PdmsTreeNode");
//     let tree_node_opt = tree_table.find_one(doc! {"ref_no":refno}, None).await?;
//     if let Some(ele_data_node) = tree_node_opt {
//         let (r, sloo) = query_design_component_db(ele_data_node, db, db_tree).await?;
//         Ok(r)
//     } else {
//         let (r, _) = query_design_component_db(EleDataNode::default(), db, db_tree).await?;
//         Ok(r)
//     }
// }

///获取到design component 的信息
pub async fn query_descomp_info(attr_map: &AttrMap, interface:&mut PdmsInterface) -> mongodb::error::Result<DesCompInfo> {
    let type_name = attr_map.get_as_string("TYPE");
    let ddangle = attr_map.get_as_string("ANGL");
    let height = attr_map.get_as_string("HEIG");
    let radius = attr_map.get_as_string("RADI");
    // let _ptre = attr_map.get_as_string("PTRE");
    //todo
    let scom_refno = attr_map.get_refno();
    let scom_info = query_scom_info(scom_refno.as_str(), interface).await?;
    let desparams = get_attr_value_f64_vec(attr_map, "PARA").unwrap_or_default();
    let mut gtype = "unset".to_string();
    let gtype_val = get_attr_value_int(attr_map, "GTYP");
    if gtype_val > 0x81BF1 {
        gtype = db1_dehash(gtype_val as u32);
    }
    match &type_name[..] {
        // "TUBI" => {
        //     let itlength = attr_map.get_as_string("ITLE");
        //     let pos = [0.0, 0.0, 0.0];
        //     let ori = DMat4::default();
        //     //圆柱体默认是X方向
        //     let direction = ori * DVec4::new(1.0, 0.0, 0.0, 0.0);
        //
        //     Ok(DesCompInfo {
        //         name: data.name,
        //         refno: data.ref_no,
        //         owner: data.owner,
        //         self_type: type_name,
        //         //spref_name: get_attr_string!(ele, SPRE),
        //         spref_name: attr_map.get_as_string( "SPRE"),
        //         gtype,
        //         scom_info: Some(scom_info),
        //         ddangle,
        //         height,
        //         itlength,
        //         radius,
        //         world_matrix: get_world_matrix_f64_db(&attr_map),
        //         world_position: pos.to_vec(),
        //         ldirection: direction.data.0.to_vec()[0].to_vec(),
        //         desparams,
        //     })
        // }
        _ => {
            Ok(DesCompInfo {
                name: attr_map.get_name(),
                refno: attr_map.get_name(),//data.ref_no,
                owner: attr_map.get_name(),//data.owner,
                type_name,
                spref_name: attr_map.get_as_string( "SPRE"),
                gtype,
                scom_info,
                ddangle,
                height,
                radius,
                world_matrix: get_world_matrix_f64_db(&attr_map),
                world_position: vec![],
                desparams,
            })
        }
    }
}

///整合SCOM对应的临时数据
pub async fn query_scom_info(refno: &str, interface: &mut PdmsInterface) -> mongodb::error::Result<Option<ScomInfo>> {
    if let Some(attr_map) = interface.get_ele_attr_map_async(refno).await?{
        let ptre_refno = attr_map.get_as_string("PTRE");
        dbg!(&ptre_refno);
        let mut axis_params = vec![];
        let mut axis_param_numbers = vec![];
        if let Some(ptre_am) = interface.get_ele_attr_map_async(ptre_refno.as_str()).await?{
            let axis_param_map = query_axis_params(&ptre_am, interface).await?;
            axis_params = axis_param_map.values().cloned().collect::<Vec<_>>();
            axis_param_numbers = axis_param_map.keys().cloned().collect::<Vec<_>>();
        }

        let gmset_refno = attr_map.get_as_string( "GMRE");
        let mut gmse_params = vec![];
        if let Some(gmse_am) = interface.get_ele_attr_map_async(gmset_refno.as_str()).await? {
            gmse_params = query_gmse_params(&gmse_am, interface).await?;
        }

        return Ok(Some(ScomInfo {
            name: attr_map.get_name(),
            gtype: attr_map.get_as_string("GTYPE"),
            dtse_params: vec![],
            gmse_params,
            axis_params,
            params: attr_map.get_as_string("PARA").replace("\n", " ").replace("  ", " "),
            axis_param_numbers,
            attr_map,
        }));
    }

    Ok(None)

}

pub async fn query_axis_params(attr_map: &AttrMap, interface:&mut PdmsInterface) -> mongodb::error::Result<BTreeMap<i32, AxisParam>> {
    // 查找ptse
    let mut map = BTreeMap::new();
    let refno = attr_map.get_refno();
    let children = interface.get_children_attr_map_async(refno.as_str()).await?;
    for child in children {
        let number = child.get_as_string("NUMB").parse::<i32>().unwrap_or(-1);
        map.entry(number).or_insert(get_axis_param(&child));
    }
    Ok(map)
}

///查询gmse的参数
pub async fn query_gmse_params(attr_map: &AttrMap, interface:&mut PdmsInterface) -> mongodb::error::Result<Vec<GmseParam>> {
    let mut gmses = vec![];
    let refno = attr_map.get_refno();
    let children = interface.get_children_attr_map_async(refno.as_str()).await?;
    for child in children {
        gmses.push(query_gmse_param(&child));
    }
    Ok(gmses)
}

///todo 结合设计模块的构件参数，对元件库的属性进行求值计算
/// ele: DESI Element
pub async fn resolve_desi_comp_attrs(attr_map: &AttrMap, des_comp_info: DesCompInfo, interface:&mut PdmsInterface) -> mongodb::error::Result<GeomsInfo> {
    let desp = get_attr_value_int_vec(&attr_map, "DESP");
    let scom = des_comp_info.scom_info.as_ref().unwrap();
    let mut context = HashMap::new();
    context.insert(DDHEIGHT_STR.to_string(), des_comp_info.height.clone());
    context.insert(DDANGLE_STR.to_string(), des_comp_info.ddangle.clone());
    context.insert(DDRADIUS_STR.to_string(), des_comp_info.radius.clone());
    for i in 0..des_comp_info.desparams.len() {
        context.insert(format!("DESP{}", i + 1), des_comp_info.desparams[i].to_string());
    }
    resolve_cata_comp_attrs(scom, interface).await
}


const DDHEIGHT_STR: &'static str = "DDHEIGHT";
const DDRADIUS_STR: &'static str = "DDRADIUS";
const DDANGLE_STR: &'static str = "DDANGLE";

///对元件库的属性进行求值计算
/// ele: SCOM Element
pub async fn resolve_cata_comp_attrs(scomp_info: &ScomInfo, interface:&mut PdmsInterface) -> mongodb::error::Result<GeomsInfo> {
    // let table = db.collection::<ElementData>(&des_comp_info.self_type);
    // let data = table.find_one(doc! {"ref_no":&ele.refno}, None).await?.unwrap();
    // let _data_map = &data.attr_data_map;
    let mut context = HashMap::new();
    context.entry(DDHEIGHT_STR.to_string()).or_insert("0.0".to_string());
    context.entry(DDRADIUS_STR.to_string()).or_insert("0.0".to_string());
    context.entry(DDANGLE_STR.to_string()).or_insert("0.0".to_string());
    //获取DTSE的expression
    query_dtse_params(scomp_info, interface, &mut context);
    context.insert("IPARAM0".to_string(), "0".to_string());
    let params = get_attr_value_f64_vec(&scomp_info.attr_map, "PARA").unwrap_or_default();
    for i in 0..params.len() {
        context.insert(format!("PARAM{}", i + 1), params[i].to_string());
        context.insert(format!("IPARAM{}", i + 1), "0".to_string());
    }

    //求解AXIS的数据
    let axis_params_map = resolve_axis_params(
        scomp_info, &context
    );

    //求解子节点几何模型的数据
    let geometries = resolve_gmses(
        &scomp_info.gmse_params,
        &context,
        &axis_params_map,
        None,
    );

    Ok(GeomsInfo {
        geometries,
        // world_matrix: scomp_info.world_matrix,
    })
}

///deprected
// pub fn parse_param_to_hashmap(text: &str) -> HashMap<String, String> {
//     let mut params: HashMap<String, String> = HashMap::new();
//     let mut index = 1;
//     for d in text.replace("\n", " ").split(" ") {
//         if d != "" && d != " " {
//             params.insert(format!("PARAM{}", index), d.to_string());
//             params.insert(format!("IPARAM{}", index), "0.0".to_string());
//             index += 1;
//         }
//     }
//     //todo 保温层厚度
//     params.insert("IPARAM".to_string(), "0.0".to_string());
//     params
// }

///获得AxisParam
pub fn get_axis_param(attr_map: &AttrMap) -> AxisParam {
    let type_name = attr_map.get_as_string("TYPE");
    let pconnect = attr_map.get_as_string("PCON");
    let pbore = attr_map.get_as_string("PBOR");
    let refno = attr_map.get_refno();
    let pos = get_attr_value_f64_vec(attr_map, "POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    match type_name.as_ref() {
        "PTAX" => {
            AxisParam {
                attr_map: attr_map.clone(),
                x: "".to_string(),
                y: "".to_string(),
                z: "".to_string(),
                distance: attr_map.get_as_string( "PDIS"),
                direction: attr_map.get_as_string( "PAXI"),
                pconnect,
                pbore,
            }
        }
        "PTCA" => {
            AxisParam {
                attr_map: attr_map.clone(),
                x: attr_map.get_as_string( "PX"),
                y: attr_map.get_as_string( "PY"),
                z: attr_map.get_as_string( "PZ"),
                distance: "".to_string(),
                direction: attr_map.get_as_string( "PTCDI"),
                pconnect,
                pbore,
            }
        }
        "PTMI" => {
            AxisParam {
                attr_map: attr_map.clone(),
                x: attr_map.get_as_string( "PX"),
                y: attr_map.get_as_string( "PY"),
                z: attr_map.get_as_string( "PZ"),
                distance: "".to_string(),
                direction: attr_map.get_as_string( "PAXI"),
                pconnect,
                pbore,
            }
        }
        "PTPOS" => {
            AxisParam {
                attr_map: attr_map.clone(),
                x: "".to_string(),
                y: "".to_string(),
                z: "".to_string(),
                distance: attr_map.get_as_string( "PTCPOS"),
                direction: attr_map.get_as_string( "PTCD"),
                pconnect,
                pbore,
            }
        }
        _ => {
            AxisParam {
                attr_map: attr_map.clone(),
                x: "".to_string(),
                y: "".to_string(),
                z: "".to_string(),
                distance: "".to_string(),
                direction: "".to_string(),
                pconnect,
                pbore,
            }
        }
    }
}

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
            &["PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF"],
        ),
        paxises,      // 先pa_axis, 后pb_axis
        centre_line_flag,
        tube_flag,
    }
}

///获得dtse的参数信息
pub async fn query_dtse_params(data: &ScomInfo, interface:&mut PdmsInterface, context: &mut HashMap<String, String>) -> mongodb::error::Result<()> {
    // let tree_table = db_tree.collection::<EleDataNode>("PdmsTreeNode");
    // let ele = &data.attr_data_map;
    // let dtse = get_as_string(&ele, "DTRE");
    // for child in &data.children {
    //     let child_data_tree = tree_table.find_one(
    //         doc! {"ref_no":child,},
    //         None,
    //     ).await?.unwrap();
    //     let child_type = child_data_tree.type_name;
    //     let table = db.collection::<ElementData>(&child_type);
    //     let node = table.find_one(
    //         doc! {"ref_no":child},
    //         None,
    //     ).await?.unwrap();
    //     let node_map = node.attr_data_map;
    //     let key = get_as_string(&node_map, "DKEY");
    //     let exp = get_as_string(&node_map, "PPRO");
    //     let default_key = format!("{}_default_expr", key);
    //     let default_expr = get_as_string(&node_map, "DPRO");
    //     context.insert(key, exp);
    //     context.insert(default_key, default_expr);
    // }
    Ok(())
}
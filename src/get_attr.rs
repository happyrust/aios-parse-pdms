use std::collections::{BTreeMap, HashMap};
use dashmap::DashMap;
use crate::pdms_types::{AttrVal, EleDataNode, ElementData, PdmsRefno};
use mongodb::{Client, Database, bson::doc};
use nalgebra_glm::{DMat4, DVec4};
use nom::number::complete::float;
use nom::Parser;
use crate::db_tool::db1_dehash;
use crate::get_attr_tool::{get_attr_double_as_dehash_string, get_attr_string_db, get_attr_strings_db, get_attr_value_as_string, get_attr_value_f64_vec, get_attr_value_int, get_world_matrix_f64_db, resolve_axis_params, resolve_gmses};
use crate::param_parse::parse_design_param_to_hashmap;
use crate::pdms_origin_data::{AxisParam, DesignComponentData, GmseParam, ScomParamStr};
use crate::pdms_parsed_data::{CateTubeImpliedParam, DesignComponent, GeoParamsData};
use crate::pdms_parsed_data::geo_params_data::CateGeoParams::TubeImplied;
use crate::pdms_types::AttrVal::IntArrayType;


pub async fn run_test() -> Result<(), Box<dyn std::error::Error>> {
    let client_uri = "mongodb://localhost:27017".to_string();
    let client = Client::with_uri_str(&client_uri).await?;
    let refno = "15192/222434";
    let refno_db = client.database("PdmsRefnoDB");
    let refno_table = refno_db.collection::<PdmsRefno>("PdmsRefno");
    let db_name_opt = refno_table.find_one(doc! {"ref_no":refno}, None).await?;
    if let Some(value) = db_name_opt {
        let db_name = value.db;
        let db_name_tree = format!("{}_tree", db_name);
        let db = client.database(&db_name);
        let db_tree = client.database(&db_name_tree);
        let refno = query_design_component_by_refno_str_db(refno, &db, &db_tree).await.unwrap();
        let scom = resolve_cata_comp_attrs(refno, &db, &db_tree).await?;
        dbg!(&scom);
    }
    Ok(())
}

pub async fn query_design_component_by_refno_str_db(refno: &str, db: &Database, db_tree: &Database) -> mongodb::error::Result<DesignComponentData> {
    let tree_table = db_tree.collection::<EleDataNode>("PdmsTreeNode");
    let tree_node_opt = tree_table.find_one(
        doc! {"ref_no":refno},
        None,
    ).await?;
    if let Some(ele_data_node) = tree_node_opt {
        let result = query_design_component_db(ele_data_node, db, db_tree).await?;
        Ok(result)
    } else {
        let result = query_design_component_db(EleDataNode::default(), db, db_tree).await?;
        Ok(result)
    }
}

pub async fn query_design_component_db(ele: EleDataNode, db: &Database, db_tree: &Database) -> mongodb::error::Result<DesignComponentData> {
    let data = get_attr_string_db(ele, &db, &db_tree).await.unwrap();
    let data_map = &data.attr_data_map;
    let self_type = get_attr_value_as_string(data_map, "TYPE");
    let ddangle = get_attr_value_as_string(data_map, "ANGL");
    let height = get_attr_value_as_string(data_map, "HEIG");
    let radius = get_attr_value_as_string(data_map, "RADI");
    let ptre = get_attr_value_as_string(data_map, "PTRE");
    let scom_param_str = query_scom_str_db(data_map, &db, &db_tree).await?;
    let desparams = get_attr_value_f64_vec(data_map, "PARA").unwrap_or_default();
    let gtype_i32=get_attr_value_int(data_map,"GTYP");
    let gtype=db1_dehash(gtype_i32 as u32 );
    match &self_type[..] {
        "TUBI" => {
            //let itlength = get_attr_string!(ele, ITLE).to_lowercase().replace("mm", "").replace(" ", "");
            //Length of implied tube
            let itlength = get_attr_value_as_string(data_map, "ITLE");
            let pos = [0.0, 0.0, 0.0];
            let ori = DMat4::default();
            //圆柱体默认是X方向
            let direction = ori * DVec4::new(1.0, 0.0, 0.0, 0.0);

            Ok(DesignComponentData {
                name: data.name,
                refno: data.ref_no,
                owner: data.owner,
                self_type,
                //spref_name: get_attr_string!(ele, SPRE),
                spref_name: get_attr_value_as_string(&data_map, "SPRE"),
                gtype,
                scom_param_str: Some(scom_param_str),
                ddangle,
                height,
                itlength,
                radius,
                // world_matrix: vec![
                //     1.0f64, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
                // ],
                world_matrix: get_world_matrix_f64_db(&data_map),
                world_position: pos.to_vec(),
                ldirection: direction.data.0.to_vec()[0].to_vec(),
                // oriflag: true,
                // posflag: false,
                desparams,
            })
        }
        _ => {
            Ok(DesignComponentData {
                name: data.name,
                // name: DbElementType::new().get_full_name(),
                refno: data.ref_no,
                owner: data.owner,
                self_type,
                spref_name: get_attr_value_as_string(&data_map, "SPRE"),
                gtype,
                scom_param_str: Some(scom_param_str),
                ddangle,
                height,
                itlength: "".to_string(),
                radius,
                world_matrix: get_world_matrix_f64_db(&data_map),
                world_position: vec![],
                ldirection: vec![],
                // oriflag: false,
                // posflag: false,
                desparams,
            })
        }
    }
}

pub async fn query_scom_str_db(ele: &DashMap<String, AttrVal>, db: &Database, db_tree: &Database) -> mongodb::error::Result<ScomParamStr> {
    let ele_ptset = get_attr_value_as_string(ele, "PTRE");
    let table = db.collection::<ElementData>("PTSE");
    let mut ptre_node = ElementData::default();
    if let Some(value) = table.find_one(doc! {"ref_no":ele_ptset}, None).await? {
        ptre_node = value;
    };
    let axis_param_map = query_axis_param_strs_db(&ptre_node, &db, &db_tree).await?;

    let axis_param_strs = axis_param_map.values().cloned().collect::<Vec<_>>();
    let axis_param_number = axis_param_map.keys().cloned().collect::<Vec<_>>();
    let ele_gmset = get_attr_value_as_string(ele, "GMRE");
    let gmse_table = db.collection::<ElementData>("GMSE");
    let mut gmse_node = ElementData::default();
    if let Some(value) = gmse_table.find_one(doc! {"ref_no":ele_gmset}, None).await? {
        gmse_node = value;
    };
    let gmse_param_strs = query_gmse_param_strs_db(&gmse_node, &db, &db_tree).await?;
    //dbg!(&gmse_param_strs);
    Ok(ScomParamStr {
        name: get_attr_value_as_string(ele, "NAME"),
        refno: ptre_node.ref_no,
        self_type: ptre_node.noun_name,
        gtype: get_attr_value_as_string(ele, "GTYP"),
        owner: ptre_node.owner,
        dataset_param_strs: vec![],
        gmse_param_strs,
        axis_param_collections: axis_param_strs,
        params: get_attr_value_as_string(ele, "PARA").replace("\n", " ").replace("  ", " "),
        axis_param_number,
    })
}

pub async fn query_axis_param_strs_db(ele: &ElementData, db: &Database, db_tree: &Database) -> mongodb::error::Result<BTreeMap<i32, AxisParam>> {
    // 查找ptse
    let mut map = BTreeMap::new();
    let table_tree = db_tree.collection::<EleDataNode>("PdmsTreeNode");
    for child_refno in &ele.children {
        let child_data_tree = table_tree.find_one(
            doc! {"ref_no":child_refno.clone(),},
            None,
        ).await?.unwrap();
        let child_type = child_data_tree.type_name;
        let table = db.collection::<ElementData>(&child_type);
        let node = table.find_one(
            doc! {"ref_no":child_refno,},
            None,
        ).await?.unwrap();
        let child_node_map = &node.attr_data_map;
        let number = get_attr_value_int(child_node_map, "NUMB");
        map.entry(number).or_insert(query_axis_param_str_db(node));
    }
    Ok(map)
}

pub async fn query_gmse_param_strs_db(ele: &ElementData, db: &Database, db_tree: &Database) -> mongodb::error::Result<Vec<GmseParam>> {
    let mut gmses = vec![];
    let table_tree = db_tree.collection::<EleDataNode>("PdmsTreeNode");
    for child_refno in &ele.children {
        let child_data_tree = table_tree.find_one(
            doc! {"ref_no":child_refno.clone(),},
            None,
        ).await?.unwrap();
        let child_type = child_data_tree.type_name;
        let table = db.collection::<ElementData>(&child_type);
        let node = table.find_one(
            doc! {"ref_no":child_refno,},
            None,
        ).await?.unwrap();
        gmses.push(query_gmse_param_str_db(&node));
    }
    Ok(gmses)
}

///todo 结合设计模块的构件参数，对元件库的属性进行求值计算
/// ele: DESI Element
pub async fn resolve_desi_comp_attrs(ele: DesignComponentData) {
    //1、get desp params
    //2、insert to context hashmap
    //params.insert(format!("DESP{}", index), d.to_string());
    //3、resolve_cata_comp_attrs
}


const DDHEIGHT_STR: &'static str = "DDHEIGHT";
const DDRADIUS_STR: &'static str = "DDRADIUS";
const DDANGLE_STR: &'static str = "DDANGLE";

///对元件库的属性进行求值计算
/// ele: SCOM Element
pub async fn resolve_cata_comp_attrs(ele: DesignComponentData, db: &Database, db_tree: &Database) -> mongodb::error::Result<DesignComponent> {
    let table = db.collection::<ElementData>(&ele.self_type);
    let data = table.find_one(doc! {"ref_no":&ele.refno}, None).await?.unwrap();
    let data_map = &data.attr_data_map;
    let scom = &ele.scom_param_str.clone().unwrap();
    let mut context = HashMap::new();//parse_param_to_hashmap(&scom.params);
    context.insert(DDHEIGHT_STR.to_string(), ele.height.clone());
    context.insert(DDRADIUS_STR.to_string(), ele.radius.clone());
    context.insert(DDANGLE_STR.to_string(), ele.ddangle.clone());
    //获取DTSE的expression
    get_dtse_params(&data, db, db_tree, &mut context);
    context.insert("PARAM".to_string(), "0".to_string());
    for i in 0..ele.desparams.len() {
        context.insert(format!("PARAM{}", i + 1), ele.desparams[i].to_string());
        context.insert(format!("IPARAM{}", i + 1), ele.desparams[i].to_string());
    }
    //log::info!("当前构件是:{},元件库参数是:{}", &ele.refno, &scom.refno);
    match &ele.self_type[..] {
        "TUBI" => {
            /// tubi 采用世界坐标系，不需要根据 world matrix 变换
            let itlength = match ele.itlength.parse::<f64>() {
                Ok(x) => x,
                Err(_) => 0.0f64,
            };
            let diameter = (&context["PARAM2"]).parse::<f64>().unwrap_or(0.0f64);
            let geometries =
                vec![GeoParamsData {
                    cate_geo_params: Some(TubeImplied(CateTubeImpliedParam {
                        center_position: ele.world_position.clone(),
                        direction: ele.ldirection.clone(),
                        diameter,
                        height: itlength,
                        centre_line_flag: true,
                        tube_flag: true,
                    }))
                }];

            Ok(DesignComponent {
                name: ele.name,
                refno: ele.refno.clone(),
                owner: ele.owner,
                spref_name: ele.spref_name,
                self_type: ele.self_type,
                gtype: ele.gtype,
                geometries,
                world_matrix: ele.world_matrix,
                // oriflag: comp_str.oriflag,
                // posflag: comp_str.posflag
            })
        }
        _ => {
            let ddangle = match ele.ddangle.parse::<f64>() {
                Ok(x) => Some(x),
                Err(_) => None,
            };
            //求解AXIS的数据
            let axis_params_map = resolve_axis_params(
                &scom, &context, &data,
            );
            //求解子节点几何模型的数据
            dbg!(&scom.gmse_param_strs);
            let mut geometries = resolve_gmses(
                &scom.gmse_param_strs,
                &context,
                &axis_params_map,
                ddangle,
            );

            Ok(DesignComponent {
                name: ele.name,
                refno: ele.refno.clone(),
                owner: ele.owner,
                spref_name: ele.spref_name,
                self_type: ele.self_type,
                gtype: ele.gtype,
                geometries,
                world_matrix: ele.world_matrix,
                // oriflag: comp_str.oriflag,
                // posflag: comp_str.posflag
            })
        }
    }
}

///deprected
pub fn parse_param_to_hashmap(text: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();
    let mut index = 1;
    for d in text.replace("\n", " ").split(" ") {
        if d != "" && d != " " {
            params.insert(format!("PARAM{}", index), d.to_string());
            params.insert(format!("IPARAM{}", index), "0.0".to_string());
            index += 1;
        }
    }
    //todo 保温层厚度
    params.insert("IPARAM".to_string(), "0.0".to_string());
    params
}

pub fn query_axis_param_str_db(ele: ElementData) -> AxisParam {
    let ele_map = ele.attr_data_map;
    let self_type = get_attr_value_as_string(&ele_map, "TYPE");
    let pconnect = get_attr_value_as_string(&ele_map, "PCON");
    let pbore = get_attr_value_as_string(&ele_map, "PBOR");
    let refno = ele.ref_no;
    let pos = get_attr_value_f64_vec(&ele_map, "POS").unwrap_or(vec![0.0, 0.0, 0.0]);
    match self_type.as_ref() {
        "PTAX" => {
            AxisParam {
                self_type,
                refno,
                x: "".to_string(),
                y: "".to_string(),
                z: "".to_string(),
                distance: get_attr_value_as_string(&ele_map, "PDIS"),
                direction: get_attr_value_as_string(&ele_map, "PAXI"),
                pconnect,
                pbore,
            }
        }
        "PTCA" => {
            AxisParam {
                self_type,
                refno,
                x: get_attr_value_as_string(&ele_map, "PX"),
                y: get_attr_value_as_string(&ele_map, "PY"),
                z: get_attr_value_as_string(&ele_map, "PZ"),
                distance: "".to_string(),
                direction: get_attr_value_as_string(&ele_map, "PTCD"),
                pconnect,
                pbore,
            }
        }
        "PTMI" => {
            AxisParam {
                self_type,
                refno,
                x: get_attr_value_as_string(&ele_map, "PX"),
                y: get_attr_value_as_string(&ele_map, "PY"),
                z: get_attr_value_as_string(&ele_map, "PZ"),
                distance: "".to_string(),
                direction: get_attr_value_as_string(&ele_map, "PAXI"),
                pconnect,
                pbore,
            }
        }
        "PTPOS" => {  // =15213/627794
            AxisParam {
                self_type,
                refno,
                x: "".to_string(),
                y: "".to_string(),
                z: "".to_string(),
                distance: get_attr_value_as_string(&ele_map, "PTCPOS"),
                direction: get_attr_value_as_string(&ele_map, "PTCD"),
                pconnect,
                pbore,
            }
        }
        _ => {
            AxisParam {
                self_type: "".to_string(),
                refno,
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

pub fn query_gmse_param_str_db(ele: &ElementData) -> GmseParam {
    let ele_map = &ele.attr_data_map;
    let mut paxises = get_attr_strings_db(ele_map, &["PAXI", "PAAX", "PBAX", "PCAX"]);
    let mut pts = vec![];
    if let Some(value) = ele_map.get("PTS") {
        match value.value() {
            IntArrayType(value) => { pts = value.to_vec() }
            _ => {}
        }
    }
    for s in pts {
        paxises.push(s.to_string());
    }
    let centre_line_flag = get_attr_value_as_string(ele_map, "CLFL") == "True";
    let tube_flag = get_attr_value_as_string(ele_map, "TUFL") == "False";
    GmseParam {
        name: ele.name.clone(),
        refno: ele.ref_no.clone(),
        owner: ele.owner.clone(),
        self_type: ele.noun_name.clone(),  // SCYL  LSNO  SCTO  SDSH  SBOX ......

        radius: get_attr_value_as_string(ele_map, "PRAD"),
        diameters: get_attr_strings_db(ele_map, &["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: get_attr_strings_db(ele_map, &["PDIS", "PBDI", "PTDI"]),
        height: get_attr_value_as_string(ele_map, "PHEI"),
        offset: get_attr_value_as_string(ele_map, "POFF"),
        // box_lengths: get_attr_strings_db(ele_map, &["PXEL", "PYEL", "PZEL"]),
        box_lengths: get_attr_strings_db(ele_map, &["PXLE", "PYLE", "PZLE"]),
        xyz: get_attr_strings_db(
            ele_map,
            &["PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF"],
        ),
        paxises,      // 先pa_axis, 后pb_axis
        centre_line_flag,
        tube_flag,
    }
}

pub async fn get_dtse_params(data: &ElementData, db: &Database, db_tree: &Database, context: &mut HashMap<String, String>) -> mongodb::error::Result<()> {
    let tree_table = db_tree.collection::<EleDataNode>("PdmsTreeNode");
    let ele = &data.attr_data_map;
    //let dtse = ele.get_attr_ele(&crate::attr_raw!(DTRE));
    let dtse = get_attr_value_as_string(&ele, "DTRE");
    for child in &data.children {
        let child_data_tree = tree_table.find_one(
            doc! {"ref_no":child,},
            None,
        ).await?.unwrap();
        let child_type = child_data_tree.type_name;
        let table = db.collection::<ElementData>(&child_type);
        let node = table.find_one(
            doc! {"ref_no":child},
            None,
        ).await?.unwrap();
        let node_map = node.attr_data_map;
        // let key = child.get_attr_string(&crate::attr_raw!(DKEY));
        let key = get_attr_value_as_string(&node_map, "DKEY");
        // let exp = child.get_attr_string(&crate::attr_raw!(PPRO));
        let exp = get_attr_value_as_string(&node_map, "PPRO");
        let default_key = format!("{}_default_expr", key);
        // let default_expr = child.get_attr_string(&crate::attr_raw!(DPRO));
        let default_expr = get_attr_value_as_string(&node_map, "DPRO");
        context.insert(key, exp);
        context.insert(default_key, default_expr);
    }
    Ok(())
}
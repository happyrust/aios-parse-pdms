use smol_str::SmolStr;
use crate::AttrMap;



//设计模块的信息
// #[derive(Clone, Debug, Default)]
// pub struct DesCompInfo {
//     pub name: SmolStr,
//     pub refno: SmolStr,
//     pub owner: SmolStr,
//     pub spre_name: SmolStr,
//     pub type_name: SmolStr,
//     pub gtype: SmolStr,
//     pub scom_info: ::core::option::Option<ScomInfo>,
//     pub ddangle: SmolStr,
//     pub height: SmolStr,
//     pub radius: SmolStr,
//     pub world_matrix: Vec<f64>,
//     pub world_position: Vec<f64>,
//     pub desparams: Vec<f64>,
// }

#[derive(Clone, Debug)]
pub struct ScomInfo {
    pub attr_map: AttrMap,
    pub gtype: SmolStr,
    pub dtse_params: Vec<DatasetParamStr>,
    pub gmse_params: Vec<GmseParam>,
    pub axis_params: Vec<AxisParam>,
    pub params: SmolStr,
    pub axis_param_numbers: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct DatasetParamStr {
    pub refno: SmolStr,
    pub name: SmolStr,
    pub self_type: SmolStr,
    pub lock: bool,
    pub owner: SmolStr,
    pub description: SmolStr,
    pub dkey: SmolStr,
    pub ptype: SmolStr,
    pub pproperty: SmolStr,
    pub dproperty: SmolStr,
    pub purpose: SmolStr,
    pub number: i32,
    pub dtitle: SmolStr,
    pub punits: SmolStr,
    pub ruse: SmolStr,
    pub lhide: bool,
}

#[derive(Clone, Debug, Default)]
pub struct GmseParam {
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub attr_map: AttrMap,
    pub radius: SmolStr,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<SmolStr>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<SmolStr>,
    pub height: SmolStr,
    pub offset: SmolStr,
    /// 顺序 x y z
    pub box_lengths: Vec<SmolStr>,
    pub xyz: Vec<SmolStr>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<SmolStr>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AxisParam {
    pub attr_map: AttrMap,
    pub x: SmolStr,
    pub y: SmolStr,
    pub z: SmolStr,
    pub distance: SmolStr,
    pub direction: SmolStr,
    pub pconnect: SmolStr,
    pub pbore: SmolStr,
}

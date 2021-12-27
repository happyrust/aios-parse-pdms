use crate::AttrMap;

#[derive(Clone, Debug, Default)]
pub struct DesignPipeStr {
    pub name: String,
    pub refno: String,
    pub bran_refnos: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DesignBranStr {
    pub name: String,
    pub refno: String,
    pub design_component_data_vec: Vec<DesCompInfo>,
}

//设计模块的信息
#[derive(Clone, Debug, Default)]
pub struct DesCompInfo {
    pub name: String,
    pub refno: String,
    pub owner: String,
    pub spref_name: String,
    pub type_name: String,
    pub gtype: String,
    pub scom_info: ::core::option::Option<ScomInfo>,
    pub ddangle: String,
    pub height: String,
    pub radius: String,
    pub world_matrix: Vec<f64>,
    pub world_position: Vec<f64>,
    pub desparams: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ScomInfo {
    pub attr_map: AttrMap,
    pub name: String,
    pub gtype: String,
    pub dtse_params: Vec<DatasetParamStr>,
    pub gmse_params: Vec<GmseParam>,
    pub axis_params: Vec<AxisParam>,
    pub params: String,
    pub axis_param_numbers: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct DatasetParamStr {
    pub refno: String,
    pub name: String,
    pub self_type: String,
    pub lock: bool,
    pub owner: String,
    pub description: String,
    pub dkey: String,
    pub ptype: String,
    pub pproperty: String,
    pub dproperty: String,
    pub purpose: String,
    pub number: i32,
    pub dtitle: String,
    pub punits: String,
    pub ruse: String,
    pub lhide: bool,
}

#[derive(Clone, Debug, Default)]
pub struct GmseParam {
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub attr_map: AttrMap,
    pub radius: String,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<String>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<String>,
    pub height: String,
    pub offset: String,
    /// 顺序 x y z
    pub box_lengths: Vec<String>,
    pub xyz: Vec<String>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<String>,
    pub centre_line_flag: bool,
    pub tube_flag: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AxisParam {
    pub attr_map: AttrMap,
    pub x: String,
    pub y: String,
    pub z: String,
    pub distance: String,
    pub direction: String,
    pub pconnect: String,
    pub pbore: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignPipeStr {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "3")]
    pub bran_refnos: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignBranStr {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub design_component_strs: ::prost::alloc::vec::Vec<DesignComponentData>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignComponentData {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub owner: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub spref_name: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub gtype: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "7")]
    pub scom_param_str: ::core::option::Option<ScomParamStr>,
    #[prost(string, tag = "8")]
    pub ddangle: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub height: ::prost::alloc::string::String,
    #[prost(string, tag = "10")]
    pub itlength: ::prost::alloc::string::String,
    #[prost(string, tag = "11")]
    pub radius: ::prost::alloc::string::String,
    #[prost(double, repeated, tag = "12")]
    pub world_matrix: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "13")]
    pub world_position: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "14")]
    pub ldirection: ::prost::alloc::vec::Vec<f64>,
    ///  bool oriflag = 16;
    ///  bool posflag = 17;
    #[prost(double, repeated, tag = "15")]
    pub desparams: ::prost::alloc::vec::Vec<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScomParamStr {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub gtype: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub owner: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "6")]
    pub dataset_param_strs: ::prost::alloc::vec::Vec<DatasetParamStr>,
    #[prost(message, repeated, tag = "7")]
    pub gmse_param_strs: ::prost::alloc::vec::Vec<GmseParam>,
    #[prost(message, repeated, tag = "8")]
    pub axis_param_collections: ::prost::alloc::vec::Vec<AxisParam>,
    #[prost(string, tag = "9")]
    pub params: ::prost::alloc::string::String,
    ///axis 的number属性
    #[prost(int32, repeated, tag = "10")]
    pub axis_param_number: ::prost::alloc::vec::Vec<i32>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DatasetParamStr {
    #[prost(string, tag = "1")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(bool, tag = "4")]
    pub lock: bool,
    #[prost(string, tag = "5")]
    pub owner: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub description: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub dkey: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub ptype: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub pproperty: ::prost::alloc::string::String,
    #[prost(string, tag = "10")]
    pub dproperty: ::prost::alloc::string::String,
    #[prost(string, tag = "11")]
    pub purpose: ::prost::alloc::string::String,
    #[prost(int32, tag = "12")]
    pub number: i32,
    #[prost(string, tag = "13")]
    pub dtitle: ::prost::alloc::string::String,
    #[prost(string, tag = "14")]
    pub punits: ::prost::alloc::string::String,
    #[prost(string, tag = "15")]
    pub ruse: ::prost::alloc::string::String,
    #[prost(bool, tag = "16")]
    pub lhide: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GmseParam {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub owner: ::prost::alloc::string::String,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    #[prost(string, tag = "4")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub radius: ::prost::alloc::string::String,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    #[prost(string, repeated, tag = "6")]
    pub diameters: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    #[prost(string, repeated, tag = "7")]
    pub distances: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, tag = "8")]
    pub height: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub offset: ::prost::alloc::string::String,
    /// 顺序 x y z
    #[prost(string, repeated, tag = "10")]
    pub box_lengths: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, repeated, tag = "12")]
    pub xyz: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    #[prost(string, repeated, tag = "13")]
    pub paxises: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(bool, tag = "14")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "15")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AxisParam {
    #[prost(string, tag = "1")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub x: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub y: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub z: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub distance: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub direction: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub pconnect: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub pbore: ::prost::alloc::string::String,
}

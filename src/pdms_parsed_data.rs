#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignPipeRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignComponentRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignBranRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RefnosRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Refnos {
    #[prost(string, repeated, tag = "1")]
    pub refnos: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignPipe {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub brans: ::prost::alloc::vec::Vec<DesignBran>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignBran {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(message, repeated, tag = "3")]
    pub components: ::prost::alloc::vec::Vec<DesignComponent>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DesignComponent {
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
    #[prost(message, repeated, tag = "8")]
    pub geometries: ::prost::alloc::vec::Vec<GeoParamsData>,
    ///  bool oriflag = 10;
    ///  bool posflag = 11;
    #[prost(double, repeated, tag = "9")]
    pub world_matrix: ::prost::alloc::vec::Vec<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Dataset {
    #[prost(string, tag = "1")]
    pub self_type: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GmseParamData {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub refno: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub owner: ::prost::alloc::string::String,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    #[prost(string, tag = "4")]
    pub self_type: ::prost::alloc::string::String,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(double, tag = "6")]
    pub angle: f64,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    #[prost(double, repeated, tag = "7")]
    pub diameters: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    #[prost(double, repeated, tag = "8")]
    pub distances: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "9")]
    pub height: f64,
    #[prost(double, tag = "10")]
    pub offset: f64,
    /// 顺序 x y z
    #[prost(double, repeated, tag = "11")]
    pub box_lengths: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "12")]
    pub xyz: ::prost::alloc::vec::Vec<f64>,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    #[prost(message, repeated, tag = "13")]
    pub paxises: ::prost::alloc::vec::Vec<CateAxisParam>,
    #[prost(bool, tag = "14")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "15")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateAxisParam {
    #[prost(double, repeated, tag = "1")]
    pub pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub dir: ::prost::alloc::vec::Vec<f64>,
    #[prost(string, tag = "3")]
    pub pconnect: ::prost::alloc::string::String,
    #[prost(double, tag = "4")]
    pub pbore: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GeoParamsData {
    #[prost(
        oneof = "geo_params_data::CateGeoParams",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17"
    )]
    pub cate_geo_params: ::core::option::Option<geo_params_data::CateGeoParams>,
}
/// Nested message and enum types in `GeoParamsData`.
pub mod geo_params_data {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum CateGeoParams {
        #[prost(message, tag = "1")]
        Boxi(super::CateBoxImpliedParam),
        #[prost(message, tag = "2")]
        Box(super::CateBoxParam),
        #[prost(message, tag = "3")]
        Cone(super::CateConeParam),
        #[prost(message, tag = "4")]
        Cylinder(super::CateCylinderParam),
        #[prost(message, tag = "5")]
        Disc(super::CateDiscParam),
        #[prost(message, tag = "6")]
        Dish(super::CateDishParam),
        #[prost(message, tag = "7")]
        Extrusion(super::CateExtrusionParam),
        #[prost(message, tag = "8")]
        Line(super::CateLineParam),
        #[prost(message, tag = "9")]
        Pyramid(super::CatePyramidParam),
        #[prost(message, tag = "10")]
        RectTorus(super::CateRectTorusParam),
        #[prost(message, tag = "11")]
        Revolution(super::CateRevolutionParam),
        #[prost(message, tag = "12")]
        Sline(super::CateSlineParam),
        #[prost(message, tag = "13")]
        SlopeBottomCylinder(super::CateSlopeBottomCylinderParam),
        #[prost(message, tag = "14")]
        Snout(super::CateSnoutParam),
        #[prost(message, tag = "15")]
        Sphere(super::CateSphereParam),
        #[prost(message, tag = "16")]
        Torus(super::CateTorusParam),
        #[prost(message, tag = "17")]
        TubeImplied(super::CateTubeImpliedParam),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateBoxImpliedParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub x_length: f64,
    #[prost(double, tag = "3")]
    pub z_length: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateBoxParam {
    #[prost(double, repeated, tag = "1")]
    pub size: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub offset: ::prost::alloc::vec::Vec<f64>,
    #[prost(bool, tag = "3")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "4")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateConeParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateCylinderParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateExtrusionParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub x: f64,
    #[prost(double, tag = "5")]
    pub y: f64,
    #[prost(double, tag = "6")]
    pub z: f64,
    #[prost(bool, tag = "7")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "8")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateDiscParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateDishParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(bool, tag = "6")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "7")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateLineParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CatePyramidParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "3")]
    pub pc: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "4")]
    pub x_bottom: f64,
    #[prost(double, tag = "5")]
    pub y_bottom: f64,
    #[prost(double, tag = "6")]
    pub x_top: f64,
    #[prost(double, tag = "7")]
    pub y_top: f64,
    #[prost(double, tag = "8")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "9")]
    pub dist_to_top: f64,
    #[prost(double, tag = "10")]
    pub x_offset: f64,
    #[prost(double, tag = "11")]
    pub y_offset: f64,
    #[prost(bool, tag = "12")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "13")]
    pub tube_flag: bool,
}
/// 截面为矩形的弯管
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateRectTorusParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub height: f64,
    #[prost(double, tag = "4")]
    pub diameter: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateRevolutionParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub angel: f64,
    #[prost(double, tag = "4")]
    pub x: f64,
    #[prost(double, tag = "5")]
    pub y: f64,
    #[prost(double, tag = "6")]
    pub z: f64,
    #[prost(bool, tag = "7")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "8")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSlineParam {
    #[prost(double, repeated, tag = "1")]
    pub start_pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub end_pt: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSlopeBottomCylinderParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub height: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(double, tag = "4")]
    pub distance: f64,
    #[prost(double, tag = "5")]
    pub x_shear: f64,
    #[prost(double, tag = "6")]
    pub y_shear: f64,
    #[prost(double, tag = "7")]
    pub alt_x_shear: f64,
    #[prost(double, tag = "8")]
    pub alt_y_shear: f64,
    #[prost(bool, tag = "9")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "10")]
    pub tube_flag: bool,
}
/// 圆台 或 管嘴
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSnoutParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub dist_to_btm: f64,
    #[prost(double, tag = "4")]
    pub dist_to_top: f64,
    #[prost(double, tag = "5")]
    pub btm_diameter: f64,
    #[prost(double, tag = "6")]
    pub top_diameter: f64,
    #[prost(double, tag = "7")]
    pub offset: f64,
    #[prost(bool, tag = "8")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "9")]
    pub tube_flag: bool,
}
/// 球
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateSphereParam {
    #[prost(message, optional, tag = "1")]
    pub axis: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "2")]
    pub dist_to_center: f64,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
///元件库里的torus参数
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateTorusParam {
    #[prost(message, optional, tag = "1")]
    pub pa: ::core::option::Option<CateAxisParam>,
    #[prost(message, optional, tag = "2")]
    pub pb: ::core::option::Option<CateAxisParam>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(bool, tag = "4")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "5")]
    pub tube_flag: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CateTubeImpliedParam {
    #[prost(double, repeated, tag = "1")]
    pub center_position: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag = "2")]
    pub direction: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, tag = "3")]
    pub diameter: f64,
    #[prost(double, tag = "4")]
    pub height: f64,
    #[prost(bool, tag = "5")]
    pub centre_line_flag: bool,
    #[prost(bool, tag = "6")]
    pub tube_flag: bool,
}
#[doc = r" Generated client implementations."]
pub mod query_pdms_data_trait_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct QueryPdmsDataTraitClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl QueryPdmsDataTraitClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> QueryPdmsDataTraitClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + Sync + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> QueryPdmsDataTraitClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            QueryPdmsDataTraitClient::new(InterceptedService::new(inner, interceptor))
        }
        #[doc = r" Compress requests with `gzip`."]
        #[doc = r""]
        #[doc = r" This requires the server to support it otherwise it might respond with an"]
        #[doc = r" error."]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        #[doc = r" Enable decompressing responses with `gzip`."]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn query_design_component(
            &mut self,
            request: impl tonic::IntoRequest<super::DesignComponentRequest>,
        ) -> Result<tonic::Response<super::DesignComponent>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignComponent",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn query_design_bran(
            &mut self,
            request: impl tonic::IntoRequest<super::DesignBranRequest>,
        ) -> Result<tonic::Response<super::DesignBran>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignBran",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn query_design_pipe(
            &mut self,
            request: impl tonic::IntoRequest<super::DesignPipeRequest>,
        ) -> Result<tonic::Response<super::DesignPipe>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignPipe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn query_refnos(
            &mut self,
            request: impl tonic::IntoRequest<super::RefnosRequest>,
        ) -> Result<tonic::Response<super::Refnos>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryRefnos",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod query_pdms_data_trait_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with QueryPdmsDataTraitServer."]
    #[async_trait]
    pub trait QueryPdmsDataTrait: Send + Sync + 'static {
        async fn query_design_component(
            &self,
            request: tonic::Request<super::DesignComponentRequest>,
        ) -> Result<tonic::Response<super::DesignComponent>, tonic::Status>;
        async fn query_design_bran(
            &self,
            request: tonic::Request<super::DesignBranRequest>,
        ) -> Result<tonic::Response<super::DesignBran>, tonic::Status>;
        async fn query_design_pipe(
            &self,
            request: tonic::Request<super::DesignPipeRequest>,
        ) -> Result<tonic::Response<super::DesignPipe>, tonic::Status>;
        async fn query_refnos(
            &self,
            request: tonic::Request<super::RefnosRequest>,
        ) -> Result<tonic::Response<super::Refnos>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct QueryPdmsDataTraitServer<T: QueryPdmsDataTrait> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: QueryPdmsDataTrait> QueryPdmsDataTraitServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for QueryPdmsDataTraitServer<T>
    where
        T: QueryPdmsDataTrait,
        B: Body + Send + Sync + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignComponent" => {
                    #[allow(non_camel_case_types)]
                    struct QueryDesignComponentSvc<T: QueryPdmsDataTrait>(pub Arc<T>);
                    impl<T: QueryPdmsDataTrait>
                        tonic::server::UnaryService<super::DesignComponentRequest>
                        for QueryDesignComponentSvc<T>
                    {
                        type Response = super::DesignComponent;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DesignComponentRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).query_design_component(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = QueryDesignComponentSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignBran" => {
                    #[allow(non_camel_case_types)]
                    struct QueryDesignBranSvc<T: QueryPdmsDataTrait>(pub Arc<T>);
                    impl<T: QueryPdmsDataTrait>
                        tonic::server::UnaryService<super::DesignBranRequest>
                        for QueryDesignBranSvc<T>
                    {
                        type Response = super::DesignBran;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DesignBranRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).query_design_bran(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = QueryDesignBranSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryDesignPipe" => {
                    #[allow(non_camel_case_types)]
                    struct QueryDesignPipeSvc<T: QueryPdmsDataTrait>(pub Arc<T>);
                    impl<T: QueryPdmsDataTrait>
                        tonic::server::UnaryService<super::DesignPipeRequest>
                        for QueryDesignPipeSvc<T>
                    {
                        type Response = super::DesignPipe;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DesignPipeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).query_design_pipe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = QueryDesignPipeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pdms_parsed_data.QueryPdmsDataTrait/QueryRefnos" => {
                    #[allow(non_camel_case_types)]
                    struct QueryRefnosSvc<T: QueryPdmsDataTrait>(pub Arc<T>);
                    impl<T: QueryPdmsDataTrait> tonic::server::UnaryService<super::RefnosRequest>
                        for QueryRefnosSvc<T>
                    {
                        type Response = super::Refnos;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RefnosRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).query_refnos(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = QueryRefnosSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: QueryPdmsDataTrait> Clone for QueryPdmsDataTraitServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: QueryPdmsDataTrait> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: QueryPdmsDataTrait> tonic::transport::NamedService for QueryPdmsDataTraitServer<T> {
        const NAME: &'static str = "pdms_parsed_data.QueryPdmsDataTrait";
    }
}

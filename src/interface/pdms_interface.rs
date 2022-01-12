
use mongodb::Client;
use mongodb::bson::doc;
use std::collections::HashSet;
use std::error::Error;
use dashmap::DashMap;
use crate::query::{query_scom_info, resolve_cata_comp_async, resolve_desi_comp};
use crate::parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, AttrVal, EleDataNode, ElementData, PDMSDBInfo, PdmsRefno};
use futures::stream::TryStreamExt;
use mongodb::options::{FindOneOptions, FindOptions};
use crate::helper::get_attr_value_f64_vec;
use crate::pdms_data::ScomInfo;

type MResult<T> = mongodb::error::Result<T>;

#[derive(Debug, Default)]
pub struct PdmsInterface {
    connection_str: String,
    client: Option<Client>,
}

impl PdmsInterface {

    pub fn new(url: &str) -> Self{
        // let client_uri = "mongodb://localhost:27017".to_string();
        Self{
            connection_str: url.to_string(),
            client: None,
        }
    }

    #[inline]
    pub async fn connect(&mut self) -> Option<Client>{
        if self.client.is_none(){
            if let Ok(client) = Client::with_uri_str(&self.connection_str).await{
                self.client = Some(client);
            }else{
                return None;
            }
        }
        self.client.clone()
    }

    pub async fn get_scom_info_async(&mut self, refno: &str) -> MResult<Option<ScomInfo>> {
        if let Some(client) = self.connect().await {
            if let Some(scom_info) = query_scom_info(refno, self).await?{
                return Ok(Some(scom_info));
            }
        }
        Ok(None)
    }

    pub fn get_scom_info(&mut self, refno: &str) -> Option<ScomInfo> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_scom_info_async(refno)).unwrap()
    }

    ///获得cata ele对应的几何体
    pub async fn get_cata_ele_geoms_async(&mut self, refno: &str) -> MResult<Option<GeomsInfo>> {
        if let Some(client) = self.connect().await {
            if let Some(scom_info) = query_scom_info(refno, self).await?{
                let scom = resolve_cata_comp_async(&scom_info, self, None).await?;
                return Ok(Some(scom));
            }
        }
        Ok(None)
    }

    /// 同步方法，获得desi ele对应的几何体
    pub fn get_des_ele_geoms(&mut self, refno: &str) -> Option<GeomsInfo> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_des_ele_geoms_async(refno)).unwrap()
    }

    ///获得desi ele对应的几何体
    pub async fn get_des_ele_geoms_async(&mut self, refno: &str) -> MResult<Option<GeomsInfo>> {
        if let Some(client) = self.connect().await {
            if let Some(geoms) = resolve_desi_comp(refno, self).await?{
                // dbg!(&geoms);
                return Ok(Some(geoms));
            }
        }
        Ok(None)
    }

    pub fn get_des_matrix(&mut self, refno: &str) -> glam::f32::Affine3A {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_des_matrix_async(refno)).unwrap()
    }

    ///获得构件的变换矩阵
    pub async fn get_des_matrix_async(&mut self, refno: &str) -> MResult<glam::f32::Affine3A> {
        if let Some(client) = self.connect().await {
            if let Some(attr) = self.get_ele_attr_map_async(refno).await?{
                if let Some(pos) = get_attr_value_f64_vec(&attr, "POS"){
                    if let Some(ang) = get_attr_value_f64_vec(&attr, "ORI"){
                        return Ok(glam::f32::Affine3A{
                            matrix3: glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32) * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)  * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32) ,
                            translation: glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
                        });
                    }
                }
            }
        }
        Ok(glam::f32::Affine3A::IDENTITY)
    }

    pub fn get_cata_ele_geoms(&mut self, refno: &str) -> Option<GeomsInfo> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_cata_ele_geoms_async(refno)).unwrap()
    }

    pub fn get_world(&mut self, db_name: &str) -> Option<EleDataNode> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_world_async(db_name)).unwrap()
    }

    pub async fn get_world_async(&mut self, db_name: &str) -> MResult<Option<EleDataNode>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&format!("{}_tree", db_name));
            let tree_collect = db.collection::<EleDataNode>("PdmsTreeNode");
            if let Some(d) = tree_collect.find_one(doc! {"owner" : "0/0"}, None).await?{
                return Ok(Some(d));
            }
        }
        Ok(None)
    }

    ///获得到当前refno所在得数据库
    pub async fn get_db_info_of_ele(&mut self, refno: &str) -> MResult<Option<PdmsRefno>>{
        if let Some(client) = self.connect().await {
            let db = client.database("PdmsRefnoDB");
            let t = db.collection::<PdmsRefno>("PdmsRefno");
            if let Some(d) = t.find_one(doc! {"ref_no":refno}, None).await?{
                return Ok(Some(d))
            }
        }
        Ok(None)
    }

    ///通过EleDataNode获取attr map 的同步方法
    pub fn get_ele_attr_map_by_node(&mut self, ele: &EleDataNode) -> Option<AttrMap> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_ele_attr_map_by_node_async(ele)).unwrap()
    }


    ///通过refno获取attr map 的同步方法
    pub fn get_ele_attr_map(&mut self, refno: &str) -> Option<AttrMap> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_ele_attr_map_async(refno)).unwrap()
    }

    pub async fn get_ele_attr_map_by_node_async(&mut self, ele: &EleDataNode) -> MResult<Option<AttrMap>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&ele.db_name);
            let t = db.collection::<ElementData>(&ele.type_name);
            let mut find_options = FindOneOptions::default();
            find_options.projection = Some(doc! {"attr_data_map": 1});
            if let Some(d) = t.find_one(doc!{ "ref_no": &ele.ref_no }, None).await?{
                return Ok(Some(AttrMap{
                    map: d.attr_data_map
                }));
            }
        }
        Ok(None)
    }

    pub async fn get_ele_attr_map_async(&mut self, refno: &str) -> MResult<Option<AttrMap>> {
        if let Some(client) = self.connect().await {
            if let Some(info) = self.get_db_info_of_ele(&refno).await?{
                let db = client.database(&info.db);
                let t = db.collection::<ElementData>(&info.type_name);
                let mut find_options = FindOneOptions::default();
                find_options.projection = Some(doc! {"attr_data_map": 1});
                if let Some(d) = t.find_one(doc!{ "ref_no": &refno }, None).await?{
                    return Ok(Some(AttrMap{
                        map: d.attr_data_map
                    }));
                }
            }
        }
        Ok(None)
    }

    ///获得ele data
    pub async fn get_ele_data_async(&mut self, refno: &str, type_name: &str) -> MResult<Option<ElementData>> {
        if let Some(client) = self.connect().await {
            if let Some(info) = self.get_db_info_of_ele(&refno).await?{
                let db = client.database(&info.db);
                let t = db.collection::<ElementData>(&info.type_name);
                if let Some(c) = t.find_one(doc!{ "ref_no": &refno }, None).await?{
                    return Ok(Some(c));
                }
            }
        }
        Ok(None)
    }

    pub async fn get_children_by_node_async(&mut self, ele: &EleDataNode) -> MResult<Vec<EleDataNode>> {
        self.get_children_async(&ele.ref_no).await
    }

    ///获得Children
    pub async fn get_children_async(&mut self, refno: &str) -> MResult<Vec<EleDataNode>> {
        let mut v = vec![];
        if let Some(client) = self.connect().await {
            if let Some(info) = self.get_db_info_of_ele(refno).await?{
                let db = client.database(&format!("{}_tree", info.db));
                let tree_collect = db.collection::<EleDataNode>("PdmsTreeNode");
                let mut cursor = tree_collect.find(doc! {"owner" : refno}, FindOptions::builder().sort( doc! { "order": 1 } ).build())/*.sort( doc! { "order": 1 } )*/.await?;
                while let Some(c) = cursor.try_next().await? {
                    v.push(c);
                }
            }
        }
        Ok(v)
    }

    pub fn get_eles_by_type(&mut self, db_name: &str, type_name: &str) -> Vec<AttrMap> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_ele_attr_map_by_type_async(db_name, type_name)).unwrap_or_default()
    }

    ///根据type name获取所有的节点
    pub async fn get_ele_attr_map_by_type_async(&mut self, db_name: &str, type_name: &str) -> MResult<Vec<AttrMap>> {
        let mut v = vec![];
        if let Some(client) = self.connect().await {
            let db = client.database(db_name);
            let t = db.collection::<ElementData>(type_name);
            // let mut find_options = FindOneOptions::default();
            // find_options.projection = Some(doc! {"attr_data_map": 1});
            let mut cursor = t.find(doc!{}, None ).await?;
            while let Some(c) = cursor.try_next().await? {
                v.push(AttrMap{
                    map: c.attr_data_map
                });
            }
        }
        Ok(v)
    }



    ///获取子节点的所有attr_map
    pub async fn get_children_attr_map_async(&mut self, refno: &str) -> MResult<Vec<AttrMap>> {
        let mut v = vec![];
        if let Some(client) = self.connect().await {
            let children = self.get_children_async(refno).await?;
            for child in children {
                if let Some(map) = self.get_ele_attr_map_by_node_async(&child).await?{
                    v.push(map );
                }
            }
        }
        Ok(v)
    }

    pub fn get_children_attr_map(&mut self, refno: &str) -> Vec<AttrMap> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_children_attr_map_async(refno)).unwrap_or_default()
    }

    pub fn get_children_by_node(&mut self, ele: &EleDataNode) -> Vec<EleDataNode> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_children_by_node_async(ele)).unwrap_or_default()
    }

    pub fn get_children(&mut self, refno: &str) -> Vec<EleDataNode> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_children_async(refno)).unwrap_or_default()
    }


    pub fn get_tubi_bore_by_refno(&mut self, refno: &str) -> f32 {
        if let Some(attr_map) = self.get_ele_attr_map(refno) {
            let lstu_ref = attr_map.get_as_string("LSTU").unwrap_or_default();
            if let Some(lstu) = self.get_ele_attr_map(lstu_ref.as_str()) {
                let catr_ref = lstu.get_as_string("CATR").unwrap_or_default();
                if let Some(cata) = self.get_ele_attr_map(catr_ref.as_str()) {
                    let v = get_attr_value_f64_vec(&cata, "PARA").unwrap_or_default();
                    if v.len() >= 2 { return v[1] as f32; }
                }
            }
        }
        0.0
    }

    pub fn get_tubi_bore_by_stu(&mut self, stu_ref: &str) -> f32 {
        if let Some(stu) = self.get_ele_attr_map(stu_ref) {
            let catr_ref = stu.get_as_string("CATR").unwrap_or_default();
            if let Some(cata) = self.get_ele_attr_map(catr_ref.as_str()) {
                let v = get_attr_value_f64_vec(&cata, "PARA").unwrap_or_default();
                if v.len() >= 2 { return v[1] as f32; }
            }
        }
        0.0
    }

    /// 获取某个db的大版本
    pub async fn get_db_version(&mut self,db_no:i32) -> MResult<Option<u32>> {
        if let Some(conn)=self.connect().await {
            let db=conn.database("PDMSDBInfos");
            let t=db.collection::<PDMSDBInfo>("PDMSDBInfos");
            if let Some(r) = t.find_one(doc! {"db_no":db_no},None).await? {
                return Ok(Some(r.version))
            }
        }
        return Ok(None)
    }

}

#[test]
fn get_ele_attr_map_test() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    if let Some(v) = interface.get_ele_attr_map("15392/2") {
        dbg!(v);
    }
}

#[test]
pub fn test_get_cata_geoms() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    dbg!(interface.get_cata_ele_geoms("15192/43621"));
}

#[test]
pub fn test_get_des_geoms() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let geoms = interface.get_des_ele_geoms("23584/5850");
    dbg!(geoms);
    // let mat = interface.get_des_matrix("23584/5457");
    // dbg!(mat);
}


#[test]
fn test_get_children() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let children = interface.get_children("16476/8");
    for child in children {
        dbg!(&child.ref_no);
    }
}

#[test]
fn get_world_test() {
    let mut  interface = PdmsInterface::new("mongodb://localhost:27017");
    let world=interface.get_world("SAMPLE_IMPDESI").unwrap();
    println!("world.refno={}",world.ref_no);
}

#[tokio::test]
async fn get_attr_in_db_test() -> MResult<()> {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let result = interface.get_db_info_of_ele("15192/222818").await?;
    dbg!(&result);
    Ok(())
}
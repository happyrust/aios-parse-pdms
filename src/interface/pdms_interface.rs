
use mongodb::Client;
use mongodb::bson::doc;
use std::collections::HashSet;
use std::error::Error;
use dashmap::DashMap;
use crate::query_scom::{query_descomp_info, query_scom_info, resolve_cata_comp_attrs};
use crate::pdms_parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, AttrVal, EleDataNode, ElementData, PdmsRefno};
use futures::stream::TryStreamExt;
use mongodb::options::{FindOneOptions, FindOptions};

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

    ///获得ele对应的几何体
    pub async fn get_cata_ele_geoms_async(&mut self, refno: &str) -> MResult<Option<GeomsInfo>> {
        if let Some(client) = self.connect().await {
            if let Some(scom_info) = query_scom_info(refno, self).await?{
                let scom = resolve_cata_comp_attrs(&scom_info, self).await?;
                return Ok(Some(scom));
            }

        }
        Ok(None)
    }

    pub fn get_ele_geoms(&mut self, refno: &str) -> Option<GeomsInfo> {
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

    pub fn get_ele_attr_map_by_node(&mut self, ele: &EleDataNode) -> Option<AttrMap> {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.get_ele_attr_map_by_node_async(ele)).unwrap()
    }

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
                let mut cursor = tree_collect.find(doc! {"owner" : refno}, None).await?;
                while let Some(c) = cursor.try_next().await? {
                    v.push(c);
                }
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

}

// #[test]
pub fn test_get_ele_geoms() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    dbg!(interface.get_ele_geoms("15192/43621"));
}
//
// #[test]
// fn test_get_children() {
//     dbg!(PdmsInterface::get_children("15192/72762"));
// }
//
#[test]
fn test_get_children() {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let w = interface.get_world("as7000_0001");
    let children = interface.get_children_by_node(w.as_ref().unwrap());
    let children = interface.get_ele_attr_map("15192/53758");
    dbg!(&children);
    // let attr_map = interface.get_ele_attr_map_by_node(w.as_ref().unwrap());
    // dbg!(attr_map);
}

#[tokio::test]
async fn get_attr_in_db_test() -> MResult<()> {
    let mut interface = PdmsInterface::new("mongodb://localhost:27017");
    let result = interface.get_db_info_of_ele("15192/222818").await?;
    dbg!(&result);
    Ok(())
}

//
// #[tokio::test]
// async fn get_attr_in_db_test() -> core::result::Result<()> {
//     let result = PdmsInterface::get_ele_geoms_async("15192/222818").await?;
//     Ok(())
// }
//
// #[tokio::test]
// async fn get_children_test() -> core::result::Result<()> {
//     let result = PdmsInterface::get_children_async("15192/222795").await?;
//     println!("result={:?}", result);
//     Ok(())
// }
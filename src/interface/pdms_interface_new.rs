use dashmap::DashMap;
use futures::{StreamExt, TryStreamExt};
use id_tree::{NodeId, Tree};
use mongodb::Client;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, FindOneOptions};
use smol_str::SmolStr;
use crate::parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, EleNode, EleNodeMongoDb, PdmsMongoAttr, RefnoInfo};

pub type MResult<T> = mongodb::error::Result<T>;


#[derive(Debug, Default)]
pub struct PdmsMongoService {
    connection_str: String,
    client: Option<Client>,
    project: String,
}

impl PdmsMongoService {
    pub fn new(url: &str, project: &str) -> Self {
        // let client_uri = "mongodb://localhost:27017".to_string();
        Self {
            connection_str: url.to_string(),
            client: None,
            project: project.to_string(),
        }
    }

    #[inline]
    pub async fn connect(&mut self) -> Option<Client> {
        if self.client.is_none() {
            if let Ok(client) = Client::with_uri_str(&self.connection_str).await {
                self.client = Some(client);
            } else {
                return None;
            }
        }
        self.client.clone()
    }

    pub async fn get_tree(&mut self,file_name:SmolStr) -> MResult<Option<Tree<EleNode>>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&self.project);
            let t = db.collection::<EleNodeMongoDb>("PdmsTree");
            let file_name = file_name.as_str();
            if let Some(t) = t.find_one(doc! {"file_name":file_name},None).await? {
                return Ok(Some(bincode::deserialize(&t.tree).unwrap()));
            }
        }
        Ok(None)
    }

    pub async fn get_node_id (&mut self,file_name:SmolStr,refno:SmolStr) -> MResult<Option<NodeId>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&self.project);
            let t = db.collection::<RefnoInfo>("PdmsNodeId");
            let file_name = file_name.as_str();
            let refno = refno.as_str();
            // if let Some(t) = t.find_one(doc! {"file_name":file_name,"refno":refno},None).await? {
            //     return Ok(Some(t.node_id));
            // }
        }
        Ok(None)
    }

    pub async fn get_children_attr_map(&mut self,file_name:SmolStr,refno:SmolStr) -> MResult<Option<Vec<PdmsMongoAttr>>> {
        // if let Some(tree) = self.get_tree(file_name.clone()).await? {
        //     if let Some(node_id) = self.get_node_id(file_name.clone(),refno).await? {
        //         let children = tree.children(&node_id).unwrap();
        //         let mut r = vec![];
        //         for c in children {
        //             let child = c.data();
        //             if let Some(attr) = self.get_ele_attr_map_async(child.refno.to_refno_str()).await? {
        //                 r.push(attr);
        //             }
        //         }
        //         return Ok(Some(r))
        //     }
        // }
        Ok(None)
    }

    pub async fn get_ele_attr_map_async(&mut self, refno: SmolStr) -> MResult<Option<PdmsMongoAttr>> {
        // if let Some(client) = self.connect().await {
        //     let db = client.database(&self.project);
        //     let t = db.collection::<PdmsMongoAttr>("PdmsAttrs");
        //     let refno = refno.as_str();
        //     if let Some(m) = t.find_one(doc! {"refno":refno }, None).await? {
        //         return Ok(Some(m));
        //     }
        // }
        Ok(None)
    }



}

#[tokio::test]
async fn get_ele_attr_map_async() -> MResult<()> {
    let mut interface = PdmsMongoService::new("mongodb://localhost:27017", "apsProject");
    let node = interface.get_ele_attr_map_async(SmolStr::new("24575/4")).await?;
    dbg!(node);
    Ok(())
}

#[tokio::test]
async fn get_node_id_test() -> MResult<()> {
    let mut interface = PdmsMongoService::new("mongodb://localhost:27017", "abaProject");
    let node = interface.get_node_id(SmolStr::new("aba0092_0001"),SmolStr::new("8284/0")).await?;
    dbg!(node);
    Ok(())
}


#[tokio::test]
async fn get_children_map_test() ->MResult<()>{
    let mut interface = PdmsMongoService::new("mongodb://localhost:27017", "abaProject");
    if let Some(children)=interface.get_children_attr_map(SmolStr::new("aba0092_0001"),SmolStr::new("16476/3049")).await?{
        dbg!(&children);
    }
    Ok(())
}
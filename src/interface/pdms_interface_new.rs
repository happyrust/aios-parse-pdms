use dashmap::DashMap;
use futures::{StreamExt, TryStreamExt};
use mongodb::Client;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, FindOneOptions };
use smol_str::SmolStr;
use crate::parsed_data::GeomsInfo;
use crate::pdms_types::{AttrMap, PdmsMongoAttr };

pub type MResult<T> = mongodb::error::Result<T>;

#[derive(Debug, Default)]
pub struct PdmsInterface {
    connection_str: String,
    client: Option<Client>,
    project: String,
}

impl PdmsInterface {
    pub fn new(url: &str,project: &str) -> Self{
        // let client_uri = "mongodb://localhost:27017".to_string();
        Self{
            connection_str: url.to_string(),
            client: None,
            project: project.to_string()
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

    // pub async fn get_children_async(&mut self,refno:SmolStr) -> MResult<Option<Vec<SmolStr>>> {
    //
    // }

    pub async fn get_ele_attr_map_async(&mut self,refno:SmolStr) -> MResult<Option<AttrMap>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&self.project);
            let t = db.collection::<PdmsMongoAttr>("PdmsAttrs");
            let refno = refno.as_str();
            if let Some(m)=t.find_one(doc! {"refno":refno },None).await?{
                return Ok(Some(m.attr))
            }
        }
        Ok(None)
    }


}

#[tokio::test]
async fn get_ele_attr_map_async() -> MResult<()>{
    let mut interface=PdmsInterface::new("mongodb://localhost:27017","apsProject");
    let node=interface.get_ele_attr_map_async(SmolStr::new("24575/4")).await?;
    dbg!(node);
    Ok(())
}
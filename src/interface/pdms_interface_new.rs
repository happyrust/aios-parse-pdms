use dashmap::DashMap;
use futures::{StreamExt, TryStreamExt};
use mongodb::Client;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, FindOneOptions };
use smol_str::SmolStr;
use crate::parsed_data::GeomsInfo;
use crate::pdms_types::AttrMap;

type MResult<T> = mongodb::error::Result<T>;

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

    pub async fn get_ele_attr_map_async(&mut self,refno:&str) -> MResult<Option<AttrMap>> {
        if let Some(client) = self.connect().await {
            let db = client.database(&self.project);
            let t = db.collection::< DashMap<SmolStr, AttrMap> >("PdmsAttrs");
            let mut opt= FindOptions::builder().projection(doc! { refno:1 }).build();
            if let Some(d) = t.find(None,opt).await?.try_next().await?{
                if let Some(v) = d.get(refno) {
                    return Ok(Some(v.value()).cloned())
                }
            }
        }
        Ok(None)
    }


}

#[tokio::test]
async fn get_ele_attr_map_async() -> MResult<()>{
    let mut interface=PdmsInterface::new("mongodb://localhost:27017","apsProject");
    let node=interface.get_ele_attr_map_async("24575/4").await?;
    dbg!(node);
    Ok(())
}
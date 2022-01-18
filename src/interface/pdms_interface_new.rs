use dashmap::DashMap;
use mongodb::Client;
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

        }
        Ok(None)
    }

    pub async fn get_cata_ele_geoms_async(&mut self,refno:&str) -> Option<GeomsInfo> {

    }
}


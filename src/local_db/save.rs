use std::error::Error;
use std::fs;
use std::path::Path;
use bonsaidb::core::circulate::Message;
use bonsaidb::core::schema::{Collection, CollectionName, Schematic, SerializedCollection};
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
use smol_str::SmolStr;
use crate::{AttrMap, parse_pdms_dir};
use crate::parse::PdmsDbData;
use crate::pdms_types::RefU64Vec;

#[derive(Default, Debug)]
pub struct PdmsConfig{
    pub data_dir: String,   //pdms的数据文件夹
    pub project_name: String,
    pub all_projects: Vec<String>,
    pub mdb_name: String,
}

// impl Collection for Vec<SmolStr> {
//     fn collection_name() -> CollectionName {
//         CollectionName::new("aios", "attrs")
//     }
//
//     fn define_views(schema: &mut Schematic) -> Result<(), Error> {
//         Ok(())
//     }
// }

// impl DefaultSerialization for AttrMap {}

pub async fn save_local() -> Result<(), bonsaidb::core::Error> {
    let pdms_config = PdmsConfig {
        // dir: "D:/AVEVA/Projects/E3D2.1/AvevaPlantSample/aps000".to_string(),
        // project_name: "aps000".to_string()
        data_dir: "C:/AVEVA/Plant/Projects12.1.SP4".to_string(),    //sam7200_0001
        project_name: "SAM".to_string(),
        all_projects: vec!["Sample".to_string()/*, "Master".to_string()*/],   //配置所有需要读取的project
        mdb_name: "SAMPLE".to_string(),
    };


    let mut data_dir = Path::new(&pdms_config.data_dir);
    for project in &pdms_config.all_projects {
        let project_dir = data_dir.join(&project);
        let mut target_dir = fs::read_dir(project_dir).unwrap().into_iter().map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        }).find(|x| x.file_name().unwrap().to_str().unwrap().ends_with("000")).unwrap();
        dbg!(&target_dir);
        let attr_db = Database::open::<AttrMap>(StorageConfiguration::new(format!("./{project}/attrs.db"))).await?;
        if let Ok(mut r) = parse_pdms_dir(target_dir.as_os_str().to_str().unwrap(), None) {
            fs::create_dir_all(project).unwrap();
            for (k, PdmsDbData{
                all_attr_map,
                ele_id_tree,
                type_ele_map,
                db_name,
                db_no,
                ..
            }) in r {
                dbg!(all_attr_map.len());
                let mut file_name = &k;
                dbg!(&db_name);
                let refs_db = Database::open::<RefU64Vec>(StorageConfiguration::new(format!("./{project}/{db_no}_refs.db"))).await?;
                //属性全部插入
                for (refno, v) in all_attr_map {
                    // dbg!(&v);
                    v.insert_into(refno.0, &attr_db).await?;
                }

                for (refno, v) in type_ele_map {
                    v.insert_into(refno as u64, &refs_db).await?;
                }

                //
            }
        }
    }

    Ok(())
}
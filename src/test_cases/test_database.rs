use std::time::Instant;
use crate::local_db::bonsaidb_local::{AiosDBManager, DbOption};
use crate::pdms_types::{CachedMeshes, RefU64};

// #[test]
// #[tokio::test]
pub async fn test_column(){

    let path = "../Projects";
    // /Volumes/[C] Windows 11/AVEVA/Plant/Projects12.1.SP4
    // #[cfg(target_arch = "arch64")]
    //     let path = "C:/AVEVA/Plant/Projects12.1.SP4";

    let mut db_manager = AiosDBManager::init(path,
                                             vec!["Sample".to_string(), "Master".to_string()],
                                             "Sample",
                                             Some(DbOption {
                                                 total_sync: false,
                                                 incr_sync: false,
                                             })).await.unwrap();
    // let mut db = db_manager.db_map.get_mut("Sample").unwrap();
    // let result = db_manager.cache_geos_data(7200).await.unwrap();
    let mut  cache_mgr = CachedMeshes::default();
    let mut time = Instant::now();

    let refno = RefU64::from_two_nums(23584, 5645);
    let geoms = db_manager.get_design_geoms(&refno, &mut cache_mgr).await;

    // let refno = RefU64::from_two_nums(23584, 6370);
    let refno = RefU64::from_two_nums(23584, 6370);
    // let refno = RefU64::from_two_nums(15192, 113114);

    //cached 一些常用的取值操作
    let refno_info = db_manager.get_refno_info(&refno).await.unwrap().unwrap();
    dbg!(&refno_info);

    let attr = db_manager.get_attr(&refno).await.unwrap().unwrap();
    if let Some(spre) = attr.get_foreign_refno("SPRE"){
        let geom = db_manager.get_sprf_geom(&spre).await.unwrap();

        dbg!(&geom);
    }


    //
    // dbg!(refno_info);
    // dbg!(db_manager.get_project_of_refno(&refno).await);
    dbg!(db_manager.get_pretty_attr(&refno).await);
    dbg!(db_manager.get_dehashed_attr(&refno).await);
    dbg!(db_manager.get_world_transform(&refno).await);
    dbg!(db_manager.get_children(&refno).await);

    dbg!(time.elapsed().as_millis());

}
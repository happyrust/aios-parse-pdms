use glam::TransformSRT;
use crate::shape::pdms_shape::BrepShapeTrait;

// mod test_file;
// pub mod sled_local;
#[cfg(not(target_arch = "wasm32"))]
pub mod sled_manager;

#[cfg(not(target_arch = "wasm32"))]
pub mod tikv_manager;

// pub mod bonsaidb_server;
pub mod helper;
pub mod consts;

#[cfg(not(target_arch = "wasm32"))]
pub mod string_database;
#[cfg(not(target_arch = "wasm32"))]
pub mod refno_info_database;
use clap::Parser;

#[derive(Debug, Default, Clone, Parser)]
pub struct DbOption {
    #[clap(long)]
    pub total_sync: bool,
    #[clap(long)]
    pub incr_sync: bool,
    #[clap(long, default_value = "12.1SP4Projects")]
    pub project_path: String,
    //#[clap(long, default_value = "MASTER", "SAMPLE")]
    pub included_projects: Vec<String>,
    #[clap(skip)]
    pub included_db_files: Option<Vec<String>>,  //if none all files parsed, if not, only included parsed
    #[clap(long)]
    pub mdb_name: String,
    #[clap(long)]
    pub project_name: String,
    #[clap(short)]
    pub main_db_code: u32,
}

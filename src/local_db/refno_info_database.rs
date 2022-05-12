use std::ops::{Deref, DerefMut};
use aios_core::pdms_types::{RefnoInfo, RefU64};
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
// use crate::pdms_types::{RefnoInfo, RefU64};

#[derive(Debug, Clone)]
pub struct RefInoDatabase{
    pub db: Database,
}

impl Deref for RefInoDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl DerefMut for RefInoDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl RefInoDatabase {
    pub fn init(path: &str) -> Self{
        Self{
            db: Database::open::<RefnoInfo>(StorageConfiguration::new(path)).expect("path not correct"),
        }
    }
    ///获得refno的project 名称
    #[inline]
    pub fn get_refno_info(&self, refno: RefU64) -> Result<Option<RefnoInfo>, bonsaidb::core::Error> {
        Ok(RefnoInfo::get(refno.get_0(), &self.db)?.map(|x| x.contents))
    }
}
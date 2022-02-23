use std::ops::{Deref, DerefMut};
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::local::config::{Builder, StorageConfiguration};
use bonsaidb::local::Database;
use crate::pdms_types::{RefnoInfo, RefU64};

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
    pub async fn init(path: &str) -> Self{
        Self{
            db: Database::open::<RefnoInfo>(StorageConfiguration::new(path)).await.expect("path not correct"),
        }
    }
    ///获得refno的project 名称
    #[inline]
    pub async fn get_refno_info(&self, refno: &RefU64) -> Result<Option<RefnoInfo>, bonsaidb::core::Error> {
        // self.get_refno_info_by_hash(refno.get_u32_hash())
        Ok(RefnoInfo::get(refno.get_0(), &self.db).await?.map(|x| x.contents))
    }
}
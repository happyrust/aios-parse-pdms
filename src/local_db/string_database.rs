use std::ops::{Deref, DerefMut};
use bonsaidb::core::schema::SerializedCollection;
use bonsaidb::local::config::{Builder, Compression, StorageConfiguration};
use bonsaidb::local::Database;
use crate::pdms_types::{AiosStr, AiosStrHash, RefnoInfo, RefU64};

#[derive(Debug, Clone)]
pub struct StringDatabase {
    pub db: Database,
}

impl Deref for StringDatabase {
    type Target = Database;

    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl DerefMut for StringDatabase {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl StringDatabase {
    pub async fn init(path: &str) -> Self {
        Self {
            db: if cfg!(feature = "compression") {
                Database::open::<AiosStr>(StorageConfiguration::new(path)
                    .default_compression(Compression::Lz4)
                ).await.expect("path not correct")
            } else {
                Database::open::<AiosStr>(StorageConfiguration::new(path)
                ).await.expect("path not correct")
            }
        }
    }
    ///获得refno的project 名称
    #[inline]
    pub async fn get_string(&self, hash: AiosStrHash) -> Result<Option<AiosStr>, bonsaidb::core::Error> {
        // self.get_refno_info_by_hash(refno.get_u32_hash())
        Ok(AiosStr::get(hash, &self.db).await?.map(|x| x.contents))
    }
}

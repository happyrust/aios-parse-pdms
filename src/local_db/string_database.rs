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
    ///获得refno的project 名称
    #[inline]
    pub fn get_string(&self, hash: AiosStrHash) -> Result<Option<AiosStr>, bonsaidb::core::Error> {
        Ok(AiosStr::get(hash, &self.db)?.map(|x| x.contents))
    }
}

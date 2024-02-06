use std::sync::{Arc, RwLock};

use super::rocksdb::RocksDBStorage;

pub struct ClusterStorage {
    rds: Arc<RwLock<RocksDBStorage>>,
}

impl ClusterStorage {
    pub fn new(rds: Arc<RwLock<RocksDBStorage>>) -> Self {
        ClusterStorage { rds }
    }

    pub fn save_broker_info(&self) {}

    pub fn get_broker_info(&self) {}

    pub fn set(&self, key: String, value: Vec<u8>) {
        let rds = self.rds.write().unwrap();
        rds.write(rds.cf_cluster(), &key, &value).unwrap();
    }

    pub fn get(&self, key: String) -> Option<Vec<u8>> {
        let rds = self.rds.read().unwrap();
        match rds.read::<Vec<u8>>(rds.cf_cluster(), &key) {
            Ok(data) => {
                return data;
            }
            Err(e) => {
                return None;
            }
        }
    }

    pub fn delete(&self, key: String) {
        let rds = self.rds.write().unwrap();
        rds.delete(rds.cf_cluster(), &key).unwrap();
    }

    pub fn exists(&self, key: String) -> bool {
        let rds = self.rds.read().unwrap();
        rds.exist(rds.cf_cluster(), &key)
    }
}
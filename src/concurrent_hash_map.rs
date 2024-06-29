use std::{collections::HashMap, sync::Arc};

use shrinkwraprs::Shrinkwrap;
use tokio::sync::{Mutex, RwLock};

#[derive(Shrinkwrap)]
pub struct ConcurrentHashMap<K, V>(RwLock<HashMap<K, Arc<Mutex<V>>>>);

impl<K, V> ConcurrentHashMap<K, V>
where
    K: std::hash::Hash + PartialEq + Eq,
{
    pub fn new() -> Self {
        Self(RwLock::new(HashMap::new()))
    }

    pub async fn get(&self, key: &K) -> Option<Arc<Mutex<V>>> {
        let lock = self.read().await;
        lock.get(key).cloned()
    }
}

impl<K, V> ConcurrentHashMap<K, V>
where
    K: std::hash::Hash + PartialEq + Eq + Clone,
    V: Default,
{
    pub async fn get_or_default(&self, key: &K) -> Arc<Mutex<V>> {
        match self.get(key).await {
            Some(state) => state,
            None => {
                let value = Arc::new(Mutex::new(V::default()));
                let mut lock = self.write().await;
                lock.insert(key.clone(), value.clone());
                value
            }
        }
    }
}

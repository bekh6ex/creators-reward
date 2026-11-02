use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

pub use bytes::Bytes;
pub use uuid::{Bytes as UuidBytes, Uuid};

pub use ids::*;

pub mod billing_usage;
pub mod ids;
pub mod viewer_registration;

#[derive(Clone)]
pub struct StupidAsyncMemoryMap<K, V>(Arc<futures::lock::Mutex<HashMap<K, V>>>);

impl<K: Hash + Eq, V: Clone> StupidAsyncMemoryMap<K, V> {
    pub fn new() -> Self {
        Self(Arc::new(futures::lock::Mutex::new(HashMap::new())))
    }

    pub async fn get(&self, k: &K) -> Option<V> {
        self.0.lock().await.get(k).cloned()
    }

    pub async fn insert(&self, k: K, v: V) {
        self.0.lock().await.insert(k, v);
    }
}

impl<K: Hash + Eq, V: Clone> Default for StupidAsyncMemoryMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

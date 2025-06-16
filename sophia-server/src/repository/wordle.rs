use std::sync::Arc;
use async_trait::async_trait;
use clap::Error;
use tokio::sync::RwLock;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use rand::{rngs::StdRng, Rng};
use rand::SeedableRng;
use std::time::{SystemTime, UNIX_EPOCH};
use sophia_core::errors::Result;
use crate::service::WordleRepo;


#[derive(Debug, Deserialize)]
struct Words {
    words: Vec<String>
}

pub struct WordleMemoryImpl {
    words: Arc<RwLock<Vec<String>>>,
    length: usize,
    cache: Arc<RwLock<String>>,
}

impl WordleMemoryImpl {
    pub fn new() -> Self {
        let file = File::open("wordle.json").expect("Failed to open words.json");
        let reader = BufReader::new(file);
        let words_data: Words = serde_json::from_reader(reader).expect("Failed to parse words.json");
        let length = words_data.words.len();
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let seed: u64 = since_the_epoch.as_secs();
        let mut rng = StdRng::seed_from_u64(seed);
        let random_index: usize = rng.gen_range(0..length);
        let cache = words_data.words[random_index].clone();
        Self {
            words: Arc::new(RwLock::new(words_data.words)),
            length,
            cache: Arc::new(RwLock::new(cache)),
        }
    }
}

#[async_trait]
impl WordleRepo for WordleMemoryImpl {
    async fn get(&self) -> Result<String>  {
        let cache = self.cache.read().await;
        Ok(cache.clone())
    }

    async fn end(&self) -> Result<()>  {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let seed: u64 = since_the_epoch.as_secs();
        let mut rng = StdRng::seed_from_u64(seed);
        let random_index: usize = rng.gen_range(0..self.length);
        let words = self.words.read().await;
        let mut cache = self.cache.write().await;
        *cache = words[random_index].clone();
        Ok(())
    }
}




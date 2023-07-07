use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct Token {
    lemma: String,
    descr: HashMap<String, String>,
    metadata: HashMap<String, String>,
}

#[derive(Deserialize, Serialize)]
pub struct Sample {
    id: String,
    year: u16,
    descr: HashMap<String, String>,
    metadata: HashMap<String, String>,
    words: u64,
    tokens: Vec<Token>,
}

#[derive(Deserialize, Serialize)]
pub struct Input {
    samples: Vec<Sample>,
}

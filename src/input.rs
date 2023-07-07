use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct IToken {
    pub lemma: String,
    pub descr: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Deserialize, Serialize)]
pub struct ISample {
    pub id: String,
    pub year: u16,
    pub descr: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub words: u64,
    pub tokens: Vec<IToken>,
}

#[derive(Deserialize, Serialize)]
pub struct Input {
    pub samples: Vec<ISample>,
}

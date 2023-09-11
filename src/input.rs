use serde::Deserialize;
use std::collections::HashMap;

pub type Year = i16;

#[derive(Deserialize)]
pub struct IToken {
    pub lemma: String,
    pub descr: Option<HashMap<String, String>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct ISample {
    pub id: String,
    pub year: Year,
    pub descr: Option<HashMap<String, String>>,
    pub metadata: HashMap<String, String>,
    pub words: u64,
    pub tokens: Vec<IToken>,
}

#[derive(Deserialize)]
pub struct Input {
    pub samples: Vec<ISample>,
}

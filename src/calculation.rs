use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SToken {
    pub count: u64,
    pub id: usize,
}

#[derive(Deserialize, Serialize)]
pub struct Sample {
    pub words: u64,
    pub tokens: Vec<SToken>,
}

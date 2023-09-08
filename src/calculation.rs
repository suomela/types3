#[derive(PartialEq, Eq, Debug)]
pub struct SToken {
    pub count: u64,
    pub id: usize,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Sample {
    pub x: u64,
    pub token_count: u64,
    pub tokens: Vec<SToken>,
}

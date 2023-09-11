#[derive(PartialEq, Eq, Debug)]
pub struct SToken {
    pub id: usize,
    pub count: u64,
    pub marked_count: u64,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Sample {
    pub x: u64,
    pub token_count: u64,
    pub tokens: Vec<SToken>,
}

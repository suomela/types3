pub struct SToken {
    pub count: u64,
    pub id: usize,
}

pub struct Sample {
    pub x: u64,
    pub tokens: Vec<SToken>,
}

pub struct SToken {
    pub count: u64,
    pub id: usize,
}

pub struct Sample {
    pub size: u64,
    pub tokens: Vec<SToken>,
}

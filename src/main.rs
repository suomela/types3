use types3::*;

fn sample(words: u64, tokens: Vec<SToken>) -> Sample {
    Sample { words, tokens }
}

fn st(count: u64, id: usize) -> SToken {
    SToken { count, id }
}

fn main() {
    let n = 10;
    let mut samples = Vec::new();
    for i in 0..n {
        samples.push(sample(1, vec![st(1, i as usize)]));
    }
    let ds = Dataset::new(samples);
    ds.count_exact();
}

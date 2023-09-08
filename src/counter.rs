use crate::calculation::{SToken, Sample};

pub trait Counter {
    fn new(total_types: usize) -> Self;
    fn get_x(&self) -> u64;
    fn get_y(&self) -> u64;
    fn reset(&mut self);
    fn feed_sample(&mut self, sample: &Sample);
}

pub struct TypeCounter {
    x: u64,
    types: u64,
    seen: Vec<bool>,
}

impl TypeCounter {
    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
    }
}

impl Counter for TypeCounter {
    fn get_x(&self) -> u64 {
        self.x
    }

    fn get_y(&self) -> u64 {
        self.types
    }

    fn new(total_types: usize) -> TypeCounter {
        TypeCounter {
            x: 0,
            types: 0,
            seen: vec![false; total_types],
        }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.types = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
    }

    fn feed_sample(&mut self, sample: &Sample) {
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.x += sample.x;
    }
}

pub fn count_types(samples: &[Sample]) -> usize {
    let mut max_type = 0;
    for sample in samples {
        for token in &sample.tokens {
            max_type = max_type.max(token.id);
        }
    }
    max_type + 1
}

pub struct TokenCounter {
    x: u64,
    tokens: u64,
}

impl Counter for TokenCounter {
    fn get_x(&self) -> u64 {
        self.x
    }

    fn get_y(&self) -> u64 {
        self.tokens
    }

    fn new(_total_types: usize) -> TokenCounter {
        TokenCounter { x: 0, tokens: 0 }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.tokens = 0;
    }

    fn feed_sample(&mut self, sample: &Sample) {
        self.x += sample.x;
        self.tokens += sample.tokens.len() as u64;
    }
}

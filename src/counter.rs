use crate::calculation::{SToken, Sample};

pub trait Counter {
    fn new(total_types: usize) -> Self;
    fn get_x(&self) -> u64;
    fn get_y(&self) -> u64;
    fn reset(&mut self);
    fn feed_sample(&mut self, sample: &Sample);
}

pub struct TypeCounter {
    size: u64,
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
        self.size
    }

    fn get_y(&self) -> u64 {
        self.types
    }

    fn new(total_types: usize) -> TypeCounter {
        TypeCounter {
            size: 0,
            types: 0,
            seen: vec![false; total_types],
        }
    }

    fn reset(&mut self) {
        self.size = 0;
        self.types = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
    }

    fn feed_sample(&mut self, sample: &Sample) {
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.size += sample.size;
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

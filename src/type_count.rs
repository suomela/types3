use crate::calculation::{SToken, Sample};

pub struct TypeCounter {
    pub size: u64,
    pub types: u64,
    seen: Vec<bool>,
}

impl TypeCounter {
    pub fn new(total_types: usize) -> TypeCounter {
        TypeCounter {
            size: 0,
            types: 0,
            seen: vec![false; total_types],
        }
    }

    pub fn reset(&mut self) {
        self.size = 0;
        self.types = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
    }

    pub fn feed_sample(&mut self, sample: &Sample) {
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.size += sample.size;
    }

    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
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

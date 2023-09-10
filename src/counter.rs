use crate::calculation::{SToken, Sample};

pub struct CounterState {
    pub x: u64,
    pub y: u64,
    pub low_y: u64,
    pub high_y: u64,
}

pub trait Counter {
    fn new(total_types: usize) -> Self;
    fn reset(&mut self);
    fn feed_sample(&mut self, sample: &Sample) -> CounterState;
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

    fn feed_sample(&mut self, sample: &Sample) -> CounterState {
        let prev_types = self.types;
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.x += sample.x;
        CounterState {
            x: self.x,
            y: self.types,
            low_y: prev_types,
            high_y: self.types,
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

pub struct TokenCounter {
    x: u64,
    tokens: u64,
}

impl Counter for TokenCounter {
    fn new(_total_types: usize) -> TokenCounter {
        TokenCounter { x: 0, tokens: 0 }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.tokens = 0;
    }

    fn feed_sample(&mut self, sample: &Sample) -> CounterState {
        let prev_tokens = self.tokens;
        self.x += sample.x;
        self.tokens += sample.token_count;
        CounterState {
            x: self.x,
            y: self.tokens,
            low_y: prev_tokens,
            high_y: self.tokens,
        }
    }
}

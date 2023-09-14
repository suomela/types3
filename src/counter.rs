use crate::{
    calculation::{SToken, Sample},
    output::MeasureY,
};

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

pub struct HapaxCounter {
    x: u64,
    hapaxes: u64,
    gain_hapax: u64,
    lose_hapax: u64,
    seen: Vec<u8>,
}

impl HapaxCounter {
    fn feed_token(&mut self, t: &SToken) {
        if t.count == 1 {
            if self.seen[t.id] == 0 {
                self.gain_hapax += 1;
                self.seen[t.id] = 1;
            } else if self.seen[t.id] == 1 {
                self.lose_hapax += 1;
                self.seen[t.id] = 2;
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if self.seen[t.id] == 0 {
                self.gain_hapax += 1;
                self.lose_hapax += 1;
                self.seen[t.id] = 2;
            } else if self.seen[t.id] == 1 {
                self.lose_hapax += 1;
                self.seen[t.id] = 2;
            }
        }
    }
}

impl Counter for HapaxCounter {
    fn new(total_types: usize) -> HapaxCounter {
        HapaxCounter {
            x: 0,
            hapaxes: 0,
            gain_hapax: 0,
            lose_hapax: 0,
            seen: vec![0; total_types],
        }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.hapaxes = 0;
        self.gain_hapax = 0;
        self.lose_hapax = 0;
        for e in self.seen.iter_mut() {
            *e = 0;
        }
    }

    fn feed_sample(&mut self, sample: &Sample) -> CounterState {
        self.gain_hapax = 0;
        self.lose_hapax = 0;
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.x += sample.x;
        let prev_y = self.hapaxes;
        self.hapaxes += self.gain_hapax;
        self.hapaxes -= self.lose_hapax;
        let cur_y = self.hapaxes;
        let low_y = prev_y.saturating_sub(self.lose_hapax);
        let high_y = prev_y + self.gain_hapax;
        debug_assert!(low_y <= prev_y);
        debug_assert!(low_y <= cur_y);
        debug_assert!(prev_y <= high_y);
        debug_assert!(cur_y <= high_y);
        CounterState {
            x: self.x,
            y: cur_y,
            low_y,
            high_y,
        }
    }
}

pub struct TypeRatioCounter {
    types: u64,
    types_marked: u64,
    seen: Vec<bool>,
    seen_marked: Vec<bool>,
}

impl TypeRatioCounter {
    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
        if t.marked_count > 0 && !self.seen_marked[t.id] {
            self.types_marked += 1;
            self.seen_marked[t.id] = true;
        }
    }
}

impl Counter for TypeRatioCounter {
    fn new(total_types: usize) -> TypeRatioCounter {
        TypeRatioCounter {
            types: 0,
            types_marked: 0,
            seen: vec![false; total_types],
            seen_marked: vec![false; total_types],
        }
    }

    fn reset(&mut self) {
        self.types = 0;
        self.types_marked = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
        for e in self.seen_marked.iter_mut() {
            *e = false;
        }
    }

    fn feed_sample(&mut self, sample: &Sample) -> CounterState {
        let prev_types_marked = self.types_marked;
        for t in &sample.tokens {
            self.feed_token(t);
        }
        CounterState {
            x: self.types,
            y: self.types_marked,
            low_y: prev_types_marked,
            high_y: self.types_marked,
        }
    }
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

pub struct SampleCounter {
    x: u64,
    samples: u64,
}

impl Counter for SampleCounter {
    fn new(_total_types: usize) -> SampleCounter {
        SampleCounter { x: 0, samples: 0 }
    }

    fn reset(&mut self) {
        self.x = 0;
        self.samples = 0;
    }

    fn feed_sample(&mut self, sample: &Sample) -> CounterState {
        let prev_samples = self.samples;
        self.x += sample.x;
        self.samples += 1;
        CounterState {
            x: self.x,
            y: self.samples,
            low_y: prev_samples,
            high_y: self.samples,
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

pub fn count_xy(measure_y: MeasureY, samples: &[Sample]) -> (u64, u64) {
    match measure_y {
        MeasureY::Types => count_xy_variant::<TypeCounter>(samples),
        MeasureY::Tokens => count_xy_variant::<TokenCounter>(samples),
        MeasureY::Hapaxes => count_xy_variant::<HapaxCounter>(samples),
        MeasureY::Samples => count_xy_variant::<SampleCounter>(samples),
        MeasureY::MarkedTypes => count_xy_variant::<TypeRatioCounter>(samples),
    }
}

fn count_xy_variant<TCounter>(samples: &[Sample]) -> (u64, u64)
where
    TCounter: Counter,
{
    let n = count_types(samples);
    let mut counter = TCounter::new(n);
    let mut c = None;
    for s in samples {
        c = Some(counter.feed_sample(s));
    }
    match c {
        None => (0, 0),
        Some(c) => (c.x, c.y),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn stm(id: usize, count: u64, marked_count: u64) -> SToken {
        SToken {
            id,
            count,
            marked_count,
        }
    }

    #[test]
    fn count_xy_tokens() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 11,
                tokens: vec![stm(0, 10, 2), stm(1, 1, 0)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![stm(1, 5, 0)],
            },
        ];
        assert_eq!(count_xy(MeasureY::Tokens, &samples), (1234 + 5678, 16));
    }

    #[test]
    fn count_xy_types() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 11,
                tokens: vec![stm(0, 10, 2), stm(1, 1, 0)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![stm(1, 5, 0)],
            },
        ];
        assert_eq!(count_xy(MeasureY::Types, &samples), (1234 + 5678, 2));
    }

    #[test]
    fn count_xy_samples() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 11,
                tokens: vec![stm(0, 10, 2), stm(1, 1, 0)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![stm(1, 5, 0)],
            },
        ];
        assert_eq!(count_xy(MeasureY::Samples, &samples), (1234 + 5678, 2));
    }

    #[test]
    fn count_xy_hapaxes() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 11,
                tokens: vec![stm(0, 10, 2), stm(1, 1, 0)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![stm(1, 5, 0)],
            },
        ];
        assert_eq!(count_xy(MeasureY::Hapaxes, &samples), (1234 + 5678, 0));
    }

    #[test]
    fn count_xy_marked_types() {
        let samples = vec![
            Sample {
                x: 0,
                token_count: 11,
                tokens: vec![stm(0, 10, 2), stm(1, 1, 0)],
            },
            Sample {
                x: 0,
                token_count: 5,
                tokens: vec![stm(1, 5, 0)],
            },
        ];
        assert_eq!(count_xy(MeasureY::MarkedTypes, &samples), (2, 1));
    }
}

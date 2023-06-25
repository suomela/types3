use std::collections::HashMap;

pub struct SToken {
    count: u64,
    id: usize,
}

pub struct Sample {
    words: u64,
    tokens: Vec<SToken>,
}

pub struct Dataset {
    samples: Vec<Sample>,
    total_words: u64,
    total_tokens: u64,
    total_types: usize,
}

impl Dataset {
    pub fn new(samples: Vec<Sample>) -> Dataset {
        let mut total_types = 0;
        let mut total_tokens = 0;
        let mut total_words = 0;
        for sample in &samples {
            total_words += sample.words;
            for stoken in &sample.tokens {
                total_tokens += stoken.count;
                total_types = total_types.max(1 + stoken.id);
            }
        }
        Dataset {
            samples,
            total_words,
            total_tokens,
            total_types,
        }
    }
}

pub struct Limits {
    lower: HashMap<(u64, u64), u64>,
    upper: HashMap<(u64, u64), u64>,
}

impl Limits {
    pub fn new() -> Limits {
        Limits {
            lower: HashMap::new(),
            upper: HashMap::new(),
        }
    }
}

pub struct Limitset {
    types_by_tokens: Limits,
    types_by_words: Limits,
    tokens_by_words: Limits,
    total: u64,
}

impl Limitset {
    pub fn new() -> Limitset {
        Limitset {
            types_by_tokens: Limits::new(),
            types_by_words: Limits::new(),
            tokens_by_words: Limits::new(),
            total: 0,
        }
    }
}

type Seen = Vec<bool>;

pub fn count_exact(ds: &Dataset, cs: &mut Limitset) {
    let n = ds.samples.len();
    let mut idx = vec![0; n];
    for (i, x) in idx.iter_mut().enumerate() {
        *x = i;
    }
    let mut seen = vec![false; n];
    count_exact_rec(ds, cs, &mut idx, 0, &mut seen);
}

fn count_exact_rec(ds: &Dataset, cs: &mut Limitset, idx: &mut [usize], i: usize, seen: &mut Seen) {
    let n = ds.samples.len();
    if i == n {
        update_counters(ds, cs, idx, seen);
    } else {
        for j in i..n {
            idx.swap(i, j);
            count_exact_rec(ds, cs, idx, i + 1, seen);
            idx.swap(i, j);
        }
    }
}

#[derive(Clone)]
struct Counter {
    types: u64,
    tokens: u64,
    words: u64,
}

fn update_counters(ds: &Dataset, cs: &mut Limitset, idx: &mut [usize], seen: &mut Seen) {
    for e in seen.iter_mut() {
        *e = false;
    }
    let mut c = Counter {
        types: 0,
        tokens: 0,
        words: 0,
    };
    for i in idx {
        let prev = c.clone();
        let sample = &ds.samples[*i];
        for t in &sample.tokens {
            if !seen[t.id] {
                c.types += 1;
                seen[t.id] = true;
            }
            c.tokens += t.count;
        }
        c.words += sample.words;
    }
    cs.total += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_binary() {
        let ds = Dataset::new(vec![
            Sample {
                words: 1,
                tokens: vec![SToken { count: 1, id: 0 }],
            },
            Sample {
                words: 1,
                tokens: vec![SToken { count: 1, id: 0 }],
            },
            Sample {
                words: 1,
                tokens: vec![SToken { count: 1, id: 1 }],
            },
        ]);
        let mut cs = Limitset::new();
        count_exact(&ds, &mut cs);
        assert_eq!(1 * 2 * 3, cs.total);
    }
}

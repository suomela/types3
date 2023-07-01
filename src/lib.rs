use core::ops::Range;

pub struct SToken {
    count: u64,
    id: usize,
}

pub struct Sample {
    words: u64,
    tokens: Vec<SToken>,
}

pub struct Dataset {
    pub samples: Vec<Sample>,
    pub total_words: u64,
    pub total_tokens: u64,
    pub total_types: u64,
}

type Seen = Vec<bool>;

#[derive(Clone)]
struct Counter {
    types: u64,
    tokens: u64,
    words: u64,
}

impl Dataset {
    pub fn new(samples: Vec<Sample>) -> Dataset {
        let mut total_types = 0;
        let mut total_tokens = 0;
        let mut total_words = 0;
        for sample in &samples {
            assert!(sample.tokens.len() as u64 <= sample.words);
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
            total_types: total_types as u64,
        }
    }

    pub fn count_exact(&self) -> Resultset {
        let mut rs = Resultset::new();
        self.count_exact_to(&mut rs);
        rs
    }

    pub fn count_exact_to(&self, rs: &mut Resultset) {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, x) in idx.iter_mut().enumerate() {
            *x = i;
        }
        let mut seen = vec![false; self.total_types as usize];
        self.count_exact_rec(rs, &mut idx, 0, &mut seen);
    }

    fn count_exact_rec(&self, rs: &mut Resultset, idx: &mut [usize], i: usize, seen: &mut Seen) {
        let n = self.samples.len();
        if i == n {
            self.update_counters(rs, idx, seen);
        } else {
            for j in i..n {
                idx.swap(i, j);
                self.count_exact_rec(rs, idx, i + 1, seen);
                idx.swap(i, j);
            }
        }
    }

    fn update_counters(&self, rs: &mut Resultset, idx: &mut [usize], seen: &mut Seen) {
        for e in seen.iter_mut() {
            *e = false;
        }
        let mut c = Counter {
            types: 0,
            tokens: 0,
            words: 0,
        };
        rs.tokens_by_words.add_start(c.tokens, c.words);
        rs.types_by_words.add_start(c.types, c.words);
        rs.types_by_tokens.add_start(c.types, c.tokens);
        for i in idx {
            let prev = c.clone();
            let sample = &self.samples[*i];
            for t in &sample.tokens {
                if !seen[t.id] {
                    c.types += 1;
                    seen[t.id] = true;
                }
                c.tokens += t.count;
            }
            c.words += sample.words;
            rs.tokens_by_words
                .add_box(prev.tokens..c.tokens, prev.words..c.words);
            rs.types_by_words
                .add_box(prev.types..c.types, prev.words..c.words);
            rs.types_by_tokens
                .add_box(prev.types..c.types, prev.tokens..c.tokens);
        }
        rs.tokens_by_words.add_end(c.tokens, c.words);
        rs.types_by_words.add_end(c.types, c.words);
        rs.types_by_tokens.add_end(c.types, c.tokens);

        rs.total += 1;
    }
}

pub struct Results {
    lower: density_curve::Grid,
    upper: density_curve::Grid,
}

impl Results {
    pub fn new() -> Results {
        Results {
            lower: density_curve::Grid::new(),
            upper: density_curve::Grid::new(),
        }
    }

    pub fn add_start(&mut self, y: u64, x: u64) {
        self.upper.add(y, x..x + 1, 1);
    }

    pub fn add_end(&mut self, y: u64, x: u64) {
        self.lower.add(y, x..x + 1, 1);
    }

    pub fn add_box(&mut self, yy: Range<u64>, xx: Range<u64>) {
        self.upper.add(yy.end, xx.start + 1..xx.end + 1, 1);
        self.lower.add(yy.start, xx, 1);
    }
}

impl Default for Results {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Resultset {
    types_by_tokens: Results,
    types_by_words: Results,
    tokens_by_words: Results,
    total: u64,
}

impl Resultset {
    pub fn new() -> Resultset {
        Resultset {
            types_by_tokens: Results::new(),
            types_by_words: Results::new(),
            tokens_by_words: Results::new(),
            total: 0,
        }
    }
}

impl Default for Resultset {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use density_curve::{Coord, SumLine, SumPoint, Value};

    fn sample(words: u64, tokens: Vec<SToken>) -> Sample {
        Sample { words, tokens }
    }

    fn st(count: u64, id: usize) -> SToken {
        SToken { count, id }
    }

    fn sl(y: Coord, sums: &[SumPoint]) -> SumLine {
        SumLine {
            y,
            sums: sums.to_vec(),
        }
    }

    fn sp(x: Coord, sum: Value) -> SumPoint {
        SumPoint { x, sum }
    }

    #[test]
    fn exact_binary_distinct() {
        let ds = Dataset::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
            sample(1, vec![st(1, 2)]),
        ]);
        assert_eq!(ds.total_words, 3);
        assert_eq!(ds.total_tokens, 3);
        assert_eq!(ds.total_types, 3);
        let rs = ds.count_exact();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower.to_sums(),
            rs.tokens_by_words.upper.to_sums(),
            rs.types_by_words.lower.to_sums(),
            rs.types_by_words.upper.to_sums(),
            rs.types_by_tokens.lower.to_sums(),
            rs.types_by_tokens.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 1 * 2 * 3)]));
        }
    }

    #[test]
    fn exact_large_distinct() {
        let ds = Dataset::new(vec![
            sample(1000, vec![st(100, 10), st(100, 11), st(100, 12)]),
            sample(1000, vec![st(100, 20), st(100, 21), st(100, 22)]),
            sample(1000, vec![st(100, 30), st(100, 31), st(100, 32)]),
        ]);
        assert_eq!(ds.total_words, 3000);
        assert_eq!(ds.total_tokens, 900);
        assert_eq!(ds.total_types, 33);
        let rs = ds.count_exact();
        assert_eq!(1 * 2 * 3, rs.total);
        let s = rs.types_by_words.lower.to_sums();
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(1000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(2000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(3000, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.types_by_words.upper.to_sums();
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(1, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(1001, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(2001, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.tokens_by_words.lower.to_sums();
        assert_eq!(s.ny, 901);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(301, &[sp(1000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(601, &[sp(2000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(901, &[sp(3000, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.tokens_by_words.upper.to_sums();
        assert_eq!(s.ny, 901);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(301, &[sp(1, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(601, &[sp(1001, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(901, &[sp(2001, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.types_by_tokens.lower.to_sums();
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 901);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(300, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(600, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(900, 0), sp(901, 1 * 2 * 3)]));
        let s = rs.types_by_tokens.upper.to_sums();
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 901);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(1, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(301, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(601, 0), sp(901, 1 * 2 * 3)]));
    }

    #[test]
    fn exact_binary_same() {
        let ds = Dataset::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
        ]);
        assert_eq!(ds.total_words, 3);
        assert_eq!(ds.total_tokens, 3);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count_exact();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower.to_sums(),
            rs.tokens_by_words.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 1 * 2 * 3)]));
        }
        for s in [
            rs.types_by_words.lower.to_sums(),
            rs.types_by_words.upper.to_sums(),
            rs.types_by_tokens.lower.to_sums(),
            rs.types_by_tokens.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
        }
    }

    #[test]
    fn exact_binary_partial_overlap() {
        let ds = Dataset::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
        ]);
        assert_eq!(ds.total_words, 3);
        assert_eq!(ds.total_tokens, 3);
        assert_eq!(ds.total_types, 2);
        let rs = ds.count_exact();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower.to_sums(),
            rs.tokens_by_words.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 1 * 2 * 3)]));
        }
        for s in [
            rs.types_by_words.lower.to_sums(),
            rs.types_by_words.upper.to_sums(),
            rs.types_by_tokens.lower.to_sums(),
            rs.types_by_tokens.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 4), sp(4, 1 * 2 * 3)]));
        }
    }

    fn exact_binary_distinct_helper(n: u64) {
        let mut fact = 1;
        let mut samples = Vec::new();
        for i in 0..n {
            samples.push(sample(1, vec![st(1, i as usize)]));
            fact *= i + 1;
        }
        let ds = Dataset::new(samples);
        assert_eq!(ds.total_words, n);
        assert_eq!(ds.total_tokens, n);
        assert_eq!(ds.total_types, n);
        let rs = ds.count_exact();
        assert_eq!(fact, rs.total);
        for s in [
            rs.tokens_by_words.lower.to_sums(),
            rs.tokens_by_words.upper.to_sums(),
            rs.types_by_words.lower.to_sums(),
            rs.types_by_words.upper.to_sums(),
            rs.types_by_tokens.lower.to_sums(),
            rs.types_by_tokens.upper.to_sums(),
        ] {
            assert_eq!(s.ny, n + 1);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len() as u64, n + 1);
            for i in 0..n + 1 {
                assert_eq!(
                    s.lines[i as usize],
                    sl(i + 1, &[sp(i, 0), sp(n + 1, fact as i64)])
                );
            }
        }
    }

    fn exact_binary_same_helper(n: u64) {
        let mut fact = 1;
        let mut samples = Vec::new();
        for i in 0..n {
            samples.push(sample(1, vec![st(1, 0)]));
            fact *= i + 1;
        }
        let ds = Dataset::new(samples);
        assert_eq!(ds.total_words, n);
        assert_eq!(ds.total_tokens, n);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count_exact();
        assert_eq!(fact, rs.total);
        for s in [
            rs.tokens_by_words.lower.to_sums(),
            rs.tokens_by_words.upper.to_sums(),
        ] {
            assert_eq!(s.ny, n + 1);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len() as u64, n + 1);
            for i in 0..n + 1 {
                assert_eq!(
                    s.lines[i as usize],
                    sl(i + 1, &[sp(i, 0), sp(n + 1, fact as i64)])
                );
            }
        }
        for s in [
            rs.types_by_words.lower.to_sums(),
            rs.types_by_words.upper.to_sums(),
            rs.types_by_tokens.lower.to_sums(),
            rs.types_by_tokens.upper.to_sums(),
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(n + 1, fact as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(n + 1, fact as i64)]));
        }
    }

    #[test]
    fn exact_binary_distinct_1() {
        exact_binary_distinct_helper(1);
    }

    #[test]
    fn exact_binary_distinct_2() {
        exact_binary_distinct_helper(2);
    }

    #[test]
    fn exact_binary_distinct_3() {
        exact_binary_distinct_helper(3);
    }

    #[test]
    fn exact_binary_distinct_4() {
        exact_binary_distinct_helper(4);
    }

    #[test]
    fn exact_binary_distinct_5() {
        exact_binary_distinct_helper(5);
    }

    #[test]
    fn exact_binary_distinct_6() {
        exact_binary_distinct_helper(6);
    }

    #[test]
    fn exact_binary_same_1() {
        exact_binary_same_helper(1);
    }

    #[test]
    fn exact_binary_same_2() {
        exact_binary_same_helper(2);
    }

    #[test]
    fn exact_binary_same_3() {
        exact_binary_same_helper(3);
    }

    #[test]
    fn exact_binary_same_4() {
        exact_binary_same_helper(4);
    }

    #[test]
    fn exact_binary_same_5() {
        exact_binary_same_helper(5);
    }

    #[test]
    fn exact_binary_same_6() {
        exact_binary_same_helper(6);
    }
}

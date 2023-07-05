mod density_curve;

use crossbeam_channel::TryRecvError;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use serde::{Deserialize, Serialize};
use std::thread;

/// *Minimum* number of tasks for exact calculation.
/// This does not influence the outcome, only performance.
/// Worst-case queue size is quadratic in `MIN_EXACT_JOBS`.
const MIN_EXACT_JOBS: u64 = 100;

/// Number of tasks for randomized calculation.
const RANDOM_JOBS: u64 = 1000;

/// Use exact method if the number of iterations is not more than `EXACT_THRESHOLD` times what was requested.
const EXACT_THRESHOLD: u64 = 2;

pub enum Method {
    Exact,
    Random(u64),
}

enum Progress {
    Tick,
    Done(Box<RawResult>),
}

#[derive(Deserialize, Serialize)]
pub struct SToken {
    pub count: u64,
    pub id: usize,
    pub flavor: usize,
}

#[derive(Deserialize, Serialize)]
pub struct Sample {
    pub words: u64,
    pub tokens: Vec<SToken>,
}

#[derive(Deserialize, Serialize)]
pub struct Samples {
    pub samples: Vec<Sample>,
}

pub struct Driver {
    /// Input data.
    samples: Vec<Sample>,
    /// All types have identifiers in `0..total_types`.
    total_types: usize,
    /// All flavors have identifiers in `0..total_flavors`.
    /// However, as a special case we set `total_flavors = 0` if all tokens have flavor 0;
    /// this effectively disables flavor-specific counters.
    total_flavors: usize,
    /// Print progress bar.
    progress: bool,
}

impl Driver {
    pub fn new(samples: Vec<Sample>) -> Driver {
        Driver::new_with_settings(samples, false)
    }

    pub fn new_with_settings(samples: Vec<Sample>, progress: bool) -> Driver {
        let mut max_type = 0;
        let mut max_flavor = 0;
        for sample in &samples {
            for token in &sample.tokens {
                max_type = max_type.max(token.id);
                max_flavor = max_flavor.max(token.flavor);
            }
        }
        let total_types = max_type + 1;
        let total_flavors = if max_flavor == 0 { 0 } else { max_flavor + 1 };
        Driver {
            samples,
            total_types,
            total_flavors,
            progress,
        }
    }

    fn progress_bar(&self, len: u64, nthreads: usize, what: &str) -> ProgressBar {
        if self.progress {
            let bar = ProgressBar::new(len);
            let style = ProgressStyle::with_template("{prefix:>12.blue.bold} {elapsed_precise} {bar:.dim} {pos:>6}/{len:6} {msg} · {eta} left").unwrap();
            bar.set_style(style);
            bar.set_prefix(what.to_owned());
            let nsamples = self.samples.len();
            let sampleword = if nsamples == 1 { "sample" } else { "samples" };
            let threadword = if nthreads == 1 { "thread" } else { "threads" };
            bar.set_message(format!(
                "{nsamples:5} {sampleword} · {nthreads} {threadword}"
            ));
            bar
        } else {
            ProgressBar::hidden()
        }
    }

    fn choose_exact_job_depth(&self) -> usize {
        let n = self.samples.len();
        let mut f = 1;
        let mut i = 0;
        while i < n && f < MIN_EXACT_JOBS {
            f *= (n - i) as u64;
            i += 1;
        }
        i
    }

    pub fn algorithm_heuristic(&self, iter: u64) -> Method {
        let n = self.samples.len();
        let mut f = 1;
        for i in 0..n {
            f *= (i + 1) as u64;
            if f > EXACT_THRESHOLD * iter {
                return Method::Random(iter);
            }
        }
        Method::Exact
    }

    pub fn count(&self, iter: u64) -> RawResult {
        self.count_method(self.algorithm_heuristic(iter))
    }

    pub fn count_seq(&self, iter: u64) -> RawResult {
        self.count_method_seq(self.algorithm_heuristic(iter))
    }

    pub fn count_method(&self, method: Method) -> RawResult {
        match method {
            Method::Exact => self.count_exact(),
            Method::Random(iter) => self.count_random(iter),
        }
    }

    pub fn count_method_seq(&self, method: Method) -> RawResult {
        match method {
            Method::Exact => self.count_exact_seq(),
            Method::Random(iter) => self.count_random_seq(iter),
        }
    }

    pub fn count_random(&self, iter: u64) -> RawResult {
        let (s1, r1) = crossbeam_channel::unbounded();
        for job in 0..RANDOM_JOBS {
            s1.send(job).expect("send succeeds");
        }
        let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
        drop(s1);
        let nthreads = num_cpus::get();
        let mut global = RawResult::new(false, self.total_flavors);
        let bar = self.progress_bar(RANDOM_JOBS, nthreads, "Random");
        thread::scope(|scope| {
            let (s2, r2) = crossbeam_channel::unbounded();
            for _ in 0..nthreads {
                let r1 = r1.clone();
                let s2 = s2.clone();
                scope.spawn(move || {
                    let mut rs = RawResult::new(false, self.total_flavors);
                    loop {
                        match r1.try_recv() {
                            Ok(job) => {
                                self.count_random_job(&mut rs, job, iter_per_job);
                                s2.send(Progress::Tick).expect("send succeeds");
                            }
                            Err(TryRecvError::Empty) => unreachable!(),
                            Err(TryRecvError::Disconnected) => break,
                        }
                    }
                    s2.send(Progress::Done(Box::new(rs)))
                        .expect("send succeeds");
                });
            }
            drop(s2);
            while let Ok(msg) = r2.recv() {
                match msg {
                    Progress::Tick => bar.inc(1),
                    Progress::Done(rs) => global.merge(&rs),
                }
            }
            bar.finish();
        });
        global
    }

    pub fn count_random_seq(&self, iter: u64) -> RawResult {
        let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
        let mut rs = RawResult::new(false, self.total_flavors);
        for job in 0..RANDOM_JOBS {
            self.count_random_job(&mut rs, job, iter_per_job);
        }
        rs
    }

    pub fn count_exact(&self) -> RawResult {
        let n = self.samples.len();
        let depth = self.choose_exact_job_depth();
        let (s1, r1) = crossbeam_channel::unbounded();
        let mut njobs = 0;
        for job in (0..n).permutations(depth) {
            s1.send(job).expect("send succeeds");
            njobs += 1;
        }
        drop(s1);
        let nthreads = num_cpus::get();
        let mut global = RawResult::new(true, self.total_flavors);
        let bar = self.progress_bar(njobs, nthreads, "Exact");
        thread::scope(|scope| {
            let (s2, r2) = crossbeam_channel::unbounded();
            for _ in 0..nthreads {
                let r1 = r1.clone();
                let s2 = s2.clone();
                scope.spawn(move || {
                    let mut rs = RawResult::new(true, self.total_flavors);
                    loop {
                        match r1.try_recv() {
                            Ok(job) => {
                                self.count_exact_start(&mut rs, &job);
                                s2.send(Progress::Tick).expect("send succeeds");
                            }
                            Err(TryRecvError::Empty) => unreachable!(),
                            Err(TryRecvError::Disconnected) => break,
                        }
                    }
                    s2.send(Progress::Done(Box::new(rs)))
                        .expect("send succeeds");
                });
            }
            drop(s2);
            while let Ok(msg) = r2.recv() {
                match msg {
                    Progress::Tick => bar.inc(1),
                    Progress::Done(rs) => global.merge(&rs),
                }
            }
            bar.finish();
        });
        global
    }

    pub fn count_exact_seq(&self) -> RawResult {
        let mut rs = RawResult::new(true, self.total_flavors);
        self.count_exact_start(&mut rs, &[]);
        rs
    }

    fn count_exact_start(&self, rs: &mut RawResult, start: &[usize]) {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        let mut used = vec![false; n];
        let mut i = 0;
        for &e in start {
            debug_assert!(!used[e]);
            used[e] = true;
            idx[i] = e;
            i += 1;
        }
        for (e, u) in used.iter_mut().enumerate() {
            if !*u {
                *u = true;
                idx[i] = e;
                i += 1;
            }
        }
        assert_eq!(i, n);
        let mut ls = LocalState::new(self.total_types, self.total_flavors);
        self.count_exact_rec(rs, &mut idx, start.len(), &mut ls);
    }

    fn count_exact_rec(
        &self,
        rs: &mut RawResult,
        idx: &mut [usize],
        i: usize,
        ls: &mut LocalState,
    ) {
        let n = self.samples.len();
        if i == n {
            self.update_counters(rs, idx, ls);
        } else {
            for j in i..n {
                idx.swap(i, j);
                self.count_exact_rec(rs, idx, i + 1, ls);
                idx.swap(i, j);
            }
        }
    }

    fn count_random_job(&self, rs: &mut RawResult, job: u64, iter_per_job: u64) {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, v) in idx.iter_mut().enumerate() {
            *v = i;
        }
        let mut ls = LocalState::new(self.total_types, self.total_flavors);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
        for _ in 0..iter_per_job {
            idx.shuffle(&mut rng);
            self.update_counters(rs, &idx, &mut ls);
        }
    }

    fn update_counters(&self, cs: &mut RawResult, idx: &[usize], ls: &mut LocalState) {
        ls.reset();
        for i in idx {
            ls.feed_sample(&self.samples[*i]);
            cs.feed(ls);
        }
        cs.finish();
    }
}

struct FCountHelper {
    types: u64,
    tokens: u64,
    seen: Vec<bool>,
}

impl FCountHelper {
    fn new(total_types: usize) -> FCountHelper {
        FCountHelper {
            types: 0,
            tokens: 0,
            seen: vec![false; total_types],
        }
    }

    fn reset(&mut self) {
        self.types = 0;
        self.tokens = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
    }

    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
        self.tokens += t.count;
    }
}

struct CountHelper {
    types: u64,
    tokens: u64,
    seen: Vec<bool>,
}

impl CountHelper {
    fn new(total_types: usize) -> CountHelper {
        CountHelper {
            types: 0,
            tokens: 0,
            seen: vec![false; total_types],
        }
    }

    fn reset(&mut self) {
        self.types = 0;
        self.tokens = 0;
        for e in self.seen.iter_mut() {
            *e = false;
        }
    }

    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
        self.tokens += t.count;
    }
}

struct LocalState {
    c: CountHelper,
    fc: Vec<FCountHelper>,
    words: u64,
}

impl LocalState {
    fn new(total_types: usize, total_flavors: usize) -> LocalState {
        LocalState {
            c: CountHelper::new(total_types),
            fc: (0..total_flavors)
                .map(|_| FCountHelper::new(total_types))
                .collect(),
            words: 0,
        }
    }

    fn reset(&mut self) {
        self.c.reset();
        for x in self.fc.iter_mut() {
            x.reset();
        }
        self.words = 0;
    }

    fn feed_token(&mut self, t: &SToken) {
        self.c.feed_token(t);
        if !self.fc.is_empty() {
            self.fc[t.flavor].feed_token(t);
        }
    }

    fn feed_sample(&mut self, sample: &Sample) {
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.words += sample.words;
    }
}

struct CounterPair {
    lower: density_curve::Counter,
    upper: density_curve::Counter,
    x: u64,
    l: u64,
    u: u64,
}

impl CounterPair {
    fn new() -> CounterPair {
        CounterPair {
            lower: density_curve::Counter::new(),
            upper: density_curve::Counter::new(),
            x: 0,
            l: 0,
            u: 0,
        }
    }

    fn merge(&mut self, other: &CounterPair) {
        debug_assert!(self.x == 0);
        debug_assert!(self.l == 0);
        debug_assert!(self.u == 0);
        debug_assert!(other.x == 0);
        debug_assert!(other.l == 0);
        debug_assert!(other.u == 0);
        self.lower.merge(&other.lower);
        self.upper.merge(&other.upper);
    }

    fn feed(&mut self, y: u64, x: u64) {
        debug_assert!(self.l <= self.u);
        debug_assert!(self.u <= y);
        debug_assert!(self.x <= x);
        if x == self.x {
            self.u = y;
        } else {
            self.lower.add(self.l, (self.x, self.x + 1), 1);
            self.lower.add(self.u, (self.x + 1, x), 1);
            self.upper.add(self.u, (self.x, self.x + 1), 1);
            self.upper.add(y, (self.x + 1, x), 1);
            self.x = x;
            self.l = y;
            self.u = y;
        }
    }

    fn finish(&mut self) {
        debug_assert!(self.l <= self.u);
        self.lower.add(self.l, (self.x, self.x + 1), 1);
        self.upper.add(self.u, (self.x, self.x + 1), 1);
        self.x = 0;
        self.l = 0;
        self.u = 0;
    }

    fn to_sums(&self) -> SumPair {
        SumPair {
            lower: self.lower.to_sums(),
            upper: self.upper.to_sums(),
        }
    }
}

impl Default for CounterPair {
    fn default() -> Self {
        Self::new()
    }
}

struct FCounterSet {
    ftypes_by_types: CounterPair,
    ftypes_by_tokens: CounterPair,
    ftypes_by_ftokens: CounterPair,
    ftypes_by_words: CounterPair,
    ftokens_by_tokens: CounterPair,
    ftokens_by_words: CounterPair,
}

impl FCounterSet {
    fn new() -> FCounterSet {
        FCounterSet {
            ftypes_by_types: CounterPair::new(),
            ftypes_by_tokens: CounterPair::new(),
            ftypes_by_ftokens: CounterPair::new(),
            ftypes_by_words: CounterPair::new(),
            ftokens_by_tokens: CounterPair::new(),
            ftokens_by_words: CounterPair::new(),
        }
    }

    fn merge(&mut self, other: &FCounterSet) {
        self.ftypes_by_types.merge(&other.ftypes_by_types);
        self.ftypes_by_tokens.merge(&other.ftypes_by_tokens);
        self.ftypes_by_ftokens.merge(&other.ftypes_by_ftokens);
        self.ftypes_by_words.merge(&other.ftypes_by_words);
        self.ftokens_by_tokens.merge(&other.ftokens_by_tokens);
        self.ftokens_by_words.merge(&other.ftokens_by_words);
    }

    pub fn to_sums(&self) -> FSumSet {
        FSumSet {
            ftypes_by_types: self.ftypes_by_types.to_sums(),
            ftypes_by_tokens: self.ftypes_by_tokens.to_sums(),
            ftypes_by_ftokens: self.ftypes_by_ftokens.to_sums(),
            ftypes_by_words: self.ftypes_by_words.to_sums(),
            ftokens_by_tokens: self.ftokens_by_tokens.to_sums(),
            ftokens_by_words: self.ftokens_by_words.to_sums(),
        }
    }

    fn feed(&mut self, words: u64, c: &CountHelper, fc: &FCountHelper) {
        let types = c.types;
        let tokens = c.tokens;
        let ftypes = fc.types;
        let ftokens = fc.tokens;
        self.ftypes_by_types.feed(ftypes, types);
        self.ftypes_by_tokens.feed(ftypes, tokens);
        self.ftypes_by_ftokens.feed(ftypes, ftokens);
        self.ftypes_by_words.feed(ftypes, words);
        self.ftokens_by_tokens.feed(ftokens, tokens);
        self.ftokens_by_words.feed(ftokens, words);
    }

    fn finish(&mut self) {
        self.ftypes_by_types.finish();
        self.ftypes_by_tokens.finish();
        self.ftypes_by_ftokens.finish();
        self.ftypes_by_words.finish();
        self.ftokens_by_tokens.finish();
        self.ftokens_by_words.finish();
    }
}

struct CounterSet {
    types_by_tokens: CounterPair,
    types_by_words: CounterPair,
    tokens_by_words: CounterPair,
}

impl CounterSet {
    fn new() -> CounterSet {
        CounterSet {
            types_by_tokens: CounterPair::new(),
            types_by_words: CounterPair::new(),
            tokens_by_words: CounterPair::new(),
        }
    }

    fn merge(&mut self, other: &CounterSet) {
        self.types_by_tokens.merge(&other.types_by_tokens);
        self.types_by_words.merge(&other.types_by_words);
        self.tokens_by_words.merge(&other.tokens_by_words);
    }

    fn feed(&mut self, words: u64, c: &CountHelper) {
        let types = c.types;
        let tokens = c.tokens;
        self.tokens_by_words.feed(tokens, words);
        self.types_by_words.feed(types, words);
        self.types_by_tokens.feed(types, tokens);
    }

    fn finish(&mut self) {
        self.tokens_by_words.finish();
        self.types_by_words.finish();
        self.types_by_tokens.finish();
    }
}

pub struct RawResult {
    c: CounterSet,
    fc: Vec<FCounterSet>,
    total: u64,
    exact: bool,
}

impl RawResult {
    fn new(exact: bool, nflavors: usize) -> RawResult {
        RawResult {
            c: CounterSet::new(),
            fc: (0..nflavors).map(|_| FCounterSet::new()).collect(),
            total: 0,
            exact,
        }
    }

    fn merge(&mut self, other: &RawResult) {
        self.c.merge(&other.c);
        for i in 0..self.fc.len() {
            self.fc[i].merge(&other.fc[i]);
        }
        self.total += other.total;
    }

    fn feed(&mut self, ls: &LocalState) {
        self.c.feed(ls.words, &ls.c);
        for (i, fc) in self.fc.iter_mut().enumerate() {
            fc.feed(ls.words, &ls.c, &ls.fc[i]);
        }
    }

    fn finish(&mut self) {
        self.c.finish();
        for fc in self.fc.iter_mut() {
            fc.finish();
        }
        self.total += 1;
    }

    pub fn to_sums(&self) -> SumSet {
        SumSet {
            types_by_tokens: self.c.types_by_tokens.to_sums(),
            types_by_words: self.c.types_by_words.to_sums(),
            tokens_by_words: self.c.tokens_by_words.to_sums(),
            by_flavor: self.fc.iter().map(|x| x.to_sums()).collect(),
            total: self.total,
            exact: self.exact,
        }
    }
}

#[derive(Serialize)]
pub struct SumPair {
    pub lower: density_curve::Sums,
    pub upper: density_curve::Sums,
}

impl SumPair {
    pub fn total_points(&self) -> usize {
        self.lower.total_points() + self.upper.total_points()
    }
}

#[derive(Serialize)]
pub struct FSumSet {
    ftypes_by_types: SumPair,
    ftypes_by_tokens: SumPair,
    ftypes_by_ftokens: SumPair,
    ftypes_by_words: SumPair,
    ftokens_by_tokens: SumPair,
    ftokens_by_words: SumPair,
}

impl FSumSet {
    pub fn total_points(&self) -> usize {
        self.ftypes_by_types.total_points()
            + self.ftypes_by_tokens.total_points()
            + self.ftypes_by_ftokens.total_points()
            + self.ftypes_by_words.total_points()
            + self.ftokens_by_tokens.total_points()
            + self.ftokens_by_words.total_points()
    }
}

#[derive(Serialize)]
pub struct SumSet {
    pub types_by_tokens: SumPair,
    pub types_by_words: SumPair,
    pub tokens_by_words: SumPair,
    pub by_flavor: Vec<FSumSet>,
    pub total: u64,
    pub exact: bool,
}

impl SumSet {
    pub fn total_points(&self) -> usize {
        let bfsum: usize = self.by_flavor.iter().map(|x| x.total_points()).sum();
        self.types_by_tokens.total_points()
            + self.types_by_words.total_points()
            + self.tokens_by_words.total_points()
            + bfsum
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
        SToken {
            count,
            id,
            flavor: 0,
        }
    }

    fn st1(count: u64, id: usize) -> SToken {
        SToken {
            count,
            id,
            flavor: 1,
        }
    }

    fn st2(count: u64, id: usize) -> SToken {
        SToken {
            count,
            id,
            flavor: 2,
        }
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
    fn exact_binary_distinct_seq() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
            sample(1, vec![st(1, 2)]),
        ]);
        assert_eq!(ds.total_types, 3);
        let rs = ds.count_exact_seq().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
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
    fn exact_binary_distinct() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
            sample(1, vec![st(1, 2)]),
        ]);
        assert_eq!(ds.total_types, 3);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
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
        let ds = Driver::new(vec![
            sample(1000, vec![st(100, 10), st(100, 11), st(100, 12)]),
            sample(1000, vec![st(100, 20), st(100, 21), st(100, 22)]),
            sample(1000, vec![st(100, 30), st(100, 31), st(100, 32)]),
        ]);
        assert_eq!(ds.total_types, 33);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        let s = rs.types_by_words.lower;
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(1000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(2000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(3000, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.types_by_words.upper;
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(1, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(1001, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(2001, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.tokens_by_words.lower;
        assert_eq!(s.ny, 901);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(301, &[sp(1000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(601, &[sp(2000, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(901, &[sp(3000, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.tokens_by_words.upper;
        assert_eq!(s.ny, 901);
        assert_eq!(s.nx, 3001);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(301, &[sp(1, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(601, &[sp(1001, 0), sp(3001, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(901, &[sp(2001, 0), sp(3001, 1 * 2 * 3)]));
        let s = rs.types_by_tokens.lower;
        assert_eq!(s.ny, 10);
        assert_eq!(s.nx, 901);
        assert_eq!(s.lines.len(), 4);
        assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[1], sl(4, &[sp(300, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[2], sl(7, &[sp(600, 0), sp(901, 1 * 2 * 3)]));
        assert_eq!(s.lines[3], sl(10, &[sp(900, 0), sp(901, 1 * 2 * 3)]));
        let s = rs.types_by_tokens.upper;
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
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
        ]);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 1 * 2 * 3)]));
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
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
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
        ]);
        assert_eq!(ds.total_types, 2);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 1 * 2 * 3)]));
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 1 * 2 * 3)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 4), sp(4, 1 * 2 * 3)]));
        }
    }

    #[test]
    fn random_binary_partial_overlap() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st(1, 1)]),
        ]);
        assert_eq!(ds.total_types, 2);
        let iter = 5000;
        let rs = ds.count_random(iter).to_sums();
        assert!(rs.total >= iter);
        assert!(rs.total < iter + RANDOM_JOBS);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, iter as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, iter as i64)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, iter as i64)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, iter as i64)]));
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, iter as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, iter as i64)]));
            assert_eq!(s.lines[2].y, 3);
            assert_eq!(s.lines[2].sums.len(), 3);
            assert_eq!(s.lines[2].sums[0], sp(2, 0));
            assert_eq!(s.lines[2].sums[1].x, 3);
            let expected = (2. / 3.) * iter as f64;
            let got = s.lines[2].sums[1].sum as f64;
            assert!(got >= 0.99 * expected);
            assert!(got <= 1.01 * expected);
            assert_eq!(s.lines[2].sums[2], sp(4, iter as i64));
        }
    }

    fn exact_binary_distinct_helper(n: u64) {
        let mut fact = 1;
        let mut samples = Vec::new();
        for i in 0..n {
            samples.push(sample(1, vec![st(1, i as usize)]));
            fact *= i + 1;
        }
        let ds = Driver::new(samples);
        assert_eq!(ds.total_types, n as usize);
        let rs = ds.count_exact().to_sums();
        assert_eq!(fact, rs.total);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
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
        let ds = Driver::new(samples);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count_exact().to_sums();
        assert_eq!(fact, rs.total);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
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
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(n + 1, fact as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(n + 1, fact as i64)]));
        }
    }

    fn random_binary_distinct_helper(n: u64, iter: u64) {
        let mut samples = Vec::new();
        for i in 0..n {
            samples.push(sample(1, vec![st(1, i as usize)]));
        }
        let ds = Driver::new(samples);
        assert_eq!(ds.total_types, n as usize);
        let rs = ds.count_random(iter).to_sums();
        assert!(rs.total >= iter);
        assert!(rs.total < iter + RANDOM_JOBS);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, n + 1);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len() as u64, n + 1);
            for i in 0..n + 1 {
                assert_eq!(
                    s.lines[i as usize],
                    sl(i + 1, &[sp(i, 0), sp(n + 1, rs.total as i64)])
                );
            }
        }
    }

    fn random_binary_same_helper(n: u64, iter: u64) {
        let mut samples = Vec::new();
        for _ in 0..n {
            samples.push(sample(1, vec![st(1, 0)]));
        }
        let ds = Driver::new(samples);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count_random(iter).to_sums();
        assert!(rs.total >= iter);
        assert!(rs.total < iter + RANDOM_JOBS);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, n + 1);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len() as u64, n + 1);
            for i in 0..n + 1 {
                assert_eq!(
                    s.lines[i as usize],
                    sl(i + 1, &[sp(i, 0), sp(n + 1, rs.total as i64)])
                );
            }
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(n + 1, rs.total as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(n + 1, rs.total as i64)]));
        }
    }

    fn auto_binary_same_helper(n: u64, iter: u64) {
        let mut samples = Vec::new();
        for _ in 0..n {
            samples.push(sample(1, vec![st(1, 0)]));
        }
        let ds = Driver::new(samples);
        assert_eq!(ds.total_types, 1);
        let rs = ds.count(iter).to_sums();
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, n + 1);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len() as u64, n + 1);
            for i in 0..n + 1 {
                assert_eq!(
                    s.lines[i as usize],
                    sl(i + 1, &[sp(i, 0), sp(n + 1, rs.total as i64)])
                );
            }
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, n + 1);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(n + 1, rs.total as i64)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(n + 1, rs.total as i64)]));
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
    fn exact_binary_distinct_7() {
        exact_binary_distinct_helper(7);
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

    #[test]
    fn exact_binary_same_7() {
        exact_binary_same_helper(7);
    }

    #[test]
    fn random_binary_distinct_5_10() {
        random_binary_distinct_helper(5, 10);
    }

    #[test]
    fn random_binary_distinct_5_5000() {
        random_binary_distinct_helper(5, 5000);
    }

    #[test]
    fn random_binary_distinct_5_1234() {
        random_binary_distinct_helper(5, 1234);
    }

    #[test]
    fn random_binary_distinct_50_5000() {
        random_binary_distinct_helper(50, 5000);
    }

    #[test]
    fn random_binary_same_5_10() {
        random_binary_same_helper(5, 10);
    }

    #[test]
    fn random_binary_same_5_5000() {
        random_binary_same_helper(5, 5000);
    }

    #[test]
    fn random_binary_same_5_1234() {
        random_binary_same_helper(5, 1234);
    }

    #[test]
    fn random_binary_same_50_5000() {
        random_binary_same_helper(50, 5000);
    }

    #[test]
    fn auto_binary_same_5_5000() {
        auto_binary_same_helper(5, 5000);
    }

    #[test]
    fn auto_binary_same_50_5000() {
        auto_binary_same_helper(50, 5000);
    }

    #[test]
    fn exact_with_empties() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![]),
            sample(1, vec![st(1, 1)]),
        ]);
        assert_eq!(ds.total_types, 2);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 4), sp(4, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 2), sp(4, 6)]));
        }
        for s in [rs.types_by_tokens.lower, rs.types_by_tokens.upper] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(3, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 6)]));
        }
    }

    #[test]
    fn exact_binary_flavor_2_overlap() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st1(1, 0)]),
            sample(1, vec![st(1, 1)]),
        ]);
        assert_eq!(ds.total_types, 2);
        assert_eq!(ds.total_flavors, 2);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [rs.tokens_by_words.lower, rs.tokens_by_words.upper] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 6)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 6)]));
        }
        for s in [
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 4), sp(4, 6)]));
        }
        for s in [
            &rs.by_flavor[0].ftypes_by_ftokens.lower,
            &rs.by_flavor[0].ftypes_by_ftokens.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(3, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 6)]));
        }
        for s in [&rs.by_flavor[0].ftypes_by_types.lower] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 4), sp(3, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 4)]));
        }
        for s in [&rs.by_flavor[0].ftypes_by_types.upper] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 5), sp(3, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 6)]));
        }
        for s in [
            &rs.by_flavor[0].ftypes_by_words.lower,
            &rs.by_flavor[0].ftypes_by_words.upper,
            &rs.by_flavor[0].ftypes_by_tokens.lower,
            &rs.by_flavor[0].ftypes_by_tokens.upper,
            &rs.by_flavor[0].ftokens_by_words.lower,
            &rs.by_flavor[0].ftokens_by_words.upper,
            &rs.by_flavor[0].ftokens_by_tokens.lower,
            &rs.by_flavor[0].ftokens_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 3);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 3);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 4), sp(4, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(3, 2), sp(4, 6)]));
        }
        for s in [
            &rs.by_flavor[1].ftypes_by_ftokens.lower,
            &rs.by_flavor[1].ftypes_by_ftokens.upper,
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, 2);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(2, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 6)]));
        }
        for s in [&rs.by_flavor[1].ftypes_by_types.lower] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 2), sp(3, 4)]));
        }
        for s in [&rs.by_flavor[1].ftypes_by_types.upper] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, 3);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(3, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 3), sp(3, 6)]));
        }
        for s in [
            &rs.by_flavor[1].ftypes_by_words.lower,
            &rs.by_flavor[1].ftypes_by_words.upper,
            &rs.by_flavor[1].ftypes_by_tokens.lower,
            &rs.by_flavor[1].ftypes_by_tokens.upper,
            &rs.by_flavor[1].ftokens_by_words.lower,
            &rs.by_flavor[1].ftokens_by_words.upper,
            &rs.by_flavor[1].ftokens_by_tokens.lower,
            &rs.by_flavor[1].ftokens_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 2);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 2);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 2), sp(3, 4), sp(4, 6)]));
        }
    }

    #[test]
    fn exact_binary_flavor_3_distinct() {
        let ds = Driver::new(vec![
            sample(1, vec![st(1, 0)]),
            sample(1, vec![st1(1, 1)]),
            sample(1, vec![st2(1, 2)]),
        ]);
        assert_eq!(ds.total_types, 3);
        assert_eq!(ds.total_flavors, 3);
        let rs = ds.count_exact().to_sums();
        assert_eq!(1 * 2 * 3, rs.total);
        for s in [
            rs.tokens_by_words.lower,
            rs.tokens_by_words.upper,
            rs.types_by_words.lower,
            rs.types_by_words.upper,
            rs.types_by_tokens.lower,
            rs.types_by_tokens.upper,
        ] {
            assert_eq!(s.ny, 4);
            assert_eq!(s.nx, 4);
            assert_eq!(s.lines.len(), 4);
            assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
            assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(4, 6)]));
            assert_eq!(s.lines[2], sl(3, &[sp(2, 0), sp(4, 6)]));
            assert_eq!(s.lines[3], sl(4, &[sp(3, 0), sp(4, 6)]));
        }
        for i in 0..3 {
            for s in [
                &rs.by_flavor[i].ftypes_by_ftokens.lower,
                &rs.by_flavor[i].ftypes_by_ftokens.upper,
            ] {
                assert_eq!(s.ny, 2);
                assert_eq!(s.nx, 2);
                assert_eq!(s.lines.len(), 2);
                assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(2, 6)]));
                assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 6)]));
            }
            for s in [
                &rs.by_flavor[i].ftypes_by_types.lower,
                &rs.by_flavor[i].ftypes_by_types.upper,
                &rs.by_flavor[i].ftypes_by_words.lower,
                &rs.by_flavor[i].ftypes_by_words.upper,
                &rs.by_flavor[i].ftypes_by_tokens.lower,
                &rs.by_flavor[i].ftypes_by_tokens.upper,
                &rs.by_flavor[i].ftokens_by_words.lower,
                &rs.by_flavor[i].ftokens_by_words.upper,
                &rs.by_flavor[i].ftokens_by_tokens.lower,
                &rs.by_flavor[i].ftokens_by_tokens.upper,
            ] {
                assert_eq!(s.ny, 2);
                assert_eq!(s.nx, 4);
                assert_eq!(s.lines.len(), 2);
                assert_eq!(s.lines[0], sl(1, &[sp(0, 0), sp(4, 6)]));
                assert_eq!(s.lines[1], sl(2, &[sp(1, 0), sp(2, 2), sp(3, 4), sp(4, 6)]));
            }
        }
    }
}

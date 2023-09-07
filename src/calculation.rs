use crate::output::{AvgResult, PointResult};
use crate::parallelism::{compute_parallel, RawResult};
use crate::shuffle::shuffle_job;
use itertools::Itertools;
use std::cmp::Ordering;

pub struct SToken {
    pub count: u64,
    pub id: usize,
}

pub struct Sample {
    pub size: u64,
    pub tokens: Vec<SToken>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub size: u64,
    pub types: u64,
}

pub fn average_at_limit(samples: &[Sample], iter: u64, limit: u64) -> AvgResult {
    let total_types = count_types(samples);
    let (r, iter) = compute_parallel(
        RawAvgResult::new,
        |job, result| {
            let mut ls = LocalState::new(total_types);
            shuffle_job(
                |idx| {
                    ls.reset();
                    for i in idx {
                        let prev = ls.types;
                        ls.feed_sample(&samples[*i]);
                        match ls.size.cmp(&limit) {
                            Ordering::Less => (),
                            Ordering::Equal => {
                                result.types_low += ls.types;
                                result.types_high += ls.types;
                                return;
                            }
                            Ordering::Greater => {
                                result.types_low += prev;
                                result.types_high += ls.types;
                                return;
                            }
                        }
                    }
                    unreachable!();
                },
                samples.len(),
                job,
            );
        },
        iter,
    );
    r.finalize(iter)
}

pub fn compare_with_points(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult> {
    assert!(!points.is_empty());
    let total_types = count_types(samples);
    let (r, iter) = compute_parallel(
        || RawPointResults {
            results: vec![RawPointResult::new(); points.len()],
        },
        |job, result| {
            let mut ls = LocalState::new(total_types);
            shuffle_job(
                |idx| {
                    ls.reset();
                    let mut j = 0;
                    for i in idx {
                        let prev = ls.types;
                        ls.feed_sample(&samples[*i]);
                        loop {
                            let p = &points[j];
                            if ls.size < p.size {
                                break;
                            }
                            if prev < p.types {
                                result.results[j].above += 1;
                            } else if ls.types > p.types {
                                result.results[j].below += 1;
                            }
                            j += 1;
                            if j == points.len() {
                                return;
                            }
                        }
                    }
                    unreachable!();
                },
                samples.len(),
                job,
            );
        },
        iter,
    );
    r.finalize(iter)
}

struct LocalState {
    size: u64,
    types: u64,
    seen: Vec<bool>,
}

impl LocalState {
    fn new(total_types: usize) -> LocalState {
        LocalState {
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

    fn feed_token(&mut self, t: &SToken) {
        if !self.seen[t.id] {
            self.types += 1;
            self.seen[t.id] = true;
        }
    }

    fn feed_sample(&mut self, sample: &Sample) {
        for t in &sample.tokens {
            self.feed_token(t);
        }
        self.size += sample.size;
    }
}

fn count_types(samples: &[Sample]) -> usize {
    let mut max_type = 0;
    for sample in samples {
        for token in &sample.tokens {
            max_type = max_type.max(token.id);
        }
    }
    max_type + 1
}

struct RawAvgResult {
    types_low: u64,
    types_high: u64,
}

impl RawResult for RawAvgResult {
    fn add(&mut self, other: Self) {
        self.types_low += other.types_low;
        self.types_high += other.types_high;
    }
}

impl RawAvgResult {
    fn new() -> RawAvgResult {
        RawAvgResult {
            types_low: 0,
            types_high: 0,
        }
    }

    fn finalize(self, iter: u64) -> AvgResult {
        AvgResult {
            types_low: self.types_low,
            types_high: self.types_high,
            iter,
        }
    }
}

#[derive(Clone, Copy)]
struct RawPointResult {
    above: u64,
    below: u64,
}

impl RawPointResult {
    fn new() -> RawPointResult {
        RawPointResult { above: 0, below: 0 }
    }

    fn add(&mut self, other: Self) {
        self.above += other.above;
        self.below += other.below;
    }
}

struct RawPointResults {
    results: Vec<RawPointResult>,
}

impl RawResult for RawPointResults {
    fn add(&mut self, other: Self) {
        debug_assert_eq!(self.results.len(), other.results.len());
        for i in 0..self.results.len() {
            self.results[i].add(other.results[i]);
        }
    }
}

impl RawPointResults {
    fn finalize(self, iter: u64) -> Vec<PointResult> {
        self.results
            .into_iter()
            .map(|x| PointResult {
                above: x.above,
                below: x.below,
                iter,
            })
            .collect_vec()
    }
}

use crate::calculation::{count_types, LocalState, Sample};
use crate::output::PointResult;
use crate::parallelism::{compute_parallel, RawResult};
use crate::shuffle::shuffle_job;
use itertools::Itertools;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub size: u64,
    pub types: u64,
}

pub fn compare_with_points(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult> {
    assert!(!points.is_empty());
    let total_types = count_types(samples);
    let (r, iter) = compute_parallel(
        || RawPointResults {
            results: vec![RawPointResult { above: 0, below: 0 }; points.len()],
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
    r.results
        .into_iter()
        .map(|x| PointResult {
            above: x.above,
            below: x.below,
            iter,
        })
        .collect_vec()
}

#[derive(Clone, Copy)]
struct RawPointResult {
    above: u64,
    below: u64,
}

impl RawPointResult {
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

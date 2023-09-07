use crate::calculation::{count_types, Sample, TypeCounter};
use crate::output::PointResult;
use crate::parallelism::{compute_parallel, ParResult};
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
        || PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        },
        |job, result| {
            let mut tc = TypeCounter::new(total_types);
            shuffle_job(
                |idx| {
                    tc.reset();
                    let mut j = 0;
                    for i in idx {
                        let prev = tc.types;
                        tc.feed_sample(&samples[*i]);
                        loop {
                            let p = &points[j];
                            if tc.size < p.size {
                                break;
                            }
                            if prev < p.types {
                                result.elems[j].above += 1;
                            } else if tc.types > p.types {
                                result.elems[j].below += 1;
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
    r.elems
        .into_iter()
        .map(|x| PointResult {
            above: x.above,
            below: x.below,
            iter,
        })
        .collect_vec()
}

#[derive(Clone, Copy)]
struct PointParResultElem {
    above: u64,
    below: u64,
}

impl PointParResultElem {
    fn add(&mut self, other: Self) {
        self.above += other.above;
        self.below += other.below;
    }
}

struct PointParResult {
    elems: Vec<PointParResultElem>,
}

impl ParResult for PointParResult {
    fn add(&mut self, other: Self) {
        debug_assert_eq!(self.elems.len(), other.elems.len());
        for i in 0..self.elems.len() {
            self.elems[i].add(other.elems[i]);
        }
    }
}

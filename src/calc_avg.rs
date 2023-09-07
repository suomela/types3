use crate::calculation::{count_types, LocalState, Sample};
use crate::output::AvgResult;
use crate::parallelism::{compute_parallel, RawResult};
use crate::shuffle::shuffle_job;
use std::cmp::Ordering;

pub fn average_at_limit(samples: &[Sample], iter: u64, limit: u64) -> AvgResult {
    let total_types = count_types(samples);
    let (r, iter) = compute_parallel(
        || RawAvgResult {
            types_low: 0,
            types_high: 0,
        },
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
    AvgResult {
        types_low: r.types_low,
        types_high: r.types_high,
        iter,
    }
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

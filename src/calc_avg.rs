use crate::calculation::Sample;
use crate::counter::{count_types, Counter, TypeCounter};
use crate::output::AvgResult;
use crate::parallelism::{compute_parallel, ParResult};
use crate::shuffle::shuffle_job;
use std::cmp::Ordering;

pub fn average_at_limit(samples: &[Sample], iter: u64, limit: u64) -> AvgResult {
    let total_types = count_types(samples);
    let (r, iter) = compute_parallel(
        || AvgParResult {
            low: 0,
            high: 0,
        },
        |job, result| {
            let mut counter = TypeCounter::new(total_types);
            shuffle_job(
                |idx| {
                    counter.reset();
                    for i in idx {
                        let prev_y = counter.get_y();
                        counter.feed_sample(&samples[*i]);
                        match counter.get_x().cmp(&limit) {
                            Ordering::Less => (),
                            Ordering::Equal => {
                                result.low += counter.get_y();
                                result.high += counter.get_y();
                                return;
                            }
                            Ordering::Greater => {
                                result.low += prev_y;
                                result.high += counter.get_y();
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
        low: r.low,
        high: r.high,
        iter,
    }
}

struct AvgParResult {
    low: u64,
    high: u64,
}

impl ParResult for AvgParResult {
    fn add(&mut self, other: Self) {
        self.low += other.low;
        self.high += other.high;
    }
}

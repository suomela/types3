use crate::calculation::Sample;
use crate::counter::{self, Counter, TokenCounter, TypeCounter};
use crate::output::{AvgResult, MeasureY};
use crate::parallelism::{self, ParResult};
use crate::shuffle;
use std::cmp::Ordering;

pub fn average_at_limit(
    measure_y: MeasureY,
    samples: &[Sample],
    iter: u64,
    limit: u64,
) -> AvgResult {
    match measure_y {
        MeasureY::Types => do_count::<TypeCounter>(samples, iter, limit),
        MeasureY::Tokens => do_count::<TokenCounter>(samples, iter, limit),
    }
}

fn do_count<TCounter>(samples: &[Sample], iter: u64, limit: u64) -> AvgResult
where
    TCounter: Counter,
{
    let total_types = counter::count_types(samples);
    let (r, iter) = parallelism::compute_parallel(
        || AvgParResult { low: 0, high: 0 },
        |job, result| {
            let mut counter = TCounter::new(total_types);
            shuffle::shuffle_job(
                |idx| calc_one(samples, limit, idx, &mut counter, result),
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

fn calc_one<TCounter>(
    samples: &[Sample],
    limit: u64,
    idx: &[usize],
    counter: &mut TCounter,
    result: &mut AvgParResult,
) where
    TCounter: Counter,
{
    counter.reset();
    for i in idx {
        let prev_y = counter.get_y();
        counter.feed_sample(&samples[*i]);
        let cur_y = counter.get_y();
        let low_y = cur_y.min(prev_y);
        let high_y = cur_y.max(prev_y);
        match counter.get_x().cmp(&limit) {
            Ordering::Less => (),
            Ordering::Equal => {
                result.low += cur_y;
                result.high += cur_y;
                return;
            }
            Ordering::Greater => {
                result.low += low_y;
                result.high += high_y;
                return;
            }
        }
    }
    unreachable!();
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

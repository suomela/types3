use crate::calculation::Sample;
use crate::counter::{self, Counter, TokenCounter, TypeCounter};
use crate::output::{MeasureY, PointResult};
use crate::parallelism::{self, ParResult};
use crate::shuffle;
use itertools::Itertools;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: u64,
    pub y: u64,
}

pub fn compare_with_points(
    measure_y: MeasureY,
    samples: &[Sample],
    iter: u64,
    points: &[Point],
) -> Vec<PointResult> {
    match measure_y {
        MeasureY::Types => do_count::<TypeCounter>(samples, iter, points),
        MeasureY::Tokens => do_count::<TokenCounter>(samples, iter, points),
    }
}

fn do_count<TCounter>(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult>
where
    TCounter: Counter,
{
    assert!(!points.is_empty());
    let total_types = counter::count_types(samples);
    let (r, iter) = parallelism::compute_parallel(
        || PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        },
        |job, result| {
            let mut counter = TCounter::new(total_types);
            shuffle::shuffle_job(
                |idx| calc_one(samples, points, idx, &mut counter, result),
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

fn calc_one<TCounter>(
    samples: &[Sample],
    points: &[Point],
    idx: &[usize],
    counter: &mut TCounter,
    result: &mut PointParResult,
) where
    TCounter: Counter,
{
    counter.reset();
    let mut j = 0;
    for i in idx {
        let prev_y = counter.get_y();
        counter.feed_sample(&samples[*i]);
        let cur_y = counter.get_y();
        let low_y = cur_y.min(prev_y);
        let high_y = cur_y.max(prev_y);
        loop {
            let p = &points[j];
            if counter.get_x() < p.x {
                break;
            }
            if high_y < p.y {
                result.elems[j].above += 1;
            } else if low_y > p.y {
                result.elems[j].below += 1;
            }
            j += 1;
            if j == points.len() {
                return;
            }
        }
    }
    unreachable!();
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

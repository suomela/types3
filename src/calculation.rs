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
    let (r, iter) = compute(samples, iter, AvgComp { limit });
    r.finalize(iter)
}

pub fn compare_with_points(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult> {
    assert!(!points.is_empty());
    let (r, iter) = compute(samples, iter, PointComp { points });
    r.finalize(iter)
}

trait Comp<TRawResult, TTracker>
where
    TRawResult: RawResult,
    TTracker: Tracker,
{
    fn build_total(&self) -> TRawResult;
    fn start(&self, result: &mut TRawResult) -> TTracker;
    fn step(
        &self,
        result: &mut TRawResult,
        tracker: &mut TTracker,
        prev: u64,
        types: u64,
        size: u64,
    ) -> bool;
}

trait Tracker {}

struct NoTracker {}

impl Tracker for NoTracker {}

struct CountTracker {
    j: usize,
}

impl Tracker for CountTracker {}

struct AvgComp {
    limit: u64,
}

struct PointComp<'a> {
    points: &'a [Point],
}

impl Comp<RawAvgResult, NoTracker> for AvgComp {
    fn start(&self, _result: &mut RawAvgResult) -> NoTracker {
        NoTracker {}
    }

    fn build_total(&self) -> RawAvgResult {
        RawAvgResult::new()
    }

    fn step(
        &self,
        result: &mut RawAvgResult,
        _tracker: &mut NoTracker,
        prev: u64,
        types: u64,
        size: u64,
    ) -> bool {
        match size.cmp(&self.limit) {
            Ordering::Less => false,
            Ordering::Equal => {
                result.types_low += types;
                result.types_high += types;
                true
            }
            Ordering::Greater => {
                result.types_low += prev;
                result.types_high += types;
                true
            }
        }
    }
}

impl Comp<RawPointResults, CountTracker> for PointComp<'_> {
    fn start(&self, _result: &mut RawPointResults) -> CountTracker {
        CountTracker { j: 0 }
    }

    fn build_total(&self) -> RawPointResults {
        RawPointResults {
            results: vec![RawPointResult::new(); self.points.len()],
        }
    }

    fn step(
        &self,
        result: &mut RawPointResults,
        tracker: &mut CountTracker,
        prev: u64,
        types: u64,
        size: u64,
    ) -> bool {
        loop {
            let p = &self.points[tracker.j];
            if size < p.size {
                return false;
            }
            if prev < p.types {
                result.results[tracker.j].above += 1;
            } else if types > p.types {
                result.results[tracker.j].below += 1;
            }
            tracker.j += 1;
            if tracker.j == self.points.len() {
                return true;
            }
        }
    }
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

fn compute<TComp, TTracker, TRawResult>(
    samples: &[Sample],
    iter: u64,
    comp: TComp,
) -> (TRawResult, u64)
where
    TComp: Comp<TRawResult, TTracker> + Send + Sync,
    TTracker: Tracker,
    TRawResult: RawResult + Send,
{
    let total_types = count_types(samples);
    compute_parallel(
        || comp.build_total(),
        |job, iter_per_job, result| {
            let mut ls = LocalState::new(total_types);
            shuffle_job(
                |idx| {
                    ls.reset();
                    let mut tracker = comp.start(result);
                    for i in idx {
                        let prev = ls.types;
                        ls.feed_sample(&samples[*i]);
                        if comp.step(result, &mut tracker, prev, ls.types, ls.size) {
                            return;
                        }
                    }
                    unreachable!();
                },
                samples.len(),
                job,
                iter_per_job,
            );
        },
        iter,
    )
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

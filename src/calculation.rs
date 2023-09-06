use crate::output::{AvgResult, PointResult};
use core::marker::PhantomData;
use crossbeam_channel::TryRecvError;
use itertools::Itertools;
use log::trace;
use rand::seq::SliceRandom;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;
use std::cmp::Ordering;
use std::thread;

/// Number of tasks for randomized calculation.
const RANDOM_JOBS: u64 = 1000;

fn compute_parallel<TRawResult, TBuilder, TRunner>(
    builder: TBuilder,
    runner: TRunner,
    iter: u64,
) -> (TRawResult, u64)
where
    TRawResult: RawResult + Send,
    TBuilder: Fn() -> TRawResult + Send + Copy,
    TRunner: Fn(u64, u64, &mut TRawResult) + Send + Copy,
{
    let (s1, r1) = crossbeam_channel::unbounded();
    for job in 0..RANDOM_JOBS {
        s1.send(job).expect("send succeeds");
    }
    let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
    let iter = iter_per_job * RANDOM_JOBS;
    drop(s1);
    let nthreads = num_cpus::get();
    let mut total = builder();
    trace!("randomized, {RANDOM_JOBS} jobs, {nthreads} threads");
    thread::scope(|scope| {
        let (s2, r2) = crossbeam_channel::unbounded();
        for _ in 0..nthreads {
            let r1 = r1.clone();
            let s2 = s2.clone();
            scope.spawn(move || {
                let mut thread_total = builder();
                loop {
                    match r1.try_recv() {
                        Ok(job) => {
                            runner(job, iter_per_job, &mut thread_total);
                        }
                        Err(TryRecvError::Empty) => unreachable!(),
                        Err(TryRecvError::Disconnected) => break,
                    }
                }
                s2.send(thread_total).expect("send succeeds");
            });
        }
        drop(s2);
        while let Ok(thread_total) = r2.recv() {
            total.add(thread_total);
        }
    });
    (total, iter)
}

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
    let (r, iter) = Driver::new(samples, AvgComp { limit }).compute(iter);
    r.finalize(iter)
}

pub fn compare_with_points(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult> {
    let (r, iter) = Driver::new(samples, PointComp { points }).compute(iter);
    r.finalize(iter)
}

trait Comp<TRawResult, TTracker>
where
    TRawResult: RawResult,
    TTracker: Tracker,
{
    fn sanity(&self);
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
    fn sanity(&self) {}

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
    fn sanity(&self) {
        assert!(!self.points.is_empty());
    }

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

struct Driver<'a, TComp, TRawResult, TTracker>
where
    TComp: Comp<TRawResult, TTracker>,
    TRawResult: RawResult,
    TTracker: Tracker,
{
    /// Input data.
    samples: &'a [Sample],
    /// All types have identifiers in `0..total_types`.
    total_types: usize,
    comp: TComp,
    _raw_result: PhantomData<TRawResult>,
    _tracker: PhantomData<TTracker>,
}

impl<TComp, TRawResult, TTracker> Driver<'_, TComp, TRawResult, TTracker>
where
    TComp: Send + Sync + Comp<TRawResult, TTracker>,
    TRawResult: Send + Sync + RawResult,
    TTracker: Send + Sync + Tracker,
{
    fn new(samples: &[Sample], comp: TComp) -> Driver<TComp, TRawResult, TTracker> {
        let mut max_type = 0;
        for sample in samples {
            for token in &sample.tokens {
                max_type = max_type.max(token.id);
            }
        }
        let total_types = max_type + 1;
        Driver {
            samples,
            total_types,
            comp,
            _raw_result: PhantomData,
            _tracker: PhantomData,
        }
    }

    fn compute(&self, iter: u64) -> (TRawResult, u64) {
        self.comp.sanity();
        compute_parallel(
            || self.comp.build_total(),
            |job, iter_per_job, result| self.job(job, iter_per_job, result),
            iter,
        )
    }

    fn job(&self, job: u64, iter_per_job: u64, result: &mut TRawResult) {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, v) in idx.iter_mut().enumerate() {
            *v = i;
        }
        let mut ls = LocalState::new(self.total_types);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
        for _ in 0..iter_per_job {
            idx.shuffle(&mut rng);
            self.calc_one(&idx, &mut ls, result);
        }
    }

    fn calc_one(&self, idx: &[usize], ls: &mut LocalState, result: &mut TRawResult) {
        ls.reset();
        let mut tracker = self.comp.start(result);
        for i in idx {
            let prev = ls.types;
            ls.feed_sample(&self.samples[*i]);
            if self
                .comp
                .step(result, &mut tracker, prev, ls.types, ls.size)
            {
                return;
            }
        }
        unreachable!();
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

trait RawResult {
    fn add(&mut self, other: Self);
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

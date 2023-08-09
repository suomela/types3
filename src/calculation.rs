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

#[derive(Clone)]
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
    Driver::new(samples).average_at_limit(iter, limit)
}

pub fn compare_with_points(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult> {
    Driver::new(samples).compare_with_points(iter, points)
}

struct Driver<'a> {
    /// Input data.
    samples: &'a [Sample],
    /// All types have identifiers in `0..total_types`.
    total_types: usize,
}

impl Driver<'_> {
    fn new(samples: &[Sample]) -> Driver {
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
        }
    }

    fn compare_with_points(&self, iter: u64, points: &[Point]) -> Vec<PointResult> {
        assert!(!points.is_empty());
        let (s1, r1) = crossbeam_channel::unbounded();
        for job in 0..RANDOM_JOBS {
            s1.send(job).expect("send succeeds");
        }
        let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
        let iter = iter_per_job * RANDOM_JOBS;
        drop(s1);
        let nthreads = num_cpus::get();
        let mut total = vec![RawPointResult::new(); points.len()];
        trace!("randomized, {RANDOM_JOBS} jobs, {nthreads} threads");
        thread::scope(|scope| {
            let (s2, r2) = crossbeam_channel::unbounded();
            for _ in 0..nthreads {
                let r1 = r1.clone();
                let s2 = s2.clone();
                scope.spawn(move || {
                    let mut thread_total = vec![RawPointResult::new(); points.len()];
                    loop {
                        match r1.try_recv() {
                            Ok(job) => {
                                self.compare_with_points_job(
                                    job,
                                    iter_per_job,
                                    points,
                                    &mut thread_total,
                                );
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
                for i in 0..points.len() {
                    total[i].add(&thread_total[i]);
                }
            }
        });
        total
            .into_iter()
            .map(|x| PointResult {
                above: x.above as f64 / iter as f64,
                below: x.below as f64 / iter as f64,
            })
            .collect_vec()
    }

    fn average_at_limit(&self, iter: u64, limit: u64) -> AvgResult {
        let (s1, r1) = crossbeam_channel::unbounded();
        for job in 0..RANDOM_JOBS {
            s1.send(job).expect("send succeeds");
        }
        let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
        let iter = iter_per_job * RANDOM_JOBS;
        drop(s1);
        let nthreads = num_cpus::get();
        let mut total = RawAvgResult::new();
        trace!("randomized, {RANDOM_JOBS} jobs, {nthreads} threads");
        thread::scope(|scope| {
            let (s2, r2) = crossbeam_channel::unbounded();
            for _ in 0..nthreads {
                let r1 = r1.clone();
                let s2 = s2.clone();
                scope.spawn(move || {
                    let mut thread_total = RawAvgResult::new();
                    loop {
                        match r1.try_recv() {
                            Ok(job) => {
                                thread_total.add(&self.average_at_limit_job(
                                    job,
                                    iter_per_job,
                                    limit,
                                ));
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
                total.add(&thread_total);
            }
        });
        AvgResult {
            types_low: total.types_low as f64 / iter as f64,
            types_high: total.types_high as f64 / iter as f64,
        }
    }

    fn compare_with_points_job(
        &self,
        job: u64,
        iter_per_job: u64,
        points: &[Point],
        result: &mut [RawPointResult],
    ) {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, v) in idx.iter_mut().enumerate() {
            *v = i;
        }
        let mut ls = LocalState::new(self.total_types);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
        for _ in 0..iter_per_job {
            idx.shuffle(&mut rng);
            self.compare_with_points_calc(&idx, &mut ls, points, result);
        }
    }

    fn average_at_limit_job(&self, job: u64, iter_per_job: u64, limit: u64) -> RawAvgResult {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, v) in idx.iter_mut().enumerate() {
            *v = i;
        }
        let mut ls = LocalState::new(self.total_types);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
        let mut total = RawAvgResult::new();
        for _ in 0..iter_per_job {
            idx.shuffle(&mut rng);
            total.add(&self.average_at_limit_calc(&idx, &mut ls, limit));
        }
        total
    }

    fn average_at_limit_calc(
        &self,
        idx: &[usize],
        ls: &mut LocalState,
        limit: u64,
    ) -> RawAvgResult {
        ls.reset();
        for i in idx {
            let prev = ls.types;
            ls.feed_sample(&self.samples[*i]);
            match ls.size.cmp(&limit) {
                Ordering::Less => (),
                Ordering::Equal => {
                    return RawAvgResult {
                        types_low: ls.types,
                        types_high: ls.types,
                    }
                }
                Ordering::Greater => {
                    return RawAvgResult {
                        types_low: prev,
                        types_high: ls.types,
                    }
                }
            }
        }
        unreachable!();
    }

    fn compare_with_points_calc(
        &self,
        idx: &[usize],
        ls: &mut LocalState,
        points: &[Point],
        result: &mut [RawPointResult],
    ) {
        ls.reset();
        let mut j = 0;
        for i in idx {
            let prev = ls.types;
            ls.feed_sample(&self.samples[*i]);
            loop {
                let p = &points[j];
                if ls.size < p.size {
                    break;
                }
                if prev < p.types {
                    result[j].above += 1;
                } else if ls.types > p.types {
                    result[j].below += 1;
                }
                j += 1;
                if j == points.len() {
                    return;
                }
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

#[derive(Clone, Copy)]
pub struct AvgResult {
    pub types_low: f64,
    pub types_high: f64,
}

#[derive(Clone, Copy)]
struct RawAvgResult {
    types_low: u64,
    types_high: u64,
}

impl RawAvgResult {
    fn new() -> RawAvgResult {
        RawAvgResult {
            types_low: 0,
            types_high: 0,
        }
    }
    fn add(&mut self, other: &RawAvgResult) {
        self.types_low += other.types_low;
        self.types_high += other.types_high;
    }
}

#[derive(Clone, Copy)]
pub struct PointResult {
    pub above: f64,
    pub below: f64,
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
    fn add(&mut self, other: &RawPointResult) {
        self.above += other.above;
        self.below += other.below;
    }
}

use crossbeam_channel::TryRecvError;
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

pub fn average_at_limit(samples: &[Sample], iter: u64, limit: u64) -> Result {
    Driver::new(samples).average_at_limit(iter, limit)
}

pub struct Driver<'a> {
    /// Input data.
    samples: &'a [Sample],
    /// All types have identifiers in `0..total_types`.
    total_types: usize,
}

impl Driver<'_> {
    pub fn new(samples: &[Sample]) -> Driver {
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

    pub fn average_at_limit(&self, iter: u64, limit: u64) -> Result {
        let (s1, r1) = crossbeam_channel::unbounded();
        for job in 0..RANDOM_JOBS {
            s1.send(job).expect("send succeeds");
        }
        let iter_per_job = (iter + RANDOM_JOBS - 1) / RANDOM_JOBS;
        let iter = iter_per_job * RANDOM_JOBS;
        drop(s1);
        let nthreads = num_cpus::get();
        let mut total = RawResult::new();
        trace!("randomized, {RANDOM_JOBS} jobs, {nthreads} threads");
        thread::scope(|scope| {
            let (s2, r2) = crossbeam_channel::unbounded();
            for _ in 0..nthreads {
                let r1 = r1.clone();
                let s2 = s2.clone();
                scope.spawn(move || {
                    let mut thread_total = RawResult::new();
                    loop {
                        match r1.try_recv() {
                            Ok(job) => {
                                thread_total.add(&self.count_job(job, iter_per_job, limit));
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
        Result {
            types_low: total.types_low as f64 / iter as f64,
            types_high: total.types_high as f64 / iter as f64,
        }
    }

    fn count_job(&self, job: u64, iter_per_job: u64, limit: u64) -> RawResult {
        let n = self.samples.len();
        let mut idx = vec![0; n];
        for (i, v) in idx.iter_mut().enumerate() {
            *v = i;
        }
        let mut ls = LocalState::new(self.total_types);
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
        let mut total = RawResult::new();
        for _ in 0..iter_per_job {
            idx.shuffle(&mut rng);
            total.add(&self.calc(&idx, &mut ls, limit));
        }
        total
    }

    fn calc(&self, idx: &[usize], ls: &mut LocalState, limit: u64) -> RawResult {
        ls.reset();
        for i in idx {
            let prev = ls.types;
            ls.feed_sample(&self.samples[*i]);
            match ls.size.cmp(&limit) {
                Ordering::Less => (),
                Ordering::Equal => {
                    return RawResult {
                        types_low: ls.types,
                        types_high: ls.types,
                    }
                }
                Ordering::Greater => {
                    return RawResult {
                        types_low: prev,
                        types_high: ls.types,
                    }
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

pub struct Result {
    pub types_low: f64,
    pub types_high: f64,
}

struct RawResult {
    types_low: u64,
    types_high: u64,
}

impl RawResult {
    fn new() -> RawResult {
        RawResult {
            types_low: 0,
            types_high: 0,
        }
    }
    fn add(&mut self, other: &RawResult) {
        self.types_low += other.types_low;
        self.types_high += other.types_high;
    }
}

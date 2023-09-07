use crossbeam_channel::TryRecvError;
use log::trace;
use std::thread;

/// Number of parallel tasks.
const RANDOM_JOBS: u64 = 1000;

#[derive(Clone, Copy)]
pub struct Job {
    pub job_id: u64,
    pub iter_per_job: u64,
}

pub trait ParResult {
    fn add(&mut self, other: Self);
}

pub fn compute_parallel<TParResult, TBuilder, TRunner>(
    builder: TBuilder,
    runner: TRunner,
    iter: u64,
) -> (TParResult, u64)
where
    TParResult: ParResult + Send,
    TBuilder: Fn() -> TParResult + Send + Copy,
    TRunner: Fn(Job, &mut TParResult) + Send + Copy,
{
    let (s1, r1) = crossbeam_channel::unbounded();
    for job in 0..RANDOM_JOBS {
        s1.send(job).unwrap();
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
                        Ok(job_id) => {
                            runner(
                                Job {
                                    job_id,
                                    iter_per_job,
                                },
                                &mut thread_total,
                            );
                        }
                        Err(TryRecvError::Empty) => unreachable!(),
                        Err(TryRecvError::Disconnected) => break,
                    }
                }
                s2.send(thread_total).unwrap();
            });
        }
        drop(s2);
        while let Ok(thread_total) = r2.recv() {
            total.add(thread_total);
        }
    });
    (total, iter)
}

#[cfg(test)]
mod test {
    use super::*;

    struct Adder {
        x: u64,
        y: u64,
    }

    impl ParResult for Adder {
        fn add(&mut self, other: Self) {
            self.x += other.x;
            self.y += other.y;
        }
    }

    #[test]
    fn compute_parallel_basic() {
        let (r, iter) = compute_parallel(
            || Adder { x: 0, y: 0 },
            |job, adder| {
                assert!(job.job_id < RANDOM_JOBS);
                assert_eq!(job.iter_per_job, 100);
                adder.x += 1;
                adder.y += job.job_id;
            },
            100 * RANDOM_JOBS,
        );
        assert_eq!(iter, 100 * RANDOM_JOBS);
        assert_eq!(r.x, RANDOM_JOBS);
        assert_eq!(r.y, RANDOM_JOBS * (RANDOM_JOBS - 1) / 2);
    }

    #[test]
    fn compute_parallel_small() {
        assert!(5 < RANDOM_JOBS);
        let (r, iter) = compute_parallel(
            || Adder { x: 0, y: 0 },
            |job, adder| {
                assert!(job.job_id < RANDOM_JOBS);
                assert_eq!(job.iter_per_job, 1);
                adder.x += 1;
                adder.y += job.job_id;
            },
            5,
        );
        assert_eq!(iter, RANDOM_JOBS);
        assert_eq!(r.x, RANDOM_JOBS);
        assert_eq!(r.y, RANDOM_JOBS * (RANDOM_JOBS - 1) / 2);
    }
}

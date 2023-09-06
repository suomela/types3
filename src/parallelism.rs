use crossbeam_channel::TryRecvError;
use log::trace;
use std::thread;

/// Number of parallel tasks.
const RANDOM_JOBS: u64 = 1000;

pub trait RawResult {
    fn add(&mut self, other: Self);
}

pub fn compute_parallel<TRawResult, TBuilder, TRunner>(
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
                        Ok(job) => {
                            runner(job, iter_per_job, &mut thread_total);
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

    impl RawResult for Adder {
        fn add(&mut self, other: Self) {
            self.x += other.x;
            self.y += other.y;
        }
    }

    #[test]
    fn compute_parallel_basic() {
        let (r, iter) = compute_parallel(
            || Adder { x: 0, y: 0 },
            |i, iter_per_job, adder| {
                assert!(i < RANDOM_JOBS);
                assert_eq!(iter_per_job, 100);
                adder.x += 1;
                adder.y += i;
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
            |i, iter_per_job, adder| {
                assert!(i < RANDOM_JOBS);
                assert_eq!(iter_per_job, 1);
                adder.x += 1;
                adder.y += i;
            },
            5,
        );
        assert_eq!(iter, RANDOM_JOBS);
        assert_eq!(r.x, RANDOM_JOBS);
        assert_eq!(r.y, RANDOM_JOBS * (RANDOM_JOBS - 1) / 2);
    }
}

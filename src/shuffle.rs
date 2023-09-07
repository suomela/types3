use crate::parallelism::RawResult;
use rand::seq::SliceRandom;
use rand_xoshiro::rand_core::SeedableRng;
use rand_xoshiro::Xoshiro256PlusPlus;

pub fn shuffle_job<TRawResult, TCalcOne>(
    mut calc_one: TCalcOne,
    n: usize,
    job: u64,
    iter_per_job: u64,
    result: &mut TRawResult,
) where
    TRawResult: RawResult,
    TCalcOne: FnMut(&[usize], &mut TRawResult),
{
    let mut idx = vec![0; n];
    for (i, v) in idx.iter_mut().enumerate() {
        *v = i;
    }
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(job);
    for _ in 0..iter_per_job {
        idx.shuffle(&mut rng);
        calc_one(&idx, result);
    }
}

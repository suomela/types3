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

#[derive(PartialEq, Eq, Debug)]
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::calculation::SToken;

    #[test]
    fn calc_one_tokens_1() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let idx = vec![0, 1];
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 0, high: 10 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1233, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 0, high: 10 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1234, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 10, high: 10 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1235, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 10, high: 15 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1234 + 5678, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 15, high: 15 });
        }
    }

    #[test]
    fn calc_one_tokens_2() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let idx = vec![1, 0];
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 1, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 0, high: 5 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 5677, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 0, high: 5 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 5678, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 5, high: 5 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 5679, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 5, high: 15 });
        }
        {
            let mut result = AvgParResult { low: 0, high: 0 };
            calc_one(&samples, 5678 + 1234, &idx, &mut counter, &mut result);
            assert_eq!(result, AvgParResult { low: 15, high: 15 });
        }
    }

    #[test]
    fn calc_one_tokens_3() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let mut result = AvgParResult { low: 0, high: 0 };
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 15 });
        let idx = vec![1, 0];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 20 });
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 20, high: 35 });
    }

    #[test]
    #[should_panic(expected = "unreachable")]
    fn calc_one_tokens_fail() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let idx = vec![0, 1];
        let mut result = AvgParResult { low: 0, high: 0 };
        calc_one(&samples, 1234 + 5678 + 1, &idx, &mut counter, &mut result);
    }

    #[test]
    fn calc_one_types_1() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let mut result = AvgParResult { low: 0, high: 0 };
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 1, high: 1 });
        let idx = vec![1, 0];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 1, high: 2 });
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 2, high: 3 });
    }

    #[test]
    fn calc_one_types_2() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 1, count: 5 }],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let mut result = AvgParResult { low: 0, high: 0 };
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 1, high: 2 });
        let idx = vec![1, 0];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 1, high: 3 });
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 2, high: 5 });
    }

    #[test]
    fn calc_one_types_3() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![
                    SToken { id: 0, count: 1 },
                    SToken { id: 1, count: 1 },
                    SToken { id: 2, count: 1 },
                    SToken { id: 3, count: 1 },
                    SToken { id: 4, count: 1 },
                    SToken { id: 5, count: 1 },
                    SToken { id: 6, count: 1 },
                    SToken { id: 7, count: 1 },
                    SToken { id: 8, count: 1 },
                    SToken { id: 9, count: 1 },
                ],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![
                    SToken { id: 10, count: 1 },
                    SToken { id: 11, count: 1 },
                    SToken { id: 12, count: 1 },
                    SToken { id: 13, count: 1 },
                    SToken { id: 14, count: 1 },
                ],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let mut result = AvgParResult { low: 0, high: 0 };
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 15 });
        let idx = vec![1, 0];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 20 });
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 20, high: 35 });
    }

    #[test]
    fn calc_one_types_4() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![
                    SToken { id: 0, count: 1 },
                    SToken { id: 1, count: 1 },
                    SToken { id: 2, count: 1 },
                    SToken { id: 3, count: 1 },
                    SToken { id: 4, count: 1 },
                    SToken { id: 5, count: 1 },
                    SToken { id: 6, count: 1 },
                    SToken { id: 7, count: 1 },
                    SToken { id: 8, count: 1 },
                    SToken { id: 9, count: 1 },
                ],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![
                    SToken { id: 0, count: 1 },
                    SToken { id: 1, count: 1 },
                    SToken { id: 2, count: 1 },
                    SToken { id: 3, count: 1 },
                    SToken { id: 4, count: 1 },
                ],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let mut result = AvgParResult { low: 0, high: 0 };
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 10 });
        let idx = vec![1, 0];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 10, high: 15 });
        let idx = vec![0, 1];
        calc_one(&samples, 2000, &idx, &mut counter, &mut result);
        assert_eq!(result, AvgParResult { low: 20, high: 25 });
    }
}

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
        let c = counter.feed_sample(&samples[*i]);
        match c.x.cmp(&limit) {
            Ordering::Less => (),
            Ordering::Equal => {
                result.low += c.y;
                result.high += c.y;
                return;
            }
            Ordering::Greater => {
                result.low += c.low_y;
                result.high += c.high_y;
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

    #[test]
    fn average_at_limit_tokens_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let iter = 10000;
        let result = average_at_limit(MeasureY::Tokens, &samples, iter, 1000);
        assert_eq!(result.iter, iter);
        assert_eq!(result.low, 0 * iter);
        assert_eq!(result.high, 10 * iter);
    }

    #[test]
    fn average_at_limit_tokens_2() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let iter = 10000;
        let result = average_at_limit(MeasureY::Tokens, &samples, iter, 1234);
        assert_eq!(result.iter, iter);
        assert_eq!(result.low, 10 * iter);
        assert_eq!(result.high, 10 * iter);
    }

    #[test]
    fn average_at_limit_tokens_3() {
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
        let iter = 10000;
        let result = average_at_limit(MeasureY::Tokens, &samples, iter, 2000);
        let fiter = iter as f64;
        let expect_low = 10.0 * fiter / 2.0 + 0.0 * fiter / 2.0;
        let expect_high = 15.0 * fiter / 2.0 + 5.0 * fiter / 2.0;
        let tolerance = 0.1;
        assert_eq!(result.iter, iter);
        assert!(result.low as f64 >= (1.0 - tolerance) * expect_low);
        assert!(result.low as f64 <= (1.0 + tolerance) * expect_low);
        assert!(result.high as f64 >= (1.0 - tolerance) * expect_high);
        assert!(result.high as f64 <= (1.0 + tolerance) * expect_high);
    }

    #[test]
    #[should_panic(expected = "thread panicked")]
    fn average_at_limit_tokens_fail() {
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
        let iter = 10000;
        let _result = average_at_limit(MeasureY::Tokens, &samples, iter, 1234 + 5678 + 1);
    }

    #[test]
    fn average_at_limit_types_1() {
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
        let iter = 10000;
        let result = average_at_limit(MeasureY::Types, &samples, iter, 2000);
        let fiter = iter as f64;
        let expect_low = 1.0 * fiter / 2.0 + 0.0 * fiter / 2.0;
        let expect_high = 1.0 * fiter / 2.0 + 1.0 * fiter / 2.0;
        let tolerance = 0.1;
        assert_eq!(result.iter, iter);
        assert!(result.low as f64 >= (1.0 - tolerance) * expect_low);
        assert!(result.low as f64 <= (1.0 + tolerance) * expect_low);
        assert!(result.high as f64 >= (1.0 - tolerance) * expect_high);
        assert!(result.high as f64 <= (1.0 + tolerance) * expect_high);
    }

    #[test]
    fn average_at_limit_types_2() {
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
        let iter = 10000;
        let result = average_at_limit(MeasureY::Types, &samples, iter, 2000);
        let fiter = iter as f64;
        let expect_low = 1.0 * fiter / 2.0 + 0.0 * fiter / 2.0;
        let expect_high = 2.0 * fiter / 2.0 + 1.0 * fiter / 2.0;
        let tolerance = 0.1;
        assert_eq!(result.iter, iter);
        assert!(result.low as f64 >= (1.0 - tolerance) * expect_low);
        assert!(result.low as f64 <= (1.0 + tolerance) * expect_low);
        assert!(result.high as f64 >= (1.0 - tolerance) * expect_high);
        assert!(result.high as f64 <= (1.0 + tolerance) * expect_high);
    }

    #[test]
    fn average_at_limit_types_3() {
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
        let iter = 10000;
        let result = average_at_limit(MeasureY::Types, &samples, iter, 2000);
        let fiter = iter as f64;
        let expect_low = 10.0 * fiter / 2.0 + 0.0 * fiter / 2.0;
        let expect_high = 15.0 * fiter / 2.0 + 5.0 * fiter / 2.0;
        let tolerance = 0.1;
        assert_eq!(result.iter, iter);
        assert!(result.low as f64 >= (1.0 - tolerance) * expect_low);
        assert!(result.low as f64 <= (1.0 + tolerance) * expect_low);
        assert!(result.high as f64 >= (1.0 - tolerance) * expect_high);
        assert!(result.high as f64 <= (1.0 + tolerance) * expect_high);
    }

    #[test]
    fn average_at_limit_types_4() {
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
        let iter = 10000;
        let result = average_at_limit(MeasureY::Types, &samples, iter, 2000);
        let fiter = iter as f64;
        let expect_low = 10.0 * fiter / 2.0 + 0.0 * fiter / 2.0;
        let expect_high = 10.0 * fiter / 2.0 + 5.0 * fiter / 2.0;
        let tolerance = 0.1;
        assert_eq!(result.iter, iter);
        assert!(result.low as f64 >= (1.0 - tolerance) * expect_low);
        assert!(result.low as f64 <= (1.0 + tolerance) * expect_low);
        assert!(result.high as f64 >= (1.0 - tolerance) * expect_high);
        assert!(result.high as f64 <= (1.0 + tolerance) * expect_high);
    }
}

use crate::calculation::{self, Sample};
use crate::counter::{
    self, Counter, HapaxCounter, SampleCounter, TokenCounter, TypeCounter, TypeRatioCounter,
};
use crate::output::{MeasureY, PointResult};
use crate::parallelism::{self, ParResult};
use crate::shuffle;
use is_sorted::IsSorted;
use itertools::Itertools;
use std::cmp::Ordering;

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
        MeasureY::Hapaxes => do_count::<HapaxCounter>(samples, iter, points),
        MeasureY::Samples => do_count::<SampleCounter>(samples, iter, points),
        MeasureY::MarkedTypes => do_count::<TypeRatioCounter>(samples, iter, points),
    }
}

fn do_count<TCounter>(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult>
where
    TCounter: Counter,
{
    calculation::verify_samples(samples);
    assert!(!points.is_empty());
    assert!(IsSorted::is_sorted(&mut points.iter()));
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
    while points[j].x == 0 {
        if points[j].y > 0 {
            result.elems[j].above += 1;
        }
        j += 1;
        if j == points.len() {
            return;
        }
    }
    for i in idx {
        let c = counter.feed_sample(&samples[*i]);
        loop {
            let p = &points[j];
            match c.x.cmp(&p.x) {
                Ordering::Less => break,
                Ordering::Equal =>
                {
                    #[allow(clippy::comparison_chain)]
                    if c.y < p.y {
                        result.elems[j].above += 1;
                    } else if c.y > p.y {
                        result.elems[j].below += 1;
                    }
                }
                Ordering::Greater => {
                    if c.high_y < p.y {
                        result.elems[j].above += 1;
                    } else if c.low_y > p.y {
                        result.elems[j].below += 1;
                    }
                }
            }
            j += 1;
            if j == points.len() {
                return;
            }
        }
    }
    unreachable!();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::calculation::SToken;

    const TOLERANCE: f64 = 0.02;
    const T1: f64 = 1.0 - TOLERANCE;
    const T2: f64 = 1.0 + TOLERANCE;
    const ITER: u64 = 100000;
    const FITER: f64 = ITER as f64;

    fn st(id: usize, count: u64) -> SToken {
        SToken {
            id,
            count,
            marked_count: 0,
        }
    }

    fn stm(id: usize, count: u64, marked_count: u64) -> SToken {
        SToken {
            id,
            count,
            marked_count,
        }
    }

    fn p(x: u64, y: u64) -> Point {
        Point { x, y }
    }

    fn pr(above: u64, below: u64, iter: u64) -> PointResult {
        PointResult { above, below, iter }
    }

    #[test]
    fn calc_one_tokens_1() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(0, 0),
            p(1, 0),
            p(1233, 0),
            p(1234, 0),
            p(1235, 0),
            p(1234 + 5678, 0),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 0, below: 0 }, // 0
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 1 }, // 1234
                    PointParResultElem { above: 0, below: 1 }, // 1235
                    PointParResultElem { above: 0, below: 1 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_2() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(0, 7),
            p(1, 7),
            p(1233, 7),
            p(1234, 7),
            p(1235, 7),
            p(1234 + 5678, 7),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 0
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 1 }, // 1234
                    PointParResultElem { above: 0, below: 1 }, // 1235
                    PointParResultElem { above: 0, below: 1 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_3() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(0, 10),
            p(1, 10),
            p(1233, 10),
            p(1234, 10),
            p(1235, 10),
            p(1234 + 5678, 10),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 0
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 0 }, // 1234
                    PointParResultElem { above: 0, below: 0 }, // 1235
                    PointParResultElem { above: 0, below: 1 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_4() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(0, 11),
            p(1, 11),
            p(1233, 11),
            p(1234, 11),
            p(1235, 11),
            p(1234 + 5678, 11),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 0
                    PointParResultElem { above: 1, below: 0 }, // 1
                    PointParResultElem { above: 1, below: 0 }, // 1233
                    PointParResultElem { above: 1, below: 0 }, // 1234
                    PointParResultElem { above: 0, below: 0 }, // 1235
                    PointParResultElem { above: 0, below: 1 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_5() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(0, 15),
            p(1, 15),
            p(1233, 15),
            p(1234, 15),
            p(1235, 15),
            p(1234 + 5678, 15),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 0
                    PointParResultElem { above: 1, below: 0 }, // 1
                    PointParResultElem { above: 1, below: 0 }, // 1233
                    PointParResultElem { above: 1, below: 0 }, // 1234
                    PointParResultElem { above: 0, below: 0 }, // 1235
                    PointParResultElem { above: 0, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_6() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 16),
            p(1233, 16),
            p(1234, 16),
            p(1235, 16),
            p(1234 + 5678, 16),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 1
                    PointParResultElem { above: 1, below: 0 }, // 1233
                    PointParResultElem { above: 1, below: 0 }, // 1234
                    PointParResultElem { above: 1, below: 0 }, // 1235
                    PointParResultElem { above: 1, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_7() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 7),
            p(1233, 7),
            p(1234, 7),
            p(1235, 7),
            p(1234 + 5678, 16),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 1 }, // 1234
                    PointParResultElem { above: 0, below: 1 }, // 1235
                    PointParResultElem { above: 1, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_tokens_8() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 7),
            p(1233, 7),
            p(1234, 7),
            p(1235, 7),
            p(1234 + 5678, 16),
        ];
        let idx = vec![1, 0];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 1
                    PointParResultElem { above: 1, below: 0 }, // 1233
                    PointParResultElem { above: 1, below: 0 }, // 1234
                    PointParResultElem { above: 1, below: 0 }, // 1235
                    PointParResultElem { above: 1, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    #[should_panic(expected = "unreachable")]
    fn calc_one_tokens_fail_1() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 7),
            p(1233, 7),
            p(1234, 7),
            p(1235, 7),
            p(1234 + 5678, 16),
            p(1234 + 5678 + 1, 16),
        ];
        let idx = vec![1, 0];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
    }

    #[test]
    fn calc_one_types_1() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(0, 5)],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 0),
            p(1233, 0),
            p(1234, 0),
            p(1235, 0),
            p(1234 + 5678, 2),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 1 }, // 1234
                    PointParResultElem { above: 0, below: 1 }, // 1235
                    PointParResultElem { above: 1, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_types_2() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(1, 5)],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 0),
            p(1233, 0),
            p(1234, 0),
            p(1235, 0),
            p(1234 + 5678, 2),
        ];
        let idx = vec![0, 1];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 0, below: 0 }, // 1
                    PointParResultElem { above: 0, below: 0 }, // 1233
                    PointParResultElem { above: 0, below: 1 }, // 1234
                    PointParResultElem { above: 0, below: 1 }, // 1235
                    PointParResultElem { above: 0, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn calc_one_types_3() {
        let samples = vec![
            Sample {
                x: 1234,
                token_count: 10,
                tokens: vec![
                    st(0, 1),
                    st(1, 1),
                    st(2, 1),
                    st(3, 1),
                    st(4, 1),
                    st(5, 1),
                    st(6, 1),
                    st(7, 1),
                    st(8, 1),
                    st(9, 1),
                ],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![st(10, 1), st(11, 1), st(12, 1), st(13, 1), st(14, 1)],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let points = vec![
            p(1, 7),
            p(1233, 7),
            p(1234, 7),
            p(1235, 7),
            p(1234 + 5678, 16),
        ];
        let idx = vec![1, 0];
        let mut result = PointParResult {
            elems: vec![PointParResultElem { above: 0, below: 0 }; points.len()],
        };
        calc_one(&samples, &points, &idx, &mut counter, &mut result);
        assert_eq!(
            result,
            PointParResult {
                elems: vec![
                    PointParResultElem { above: 1, below: 0 }, // 1
                    PointParResultElem { above: 1, below: 0 }, // 1233
                    PointParResultElem { above: 1, below: 0 }, // 1234
                    PointParResultElem { above: 1, below: 0 }, // 1235
                    PointParResultElem { above: 1, below: 0 }, // 1234 + 5678
                ]
            }
        );
    }

    #[test]
    fn compare_with_points_tokens_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 7), p(1233, 7), p(1234, 7)];
        let result = compare_with_points(MeasureY::Tokens, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![pr(0, 0, ITER), pr(0, 0, ITER), pr(0, ITER, ITER),]
        );
    }

    #[test]
    fn compare_with_points_tokens_2() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 11), p(1233, 11), p(1234, 11)];
        let result = compare_with_points(MeasureY::Tokens, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![pr(ITER, 0, ITER), pr(ITER, 0, ITER), pr(ITER, 0, ITER),]
        );
    }

    #[test]
    #[should_panic(expected = "is_sorted")]
    fn compare_with_points_tokens_fail_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 11), p(1234, 11), p(1233, 11)];
        let _result = compare_with_points(MeasureY::Tokens, &samples, ITER, &points);
    }

    #[test]
    #[should_panic(expected = "thread panicked")]
    fn compare_with_points_tokens_fail_2() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 11), p(1233, 11), p(1235, 11)];
        let _result = compare_with_points(MeasureY::Tokens, &samples, ITER, &points);
    }

    #[test]
    fn compare_with_points_types_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 2), p(1233, 2), p(1234, 2)];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![pr(ITER, 0, ITER), pr(ITER, 0, ITER), pr(ITER, 0, ITER),]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![st(0, 10)],
        }];
        let points = vec![p(1, 2), p(1233, 2), p(1234, 2)];
        let result = compare_with_points(MeasureY::Hapaxes, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![pr(ITER, 0, ITER), pr(ITER, 0, ITER), pr(ITER, 0, ITER),]
        );
    }

    #[test]
    fn compare_with_points_types_2() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 4),
            p(122, 4),
            p(123, 4),
            p(124, 4),
            p(245, 4),
            p(246, 4),
            p(247, 4),
            p(368, 4),
            p(369, 4),
        ];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_2() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 4),
            p(122, 4),
            p(123, 4),
            p(124, 4),
            p(245, 4),
            p(246, 4),
            p(247, 4),
            p(368, 4),
            p(369, 4),
        ];
        let result = compare_with_points(MeasureY::Hapaxes, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_types_3() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 2),
            p(122, 2),
            p(123, 2),
            p(124, 2),
            p(245, 2),
            p(246, 2),
            p(247, 2),
            p(368, 2),
            p(369, 2),
        ];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, ITER, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_3a() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(0, 1)],
            },
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(1, 1)],
            },
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(2, 1)],
            },
        ];
        let points = vec![
            p(1, 2),
            p(122, 2),
            p(123, 2),
            p(124, 2),
            p(245, 2),
            p(246, 2),
            p(247, 2),
            p(368, 2),
            p(369, 2),
        ];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, ITER, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_3b() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 2),
            p(122, 2),
            p(123, 2),
            p(124, 2),
            p(245, 2),
            p(246, 2),
            p(247, 2),
            p(368, 2),
            p(369, 2),
        ];
        let result = compare_with_points(MeasureY::Hapaxes, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
                pr(ITER, 0, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_types_4() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 1),
            p(122, 1),
            p(123, 1),
            p(124, 1),
            p(245, 1),
            p(246, 1),
            p(247, 1),
            p(368, 1),
            p(369, 1),
        ];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_4a() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(0, 1)],
            },
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(1, 1)],
            },
            Sample {
                x: 123,
                token_count: 1,
                tokens: vec![st(2, 1)],
            },
        ];
        let points = vec![
            p(1, 1),
            p(122, 1),
            p(123, 1),
            p(124, 1),
            p(245, 1),
            p(246, 1),
            p(247, 1),
            p(368, 1),
            p(369, 1),
        ];
        let result = compare_with_points(MeasureY::Hapaxes, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
                pr(0, ITER, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_hapaxes_4b() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![
            p(1, 1),
            p(122, 1),
            p(123, 1),
            p(124, 1),
            p(245, 1),
            p(246, 1),
            p(247, 1),
            p(368, 1),
            p(369, 1),
        ];
        let result = compare_with_points(MeasureY::Hapaxes, &samples, ITER, &points);
        assert_eq!(
            result,
            vec![
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(ITER, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(ITER, 0, ITER),
                pr(0, 0, ITER),
                pr(0, 0, ITER),
                pr(ITER, 0, ITER),
            ]
        );
    }

    #[test]
    fn compare_with_points_types_5() {
        let samples = vec![
            Sample {
                x: 100,
                token_count: 10,
                tokens: vec![st(0, 10)],
            },
            Sample {
                x: 200,
                token_count: 10,
                tokens: vec![st(1, 10)],
            },
            Sample {
                x: 100,
                token_count: 10,
                tokens: vec![st(2, 10)],
            },
        ];
        let points = vec![p(50, 1), p(150, 1), p(250, 1), p(350, 1)];
        let result = compare_with_points(MeasureY::Types, &samples, ITER, &points);
        let expected_below = FITER / 3.0;
        assert_eq!(result[0], pr(0, 0, ITER));
        assert_eq!(result[1], pr(0, 0, ITER));
        assert_eq!(result[2].above, 0);
        assert!(result[2].below as f64 >= T1 * expected_below);
        assert!(result[2].below as f64 <= T2 * expected_below);
        assert_eq!(result[3], pr(0, ITER, ITER));
    }

    #[test]
    fn compare_with_points_type_ratio_1() {
        let samples = vec![Sample {
            x: 0,
            token_count: 2,
            tokens: vec![st(0, 1), stm(1, 1, 1)],
        }];
        let points = vec![
            p(1, 0),
            p(1, 1),
            p(1, 2),
            p(1, 3),
            p(2, 0),
            p(2, 1),
            p(2, 2),
            p(2, 3),
        ];
        let result = compare_with_points(MeasureY::MarkedTypes, &samples, ITER, &points);
        assert_eq!(result[0], pr(0, 0, ITER));
        assert_eq!(result[1], pr(0, 0, ITER));
        assert_eq!(result[2], pr(ITER, 0, ITER));
        assert_eq!(result[3], pr(ITER, 0, ITER));
        assert_eq!(result[4], pr(0, ITER, ITER));
        assert_eq!(result[5], pr(0, 0, ITER));
        assert_eq!(result[6], pr(ITER, 0, ITER));
        assert_eq!(result[7], pr(ITER, 0, ITER));
    }

    #[test]
    fn compare_with_points_type_ratio_2() {
        let samples = vec![Sample {
            x: 0,
            token_count: 7,
            tokens: vec![
                stm(0, 1, 1),
                stm(1, 1, 1),
                stm(2, 1, 1),
                st(3, 1),
                st(4, 1),
                st(5, 1),
                st(6, 1),
            ],
        }];
        let points = vec![
            p(6, 0),
            p(6, 1),
            p(6, 2),
            p(6, 3),
            p(6, 4),
            p(7, 0),
            p(7, 1),
            p(7, 2),
            p(7, 3),
            p(7, 4),
        ];
        let result = compare_with_points(MeasureY::MarkedTypes, &samples, ITER, &points);
        assert_eq!(result[0], pr(0, 0, ITER));
        assert_eq!(result[1], pr(0, 0, ITER));
        assert_eq!(result[2], pr(0, 0, ITER));
        assert_eq!(result[3], pr(0, 0, ITER));
        assert_eq!(result[4], pr(ITER, 0, ITER));
        assert_eq!(result[5], pr(0, ITER, ITER));
        assert_eq!(result[6], pr(0, ITER, ITER));
        assert_eq!(result[7], pr(0, ITER, ITER));
        assert_eq!(result[8], pr(0, 0, ITER));
        assert_eq!(result[9], pr(ITER, 0, ITER));
    }

    #[test]
    fn compare_with_points_type_ratio_3() {
        let samples = vec![
            Sample {
                x: 0,
                token_count: 1,
                tokens: vec![st(0, 1)],
            },
            Sample {
                x: 0,
                token_count: 1,
                tokens: vec![stm(1, 1, 1)],
            },
        ];
        let points = vec![
            p(1, 0),
            p(1, 1),
            p(1, 2),
            p(1, 3),
            p(2, 0),
            p(2, 1),
            p(2, 2),
            p(2, 3),
        ];
        let result = compare_with_points(MeasureY::MarkedTypes, &samples, ITER, &points);
        assert!(result[0].above as f64 >= T1 * 0.0 * FITER);
        assert!(result[0].above as f64 <= T2 * 0.0 * FITER);
        assert!(result[0].below as f64 >= T1 * 0.5 * FITER);
        assert!(result[0].below as f64 <= T2 * 0.5 * FITER);
        assert!(result[1].above as f64 >= T1 * 0.5 * FITER);
        assert!(result[1].above as f64 <= T2 * 0.5 * FITER);
        assert!(result[1].below as f64 >= T1 * 0.0 * FITER);
        assert!(result[1].below as f64 <= T2 * 0.0 * FITER);
        assert!(result[2].above as f64 >= T1 * 1.0 * FITER);
        assert!(result[2].above as f64 <= T2 * 1.0 * FITER);
        assert!(result[2].below as f64 >= T1 * 0.0 * FITER);
        assert!(result[2].below as f64 <= T2 * 0.0 * FITER);
        assert!(result[3].above as f64 >= T1 * 1.0 * FITER);
        assert!(result[3].above as f64 <= T2 * 1.0 * FITER);
        assert!(result[3].below as f64 >= T1 * 0.0 * FITER);
        assert!(result[3].below as f64 <= T2 * 0.0 * FITER);
        assert_eq!(result[4], pr(0, ITER, ITER));
        assert_eq!(result[5], pr(0, 0, ITER));
        assert_eq!(result[6], pr(ITER, 0, ITER));
        assert_eq!(result[7], pr(ITER, 0, ITER));
    }

    #[test]
    fn compare_with_points_type_ratio_4() {
        let mut samples = vec![];
        let mut tokens = vec![];
        for i in 0..100 {
            if i == 0 {
                tokens.push(stm(i, 1, 1));
            } else {
                tokens.push(st(i, 1));
            }
        }
        samples.push(Sample {
            x: 0,
            token_count: 100,
            tokens,
        });
        let mut tokens = vec![];
        for i in 100..300 {
            if i == 100 {
                tokens.push(stm(i, 1, 1));
            } else {
                tokens.push(st(i, 1));
            }
        }
        samples.push(Sample {
            x: 0,
            token_count: 200,
            tokens,
        });
        let mut tokens = vec![];
        for i in 300..400 {
            if i == 300 {
                tokens.push(stm(i, 1, 1));
            } else {
                tokens.push(st(i, 1));
            }
        }
        samples.push(Sample {
            x: 0,
            token_count: 100,
            tokens,
        });
        let points = vec![p(50, 1), p(150, 1), p(250, 1), p(350, 1)];
        let result = compare_with_points(MeasureY::MarkedTypes, &samples, ITER, &points);
        let expected_below = FITER / 3.0;
        assert_eq!(result[0], pr(0, 0, ITER));
        assert_eq!(result[1], pr(0, 0, ITER));
        assert_eq!(result[2].above, 0);
        assert!(result[2].below as f64 >= T1 * expected_below);
        assert!(result[2].below as f64 <= T2 * expected_below);
        assert_eq!(result[3], pr(0, ITER, ITER));
    }
}

use crate::calculation::Sample;
use crate::counter::{self, Counter, HapaxCounter, TokenCounter, TypeCounter};
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
    }
}

fn do_count<TCounter>(samples: &[Sample], iter: u64, points: &[Point]) -> Vec<PointResult>
where
    TCounter: Counter,
{
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
        let points = vec![
            Point { x: 1, y: 0 },
            Point { x: 1233, y: 0 },
            Point { x: 1234, y: 0 },
            Point { x: 1235, y: 0 },
            Point {
                x: 1234 + 5678,
                y: 0,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
            Point { x: 1235, y: 7 },
            Point {
                x: 1234 + 5678,
                y: 7,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 10 },
            Point { x: 1233, y: 10 },
            Point { x: 1234, y: 10 },
            Point { x: 1235, y: 10 },
            Point {
                x: 1234 + 5678,
                y: 10,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 11 },
            Point { x: 1233, y: 11 },
            Point { x: 1234, y: 11 },
            Point { x: 1235, y: 11 },
            Point {
                x: 1234 + 5678,
                y: 11,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 15 },
            Point { x: 1233, y: 15 },
            Point { x: 1234, y: 15 },
            Point { x: 1235, y: 15 },
            Point {
                x: 1234 + 5678,
                y: 15,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 16 },
            Point { x: 1233, y: 16 },
            Point { x: 1234, y: 16 },
            Point { x: 1235, y: 16 },
            Point {
                x: 1234 + 5678,
                y: 16,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
            Point { x: 1235, y: 7 },
            Point {
                x: 1234 + 5678,
                y: 16,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
            Point { x: 1235, y: 7 },
            Point {
                x: 1234 + 5678,
                y: 16,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TokenCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
            Point { x: 1235, y: 7 },
            Point {
                x: 1234 + 5678,
                y: 16,
            },
            Point {
                x: 1234 + 5678 + 1,
                y: 16,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 0, count: 5 }],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 0 },
            Point { x: 1233, y: 0 },
            Point { x: 1234, y: 0 },
            Point { x: 1235, y: 0 },
            Point {
                x: 1234 + 5678,
                y: 2,
            },
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
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 5678,
                token_count: 5,
                tokens: vec![SToken { id: 1, count: 5 }],
            },
        ];
        let mut counter = TypeCounter::new(counter::count_types(&samples));
        let points = vec![
            Point { x: 1, y: 0 },
            Point { x: 1233, y: 0 },
            Point { x: 1234, y: 0 },
            Point { x: 1235, y: 0 },
            Point {
                x: 1234 + 5678,
                y: 2,
            },
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
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
            Point { x: 1235, y: 7 },
            Point {
                x: 1234 + 5678,
                y: 16,
            },
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
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let points = vec![
            Point { x: 1, y: 7 },
            Point { x: 1233, y: 7 },
            Point { x: 1234, y: 7 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Tokens, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    fn compare_with_points_tokens_2() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let points = vec![
            Point { x: 1, y: 11 },
            Point { x: 1233, y: 11 },
            Point { x: 1234, y: 11 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Tokens, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    #[should_panic(expected = "is_sorted")]
    fn compare_with_points_tokens_fail_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let points = vec![
            Point { x: 1, y: 11 },
            Point { x: 1234, y: 11 },
            Point { x: 1233, y: 11 },
        ];
        let iter = 10000;
        let _result = compare_with_points(MeasureY::Tokens, &samples, iter, &points);
    }

    #[test]
    #[should_panic(expected = "thread panicked")]
    fn compare_with_points_tokens_fail_2() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let points = vec![
            Point { x: 1, y: 11 },
            Point { x: 1233, y: 11 },
            Point { x: 1235, y: 11 },
        ];
        let iter = 10000;
        let _result = compare_with_points(MeasureY::Tokens, &samples, iter, &points);
    }

    #[test]
    fn compare_with_points_types_1() {
        let samples = vec![Sample {
            x: 1234,
            token_count: 10,
            tokens: vec![SToken { id: 0, count: 10 }],
        }];
        let points = vec![
            Point { x: 1, y: 2 },
            Point { x: 1233, y: 2 },
            Point { x: 1234, y: 2 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Types, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    fn compare_with_points_types_2() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 1, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 2, count: 10 }],
            },
        ];
        let points = vec![
            Point { x: 1, y: 4 },
            Point { x: 122, y: 4 },
            Point { x: 123, y: 4 },
            Point { x: 124, y: 4 },
            Point { x: 245, y: 4 },
            Point { x: 246, y: 4 },
            Point { x: 247, y: 4 },
            Point { x: 368, y: 4 },
            Point { x: 369, y: 4 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Types, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    fn compare_with_points_types_3() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 1, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 2, count: 10 }],
            },
        ];
        let points = vec![
            Point { x: 1, y: 2 },
            Point { x: 122, y: 2 },
            Point { x: 123, y: 2 },
            Point { x: 124, y: 2 },
            Point { x: 245, y: 2 },
            Point { x: 246, y: 2 },
            Point { x: 247, y: 2 },
            Point { x: 368, y: 2 },
            Point { x: 369, y: 2 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Types, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: iter,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    fn compare_with_points_types_4() {
        let samples = vec![
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 1, count: 10 }],
            },
            Sample {
                x: 123,
                token_count: 10,
                tokens: vec![SToken { id: 2, count: 10 }],
            },
        ];
        let points = vec![
            Point { x: 1, y: 1 },
            Point { x: 122, y: 1 },
            Point { x: 123, y: 1 },
            Point { x: 124, y: 1 },
            Point { x: 245, y: 1 },
            Point { x: 246, y: 1 },
            Point { x: 247, y: 1 },
            Point { x: 368, y: 1 },
            Point { x: 369, y: 1 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Types, &samples, iter, &points);
        assert_eq!(
            result,
            vec![
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: 0,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
                PointResult {
                    above: 0,
                    below: iter,
                    iter: iter
                },
            ]
        );
    }

    #[test]
    fn compare_with_points_types_5() {
        let samples = vec![
            Sample {
                x: 100,
                token_count: 10,
                tokens: vec![SToken { id: 0, count: 10 }],
            },
            Sample {
                x: 200,
                token_count: 10,
                tokens: vec![SToken { id: 1, count: 10 }],
            },
            Sample {
                x: 100,
                token_count: 10,
                tokens: vec![SToken { id: 2, count: 10 }],
            },
        ];
        let points = vec![
            Point { x: 50, y: 1 },
            Point { x: 150, y: 1 },
            Point { x: 250, y: 1 },
            Point { x: 350, y: 1 },
        ];
        let iter = 10000;
        let result = compare_with_points(MeasureY::Types, &samples, iter, &points);
        assert_eq!(
            result[0],
            PointResult {
                above: 0,
                below: 0,
                iter: iter
            }
        );
        assert_eq!(
            result[1],
            PointResult {
                above: 0,
                below: 0,
                iter: iter
            }
        );
        assert_eq!(result[2].above, 0);
        assert!(result[2].below as f64 >= 0.31 * (iter as f64));
        assert!(result[2].below as f64 <= 0.35 * (iter as f64));
        assert_eq!(
            result[3],
            PointResult {
                above: 0,
                below: iter,
                iter: iter
            }
        );
    }
}

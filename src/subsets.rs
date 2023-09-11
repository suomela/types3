use crate::calc_point::Point;
use crate::calculation::{SToken, Sample};
use crate::categories::{self, Category};
use crate::counter;
use crate::errors::{self, Result};
use crate::output::{self, MeasureX, MeasureY, Years};
use crate::samples::CSample;
use itertools::Itertools;
use log::debug;
use std::collections::{HashMap, HashSet};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubsetKey<'a> {
    pub category: Category<'a>,
    pub period: Years,
}

impl SubsetKey<'_> {
    pub fn pretty(&self) -> String {
        match &self.category {
            None => output::pretty_period(&self.period),
            Some((k, v)) => format!("{}, {} = {}", output::pretty_period(&self.period), k, v),
        }
    }
}

pub struct Subset<'a> {
    pub category: Category<'a>,
    pub period: Years,
    pub samples: Vec<Sample>,
    pub total_x: u64,
    pub total_y: u64,
    pub points: HashSet<Point>,
}

impl<'a> Subset<'a> {
    pub fn pretty(&self) -> String {
        self.key().pretty()
    }

    pub fn key(&self) -> SubsetKey {
        SubsetKey {
            category: self.category,
            period: self.period,
        }
    }

    pub fn get_point(&self) -> Point {
        Point {
            x: self.total_x,
            y: self.total_y,
        }
    }

    pub fn get_parent_period(&self, years: Years) -> SubsetKey<'a> {
        SubsetKey {
            category: self.category,
            period: years,
        }
    }

    pub fn get_parent_category(&self) -> SubsetKey<'a> {
        assert!(self.category.is_some());
        SubsetKey {
            category: None,
            period: self.period,
        }
    }

    pub fn get_parents(&self, years: Years) -> Vec<SubsetKey<'a>> {
        match self.category {
            None => vec![self.get_parent_period(years)],
            Some(_) => vec![self.get_parent_period(years), self.get_parent_category()],
        }
    }
}

pub fn build_subset<'a>(
    measure_x: MeasureX,
    measure_y: MeasureY,
    samples: &[CSample<'a>],
    key: SubsetKey<'a>,
    split_samples: bool,
) -> Result<Subset<'a>> {
    let category = key.category;
    let period = key.period;
    let filter = |s: &&CSample| {
        period.0 <= s.year && s.year < period.1 && categories::matches(category, s.metadata)
    };
    let samples = samples.iter().filter(filter).collect_vec();

    let mut lemmas = HashSet::new();
    for s in &samples {
        for t in &s.tokens {
            lemmas.insert(t.token);
        }
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();
    let lemmamap: HashMap<&str, usize> = lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();
    let samples = if split_samples {
        assert!(measure_x != MeasureX::Words);
        let mut split = vec![];
        for s in samples {
            for t in &s.tokens {
                let token = SToken {
                    id: lemmamap[t.token],
                    count: 1,
                    marked_count: if t.marked { 1 } else { 0 },
                };
                split.push(Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![token],
                })
            }
        }
        split
    } else {
        #[derive(Clone, Copy)]
        struct TokenCount {
            count: u64,
            marked_count: u64,
        }
        samples
            .into_iter()
            .map(|s| {
                let mut tokencount = HashMap::new();
                for t in &s.tokens {
                    let id = lemmamap[t.token];
                    let e = tokencount.entry(id).or_insert(TokenCount {
                        count: 0,
                        marked_count: 0,
                    });
                    e.count += 1;
                    if t.marked {
                        e.marked_count += 1;
                    }
                }
                let mut tokens = tokencount
                    .iter()
                    .map(|(&id, &x)| SToken {
                        id,
                        count: x.count,
                        marked_count: x.marked_count,
                    })
                    .collect_vec();
                tokens.sort_by_key(|t| t.id);
                let token_count = tokens.iter().map(|t| t.count).sum();
                let x = match measure_x {
                    MeasureX::Tokens => token_count,
                    MeasureX::Words => s.words,
                    MeasureX::Types => 0,
                };
                Sample {
                    x,
                    token_count,
                    tokens,
                }
            })
            .collect_vec()
    };
    let (total_x, total_y) = counter::count_xy(measure_y, &samples);
    let s = Subset {
        category,
        period,
        samples,
        total_x,
        total_y,
        points: HashSet::new(),
    };
    debug!(
        "{}: {} samples, {} {} / {} {}",
        s.pretty(),
        s.samples.len(),
        s.total_y,
        measure_y,
        s.total_x,
        measure_x,
    );
    if total_x == 0 {
        return Err(errors::invalid_input(format!(
            "{}: zero-size subset",
            s.pretty()
        )));
    }
    Ok(s)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::samples::CToken;

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

    fn ct(token: &str) -> CToken {
        CToken {
            token,
            marked: false,
        }
    }

    fn ctm(token: &str) -> CToken {
        CToken {
            token,
            marked: true,
        }
    }

    fn meta(l: &[(&str, &str)]) -> HashMap<String, String> {
        let mut m = HashMap::new();
        for &(k, v) in l {
            m.insert(k.to_owned(), v.to_owned());
        }
        m
    }

    #[test]
    fn build_subsets_types_words_empty1() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1600),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![Sample {
                x: 1234,
                token_count: 0,
                tokens: vec![]
            }]
        );
        assert_eq!(r.total_x, 1234);
        assert_eq!(r.total_y, 0);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_words_empty2() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 1234,
                    token_count: 0,
                    tokens: vec![]
                },
                Sample {
                    x: 5678,
                    token_count: 0,
                    tokens: vec![]
                }
            ]
        );
        assert_eq!(r.total_x, 1234 + 5678);
        assert_eq!(r.total_y, 0);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_words_distinct() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("a"), ct("a"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 1234,
                    token_count: 3,
                    tokens: vec![st(0, 2), st(1, 1),]
                },
                Sample {
                    x: 5678,
                    token_count: 2,
                    tokens: vec![st(2, 1), st(3, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 1234 + 5678);
        assert_eq!(r.total_y, 4);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_words_basic() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 1234,
                    token_count: 3,
                    tokens: vec![st(0, 1), st(1, 2),]
                },
                Sample {
                    x: 5678,
                    token_count: 2,
                    tokens: vec![st(1, 1), st(2, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 1234 + 5678);
        assert_eq!(r.total_y, 3);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_tokens_basic() {
        let my = MeasureY::Types;
        let mx = MeasureX::Tokens;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 3,
                    token_count: 3,
                    tokens: vec![st(0, 1), st(1, 2),]
                },
                Sample {
                    x: 2,
                    token_count: 2,
                    tokens: vec![st(1, 1), st(2, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 3 + 2);
        assert_eq!(r.total_y, 3);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_tokens_words_basic() {
        let my = MeasureY::Tokens;
        let mx = MeasureX::Words;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 1234,
                    token_count: 3,
                    tokens: vec![st(0, 1), st(1, 2),]
                },
                Sample {
                    x: 5678,
                    token_count: 2,
                    tokens: vec![st(1, 1), st(2, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 1234 + 5678);
        assert_eq!(r.total_y, 3 + 2);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_tokens_split() {
        let my = MeasureY::Types;
        let mx = MeasureX::Tokens;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, true).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![st(1, 1),]
                },
                Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![st(1, 1),]
                },
                Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![st(0, 1),]
                },
                Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![st(1, 1),]
                },
                Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![st(2, 1),]
                },
            ]
        );
        assert_eq!(r.total_x, 3 + 2);
        assert_eq!(r.total_y, 3);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_words_category1() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let meta1 = meta(&[("x", "a"), ("y", "b")]);
        let meta2 = meta(&[("x", "c"), ("z", "d")]);
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &meta1,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &meta2,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: Some(("y", "b")),
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![Sample {
                x: 1234,
                token_count: 3,
                tokens: vec![st(0, 1), st(1, 2),]
            },]
        );
        assert_eq!(r.total_x, 1234);
        assert_eq!(r.total_y, 2);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_types_words_category2() {
        let my = MeasureY::Types;
        let mx = MeasureX::Words;
        let meta1 = meta(&[("x", "a"), ("y", "b")]);
        let meta2 = meta(&[("x", "c"), ("z", "d")]);
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &meta1,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &meta2,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: Some(("x", "a")),
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![Sample {
                x: 1234,
                token_count: 3,
                tokens: vec![st(0, 1), st(1, 2),]
            },]
        );
        assert_eq!(r.total_x, 1234);
        assert_eq!(r.total_y, 2);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_marked_types_none() {
        let my = MeasureY::MarkedTypes;
        let mx = MeasureX::Types;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ct("c"), ct("c"), ct("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ct("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 0,
                    token_count: 3,
                    tokens: vec![st(0, 1), st(1, 2),]
                },
                Sample {
                    x: 0,
                    token_count: 2,
                    tokens: vec![st(1, 1), st(2, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 3);
        assert_eq!(r.total_y, 0);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_marked_types_all() {
        let my = MeasureY::MarkedTypes;
        let mx = MeasureX::Types;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ctm("c"), ctm("c"), ctm("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ctm("c"), ctm("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 0,
                    token_count: 3,
                    tokens: vec![stm(0, 1, 1), stm(1, 2, 2),]
                },
                Sample {
                    x: 0,
                    token_count: 2,
                    tokens: vec![stm(1, 1, 1), stm(2, 1, 1),]
                }
            ]
        );
        assert_eq!(r.total_x, 3);
        assert_eq!(r.total_y, 3);
        assert_eq!(r.points, HashSet::new());
    }

    #[test]
    fn build_subsets_marked_types_some() {
        let my = MeasureY::MarkedTypes;
        let mx = MeasureX::Types;
        let no_metadata = HashMap::new();
        let samples = vec![
            CSample {
                year: 1555,
                metadata: &no_metadata,
                words: 1234,
                tokens: vec![ctm("c"), ct("c"), ctm("b")],
            },
            CSample {
                year: 1666,
                metadata: &no_metadata,
                words: 5678,
                tokens: vec![ctm("c"), ct("d")],
            },
        ];
        let key = SubsetKey {
            category: None,
            period: (1500, 1700),
        };
        let r = build_subset(mx, my, &samples, key, false).unwrap();
        assert_eq!(r.category, key.category);
        assert_eq!(r.period, key.period);
        assert_eq!(
            r.samples,
            vec![
                Sample {
                    x: 0,
                    token_count: 3,
                    tokens: vec![stm(0, 1, 1), stm(1, 2, 1),]
                },
                Sample {
                    x: 0,
                    token_count: 2,
                    tokens: vec![stm(1, 1, 1), stm(2, 1, 0),]
                }
            ]
        );
        assert_eq!(r.total_x, 3);
        assert_eq!(r.total_y, 2);
        assert_eq!(r.points, HashSet::new());
    }
}

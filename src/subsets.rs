use crate::calc_point::Point;
use crate::calculation::{SToken, Sample};
use crate::categories::{self, Category};
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
        lemmas.extend(&s.tokens);
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();
    let lemmamap: HashMap<&str, usize> = lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();
    let total_types = lemmas.len() as u64;

    let samples = if split_samples {
        assert_eq!(measure_x, MeasureX::Tokens);
        let mut split = vec![];
        for s in samples {
            for lemma in &s.tokens {
                let token = SToken {
                    count: 1,
                    id: lemmamap[lemma],
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
        samples
            .into_iter()
            .map(|s| {
                let mut tokencount = HashMap::new();
                for lemma in &s.tokens {
                    let id = lemmamap[lemma];
                    *tokencount.entry(id).or_insert(0) += 1;
                }
                let mut tokens = tokencount
                    .iter()
                    .map(|(&id, &count)| SToken { id, count })
                    .collect_vec();
                tokens.sort_by_key(|t| t.id);
                let token_count = tokens.iter().map(|t| t.count).sum();
                let x = match measure_x {
                    MeasureX::Tokens => token_count,
                    MeasureX::Words => s.words,
                };
                Sample {
                    x,
                    token_count,
                    tokens,
                }
            })
            .collect_vec()
    };
    let total_x: u64 = samples.iter().map(|s| s.x).sum();
    let total_tokens = samples.iter().map(|s| s.token_count).sum();
    let total_y = match measure_y {
        MeasureY::Types => total_types,
        MeasureY::Tokens => total_tokens,
    };
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

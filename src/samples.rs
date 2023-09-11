use crate::categories::{self, Category};
use crate::errors::{self, Result};
use crate::input::{ISample, Year};
use crate::output::Years;
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet};

pub struct CToken<'a> {
    pub token: &'a str,
    pub marked: bool,
}

pub struct CSample<'a> {
    pub year: Year,
    pub metadata: &'a HashMap<String, String>,
    pub words: u64,
    pub tokens: Vec<CToken<'a>>,
}

fn get_sample<'a>(restrict_tokens: Category, mark_tokens: Category, s: &'a ISample) -> CSample<'a> {
    CSample {
        year: s.year,
        metadata: &s.metadata,
        words: s.words,
        tokens: s
            .tokens
            .iter()
            .filter_map(|t| {
                if categories::matches(restrict_tokens, &t.metadata) {
                    Some(CToken {
                        token: &t.lemma as &str,
                        marked: categories::matches(mark_tokens, &t.metadata),
                    })
                } else {
                    None
                }
            })
            .collect_vec(),
    }
}

pub fn get_samples<'a>(
    restrict_samples: Category,
    restrict_tokens: Category,
    mark_tokens: Category,
    samples: &'a [ISample],
) -> Vec<CSample<'a>> {
    samples
        .iter()
        .filter_map(|s| {
            if categories::matches(restrict_samples, &s.metadata) {
                Some(get_sample(restrict_tokens, mark_tokens, s))
            } else {
                None
            }
        })
        .collect_vec()
}

pub fn get_years(samples: &[CSample]) -> Years {
    let mut years = None;
    for s in samples {
        years = match years {
            None => Some((s.year, s.year + 1)),
            Some((a, b)) => Some((a.min(s.year), b.max(s.year + 1))),
        };
    }
    years.expect("there are samples")
}

pub fn get_categories<'a>(key: &'a str, samples: &[CSample<'a>]) -> Result<Vec<Category<'a>>> {
    let mut values = HashSet::new();
    for s in samples {
        match s.metadata.get(key) {
            None => (),
            Some(val) => {
                values.insert(val);
            }
        };
    }
    if values.is_empty() {
        return Err(errors::invalid_input(format!(
            "there are no samples with metadata key {}",
            key
        )));
    }
    let mut values = values.into_iter().collect_vec();
    values.sort();
    let valstring = values.iter().join(", ");
    let categories = values
        .into_iter()
        .map(|val| Some((key as &str, val as &str)))
        .collect_vec();
    info!("categories: {} = {}", key, valstring);
    Ok(categories)
}

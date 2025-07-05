//! Internal representation of tokens and samples.

use crate::categories::{self, Category};
use crate::errors::{self, Result};
use crate::input::{ISample, Year};
use crate::output::Years;
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet};

/// Internal representation of tokens,
pub struct CToken<'a> {
    /// Lemma.
    /// See [crate::input::IToken::lemma].
    pub token: &'a str,
    /// Is this marked as relevant?
    /// See [crate::driver::DriverArgs::mark_tokens].
    pub marked: bool,
}

/// Internal representation of samples.
pub struct CSample<'a> {
    /// Year.
    /// See [crate::input::ISample::year].
    pub year: Year,
    /// Metadata related to this sample.
    /// See [crate::input::ISample::metadata].
    pub metadata: &'a HashMap<String, String>,
    /// The number of words in this sample.
    /// See [crate::input::ISample::words].
    pub words: u64,
    /// Tokens of this sample.
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

/// Filter and convert samples.
///
/// Turn a list of [crate::input::ISample] objects into [crate::input::CSample] objects.
/// Only samples with year in range `years` are kept.
/// Only samples that match `restrict_samples` are kept.
/// Only tokens that match `restrict_tokens` are kept.
/// Tokens that match `mark_tokens` are marked.
/// Token metadata is then discarded.
pub fn get_samples<'a>(
    years: &Years,
    restrict_samples: Category,
    restrict_tokens: Category,
    mark_tokens: Category,
    samples: &'a [ISample],
) -> Vec<CSample<'a>> {
    samples
        .iter()
        .filter_map(|s| {
            if years.0 <= s.year
                && s.year < years.1
                && categories::matches(restrict_samples, &s.metadata)
            {
                Some(get_sample(restrict_tokens, mark_tokens, s))
            } else {
                None
            }
        })
        .collect_vec()
}

/// Get the range of years represented by a list of samples.
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

/// Get all categories for a given key.
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
            "there are no samples with metadata key {key}"
        )));
    }
    let mut values = values.into_iter().collect_vec();
    values.sort();
    let valstring = values.iter().join(", ");
    let categories = values
        .into_iter()
        .map(|val| Some((key as &str, val as &str)))
        .collect_vec();
    info!(target: "types3", "categories: {key} = {valstring}");
    Ok(categories)
}

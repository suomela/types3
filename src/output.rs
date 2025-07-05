//! Data structures for representing the output.

use crate::input::Year;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

/// What to calculate.
///
/// In the visualizations, this corresponds to what will be put in the y axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureY {
    /// Number of distinct lemmas.
    Types,
    /// Number of tokens.
    Tokens,
    /// Number of hapax legomena (types with only one token).
    Hapaxes,
    /// Number of smaples.
    Samples,
    /// Number of distinct lemmas in marked tokens.
    MarkedTypes,
}

impl fmt::Display for MeasureY {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MeasureY::Types => write!(f, "types"),
            MeasureY::Tokens => write!(f, "tokens"),
            MeasureY::Hapaxes => write!(f, "hapaxes"),
            MeasureY::Samples => write!(f, "samples"),
            MeasureY::MarkedTypes => write!(f, "marked types"),
        }
    }
}

/// Criterion used to compare subcorpora.
///
/// We will accumulate samples until they have the same size according to this measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureX {
    /// Number of running words.
    Words,
    /// Number of tokens.
    Tokens,
    /// Number of distinct lemmas.
    Types,
}

impl fmt::Display for MeasureX {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MeasureX::Words => write!(f, "words"),
            MeasureX::Tokens => write!(f, "tokens"),
            MeasureX::Types => write!(f, "types"),
        }
    }
}

/// Time period (range of years).
pub type Years = (Year, Year);

/// Representation for an optional key-value pair.
///
/// See [crate::categories::Category] for the non-owned version.
pub type OCategory = Option<(String, String)>;

/// Representation for the average value.
///
/// We measure the average number of things of type [Output::measure_y],
/// in random subcorpora with [Output::limit] many things of type
/// [Output::measure_x].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct AvgResult {
    /// Lower bound for the sum. Divide by `iter` to get the lower bound for the average.
    pub low: u64,
    /// Upper bound for the sum. Divide by `iter` to get the upper bound for the average.
    pub high: u64,
    /// Number of random samples accumulated.
    pub iter: u64,
}

/// Representation for statistical significance.
///
/// We see if the total number of things of type [Output::measure_y]
/// is particularly low or high in comparison with random corpora with
/// the same number of things of type [Output::measure_x].
///
/// For example, if we have a subcorpus with particularly high values,
/// then we expect to see:
/// - above/iter ≈ 0.999…
/// - (iter - above) / iter ≈ 0.000…
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct PointResult {
    /// How many times we are above what is observed in a random subcorpus.
    pub above: u64,
    /// How many times we are below what is observed in a random subcorpus.
    pub below: u64,
    /// Number of random samples accumulated.
    pub iter: u64,
}

/// One point in the curves (one category, one time period).
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OResult {
    /// Time period.
    pub period: Years,
    /// Average numbers for [Output::measure_y] in subcorpora with [Output::limit]
    /// many things of type [Output::measure_x].
    pub average_at_limit: AvgResult,
    /// Do we have in this time period significantly many or few things of type
    /// [Output::measure_y] in comparison with other time periods in the same category.
    pub vs_time: PointResult,
    /// Do we have in this category significantly many or few things of type
    /// [Output::measure_y] in comparison with other categories in the same time period.
    pub vs_categories: Option<PointResult>,
}

/// One result curve (one category, all time periods).
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OCurve {
    /// Which category?
    pub category: OCategory,
    /// Time series.
    pub results: Vec<OResult>,
}

/// Results of the calculation.
#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Output {
    /// Sample-level restriction.
    /// Can be either a key-value pair, or `None`.
    /// See [crate::driver::DriverArgs::restrict_samples].
    pub restrict_samples: OCategory,
    /// Token-level restriction.
    /// Can be either a key-value pair, or `None`.
    /// See [crate::driver::DriverArgs::restrict_tokens].
    pub restrict_tokens: OCategory,
    /// Which tokens were marked.
    /// See [crate::driver::DriverArgs::mark_tokens].
    pub mark_tokens: OCategory,
    /// Results.
    pub curves: Vec<OCurve>,
    /// Range of years covered.
    pub years: Years,
    /// Time periods covered.
    /// This is redundant information in the sense that each curve in [Output::curves]
    /// covers exactly the same time periods as what is specified here.
    pub periods: Vec<Years>,
    /// What was calculated.
    /// In the visualizations, this corresponds to what is put in the y axis.
    /// See [crate::driver::DriverArgs::measure_y].
    pub measure_y: MeasureY,
    /// Criterion used to compare subcorpora.
    /// We accumulate samples until they have the same size according to this measure.
    /// See [crate::driver::DriverArgs::measure_x].
    pub measure_x: MeasureX,
    /// Did we split samples?
    /// See [crate::driver::DriverArgs::split_samples].
    pub split_samples: bool,
    /// What was the size limit that we used for calculating averages.
    pub limit: u64,
    /// The number of iterations.
    pub iter: u64,
}

/// Structure for saving errors in a machine-readable form.
///
/// This is used for communication between types3-calc and types3-ui:
/// types3-calc can save errors as a JSON serialization of `OError`,
/// and types3-ui can read it.
#[derive(Serialize)]
pub struct OError {
    /// Human-readable error message.
    pub error: String,
}

/// Human-friendly representation for [AvgResult].
///
/// # Examples
/// ```
/// use types3::output::{AvgResult, avg_string};
/// let x = AvgResult { low: 10, high: 20, iter: 100 };
/// assert_eq!("0.10–0.20", avg_string(&x));
/// ```
pub fn avg_string(ar: &AvgResult) -> String {
    let low = ar.low as f64 / ar.iter as f64;
    let high = ar.high as f64 / ar.iter as f64;
    format!("{low:.2}–{high:.2}")
}

/// Human-friendly representation for [PointResult].
///
/// # Examples
/// ```
/// use types3::output::{PointResult, point_string};
/// let x = PointResult { above: 9995, below: 3, iter: 10000 };
/// assert_eq!("+++", point_string(&x));
/// ```
pub fn point_string(pr: &PointResult) -> String {
    let above = (pr.iter - pr.above) as f64 / pr.iter as f64;
    let below = (pr.iter - pr.below) as f64 / pr.iter as f64;
    let s = if above < 0.0001 {
        "++++"
    } else if above < 0.001 {
        "+++"
    } else if above < 0.01 {
        "++"
    } else if above < 0.1 {
        "+"
    } else if below < 0.0001 {
        "----"
    } else if below < 0.001 {
        "---"
    } else if below < 0.01 {
        "--"
    } else if below < 0.1 {
        "-"
    } else {
        "0"
    };
    s.to_owned()
}

/// Human-friendly representation for [Years].
///
/// # Examples
/// ```
/// use types3::output::pretty_period;
/// assert_eq!("1900–1999", pretty_period(&(1900, 2000)));
/// ```
pub fn pretty_period(p: &Years) -> String {
    format!("{}–{}", p.0, p.1 - 1)
}

/// Human-friendly representation for a list of [Years].
///
/// # Examples
/// ```
/// use types3::output::pretty_periods;
/// let x = [
///     (1990, 2000),
///     (2000, 2010),
///     (2010, 2020),
///     (2020, 2030),
///     (2030, 2040),
///     (2040, 2050),
/// ];
/// assert_eq!("1990–1999, 2000–2009, ..., 2040–2049", pretty_periods(&x));
/// ```
pub fn pretty_periods(periods: &[Years]) -> String {
    if periods.len() >= 5 {
        pretty_periods(&periods[0..2]) + ", ..., " + &pretty_period(periods.last().unwrap())
    } else {
        periods.iter().map(pretty_period).collect_vec().join(", ")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pretty_period_basic() {
        assert_eq!(pretty_period(&(1990, 2000)), "1990–1999");
    }

    #[test]
    fn pretty_periods_basic() {
        assert_eq!(pretty_periods(&[(1990, 2000)]), "1990–1999");
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010)]),
            "1990–1999, 2000–2009"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020)]),
            "1990–1999, 2000–2009, 2010–2019"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020), (2020, 2030)]),
            "1990–1999, 2000–2009, 2010–2019, 2020–2029"
        );
        assert_eq!(
            pretty_periods(&[
                (1990, 2000),
                (2000, 2010),
                (2010, 2020),
                (2020, 2030),
                (2030, 2040)
            ]),
            "1990–1999, 2000–2009, ..., 2030–2039"
        );
        assert_eq!(
            pretty_periods(&[
                (1990, 2000),
                (2000, 2010),
                (2010, 2020),
                (2020, 2030),
                (2030, 2040),
                (2040, 2050)
            ]),
            "1990–1999, 2000–2009, ..., 2040–2049"
        );
    }
}

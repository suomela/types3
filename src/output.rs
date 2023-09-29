//! Data structures for representing the output.

use crate::input::Year;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureY {
    Types,
    Tokens,
    Hapaxes,
    Samples,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureX {
    Words,
    Tokens,
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

pub type Years = (Year, Year);
pub type OCategory = Option<(String, String)>;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct AvgResult {
    pub low: u64,
    pub high: u64,
    pub iter: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct PointResult {
    pub above: u64,
    pub below: u64,
    pub iter: u64,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OResult {
    pub period: Years,
    pub average_at_limit: AvgResult,
    pub vs_time: PointResult,
    pub vs_categories: Option<PointResult>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct OCurve {
    pub category: OCategory,
    pub results: Vec<OResult>,
}

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Output {
    pub restrict_samples: OCategory,
    pub restrict_tokens: OCategory,
    pub mark_tokens: OCategory,
    pub curves: Vec<OCurve>,
    pub years: Years,
    pub periods: Vec<Years>,
    pub measure_y: MeasureY,
    pub measure_x: MeasureX,
    pub split_samples: bool,
    pub limit: u64,
    pub iter: u64,
}

#[derive(Serialize)]
pub struct OError {
    pub error: String,
}

pub fn avg_string(ar: &AvgResult) -> String {
    let low = ar.low as f64 / ar.iter as f64;
    let high = ar.high as f64 / ar.iter as f64;
    format!("{:.2}–{:.2}", low, high)
}

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

pub fn pretty_period(p: &Years) -> String {
    format!("{}-{}", p.0, p.1 - 1)
}

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
        assert_eq!(pretty_period(&(1990, 2000)), "1990-1999");
    }

    #[test]
    fn pretty_periods_basic() {
        assert_eq!(pretty_periods(&[(1990, 2000)]), "1990-1999");
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010)]),
            "1990-1999, 2000-2009"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020)]),
            "1990-1999, 2000-2009, 2010-2019"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020), (2020, 2030)]),
            "1990-1999, 2000-2009, 2010-2019, 2020-2029"
        );
        assert_eq!(
            pretty_periods(&[
                (1990, 2000),
                (2000, 2010),
                (2010, 2020),
                (2020, 2030),
                (2030, 2040)
            ]),
            "1990-1999, 2000-2009, ..., 2030-2039"
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
            "1990-1999, 2000-2009, ..., 2040-2049"
        );
    }
}

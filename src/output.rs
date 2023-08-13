use crate::input::Year;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Measure {
    Words,
    Tokens,
}

impl fmt::Display for Measure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Measure::Words => write!(f, "words"),
            Measure::Tokens => write!(f, "tokens"),
        }
    }
}

pub type Years = (Year, Year);
pub type OCategory = Option<(String, String)>;


#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct AvgResult {
    pub types_low: u64,
    pub types_high: u64,
    pub iter: u64,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct PointResult {
    pub above: u64,
    pub below: u64,
    pub iter: u64,
}

#[derive(Deserialize, Serialize)]
pub struct OResult {
    pub period: Years,
    pub average_at_limit: AvgResult,
    pub vs_time: PointResult,
    pub vs_categories: Option<PointResult>,
}

#[derive(Deserialize, Serialize)]
pub struct OCurve {
    pub category: OCategory,
    pub results: Vec<OResult>,
}

#[derive(Deserialize, Serialize)]
pub struct Output {
    pub restrict_samples: OCategory,
    pub curves: Vec<OCurve>,
    pub years: Years,
    pub periods: Vec<Years>,
    pub measure: Measure,
    pub limit: u64,
    pub iter: u64,
}

pub fn avg_string(ar: &AvgResult) -> String {
    let low = ar.types_low as f64 / ar.iter as f64;
    let high = ar.types_high as f64 / ar.iter as f64;
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

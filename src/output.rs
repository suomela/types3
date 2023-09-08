use crate::input::Year;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureY {
    Types,
    Tokens,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MeasureX {
    Words,
    Tokens,
}

impl fmt::Display for MeasureX {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MeasureX::Words => write!(f, "words"),
            MeasureX::Tokens => write!(f, "tokens"),
        }
    }
}

pub type Years = (Year, Year);
pub type OCategory = Option<(String, String)>;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct AvgResult {
    pub low: u64,
    pub high: u64,
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
    pub restrict_tokens: OCategory,
    pub curves: Vec<OCurve>,
    pub years: Years,
    pub periods: Vec<Years>,
    pub measure_y: MeasureY,
    pub measure_x: MeasureX,
    pub split_samples: bool,
    pub limit: u64,
    pub iter: u64,
}

#[derive(Deserialize, Serialize)]
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

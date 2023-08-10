use crate::calculation::{AvgResult, PointResult};
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
    pub curves: Vec<OCurve>,
    pub years: Years,
    pub periods: Vec<Years>,
    pub measure: Measure,
    pub limit: u64,
    pub iter: u64,
}

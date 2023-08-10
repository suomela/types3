use crate::calculation::{AvgResult, PointResult};
use crate::input::Year;
use serde::{Deserialize, Serialize};

pub type Years = (Year, Year);

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Category {
    All,
    Subset(String, String),
}

#[derive(Deserialize, Serialize)]
pub struct OResult {
    pub period: Years,
    pub limit: u64,
    pub average_at_limit: AvgResult,
    pub vs_time: PointResult,
    pub vs_categories: Option<PointResult>,
}

#[derive(Deserialize, Serialize)]
pub struct OCurve {
    pub category: Category,
    pub results: Vec<OResult>,
}

#[derive(Deserialize, Serialize)]
pub struct Output {
    pub curves: Vec<OCurve>,
    pub periods: Vec<Years>,
}

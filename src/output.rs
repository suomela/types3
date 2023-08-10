use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Category {
    All,
    Subset(String, String),
}

#[derive(Deserialize, Serialize)]
pub struct OCurve {
    pub category: Category,
}

#[derive(Deserialize, Serialize)]
pub struct Output {
    pub curves: Vec<OCurve>,
}

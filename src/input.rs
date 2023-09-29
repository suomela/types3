//! Data structures for representing the input.

use serde::Deserialize;
use std::collections::HashMap;

/// Type used to represent years.
pub type Year = i16;

/// One token in the input.
#[derive(Deserialize)]
pub struct IToken {
    /// Lemma.
    /// Tokens with the same lemma are considered to represent the same type.
    pub lemma: String,
    /// Optional free-form description of this token.
    /// This does not influence calculations.
    pub descr: Option<HashMap<String, String>>,
    /// Metadata related to this token.
    /// This can be used to select what to calculate; see [crate::driver::DriverArgs].
    pub metadata: HashMap<String, String>,
}

/// One sample in the input.
#[derive(Deserialize)]
pub struct ISample {
    /// Sample identifier.
    pub id: String,
    /// Year.
    /// This is used to determine which samples belong to which periods.
    pub year: Year,
    /// Optional free-form description of this sample.
    /// This does not influence calculations.
    pub descr: Option<HashMap<String, String>>,
    /// Metadata related to this sample.
    /// This can be used to select what to calculate; see [crate::driver::DriverArgs].
    pub metadata: HashMap<String, String>,
    /// The number of words in this sample.
    /// This is relevant for [crate::output::MeasureX::Words].
    pub words: u64,
    /// Tokens of this sample.
    pub tokens: Vec<IToken>,
}

/// The entire input.
#[derive(Deserialize)]
pub struct Input {
    /// Samples.
    pub samples: Vec<ISample>,
}

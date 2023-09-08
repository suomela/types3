use crate::input::Year;
use std::collections::HashMap;

pub struct CSample<'a> {
    pub year: Year,
    pub metadata: &'a HashMap<String, String>,
    pub words: u64,
    pub tokens: Vec<&'a str>,
}

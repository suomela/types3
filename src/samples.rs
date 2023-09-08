use crate::input::Year;
use crate::output::Years;
use std::collections::HashMap;

pub struct CSample<'a> {
    pub year: Year,
    pub metadata: &'a HashMap<String, String>,
    pub words: u64,
    pub tokens: Vec<&'a str>,
}

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

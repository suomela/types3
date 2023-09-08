use crate::errors::{self, Result};
use crate::output::OCategory;
use crate::samples::CSample;
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet};

pub type Category<'a> = Option<(&'a str, &'a str)>;

pub fn owned_cat(category: Category) -> OCategory {
    category.map(|(k, v)| (k.to_owned(), v.to_owned()))
}

pub fn matches(category: Category, metadata: &HashMap<String, String>) -> bool {
    match category {
        None => true,
        Some((k, v)) => match metadata.get(k) {
            None => false,
            Some(v2) => v == v2,
        },
    }
}

pub fn parse_restriction(arg: &Option<String>) -> Result<Category> {
    match arg {
        None => Ok(None),
        Some(r) => {
            let parts = r.split('=').collect_vec();
            if parts.len() != 2 {
                return Err(errors::invalid_argument(format!(
                    "restriction should be of the form 'key=value', got '{r}'"
                )));
            }
            let category = Some((parts[0], parts[1]));
            Ok(category)
        }
    }
}

pub fn get_categories<'a>(key: &'a str, samples: &[CSample<'a>]) -> Result<Vec<Category<'a>>> {
    let mut values = HashSet::new();
    for s in samples {
        match s.metadata.get(key) {
            None => (),
            Some(val) => {
                values.insert(val);
            }
        };
    }
    if values.is_empty() {
        return Err(errors::invalid_input(format!(
            "there are no samples with metadata key {}",
            key
        )));
    }
    let mut values = values.into_iter().collect_vec();
    values.sort();
    let valstring = values.iter().join(", ");
    let categories = values
        .into_iter()
        .map(|val| Some((key as &str, val as &str)))
        .collect_vec();
    info!("categories: {} = {}", key, valstring);
    Ok(categories)
}

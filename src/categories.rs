use crate::errors::{invalid_argument, Result};
use crate::output::OCategory;
use itertools::Itertools;
use std::collections::HashMap;

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
                return Err(invalid_argument(format!(
                    "restriction should be of the form 'key=value', got '{r}'"
                )));
            }
            let category = Some((parts[0], parts[1]));
            Ok(category)
        }
    }
}

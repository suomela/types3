//! Types and utilities related to token and sample categories.

use crate::errors::{self, Result};
use crate::output::OCategory;
use itertools::Itertools;
use std::collections::HashMap;

/// Representation for an optional key-value pair.
///
/// See [crate::output::OCategory] for the owned version.
pub type Category<'a> = Option<(&'a str, &'a str)>;

/// Converts [Category] to [OCategory].
pub fn owned_cat(category: Category) -> OCategory {
    category.map(|(k, v)| (k.to_owned(), v.to_owned()))
}

/// Checks if `metadata` contains a key-value pair that matches `category`.
///
/// If `category` is `None`, this always returns `true`.
///
/// # Examples
/// ```
/// use types3::categories::matches;
/// use std::collections::HashMap;
/// let mut md = HashMap::new();
/// md.insert("a".to_owned(), "x".to_owned());
/// assert!(matches(None, &md));
/// assert!(matches(Some(("a", "x")), &md));
/// assert!(!matches(Some(("a", "y")), &md));
/// ```
pub fn matches(category: Category, metadata: &HashMap<String, String>) -> bool {
    match category {
        None => true,
        Some((k, v)) => match metadata.get(k) {
            None => false,
            Some(v2) => v == v2,
        },
    }
}

/// Parses a key-value pair given in the command line.
///
/// # Examples
/// ```
/// use types3::categories::parse_restriction;
/// assert_eq!(parse_restriction(&None).unwrap(), None);
/// assert_eq!(parse_restriction(&Some("a b=c d".to_owned())).unwrap(), Some(("a b", "c d")));
/// assert!(parse_restriction(&Some("a=b=c".to_owned())).is_err());
/// ```
pub fn parse_restriction(arg: &Option<String>) -> Result<Category<'_>> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn matches_empty() {
        let empty = HashMap::new();
        assert!(matches(None, &empty));
        assert!(!matches(Some(("a", "x")), &empty));
    }

    #[test]
    fn matches_nonempty() {
        let mut md = HashMap::new();
        md.insert("a".to_owned(), "x".to_owned());
        md.insert("b".to_owned(), "y".to_owned());
        md.insert("c".to_owned(), "z".to_owned());
        assert!(matches(None, &md));
        assert!(!matches(Some(("a", "y")), &md));
        assert!(matches(Some(("a", "x")), &md));
        assert!(!matches(Some(("d", "z")), &md));
    }

    #[test]
    fn parse_restriction_basic() {
        assert_eq!(None, parse_restriction(&None).unwrap());
        assert_eq!(
            Some(("a", "b")),
            parse_restriction(&Some("a=b".to_owned())).unwrap()
        );
        assert_eq!(
            Some(("a b", "c d")),
            parse_restriction(&Some("a b=c d".to_owned())).unwrap()
        );
        assert_eq!(
            Some(("", "")),
            parse_restriction(&Some("=".to_owned())).unwrap()
        );
    }

    #[test]
    fn parse_restriction_fail() {
        parse_restriction(&Some("".to_owned())).unwrap_err();
        parse_restriction(&Some("a".to_owned())).unwrap_err();
        parse_restriction(&Some("a=b=c".to_owned())).unwrap_err();
        parse_restriction(&Some("a=b=c=d".to_owned())).unwrap_err();
    }
}

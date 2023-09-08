use crate::calc_point::Point;
use crate::calculation::Sample;
use crate::categories::Category;
use crate::output::{self, Years};
use std::collections::HashSet;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubsetKey<'a> {
    pub category: Category<'a>,
    pub period: Years,
}

impl SubsetKey<'_> {
    pub fn pretty(&self) -> String {
        match &self.category {
            None => output::pretty_period(&self.period),
            Some((k, v)) => format!("{}, {} = {}", output::pretty_period(&self.period), k, v),
        }
    }
}
pub struct Subset<'a> {
    pub category: Category<'a>,
    pub period: Years,
    pub samples: Vec<Sample>,
    pub total_x: u64,
    pub total_y: u64,
    pub points: HashSet<Point>,
}

impl<'a> Subset<'a> {
    pub fn pretty(&self) -> String {
        self.key().pretty()
    }

    pub fn key(&self) -> SubsetKey {
        SubsetKey {
            category: self.category,
            period: self.period,
        }
    }

    pub fn get_point(&self) -> Point {
        Point {
            x: self.total_x,
            y: self.total_y,
        }
    }

    pub fn get_parent_period(&self, years: Years) -> SubsetKey<'a> {
        SubsetKey {
            category: self.category,
            period: years,
        }
    }

    pub fn get_parent_category(&self) -> SubsetKey<'a> {
        assert!(self.category.is_some());
        SubsetKey {
            category: None,
            period: self.period,
        }
    }

    pub fn get_parents(&self, years: Years) -> Vec<SubsetKey<'a>> {
        match self.category {
            None => vec![self.get_parent_period(years)],
            Some(_) => vec![self.get_parent_period(years), self.get_parent_category()],
        }
    }
}

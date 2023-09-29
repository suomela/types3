//! Main entry point for calculating everything.

use crate::calc_avg;
use crate::calc_point::{self, Point};
use crate::categories::{self, Category};
use crate::errors::{self, Result};
use crate::information;
use crate::input::{Input, Year};
use crate::output::{self, MeasureX, MeasureY, OCurve, OResult, Output, PointResult, Years};
use crate::samples;
use crate::subsets::{self, Subset, SubsetKey};
use itertools::Itertools;
use log::{debug, info};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

/// What to calculate?
pub struct DriverArgs<'a> {
    /// Sample metadata category to consider.
    /// If specified, calculate curves for each distinct value that we have for this metadata key in [crate::input::ISample::metadata].
    /// If not specified, calculate just one curve for all data.
    pub category: Option<&'a str>,

    /// What to calculate.
    /// In the visualizations, this corresponds to what will be put in the y axis.
    pub measure_y: MeasureY,

    /// Criterion used to compare subcorpora.
    /// We will accumulate samples until they have the same size according to this measure.
    pub measure_x: MeasureX,

    /// Number of iterations.
    /// How many random permutations to produce.
    pub iter: u64,

    /// Periodization offset.
    /// If 0, period starting points will be multiples of the step size.
    /// For example, if we use 100-year steps, we will have periods starting at 1800, 1900, 2000, etc.
    /// By setting the `offset` e.g. to 12, we will switch to periods starting at 1812, 1912, 2012, etc.
    pub offset: Year,

    /// Starting year.
    /// Empty periods in the beginning will be automatically omitted,
    /// so the starting year can be safely set to e.g. 0.
    pub start: Year,

    /// Final year.
    /// Empty periods in the end will be automatically omitted,
    /// so the final year can be safely set to e.g. 9999.
    pub end: Year,

    /// Windows size.
    pub window: Year,

    /// Step size.
    pub step: Year,

    /// Sample-level restriction.
    /// Can be either a key-value pair (which refers to [crate::input::ISample::metadata]), or `None` if there is no need to restrict based on sample metadata.
    pub restrict_samples: Category<'a>,

    /// Token-level restriction.
    /// Can be either a key-value pair (which refers to [crate::input::IToken::metadata]), or `None` if there is no need to restrict based on token metadata.
    pub restrict_tokens: Category<'a>,

    /// Which tokens are marked.
    /// Can be either a key-value pair (which refers to [crate::input::IToken::metadata]), or `None` if there is no need to mark tokens.
    /// Marking is relevant if [DriverArgs::measure_y] is set to [MeasureY::MarkedTypes].
    pub mark_tokens: Category<'a>,

    /// Do we split samples?
    pub split_samples: bool,
}

struct Curve<'a> {
    category: Category<'a>,
    keys: Vec<SubsetKey<'a>>,
}

fn get_periods(args: &DriverArgs, years: &Years) -> Vec<Years> {
    let mut periods = vec![];
    let mut y = args.offset;
    while y + args.step <= years.0 {
        y += args.step;
    }
    loop {
        let p = (y, y + args.window);
        periods.push(p);
        if p.1 >= years.1 {
            break;
        }
        y += args.step;
    }
    info!(target: "types3", "periods: {}", output::pretty_periods(&periods));
    periods
}

fn build_curve<'a>(category: Category<'a>, periods: &[Years]) -> Curve<'a> {
    let keys = periods
        .iter()
        .map(|&period| SubsetKey { category, period })
        .collect_vec();
    Curve { category, keys }
}

fn build_curves<'a>(categories: &[Category<'a>], periods: &[Years]) -> Vec<Curve<'a>> {
    categories
        .iter()
        .map(|category| build_curve(*category, periods))
        .collect_vec()
}

type TopResults<'a> = HashMap<(SubsetKey<'a>, Point), PointResult>;

/// Calculate everything.
///
/// This is the main entry point for the library.
pub fn calc(args: &DriverArgs, input: &Input) -> Result<Output> {
    Calc::new(args, input)?.calc()
}

struct Calc<'a> {
    years: Years,
    periods: Vec<Years>,
    curves: Vec<Curve<'a>>,
    subset_map: HashMap<SubsetKey<'a>, Subset<'a>>,
    iter: u64,
    measure_y: MeasureY,
    measure_x: MeasureX,
    restrict_samples: Category<'a>,
    restrict_tokens: Category<'a>,
    mark_tokens: Category<'a>,
    split_samples: bool,
}

impl<'a> Calc<'a> {
    fn new(args: &'a DriverArgs, input: &'a Input) -> Result<Calc<'a>> {
        information::statistics(&input.samples);
        let samples = samples::get_samples(
            args.restrict_samples,
            args.restrict_tokens,
            args.mark_tokens,
            &input.samples,
        );
        information::post_statistics(&samples);
        if samples.is_empty() {
            return Err(errors::invalid_input_ref("no samples found"));
        }
        let categories = match &args.category {
            None => vec![None],
            Some(key) => samples::get_categories(key, &samples)?,
        };
        let years = {
            let years = samples::get_years(&samples);
            info!(target: "types3", "years in input data: {}", output::pretty_period(&years));
            (years.0.max(args.start), years.1.min(args.end + 1))
        };
        let periods = get_periods(args, &years);
        let curves = build_curves(&categories, &periods);
        let mut subset_map = HashMap::new();
        for curve in &curves {
            for key in &curve.keys {
                let subset = subsets::build_subset(
                    args.measure_x,
                    args.measure_y,
                    &samples,
                    *key,
                    args.split_samples,
                )?;
                let point = subset.get_point();
                let parents = subset.get_parents(years);
                subset_map.insert(*key, subset);
                for parent in &parents {
                    let x = match subset_map.entry(*parent) {
                        Occupied(e) => e.into_mut(),
                        Vacant(e) => e.insert(subsets::build_subset(
                            args.measure_x,
                            args.measure_y,
                            &samples,
                            *parent,
                            args.split_samples,
                        )?),
                    };
                    x.points.insert(point);
                }
            }
        }
        Ok(Calc {
            years,
            periods,
            curves,
            subset_map,
            iter: args.iter,
            measure_y: args.measure_y,
            measure_x: args.measure_x,
            restrict_samples: args.restrict_samples,
            restrict_tokens: args.restrict_tokens,
            mark_tokens: args.mark_tokens,
            split_samples: args.split_samples,
        })
    }

    fn size_limit(&self) -> u64 {
        self.curves
            .iter()
            .map(|c| self.curve_size_limit(c))
            .min()
            .expect("at least one curve")
    }

    fn curve_size_limit(&self, curve: &Curve) -> u64 {
        curve
            .keys
            .iter()
            .map(|k| self.subset_map[k].total_x)
            .min()
            .expect("at least one period")
    }

    fn calc(self) -> Result<Output> {
        let mut top_results = HashMap::new();
        for subset in self.subset_map.values() {
            self.calc_top(subset, &mut top_results);
        }
        let limit = self.size_limit();
        debug!(target: "types3", "size limit: {} {}", limit, self.measure_x);
        let curves = self
            .curves
            .iter()
            .map(|c| self.calc_curve(c, limit, &top_results))
            .collect_vec();
        Ok(Output {
            curves,
            years: self.years,
            periods: self.periods,
            measure_y: self.measure_y,
            measure_x: self.measure_x,
            iter: self.iter,
            limit,
            restrict_tokens: categories::owned_cat(self.restrict_tokens),
            restrict_samples: categories::owned_cat(self.restrict_samples),
            mark_tokens: categories::owned_cat(self.mark_tokens),
            split_samples: self.split_samples,
        })
    }

    fn calc_top(&self, subset: &'a Subset, top_results: &mut TopResults<'a>) {
        if subset.points.is_empty() {
            return;
        }
        let mut points = subset.points.iter().copied().collect_vec();
        let key = subset.key();
        points.sort();
        let results =
            calc_point::compare_with_points(self.measure_y, &subset.samples, self.iter, &points);
        for (i, p) in points.into_iter().enumerate() {
            top_results.insert((key, p), results[i]);
        }
        debug!(target: "types3", "{}: calculated {} points", subset.pretty(), results.len());
    }

    fn calc_curve(&self, curve: &Curve, limit: u64, top_results: &TopResults) -> OCurve {
        OCurve {
            category: categories::owned_cat(curve.category),
            results: curve
                .keys
                .iter()
                .map(|k| self.calc_relevant(&self.subset_map[k], limit, top_results))
                .collect_vec(),
        }
    }

    fn calc_relevant(&self, subset: &Subset, limit: u64, top_results: &TopResults) -> OResult {
        let mut msg = format!("{}: ", subset.pretty());
        let average_at_limit =
            calc_avg::average_at_limit(self.measure_y, &subset.samples, self.iter, limit);
        msg.push_str(&format!(
            "{} {} / {} {}",
            output::avg_string(&average_at_limit),
            self.measure_y,
            limit,
            self.measure_x
        ));
        let p = subset.get_point();
        let vs_time = {
            let k = subset.get_parent_period(self.years);
            let pr = top_results[&(k, p)];
            msg.push_str(&format!(
                ", {} vs. other time points",
                output::point_string(&pr)
            ));
            pr
        };
        let vs_categories = match subset.category {
            None => None,
            Some(_) => {
                let k = subset.get_parent_category();
                let pr = top_results[&(k, p)];
                msg.push_str(&format!(
                    ", {} vs. other categories",
                    output::point_string(&pr)
                ));
                Some(pr)
            }
        };
        debug!(target: "types3", "{msg}");
        OResult {
            period: subset.period,
            average_at_limit,
            vs_time,
            vs_categories,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn build_args<'a>(window: Year, step: Year, offset: Year) -> DriverArgs<'a> {
        DriverArgs {
            category: None,
            measure_y: MeasureY::Types,
            measure_x: MeasureX::Tokens,
            iter: 0,
            offset,
            start: 0,
            end: 9999,
            window,
            step,
            restrict_samples: None,
            restrict_tokens: None,
            mark_tokens: None,
            split_samples: false,
        }
    }

    #[test]
    fn get_periods_10_10() {
        let args = build_args(10, 10, 0);
        assert_eq!(
            get_periods(&args, &(1911, 1979)),
            [
                (1910, 1920),
                (1920, 1930),
                (1930, 1940),
                (1940, 1950),
                (1950, 1960),
                (1960, 1970),
                (1970, 1980),
            ]
        );
        assert_eq!(
            get_periods(&args, &(1910, 1980)),
            [
                (1910, 1920),
                (1920, 1930),
                (1930, 1940),
                (1940, 1950),
                (1950, 1960),
                (1960, 1970),
                (1970, 1980),
            ]
        );
        assert_eq!(
            get_periods(&args, &(1909, 1981)),
            [
                (1900, 1910),
                (1910, 1920),
                (1920, 1930),
                (1930, 1940),
                (1940, 1950),
                (1950, 1960),
                (1960, 1970),
                (1970, 1980),
                (1980, 1990),
            ]
        );
    }

    #[test]
    fn get_periods_40_10() {
        let args = build_args(40, 10, 0);
        assert_eq!(
            get_periods(&args, &(1911, 1979)),
            [(1910, 1950), (1920, 1960), (1930, 1970), (1940, 1980),]
        );
        assert_eq!(
            get_periods(&args, &(1910, 1980)),
            [(1910, 1950), (1920, 1960), (1930, 1970), (1940, 1980),]
        );
        assert_eq!(
            get_periods(&args, &(1909, 1981)),
            [
                (1900, 1940),
                (1910, 1950),
                (1920, 1960),
                (1930, 1970),
                (1940, 1980),
                (1950, 1990),
            ]
        );
    }

    #[test]
    fn get_periods_10_10_offset1() {
        let args = build_args(10, 10, 1);
        assert_eq!(
            get_periods(&args, &(1911, 1979)),
            [
                (1911, 1921),
                (1921, 1931),
                (1931, 1941),
                (1941, 1951),
                (1951, 1961),
                (1961, 1971),
                (1971, 1981),
            ]
        );
        assert_eq!(
            get_periods(&args, &(1910, 1980)),
            [
                (1901, 1911),
                (1911, 1921),
                (1921, 1931),
                (1931, 1941),
                (1941, 1951),
                (1951, 1961),
                (1961, 1971),
                (1971, 1981),
            ]
        );
        assert_eq!(
            get_periods(&args, &(1909, 1981)),
            [
                (1901, 1911),
                (1911, 1921),
                (1921, 1931),
                (1931, 1941),
                (1941, 1951),
                (1951, 1961),
                (1961, 1971),
                (1971, 1981),
            ]
        );
        assert_eq!(
            get_periods(&args, &(1908, 1982)),
            [
                (1901, 1911),
                (1911, 1921),
                (1921, 1931),
                (1931, 1941),
                (1941, 1951),
                (1951, 1961),
                (1961, 1971),
                (1971, 1981),
                (1981, 1991),
            ]
        );
    }
}

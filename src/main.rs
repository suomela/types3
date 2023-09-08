use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::{error, fs, io, process};
use types3::calc_avg;
use types3::calc_point::{self, Point};
use types3::calculation::{SToken, Sample};
use types3::categories::{self, Category};
use types3::errors::{self, Result};
use types3::input::{ISample, Input, Year};
use types3::output::{
    avg_string, point_string, MeasureX, MeasureY, OCurve, OError, OResult, Output, PointResult,
    Years,
};
use types3::samples::CSample;

const DEFAULT_ITER: u64 = 1_000_000;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file (JSON)
    infile: String,
    /// Output file (JSON)
    outfile: String,
    /// Metadata category
    #[arg(long)]
    category: Option<String>,
    /// Count tokens (instead of types)
    #[arg(long, default_value_t = false)]
    count_tokens: bool,
    /// Compare with running words (instead of tokens)
    #[arg(long, default_value_t = false)]
    words: bool,
    /// Number of iterations
    #[arg(short, long, default_value_t = DEFAULT_ITER)]
    iter: u64,
    /// Starting offset
    #[arg(long, default_value_t = 0)]
    offset: Year,
    /// Starting year
    #[arg(long, default_value_t = 0)]
    start: Year,
    /// Ending year
    #[arg(long, default_value_t = 9999)]
    end: Year,
    /// Window length (years)
    #[arg(long)]
    window: Year,
    /// Step length (years)
    #[arg(long)]
    step: Year,
    /// Sample category restriction, of the form key=value
    #[arg(long)]
    restrict_samples: Option<String>,
    /// Token category restriction, of the form key=value
    #[arg(long)]
    restrict_tokens: Option<String>,
    /// Can we split samples?
    #[arg(long)]
    split_samples: bool,
    /// Report errors as a JSON file
    #[arg(long)]
    error_file: Option<String>,
    /// Produce compact JSON files
    #[arg(long)]
    compact: bool,
    /// Verbosity
    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,
}

fn get_categories<'a>(args: &'a Args, samples: &[CSample<'a>]) -> Result<Vec<Category<'a>>> {
    match &args.category {
        None => Ok(vec![None]),
        Some(key) => {
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
    }
}

fn get_years(args: &Args, samples: &[CSample]) -> Years {
    let mut years = None;
    for s in samples {
        years = match years {
            None => Some((s.year, s.year + 1)),
            Some((a, b)) => Some((a.min(s.year), b.max(s.year + 1))),
        };
    }
    let years = years.expect("there are samples");
    info!("years in input data: {}", pretty_period(&years));
    (years.0.max(args.start), years.1.min(args.end + 1))
}

fn get_periods(args: &Args, years: &Years) -> Vec<Years> {
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
    info!("periods: {}", pretty_periods(&periods));
    periods
}

fn pretty_period(p: &Years) -> String {
    format!("{}-{}", p.0, p.1 - 1)
}

fn pretty_periods(periods: &[Years]) -> String {
    if periods.len() >= 5 {
        pretty_periods(&periods[0..2]) + ", ..., " + &pretty_period(periods.last().unwrap())
    } else {
        periods.iter().map(pretty_period).collect_vec().join(", ")
    }
}

fn explain_metadata_one(k: &str, vv: &HashSet<&str>) -> String {
    let vals = vv.iter().copied().sorted().collect_vec();
    format!("{} = {}", k, vals.join(", "))
}

fn explain_metadata(metadata: &HashMap<&str, HashSet<&str>>) -> String {
    let keys = metadata.keys().copied().sorted().collect_vec();
    keys.iter()
        .map(|k| explain_metadata_one(k, &metadata[k]))
        .join("; ")
}

fn statistics(samples: &[ISample]) {
    let mut lemmas = HashSet::new();
    let mut token_metadata: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut sample_metadata: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut tokencount = 0;
    for s in samples {
        for (k, v) in s.metadata.iter() {
            sample_metadata.entry(k).or_default().insert(v);
        }
        for t in &s.tokens {
            tokencount += 1;
            for (k, v) in t.metadata.iter() {
                token_metadata.entry(k).or_default().insert(v);
            }
            lemmas.insert(&t.lemma);
        }
    }
    info!("before filtering: samples: {}", samples.len());
    info!("before filtering: tokens: {}", tokencount);
    info!("before filtering: distinct lemmas: {}", lemmas.len());
    info!(
        "token metadata categories: {}",
        explain_metadata(&token_metadata)
    );
    info!(
        "sample metadata categories: {}",
        explain_metadata(&sample_metadata)
    );
}

fn post_statistics(samples: &[CSample]) {
    let mut lemmas = HashSet::new();
    let mut tokencount = 0;
    for s in samples {
        for lemma in &s.tokens {
            tokencount += 1;
            lemmas.insert(lemma);
        }
    }
    info!("after filtering: samples: {}", samples.len());
    info!("after filtering: tokens: {}", tokencount);
    info!("after filtering: distinct lemmas: {}", lemmas.len());
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SubsetKey<'a> {
    category: Category<'a>,
    period: Years,
}

impl SubsetKey<'_> {
    fn pretty(&self) -> String {
        match &self.category {
            None => pretty_period(&self.period),
            Some((k, v)) => format!("{}, {} = {}", pretty_period(&self.period), k, v),
        }
    }
}

struct Subset<'a> {
    category: Category<'a>,
    period: Years,
    samples: Vec<Sample>,
    total_x: u64,
    total_y: u64,
    points: HashSet<Point>,
}

impl<'a> Subset<'a> {
    fn pretty(&self) -> String {
        self.key().pretty()
    }

    fn key(&self) -> SubsetKey {
        SubsetKey {
            category: self.category,
            period: self.period,
        }
    }

    fn get_point(&self) -> Point {
        Point {
            x: self.total_x,
            y: self.total_y,
        }
    }

    fn get_parent_period(&self, years: Years) -> SubsetKey<'a> {
        SubsetKey {
            category: self.category,
            period: years,
        }
    }

    fn get_parent_category(&self) -> SubsetKey<'a> {
        assert!(self.category.is_some());
        SubsetKey {
            category: None,
            period: self.period,
        }
    }

    fn get_parents(&self, years: Years) -> Vec<SubsetKey<'a>> {
        match self.category {
            None => vec![self.get_parent_period(years)],
            Some(_) => vec![self.get_parent_period(years), self.get_parent_category()],
        }
    }
}

fn get_sample<'a>(restrict_tokens: Category, s: &'a ISample) -> CSample<'a> {
    CSample {
        year: s.year,
        metadata: &s.metadata,
        words: s.words,
        tokens: s
            .tokens
            .iter()
            .filter_map(|t| {
                if categories::matches(restrict_tokens, &t.metadata) {
                    Some(&t.lemma as &str)
                } else {
                    None
                }
            })
            .collect_vec(),
    }
}

fn get_samples<'a>(
    restrict_samples: Category,
    restrict_tokens: Category,
    samples: &'a [ISample],
) -> Vec<CSample<'a>> {
    samples
        .iter()
        .filter_map(|s| {
            if categories::matches(restrict_samples, &s.metadata) {
                Some(get_sample(restrict_tokens, s))
            } else {
                None
            }
        })
        .collect_vec()
}

fn build_subset<'a>(
    measure_x: MeasureX,
    measure_y: MeasureY,
    samples: &[CSample<'a>],
    key: SubsetKey<'a>,
    split_samples: bool,
) -> Result<Subset<'a>> {
    let category = key.category;
    let period = key.period;
    let filter = |s: &&CSample| {
        period.0 <= s.year && s.year < period.1 && categories::matches(category, s.metadata)
    };
    let samples = samples.iter().filter(filter).collect_vec();

    let mut lemmas = HashSet::new();
    for s in &samples {
        lemmas.extend(&s.tokens);
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();
    let lemmamap: HashMap<&str, usize> = lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();
    let total_types = lemmas.len() as u64;

    let samples = if split_samples {
        assert_eq!(measure_x, MeasureX::Tokens);
        let mut split = vec![];
        for s in samples {
            for lemma in &s.tokens {
                let token = SToken {
                    count: 1,
                    id: lemmamap[lemma],
                };
                split.push(Sample {
                    x: 1,
                    token_count: 1,
                    tokens: vec![token],
                })
            }
        }
        split
    } else {
        samples
            .into_iter()
            .map(|s| {
                let mut tokencount = HashMap::new();
                for lemma in &s.tokens {
                    let id = lemmamap[lemma];
                    *tokencount.entry(id).or_insert(0) += 1;
                }
                let mut tokens = tokencount
                    .iter()
                    .map(|(&id, &count)| SToken { id, count })
                    .collect_vec();
                tokens.sort_by_key(|t| t.id);
                let token_count = tokens.iter().map(|t| t.count).sum();
                let x = match measure_x {
                    MeasureX::Tokens => token_count,
                    MeasureX::Words => s.words,
                };
                Sample {
                    x,
                    token_count,
                    tokens,
                }
            })
            .collect_vec()
    };
    let total_x: u64 = samples.iter().map(|s| s.x).sum();
    let total_tokens = samples.iter().map(|s| s.token_count).sum();
    let total_y = match measure_y {
        MeasureY::Types => total_types,
        MeasureY::Tokens => total_tokens,
    };
    let s = Subset {
        category,
        period,
        samples,
        total_x,
        total_y,
        points: HashSet::new(),
    };
    debug!(
        "{}: {} samples, {} {} / {} {}",
        s.pretty(),
        s.samples.len(),
        s.total_y,
        measure_y,
        s.total_x,
        measure_x,
    );
    if total_x == 0 {
        return Err(errors::invalid_input(format!(
            "{}: zero-size subset",
            s.pretty()
        )));
    }
    Ok(s)
}

struct Curve<'a> {
    category: Category<'a>,
    keys: Vec<SubsetKey<'a>>,
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
    split_samples: bool,
}

impl<'a> Calc<'a> {
    fn new(args: &'a Args, input: &'a Input) -> Result<Calc<'a>> {
        let measure_x = if args.words {
            if args.split_samples {
                Err(errors::invalid_argument_ref(
                    "cannot select both --words and --split-samples",
                ))
            } else {
                Ok(MeasureX::Words)
            }
        } else {
            Ok(MeasureX::Tokens)
        }?;
        let measure_y = if args.count_tokens {
            MeasureY::Tokens
        } else {
            MeasureY::Types
        };
        statistics(&input.samples);
        let restrict_samples = categories::parse_restriction(&args.restrict_samples)?;
        let restrict_tokens = categories::parse_restriction(&args.restrict_tokens)?;
        let samples = get_samples(restrict_samples, restrict_tokens, &input.samples);
        post_statistics(&samples);
        if samples.is_empty() {
            return Err(errors::invalid_input_ref("no samples found"));
        }
        let categories = get_categories(args, &samples)?;
        let years = get_years(args, &samples);
        let periods = get_periods(args, &years);
        let curves = build_curves(&categories, &periods);
        let mut subset_map = HashMap::new();
        for curve in &curves {
            for key in &curve.keys {
                let subset =
                    build_subset(measure_x, measure_y, &samples, *key, args.split_samples)?;
                let point = subset.get_point();
                let parents = subset.get_parents(years);
                subset_map.insert(*key, subset);
                for parent in &parents {
                    let x = match subset_map.entry(*parent) {
                        Occupied(e) => e.into_mut(),
                        Vacant(e) => e.insert(build_subset(
                            measure_x,
                            measure_y,
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
            measure_y,
            measure_x,
            restrict_samples,
            restrict_tokens,
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
        debug!("size limit: {} {}", limit, self.measure_x);
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
        debug!("{}: calculated {} points", subset.pretty(), results.len());
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
            avg_string(&average_at_limit),
            self.measure_y,
            limit,
            self.measure_x
        ));
        let p = subset.get_point();
        let vs_time = {
            let k = subset.get_parent_period(self.years);
            let pr = top_results[&(k, p)];
            msg.push_str(&format!(", {} vs. other time points", point_string(&pr)));
            pr
        };
        let vs_categories = match subset.category {
            None => None,
            Some(_) => {
                let k = subset.get_parent_category();
                let pr = top_results[&(k, p)];
                msg.push_str(&format!(", {} vs. other categories", point_string(&pr)));
                Some(pr)
            }
        };
        debug!("{msg}");
        OResult {
            period: subset.period,
            average_at_limit,
            vs_time,
            vs_categories,
        }
    }
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    let output = Calc::new(args, &input)?.calc()?;
    info!("write: {}", args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    if args.compact {
        serde_json::to_writer(writer, &output)?;
    } else {
        serde_json::to_writer_pretty(writer, &output)?;
    }
    Ok(())
}

fn store_error(error_file: &str, e: &dyn error::Error) -> Result<()> {
    let error = OError {
        error: format!("{e}"),
    };
    let file = fs::File::create(error_file)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer(writer, &error)?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    pretty_env_logger::formatted_timed_builder()
        .filter_level(args.verbose.log_level_filter())
        .init();
    match process(&args) {
        Ok(()) => (),
        Err(e) => {
            match args.error_file {
                Some(filename) => match store_error(&filename, &*e) {
                    Ok(()) => {
                        info!("error reported: {e}");
                    }
                    Err(e2) => {
                        error!("{e}");
                        error!("{e2}");
                    }
                },
                None => error!("{e}"),
            }
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pretty_period_basic() {
        assert_eq!(pretty_period(&(1990, 2000)), "1990-1999");
    }

    #[test]
    fn pretty_periods_basic() {
        assert_eq!(pretty_periods(&[(1990, 2000)]), "1990-1999");
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010)]),
            "1990-1999, 2000-2009"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020)]),
            "1990-1999, 2000-2009, 2010-2019"
        );
        assert_eq!(
            pretty_periods(&[(1990, 2000), (2000, 2010), (2010, 2020), (2020, 2030)]),
            "1990-1999, 2000-2009, 2010-2019, 2020-2029"
        );
        assert_eq!(
            pretty_periods(&[
                (1990, 2000),
                (2000, 2010),
                (2010, 2020),
                (2020, 2030),
                (2030, 2040)
            ]),
            "1990-1999, 2000-2009, ..., 2030-2039"
        );
        assert_eq!(
            pretty_periods(&[
                (1990, 2000),
                (2000, 2010),
                (2010, 2020),
                (2020, 2030),
                (2030, 2040),
                (2040, 2050)
            ]),
            "1990-1999, 2000-2009, ..., 2040-2049"
        );
    }
}

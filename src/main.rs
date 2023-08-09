use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::{error, fs, process, result};
use types3::calculation;
use types3::calculation::{Point, PointResult, SToken, Sample};
use types3::input::{ISample, Input, Year};

const DEFAULT_ITER: u64 = 1_000_000;

type Result<T> = result::Result<T, Box<dyn error::Error>>;
type Years = (Year, Year);

#[derive(Debug, Clone)]
struct InvalidInput(String);

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid input: {}", self.0)
    }
}

fn invalid_input(s: String) -> Box<dyn error::Error> {
    InvalidInput(s).into()
}

fn invalid_input_ref(s: &str) -> Box<dyn error::Error> {
    InvalidInput(s.to_owned()).into()
}

impl error::Error for InvalidInput {}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file
    infile: String,
    /// Metadata category
    #[arg(long)]
    category: Option<String>,
    /// Count words (instead of tokens)
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
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

fn calc_periods(args: &Args, years: &Years) -> Vec<(bool, Years)> {
    let mut periods = vec![(false, *years)];
    let mut y = args.offset;
    while y + args.step <= years.0 {
        y += args.step;
    }
    loop {
        let p = (y, y + args.window);
        periods.push((true, p));
        if p.1 >= years.1 {
            break;
        }
        y += args.step;
    }
    periods
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Category {
    All,
    Subset(String, String),
}

impl Category {
    fn matches(&self, sample: &ISample) -> bool {
        match self {
            Category::All => true,
            Category::Subset(k, v) => sample.metadata.get(k) == Some(v),
        }
    }
}

fn get_categories(args: &Args, samples: &[ISample]) -> Result<Vec<(bool, Category)>> {
    match &args.category {
        None => Ok(vec![(true, Category::All)]),
        Some(key) => {
            let mut categories = vec![(false, Category::All)];
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
                return Err(invalid_input(format!(
                    "there are no samples with metadata key {}",
                    key
                )));
            }
            let mut values = values.into_iter().collect_vec();
            values.sort();
            categories.extend(
                values
                    .into_iter()
                    .map(|val| (true, Category::Subset(key.to_owned(), val.to_owned()))),
            );
            Ok(categories)
        }
    }
}

fn get_years(args: &Args, samples: &[ISample]) -> Years {
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

fn get_periods(args: &Args, years: Years) -> Vec<(bool, Years)> {
    let periods = calc_periods(args, &years);
    let relevant = periods
        .iter()
        .filter_map(|(r, p)| if *r { Some(*p) } else { None })
        .collect_vec();
    info!("periods: {}", pretty_periods(&relevant));
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

fn statistics(samples: &[ISample]) {
    let mut lemmas = HashSet::new();
    let mut metadata_keys = HashSet::new();
    for s in samples {
        for k in s.metadata.keys() {
            metadata_keys.insert(k);
        }
        for t in &s.tokens {
            lemmas.insert(&t.lemma);
        }
    }
    info!("distinct lemmas: {}", lemmas.len());
    let mut metadata_keys = metadata_keys.into_iter().collect_vec();
    metadata_keys.sort();
    info!(
        "metadata categories: {}",
        if metadata_keys.is_empty() {
            "-".to_owned()
        } else {
            metadata_keys.into_iter().cloned().collect_vec().join(", ")
        }
    );
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct SubsetKey {
    category: Category,
    period: Years,
}

impl SubsetKey {
    fn pretty(&self) -> String {
        match &self.category {
            Category::All => pretty_period(&self.period),
            Category::Subset(k, v) => format!("{}, {} = {}", pretty_period(&self.period), k, v),
        }
    }
}

struct Subset {
    category: Category,
    period: Years,
    samples: Vec<Sample>,
    total_size: u64,
    total_types: u64,
    relevant: bool,
    points: HashSet<Point>,
}

impl Subset {
    fn pretty(&self) -> String {
        self.key().pretty()
    }

    fn key(&self) -> SubsetKey {
        SubsetKey {
            category: self.category.clone(),
            period: self.period,
        }
    }

    fn get_point(&self) -> Point {
        Point {
            size: self.total_size,
            types: self.total_types,
        }
    }

    fn get_parent_period(&self, years: Years) -> SubsetKey {
        SubsetKey {
            category: self.category.clone(),
            period: years,
        }
    }

    fn get_parent_category(&self) -> SubsetKey {
        assert!(self.category != Category::All);
        SubsetKey {
            category: Category::All,
            period: self.period,
        }
    }

    fn get_parents(&self, years: Years) -> Vec<SubsetKey> {
        match self.category {
            Category::All => vec![self.get_parent_period(years)],
            Category::Subset(_, _) => {
                vec![self.get_parent_period(years), self.get_parent_category()]
            }
        }
    }
}

fn build_subsets(
    args: &Args,
    samples: &[ISample],
    categories: &[(bool, Category)],
    periods: &[(bool, Years)],
) -> Vec<Subset> {
    let mut subsets = Vec::new();
    for (r1, category) in categories {
        for (r2, period) in periods {
            let filter =
                |s: &&ISample| period.0 <= s.year && s.year < period.1 && category.matches(s);
            let samples = samples.iter().filter(filter).collect_vec();

            let mut lemmas = HashSet::new();
            for s in &samples {
                for t in &s.tokens {
                    lemmas.insert(&t.lemma);
                }
            }
            let mut lemmas = lemmas.into_iter().collect_vec();
            lemmas.sort();
            let lemmamap: HashMap<&String, usize> =
                lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();
            let total_types = lemmas.len() as u64;

            let samples = samples
                .into_iter()
                .map(|s| {
                    let mut tokencount = HashMap::new();
                    for t in &s.tokens {
                        let id = lemmamap[&t.lemma];
                        *tokencount.entry(id).or_insert(0) += 1;
                    }
                    let mut tokens = tokencount
                        .iter()
                        .map(|(&id, &count)| SToken { id, count })
                        .collect_vec();
                    tokens.sort_by_key(|t| t.id);
                    let size = if args.words {
                        s.words
                    } else {
                        s.tokens.len() as u64
                    };
                    Sample { size, tokens }
                })
                .collect_vec();
            let total_size: u64 = samples.iter().map(|s| s.size).sum();
            let relevant = *r1 && *r2;
            let s = Subset {
                category: category.clone(),
                period: *period,
                samples,
                total_size,
                total_types,
                relevant,
                points: HashSet::new(),
            };
            debug!(
                "{}: {} samples, {} types / {} size",
                s.pretty(),
                s.samples.len(),
                s.total_types,
                s.total_size,
            );
            subsets.push(s);
        }
    }
    subsets
}

type TopResults = HashMap<(SubsetKey, Point), PointResult>;

struct Calc {
    years: Years,
    relevant_keys: Vec<SubsetKey>,
    top_keys: Vec<SubsetKey>,
    subset_map: HashMap<SubsetKey, Subset>,
    iter: u64,
}

impl Calc {
    fn new(args: &Args, input: &Input) -> Result<Calc> {
        info!("samples: {}", input.samples.len());
        if input.samples.is_empty() {
            return Err(invalid_input_ref("no samples"));
        }
        statistics(&input.samples);
        let categories = get_categories(args, &input.samples)?;
        let years = get_years(args, &input.samples);
        let periods = get_periods(args, years);
        let subsets = build_subsets(args, &input.samples, &categories, &periods);
        let keys = subsets.iter().map(|s| s.key()).collect_vec();
        let mut subset_map: HashMap<SubsetKey, Subset> =
            subsets.into_iter().map(|s| (s.key(), s)).collect();
        let relevant_keys = keys
            .iter()
            .cloned()
            .filter(|k| subset_map[k].relevant)
            .collect_vec();
        for k in &relevant_keys {
            let s = &subset_map[k];
            let point = s.get_point();
            let parents = s.get_parents(years);
            for parent in parents {
                subset_map.entry(parent).and_modify(|s1| {
                    s1.points.insert(point);
                });
            }
        }
        let top_keys = keys
            .into_iter()
            .filter(|k| !subset_map[k].points.is_empty())
            .collect_vec();
        let iter = args.iter;
        Ok(Calc {
            years,
            relevant_keys,
            top_keys,
            subset_map,
            iter,
        })
    }

    fn calc(&self) -> Result<()> {
        let mut top_results = HashMap::new();
        let top = self
            .top_keys
            .iter()
            .map(|k| &self.subset_map[k])
            .collect_vec();
        for &subset in &top {
            self.calc_top(subset, &mut top_results);
        }
        let relevant = self
            .relevant_keys
            .iter()
            .map(|k| &self.subset_map[k])
            .collect_vec();
        let size_limit = relevant
            .iter()
            .map(|p| p.total_size)
            .min()
            .expect("at least one subset");
        debug!("size limit: {}", size_limit);
        for &subset in &relevant {
            self.calc_relevant(subset, size_limit, &top_results);
        }
        Ok(())
    }

    fn calc_top(&self, subset: &Subset, top_results: &mut TopResults) {
        let key = subset.key();
        let mut points = subset.points.iter().cloned().collect_vec();
        points.sort();
        let results = calculation::compare_with_points(&subset.samples, self.iter, &points);
        for (i, p) in points.into_iter().enumerate() {
            top_results.insert((key.clone(), p), results[i]);
        }
        debug!("{}: calculated {} points", subset.pretty(), results.len());
    }

    fn calc_relevant(&self, subset: &Subset, limit: u64, top_results: &TopResults) {
        let mut msg = format!("{}: ", subset.pretty());
        let ar = calculation::average_at_limit(&subset.samples, self.iter, limit);
        msg.push_str(&format!(
            "{:.2}–{:.2} types / {} size",
            ar.types_low, ar.types_high, limit
        ));
        let p = subset.get_point();
        let k = subset.get_parent_period(self.years);
        let pr = top_results[&(k, p)];
        msg.push_str(&format!(", {} vs. other time points", symbols(pr)));
        if subset.category != Category::All {
            let k = subset.get_parent_category();
            let pr = top_results[&(k, p)];
            msg.push_str(&format!(", {} vs. other categories", symbols(pr)));
        }
        debug!("{msg}");
    }
}

fn symbols(pr: PointResult) -> &'static str {
    if pr.above > 0.9999 {
        "++++"
    } else if pr.above > 0.999 {
        "+++"
    } else if pr.above > 0.99 {
        "++"
    } else if pr.above > 0.9 {
        "+"
    } else if pr.below > 0.9999 {
        "----"
    } else if pr.below > 0.999 {
        "---"
    } else if pr.below > 0.99 {
        "--"
    } else if pr.below > 0.9 {
        "-"
    } else {
        "0"
    }
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    let calc = Calc::new(args, &input)?;
    calc.calc()?;
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
            error!("{e}");
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

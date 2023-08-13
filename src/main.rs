use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::{error, fmt, fs, io, process, result};
use types3::calculation::{self, Point, SToken, Sample};
use types3::input::{ISample, Input, Year};
use types3::output::{
    avg_string, point_string, Measure, OCategory, OCurve, OResult, Output, PointResult, Years,
};

const DEFAULT_ITER: u64 = 1_000_000;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
struct InvalidInput(String);

#[derive(Debug)]
struct InvalidArgument(String);

impl fmt::Display for InvalidInput {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid input: {}", self.0)
    }
}

impl fmt::Display for InvalidArgument {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid argument: {}", self.0)
    }
}

impl error::Error for InvalidInput {}

impl error::Error for InvalidArgument {}

fn invalid_input(s: String) -> Box<dyn error::Error> {
    InvalidInput(s).into()
}

fn invalid_input_ref(s: &str) -> Box<dyn error::Error> {
    InvalidInput(s.to_owned()).into()
}

fn invalid_argument(s: String) -> Box<dyn error::Error> {
    InvalidArgument(s).into()
}

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
    /// Sample category restriction, of the form key=value
    #[arg(long)]
    restrict_samples: Option<String>,
    /// Token category restriction, of the form key=value
    #[arg(long)]
    restrict_tokens: Option<String>,
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

type Category<'a> = Option<(&'a str, &'a str)>;

fn owned_cat(category: Category) -> OCategory {
    category.map(|(k, v)| (k.to_owned(), v.to_owned()))
}

fn matches(category: Category, metadata: &HashMap<String, String>) -> bool {
    match category {
        None => true,
        Some((k, v)) => match metadata.get(k) {
            None => false,
            Some(v2) => v == v2,
        },
    }
}

fn parse_restriction(arg: &Option<String>) -> Result<Category> {
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
                return Err(invalid_input(format!(
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
    total_size: u64,
    total_types: u64,
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
            size: self.total_size,
            types: self.total_types,
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

struct CSample<'a> {
    year: Year,
    metadata: &'a HashMap<String, String>,
    words: u64,
    tokens: Vec<&'a str>,
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
                if matches(restrict_tokens, &t.metadata) {
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
            if matches(restrict_samples, &s.metadata) {
                Some(get_sample(restrict_tokens, s))
            } else {
                None
            }
        })
        .collect_vec()
}

fn build_subset<'a>(measure: Measure, samples: &[CSample<'a>], key: SubsetKey<'a>) -> Subset<'a> {
    let category = key.category;
    let period = key.period;
    let filter =
        |s: &&CSample| period.0 <= s.year && s.year < period.1 && matches(category, s.metadata);
    let samples = samples.iter().filter(filter).collect_vec();

    let mut lemmas = HashSet::new();
    for s in &samples {
        lemmas.extend(&s.tokens);
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();
    let lemmamap: HashMap<&str, usize> = lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();
    let total_types = lemmas.len() as u64;

    let samples = samples
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
            let size = match measure {
                Measure::Tokens => s.tokens.len() as u64,
                Measure::Words => s.words,
            };
            Sample { size, tokens }
        })
        .collect_vec();
    let total_size: u64 = samples.iter().map(|s| s.size).sum();
    let s = Subset {
        category,
        period,
        samples,
        total_size,
        total_types,
        points: HashSet::new(),
    };
    debug!(
        "{}: {} samples, {} types / {} {}",
        s.pretty(),
        s.samples.len(),
        s.total_types,
        s.total_size,
        measure,
    );
    s
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
    measure: Measure,
    restrict_samples: Category<'a>,
    restrict_tokens: Category<'a>,
}

impl<'a> Calc<'a> {
    fn new(args: &'a Args, input: &'a Input) -> Result<Calc<'a>> {
        let measure = if args.words {
            Measure::Words
        } else {
            Measure::Tokens
        };
        statistics(&input.samples);
        let restrict_samples = parse_restriction(&args.restrict_samples)?;
        let restrict_tokens = parse_restriction(&args.restrict_tokens)?;
        let samples = get_samples(restrict_samples, restrict_tokens, &input.samples);
        post_statistics(&samples);
        if samples.is_empty() {
            return Err(invalid_input_ref("no samples found"));
        }
        let categories = get_categories(args, &samples)?;
        let years = get_years(args, &samples);
        let periods = get_periods(args, &years);
        let curves = build_curves(&categories, &periods);
        let mut subset_map = HashMap::new();
        for curve in &curves {
            for key in &curve.keys {
                let subset = build_subset(measure, &samples, *key);
                let point = subset.get_point();
                let parents = subset.get_parents(years);
                subset_map.insert(*key, subset);
                for parent in &parents {
                    subset_map
                        .entry(*parent)
                        .or_insert_with(|| build_subset(measure, &samples, *parent))
                        .points
                        .insert(point);
                }
            }
        }
        let iter = args.iter;
        Ok(Calc {
            years,
            periods,
            curves,
            subset_map,
            iter,
            measure,
            restrict_samples,
            restrict_tokens,
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
            .map(|k| self.subset_map[k].total_size)
            .min()
            .expect("at least one period")
    }

    fn calc(self) -> Result<Output> {
        let mut top_results = HashMap::new();
        for subset in self.subset_map.values() {
            self.calc_top(subset, &mut top_results);
        }
        let limit = self.size_limit();
        debug!("size limit: {} {}", limit, self.measure);
        let curves = self
            .curves
            .iter()
            .map(|c| self.calc_curve(c, limit, &top_results))
            .collect_vec();
        Ok(Output {
            curves,
            years: self.years,
            periods: self.periods,
            measure: self.measure,
            iter: self.iter,
            limit,
            restrict_tokens: owned_cat(self.restrict_tokens),
            restrict_samples: owned_cat(self.restrict_samples),
        })
    }

    fn calc_top(&self, subset: &'a Subset, top_results: &mut TopResults<'a>) {
        if subset.points.is_empty() {
            return;
        }
        let mut points = subset.points.iter().copied().collect_vec();
        let key = subset.key();
        points.sort();
        let results = calculation::compare_with_points(&subset.samples, self.iter, &points);
        for (i, p) in points.into_iter().enumerate() {
            top_results.insert((key, p), results[i]);
        }
        debug!("{}: calculated {} points", subset.pretty(), results.len());
    }

    fn calc_curve(&self, curve: &Curve, limit: u64, top_results: &TopResults) -> OCurve {
        OCurve {
            category: owned_cat(curve.category),
            results: curve
                .keys
                .iter()
                .map(|k| self.calc_relevant(&self.subset_map[k], limit, top_results))
                .collect_vec(),
        }
    }

    fn calc_relevant(&self, subset: &Subset, limit: u64, top_results: &TopResults) -> OResult {
        let mut msg = format!("{}: ", subset.pretty());
        let average_at_limit = calculation::average_at_limit(&subset.samples, self.iter, limit);
        msg.push_str(&format!(
            "{} types / {} {}",
            avg_string(&average_at_limit),
            limit,
            self.measure
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
    serde_json::to_writer_pretty(writer, &output)?;
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

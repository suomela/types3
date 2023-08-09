use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::{error, fs, process, result};
use types3::calculation;
use types3::calculation::{SToken, Sample};
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

#[derive(Clone, PartialEq, Eq, Hash)]
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

fn get_periods(args: &Args, samples: &[ISample]) -> Vec<(bool, Years)> {
    let mut years = None;
    for s in samples {
        years = match years {
            None => Some((s.year, s.year + 1)),
            Some((a, b)) => Some((a.min(s.year), b.max(s.year + 1))),
        };
    }
    let years = years.expect("there are samples");
    info!("years in input data: {}", pretty_period(&years));
    let years = (years.0.max(args.start), years.1.min(args.end + 1));
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

#[derive(PartialEq, Eq, Hash)]
struct SubsetKey {
    category: Category,
    period: Years,
}

struct Subset {
    category: Category,
    period: Years,
    samples_by_words: Vec<Sample>,
    samples_by_tokens: Vec<Sample>,
    total_words: u64,
    total_tokens: u64,
    total_types: usize,
    relevant: bool,
}

impl Subset {
    fn pretty(&self) -> String {
        match &self.category {
            Category::All => pretty_period(&self.period),
            Category::Subset(k, v) => format!("{}, {} = {}", pretty_period(&self.period), k, v),
        }
    }

    fn key(&self) -> SubsetKey {
        SubsetKey {
            category: self.category.clone(),
            period: self.period,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Limit {
    Words(u64),
    Tokens(u64),
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Limit::Tokens(tokens) => write!(f, "{} tokens", tokens),
            Limit::Words(words) => write!(f, "{} words", words),
        }
    }
}

fn build_subsets(
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
            let total_words: u64 = samples.iter().map(|s| s.words).sum();
            let total_tokens: u64 = samples.iter().map(|s| s.tokens.len() as u64).sum();

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
            let total_types = lemmas.len();

            let mut samples_by_tokens = Vec::new();
            let mut samples_by_words = Vec::new();

            for s in samples {
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
                samples_by_tokens.push(Sample {
                    size: s.tokens.len() as u64,
                    tokens: tokens.clone(),
                });
                samples_by_words.push(Sample {
                    size: s.words,
                    tokens,
                });
            }
            let relevant = *r1 && *r2;
            let s = Subset {
                category: category.clone(),
                period: *period,
                samples_by_tokens,
                samples_by_words,
                total_words,
                total_tokens,
                total_types,
                relevant,
            };
            debug!(
                "{}: {} samples, {} words, {} tokens, {} lemmas",
                s.pretty(),
                s.samples_by_tokens.len(),
                s.total_words,
                s.total_tokens,
                s.total_types,
            );
            subsets.push(s);
        }
    }
    subsets
}

struct Calc {
    relevant: Vec<SubsetKey>,
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
        let periods = get_periods(args, &input.samples);
        let subsets = build_subsets(&input.samples, &categories, &periods);
        let relevant = subsets
            .iter()
            .filter_map(|s| if s.relevant { Some(s.key()) } else { None })
            .collect_vec();
        let subset_map: HashMap<SubsetKey, Subset> =
            subsets.into_iter().map(|s| (s.key(), s)).collect();
        let iter = args.iter;
        Ok(Calc {
            relevant,
            subset_map,
            iter,
        })
    }

    fn calc(&self) -> Result<()> {
        let relevant = self
            .relevant
            .iter()
            .map(|k| &self.subset_map[k])
            .collect_vec();
        let word_limit = relevant
            .iter()
            .map(|p| p.total_words)
            .min()
            .expect("at least one subset");
        let token_limit = relevant
            .iter()
            .map(|p| p.total_tokens)
            .min()
            .expect("at least one subset");
        debug!("thresholds: {} words, {} tokens", word_limit, token_limit);
        for &subset in &relevant {
            self.limited(subset, Limit::Words(word_limit));
        }
        for &subset in &relevant {
            self.limited(subset, Limit::Tokens(token_limit));
        }
        Ok(())
    }

    fn limited(&self, subset: &Subset, limit: Limit) {
        let r = match limit {
            Limit::Tokens(tokens) => {
                calculation::average_at_limit(&subset.samples_by_tokens, self.iter, tokens)
            }
            Limit::Words(words) => {
                calculation::average_at_limit(&subset.samples_by_words, self.iter, words)
            }
        };
        debug!(
            "{}: {:.2}–{:.2} types / {}",
            subset.pretty(),
            r.types_low,
            r.types_high,
            limit
        );
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

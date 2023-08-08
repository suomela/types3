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

impl error::Error for InvalidInput {}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file
    infile: String,
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

fn calc_periods(args: &Args, years: &Years) -> Vec<Years> {
    let mut periods = Vec::new();
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
    periods
}

fn get_periods(args: &Args, samples: &[ISample]) -> Vec<Years> {
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

fn statistics(samples: &[ISample]) {
    let mut lemmas = HashSet::new();
    for s in samples {
        for t in &s.tokens {
            lemmas.insert(&t.lemma);
        }
    }
    info!("distinct lemmas: {}", lemmas.len());
}

struct Period {
    period: Years,
    samples_by_words: Vec<Sample>,
    samples_by_tokens: Vec<Sample>,
    total_words: u64,
    total_tokens: u64,
    total_lemmas: usize,
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

fn limited(args: &Args, period: &Period, limit: Limit) {
    let r = match limit {
        Limit::Tokens(tokens) => calculation::count(&period.samples_by_tokens, args.iter, tokens),
        Limit::Words(words) => calculation::count(&period.samples_by_words, args.iter, words),
    };
    debug!(
        "{}: {:.2}–{:.2} types / {}",
        pretty_period(&period.period),
        r.types_low,
        r.types_high,
        limit
    );
}

fn build_periods(samples: &[ISample], periods: Vec<Years>) -> Vec<Period> {
    periods
        .into_iter()
        .map(|period| {
            let in_period = |s: &&ISample| period.0 <= s.year && s.year < period.1;
            let samples = samples.iter().filter(in_period).collect_vec();
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
            let total_lemmas = lemmas.len();

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
            let p = Period {
                period,
                samples_by_tokens,
                samples_by_words,
                total_words,
                total_tokens,
                total_lemmas,
            };
            debug!(
                "{}: {} samples, {} words, {} tokens, {} lemmas",
                pretty_period(&p.period),
                p.samples_by_tokens.len(),
                p.total_words,
                p.total_tokens,
                p.total_lemmas,
            );
            p
        })
        .collect_vec()
}

fn calc(args: &Args, input: &Input) -> Result<()> {
    info!("samples: {}", input.samples.len());
    if input.samples.is_empty() {
        return Err(InvalidInput("no samples".to_owned()).into());
    }
    statistics(&input.samples);
    let periods = get_periods(args, &input.samples);
    let periods = build_periods(&input.samples, periods);

    let word_limit = periods
        .iter()
        .map(|p| p.total_words)
        .min()
        .expect("at least one period");
    let token_limit = periods
        .iter()
        .map(|p| p.total_tokens)
        .min()
        .expect("at least one period");
    debug!("thresholds: {} words, {} tokens", word_limit, token_limit);

    for period in &periods {
        limited(args, period, Limit::Words(word_limit));
    }
    for period in &periods {
        limited(args, period, Limit::Tokens(token_limit));
    }

    Ok(())
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    calc(args, &input)?;
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

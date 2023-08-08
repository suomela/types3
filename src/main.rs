use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::{error, fs, process, result};
use types3::input::{ISample, Input, Year};

const DEFAULT_ITER: u64 = 100_000;

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
    #[arg(long, default_value_t = DEFAULT_ITER)]
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

fn get_lemmas(samples: &[ISample]) -> Vec<&String> {
    let mut lemmas = HashSet::new();
    for s in samples {
        for t in &s.tokens {
            lemmas.insert(&t.lemma);
        }
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();
    info!("distinct lemmas: {}", lemmas.len());
    lemmas
}

struct Period<'a> {
    period: Years,
    samples: Vec<&'a ISample>,
    total_words: u64,
    total_tokens: usize,
}

fn calc(args: &Args, input: &Input) -> Result<()> {
    info!("samples: {}", input.samples.len());
    if input.samples.is_empty() {
        return Err(InvalidInput("no samples".to_owned()).into());
    }
    let periods = get_periods(args, &input.samples);
    let lemmas = get_lemmas(&input.samples);
    let _lemmamap: HashMap<&String, usize> =
        lemmas.iter().enumerate().map(|(i, &x)| (x, i)).collect();

    let periods = periods
        .into_iter()
        .map(|period| {
            let in_period = |s: &&ISample| period.0 <= s.year && s.year < period.1;
            let samples = input.samples.iter().filter(in_period).collect_vec();
            let total_words: u64 = samples.iter().map(|s| s.words).sum();
            let total_tokens: usize = samples.iter().map(|s| s.tokens.len()).sum();
            let p = Period {
                period,
                samples,
                total_words,
                total_tokens,
            };
            debug!(
                "{}: samples: {}, words: {}, tokens: {}",
                pretty_period(&p.period),
                p.samples.len(),
                p.total_words,
                p.total_tokens
            );
            p
        })
        .collect_vec();
    let min_words = periods.iter().map(|p| p.total_words).min().expect("at least one period");
    debug!("threshold: {} words", min_words);
    
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

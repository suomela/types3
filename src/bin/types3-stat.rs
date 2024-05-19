use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use itertools::Itertools;
use log::{error, info};
use std::collections::{HashMap, HashSet};
use std::{error, fs, io, process};
use types3::driver;
use types3::errors::{self, Result};
use types3::input::{ISample, Input, Year};
use types3::output::{self, OError, Years};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file (JSON)
    infile: String,
    /// Output file (JSON)
    outfile: String,
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

pub fn get_years(samples: &[ISample]) -> Years {
    let mut years = None;
    for s in samples {
        years = match years {
            None => Some((s.year, s.year + 1)),
            Some((a, b)) => Some((a.min(s.year), b.max(s.year + 1))),
        };
    }
    years.expect("there are samples")
}

struct RawStat<'a> {
    samples: u64,
    words: u64,
    tokens: u64,
    types: HashSet<&'a String>,
}

impl<'a> RawStat<'a> {
    fn new() -> Self {
        Self {
            samples: 0,
            words: 0,
            tokens: 0,
            types: HashSet::new(),
        }
    }

    fn feed_sample(&mut self, sample: &'a ISample) {
        self.samples += 1;
        self.words += sample.words;
        for token in &sample.tokens {
            self.tokens += 1;
            self.types.insert(&token.lemma);
        }
    }
}

type MdPair<'a> = (&'a String, &'a String);

fn stat(args: &Args, samples: &[ISample]) -> Result<()> {
    if samples.is_empty() {
        return Err(errors::invalid_input_ref("no samples found"));
    }
    let years = get_years(samples);
    info!(target: "types3", "years in input data: {}", output::pretty_period(&years));
    let years = (years.0.max(args.start), years.1.min(args.end + 1));
    let mut periods = driver::get_periods(args.offset, args.window, args.step, &years);
    periods.push(years);

    let samples = samples
        .iter()
        .filter(|s| years.0 <= s.year && s.year <= years.1)
        .collect_vec();

    let mut smd: HashSet<MdPair> = HashSet::new();
    for sample in &samples {
        for md in &sample.metadata {
            smd.insert(md);
        }
    }
    let mut smd: Vec<MdPair> = smd.into_iter().collect_vec();
    smd.sort();
    let smd_map: HashMap<MdPair, usize> = smd.iter().enumerate().map(|(i, &x)| (x, i)).collect();

    for period in &periods {
        let mut overall = RawStat::new();
        let mut by_smd = (0..smd.len()).map(|_| RawStat::new()).collect_vec();
        for sample in &samples {
            if period.0 <= sample.year && sample.year <= period.1 {
                overall.feed_sample(sample);
                for md in &sample.metadata {
                    by_smd[smd_map[&md]].feed_sample(sample);
                }
            }
        }
        println!("period: {}", output::pretty_period(period));
        println!("- samples: {}", overall.samples);
        println!("- words: {}", overall.words);
        println!("- tokens: {}", overall.tokens);
        println!("- types: {}", overall.types.len());
        for (i, md) in smd.iter().enumerate() {
            println!("  {} = {}:", md.0, md.1);
            let s = &by_smd[i];
            println!("  - samples: {}", s.samples);
            println!("  - words: {}", s.words);
            println!("  - tokens: {}", s.tokens);
            println!("  - types: {}", s.types.len());
        }
    }
    Ok(())
}

fn process(args: &Args) -> Result<()> {
    info!(target: "types3", "read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    stat(args, &input.samples)?;
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
                        info!(target: "types3", "error reported: {e}");
                    }
                    Err(e2) => {
                        error!(target: "types3", "{e}");
                        error!(target: "types3", "{e2}");
                    }
                },
                None => error!(target: "types3", "{e}"),
            }
            process::exit(1);
        }
    }
}

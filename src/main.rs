use clap::Parser;
use itertools::Itertools;
use log::{error, info};
use std::collections::{HashMap, HashSet};
use std::{error, fs, process, result};
use types3::calculation::{Driver, Flavors, SToken, Sample};
use types3::input::Input;

const DEFAULT_ITER: u64 = 100_000;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file
    infile: String,
    /// Number of iterations
    #[arg(short, long, default_value_t = DEFAULT_ITER)]
    iter: u64,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn calc(args: &Args, input: &Input) {
    let mut lemmas = HashSet::new();
    let mut flavors: HashMap<&String, HashSet<&String>> = HashMap::new();
    for s in &input.samples {
        for t in &s.tokens {
            lemmas.insert(&t.lemma);
            for (k, v) in &t.metadata {
                flavors.entry(k).or_insert(HashSet::new()).insert(v);
            }
        }
    }
    let mut lemmas = lemmas.iter().collect_vec();
    lemmas.sort();
    let lemmamap: HashMap<&String, usize> =
        lemmas.iter().enumerate().map(|(i, &&x)| (x, i)).collect();
    let mut flavorkeys = flavors.keys().collect_vec();
    flavorkeys.sort();
    let mut flavorstart = HashMap::new();
    let mut flavormap = HashMap::new();
    let mut flavorcount = 0;
    for &&x in &flavorkeys {
        flavorstart.insert(x, flavorcount);
        let mut flavorvalues = flavors[x].iter().copied().collect_vec();
        flavorvalues.sort();
        for y in flavorvalues {
            flavormap.insert((x, y), flavorcount);
            flavorcount += 1;
        }
    }
    info!(
        "input: {} samples, {} distinct lemmas, {} flavor tags, {} flavors",
        input.samples.len(),
        lemmas.len(),
        flavorkeys.len(),
        flavorcount,
    );
    assert!(flavorcount <= Flavors::BITS);
    let samples = input
        .samples
        .iter()
        .map(|s| {
            let mut tokencount = HashMap::new();
            for t in &s.tokens {
                let id = lemmamap[&t.lemma];
                let mut flavors = 0;
                for (k, v) in &t.metadata {
                    let flavor = flavormap[&(k, v)];
                    flavors |= 1 << flavor;
                }
                *tokencount.entry((id, flavors)).or_insert(0) += 1;
            }
            let mut tokens = tokencount
                .iter()
                .map(|(&(id, flavors), &count)| SToken { id, flavors, count })
                .collect_vec();
            tokens.sort_by_key(|t| t.id);
            Sample {
                words: s.words,
                tokens,
            }
        })
        .collect_vec();
    let driver = Driver::new(samples);
    let result = driver.count(args.iter).to_sums();
    info!(
        "finished: {} iterations, {}, {} result points",
        result.total,
        if result.exact { "exact" } else { "not exact" },
        result.total_points(),
    );
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    calc(args, &input);
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

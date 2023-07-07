use clap::Parser;
use console::style;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::{error, fs, process, result};
use types3::input::*;

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
    /// Show progress
    #[arg(short, long)]
    verbose: bool,
}

fn msg(verbose: bool, prefix: &str, tail: &str) {
    if verbose {
        eprintln!("{} {}", style(format!("{prefix:>12}")).blue().bold(), tail,);
    }
}

fn error(prefix: &str, tail: &str) {
    eprintln!("{} {}", style(format!("{prefix:>12}")).red().bold(), tail,);
    process::exit(1);
}

fn calc(args: &Args, input: &Input) {
    let mut lemmas = HashSet::new();
    let mut flavors: HashMap<String, HashSet<String>> = HashMap::new();
    for s in &input.samples {
        for t in &s.tokens {
            lemmas.insert(t.lemma.to_owned());
            for (k, v) in &t.metadata {
                flavors
                    .entry(k.to_owned())
                    .or_insert(HashSet::new())
                    .insert(v.to_owned());
            }
        }
    }
    let mut lemmas = lemmas.into_iter().collect_vec();
    lemmas.sort();

    msg(
        args.verbose,
        "Input",
        &format!(
            "{} samples, {} distinct lemmas, {} flavor tags",
            input.samples.len(),
            lemmas.len(),
            flavors.len(),
        ),
    );
}

fn process(args: &Args) -> Result<()> {
    msg(args.verbose, "Read", &args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    calc(args, &input);
    Ok(())
}

fn main() {
    let args = Args::parse();
    match process(&args) {
        Ok(()) => (),
        Err(e) => error("Error", &format!("{e}")),
    }
}

use clap::Parser;
use console::style;
use std::{error, fs, io, process, result};
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

fn process(args: &Args) -> Result<()> {
    msg(args.verbose, "Read", &args.infile);
    let file = fs::File::open(&args.infile)?;
    let reader = io::BufReader::new(file);
    let _: Input = serde_json::from_reader(reader)?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    match process(&args) {
        Ok(()) => (),
        Err(e) => error("Error", &format!("{e}")),
    }
}

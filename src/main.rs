use clap::Parser;
use console::style;
use std::{error, fmt, fs, io, process, result};
use types3::*;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Number of iterations
    iter: u64,
    /// Input file
    infile: String,
    /// Output file
    outfile: String,
    /// Show progress
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone)]
struct InvalidArgumentsError;

impl fmt::Display for InvalidArgumentsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "expected command line arguments: ITERATIONS INPUT_FILE OUTPUT_FILE"
        )
    }
}

impl error::Error for InvalidArgumentsError {}

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
    let samples: Vec<Sample> = serde_json::from_reader(reader)?;
    let driver = Driver::new_with_settings(samples, args.verbose);
    let result = driver.count(args.iter).to_sums();
    msg(args.verbose, "Write", &args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer(writer, &result)?;
    msg(
        args.verbose,
        "Finished",
        &format!(
            "{} iterations, {}, {} result points",
            result.total,
            if result.exact { "exact" } else { "not exact" },
            result.total_points(),
        ),
    );
    Ok(())
}

fn main() {
    let args = Args::parse();
    match process(&args) {
        Ok(()) => (),
        Err(e) => error("Error", &format!("{e}")),
    }
}

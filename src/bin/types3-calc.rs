use clap::Parser;
use console::style;
use std::{error, fs, io, process, result};
use types3::calculation::{Driver, Sample};

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
    /// Pretty print results
    #[arg(short, long)]
    pretty: bool,
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
    let indata = fs::read_to_string(&args.infile)?;
    let samples: Vec<Sample> = serde_json::from_str(&indata)?;
    let driver = Driver::new_with_settings(samples, args.verbose);
    let result = driver.count(args.iter).to_sums();
    msg(args.verbose, "Write", &args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    if args.pretty {
        serde_json::to_writer_pretty(writer, &result)?;
    } else {
        serde_json::to_writer(writer, &result)?;
    }
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

use clap::Parser;
use log::{error, info};
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
    /// Pretty print results
    #[arg(short, long)]
    pretty: bool,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let samples: Vec<Sample> = serde_json::from_str(&indata)?;
    let driver = Driver::new(samples);
    let result = driver.count(args.iter).to_sums();
    info!("write: {}", args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    if args.pretty {
        serde_json::to_writer_pretty(writer, &result)?;
    } else {
        serde_json::to_writer(writer, &result)?;
    }
    info!(
        "finished: {} iterations, {}, {} result points",
        result.total,
        if result.exact { "exact" } else { "not exact" },
        result.total_points(),
    );
    Ok(())
}

fn main() {
    let args = Args::parse();
    match process(&args) {
        Ok(()) => (),
        Err(e) => {
            error!("{e}");
            process::exit(1);
        }
    }
}

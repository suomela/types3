use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use log::{error, info};
use std::{error, fs, io, process};
use types3::categories;
use types3::driver::{Calc, DriverArgs};
use types3::errors::{self, Result};
use types3::input::{Input, Year};
use types3::output::OError;

const DEFAULT_ITER: u64 = 1_000_000;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file (JSON)
    infile: String,
    /// Output file (JSON)
    outfile: String,
    /// Metadata category
    #[arg(long)]
    category: Option<String>,
    /// Count tokens (instead of types)
    #[arg(long, default_value_t = false)]
    count_tokens: bool,
    /// Count hapaxes (instead of types)
    #[arg(long, default_value_t = false)]
    count_hapaxes: bool,
    /// Count samples (instead of types)
    #[arg(long, default_value_t = false)]
    count_samples: bool,
    /// Compare with running words (instead of tokens)
    #[arg(long, default_value_t = false)]
    words: bool,
    /// Compare marked types vs. types
    #[arg(long, default_value_t = false)]
    type_ratio: bool,
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
    /// Sample category restriction, of the form key=value
    #[arg(long)]
    restrict_samples: Option<String>,
    /// Token category restriction, of the form key=value
    #[arg(long)]
    restrict_tokens: Option<String>,
    /// Which tokens to mark, of the form key=value
    #[arg(long)]
    mark_tokens: Option<String>,
    /// Can we split samples?
    #[arg(long)]
    split_samples: bool,
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

impl Args {
    fn to_driver_args(&self) -> Result<DriverArgs> {
        let category: Option<&str> = match &self.category {
            None => None,
            Some(key) => Some(key),
        };
        let restrict_samples = categories::parse_restriction(&self.restrict_samples)?;
        let restrict_tokens = categories::parse_restriction(&self.restrict_tokens)?;
        let mark_tokens = categories::parse_restriction(&self.mark_tokens)?;
        Ok(DriverArgs {
            category,
            count_tokens: self.count_tokens,
            count_hapaxes: self.count_hapaxes,
            count_samples: self.count_samples,
            words: self.words,
            type_ratio: self.type_ratio,
            iter: self.iter,
            offset: self.offset,
            start: self.start,
            end: self.end,
            window: self.window,
            step: self.step,
            restrict_samples,
            restrict_tokens,
            mark_tokens,
            split_samples: self.split_samples,
        })
    }
}

fn arg_sanity(args: &Args) -> Result<()> {
    if args.words && args.split_samples {
        return Err(errors::invalid_argument_ref(
            "cannot select both --words and --split-samples",
        ));
    }
    if args.words && args.type_ratio {
        return Err(errors::invalid_argument_ref(
            "cannot select both --words and --type-ratio",
        ));
    }
    let mut c = 0;
    for f in [
        args.count_tokens,
        args.count_hapaxes,
        args.count_samples,
        args.type_ratio,
    ] {
        if f {
            c += 1;
        }
    }
    if c > 1 {
        return Err(errors::invalid_argument_ref(
            "can select at most one of --count-tokens, --count-hapaxes, --count-samples, and --type-ratio",
        ));
    }
    Ok(())
}

fn process(args: &Args) -> Result<()> {
    arg_sanity(args)?;
    info!(target: "types3", "read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    let driver_args = &args.to_driver_args()?;
    let output = Calc::new(driver_args, &input)?.calc()?;
    info!(target: "types3", "write: {}", args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    if args.compact {
        serde_json::to_writer(writer, &output)?;
    } else {
        serde_json::to_writer_pretty(writer, &output)?;
    }
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

use console::style;
use std::{env, error, fmt, fs, io, process, result};
use types3::*;

type Result<T> = result::Result<T, Box<dyn error::Error>>;

struct Args {
    iter: u64,
    infile: String,
    outfile: String,
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

fn msg(prefix: &str, tail: &str) {
    eprintln!("{} {}", style(format!("{prefix:>12}")).blue().bold(), tail,);
}

fn error(prefix: &str, tail: &str) {
    eprintln!("{} {}", style(format!("{prefix:>12}")).red().bold(), tail,);
    process::exit(1);
}

fn parse_args() -> Result<Args> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(InvalidArgumentsError.into());
    }
    let iter: u64 = args[1].parse()?;
    let infile = args[2].to_owned();
    let outfile = args[3].to_owned();
    Ok(Args {
        iter,
        infile,
        outfile,
    })
}

fn process(args: &Args) -> Result<()> {
    msg("Read", &args.infile);
    let file = fs::File::open(&args.infile)?;
    let reader = io::BufReader::new(file);
    let samples: Vec<Sample> = serde_json::from_reader(reader)?;
    let driver = Driver::new_with_progress(samples);
    let result = driver.count(args.iter).to_sums();
    msg("Write", &args.outfile);
    let file = fs::File::create(&args.outfile)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer(writer, &result)?;
    msg(
        "Finished",
        &format!(
            "{} iterations, {} result points",
            result.total,
            result.total_points()
        ),
    );
    Ok(())
}

fn do_all() -> Result<()> {
    process(&parse_args()?)
}

fn main() {
    match do_all() {
        Ok(()) => (),
        Err(e) => error("Error", &format!("{e}")),
    }
}

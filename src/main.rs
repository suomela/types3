use console::style;
use std::{env, error, fmt, io, process, result};
use types3::*;

const NAME: &str = "types3";

type Result<T> = result::Result<T, Box<dyn error::Error>>;

struct Args {
    iter: u64,
}

#[derive(Debug, Clone)]
struct InvalidArgumentsError;

impl fmt::Display for InvalidArgumentsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "expected one command line argument: number of iterations"
        )
    }
}

impl error::Error for InvalidArgumentsError {}

fn parse_args() -> Result<Args> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(InvalidArgumentsError.into());
    }
    let iter: u64 = args[1].parse()?;
    Ok(Args { iter })
}

fn process(iter: u64) -> Result<()> {
    let samples: Vec<Sample> = serde_json::from_reader(io::stdin())?;
    let driver = Driver::new_with_progress(samples);
    let result = driver.count(iter).to_sums();
    serde_json::to_writer(io::stdout(), &result)?;
    Ok(())
}

fn do_all() -> Result<()> {
    let args = parse_args()?;
    process(args.iter)
}

fn main() {
    match do_all() {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "{} {} {}",
                style(format!("{NAME}:")).blue(),
                style("error:").red().bold(),
                e
            );
            process::exit(1);
        }
    }
}

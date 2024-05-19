use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use itertools::Itertools;
use log::{error, info};
use rust_xlsxwriter::{Format, Workbook};
use std::collections::{HashMap, HashSet};
use std::{error, fs, io, process};
use types3::categories;
use types3::driver;
use types3::errors::{self, Result};
use types3::input::{ISample, Input, Year};
use types3::output::{self, OError};
use types3::samples::{self, CSample};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Input file (JSON)
    infile: String,
    /// Output file (XLSX)
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
    /// Sample metadata restriction, of the form key=value
    #[arg(long)]
    restrict_samples: Option<String>,
    /// Token metadata restriction, of the form key=value
    #[arg(long)]
    restrict_tokens: Option<String>,
    /// Report errors as a JSON file
    #[arg(long)]
    error_file: Option<String>,
    /// Verbosity
    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,
}

struct RawStat<'a> {
    samples: u64,
    words: u64,
    tokens: u64,
    types: HashSet<&'a str>,
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

    fn feed_sample(&mut self, sample: &'a CSample) {
        self.samples += 1;
        self.words += sample.words;
        for token in &sample.tokens {
            self.tokens += 1;
            self.types.insert(token.token);
        }
    }

    fn get(&self, kind: &Kind) -> u64 {
        match kind {
            Kind::Samples => self.samples,
            Kind::Words => self.words,
            Kind::Tokens => self.tokens,
            Kind::Types => self.types.len() as u64,
        }
    }
}

type MdPair<'a> = (&'a String, &'a String);

enum Kind {
    Samples,
    Words,
    Tokens,
    Types,
}

impl Kind {
    fn sheetname(&self) -> &'static str {
        match self {
            Kind::Samples => "samples",
            Kind::Words => "words",
            Kind::Tokens => "tokens",
            Kind::Types => "types",
        }
    }
}

const SHEETS: &[Kind] = &[Kind::Samples, Kind::Words, Kind::Tokens, Kind::Types];

fn stat(args: &Args, samples: &[ISample]) -> Result<Workbook> {
    let restrict_samples = categories::parse_restriction(&args.restrict_samples)?;
    let restrict_tokens = categories::parse_restriction(&args.restrict_tokens)?;
    let samples = samples::get_samples(restrict_samples, restrict_tokens, None, samples);
    if samples.is_empty() {
        return Err(errors::invalid_input_ref("no samples found"));
    }
    let years = samples::get_years(&samples);
    info!(target: "types3", "years in input data: {}", output::pretty_period(&years));
    let years = (years.0.max(args.start), years.1.min(args.end + 1));

    let mut periods = driver::get_periods(args.offset, args.window, args.step, &years);
    periods.push(years);

    let skip = |md: &MdPair| -> bool {
        match restrict_samples {
            None => false,
            Some((k, v)) => md.0 == k && md.1 == v,
        }
    };

    let mut smd: HashSet<MdPair> = HashSet::new();
    for sample in &samples {
        for md in sample.metadata {
            if !skip(&md) {
                smd.insert(md);
            }
        }
    }
    let mut smd: Vec<MdPair> = smd.into_iter().collect_vec();
    smd.sort();
    let smd_map: HashMap<MdPair, usize> = smd.iter().enumerate().map(|(i, &x)| (x, i)).collect();

    let mut by_period = vec![];
    for period in &periods {
        let mut overall = RawStat::new();
        let mut by_smd = (0..smd.len()).map(|_| RawStat::new()).collect_vec();
        for sample in &samples {
            if period.0 <= sample.year && sample.year < period.1 {
                overall.feed_sample(sample);
                for md in sample.metadata {
                    if !skip(&md) {
                        by_smd[smd_map[&md]].feed_sample(sample);
                    }
                }
            }
        }
        by_period.push((period, overall, by_smd));
    }

    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    for kind in SHEETS {
        const PWIDTH: f32 = 6.0;
        const WIDTH: f32 = 12.0;
        let sheet = workbook.add_worksheet();
        sheet.set_name(kind.sheetname())?;
        let mut baserow = 0;
        if let Some(md) = restrict_samples {
            sheet.write_with_format(baserow, 0, format!("Samples: {} = {}", md.0, md.1), &bold)?;
            baserow += 1;
        }
        if let Some(md) = restrict_tokens {
            sheet.write_with_format(baserow, 0, format!("Tokens: {} = {}", md.0, md.1), &bold)?;
            baserow += 1;
        }
        if baserow > 0 {
            baserow += 1;
        }
        sheet.write_with_format(baserow, 0, "Period", &bold)?;
        sheet.write_with_format(baserow, 2, "Everything", &bold)?;
        sheet.set_column_width(0, PWIDTH)?;
        sheet.set_column_width(1, PWIDTH)?;
        sheet.set_column_width(2, WIDTH)?;
        for (j, md) in smd.iter().enumerate() {
            let col = (j + 3) as u16;
            sheet.write_with_format(baserow, col, md.0, &bold)?;
            sheet.write_with_format(baserow + 1, col, md.1, &bold)?;
            sheet.set_column_width(col, WIDTH)?;
        }
        for (i, (period, overall, by_smd)) in by_period.iter().enumerate() {
            let row = i as u32 + baserow + 2;
            sheet.write_with_format(row, 0, period.0, &bold)?;
            sheet.write_with_format(row, 1, period.1 - 1, &bold)?;
            sheet.write(row, 2, overall.get(kind))?;
            for (j, md) in by_smd.iter().enumerate() {
                let col = (j + 3) as u16;
                sheet.write(row, col, md.get(kind))?;
            }
        }
    }
    Ok(workbook)
}

fn process(args: &Args) -> Result<()> {
    info!(target: "types3", "read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    let mut workbook = stat(args, &input.samples)?;
    info!(target: "types3", "write: {}", args.outfile);
    workbook.save(&args.outfile)?;
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

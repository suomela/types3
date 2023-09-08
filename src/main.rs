use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use itertools::Itertools;
use log::{debug, error, info};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::{error, fs, io, process};
use types3::calc_avg;
use types3::calc_point::{self, Point};
use types3::categories::{self, Category};
use types3::errors::{self, Result};
use types3::information;
use types3::input::{Input, Year};
use types3::output::{
    self, MeasureX, MeasureY, OCurve, OError, OResult, Output, PointResult, Years,
};
use types3::samples;
use types3::subsets::{self, Subset, SubsetKey};

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
    /// Compare with running words (instead of tokens)
    #[arg(long, default_value_t = false)]
    words: bool,
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

fn get_periods(args: &Args, years: &Years) -> Vec<Years> {
    let mut periods = vec![];
    let mut y = args.offset;
    while y + args.step <= years.0 {
        y += args.step;
    }
    loop {
        let p = (y, y + args.window);
        periods.push(p);
        if p.1 >= years.1 {
            break;
        }
        y += args.step;
    }
    info!("periods: {}", output::pretty_periods(&periods));
    periods
}

struct Curve<'a> {
    category: Category<'a>,
    keys: Vec<SubsetKey<'a>>,
}

fn build_curve<'a>(category: Category<'a>, periods: &[Years]) -> Curve<'a> {
    let keys = periods
        .iter()
        .map(|&period| SubsetKey { category, period })
        .collect_vec();
    Curve { category, keys }
}

fn build_curves<'a>(categories: &[Category<'a>], periods: &[Years]) -> Vec<Curve<'a>> {
    categories
        .iter()
        .map(|category| build_curve(*category, periods))
        .collect_vec()
}

type TopResults<'a> = HashMap<(SubsetKey<'a>, Point), PointResult>;

struct Calc<'a> {
    years: Years,
    periods: Vec<Years>,
    curves: Vec<Curve<'a>>,
    subset_map: HashMap<SubsetKey<'a>, Subset<'a>>,
    iter: u64,
    measure_y: MeasureY,
    measure_x: MeasureX,
    restrict_samples: Category<'a>,
    restrict_tokens: Category<'a>,
    split_samples: bool,
}

impl<'a> Calc<'a> {
    fn new(args: &'a Args, input: &'a Input) -> Result<Calc<'a>> {
        let measure_x = if args.words {
            if args.split_samples {
                Err(errors::invalid_argument_ref(
                    "cannot select both --words and --split-samples",
                ))
            } else {
                Ok(MeasureX::Words)
            }
        } else {
            Ok(MeasureX::Tokens)
        }?;
        let measure_y = if args.count_tokens {
            MeasureY::Tokens
        } else {
            MeasureY::Types
        };
        information::statistics(&input.samples);
        let restrict_samples = categories::parse_restriction(&args.restrict_samples)?;
        let restrict_tokens = categories::parse_restriction(&args.restrict_tokens)?;
        let samples = samples::get_samples(restrict_samples, restrict_tokens, &input.samples);
        information::post_statistics(&samples);
        if samples.is_empty() {
            return Err(errors::invalid_input_ref("no samples found"));
        }
        let categories = match &args.category {
            None => vec![None],
            Some(key) => samples::get_categories(key, &samples)?,
        };
        let years = {
            let years = samples::get_years(&samples);
            info!("years in input data: {}", output::pretty_period(&years));
            (years.0.max(args.start), years.1.min(args.end + 1))
        };
        let periods = get_periods(args, &years);
        let curves = build_curves(&categories, &periods);
        let mut subset_map = HashMap::new();
        for curve in &curves {
            for key in &curve.keys {
                let subset = subsets::build_subset(
                    measure_x,
                    measure_y,
                    &samples,
                    *key,
                    args.split_samples,
                )?;
                let point = subset.get_point();
                let parents = subset.get_parents(years);
                subset_map.insert(*key, subset);
                for parent in &parents {
                    let x = match subset_map.entry(*parent) {
                        Occupied(e) => e.into_mut(),
                        Vacant(e) => e.insert(subsets::build_subset(
                            measure_x,
                            measure_y,
                            &samples,
                            *parent,
                            args.split_samples,
                        )?),
                    };
                    x.points.insert(point);
                }
            }
        }
        Ok(Calc {
            years,
            periods,
            curves,
            subset_map,
            iter: args.iter,
            measure_y,
            measure_x,
            restrict_samples,
            restrict_tokens,
            split_samples: args.split_samples,
        })
    }

    fn size_limit(&self) -> u64 {
        self.curves
            .iter()
            .map(|c| self.curve_size_limit(c))
            .min()
            .expect("at least one curve")
    }

    fn curve_size_limit(&self, curve: &Curve) -> u64 {
        curve
            .keys
            .iter()
            .map(|k| self.subset_map[k].total_x)
            .min()
            .expect("at least one period")
    }

    fn calc(self) -> Result<Output> {
        let mut top_results = HashMap::new();
        for subset in self.subset_map.values() {
            self.calc_top(subset, &mut top_results);
        }
        let limit = self.size_limit();
        debug!("size limit: {} {}", limit, self.measure_x);
        let curves = self
            .curves
            .iter()
            .map(|c| self.calc_curve(c, limit, &top_results))
            .collect_vec();
        Ok(Output {
            curves,
            years: self.years,
            periods: self.periods,
            measure_y: self.measure_y,
            measure_x: self.measure_x,
            iter: self.iter,
            limit,
            restrict_tokens: categories::owned_cat(self.restrict_tokens),
            restrict_samples: categories::owned_cat(self.restrict_samples),
            split_samples: self.split_samples,
        })
    }

    fn calc_top(&self, subset: &'a Subset, top_results: &mut TopResults<'a>) {
        if subset.points.is_empty() {
            return;
        }
        let mut points = subset.points.iter().copied().collect_vec();
        let key = subset.key();
        points.sort();
        let results =
            calc_point::compare_with_points(self.measure_y, &subset.samples, self.iter, &points);
        for (i, p) in points.into_iter().enumerate() {
            top_results.insert((key, p), results[i]);
        }
        debug!("{}: calculated {} points", subset.pretty(), results.len());
    }

    fn calc_curve(&self, curve: &Curve, limit: u64, top_results: &TopResults) -> OCurve {
        OCurve {
            category: categories::owned_cat(curve.category),
            results: curve
                .keys
                .iter()
                .map(|k| self.calc_relevant(&self.subset_map[k], limit, top_results))
                .collect_vec(),
        }
    }

    fn calc_relevant(&self, subset: &Subset, limit: u64, top_results: &TopResults) -> OResult {
        let mut msg = format!("{}: ", subset.pretty());
        let average_at_limit =
            calc_avg::average_at_limit(self.measure_y, &subset.samples, self.iter, limit);
        msg.push_str(&format!(
            "{} {} / {} {}",
            output::avg_string(&average_at_limit),
            self.measure_y,
            limit,
            self.measure_x
        ));
        let p = subset.get_point();
        let vs_time = {
            let k = subset.get_parent_period(self.years);
            let pr = top_results[&(k, p)];
            msg.push_str(&format!(
                ", {} vs. other time points",
                output::point_string(&pr)
            ));
            pr
        };
        let vs_categories = match subset.category {
            None => None,
            Some(_) => {
                let k = subset.get_parent_category();
                let pr = top_results[&(k, p)];
                msg.push_str(&format!(
                    ", {} vs. other categories",
                    output::point_string(&pr)
                ));
                Some(pr)
            }
        };
        debug!("{msg}");
        OResult {
            period: subset.period,
            average_at_limit,
            vs_time,
            vs_categories,
        }
    }
}

fn process(args: &Args) -> Result<()> {
    info!("read: {}", args.infile);
    let indata = fs::read_to_string(&args.infile)?;
    let input: Input = serde_json::from_str(&indata)?;
    let output = Calc::new(args, &input)?.calc()?;
    info!("write: {}", args.outfile);
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
                        info!("error reported: {e}");
                    }
                    Err(e2) => {
                        error!("{e}");
                        error!("{e2}");
                    }
                },
                None => error!("{e}"),
            }
            process::exit(1);
        }
    }
}

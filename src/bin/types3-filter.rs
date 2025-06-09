use anyhow::{Context, Result};
use clap::Parser;
use cliclack::log;
use itertools::Itertools;
use std::collections::HashMap;
use std::{fs, io};
use types3::input::{ISample, IToken, Input};

/// Convert
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Input file (JSON)
    infile: String,
    /// Output file (JSON)
    outfile: String,
}

#[derive(Clone, PartialEq, Eq)]
enum How {
    Remove,
    Keep,
}

#[derive(Clone, PartialEq, Eq)]
enum What {
    Tokens,
    Samples,
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Undo,
    Restrict(How, What),
    Save,
    Quit,
}

struct CategorySelection {
    key: String,
    values: Vec<String>,
}

fn select_samples(samples: &[ISample]) -> Result<Option<CategorySelection>> {
    let nsamples: usize = samples.len();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for sample in samples {
        for key in sample.metadata.keys() {
            *counts.entry(key).or_default() += 1;
        }
    }
    loop {
        let mut items = vec![];
        items.push((None, "Oops, go back".to_owned(), ""));
        for (&key, &count) in counts.iter().sorted() {
            items.push((Some(key), format!("{key} ({count}/{nsamples} samples)"), ""));
        }
        let choice = cliclack::select("Select from which category?")
            .items(&items)
            .interact()?;
        match choice {
            None => return Ok(None),
            Some(key) => {
                let mut counts: HashMap<&str, usize> = HashMap::new();
                for sample in samples {
                    match sample.metadata.get(key) {
                        None => (),
                        Some(val) => *counts.entry(val).or_default() += 1,
                    }
                }
                let mut items = vec![];
                for (&val, &count) in counts.iter().sorted() {
                    items.push((val, format!("{val} ({count}/{nsamples} samples)"), ""));
                }
                let choices = cliclack::multiselect("Select which values (or none to go back)?")
                    .items(&items)
                    .required(false)
                    .interact()?;
                if !choices.is_empty() {
                    return Ok(Some(CategorySelection {
                        key: key.to_owned(),
                        values: choices.iter().map(|&s| s.to_owned()).collect_vec(),
                    }));
                }
            }
        }
    }
}

fn select_tokens(samples: &[ISample]) -> Result<Option<CategorySelection>> {
    let ntokens: usize = samples.iter().map(|s| s.tokens.len()).sum();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for sample in samples {
        for token in &sample.tokens {
            for key in token.metadata.keys() {
                *counts.entry(key).or_default() += 1;
            }
        }
    }
    loop {
        let mut items = vec![];
        items.push((None, "Oops, go back".to_owned(), ""));
        for (&key, &count) in counts.iter().sorted() {
            items.push((Some(key), format!("{key} ({count}/{ntokens} tokens)"), ""));
        }
        let choice = cliclack::select("Select from which category?")
            .items(&items)
            .interact()?;
        match choice {
            None => return Ok(None),
            Some(key) => {
                let mut counts: HashMap<&str, usize> = HashMap::new();
                for sample in samples {
                    for token in &sample.tokens {
                        match token.metadata.get(key) {
                            None => (),
                            Some(val) => *counts.entry(val).or_default() += 1,
                        }
                    }
                }
                let mut items = vec![];
                for (&val, &count) in counts.iter().sorted() {
                    items.push((val, format!("{val} ({count}/{ntokens} tokens)"), ""));
                }
                let choices = cliclack::multiselect("Select which values (or none to go back)?")
                    .items(&items)
                    .required(false)
                    .interact()?;
                if !choices.is_empty() {
                    return Ok(Some(CategorySelection {
                        key: key.to_owned(),
                        values: choices.iter().map(|&s| s.to_owned()).collect_vec(),
                    }));
                }
            }
        }
    }
}

fn summarize(samples: &[ISample]) -> String {
    let nsamples = samples.len();
    let ntokens: usize = samples.iter().map(|s| s.tokens.len()).sum();
    format!("{ntokens} tokens in {nsamples} samples")
}

fn matches(cs: &CategorySelection, metadata: &HashMap<String, String>) -> bool {
    match metadata.get(&cs.key) {
        None => false,
        Some(val) => cs.values.contains(val),
    }
}

fn restrict_tokens(
    how: &How,
    what: &What,
    cs: &CategorySelection,
    tokens: Vec<IToken>,
) -> Vec<IToken> {
    tokens
        .into_iter()
        .filter(|t| match what {
            What::Tokens => {
                let m = matches(cs, &t.metadata);
                match how {
                    How::Keep => m,
                    How::Remove => !m,
                }
            }
            What::Samples => true,
        })
        .collect_vec()
}

fn restrict_samples_or_tokens(
    how: &How,
    what: &What,
    cs: &CategorySelection,
    samples: Vec<ISample>,
) -> Vec<ISample> {
    samples
        .into_iter()
        .filter(|s| match what {
            What::Samples => {
                let m = matches(cs, &s.metadata);
                match how {
                    How::Keep => m,
                    How::Remove => !m,
                }
            }
            What::Tokens => true,
        })
        .map(|s| ISample {
            id: s.id,
            year: s.year,
            descr: s.descr,
            metadata: s.metadata,
            words: s.words,
            tokens: restrict_tokens(how, what, cs, s.tokens),
        })
        .collect_vec()
}

fn main() -> Result<()> {
    let args = Args::parse();
    cliclack::intro("types3-filter")?;
    log::info(format!("Reading {}...", args.infile))?;
    let indata =
        fs::read_to_string(&args.infile).with_context(|| format!("cannot read {}", args.infile))?;
    let input: Input =
        serde_json::from_str(&indata).with_context(|| format!("cannot parse {}", args.infile))?;
    let mut restrictions: Vec<(How, What, CategorySelection)> = vec![];
    loop {
        let mut samples = input.samples.clone();
        let mut stack = vec![];
        let options = textwrap::Options::new(70).subsequent_indent(" ");
        stack.push(format!("{} ← input", summarize(&samples)));
        for (how, what, cs) in &restrictions {
            samples = restrict_samples_or_tokens(how, what, cs, samples);
            let line = format!(
                "{} ← {} {} where '{}' is {}",
                summarize(&samples),
                match how {
                    How::Keep => "keep",
                    How::Remove => "remove",
                },
                match what {
                    What::Samples => "samples",
                    What::Tokens => "tokens",
                },
                cs.key,
                cs.values.iter().map(|x| format!("'{x}'")).join(" or "),
            );
            stack.push(textwrap::fill(&line, &options));
        }

        cliclack::note("Restrictions", stack.join("\n"))?;

        let mut items = vec![];
        if !restrictions.is_empty() {
            items.push((Action::Undo, "Remove last restriction", ""));
        }
        items.push((
            Action::Restrict(How::Remove, What::Tokens),
            "Select which tokens to remove",
            "",
        ));
        items.push((
            Action::Restrict(How::Keep, What::Tokens),
            "Select which tokens to keep",
            "",
        ));
        items.push((
            Action::Restrict(How::Remove, What::Samples),
            "Select which samples to remove",
            "",
        ));
        items.push((
            Action::Restrict(How::Keep, What::Samples),
            "Select which samples to keep",
            "",
        ));
        items.push((
            Action::Save,
            "Write current restrictions to the output file",
            "",
        ));
        items.push((Action::Quit, "Quit", ""));
        let choice = cliclack::select("Action?").items(&items).interact()?;
        match choice {
            Action::Quit => break,
            Action::Undo => {
                restrictions.pop();
            }
            Action::Save => {
                let filename: String = cliclack::input("file name")
                    .default_input(&args.outfile)
                    .interact()?;
                let file = fs::File::create(&filename)?;
                let writer = io::BufWriter::new(file);
                let new_input = Input { samples };
                serde_json::to_writer_pretty(writer, &new_input)?;
                log::info(format!("Wrote to {}", filename))?;
            }
            Action::Restrict(how, What::Tokens) => match select_tokens(&samples)? {
                None => (),
                Some(cs) => restrictions.push((how, What::Tokens, cs)),
            },
            Action::Restrict(how, What::Samples) => match select_samples(&samples)? {
                None => (),
                Some(cs) => restrictions.push((how, What::Samples, cs)),
            },
        }
    }
    cliclack::outro("Bye!")?;
    Ok(())
}

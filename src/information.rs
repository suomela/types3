use crate::input::ISample;
use crate::samples::CSample;
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet};

fn explain_metadata_one(k: &str, vv: &HashSet<&str>) -> String {
    let vals = vv.iter().copied().sorted().collect_vec();
    format!("{} = {}", k, vals.join(", "))
}

fn explain_metadata(metadata: &HashMap<&str, HashSet<&str>>) -> String {
    let keys = metadata.keys().copied().sorted().collect_vec();
    keys.iter()
        .map(|k| explain_metadata_one(k, &metadata[k]))
        .join("; ")
}

pub fn statistics(samples: &[ISample]) {
    let mut lemmas = HashSet::new();
    let mut token_metadata: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut sample_metadata: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut tokencount = 0;
    for s in samples {
        for (k, v) in s.metadata.iter() {
            sample_metadata.entry(k).or_default().insert(v);
        }
        for t in &s.tokens {
            tokencount += 1;
            for (k, v) in t.metadata.iter() {
                token_metadata.entry(k).or_default().insert(v);
            }
            lemmas.insert(&t.lemma);
        }
    }
    info!(target: "types3", "before filtering: samples: {}", samples.len());
    info!(target: "types3", "before filtering: tokens: {}", tokencount);
    info!(target: "types3", "before filtering: distinct lemmas: {}", lemmas.len());
    info!(target: "types3",
        "token metadata categories: {}",
        explain_metadata(&token_metadata)
    );
    info!(target: "types3",
        "sample metadata categories: {}",
        explain_metadata(&sample_metadata)
    );
}

pub fn post_statistics(samples: &[CSample]) {
    let mut lemmas = HashSet::new();
    let mut marked_lemmas = HashSet::new();
    let mut tokencount = 0;
    let mut marked_tokencount = 0;
    for s in samples {
        for t in &s.tokens {
            tokencount += 1;
            lemmas.insert(t.token);
            if t.marked {
                marked_tokencount += 1;
                marked_lemmas.insert(t.token);
            }
        }
    }
    info!(target: "types3", "after filtering: samples: {}", samples.len());
    info!(target: "types3",
        "after filtering: tokens: {}, marked: {}",
        tokencount, marked_tokencount
    );
    info!(target: "types3",
        "after filtering: distinct lemmas: {}, marked: {}",
        lemmas.len(),
        marked_lemmas.len()
    );
}

use std::{path::PathBuf, fs};
use types3::{Driver, Sample};

fn slurp(filename: &str) -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push(filename);
    fs::read_to_string(d).expect("reading succeeds")
}

#[test]
fn example_1() {
    let iter = 10000;
    let input = slurp("examples/in1.json");
    let expected = slurp("examples/out1.json");
    let samples: Vec<Sample> = serde_json::from_str(&input).unwrap();
    let driver = Driver::new(samples);
    let result = driver.count(iter).to_sums();
    let output = serde_json::to_string(&result).unwrap();
    assert_eq!(output, expected);
}

use std::fs;
use std::path::PathBuf;
use types3::driver::{self, DriverArgs};
use types3::input::Input;
use types3::output::{MeasureX, MeasureY, Output};

fn init() {
    let _ = pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .try_init();
}

fn slurp(filename: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    let mut path = PathBuf::from(dir);
    path.push(filename);
    fs::read_to_string(path).unwrap()
}

#[test]
fn test_basic() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-types-vs-tokens.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_category() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-types-vs-tokens-gender.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: Some("gender"),
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_bad_category() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: Some("nonexisting"),
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    assert!(driver::calc(&driver_args, &input).is_err());
}

#[test]
fn test_tokens_words() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-tokens-vs-words.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Tokens,
        measure_x: MeasureX::Words,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_hapaxes_words() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-hapaxes-vs-words.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Hapaxes,
        measure_x: MeasureX::Words,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_samples_words() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-samples-vs-words.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Samples,
        measure_x: MeasureX::Words,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_type_ratio() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-type-ratio-split-ity-female.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::MarkedTypes,
        measure_x: MeasureX::Types,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: Some(("gender", "female")),
        restrict_tokens: None,
        mark_tokens: Some(("variant", "ity")),
        split_samples: true,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_minimum() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-types-vs-tokens-1000.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1000,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_category_minimum() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let data = slurp("integration-test/calc-expected/ceec-types-vs-tokens-1000-gender.json");
    let expected: Output = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: Some("gender"),
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1000,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    let output = driver::calc(&driver_args, &input).unwrap();
    assert_eq!(output, expected);
}

#[test]
fn test_future_start() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 3000,
        end: 9999,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    assert!(driver::calc(&driver_args, &input).is_err());
}

#[test]
fn test_past_end() {
    init();
    let data = slurp("sample-data/ceec.json");
    let input: Input = serde_json::from_str(&data).unwrap();
    let driver_args = DriverArgs {
        category: None,
        measure_y: MeasureY::Types,
        measure_x: MeasureX::Tokens,
        iter: 10000,
        offset: 0,
        start: 0,
        end: 1000,
        window: 20,
        step: 20,
        minimum_size: 1,
        restrict_samples: None,
        restrict_tokens: None,
        mark_tokens: None,
        split_samples: false,
    };
    assert!(driver::calc(&driver_args, &input).is_err());
}

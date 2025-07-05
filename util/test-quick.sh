#!/bin/bash

set -e

cargo test
cargo clippy --all-targets
cargo build --release
integration-test/test-calc.sh

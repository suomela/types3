#!/bin/bash

set -e

cargo test
cargo clippy
cargo build --release
integration-test/test-calc.sh

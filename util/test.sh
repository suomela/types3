#!/bin/bash

set -e

util/setup.sh
cargo test
cargo clippy
integration-test/test-calc.sh
integration-test/test-calc2.sh
integration-test/test-plot.sh
integration-test/test-version.sh

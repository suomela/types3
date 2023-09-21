#!/bin/bash

set -e

cargo build --release
integration-test/benchmark-calc.sh

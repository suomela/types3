#!/bin/bash

set -e

cd integration-test
rm -rf calc2
mkdir -p calc2

base="../types3-calc --window 20 --step 20 --iter 100000 ../sample-data/ceec.json -v -v"
what="$base"
$what calc2/ceec-types-vs-tokens-ity.json --restrict-tokens variant=ity
$what calc2/ceec-types-vs-tokens-ness.json --restrict-tokens variant=ness
what="$base --words"
$what calc2/ceec-types-vs-words-ity.json --restrict-tokens variant=ity
$what calc2/ceec-types-vs-words-ness.json --restrict-tokens variant=ness

python3 verify-calc.py
echo "SUCCESS: all results look good."

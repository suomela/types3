#!/bin/bash

set -e

cd integration-test
rm -rf calc
mkdir -p calc

base="../types3-calc --window 20 --step 20 --iter 10000 ../sample-data/ceec.json"
what="$base"
$what calc/ceec-tokens.json
$what calc/ceec-tokens-ity.json --restrict-tokens variant=ity
$what calc/ceec-tokens-female.json --restrict-samples gender=female
$what calc/ceec-tokens-gender.json --category gender
$what calc/ceec-tokens-socmob.json --category socmob
$what calc/ceec-tokens-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --split-samples"
$what calc/ceec-split.json
$what calc/ceec-split-ity.json --restrict-tokens variant=ity
$what calc/ceec-split-female.json --restrict-samples gender=female
$what calc/ceec-split-gender.json --category gender
$what calc/ceec-split-socmob.json --category socmob
$what calc/ceec-split-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --words"
$what calc/ceec-words.json
$what calc/ceec-words-ity.json --restrict-tokens variant=ity
$what calc/ceec-words-female.json --restrict-samples gender=female
$what calc/ceec-words-gender.json --category gender
$what calc/ceec-words-socmob.json --category socmob
$what calc/ceec-words-gender-ity.json --category gender --restrict-tokens variant=ity

cd calc
for a in *.json; do
    diff ../calc-expected/$a $a
done
echo "SUCCESS: all results agree."

#!/bin/bash

set -e

cd integration-test
rm -rf calc
mkdir -p calc

base="../types3-calc --window 20 --step 20 --iter 10000 ../sample-data/ceec.json"
what="$base"
$what calc/ceec-types-vs-tokens.json
$what calc/ceec-types-vs-tokens-ity.json --restrict-tokens variant=ity
$what calc/ceec-types-vs-tokens-female.json --restrict-samples gender=female
$what calc/ceec-types-vs-tokens-gender.json --category gender
$what calc/ceec-types-vs-tokens-socmob.json --category socmob
$what calc/ceec-types-vs-tokens-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --split-samples"
$what calc/ceec-types-vs-tokens-split.json
$what calc/ceec-types-vs-tokens-split-ity.json --restrict-tokens variant=ity
$what calc/ceec-types-vs-tokens-split-female.json --restrict-samples gender=female
$what calc/ceec-types-vs-tokens-split-gender.json --category gender
$what calc/ceec-types-vs-tokens-split-socmob.json --category socmob
$what calc/ceec-types-vs-tokens-split-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --words"
$what calc/ceec-types-vs-words.json
$what calc/ceec-types-vs-words-ity.json --restrict-tokens variant=ity
$what calc/ceec-types-vs-words-female.json --restrict-samples gender=female
$what calc/ceec-types-vs-words-gender.json --category gender
$what calc/ceec-types-vs-words-socmob.json --category socmob
$what calc/ceec-types-vs-words-gender-ity.json --category gender --restrict-tokens variant=ity

cd calc
for a in *.json; do
    diff ../calc-expected/$a $a
done
echo "SUCCESS: all results agree."

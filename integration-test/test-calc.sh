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
what="$base --minimum-size 100"
$what calc/ceec-types-vs-tokens-100.json
$what calc/ceec-types-vs-tokens-100-ity.json --restrict-tokens variant=ity
$what calc/ceec-types-vs-tokens-100-female.json --restrict-samples gender=female
$what calc/ceec-types-vs-tokens-100-gender.json --category gender
$what calc/ceec-types-vs-tokens-100-socmob.json --category socmob
$what calc/ceec-types-vs-tokens-100-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --minimum-size 1000"
$what calc/ceec-types-vs-tokens-1000.json
$what calc/ceec-types-vs-tokens-1000-ity.json --restrict-tokens variant=ity
$what calc/ceec-types-vs-tokens-1000-female.json --restrict-samples gender=female
$what calc/ceec-types-vs-tokens-1000-gender.json --category gender
$what calc/ceec-types-vs-tokens-1000-socmob.json --category socmob
$what calc/ceec-types-vs-tokens-1000-gender-ity.json --category gender --restrict-tokens variant=ity
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
what="$base --count-hapaxes"
$what calc/ceec-hapaxes-vs-tokens.json
$what calc/ceec-hapaxes-vs-tokens-ity.json --restrict-tokens variant=ity
$what calc/ceec-hapaxes-vs-tokens-female.json --restrict-samples gender=female
$what calc/ceec-hapaxes-vs-tokens-gender.json --category gender
$what calc/ceec-hapaxes-vs-tokens-socmob.json --category socmob
$what calc/ceec-hapaxes-vs-tokens-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --count-hapaxes --split-samples"
$what calc/ceec-hapaxes-vs-tokens-split.json
what="$base --count-hapaxes --words"
$what calc/ceec-hapaxes-vs-words.json
what="$base --count-tokens --words"
$what calc/ceec-tokens-vs-words.json
$what calc/ceec-tokens-vs-words-ity.json --restrict-tokens variant=ity
$what calc/ceec-tokens-vs-words-female.json --restrict-samples gender=female
$what calc/ceec-tokens-vs-words-gender.json --category gender
$what calc/ceec-tokens-vs-words-socmob.json --category socmob
$what calc/ceec-tokens-vs-words-gender-ity.json --category gender --restrict-tokens variant=ity
what="$base --count-tokens"
$what calc/ceec-tokens-vs-tokens.json
what="$base --count-tokens --split-samples"
$what calc/ceec-tokens-vs-tokens-split.json
what="$base --count-samples --words"
$what calc/ceec-samples-vs-words.json
what="$base --count-samples"
$what calc/ceec-samples-vs-tokens.json
what="$base --type-ratio"
$what calc/ceec-type-ratio-none.json
what="$base --type-ratio --split-samples"
$what calc/ceec-type-ratio-split-none.json
what="$base --type-ratio --mark-tokens variant=ity"
$what calc/ceec-type-ratio-ity.json
$what calc/ceec-type-ratio-ity-female.json --restrict-samples gender=female
$what calc/ceec-type-ratio-ity-gender.json --category gender
what="$base --type-ratio --split-samples --mark-tokens variant=ity"
$what calc/ceec-type-ratio-split-ity.json
$what calc/ceec-type-ratio-split-ity-female.json --restrict-samples gender=female
$what calc/ceec-type-ratio-split-ity-gender.json --category gender

base="../types3-calc --window 50 --step 10 --iter 10000 ../sample-data/ced-ppceme-chelar.json"
what="$base --minimum-size 100"
$what calc/ced-ppceme-chelar-types-vs-tokens-100-corpus.json --category corpus

cd calc
for a in *.json; do
    diff ../calc-expected/$a $a
done
echo "SUCCESS: all results agree."

#!/bin/bash

set -e

cd integration-test
rm -rf calc3
mkdir -p calc3

base="time ../types3-calc --window 20 --step 20 --iter 100000 ../sample-data/ceec.json"
what="$base"
$what calc3/ceec-types-vs-tokens.json
what="$base --split-samples"
$what calc3/ceec-types-vs-tokens-split.json
# what="$base --words"
# $what calc3/ceec-types-vs-words.json
# what="$base --count-hapaxes"
# $what calc3/ceec-hapaxes-vs-tokens.json
# what="$base --count-hapaxes --split-samples"
# $what calc3/ceec-hapaxes-vs-tokens-split.json
# what="$base --count-hapaxes --words"
# $what calc3/ceec-hapaxes-vs-words.json
# what="$base --count-tokens --words"
# $what calc3/ceec-tokens-vs-words.json
# what="$base --count-tokens"
# $what calc3/ceec-tokens-vs-tokens.json
# what="$base --count-tokens --split-samples"
# $what calc3/ceec-tokens-vs-tokens-split.json
# what="$base --count-samples --words"
# $what calc3/ceec-samples-vs-words.json
# what="$base --count-samples"
# $what calc3/ceec-samples-vs-tokens.json
# what="$base --type-ratio --mark-tokens variant=ity"
# $what calc3/ceec-type-ratio-ity.json
# what="$base --type-ratio --split-samples --mark-tokens variant=ity"
# $what calc3/ceec-type-ratio-split-ity.json

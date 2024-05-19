#!/bin/bash

set -e

cd integration-test
rm -rf stat
mkdir -p stat

base="../types3-stat --window 20 --step 20 ../sample-data/ceec.json"
what="$base"
$what stat/ceec.xlsx
$what stat/ceec-ity.xlsx --restrict-tokens variant=ity
$what stat/ceec-female.xlsx --restrict-samples gender=female
$what stat/ceec-ity-female.xlsx --restrict-tokens variant=ity --restrict-samples gender=female

base="../types3-stat --window 50 --step 10 ../sample-data/ced-ppceme-chelar.json"
$what stat/ced-ppceme-chelar.xlsx

echo "SUCCESS: we were able to run types3-stat."

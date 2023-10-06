#!/bin/bash

set -e

git clone https://github.com/suomela/types-examples
python3 util/ceec-convert.py types-examples/ceec-input/db/types.sqlite ceec.json
rm -rf types-examples

git clone https://github.com/suomela/suffix-competition
python3 util/ced-ppceme-chelar-convert.py suffix-competition/data.json ced-ppceme-chelar.json
rm -rf suffix-competition

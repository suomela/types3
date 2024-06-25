#!/bin/bash

set -e

cd integration-test
rm -rf convert
mkdir -p convert

../types3-convert ../data-format/example-samples.csv ../data-format/example-tokens.csv convert/example.json
diff ../data-format/example.json convert/example.json

echo "SUCCESS: all results agree."

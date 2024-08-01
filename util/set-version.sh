#!/bin/bash

set -e

version="$1"
perl -pi -e "s/^__version__ = '.*'/__version__ = '$version'/" python/types3/version.py
perl -pi -e 's/^version = ".*"/version = "'$version'"/' Cargo.toml

util/setup.sh
integration-test/test-version.sh

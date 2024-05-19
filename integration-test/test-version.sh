#!/bin/bash

set -e

WHERE=$(readlink -f "$0")
WHERE=$(dirname "$WHERE")
export TYPES3_BASEDIR=$(dirname "$WHERE")

if [ ! -e "$TYPES3_BASEDIR/venv/bin/activate" ]; then
    echo "Cannot find Python virtual environment"
    exit 1
fi

if [ ! -e "$TYPES3_BASEDIR/target/release/types3-calc" ]; then
    echo "Cannot find types3-calc binary"
    exit 1
fi

source "$TYPES3_BASEDIR/venv/bin/activate"

v1=$($TYPES3_BASEDIR/target/release/types3-calc --version)
v2=$(python3 $TYPES3_BASEDIR/python/types3-version.py)

if [ "$v1" != "$v2" ]; then
    echo "Software version mismatch:"
    echo "- Rust code: $v1"
    echo "- Python code: $v2"
    exit 1
fi

echo "SUCCESS: version numbers agree."

#!/bin/bash

set -e

cargo build --release
python3 -m venv venv
source venv/bin/activate
python3 -m pip install --upgrade pip
python3 -m pip install appdirs matplotlib

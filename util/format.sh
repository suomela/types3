#!/bin/bash

set -e

source "venv/bin/activate"
yapf -ip python/*.py python/types3/*.py
cargo fmt

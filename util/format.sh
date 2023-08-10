#!/bin/bash

set -e

source "venv/bin/activate"
yapf -ip python/*.py
cargo fmt

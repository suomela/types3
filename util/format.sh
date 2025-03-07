#!/bin/bash

set -e

uvx ruff format
cargo fmt

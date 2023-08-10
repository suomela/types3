#!/bin/bash

set -e

pipenv run yapf -ip types3/*.py
cargo fmt

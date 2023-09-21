# types3: Type accumulation curves

This is a tool for analyzing textual diversity, richness, and productivity in text corpora and other data sets.

![Screenshot: user interface](doc/types3.png)

## Setup

You will need to have Rust. Just use [rustup](https://www.rust-lang.org/tools/install) as usual.

You will need to have Python, with Tkinter support. On macOS, use [Homebrew](https://brew.sh) for that:

    brew install python python-tk

Once everything is set up, you can compile the code and install the relevant Python modules with:

    util/setup.sh

## Usage

Try these:

    ./types3-ui --help
    ./types3-calc --help
    ./types3-plot --help

## Sample data

To explore the sample data set that we have in `sample-data`, try:

    ./types3-ui sample-data/ceec.json


## Prior versions

- [types2](https://github.com/suomela/types): Type and hapax accumulation curves
- [TypeRatio](https://github.com/suomela/type-ratio): comparing competing suffixes

## Author

- [Jukka Suomela](https://jukkasuomela.fi/), Aalto University

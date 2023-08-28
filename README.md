# types3: Type accumulation curves

This is a tool for analyzing textual diversity, richness, and productivity in text corpora and other data sets.

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

See [types3-examples](https://github.com/suomela/types3-examples) for some sample data. Download the data set and explore it with `types3-ui`, e.g. as follows:

    ./types3-ui ../types3-examples/ceec.json

## Author

- [Jukka Suomela](https://jukkasuomela.fi/), Aalto University

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

To explore the [sample data sets](sample-data), try:

    ./types3-ui sample-data/ceec.json
    ./types3-ui sample-data/ced-ppceme-chelar.json

## Prior versions

- [types2](https://github.com/suomela/types): type and hapax accumulation curves
- [TypeRatio](https://github.com/suomela/type-ratio): comparing competing suffixes

## Author

- [Jukka Suomela](https://jukkasuomela.fi/), Aalto University

## Acknowledgements

Thanks to [Paula Rodríguez-Puente](https://www.usc-vlcg.es/PRP.htm) and [Tanja Säily](https://tanjasaily.fi/) for help with developing these tools.

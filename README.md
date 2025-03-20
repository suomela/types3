# types3: Type accumulation curves

This is a tool for analyzing textual diversity, richness, and productivity in text corpora and other data sets.

![Screenshot: user interface](doc/types3.png)

## References

This tool has been used in:

- Margherita Fantoli, Jukka Suomela, Toon Van Hal, Mark Depauw, Lari Virkki, and Mikko Tolonen (2025): "Quantifying the Presence of Ancient Greek and Latin Classics in Early Modern Britain." *Journal of Cultural Analytics*, volume 10, issue 1. [doi:10.22148/001c.128008](https://doi.org/10.22148/001c.128008)

## Setup

You will need some Unix-like operating system, Python with Tkinter support, and Rust. See below for detailed instructions for your operating system.

### macOS

Open a terminal.

Make sure you have Homebrew installed and up-to-date:

- Run `brew update` and `brew upgrade` and see if it works.
- If not, follow the usual [Homebrew installation instructions](https://brew.sh).
- After installation, you can close and re-open the terminal.

Make sure you have got the relevant packages installed, by running this command:

    brew install python python-tk

Make sure you have got Rust installed and up-to-date:

- Run `rustup update` and see if it works.
- If not, follow the usual [Rust installation instructions](https://www.rust-lang.org/tools/install). You need to copy-paste one command to the terminal and follow instructions (all default settings are fine).
- After installation, you can close and re-open the terminal.

Download and set up types3:

    git clone https://github.com/suomela/types3.git
    cd types3
    util/setup.sh

Try it out with our sample data set:

    ./types3-ui sample-data/ceec.json

### Ubuntu Linux

Open a terminal.

Make sure you have got the relevant packages installed, by running these commands:

    sudo apt-get update
    sudo apt-get install git curl build-essential python3 python3-venv python3-tk

Make sure you have got Rust installed and up-to-date:

- Run `rustup update` and see if it works.
- If not, follow the usual [Rust installation instructions](https://www.rust-lang.org/tools/install). You need to copy-paste one command to the terminal and follow instructions (all default settings are fine).
- After installation, you can close and re-open the terminal.

Download and set up types3:

    git clone https://github.com/suomela/types3.git
    cd types3
    util/setup.sh

Try it out with our sample data set:

    ./types3-ui sample-data/ceec.json

### Windows

Install [WSL](https://learn.microsoft.com/en-us/windows/wsl/install) (Windows Subsystem for Linux) if you do not have it yet:

- Open a terminal.
- Run `wsl --install` to install WSL.
- Restart the computer when instructed to do so.
- Open "Ubuntu" (from the Start menu).
- Follow the instructions to set your Linux username and password.

Now you have got Ubuntu Linux running inside your Windows computer, and you can follow the instructions for Ubuntu Linux above. Just make sure that you run all commands inside "Ubuntu" (and not e.g. in the regular Windows terminal).

## Usage

If you have input data in `data.json`, you can start to explore it with:

    ./types3-ui data.json

For more information, see:

    ./types3-ui --help
    ./types3-convert --help
    ./types3-calc --help
    ./types3-plot --help
    ./types3-stat --help

## Data format

See [data-format](data-format) for more information on the data format and how to convert your own data into the right format.

## Sample data

See [sample-data](sample-data) for our sample data sets. To explore our sample data sets, try:

    ./types3-ui sample-data/ceec.json

and:

    ./types3-ui sample-data/ced-ppceme-chelar.json

## Tests

To run all automatic tests, you will also need to have ImageMagick installed:

- On Ubuntu Linux run `apt-get install imagemagick`
- On macOS run `brew install imagemagick`

Depending on your operating system, you may also need to [adjust ImageMagick security policy](https://stackoverflow.com/questions/52998331/imagemagick-security-policy-pdf-blocking-conversion) to enable PDF-to-PNG conversion.

Then run:

    util/test.sh

## Prior versions

- [types2](https://github.com/suomela/types): type and hapax accumulation curves
- [TypeRatio](https://github.com/suomela/type-ratio): comparing competing suffixes

## Author

- [Jukka Suomela](https://jukkasuomela.fi/), Aalto University

## Acknowledgements

Thanks to [Paula Rodríguez-Puente](https://www.usc-vlcg.es/PRP.htm) and [Tanja Säily](https://tanjasaily.fi/) for help with developing these tools.

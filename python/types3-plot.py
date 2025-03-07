import argparse
import json
import logging
import matplotlib
import matplotlib.pyplot as plt
import types3.plot

cli = argparse.ArgumentParser(
    description="Plot type accumulation curves (used by types3-ui)."
)
cli.add_argument(
    "--verbose", "-v", action="count", default=0, help="Increase verbosity"
)
cli.add_argument(
    "--legend", default="best", help='Legend placement (e.g. "upper right")'
)
cli.add_argument("--wide", action="store_true", help="Wide layout")
cli.add_argument("--large", action="store_true", help="Large fonts")
cli.add_argument("--dpi", default=300, type=int, help="PNG resolution")
cli.add_argument("infile", help="Input file (JSON)")
cli.add_argument("outfile", help="Output file (with extension .pdf, .png, or .txt)")
cli.add_argument(
    "--version", action="version", version="%(prog)s " + types3.__version__
)


def plot(args):
    matplotlib.use("Agg")
    matplotlib.rcParams["axes.titlesize"] = "medium"
    matplotlib.rcParams["savefig.dpi"] = args.dpi
    if args.large:
        matplotlib.rcParams["font.size"] = 14
        matplotlib.rcParams["axes.titlepad"] = 10
    logging.info(f"read: {args.infile}")
    with open(args.infile) as f:
        data = json.load(f)
    if args.outfile.endswith(".txt"):
        logging.info("export...")
        text = types3.plot.text(data)
        logging.info(f"write: {args.outfile}")
        with open(args.outfile, "w") as f:
            f.write(text)
    else:
        logging.info("plot...")
        dims = types3.plot.DIMS_PLOT_WIDE if args.wide else types3.plot.DIMS_PLOT
        dims = types3.plot.set_height(data, dims)
        fig = plt.figure(figsize=(dims.width, dims.height))
        types3.plot.plot(fig, data, dims, legend=args.legend)
        logging.info(f"write: {args.outfile}")
        fig.savefig(args.outfile)


def main():
    args = cli.parse_args()
    if args.verbose >= 2:
        loglevel = logging.DEBUG
    elif args.verbose >= 1:
        loglevel = logging.INFO
    else:
        loglevel = logging.WARN
    logging.basicConfig(format="%(levelname)s %(message)s", level=loglevel)
    plot(args)


if __name__ == "__main__":
    main()

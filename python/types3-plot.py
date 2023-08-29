import argparse
import json
import logging
import matplotlib
import types3.plot

matplotlib.use('Agg')
matplotlib.rcParams['axes.titlesize'] = 'medium'
import matplotlib.pyplot as plt

cli = argparse.ArgumentParser()
cli.add_argument('--verbose',
                 '-v',
                 action='count',
                 default=0,
                 help='Increase verbosity')
cli.add_argument('--legend',
                 default='best',
                 help='Legend placement (e.g. "upper right")')
cli.add_argument('infile', help='Input file (JSON)')
cli.add_argument('outfile', help='Output file (PDF)')


def plot(args):
    logging.info(f'read: {args.infile}')
    with open(args.infile) as f:
        data = json.load(f)
    logging.info(f'plot...')
    dims = types3.plot.DIMS_PLOT
    dims = types3.plot.set_height(data, dims)
    fig = plt.figure(figsize=(dims.width, dims.height))
    types3.plot.plot(fig, data, dims, legend=args.legend)
    logging.info(f'write: {args.outfile}')
    fig.savefig(args.outfile)


if __name__ == '__main__':
    args = cli.parse_args()
    if args.verbose >= 1:
        loglevel = logging.DEBUG
    else:
        loglevel = logging.INFO
    logging.basicConfig(format='%(levelname)s %(message)s', level=loglevel)
    plot(args)

import argparse
import json
import logging
import math
import matplotlib

matplotlib.use('Agg')
matplotlib.rcParams['axes.titlesize'] = 'medium'
import matplotlib.pyplot as plt

COLORS = ['#f26924', '#0088cc', '#3ec636']

cli = argparse.ArgumentParser()
cli.add_argument('--verbose',
                 '-v',
                 action='count',
                 default=0,
                 help='Increase verbosity')
cli.add_argument('infile', help='Input file (JSON)')
cli.add_argument('outfile', help='Output file (PDF)')

MAX_SIGNIFICANCE = 4


def catname(cats):
    s = []
    for cat in cats:
        if cat is not None:
            k, v = cat
            s.append(f'{k} = {v}')
    if len(s) == 0:
        s.append('everything')
    return ', '.join(s)


def significance(x, n):
    assert 0 <= x <= n
    p = (n - x) / n
    try:
        return min(MAX_SIGNIFICANCE, -math.log10(p))
    except ValueError:
        return MAX_SIGNIFICANCE


def get_avg(r):
    period = r['period']
    x = period[0]
    ar = r['average_at_limit']
    y = (ar['types_low'] + ar['types_high']) / (2 * ar['iter'])
    return (x, y)


def get_vs(r, what):
    period = r['period']
    x = period[0]
    pr = r[what]
    above = significance(pr['above'], pr['iter'])
    below = significance(pr['below'], pr['iter'])
    return (x, above, -below)


def plot(args):
    logging.info(f'read: {args.infile}')
    with open(args.infile) as f:
        data = json.load(f)
    logging.info(f'plot...')
    measure = data['measure']
    limit = data['limit']
    periods = data['periods']
    curves = data['curves']
    restriction = data['restrict_samples']
    has_cats = curves[0]['category'] is not None
    nn = len(curves)

    sigmarg = 0
    h1 = 4
    h2 = 1
    m1 = 0.5
    m2 = 1.5
    m3 = 0.1

    h = m1 + h1
    h += m1 + h2 * nn + m3 * (nn - 1)
    if has_cats:
        h += m1 + h2 * nn + m3 * (nn - 1)
    h += m2
    x = 0.1
    w = 0.8

    xx = [a for (a, b) in periods]
    periodlabels = [f'{a}–{b-1}' for (a, b) in periods]

    fig = plt.figure(figsize=(7, h))
    axs2 = []
    axs3 = []
    y = m1
    y += h1
    ax = fig.add_axes([x, 1 - y / h, w, h1 / h])
    ax.set_title(f'Types in subcorpora with {limit} {measure}')
    ax.set_xticks(xx, [])
    ax1 = ax
    last = ax
    y += m1
    for i, curve in enumerate(curves):
        if i != 0:
            y += m3
        y += h2
        ax = fig.add_axes([x, 1 - y / h, w, h2 / h])
        if i == 0:
            ax.set_title(f'Significance of differences in time')
        ax.set_ylim((-MAX_SIGNIFICANCE - sigmarg, MAX_SIGNIFICANCE + sigmarg))
        ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1), [])
        ax.set_xticks([], [])
        axs2.append(ax)
        last = ax
    last.set_xticks(xx, [])
    if has_cats:
        y += m1
        for i, curve in enumerate(curves):
            if i != 0:
                y += m3
            y += h2
            ax = fig.add_axes([x, 1 - y / h, w, h2 / h])
            if i == 0:
                ax.set_title(
                    f'Significance in comparison with other categories')
            ax.set_ylim(
                (-MAX_SIGNIFICANCE - sigmarg, MAX_SIGNIFICANCE + sigmarg))
            ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1), [])
            axs3.append(ax)
            last = ax
    y += m2
    assert y == h, [y, h]
    last.set_xticks(xx, periodlabels, rotation='vertical')

    ymax = 0
    for i, curve in enumerate(curves):
        if curve['category']:
            color = COLORS[i]
        else:
            color = '#000000'
        label = catname([restriction, curve['category']])
        points = [get_avg(r) for r in curve['results']]
        xx, yy = zip(*points)
        ax1.plot(xx,
                 yy,
                 label=label,
                 color=color,
                 markeredgecolor=color,
                 markerfacecolor=color,
                 marker='o')
        ymax = max(ymax, max(yy))

        def plotter(ax, points):
            xx, yy1, yy2 = zip(*points)
            ax.fill_between(xx, yy1, yy2, color=color, alpha=0.7, linewidth=0)
            for i in range(1, MAX_SIGNIFICANCE):
                ax.fill_between(xx,
                                -i,
                                +i,
                                color='#ffffff',
                                alpha=0.4,
                                linewidth=0)
            ax.axhline(0, color='#000000', linewidth=0.8)

        points = [get_vs(r, 'vs_time') for r in curve['results']]
        plotter(axs2[i], points)

        if has_cats:
            points = [get_vs(r, 'vs_categories') for r in curve['results']]
            plotter(axs3[i], points)

    ax1.set_ylim((0, ymax * 1.05))
    ax1.legend(loc='lower left')
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

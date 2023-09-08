import math
from collections import namedtuple

COLORS = ['#f26924', '#0088cc', '#3ec636']
MAX_SIGNIFICANCE = 4
SIG_MARG = 0

Dims = namedtuple('Dims', [
    'h1', 'h2', 'm1', 'm2', 'm3', 'm4', 'x0', 'x1', 'w', 'width', 'height',
    'columns'
])

DIMS_UI = Dims(
    h1=4,
    h2=0.8,
    m1=0.3,
    m2=0.6,
    m3=0.1,
    m4=1.5,
    x0=0.1 * 7,
    x1=None,
    w=0.8 * 7,
    width=7,
    height=15,
    columns=1,
)

DIMS_PLOT = Dims(
    h1=4,
    h2=1,
    m1=0.5,
    m2=0.5,
    m3=0.1,
    m4=1.5,
    x0=0.1 * 7,
    x1=None,
    w=0.8 * 7,
    width=7,
    height=None,
    columns=1,
)

DIMS_PLOT_WIDE = Dims(
    h1=5,
    h2=1,
    m1=0.5,
    m2=0.8,
    m3=0.1,
    m4=1.5,
    x0=0.1 * 7,
    x1=0.9 * 7,
    w=0.8 * 7,
    width=1.9 * 7,
    height=None,
    columns=2,
)


def _catname(cats):
    s = []
    for cat in cats:
        if cat is not None:
            k, v = cat
            s.append(v)
    if len(s) == 0:
        s.append('everything')
    return ', '.join(s)


def _significance(x, n):
    assert 0 <= x <= n
    p = (n - x) / n
    try:
        return min(MAX_SIGNIFICANCE, -math.log10(p))
    except ValueError:
        return MAX_SIGNIFICANCE


def _get_avg(r):
    period = r['period']
    x = period[0]
    ar = r['average_at_limit']
    y = (ar['low'] + ar['high']) / (2 * ar['iter'])
    return (x, y)


def _get_vs(r, what):
    period = r['period']
    x = period[0]
    pr = r[what]
    above = _significance(pr['above'], pr['iter'])
    below = _significance(pr['below'], pr['iter'])
    return (x, above, -below)


def set_height(data, dims):
    curves = data['curves']
    nn = len(curves)
    has_cats = curves[0]['category'] is not None
    if dims.columns == 1:
        y = dims.m1
        y += dims.h1
        y += dims.m2
        y += nn * dims.h2
        y += (nn - 1) * dims.m3
        if has_cats:
            y += dims.m2
            y += nn * dims.h2
            y += (nn - 1) * dims.m3
        y += dims.m4
    else:
        y1 = dims.m1
        y1 += dims.h1
        y1 += dims.m4
        y2 = dims.m1
        y2 += nn * dims.h2
        y2 += (nn - 1) * dims.m3
        if has_cats:
            y2 += dims.m2
            y2 += nn * dims.h2
            y2 += (nn - 1) * dims.m3
        y2 += dims.m4
        y = max(y1, y2)
    return dims._replace(height=y)


def plot(fig, data, dims, legend):
    measure = data['measure']
    limit = data['limit']
    periods = data['periods']
    curves = data['curves']
    restrictions = [data['restrict_samples'], data['restrict_tokens']]
    has_cats = curves[0]['category'] is not None

    xx = [a for (a, b) in periods]
    periodlabels = [f'{a}–{b-1}' for (a, b) in periods]

    col = dims.x0
    axs2 = []
    axs3 = []
    y = dims.m1
    y += dims.h1
    ax = fig.add_axes([
        col / dims.width, 1 - y / dims.height, dims.w / dims.width,
        dims.h1 / dims.height
    ])
    ax.set_title(f'Types in subcorpora with {limit} {measure}')
    ax.set_xticks(xx, [])
    ax1 = ax
    last = ax
    y += dims.m2
    if dims.columns > 1:
        last.set_xticks(xx, periodlabels, rotation='vertical')
        col += dims.x1
        y = dims.m1
    for i, curve in enumerate(curves):
        if i != 0:
            y += dims.m3
        y += dims.h2
        ax = fig.add_axes([
            col / dims.width, 1 - y / dims.height, dims.w / dims.width,
            dims.h2 / dims.height
        ])
        if i == 0:
            ax.set_title(f'Significance of differences in time')
        ax.set_ylim(
            (-MAX_SIGNIFICANCE - SIG_MARG, MAX_SIGNIFICANCE + SIG_MARG))
        ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1), [])
        ax.set_xticks([], [])
        axs2.append(ax)
        last = ax
    last.set_xticks(xx, [])
    if has_cats:
        y += dims.m2
        for i, curve in enumerate(curves):
            if i != 0:
                y += dims.m3
            y += dims.h2
            ax = fig.add_axes([
                col / dims.width, 1 - y / dims.height, dims.w / dims.width,
                dims.h2 / dims.height
            ])
            if i == 0:
                ax.set_title(
                    f'Significance in comparison with other categories')
            ax.set_ylim(
                (-MAX_SIGNIFICANCE - SIG_MARG, MAX_SIGNIFICANCE + SIG_MARG))
            ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1), [])
            ax.set_xticks([], [])
            axs3.append(ax)
            last = ax
    last.set_xticks(xx, periodlabels, rotation='vertical')

    ymax = 0
    for i, curve in enumerate(curves):
        if curve['category']:
            color = COLORS[i]
        else:
            color = '#000000'
        label = _catname(restrictions + [curve['category']])
        points = [_get_avg(r) for r in curve['results']]
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

        points = [_get_vs(r, 'vs_time') for r in curve['results']]
        plotter(axs2[i], points)

        if has_cats:
            points = [_get_vs(r, 'vs_categories') for r in curve['results']]
            plotter(axs3[i], points)

    ax1.set_ylim((0, ymax * 1.05))
    ax1.legend(loc=legend)

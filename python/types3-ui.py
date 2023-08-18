import appdirs
import argparse
import hashlib
import json
import logging
import math
import matplotlib
import queue
import subprocess
import sys
import threading
import tkinter as tk
from collections import defaultdict
from pathlib import Path
from tkinter import ttk
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure

matplotlib.rcParams['axes.titlesize'] = 'medium'

cli = argparse.ArgumentParser()
cli.add_argument('--verbose',
                 '-v',
                 action='count',
                 default=0,
                 help='Increase verbosity')
cli.add_argument('infile', help='Input file (JSON)')


def sanity_check():
    tcltk_version = tk.Tcl().eval('info patchlevel')
    if tcltk_version.split('.') < ['8', '6']:
        logging.error(f'Unsupported Tcl/Tk version {tcltk_version}')
        sys.exit(1)


def metadata_choices(metadata):
    r = ['everything']
    m = {'everything': None}
    for k in sorted(metadata.keys()):
        for v in sorted(metadata[k]):
            l = f'{k} = {v}'
            assert l not in m
            m[l] = (k, v)
            r.append(l)
    return m, r


def metadata_top_choices(metadata):
    r = ['none']
    m = {'none': None}
    for k in sorted(metadata.keys()):
        vv = ', '.join(sorted(metadata[k]))
        l = f'{k} ({vv})'
        assert l not in m
        m[l] = k
        r.append(l)
    return m, r


def cmd_digest(x):
    x = json.dumps(x)
    x = bytes(x, encoding='utf-8')
    return hashlib.sha256(x).hexdigest()


MIN_ITER = 1_000
MAX_ITER = 100_000
ITER_STEP = 10
TIMEOUT = 0.1
MAX_SIGNIFICANCE = 4
FIG_WIDTH = 7
FIG_HEIGHT = 10
COLORS = ['#f26924', '#0088cc', '#3ec636']


def catname(cats):
    s = []
    for cat in cats:
        if cat is not None:
            k, v = cat
            s.append(v)
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


class Runner:

    def __init__(self, infile, cachedir, runner_queue, result_queue, root):
        self.infile = infile
        self.cachedir = cachedir
        self.runner_queue = runner_queue
        self.result_queue = result_queue
        self.root = root
        self.current = None
        self.process = None
        self.iter = None

    def msg(self, x):
        self.result_queue.put(x)
        self.root.event_generate('<<NewResults>>')

    def start_cmd(self):
        assert self.process is None
        assert self.current is not None
        assert self.iter is not None
        digest = cmd_digest(self.current)
        self.tempfile = self.cachedir / f'{digest}-{self.iter}.new'
        self.outfile = self.cachedir / f'{digest}-{self.iter}.json'
        full_cmd = [
            './types3-calc', self.infile, self.tempfile, '-i',
            str(self.iter)
        ] + self.current
        logging.debug(f'starting: {full_cmd}...')
        self.process = subprocess.Popen(full_cmd)

    def process_poll(self):
        assert self.process is not None
        assert self.current is not None
        assert self.iter is not None
        ret = self.process.poll()
        if ret is None:
            return
        self.process = None
        if ret != 0:
            logging.warning(f'process failed')
            self.iter = None
            self.current = None
            return
        logging.debug(f'process finished successfully')
        self.tempfile.rename(self.outfile)
        if self.iter < MAX_ITER:
            self.msg(('DONE-WORKING', self.current, self.iter))
            self.iter *= ITER_STEP
            self.start_cmd()
        else:
            self.msg(('DONE', self.current, self.iter))
            logging.debug(f'all iterations done')
            self.iter = None
            self.current = None

    def terminate(self):
        assert self.process is not None
        assert self.current is not None
        assert self.iter is not None
        logging.debug(f'stopping...')
        self.process.terminate()
        logging.debug(f'waiting...')
        self.process.wait()
        logging.debug(f'stopped')
        self.iter = None
        self.process = None
        self.current = None

    def try_cache(self):
        assert self.process is None
        assert self.current is not None
        assert self.iter is not None
        digest = cmd_digest(self.current)
        best = None
        all_done = False
        while True:
            cached = self.cachedir / f'{digest}-{self.iter}.json'
            if cached.exists():
                best = self.iter
                if self.iter < MAX_ITER:
                    self.iter *= ITER_STEP
                else:
                    all_done = True
                    break
            else:
                break
        if all_done:
            self.msg(('DONE', self.current, self.iter))
            logging.debug(f'all iterations in cache')
            self.iter = None
            self.current = None
        else:
            if best is not None:
                self.msg(('DONE-WORKING', self.current, best))
            else:
                self.msg(('WORKING', self.current, 0))
            self.start_cmd()

    def run(self):
        logging.debug(f'runner started')
        while True:
            if self.process:
                need_poll = False
                try:
                    cmd = self.runner_queue.get(timeout=TIMEOUT)
                except queue.Empty:
                    need_poll = True
                if need_poll:
                    self.process_poll()
                    continue
            else:
                cmd = self.runner_queue.get()
            if cmd == self.current:
                continue
            if self.process:
                self.terminate()
            if cmd == 'STOP':
                break
            assert self.iter is None
            self.iter = MIN_ITER
            self.current = cmd
            self.try_cache()
        logging.debug(f'runner done')


class App:

    def __init__(self, root, args):
        root.title('types3')
        self.infile = args.infile
        self.cur_args = None
        self._read_infile()
        self._setup_cache()
        self._build_ui(root)
        self._setup_menu(root)
        self._setup_hooks(root)
        logging.info(f'ready')

    def _read_infile(self):
        logging.info(f'read: {self.infile}')
        with open(self.infile) as f:
            raw_data = f.read()
        raw_bytes = bytes(raw_data, encoding='utf-8')
        self.data_digest = hashlib.sha256(raw_bytes).hexdigest()
        data = json.loads(raw_data)
        years = set()
        self.sample_metadata = defaultdict(set)
        self.token_metadata = defaultdict(set)
        for s in data['samples']:
            years.add(s['year'])
            for k, v in s['metadata'].items():
                self.sample_metadata[k].add(v)
            for t in s['tokens']:
                for k, v in t['metadata'].items():
                    self.token_metadata[k].add(v)

    def _setup_cache(self):
        self.cachedir = Path(
            appdirs.user_cache_dir('types3')) / self.data_digest
        self.cachedir.mkdir(parents=True, exist_ok=True)
        logging.debug(f'cache directory: {self.cachedir}')

    def _build_ui(self, root):
        mainframe = ttk.Frame(root, padding='3 3 3 3')
        mainframe.grid(column=0, row=0, sticky='nwes')
        root.columnconfigure(0, weight=1)
        root.rowconfigure(0, weight=1)

        canvasframe = ttk.Frame(mainframe,
                                padding='5 5 5 5',
                                borderwidth=1,
                                relief='sunken')
        canvasframe.grid(column=1, row=1, padx=3, pady=3, sticky='nw')
        widgetframe = ttk.Frame(mainframe, padding='5 5 5 5')
        widgetframe.grid(column=2, row=1, padx=3, pady=3, sticky='nw')

        widgetframe.columnconfigure(1, minsize=100)
        widgetframe.columnconfigure(2, minsize=300)

        row = 1

        e = ttk.Label(widgetframe, text='X axis:')
        e.grid(column=1, row=row, sticky='e')
        self.vs_what = tk.StringVar()
        vs_what_choices = ['tokens', 'words']
        e = ttk.OptionMenu(widgetframe, self.vs_what, vs_what_choices[0],
                           *vs_what_choices)
        e.grid(column=2, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Categories:')
        e.grid(column=1, row=row, sticky='e')
        self.category = tk.StringVar()
        self.category_map, category_choices = metadata_top_choices(
            self.sample_metadata)
        e = ttk.OptionMenu(widgetframe, self.category, category_choices[0],
                           *category_choices)
        e.grid(column=2, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Sample restriction:')
        e.grid(column=1, row=row, sticky='e')
        self.restrict_samples = tk.StringVar()
        self.restrict_samples_map, restrict_samples_choices = metadata_choices(
            self.sample_metadata)
        e = ttk.OptionMenu(widgetframe, self.restrict_samples,
                           restrict_samples_choices[0],
                           *restrict_samples_choices)
        e.grid(column=2, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Token restriction:')
        e.grid(column=1, row=row, sticky='e')
        self.restrict_tokens = tk.StringVar()
        self.restrict_tokens_map, restrict_tokens_choices = metadata_choices(
            self.token_metadata)
        e = ttk.OptionMenu(widgetframe, self.restrict_tokens,
                           restrict_tokens_choices[0],
                           *restrict_tokens_choices)
        e.grid(column=2, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Window size:')
        e.grid(column=1, row=row, sticky='e')
        self.window = tk.StringVar(value='10')
        e = ttk.Entry(widgetframe, textvariable=self.window, width=6)
        e.grid(column=2, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Step size:')
        e.grid(column=1, row=row, sticky='e')
        self.step = tk.StringVar(value='10')
        e = ttk.Entry(widgetframe, textvariable=self.step, width=6)
        e.grid(column=2, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Start year (optional):')
        e.grid(column=1, row=row, sticky='e')
        self.start = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.start, width=6)
        e.grid(column=2, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='End year (optional):')
        e.grid(column=1, row=row, sticky='e')
        self.end = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.end, width=6)
        e.grid(column=2, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Period offset (optional):')
        e.grid(column=1, row=row, sticky='e')
        self.offset = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.offset, width=6)
        e.grid(column=2, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Iterations:')
        e.grid(column=1, row=row, sticky='e')
        self.iter = tk.StringVar(value='')
        e = ttk.Label(widgetframe, textvariable=self.iter)
        e.grid(column=2, row=row, sticky='w')
        row += 1

        self.fig = Figure(figsize=(FIG_WIDTH, FIG_HEIGHT))
        self.canvas = FigureCanvasTkAgg(self.fig, master=canvasframe)
        self.canvas.draw()
        self.canvas.get_tk_widget().grid(column=1, row=1, sticky='w')

        for child in widgetframe.winfo_children():
            child.grid_configure(padx=5, pady=3)

    def _setup_menu(self, root):
        if root.tk.call('tk', 'windowingsystem') == 'aqua':
            # macOS: cmd-q and "Quit" in the application menu will
            # close the window instead of just killing Python
            menubar = tk.Menu(root)
            appmenu = tk.Menu(menubar, name='apple')
            menubar.add_cascade(menu=appmenu)
            root.createcommand('tk::mac::Quit', root.destroy)

    def _setup_hooks(self, root):
        self.vs_what.trace_add('write', self.update)
        self.category.trace_add('write', self.update)
        self.restrict_samples.trace_add('write', self.update)
        self.restrict_tokens.trace_add('write', self.update)
        root.bind('<<NewResults>>', self.new_results)
        self.result_queue = queue.Queue()
        self.runner_queue = queue.Queue()
        runner = Runner(self.infile, self.cachedir, self.runner_queue,
                        self.result_queue, root)
        self.runner_thread = threading.Thread(target=runner.run)
        self.runner_thread.start()
        self.update()

    def parse_required_int(self, errors, label, min, max, v):
        x = self.parse_opt_int(errors, label, min, max, v)
        if x is None:
            errors.append(f'{label} is required')
            return None
        return x

    def parse_opt_int(self, errors, label, min, max, v):
        if v is None:
            return None
        if v.strip() == '':
            return None
        try:
            x = int(v)
        except:
            errors.append(f'{label} is not a valid number')
            return None
        if min is not None and x < min:
            errors.append(f'{label} should be at least {min}')
            return None
        if max is not None and x > max:
            errors.append(f'{label} should be at most {max}')
            return None
        return x

    def update(self, *x):
        args = []
        errors = []
        window = self.parse_required_int(errors, 'Window size', 1, None,
                                         self.window.get())
        if window is not None:
            args += ['--window', str(window)]
        step = self.parse_required_int(errors, 'Step size', 1, None,
                                       self.step.get())
        if step is not None:
            args += ['--step', str(step)]
        start = self.parse_opt_int(errors, 'Start year', None, None,
                                   self.start.get())
        if start is not None:
            args += ['--start', str(start)]
        end = self.parse_opt_int(errors, 'End year', None, None,
                                 self.end.get())
        if end is not None:
            args += ['--end', str(end)]
        offset = self.parse_opt_int(errors, 'Offset', None, None,
                                    self.offset.get())
        if offset is not None:
            args += ['--offset', str(offset)]
        category = self.category_map[self.category.get()]
        if category is not None:
            args += ['--category', category]
        restrict_samples = self.restrict_samples_map[
            self.restrict_samples.get()]
        if restrict_samples is not None:
            args += ['--restrict-samples', '='.join(restrict_samples)]
        restrict_tokens = self.restrict_tokens_map[self.restrict_tokens.get()]
        if restrict_tokens is not None:
            args += ['--restrict-tokens', '='.join(restrict_tokens)]
        vs_what = self.vs_what.get()
        if vs_what == 'words':
            args += ['--words']
        if errors:
            logging.warning(errors)
            return
        if self.cur_args != args:
            self.cur_args = args
            self.runner_queue.put(args)

    def run(self, root):
        root.mainloop()
        logging.debug(f'stopping...')
        self.runner_queue.put('STOP')
        self.runner_thread.join()
        logging.info(f'done')

    def new_results(self, *_):
        to_draw = None
        while True:
            try:
                x = self.result_queue.get_nowait()
            except queue.Empty:
                break
            logging.debug(x)
            what, cmd, iter = x
            if what == 'WORKING':
                to_draw = None
                self.iter.set('… (working)')
            elif what == 'DONE-WORKING':
                to_draw = (cmd, iter)
                self.iter.set(f'{iter}… (more coming)')
            elif what == 'DONE':
                to_draw = (cmd, iter)
                self.iter.set(f'{iter} (all done)')
        if to_draw:
            self.draw(*to_draw)

    def draw(self, cmd, iter):
        digest = cmd_digest(cmd)
        outfile = self.cachedir / f'{digest}-{iter}.json'
        with open(outfile) as f:
            data = json.load(f)

        self.fig.clear()

        measure = data['measure']
        limit = data['limit']
        periods = data['periods']
        curves = data['curves']
        restrictions = [data['restrict_samples'], data['restrict_tokens']]
        has_cats = curves[0]['category'] is not None
        nn = len(curves)

        sigmarg = 0
        h1 = 4
        h2 = 0.8
        m1 = 0.3
        m2 = 0.6
        m3 = 0.1
        x = 0.1
        w = 0.8
        h = FIG_HEIGHT

        xx = [a for (a, b) in periods]
        periodlabels = [f'{a}–{b-1}' for (a, b) in periods]

        axs2 = []
        axs3 = []
        y = m1
        y += h1
        ax = self.fig.add_axes([x, 1 - y / h, w, h1 / h])
        ax.set_title(f'Types in subcorpora with {limit} {measure}')
        ax.set_xticks(xx, [])
        ax1 = ax
        last = ax
        y += m2
        for i, curve in enumerate(curves):
            if i != 0:
                y += m3
            y += h2
            ax = self.fig.add_axes([x, 1 - y / h, w, h2 / h])
            if i == 0:
                ax.set_title(f'Significance of differences in time')
            ax.set_ylim(
                (-MAX_SIGNIFICANCE - sigmarg, MAX_SIGNIFICANCE + sigmarg))
            ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1), [])
            ax.set_xticks([], [])
            axs2.append(ax)
            last = ax
        last.set_xticks(xx, [])
        if has_cats:
            y += m2
            for i, curve in enumerate(curves):
                if i != 0:
                    y += m3
                y += h2
                ax = self.fig.add_axes([x, 1 - y / h, w, h2 / h])
                if i == 0:
                    ax.set_title(
                        f'Significance in comparison with other categories')
                ax.set_ylim(
                    (-MAX_SIGNIFICANCE - sigmarg, MAX_SIGNIFICANCE + sigmarg))
                ax.set_yticks(range(-MAX_SIGNIFICANCE, MAX_SIGNIFICANCE + 1),
                              [])
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
            label = catname(restrictions + [curve['category']])
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
                ax.fill_between(xx,
                                yy1,
                                yy2,
                                color=color,
                                alpha=0.8,
                                linewidth=0)
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
        ax1.legend(loc='lower right')
        self.canvas.draw()


if __name__ == '__main__':
    args = cli.parse_args()
    if args.verbose >= 1:
        loglevel = logging.DEBUG
    else:
        loglevel = logging.INFO
    logging.basicConfig(format='%(levelname)s %(message)s', level=loglevel)
    sanity_check()
    root = tk.Tk()
    App(root, args).run(root)

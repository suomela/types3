import argparse
import hashlib
import json
import logging
import math
import os
import queue
import subprocess
import sys
import threading
import tkinter as tk
from collections import defaultdict
from pathlib import Path
from tkinter import ttk
import appdirs
import matplotlib
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
from matplotlib.figure import Figure
import types3.plot

matplotlib.rcParams['axes.titlesize'] = 'medium'

OUTPUT_VERSION = 'v3'
MIN_ITER = 1_000
MAX_ITER = 1_000_000
ITER_STEP = 10
TIMEOUT = 0.1
WINDOW_INIT_SIZE = '1200x1050'
WIDGET_WIDTH = 300

cli = argparse.ArgumentParser()
cli.add_argument('--verbose',
                 '-v',
                 action='count',
                 default=0,
                 help='Increase verbosity')
cli.add_argument('infile', help='Input file (JSON)')
cli.add_argument('--version',
                 action='version',
                 version='%(prog)s ' + types3.__version__)


def sanity_check():
    tcltk_version = tk.Tcl().eval('info patchlevel')
    if tcltk_version.split('.') < ['8', '6']:
        logging.error(f'Unsupported Tcl/Tk version {tcltk_version}')
        sys.exit(1)
    if 'TYPES3_BASEDIR' not in os.environ:
        logging.error('TYPES3_BASEDIR environment variable not defined')
        sys.exit(1)


def metadata_choices(metadata):
    r = ['everything']
    m = {'everything': None}
    for k in sorted(metadata.keys()):
        for v in sorted(metadata[k]):
            l = f'{k}: {v}'
            assert l not in m
            m[l] = (k, v)
            r.append(l)
    return m, r


def metadata_top_choices(metadata):
    r = ['none']
    m = {'none': None}
    for k in sorted(metadata.keys()):
        vv = ', '.join(sorted(metadata[k]))
        if len(vv) > 25:
            vv = vv[:20] + '…'
        l = f'{k} ({vv})'
        assert l not in m
        m[l] = k
        r.append(l)
    return m, r


def cmd_digest(x):
    x = json.dumps(x)
    x = bytes(x, encoding='utf-8')
    return hashlib.sha256(x).hexdigest()


class Runner:

    def __init__(self, infile, cachedir, verbose, runner_queue, result_queue,
                 root):
        self.infile = infile
        self.cachedir = cachedir
        self.verbose = verbose
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
        self.errfile = self.cachedir / f'{digest}-{self.iter}.err'
        self.tempfile = self.cachedir / f'{digest}-{self.iter}.new'
        self.outfile = self.cachedir / f'{digest}-{self.iter}.json'
        basedir = Path(os.environ['TYPES3_BASEDIR'])
        tool = basedir / 'target/release/types3'
        base_args = [
            tool, self.infile, self.tempfile, '--error-file', self.errfile,
            '--iter',
            str(self.iter)
        ]
        for _ in range(self.verbose):
            base_args += ['--verbose']
        full_cmd = base_args + self.current
        logging.debug(f'starting: {full_cmd}...')
        try:
            self.process = subprocess.Popen(full_cmd)
        except Exception as e:
            logging.warning(f'starting {full_cmd} failed with {e}')
            error = 'Cannot start calculations.'
            self.msg(('ERROR', self.current, self.iter, error))
            self.iter = None
            self.current = None

    def process_poll(self):
        assert self.process is not None
        assert self.current is not None
        assert self.iter is not None
        ret = self.process.poll()
        if ret is None:
            return
        self.process = None
        if ret != 0:
            error = 'Unknown error during calculation.'
            if self.errfile.exists():
                with open(self.errfile) as f:
                    error_struct = json.load(f)
                    error = error_struct['error']
                self.errfile.unlink()
            else:
                logging.warning('process failed without telling why')
            self.msg(('ERROR', self.current, self.iter, error))
            self.iter = None
            self.current = None
            return
        logging.debug('process finished successfully')
        self.tempfile.rename(self.outfile)
        if self.iter < MAX_ITER:
            self.msg(('DONE-WORKING', self.current, self.iter, None))
            self.iter *= ITER_STEP
            self.start_cmd()
        else:
            self.msg(('DONE', self.current, self.iter, None))
            logging.debug('all iterations done')
            self.iter = None
            self.current = None

    def terminate(self):
        assert self.process is not None
        assert self.current is not None
        assert self.iter is not None
        logging.debug('stopping...')
        self.process.terminate()
        logging.debug('waiting...')
        self.process.wait()
        logging.debug('stopped')
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
            self.msg(('DONE', self.current, self.iter, None))
            logging.debug('all iterations in cache')
            self.iter = None
            self.current = None
        else:
            if best is not None:
                self.msg(('DONE-WORKING', self.current, best, None))
            else:
                self.msg(('WORKING', self.current, 0, None))
            self.start_cmd()

    def run(self):
        logging.debug('runner started')
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
        logging.debug('runner done')


class App:

    def __init__(self, root, args):
        root.title('types3')
        self.verbose = args.verbose
        self.infile = args.infile
        self.cur_args = None
        self.cur_outfile = None
        self._read_infile()
        self._setup_cache()
        self._build_ui(root)
        self._setup_menu(root)
        self._setup_hooks(root)
        logging.debug('ready')

    def _read_infile(self):
        logging.debug(f'read: {self.infile}')
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
        gcd = math.gcd(*years)
        self.default_step = max(gcd, 10)

    def _setup_cache(self):
        self.cachedir = Path(appdirs.user_cache_dir(
            'types3')) / OUTPUT_VERSION / self.data_digest
        self.cachedir.mkdir(parents=True, exist_ok=True)
        logging.debug(f'cache directory: {self.cachedir}')

    def _build_ui(self, root):
        root.geometry(WINDOW_INIT_SIZE)
        root.columnconfigure(0, weight=1)
        root.rowconfigure(0, weight=1)

        mainframe = ttk.Frame(root)
        mainframe.grid(column=0, row=0, sticky='nwes')
        mainframe.rowconfigure(0, weight=1)
        mainframe.columnconfigure(0, weight=1)
        mainframe.columnconfigure(1, weight=0)

        canvasframe = ttk.Frame(mainframe)
        canvasframe.grid(column=0, row=0, padx=3, pady=3, sticky='nwes')
        canvasframe.columnconfigure(0, weight=1)
        canvasframe.columnconfigure(1, weight=0)
        canvasframe.rowconfigure(0, weight=1)
        canvasframe.rowconfigure(1, weight=0)

        widgetframe = ttk.Frame(mainframe, padding='5 5 5 5')
        widgetframe.grid(column=1, row=0, padx=3, pady=3, sticky='nw')
        widgetframe.columnconfigure(1, minsize=WIDGET_WIDTH)

        row = 0

        e = ttk.Label(widgetframe, text='Export:')
        e.grid(column=0, row=row, sticky='e')
        e = ttk.Button(widgetframe, text='Save as…', command=self.save)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        self.save_format = tk.StringVar()
        what_choices = [
            'PDF',
            'PNG',
        ]
        e = ttk.OptionMenu(widgetframe, self.save_format, what_choices[0],
                           *what_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        self.save_wide = tk.StringVar(value='')
        e = ttk.Checkbutton(widgetframe,
                            text='Wide layout',
                            onvalue='wide',
                            offvalue='',
                            variable=self.save_wide)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        self.save_large = tk.StringVar(value='')
        e = ttk.Checkbutton(widgetframe,
                            text='Large fonts',
                            onvalue='large',
                            offvalue='',
                            variable=self.save_large)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Legend:')
        e.grid(column=0, row=row, sticky='e')
        self.save_legend = tk.StringVar()
        what_choices = [
            'lower right',
            'lower left',
            'upper right',
            'upper left',
        ]
        e = ttk.OptionMenu(widgetframe, self.save_legend, what_choices[0],
                           *what_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='PNG DPI:')
        e.grid(column=0, row=row, sticky='e')
        self.save_dpi = tk.StringVar()
        what_choices = [
            '100',
            '200',
            '300',
            '400',
            '600',
        ]
        e = ttk.OptionMenu(widgetframe, self.save_dpi, '300', *what_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Separator(widgetframe, orient='horizontal')
        e.grid(column=0, row=row, columnspan=2, sticky='ew')
        row += 1

        e = ttk.Label(widgetframe, text='Calculate:')
        e.grid(column=0, row=row, sticky='e')
        self.what = tk.StringVar()
        what_choices = [
            'types vs. tokens, using samples',
            'types vs. tokens, individually',
            'types vs. words, using samples',
            'hapaxes vs. tokens, using samples',
            'hapaxes vs. tokens, individually',
            'hapaxes vs. words, using samples',
            'tokens vs. words, using samples',
            'samples vs. tokens',
            'samples vs. words',
            'type ratio, using samples',
            'type ratio, individually',
            # Useful for testing:
            # 'tokens vs. tokens, using samples',
            # 'tokens vs. tokens, individually',
        ]
        e = ttk.OptionMenu(widgetframe, self.what, what_choices[0],
                           *what_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='What is relevant:')
        e.grid(column=0, row=row, sticky='e')
        self.mark_tokens = tk.StringVar()
        self.mark_tokens_map, mark_tokens_choices = metadata_choices(
            self.token_metadata)
        e = ttk.OptionMenu(widgetframe, self.mark_tokens,
                           mark_tokens_choices[0], *mark_tokens_choices)
        e.grid(column=1, row=row, sticky='w')
        self.mark_tokens_menu = e
        self.mark_tokens_menu.configure(state="disabled")
        row += 1

        e = ttk.Label(widgetframe, text='Categories:')
        e.grid(column=0, row=row, sticky='e')
        self.category = tk.StringVar()
        self.category_map, category_choices = metadata_top_choices(
            self.sample_metadata)
        e = ttk.OptionMenu(widgetframe, self.category, category_choices[0],
                           *category_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Sample restriction:')
        e.grid(column=0, row=row, sticky='e')
        self.restrict_samples = tk.StringVar()
        self.restrict_samples_map, restrict_samples_choices = metadata_choices(
            self.sample_metadata)
        e = ttk.OptionMenu(widgetframe, self.restrict_samples,
                           restrict_samples_choices[0],
                           *restrict_samples_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Label(widgetframe, text='Token restriction:')
        e.grid(column=0, row=row, sticky='e')
        self.restrict_tokens = tk.StringVar()
        self.restrict_tokens_map, restrict_tokens_choices = metadata_choices(
            self.token_metadata)
        e = ttk.OptionMenu(widgetframe, self.restrict_tokens,
                           restrict_tokens_choices[0],
                           *restrict_tokens_choices)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Separator(widgetframe, orient='horizontal')
        e.grid(column=0, row=row, columnspan=2, sticky='ew')
        row += 1

        e = ttk.Label(widgetframe, text='Window size:')
        e.grid(column=0, row=row, sticky='e')
        self.window = tk.StringVar(value=str(self.default_step))
        e = ttk.Entry(widgetframe, textvariable=self.window, width=6)
        e.grid(column=1, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Step size:')
        e.grid(column=0, row=row, sticky='e')
        self.step = tk.StringVar(value=str(self.default_step))
        e = ttk.Entry(widgetframe, textvariable=self.step, width=6)
        e.grid(column=1, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Start year (optional):')
        e.grid(column=0, row=row, sticky='e')
        self.start = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.start, width=6)
        e.grid(column=1, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='End year (optional):')
        e.grid(column=0, row=row, sticky='e')
        self.end = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.end, width=6)
        e.grid(column=1, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(widgetframe, text='Period offset (optional):')
        e.grid(column=0, row=row, sticky='e')
        self.offset = tk.StringVar()
        e = ttk.Entry(widgetframe, textvariable=self.offset, width=6)
        e.grid(column=1, row=row, sticky='w')
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Separator(widgetframe, orient='horizontal')
        e.grid(column=0, row=row, columnspan=2, sticky='ew')
        row += 1

        e = ttk.Label(widgetframe, text='Iterations:')
        e.grid(column=0, row=row, sticky='e')
        self.iter = tk.StringVar(value='')
        e = ttk.Label(widgetframe, textvariable=self.iter)
        e.grid(column=1, row=row, sticky='w')
        row += 1

        e = ttk.Separator(widgetframe, orient='horizontal')
        e.grid(column=0, row=row, columnspan=2, sticky='ew')
        row += 1

        self.error = tk.StringVar(value='')
        e = ttk.Label(widgetframe,
                      textvariable=self.error,
                      wraplength=WIDGET_WIDTH)
        e.grid(column=0, columnspan=2, row=row, sticky='w')
        row += 1

        for child in widgetframe.winfo_children():
            child.grid_configure(padx=5, pady=3)

        scrollablecanvas = tk.Canvas(canvasframe,
                                     borderwidth=0,
                                     highlightthickness=0)
        scrollableframe = ttk.Frame(scrollablecanvas)
        scrollablecanvas.grid(column=0, row=0, sticky='nesw')
        sx = ttk.Scrollbar(canvasframe,
                           orient='horizontal',
                           command=scrollablecanvas.xview)
        sx.grid(row=1, column=0, sticky='ew')
        sy = ttk.Scrollbar(canvasframe,
                           orient='vertical',
                           command=scrollablecanvas.yview)
        sy.grid(row=0, column=1, sticky='ns')
        scrollablecanvas.configure(yscrollcommand=sy.set,
                                   xscrollcommand=sx.set)
        scrollablecanvas.grid(row=0, column=0, sticky='nesw')
        scrollablecanvas.create_window((0, 0),
                                       window=scrollableframe,
                                       anchor='nw')
        scrollableframe.bind(
            '<Configure>', lambda ev: scrollablecanvas.configure(
                scrollregion=scrollablecanvas.bbox('all')))

        dims = types3.plot.DIMS_UI
        self.fig = Figure(figsize=(dims.width, dims.height))
        self.canvas = FigureCanvasTkAgg(self.fig, master=scrollableframe)
        self.canvas.draw()
        e = self.canvas.get_tk_widget()
        e.grid(column=0, row=0, sticky='ne')

    def _setup_menu(self, root):
        if root.tk.call('tk', 'windowingsystem') == 'aqua':
            # macOS: cmd-q and "Quit" in the application menu will
            # close the window instead of just killing Python
            menubar = tk.Menu(root)
            appmenu = tk.Menu(menubar, name='apple')
            menubar.add_cascade(menu=appmenu)
            root.createcommand('tk::mac::Quit', root.destroy)

    def _setup_hooks(self, root):
        self.what.trace_add('write', self.update)
        self.category.trace_add('write', self.update)
        self.restrict_samples.trace_add('write', self.update)
        self.restrict_tokens.trace_add('write', self.update)
        self.mark_tokens.trace_add('write', self.update)
        root.bind('<<NewResults>>', self.new_results)
        self.result_queue = queue.Queue()
        self.runner_queue = queue.Queue()
        runner = Runner(self.infile, self.cachedir, self.verbose,
                        self.runner_queue, self.result_queue, root)
        self.runner_thread = threading.Thread(target=runner.run)
        self.runner_thread.start()
        self.update()

    def parse_required_int(self, errors, label, vmin, vmax, v):
        x = self.parse_opt_int(errors, label, vmin, vmax, v)
        if x is None:
            errors.append(f'{label} is required.')
            return None
        return x

    def parse_opt_int(self, errors, label, vmin, vmax, v):
        if v is None:
            return None
        if v.strip() == '':
            return None
        try:
            x = int(v)
        except:
            errors.append(f'{label} is not a valid number.')
            return None
        if vmin is not None and x < vmin:
            errors.append(f'{label} should be at least {vmin}.')
            return None
        if vmax is not None and x > vmax:
            errors.append(f'{label} should be at most {vmax}.')
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
        mark_tokens = self.mark_tokens_map[self.mark_tokens.get()]
        if mark_tokens is not None:
            args += ['--mark-tokens', '='.join(mark_tokens)]
        what = self.what.get()
        extra, marked = {
            'types vs. tokens, using samples': ([], False),
            'types vs. tokens, individually': (['--split-samples'], False),
            'types vs. words, using samples': (['--words'], False),
            'hapaxes vs. tokens, using samples': (['--count-hapaxes'], False),
            'hapaxes vs. tokens, individually':
            ['--count-hapaxes', '--split-samples'],
            'hapaxes vs. words, using samples':
            (['--count-hapaxes', '--words'], False),
            'tokens vs. tokens, using samples': (['--count-tokens'], False),
            'tokens vs. tokens, individually':
            ['--count-tokens', '--split-samples'],
            'tokens vs. words, using samples': (['--count-tokens',
                                                 '--words'], False),
            'samples vs. tokens': (['--count-samples'], False),
            'samples vs. words': (['--count-samples', '--words'], False),
            'type ratio, using samples': (['--type-ratio'], True),
            'type ratio, individually': (['--type-ratio',
                                          '--split-samples'], True),
        }.get(what, ([], False))
        args += extra
        if marked:
            self.mark_tokens_menu.configure(state="normal")
        else:
            self.mark_tokens_menu.configure(state="disabled")
        if errors:
            logging.debug(errors)
            self.error.set('\n'.join(errors))
            return
        if self.cur_args != args:
            self.cur_args = args
            self.runner_queue.put(args)

    def run(self, root):
        root.mainloop()
        logging.debug('stopping...')
        self.runner_queue.put('STOP')
        self.runner_thread.join()
        logging.debug('done')

    def new_results(self, *_):
        to_draw = None
        while True:
            try:
                x = self.result_queue.get_nowait()
            except queue.Empty:
                break
            logging.debug(x)
            what, cmd, iter, error = x
            if what == 'WORKING':
                to_draw = None
                self.iter.set('… (working)')
                self.error.set('')
            elif what == 'DONE-WORKING':
                to_draw = (cmd, iter)
                self.iter.set(f'{iter}… (more coming)')
                self.error.set('')
            elif what == 'DONE':
                to_draw = (cmd, iter)
                self.iter.set(f'{iter} (all done)')
                self.error.set('')
            elif what == 'ERROR':
                self.iter.set('—')
                self.error.set(error)
            else:
                assert False, what
        if to_draw:
            self.draw(*to_draw)

    def draw(self, cmd, iter):
        digest = cmd_digest(cmd)
        outfile = self.cachedir / f'{digest}-{iter}.json'
        self.cur_outfile = outfile
        with open(outfile) as f:
            data = json.load(f)

        self.fig.clear()
        dims = types3.plot.DIMS_UI
        types3.plot.plot(self.fig, data, dims, legend='lower right')
        self.canvas.draw()

    def save(self, *_):
        if self.cur_outfile is None:
            return
        ftmap = {
            'PDF': [('PDF', '*.pdf')],
            'PNG': [('PNG', '*.png')],
        }
        fmt = self.save_format.get()
        if fmt not in ftmap:
            fmt = 'PDF'
        filetypes = ftmap[fmt]
        save_filename = tk.filedialog.asksaveasfilename(
            filetypes=filetypes,
            defaultextension=filetypes,
            initialfile='types3')
        if not save_filename:
            return
        basedir = Path(os.environ['TYPES3_BASEDIR'])
        tool = basedir / 'types3-plot'
        cmd = [
            tool,
            self.cur_outfile,
            save_filename,
            '--legend',
            self.save_legend.get(),
            '--dpi',
            self.save_dpi.get(),
        ]
        if self.save_wide.get() == 'wide':
            cmd += ['--wide']
        if self.save_large.get() == 'large':
            cmd += ['--large']
        for _ in range(self.verbose):
            cmd += ['--verbose']
        try:
            subprocess.run(cmd, check=True)
        except Exception as e:
            logging.warning(f'starting {cmd} failed with {e}')
            tk.messagebox.showerror(
                message=f'Could not export as {save_filename}')


def main():
    args = cli.parse_args()
    if args.verbose >= 2:
        loglevel = logging.DEBUG
    elif args.verbose >= 1:
        loglevel = logging.INFO
    else:
        loglevel = logging.WARN
    logging.basicConfig(format='%(levelname)s %(message)s', level=loglevel)
    sanity_check()
    root = tk.Tk()
    App(root, args).run(root)


if __name__ == '__main__':
    main()

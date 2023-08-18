import appdirs
import argparse
import json
import logging
import math
# import matplotlib
# matplotlib.use('Agg')
# matplotlib.rcParams['axes.titlesize'] = 'medium'
# import matplotlib.pyplot as plt
import queue
import subprocess
import sys
import threading
import tkinter as tk
from collections import defaultdict
from pathlib import Path
from tkinter import ttk

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


MIN_ITER = 1_000
MAX_ITER = 100_000
ITER_STEP = 10
TIMEOUT = 0.1


class Runner:

    def __init__(self, args, runner_queue, result_queue, root):
        self.infile = args.infile
        self.runner_queue = runner_queue
        self.result_queue = result_queue
        self.root = root
        self.current = None
        self.process = None
        self.iter = None

    def msg(self, x):
        self.root.event_generate('<<NewResults>>')
        self.result_queue.put(x)

    def start_cmd(self):
        assert self.process is None
        assert self.current is not None
        assert self.iter is not None
        full_cmd = [
            './types3-calc', self.infile, 'temp.json', '-i',
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
            self.msg(('WORKING', ))
            self.start_cmd()
        logging.debug(f'runner done')


class App:

    def __init__(self, root, args):
        root.title('types3')

        self.cur_args = None

        self.cachedir = Path(appdirs.user_cache_dir('types3'))
        self.cachedir.mkdir(parents=True, exist_ok=True)
        logging.debug(f'cache directory: {self.cachedir}')

        self.infile = args.infile
        logging.info(f'read: {self.infile}')
        with open(self.infile) as f:
            data = json.load(f)
        years = set()
        sample_metadata = defaultdict(set)
        token_metadata = defaultdict(set)
        for s in data['samples']:
            years.add(s['year'])
            for k, v in s['metadata'].items():
                sample_metadata[k].add(v)
            for t in s['tokens']:
                for k, v in t['metadata'].items():
                    token_metadata[k].add(v)

        mainframe = ttk.Frame(root, padding='3 3 12 12')
        mainframe.grid(column=0, row=0, sticky=(tk.N, tk.W, tk.E, tk.S))
        root.columnconfigure(0, weight=1)
        root.rowconfigure(0, weight=1)

        mainframe.columnconfigure(1, minsize=100)
        mainframe.columnconfigure(2, minsize=300)
        mainframe.columnconfigure(3, minsize=100)
        mainframe.columnconfigure(4, minsize=100)

        row = 1

        e = ttk.Label(mainframe, text='X axis:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.vs_what = tk.StringVar()
        vs_what_choices = ['tokens', 'words']
        e = ttk.OptionMenu(mainframe, self.vs_what, vs_what_choices[0],
                           *vs_what_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Categories:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.category = tk.StringVar()
        self.category_map, category_choices = metadata_top_choices(
            sample_metadata)
        e = ttk.OptionMenu(mainframe, self.category, category_choices[0],
                           *category_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Sample restriction:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.restrict_samples = tk.StringVar()
        self.restrict_samples_map, restrict_samples_choices = metadata_choices(
            sample_metadata)
        e = ttk.OptionMenu(mainframe, self.restrict_samples,
                           restrict_samples_choices[0],
                           *restrict_samples_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Token restriction:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.restrict_tokens = tk.StringVar()
        self.restrict_tokens_map, restrict_tokens_choices = metadata_choices(
            token_metadata)
        e = ttk.OptionMenu(mainframe, self.restrict_tokens,
                           restrict_tokens_choices[0],
                           *restrict_tokens_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        row = 1

        e = ttk.Label(mainframe, text='Window size:')
        e.grid(column=3, row=row, sticky=tk.E)
        self.window = tk.StringVar(value='10')
        e = ttk.Entry(mainframe, textvariable=self.window, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(mainframe, text='Step size:')
        e.grid(column=3, row=row, sticky=tk.E)
        self.step = tk.StringVar(value='10')
        e = ttk.Entry(mainframe, textvariable=self.step, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(mainframe, text='Start year (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.start = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.start, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(mainframe, text='End year (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.end = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.end, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        e = ttk.Label(mainframe, text='Period offset (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.offset = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.offset, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        e.bind('<FocusOut>', self.update)
        e.bind('<Return>', self.update)
        row += 1

        for child in mainframe.winfo_children():
            child.grid_configure(padx=5, pady=2)

        # macOS: cmd-q and "Quit" in the application menu will close the window instead of just killing Python
        if root.tk.call('tk', 'windowingsystem') == 'aqua':
            menubar = tk.Menu(root)
            appmenu = tk.Menu(menubar, name='apple')
            menubar.add_cascade(menu=appmenu)
            root.createcommand('tk::mac::Quit', root.destroy)

        self.vs_what.trace_add('write', self.update)
        self.category.trace_add('write', self.update)
        self.restrict_samples.trace_add('write', self.update)
        self.restrict_tokens.trace_add('write', self.update)

        root.bind('<<NewResults>>', self.new_results)
        self.result_queue = queue.Queue()
        self.runner_queue = queue.Queue()
        runner = Runner(args, self.runner_queue, self.result_queue, root)
        self.runner_thread = threading.Thread(target=runner.run)
        self.runner_thread.start()
        self.update()
        logging.info(f'ready')

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
        while True:
            try:
                x = self.result_queue.get_nowait()
                logging.debug(f'got results: {x}')
            except queue.Empty:
                break


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

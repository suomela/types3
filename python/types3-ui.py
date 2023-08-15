import appdirs
import argparse
import json
import logging
import math
# import matplotlib
# matplotlib.use('Agg')
# matplotlib.rcParams['axes.titlesize'] = 'medium'
# import matplotlib.pyplot as plt
import sys
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
    for k in sorted(metadata.keys()):
        for v in sorted(metadata[k]):
            r.append(f'{k}={v}')
    return r


def metadata_top_choices(metadata):
    r = ['none']
    for k in sorted(metadata.keys()):
        vv = ', '.join(sorted(metadata[k]))
        r.append(f'{k} ({vv})')
    return r


class App:

    def __init__(self, root, args):
        root.title('types3')

        self.cachedir = Path(appdirs.user_cache_dir('types3'))
        self.cachedir.mkdir(parents=True, exist_ok=True)
        logging.debug(f'cache directory: {self.cachedir}')

        logging.info(f'read: {args.infile}')
        with open(args.infile) as f:
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
        self.compare = tk.StringVar()
        compare_choices = metadata_top_choices(sample_metadata)
        e = ttk.OptionMenu(mainframe, self.compare, compare_choices[0],
                           *compare_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Sample restriction:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.restrict_samples = tk.StringVar()
        restrict_samples_choices = metadata_choices(sample_metadata)
        e = ttk.OptionMenu(mainframe, self.restrict_samples,
                           restrict_samples_choices[0],
                           *restrict_samples_choices)
        e.grid(column=2, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Token restriction:')
        e.grid(column=1, row=row, sticky=tk.E)
        self.restrict_tokens = tk.StringVar()
        restrict_tokens_choices = metadata_choices(token_metadata)
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
        row += 1

        e = ttk.Label(mainframe, text='Step size:')
        e.grid(column=3, row=row, sticky=tk.E)
        self.step = tk.StringVar(value='10')
        e = ttk.Entry(mainframe, textvariable=self.step, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Start year (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.start = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.start, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='End year (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.end = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.end, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        row += 1

        e = ttk.Label(mainframe, text='Period offset (optional):')
        e.grid(column=3, row=row, sticky=tk.E)
        self.offset = tk.StringVar()
        e = ttk.Entry(mainframe, textvariable=self.offset, width=6)
        e.grid(column=4, row=row, sticky=tk.W)
        row += 1

        for child in mainframe.winfo_children():
            child.grid_configure(padx=5, pady=2)

        logging.info(f'ready')


if __name__ == '__main__':
    args = cli.parse_args()
    if args.verbose >= 1:
        loglevel = logging.DEBUG
    else:
        loglevel = logging.INFO
    logging.basicConfig(format='%(levelname)s %(message)s', level=loglevel)
    sanity_check()
    root = tk.Tk()
    app = App(root, args)
    root.mainloop()
    logging.info(f'done')

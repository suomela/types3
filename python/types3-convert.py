import argparse
import csv
import json
import logging
import sys
import types3

cli = argparse.ArgumentParser(
    description='Convert CSV files to types3-compatible format.')
cli.add_argument('--verbose',
                 '-v',
                 action='count',
                 default=0,
                 help='Increase verbosity')
cli.add_argument('samplefile', help='Input file: samples (CSV)')
cli.add_argument('tokenfile', help='Input file: tokens (CSV)')
cli.add_argument('outfile', help='Output file (JSON)')
cli.add_argument('--version',
                 action='version',
                 version='%(prog)s ' + types3.__version__)


class MyError(Exception):
    pass


def convert(args):
    logging.info(f'{args.samplefile}: reading')
    samples = []
    samplemap = {}
    with open(args.samplefile, newline='', encoding='utf-8-sig') as f:
        for r in csv.DictReader(f):
            KEYS = ['id', 'words', 'year']
            for key in KEYS:
                if key not in r:
                    got = ', '.join(sorted(r.keys()))
                    raise MyError(
                        f'{args.samplefile}: I was expecting to see column {key} in the sample file, but I only got these columns: {got}'
                    )
            metadata = {}
            for key, value in r.items():
                if key not in KEYS:
                    metadata[key] = value
            sample = dict(
                id=r['id'],
                words=int(r['words']),
                year=int(r['year']),
                metadata=metadata,
                tokens=[],
            )
            samples.append(sample)
            sample_id = sample['id']
            if sample_id in samplemap:
                raise MyError(
                    f'{args.samplefile}: duplicate sample ID {sample_id}')
            samplemap[sample_id] = sample
    logging.info(f'{args.samplefile}: {len(samples)} samples')
    logging.info(f'{args.tokenfile}: reading')
    tokencount = 0
    with open(args.tokenfile, newline='', encoding='utf-8-sig') as f:
        for r in csv.DictReader(f):
            KEYS = ['id', 'lemma']
            for key in KEYS:
                if key not in r:
                    got = ', '.join(sorted(r.keys()))
                    raise MyError(
                        f'{args.tokenfile}: I was expecting to see column {key} in the token file, but I only got these columns: {got}'
                    )
            sample_id = r['id']
            if sample_id not in samplemap:
                raise MyError(
                    f'{args.tokenfile}: sample ID {sample_id} was not found in {args.samplefile}'
                )
            sample = samplemap[sample_id]
            metadata = {}
            for key, value in r.items():
                if key not in KEYS:
                    metadata[key] = value
            token = dict(
                lemma=r['lemma'],
                metadata=metadata,
            )
            sample['tokens'].append(token)
            tokencount += 1
    logging.info(f'{args.samplefile}: {tokencount} tokens')
    logging.info(f'{args.outfile}: writing')
    data = dict(samples=samples)
    with open(args.outfile, 'w') as f:
        json.dump(data, f, indent=1)
    logging.info(f'{args.outfile}: done')


def main():
    args = cli.parse_args()
    if args.verbose >= 2:
        loglevel = logging.DEBUG
    elif args.verbose >= 1:
        loglevel = logging.INFO
    else:
        loglevel = logging.WARN
    logging.basicConfig(format='%(levelname)s %(message)s', level=loglevel)
    try:
        convert(args)
    except MyError as e:
        logging.exception(e, exc_info=False)
        sys.exit(1)
    except OSError as e:
        logging.exception(e, exc_info=False)
        sys.exit(1)


if __name__ == '__main__':
    main()

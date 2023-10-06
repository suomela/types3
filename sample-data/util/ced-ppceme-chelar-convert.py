import json
import re
import sys

# Genre classifications from https://github.com/suomela/suffix-competition-code

CLASS_SPEECH = {
    'Letters, private': 'speech-related',
    'Diary, private': 'speech-related',
    'Trial proceedings': 'speech-related',
    'Witness depositions': 'speech-related',
    'Drama comedy': 'speech-related',
    'Sermon': 'speech-related',
    'Bible': 'writing-based',
    'Educational treatise': 'writing-based',
    '(Auto)biography': 'writing-based',
    'Travelogue': 'writing-based',
    'History': 'writing-based',
    'Law': 'writing-based',
    'Law reports': 'writing-based',
    'Medicine': 'writing-based',
    'Philosophy': 'writing-based',
    'Letters, non-private': 'writing-based',
    'Science, other': 'writing-based',
}

CLASS_LEGAL = {
    'Letters, private': 'other',
    'Diary, private': 'other',
    'Trial proceedings': 'other',
    'Witness depositions': 'other',
    'Drama comedy': 'other',
    'Sermon': 'other',
    'Bible': 'other',
    'Educational treatise': 'other',
    '(Auto)biography': 'other',
    'Travelogue': 'other',
    'History': 'other',
    'Law': 'legal',
    'Law reports': 'legal',
    'Medicine': 'other',
    'Philosophy': 'other',
    'Letters, non-private': 'other',
    'Science, other': 'other',
}

# Ranges of years are replaced with the midpoint


def parse_year(x):
    m = re.fullmatch(r'[ac]?(1[5-7][0-9][0-9])', x)
    if m:
        return int(m.group(1))
    m = re.fullmatch(r'c?(1[5-7])([0-9][0-9])-c?([0-9][0-9])', x)
    if m:
        y1 = int(m.group(1) + m.group(2))
        y2 = int(m.group(1) + m.group(3))
        y = round((y1 + y2) / 2)
        return y
    m = re.fullmatch(r'c?(1[5-7][0-9][0-9])-c?(1[5-7][0-9][0-9])', x)
    if m:
        y1 = int(m.group(1))
        y2 = int(m.group(2))
        y = round((y1 + y2) / 2)
        return y
    assert False, x


def main():
    srcfile, destfile = sys.argv[1:]
    with open(srcfile) as f:
        input_data = json.load(f)

    samplemap = {}

    for d in input_data['samples']:
        sample = d['sample']
        corpus = d['corpus']
        wordcount = d['words']
        year = parse_year(d['year'])
        samplecode = f'{corpus}-{sample}'
        samplemap[samplecode] = dict(
            id=samplecode,
            words=wordcount,
            metadata=dict(
                corpus=d['corpus'],
                genre=d['genre'],
                speech=CLASS_SPEECH[d['genre']],
                legal=CLASS_LEGAL[d['genre']],
            ),
            tokens=[],
            year=year,
        )

    for token in input_data['tokens']:
        corpus, sample, dataset, token, before, word, after = token
        samplecode = f'{corpus}-{sample}'
        s = samplemap[samplecode]
        s['tokens'].append(dict(
            lemma=token,
            metadata=dict(variant=dataset),
        ))

    samples = sorted(samplemap.values(), key=lambda x: x['id'])
    data = dict(samples=samples)

    with open(destfile, 'w') as f:
        json.dump(data, f, indent=1)


main()

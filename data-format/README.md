# types3 data format

Assume you have a CSV file that describes your samples, like this (see [example_samples.md](example_samples.md) for an example):

```csv
id,words,year,gender
7009,107744,1848,M
2997,55962,1917,F
7,53750,1913,F
````

And another CSV file that describes your tokens, like this (see [example_tokens.md](example_tokens.md) for an example):

```csv
id,lemma,variant
7009,seek,be-going-to-verb
7009,write,be-going-to-verb
7009,write,gonna
2997,last,be-going-to-verb
2997,fall,be-going-to-verb
2997,write,be-going-to-verb
7,paint,gonna
7,seek,be-going-to-verb
```

Note that in the second file the ***id*** column refers to the identifier of the sample.

Columns ***id***, ***words***, ***year***, and ***lemma*** are required; any other columns are considered to be additional metadata and classifications.

Now if your samples are listed in file `samples.csv` and your tokens are listed in file `tokens.csv`, you can use the following command to convert it into a JSON file `data.json` that is suitable for types3:

```bash
./types3-convert samples.csv tokens.csv data.json
```

The end result will be a JSON file with the same information, but structured differently: it contains a list of samples, and for each sample it contains a list of tokens. See [example.json](example.json) for an example.

Then you can open `data.json` in the user interface and start to explore it:

```bash
./types3-ui data.json
```
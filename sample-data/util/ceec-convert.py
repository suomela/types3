import json
import sqlite3
import sys


def main():
    srcfile, destfile = sys.argv[1:]
    con = sqlite3.connect(srcfile)
    cur = con.cursor()

    corpuscode = "ceec-1680-1800"

    samplemap = {}

    for samplecode, wordcount in cur.execute(
        """
        SELECT samplecode, wordcount
        FROM sample
        WHERE corpuscode = ?
    """,
        [corpuscode],
    ):
        samplemap[samplecode] = dict(
            id=samplecode,
            words=wordcount,
            metadata={},
            tokens=[],
            year=None,
        )

    for samplecode, datasetcode, tokencode, tokencount in cur.execute(
        """
        SELECT samplecode, datasetcode, tokencode, tokencount
        FROM token
        WHERE corpuscode = ?
        ORDER BY tokencode
    """,
        [corpuscode],
    ):
        for _ in range(tokencount):
            samplemap[samplecode]["tokens"].append(
                dict(
                    lemma=tokencode,
                    metadata=dict(
                        variant=datasetcode,
                    ),
                )
            )

    for samplecode, groupcode, collectioncode in cur.execute(
        """
        SELECT samplecode, groupcode, collectioncode
        FROM sample_collection
        JOIN collection USING (corpuscode, collectioncode)
        WHERE corpuscode = ?
    """,
        [corpuscode],
    ):
        if groupcode == "period":
            samplemap[samplecode]["year"] = int(collectioncode)
        elif groupcode == "gender":
            samplemap[samplecode]["metadata"][groupcode] = {"F": "female", "M": "male"}[
                collectioncode
            ]
        elif groupcode == "socmob":
            samplemap[samplecode]["metadata"][groupcode] = collectioncode
        else:
            pass

    samples = sorted(samplemap.values(), key=lambda x: x["id"])
    data = dict(samples=samples)

    with open(destfile, "w") as f:
        json.dump(data, f, indent=1)


main()

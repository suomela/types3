#!/bin/bash

set -e

cd integration-test
rm -rf plot text
mkdir -p plot text

for a in calc/ceec-types-vs-tokens.json; do
    x="${a#calc/}"
    x="${x%.json}"
    b="plot/$x.pdf"
    d="text/$x.txt"
    ../types3-plot --legend 'lower right' "$a" "$b"
    ../types3-plot "$a" "$d"
done

for a in calc/*.json; do
    x="${a#calc/}"
    x="${x%.json}"
    b="plot/$x.pdf"
    c="plot/$x-wide.pdf"
    d="text/$x.txt"
    ../types3-plot --legend 'lower right' "$a" "$b" &
    ../types3-plot --legend 'lower right' --wide "$a" "$c" &
    ../types3-plot "$a" "$d" &
done
wait

for dir in plot plot-expected; do
    cd "$dir"
    for a in *.pdf; do
        b="${a%.pdf}.png"
        magick -quiet -density 100 "$a" "$b" &
    done
    wait

    for a in *.pdf; do
        b="${a%.pdf}.png"
        c="${a%.pdf}.tmp"
        magick identify -quiet -format "%# %wx%h $a\n" "$b" > "$c" &
    done
    wait

    cat *.tmp > hash.txt
    rm *.png *.tmp
    cd ..
done

diff plot-expected/hash.txt plot/hash.txt
rm -f plot-expected/hash.txt plot/hash.txt

cd text
for a in *.txt; do
    diff ../text-expected/$a $a
done
echo "SUCCESS: all images and text files agree."

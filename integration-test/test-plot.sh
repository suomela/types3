#!/bin/bash

set -e

cd integration-test
rm -rf plot
mkdir -p plot

for a in calc/*.json; do
    x="${a#calc/}"
    x="${x%.json}"
    b="plot/$x.pdf"
    c="plot/$x-wide.pdf"
    ../types3-plot --legend 'lower right' "$a" "$b" &
    ../types3-plot --legend 'lower right' --wide "$a" "$c" &
done
wait

for dir in plot plot-expected; do
    cd "$dir"
    for a in *.pdf; do
        b="${a%.pdf}.png"
        convert -density 100 "$a" "$b" &
    done
    wait

    for a in *.pdf; do
        b="${a%.pdf}.png"
        c="${a%.pdf}.tmp"
        identify -quiet -format "%# %wx%h $a\n" "$b" > "$c" &
    done
    wait

    cat *.tmp > hash.txt
    rm *.png *.tmp
    cd ..
done

diff plot-expected/hash.txt plot/hash.txt
rm -f plot-expected/hash.txt plot/hash.txt
echo "SUCCESS: all images agree."

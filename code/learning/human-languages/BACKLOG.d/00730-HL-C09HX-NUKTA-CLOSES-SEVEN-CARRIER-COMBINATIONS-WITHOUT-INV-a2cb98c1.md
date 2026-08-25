## HL-C09HX — Nukta closes seven carrier combinations without inventing letters

Unicode 17 defines U+093C DEVANAGARI SIGN NUKTA as a true diacritic: a
subscript dot that extends the consonant inventory. The shared script data now
models the carrier-plus-mark composition directly for the seven combinations in
the current corpus — **क़, ख़, ग़, ज़, ड़, ढ़,** and **फ़** — while explicitly
distinguishing Unicode character order from a claimed universal handwriting
order.

This one combining mark reduces shared Hindi, Marathi, and Sanskrit closure
debt from **106 to 96 affected realizations** and from **13 to 12 missing
glyphs**. The reranked corpus now puts retroflex **ट** first at 18 affected
realizations, followed by **छ** at 15 and **ड** at 13; source and font-fit **ट**
next.
The largest production `script-data` batch containing Devanagari remains below
the 250 kB authored-data target at **176.99 kB**.


## HL-C09HN — Arabic لا is a ligature, not a thirty-second letter

The closing Arabic audit models obligatory **لا** with its source-backed ordinary
two-stroke order: descend from the upper right, lift, then cross from the upper
left and finish along the baseline. The editable identity remains the two Unicode
letters **ل + ا**; U+FEFB **ﻻ** is retained only as the joined Noto Naskh outline
used by the font-fit gate.

Arabic's sourced shape audit is now complete at **31 learner rows**, **29
canonical base/standalone rows**, one seated-Hamza composition family, and one
obligatory ligature. Its separate corpus-closure flag remains false: exercising
that gate reveals romanized teaching strings plus composed **أ, إ, and آ** are
still treated as missing base characters. The next highest-priority item is to
make closure validation composition-aware and distinguish transliteration from
Arabic spelling before asserting script completeness. The production
Arabic-bearing `script-data` batch remains below the 250 kB authored-data target
at **51.92 kB**.


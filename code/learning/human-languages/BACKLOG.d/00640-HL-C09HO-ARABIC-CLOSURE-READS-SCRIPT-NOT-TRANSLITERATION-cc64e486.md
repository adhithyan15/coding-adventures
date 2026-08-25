## HL-C09HO — Arabic closure reads script, not transliteration

The completion gate now filters a headword to the Unicode script owned by its
inventory and compares both the headword and inventory in canonical decomposed
form. Legacy Latin **as-salāmu** teaching text therefore no longer becomes a
list of fake missing Arabic glyphs, while precomposed **أ, إ, and آ** resolve to
Alif plus Hamza Above, Hamza Below, or the newly inventoried Maddah Above.

Arabic can now assert corpus closure without duplicating composed characters as
base letters. The next highest-value completion audit is **Cyrillic**: its 33
sourced rows already match the Russian alphabet, but the still-false completion
claim needs the same real-corpus gate before it changes. The Arabic-bearing
`script-data` batch remains below the 250 kB authored-data target at **52.23
kB**.


---
category: Testing & coverage
---

# An exact-form headword check misses plural and gender variants of taught words

**What went wrong.** The Spanish, Italian and Portuguese B1 tranches screened every new headword against the track's taught headwords, normalized and with articles stripped, and against the forward-reference matcher. Both comparisons are exact-form. Portuguese *o lábio* passed even though PT-C120 teaches *os lábios*, as did *os gémeos* against *o gémeo* and *a luva* against *as luvas*. Italian *i gemelli* passed against *il gemello*, and *l'infermiere* against *l'infermiera*. Each would have counted as a new headword on the level gate for a word the learner already had.

**The fix.** A reviewer caught the Portuguese three by reading. A variant check, plural endings and gender pairs per language compared against the taught list, then found the two Italian cases, which were replaced in the open PR. Most of its hits are different words that merely look alike (*tesouro*/*tesoura*, *partito*/*partita*, *caso*/*casa*), so every hit needs a human reading. Spanish B1 had none.

**Do differently.** Before drafting, and again before generating, run a plural/gender variant check of new headwords against the taught list alongside the exact-form screen, and read every hit by hand. A level-gate headword count is only honest if a plural of a taught noun is not a new word.

---
category: Testing & coverage
---

# A nineteenth-century dictionary's cognate comparison is not a cognate

Monier-Williams (1899) ends many entries with a "cf." note comparing the
Sanskrit word with Greek, Latin, Gothic or Irish words. The Sanskrit
etymology pass treated those notes as cognate evidence, and the reviews
found the largest single class of error there: look-alikes that a modern
source assigns to a different root. *bala* "strength" was set beside
Latin *valēre*, *agra* "tip" beside Greek *akron*, *anya* "other" beside
Latin *alius*, and *śuṣyati* "dries" beside Latin *siccus*. EtymDB 2.1
puts each pair under different roots. The opposite also happened: *śaṅkha*
and *aṅgāra* were called "origin uncertain" although MW itself compares
*concha* and Lithuanian *anglis*, and EtymDB has both as inherited.

The fix was to check every MW comparison against EtymDB before printing
it, keep only the cousins both agree on (or that EtymDB alone gives,
cited to EtymDB), and drop or attribute the rest ("MW compares ...").

**What to do:** never print a "cousin of English X" from an old
dictionary's comparison alone. Check it against a modern etymological
source (EtymDB for this repo). MW's cognate notes also sit under a
sister headword (*svad*'s under *svādu*), so read the exact record
before saying "MW compares".

## HL-C385 — devanagari.json carries 44 letters and ङ is not one of them

`data/scripts/devanagari.json` lists **44** letters. It includes **ळ**, which is
a Marathi letter Hindi does not use, and it **omits ङ**, the ka-varga nasal.

Found while checking whether `HI-A1-SCR-15` could close: a script that walks the
five vargas expects twenty-five stops, and the ka-varga came back with four.

### Why it has never mattered

**ङ appears zero times in the Hindi corpus.** In modern Hindi its sound is
written with the anusvāra — अंक, रंग — so a reader meets the *sound* constantly
and the *letter* never. Nothing is currently owed to a learner because of this.

### Why it should still be fixed

1. **The file is the source of truth for three tracks**, not one. Hindi,
   Marathi and Sanskrit all read it, and Sanskrit does use ङ standalone.
2. **Any audit that walks the series silently gets a wrong answer.** Mine did,
   and only caught it because the varga came up one short against a count I
   already expected. A check that had trusted the file would have reported the
   ka-varga complete.
3. **ळ being present while ङ is absent** suggests the list grew by what each
   track happened to need rather than from the series, which is worth
   confirming before anything else is built on it.

### What to do

Add ङ with the same shape as every other entry — `components`, `strokeOrder`,
`strokeOrderNote`, `penLifts` and a `strokeOrderSource` citation — or record
explicitly in the file why it is absent. **Do not add a Hindi lesson for it**:
no Hindi word in the corpus uses it, and drawing a letter no reader will meet is
the opposite of the gentle ramp. If Sanskrit needs it, the lesson belongs there.

Worth checking the other script data files for the same kind of gap before
trusting any of them to be a complete series.

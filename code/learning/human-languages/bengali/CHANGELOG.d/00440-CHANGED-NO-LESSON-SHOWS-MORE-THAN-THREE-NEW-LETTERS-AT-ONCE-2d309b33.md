## Changed — no lesson shows more than three new letters at once

The script-ramp report (`ramp.script`, policy `maxNewGlyphsPerLesson: 3`)
counts every target-script shape the first time a lesson body shows it.
It listed three Bengali lessons; it now lists none. Script closure stays at 0.

| lesson | new shapes before | after |
|---|---|---|
| BN-C01-nomoshkar | 7 নমস্কার | 2 ন ম |
| BN-C01-dhonnobad | 4 ধ য ব দ | 3 ধ ্ য |
| BN-C02-alaap | 6 ল প ে ভ ো গ | 2 ল প |

- **BN-C01-nomoshkar** keeps the greeting meaning-first under a romanized
  heading. Its letters block meets ন *nô* and ম *mô* (Bengali's inherent *ô*)
  and traces ন once; it says plainly that the whole written word waits for
  chapter 2, whose runway already ends on BN-W01-nomoshkar-read.
- **BN-C01-dhonnobad** teaches ধ and the conjunct ন্য with its hasanta. The
  *b*, the *d*, the long *ā* and the whole spelling come later; the *v → b*
  point is kept in romanization.
- **BN-C02-alaap** shows only its key word, আলাপ; *kore bhālo lāglo* stays
  spoken, so its four other shapes arrive later one or two at a time.
- BN-C01-ashi no longer calls স "again": it is the *s → sh* shift heard in
  *nômoshkar*.
- The deferred shapes now first appear in the lessons that use or teach them
  (BN-C01-hyan-na and BN-W01-na-trace 3 and 2, BN-W01-ka, BN-C03-kemon,
  BN-C03-bhalo and BN-W03-ga 1 each). Regenerated book chapters 1 and 3,
  narration, modality owners and hash shards.

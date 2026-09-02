## HL-C194 — Tamil's remaining closure debt is ORDERING, and the letter உ names it

The Tamil headword sweep is done: all 21 lessons whose native-script headword
carried no `romanization` now declare one, transcribed from that lesson's own
prose. `headwordsWithoutRomanization` is **0**, and closure violations fell
**29 → 21** on their own as the exposure rule started applying.

What is left is a different problem, and the earlier entry did not separate the
two. Tamil's glyph inventory is CLOSED — `neverTaughtGlyphs: 0`, 51 shown and
51 taught — so none of the remaining 21 violations is a missing lesson. Every
one is a glyph taught **after** the lesson that asks the reader to decode it,
and closure is measured in **reading order**.

Nineteen of the 21 sit in chapters 1–6, where the track opens on Tamil prose
before the script strand has started: the first script lesson is
`TA-W00-va-guided-copy` in chapter 1, and the next is `TA-W01-curves-va-ka` in
chapter 4. Those nineteen are a body-prose problem and want a decision about
how much script chapters 1–6 should show, not a resequencing.

**The two that are purely an ordering bug, and are cheap:**

```
 75  TA-C09-mannikkavum  ch9   [1]  உ
139  TA-C32-saappidu     ch32  [1]  உ
```

Both violate on **one glyph**, and it is the same one. The vowel **sign** ு is
taught in chapter 4 (`TA-W01-curves-va-ka`); the vowel **letter** உ is not
taught until chapter 36 (`TA-W17-read-unavu`) — thirty-two chapters after its
own sign, and long after the ordinary words that use it. Every other letter in
the `TA-S1xx` series is taught in the teens; உ is the outlier, and it has no
`TA-S`-series lesson of its own at all.

Teaching உ in a short `TA-S`-series lesson before chapter 9 would retire both
violations outright, and would also shave உ off `TA-C01-illai`,
`TA-C02-magizhcci`, `TA-C02-practice` and `TA-C05-velai-sey`, which violate on
other glyphs too and so would not flip. Expect **21 → 19** from the fix, plus
four lessons carrying less debt each.

Two checks the last tranche learned the hard way and this one must repeat:

- **Check the forward-reference gate before choosing a headword.** `நில்` was
  rejected because `TA-C20-vaanilai` already glosses it in prose.
- **Check the glyph inventory before writing any new Tamil.** `ஓடு` was
  rejected because ஓ appears nowhere in the corpus; writing it would take
  `neverTaughtGlyphs` from 0 back to 1 and reopen a closure the track has
  finished. The inventory is the 51 glyphs in
  `core/gentle-ramp-snapshots/tamil.d/`, and it should stay closed.

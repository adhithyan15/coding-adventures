## HL-C316 — an empty joining column is usually not a script debt; check the glyphs before scheduling letters

Gujarati's A1 `Jodaan` column measured **0 of 11** and is now 10. The number is
the smaller half of the finding.

### Measure the glyphs before you plan the tranche

The obvious reading of an empty joining column in a script track is that the
letters are not there yet. Gujarati's script columns encouraged it: closure over
the corpus was perfect (43 of 43) while the ALPHABET was about 21 of 35 with
**0 of 10 digits**, so "we cannot write the joining words yet" was the natural
hypothesis.

It was wrong, and one command said so before any lesson was designed — take the
union of Gujarati codepoints in every `type: writing` / `delivery: script`
lesson body, then check each candidate word against it:

    ane, athava, pan, ke, kemke, maate, tethi, jo, jyaare, je   ALL SPELLABLE
    maaf                                                        needs ફ
    kshama                                                      needs ષ

**Ten of the eleven devices needed no new sign at all.** The tranche spent
exactly one letter in seven chapters, and spent it on the apology rather than on
the joining. Had the glyph budget been assumed instead of measured, the plan
would have opened with a run of script lessons the material did not need.

The general rule: **an absent word and an unwritable word look identical in a
coverage report.** They are different debts with different fixes, and the union
of taught glyphs separates them in one pass.

Where it did bite, it bit precisely: `teo` (`PRON-04`) needs the independent
vowel **ઓ**, which no lesson teaches. That point is left uncovered with the
reason in the file, so it now names its own fix as a script lesson instead of
reading as missing vocabulary.

### The corollary for the inventory's own notes

Gujarati's inventory said `jo` had "16 apparent hits, all inside `aavjo` or the
stem of `jovu`". That substring warning is not only a measurement caveat — it is
**lesson content**. A reader meeting `જો` on a page has the same problem the
grep had, so the writing lesson says outright: check that it stands alone, at
the front of its clause. The inventory's methodology notes are teachable
material, not just provenance.

### Two things that generalise to any joining tranche

- **The chapter-boundary retrieval works.** Every chapter opens by naming the
  two preceding items. Across 38 new atoms R1 misses rose by **4**. The debt
  that does accumulate is at R2-R4, and it is structural: the last chapters in a
  book have no later lessons to be retrieved from.
- **Decompose the reinforcement rise or it is meaningless.** +81 total split into
  +41 own atoms, +47 pre-existing windows that did not exist until the track grew
  (R4 is distance 80-250; a 228-lesson track gives an atom at position 151 no R4
  to miss, a 263-lesson one does), -4 closed deliberately by a cold read placed
  inside R4, and -3 pre-existing misses closed by the chapter-opening retrievals.
  Measured against the same corpus with the new chapters filtered out, not
  differenced from a remembered number.

## Unreleased — chapters 11 and 12 are generated; Marathi owns no hand-written chapter (HL-C287)

`ch11-farewells.tex` and `ch12-first-verbs.tex` now generate from their lessons.
With chapters 9 and 10 already done, **Marathi holds no hand-written chapter and
no schema-v1 lesson at all** — `handwritten_parity.py --check marathi` answers
"already retired, nothing handwritten remains" and exits 0.

- **Eleven schema-v1 lessons migrated**, 21 atoms between them, none over the
  per-lesson budget of three. Per-character non-ASCII census across all eleven:
  **zero** Devanagari or IAST characters changed.
- **`MR-C04-kalji-ghya` and `MR-C04-practice` were in no path segment**; both
  append to `MR-PATH-010` at their sequence positions under a new
  `MR-EXT-040-FAREWELL-CARE-AND-CONSOLIDATION`. As in chapter 10, appending to
  an existing segment avoids forcing the canonical re-shard to renumber every
  following path file.
- **`ळ` is taught, and chapter 11 is where a lesson finally says so.**
  `MR-C04-kalji-ghya` teaches the retroflex *ḷa* in its letters block; it now
  requires and practises `MR-SCRIPT-LLA-01`, the atom `MR-W03-lla` has owned
  since chapter 3. `scriptClosureViolations` stays at **0**.
- **`MR-C05-practice` required three atoms its prerequisite chain never
  reached.** The chain ran `kaam-karne → rahne → bolne` and never touched
  `MR-C05-mi-marathi-bolto`, whose phrase, etymon and no-copula atoms the recap
  tests. The lesson is now a prerequisite, which is what `reviews_of` had been
  claiming all along.
- **A second atom minted alongside an existing one.** `MR-C07-jane` in chapter
  14 owns `MR-GRAMMAR-PRESENT-GENDER`, for the gendered present that chapter 12
  teaches *first*. Chapter 12 takes `MR-GRAMMAR-PRESENT-GENDERED-ENDING`,
  the same call as `MR-LEX-AAHE-COPULA` in chapter 9 and for the same reason:
  re-pointing a generated chapter has the larger blast radius.
- **Subject–verb agreement fixed** in `MR-C04-practice`: "What **do** every
  Marathi goodbye have in common?" It is the chapter's closing question, and
  only the rendered page made it obvious.

Counters, re-measured: lesson-content budget **194 → 205**; `atomsTaught`
**197 → 218**; `atomMeasurementBlindLessons` **13 → 2**;
`reinforcementWindowMisses` **293 → 353**, the same legitimate rise as before.
`forwardReferences` stays at **4** and `scriptClosureViolations` at **0**, both
measured before and after rather than assumed. Idioms, senses and culture
claims never moved off 5 / 4 / 7 across the whole 179 → 205 migration.

Verified: 124 test files / 1730 passing; all eleven `check:*` gates;
language-ladder 39 files / 442 passing; the whole Marathi book compiles under
XeLaTeX with zero overfull boxes, and both chapters' pages were read as rendered
PDF — the retroflex ळ, the conjuncts in पुण्यात and करणे, the √kṛ radical and
the three-column tables all set correctly.


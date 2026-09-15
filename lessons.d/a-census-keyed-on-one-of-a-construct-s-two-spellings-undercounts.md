---
category: Testing & coverage
---

# A census keyed on one of a construct's two spellings undercounts silently, and the number gets published

ADJ `table` rows may carry their provenance block on its own lines or **inline**:

```
row (venus, 2) { source "Venus is the second planet from the Sun, and the sixth largest planet." }
```

Every census I ran in one session detected converted tables with `\n        source "` —
eight spaces. That needle reads **zero** spans from an inline block. `biology/kingdoms.adj`
(22 rows) and `metrology/si-base-units.adj` (7 rows) write every row that way and were
**published on the tracking issue as unconverted work**; `astronomy/planets.adj` writes
seven of eight that way and was published with one row span instead of eight. Anyone
following that queue would have opened an already-converted table.

The same needle then understated a second ledger by 22: a bare-ending-span triage listed
its top contributor nowhere, because that contributor was `kingdoms.adj`.

**It was not found by re-reading the script.** I went to add a structural test to
`planets.adj`, opened the file, and saw eight row blocks where my census reported one span.
Nothing about the census looked wrong from the outside — it produced plausible, stable,
confidently-wrong numbers for hours.

**Rule:** before counting instances of a construct, enumerate the forms the grammar
actually admits and probe the extractor on **one file of each form** as a control. The
corrected parser here asserts it finds 8 spans in an all-inline table and 11 in a
multi-line one before any result is believed; a test built on it asserts its parse found
the expected number of rows **before** asserting anything about them — which immediately
caught a second bug, a row scan splitting on `)` that swallowed a whole table because
`row (length, meter, "m") { source "Length - meter (m)" }` contains one inside the span.
A guard with a too-narrow needle does not fail; it goes silently vacuous over an empty
list and reports that nothing is wrong.

**Corollary on publishing:** a census posted to an issue is acted on by other people. Date
it, state the scope searched and the needle used, and when it turns out wrong post the
correction with the same prominence — including which entries were wrong in which
direction. Two entries here needed removing from a queue, not just a total adjusting.

**And a repeat I should not have made.** In the same session I wrote an absence-gate —
"this filename must never appear in the file" — which failed on the header comment
explaining why the filename is banned. That is
[[an-absence-gate-that-scans-the-whole-file-fails-on-its-own]], recorded the day before by
another lane, with three instances of the identical shape. `CLAUDE.md` says to read
`lessons.d/` before implementation work; I did not, and paid the same toll a fourth time.
Scan code lines, or in this case locator *values*, not the whole file.

**The title undercounts: there is a third shape, and the corrected census missed it.**
Measured on `origin/main` `a146ad8f29` (2026-09-14), a row block can also be written over
several lines and hold **`cites` with no `source` at all**. That is the honest form for a row no
span states outright, and `earth-science/water-cycle.adj` (5 blocks) and
`physics/energy-form-family.adj` (8 blocks) ship it. The census I built to fix the inline blind
spot counted *`source` spans inside a block*. So it read both tables as unconverted, and I
published **27 / 288** and **28 / 287** converted/remaining on the tracking issue while the tree
held **30 / 285**. One of those comments said in the same paragraph that `energy-form-family`
had shipped "with per-row `cites`, not `source`". Two header sweeps from the same session asked
"`source` **or** `cites`" and counted 30, and one of them put "30 converted tables" into the
CHANGELOG. I never compared the two numbers.

**And the fix did not reach the copies.** The eight-space needle had been replaced in the
census I was publishing from, and still sat in three sibling scripts: a blocker triage, a
remaining-tables census and a converted-tables selector. It surfaced when the triage printed
`kingdoms.adj` and `si-base-units.adj` among its largest *unconverted* candidates, after both
had been corrected in public.

**Rule, sharpened:**
- **Test for the construct, not for what it holds or how it is laid out.** "Does any `row (…)`
  line open a brace?" is right for all three shapes. A test for a keyword inside the block is
  wrong for every block that uses a different keyword.
- **When one instrument is corrected, grep for every copy of its test before republishing any
  number from any of them.** Run each copy on one control file per known shape.
- **When two instruments count the same thing differently, name the differing items before
  publishing either number.** 28 against 30 was two named files, and finding them took one run.

Related: [[a-source-code-audit-that-searches-for-a-rust-literal-in-one]] — the same
too-narrow-needle family on the Rust side, where the guards used a raw string literal and
the audit searched for the escaped one.

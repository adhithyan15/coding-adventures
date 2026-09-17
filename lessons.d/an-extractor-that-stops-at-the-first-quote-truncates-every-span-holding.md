---
category: Testing & coverage
---

# An extractor that stops at the first quote truncates every span holding an escaped quote and manufactures false positives

Reproducing #15185's census over `adj-facts-stdlib`, the extractor took a span value as everything
up to the first `"` on the line:

```python
rest = s[len(kw):]
if Q in rest:
    out.append(rest[: rest.index(Q)])
```

A span containing an escaped quote ends early there. The fragment then terminates on a **backslash**,
which is not in anyone's list of sentence terminators, so the predicate "does this span end without
terminal punctuation?" answers **yes** for every one of them.

**Measured at `origin/main` `1660847a39`.** The corpus is unchanged across `34e305c158…1660847a39` —
none of the intervening commits (#15392, #15394, #15395) touches any file under `adj-facts-stdlib` —
so the figure holds at this branch's base. Re-running it at each of those revs was one measurement
over identical bytes, repeated; it is not corroboration, and that is the shape that let the error
below survive. 17 span lines hold an escaped quote, and all 17 of the bare spans ending in a
backslash were those same 17. Honouring escapes moves the count from
**100 to 84**: **16** of the 17 stop being bare, and one does not. `chemistry/elements.adj:65` ends
`"Hydrogen",` — a comma — so it is bare on its own merits under *both* extractors, a true positive the
blind extractor happened to flag for the wrong reason. Its near-twin `element-symbols.adj:55` ends
`"Oxygen"` and does drop out, which is what makes the pair easy to conflate.

**That correction was itself the defect this shard describes.** The 83 was `100 − 17`, computed from
the artifact count rather than measured — while an escape-aware run in the same session had already
printed **84** in a table four paragraphs from the prose that said 83. Two views of one corpus
disagreed inside my own output and I shipped the inferred one.

Escape-aware, the same predicate at the issue's own base `b3ab5a7963` returns **81** — the issue's
published figure, exactly.

**The false positives clustered, which is what made them convincing.** `anatomy/muscle-groups.adj`
carried 4 of the 17 — enough to rank third in a contributor table. The issue's own table omits that
file, and I had a comment drafted reporting the omission as a defect **in the issue**. It was a
defect in me: the issue's figure is what an escape-aware extractor reproduces exactly, and mine was
not one. Under the corrected extractor the file drops out entirely, as the issue always had it.

This wrong instrument did not produce noise. It produced a specific file, with a plausible count, in
the right corpus, ranked where a real contributor would rank — and an accusation aimed at the one
artefact that was correct.

**What the control had to be.** Not "run it and see if the number looks right": 100 looked as
reasonable as 84. The arms have to be a span holding an escaped quote that **is** terminated and one
that **is not**, and the extractor must tell them apart. The escape-blind version calls both bare, so
the arms do not differ and the instrument is refused before any census prints:

```python
pos = 'source "He said \\"stop\\" loudly."'   # terminated
neg = 'source "He said \\"stop\\" loudly"'    # bare
# escape-blind: both read bare -> arms do not differ -> refuse to report
```

**Do not read the denominator as vindicated.** The corrected run reproduces `unterminated` exactly
and still does **not** reproduce `examined`: 671 against the issue's 695 at the same rev. The escape
fix cannot explain that gap — a truncated span was still counted once, so honouring escapes changes
what counts as bare without changing what counts at all. Fixing the defect you found is not evidence
about the defect you did not.

This is an instance of a rule this corpus already recorded earlier the same day, about this same
census:
[[writing-a-lesson-down-does-not-stop-you-rebuilding-the-bug-assert]] — a prose rule cannot reject a
misreading of itself, so state it as a check with a control on a file whose answer is known
independently. The advice I skipped is its: **commit the instrument, control and all.**
All four scripts behind this measurement live in a scratch directory, so the next rebuild starts from
prose again, which is exactly how this bug arrived.

Related: [[a-census-keyed-on-one-of-a-construct-s-two-spellings-undercounts]] — the same corpus and
the same family of too-narrow needle, there missing a construct's second spelling, here missing its
escaping.

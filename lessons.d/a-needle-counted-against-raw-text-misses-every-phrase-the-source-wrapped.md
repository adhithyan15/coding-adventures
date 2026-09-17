---
category: Testing & coverage
---

# A needle counted against raw text misses every phrase the source wrapped across a line break

Verifying that a merge had landed, I built a two-arm content test: two positive needles that had to
appear in the merged file, and one negative control that had to be absent. Then, to check the
negative control was discriminating rather than merely empty, I looked for a **near-miss** phrase the
file genuinely contains.

The near-miss read **0**. So did a second one. The file plainly contained both.

**They wrapped.** The shard is hard-wrapped at about 100 columns, and `str.count` over the raw text
sees a newline in the middle of the phrase. The same needles over whitespace-normalised text:

    needle                      raw   flattened
    corpus that grew              0          1
    a corpus that grew by two     0          1
    grew by two                   1          1
    the corpus grew               0          0

Only the flattened column is admissible. The raw zeros are line breaks, not absences — and note the
last row, which is **genuinely** absent under both. A raw count cannot tell those two cases apart,
which is exactly what makes it dangerous: it returns the answer you were hoping for.

**The damage was to the control, not to the result.** The merge itself was proven by comparing the
two sides for exact equality — they matched, so no wrong conclusion shipped. What failed was the
check on the check: my near-miss control could not have caught a mislabelled negative needle, because
it matched nothing either. **A control that reads zero for the same reason the thing it guards reads
zero is not a control.** Both arms were dark and the instrument reported success.

*A second unit error, found while writing this up and left in rather than quietly fixed.* That merge
check reported "3528 bytes on both sides". It compared `len()` of two **decoded strings**, which
counts characters; the file's five em-dashes and one ellipsis are three bytes each in UTF-8 — those
six characters account for all twelve extra bytes, residual zero — and the blob is **3540 bytes**,
identical at every rev I checked, so nothing had drifted. The equality was real and
the conclusion held. The unit was not what I called it. Saying "bytes" when you measured characters
is the same species of error as counting raw when the source wraps: the number is defensible and the
sentence around it is not.

**The tell was available and I nearly walked past it.** The script printed a sentence dump beside the
counts, and the dump showed the phrase intact — because the dump normalised whitespace before
printing while the counter did not. Two views of one file disagreed inside a single run's output.
That disagreement is the finding; treating the reassuring view as the real one is how it gets lost.

**Operationally:** flatten before counting, in every arm, including the controls.

    flat = " ".join(text.split())

Do it even when the needle is short, because what wraps is the *source*, not the needle — a
four-word phrase wraps whenever it happens to straddle the margin.

**But flattening is not universally right, and this shard is its own counter-example.** In reflowed
prose a raw miss is the wrong answer: no reader of the rendered text perceives the break. Measured on
this file, 29 prose needles spanning a line break read **0 raw, 1 flattened**, and there the
flattened count is the true one. Inside a **preformatted block** the reverse holds — flattening joins
lines the reader sees as separate and invents matches across them. Four needles spanning row
boundaries of the table above also read **0 raw, 1 flattened**, and there the flattened 1 is false.

**The two signatures are identical.** Both produce raw=0 and flattened=1, so the disagreement itself
tells you nothing about which count is wrong; only knowing whether the text is reflowed or
preformatted can. Flatten prose; count preformatted blocks line by line.

Related: [[a-filter-must-be-proven-against-a-known-positive-input-never-only]] — the neighbouring
failure, and the mirror image of this one. There the **probe** failed to reproduce a wrap (the test
string put the whole phrase on line two, so nothing was ever wrapped); here the probe was fine and
the **counting method** could not see the wrap that was really there. Same subject, opposite end:
one is about building the control, the other about how you measure with it.

Related: [[an-extractor-that-stops-at-the-first-quote-truncates-every-span-holding]] — the same day,
the same corpus, and the same shape of defect: an instrument that silently returns a plausible number
because it cannot represent what the text actually contains.

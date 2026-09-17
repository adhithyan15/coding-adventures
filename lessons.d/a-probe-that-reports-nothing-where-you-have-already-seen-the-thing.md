---
category: Testing & coverage
---

# A probe that reports nothing where you have already seen the thing is broken not informative

While checking a security-review finding I wrote a probe to count the HTML tags
inside three quoted sentences on a cited page. Its per-span block, in full, for
the sentence this shard is about:

```
  span at .adj line 148:
    inline-stripped count      : x1
    naive all-tags->space count: x0
    tags inside the sentence   : 0
    anchor texts               : []
```

Three of those four lines read the same for all three spans — including the
first: `inline-stripped count : x1` says the span *was* located in the stripped
view, every time. The one that varies is the naive count, cardiac's `x0` against
`x1` for the other two, and it was a second warning I did not take.

A first draft of this shard quoted only the last two lines, unmarked, showing
the reader less evidence than the instrument had emitted. A later draft said
"the first two differ", which is true of one of them. Both errors are the same
shape as the one the shard is about, committed against the shard's own opening
quotation.

Minutes earlier I had read the raw bytes of one of those sentences myself and
seen its markup:

```
one nucleus per <a class='glossaryTerm' href='https://api.seer.cancer.gov/rest/
glossary/latest/id/550ed183e4b0c48f31dad283'>cell</a>, striations, and <a class=
'glossaryTerm' href='...'>intercalated disks</a>.
```

(Every line break in that block is display wrapping — the page has it on one
line, and one of the breaks falls inside an attribute rather than the URL.
Joining the lines with no separator reproduces the page bytes exactly, except
the second `href`, which is elided. Marked, because an unmarked abridgement is
the failure this shard is about, and a first draft shipped one here — eliding
both URLs without saying so, then disclosing only a URL wrap when two breaks
were introduced.)

Two anchors, plainly there. The probe said zero. I very nearly replied "the
reviewer is wrong, there are no tags" — the same move that had already cost me
once that week, when I disputed a correct finding with a substring count.

### The rule

**When an instrument's output contradicts something you have observed directly,
the instrument is wrong until proven otherwise.** Not surprising — wrong. Stop
and repair it before reading another number off it, because every other number
it produced came through the same code path and none of them has been shown to
work.

This is narrower than "a zero is a question with several answers", and the
narrowness is the point: normally a zero opens a diagnosis, and you have to work
out which of several causes produced it. Here one cause is already excluded by
direct observation, so the diagnosis is closed before it begins. Nothing the
instrument says next is admissible until it is fixed.

### The bug, which is worth knowing on its own

The probe located each sentence's region by searching the **raw** page for the
span's **first four words and its last four**, then slicing between them. When
markup splits either end, that search finds nothing and the region comes back
empty. A tag scan over an empty region returns zero — confidently, in the same
format as a real answer. The instrument had no way to distinguish *"I did not
find the region"* from *"the region has no tags"*.

Replaying it per span shows the failures are not uniform, and the differences
matter:

```
skeletal  head found, tail found     -> region  94 chars, 0 tags
cardiac   head found, tail NOT found -> region   0 chars
envelope  head NOT found, tail NOT found -> region 0 chars
```

The motivating case failed on the **tail**, not the first four words: the page
writes `striations, and <a ...>intercalated disks</a>.`, so **both** of the
anchor's tags fall inside the last four words — the opening one between "and"
and "intercalated", the closing one between "disks" and the period — and the
needle `striations, and intercalated disks.` occurs **zero** times in the raw
page. An earlier draft blamed the closing tag alone, which its own quoted
snippet contradicts.

And skeletal's zero was **true** — a real 94-character region with genuinely no
markup. Only two of the three zeros were artifacts. A first draft of this
paragraph said the region came back empty for all three and blamed the first
four words in every case; both claims were wrong, and a review caught them.

Rewritten: build the inline-stripped text alongside an index mapping every
stripped character back to its raw offset; find the span in the stripped text,
where it was **measured** to occur exactly once — x1 for all three spans, though
the instrument only guards against zero occurrences and would silently take the
first of several; map both ends back; read the raw slice
between them. The markup is then read out of bytes guaranteed to be the right
region. Real answers, each with the region it was measured over, because the
corollary below demands exactly that: **0 tags in the skeletal sentence's 94
characters, 4 in the cardiac's 318** (two anchor pairs, around "cell" and
"intercalated disks") **and 2 in the envelope's 247** (one pair, around
"tissue") — against the broken version's 0, 0, 0, of which two were artifacts of
an empty region and one was correct, because that region was real.

A first draft of that sentence said "2 for the second, 4 for the third",
transposing them, because I copied my own earlier summary instead of re-reading
the instrument's output. In a shard about not trusting an instrument's echo. It
was caught by re-running the probe rather than by quoting it, which is the only
method this shard actually endorses.

### What makes the repaired version trustworthy

A **positive control inside the instrument**, firing on a case whose answer is
already known:

```
ok = bool(got and got[0] >= 2 and "cell" in got[1])
```

If that is false the instrument prints `INSTRUMENT NOT TESTED -- do not report
the other answers.` — trailing period included, because dropping it would be one
more unmarked deviation — and the answers it has already printed are not to be
used.
Note what the control does **not** do: it never checks for `intercalated disks`,
and its numeric threshold is on tags, not anchors. An earlier draft of this
shard described it as requiring "two anchors, 'cell' and 'intercalated disks'" —
overstating the guard, and re-conflating tags with anchors forty lines after
correcting that same conflation. The code is quoted above rather than
paraphrased for exactly that reason.

It also cannot withhold anything: the per-span results are printed before the
control block runs, so the warning annotates output that has already appeared.
That is weaker than a gate and still sufficient — the point is that the
instrument can say it is not working.

That is the whole difference between the two versions. Both will happily print
zeros. Only the second can tell me it is not working.

### The corollary

**"I did not find it" and "it is not there" must be different outputs.** Any
probe that collapses them will eventually hand you the second when it means the
first, and it will do so in the confident, well-formatted voice it uses for real
results. A region-finder that returns an empty region, a loop whose input is
empty, a matcher whose needle never had a chance — all three print the same
zero as a genuine absence.

The cheapest guard is a denominator: report *how much was examined* beside every
count. "0 tags in a 94-character region" is a finding — that is exactly what the
skeletal sentence measured, and it is true. "0 tags" alone is not, because it is
also what a 0-character region produces, which is what the same probe reported
for the other two.

Related, and each a different cause of the same symptom:
[[a-zero-occurrence-count-is-a-question-with-several-answers-not-a]] — a zero is
a symptom with several causes; this shard covers the case where one cause is
already ruled out by direct observation.
[[count-what-you-inspected-or-the-loop-that-inspects-nothing-reports-success]] —
there the *input* was empty, not the region.
[[i-built-the-exact-vacuous-check-i-had-spent-the-day-criticising]] — the
closest relative: a control that only tested the axis already in mind, and so
could not fail the way the real defect failed. A positive control is the fix for
both.
[[an-instrument-that-enforces-a-convention-nobody-wrote-down-manufactures-defects]]
— a different failure from the same session: there an instrument invented a
rule, here one could not see what it was pointed at. **Not the same script**,
though a draft of this line claimed it was: that shard's instruments are
line-width checkers over `.adj` prose, this one's is an HTML tag probe over a
cached page. What the two share is a session and a review, not a tool.

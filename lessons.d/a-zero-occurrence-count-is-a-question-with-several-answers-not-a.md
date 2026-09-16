---
category: Testing & coverage
---

# A zero occurrence count is a question with several answers, not a finding

When a shipped quotation scores zero occurrences on the page it cites, that
count names a *symptom*. Several unrelated causes produce it, and only one of
them is a defect in the file. Reporting the zero, or acting on it, skips the
diagnosis.

In one session of `adj-facts-stdlib` provenance work, the same symptom appeared
five times from five different causes. **Four were mine.**

**1. The needle's formatting.** `anatomy/hand-bones.adj` has row keys that are
plural atoms — `carpals`, `metacarpals`. The page writes `8 carpal bones`,
`5 metacarpal bones`. So `carpals` scored 0 while `carpal` scored 16 as a
substring — 11 by word boundary, the other 5 being inside `metacarpal`. The
table was fine; the needle demanded a form the source never uses.

**2. The normalizer's blind spot.** `anatomy/foot-bones.adj` cites Wikipedia,
which embeds template JSON in `data-mw` attributes: **55** such attributes on
that page, **38** of them carrying template payloads. (The substring `data-mw`
occurs 126 times, but that counts nine different attribute names —
`data-mw-section-id`, `data-mw-deduplicate` and so on — most carrying no JSON.
I quoted 126 as the payload count until a review asked what the number counted.)
A tag-stripping regex terminates on a `>` inside the single-quoted JSON, welding
it into the running text mid-sentence, so the envelope cannot match. Strip the
`data-mw='…'` attributes first and it matches exactly once.

**3. Two divergences wearing one zero.** `earth-science/plate-boundaries.adj`
quotes a sentence the page writes as `(“subducts”)` with U+201C/U+201D while the
header escapes plain ASCII quotes — and the header also ends it with a period
where the page has a **semicolon**, because on the page it is item 2 of a
three-item list. Measured on the tag-stripped page: curly+semicolon **1**,
curly+period **0**, ascii+period **0**, ascii+semicolon **0**.

So repairing the quotes still yields zero; a second divergence waits at the
terminal punctuation. Half of this case is a normalization artifact and half is
a real verbatim defect of the same class as case 5 — which is exactly why a
single zero must not be diagnosed once and closed.

**4. My own truncated copy.** Same `foot-bones` file, second attempt: I compared
the page against an *elided* envelope taken from my earlier probe's output
rather than from the file's bytes, "found" a paraphrase that did not exist, and
started reasoning about a new defect class.

**5. An actual defect.** `geometry/angle-types.adj` and `angle-pairs.adj` cite
pages that write the degree as a `\(N^{\circ}\)` MathJax macro while the shipped
spans carry a typographic U+00B0. Prose prefix scored 1, full span scored 0.
Real, and filed.

**How to apply.** A zero is the start of the work, not the end of it. Re-read
the shipped span **verbatim from the file** — never from an instrument's echo —
then diff it against the page and print the first diverging character with both
sides around it. The divergence point names the cause; the count never does.

**Once the needle is known to be the file's bytes**, the first differing pair is
distinct in four of the five cases:

    case 1  needle form       's'  vs  ' '     (carpals / carpal bones)
    case 2  data-mw weld      '.'  vs  'u'     (pl.: / plural</span>"}]]...)
    case 3  smart quotes      '"'  vs  '“'     (then again at '.' vs ';')
    case 5  MathJax           '0'  vs  '\'     (at the macro delimiter,
                                                before the digit)

Two qualifications the first draft of this shard lacked. **A repaired first
divergence may expose a second** — case 3 diverges at the quote characters and
again, once those are fixed, at the terminal punctuation. And **the divergence
point only names the cause if the needle is trustworthy**: in case 4 the needle
was a truncated echo, so its divergence named a paraphrase that did not exist.

**Case 4 is the exception, and it is the most useful one.** There the file and
the page AGREE — the diff is empty. The mismatch existed only between the page
and my truncated copy, so an empty diff is not "no information", it is the
diagnosis: *the file is fine and your copy is wrong.* A zero that survives the
count but vanishes under the diff points at the instrument, not the artifact.

Two corollaries earned the hard way:

- **Never reason from your own instrument's abbreviated output.** A probe that
  prints `span[:66]` is telling you what it chose to show, not what is in the
  file. Case 4 was an entire invented defect built on a truncated echo.
- **A normalizer is part of the measurement.** Tag-stripping, whitespace
  collapsing and quote folding each decide what "the page says". State which
  ones ran, because a zero produced by the normalizer looks exactly like a zero
  produced by the file.

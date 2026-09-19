## HL-C409 — `unquote()` does not decode escapes, and no check compares a headword against anything

**Status: OPEN.** Found by security review of the Malayalam punctuation chapter,
after the defect had already passed validate, all twelve gates, the full suite,
a strict book compile and the LaTeX warning scanner.

**The defect.** `ML-W108-quotation` was written with the YAML-documented escape:

```yaml
headword: "\" \""
```

`src/frontmatter.ts`'s `unquote()` strips the outer pair and decodes nothing, so
the headword became the five-character string `\" \"` — literal backslashes —
and shipped into `narration/ch108.json` as `"\\\" \\\""` beside five clean
siblings. Fixed in that chapter by switching to single quotes (`'" "'`).

**Two separable problems.**

**1. The reader accepts a shape it cannot handle.** Double-quoted YAML with
escapes is valid YAML and is what a contributor will reach for. `unquote()`
silently produces a different string than every YAML parser would. Either decode
`\\` and `\"` inside double-quoted scalars, or **reject** a double-quoted scalar
containing a backslash with an error naming the single-quote fix. Rejecting is
the safer change: decoding alters parsing corpus-wide.

**2. Nothing compares a headword against anything.** That is why this survived
every gate. The rendered *title* comes from the body `#` heading, so it was
correct while the frontmatter was wrong, and no check looks at the two together.
A cheap guard: assert that a lesson's `headword` round-trips identically through
a real YAML parser and through `unquote()`, across all 23 tracks. That single
test catches both this and the sibling trap below.

**The sibling trap, same chapter.** `ML-R108-marks-recall` had an **unquoted**
plain scalar containing `: ` (colon-space):

```yaml
headword: (. , ? ! " " — : ( ) - /)
```

The repo's reader takes `indexOf(":")` and handles it; `yaml.safe_load` raises
*"mapping values are not allowed here"* at column 26. So the file was readable by
the build and unreadable by every other tool that might ever lint or edit it.
Also fixed by single-quoting.

**Why this is worth closing rather than tolerating.** `CONTENT_TYPES` in
`constants.ts` restricts the glossary and index — the sinks that interpolate a
raw `headword` into `\textbf{…}` — to `word` and `phrase`. Both offending
lessons were `writing` and `review`, so no backslash reached LaTeX. The same
frontmatter shape on a `word` lesson would reach it, and `\"` is a TeX accent
control sequence. This is a latent injection path held shut by a lesson-type
filter that nobody wrote for that purpose.

**THE SWEEP WAS RUN WHEN THIS SHARD WAS FILED, over all 7,186 lessons in 23
tracks, and it splits the problem cleanly.**

| condition | count |
|---|---|
| headwords containing a literal backslash | **0** |
| `headword`/`romanization`/`gloss`/`concept_tag` the two parsers disagree on | **0** |
| lessons whose frontmatter a real YAML parser rejects outright | **245** |

So **problem 1 is currently at zero** — `ML-W108-quotation` was the only lesson
in the corpus that had it, and it is fixed. That makes the guard cheap to add
and cheap to keep green, and it should go in before the next contributor writes
the shape again.

**Problem 2 is 245 files of pre-existing debt**, concentrated in Arabic
(`AR-C25`..`AR-C36` and many more), all failing with *"while parsing a block
mapping"* — the colon-space trap, not the escape one. That is a separate and
much larger job than this shard, and it is the reason the round-trip guard
cannot simply be asserted corpus-wide on day one: it would fail 245 files that
build correctly today.

**What closing this needs.** Add the backslash-in-scalar check first, since it is
at zero and would have caught the original defect. Then either decode or reject
double-quoted escapes in `unquote()`. The full round-trip assertion comes last,
behind a pass over those 245 files — and that pass deserves a shard of its own
rather than being smuggled in here.

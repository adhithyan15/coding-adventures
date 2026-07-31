# ADJ-ARGUMENT-COMPOSITION — worked whole-paper example (AC-2)

The first **multi-paragraph** worked example for [`ADJ-ARGUMENT-COMPOSITION.md`](../../../ADJ-ARGUMENT-COMPOSITION.md):
a three-paragraph "paper" composed into one `argument` the engine **derives**, `adj-verify`
**byte-anchors across three separate source snapshots**, and `--explain` renders as a
cross-paragraph chain. It is the empirical proof of the AC-1 finding — whole-paper composition
needs **zero new constructs**, and grounding stays **per-paragraph** (multi-snapshot).

## The paper

Three short paragraphs (illustrative, our own words, not attributed to any real paper), each its
own pinned source document (its SHA-256 is the `snapshot` its citations name):

- **`p2-loading.source.txt`** — the loading paragraph. Grounds `stress_amplitude(axle, 420)` and
  `endurance_limit(axle, 380)`, from which paragraph's own inference concludes
  `exceeds_endurance(axle)`.
- **`p3-fractography.source.txt`** — the fractography paragraph. Grounds `shows(surface,
  beach_marks)` and `diagnostic_of(beach_marks, fatigue)`, concluding `fatigue_indicated(axle)`.
- **`p4-discussion.source.txt`** — the discussion paragraph. Its inference `i3` takes the **two
  earlier paragraphs' conclusions** (`i1`, `i2`) as premises-by-reference (`from i1, i2`) and
  reaches the paper's thesis, `failed_by(axle, fatigue)`.

`axle-paper.adj` is the composed argument — one block whose seven citations (4 premises + 3
inference warrants) each name **their own paragraph's** snapshot.

## What it proves

- `adj-lang-cli axle-paper.adj` **derives `failed_by(axle, fatigue)`** by chaining across the three
  paragraphs: `i3` (discussion) ← `i1` (loading) + `i2` (fractography) ← their paragraphs' premises.
  The proof DAG *is* the paper's argument graph; each step keeps **its own paragraph's provenance**.
- `adj-verify --snapshots <dir> axle-paper.adj` — with all three paragraph sources placed as
  content-addressed snapshots in `<dir>` — **byte-anchors all seven citations across the three
  snapshots** (`quotes_verified: 7`, `verified: true`). This is the *multi-snapshot* proof: every
  paragraph's citations re-check against the paragraph they came from, not a single blob.
- `adj-lang-cli --explain axle-paper.adj` renders the cross-paragraph chain — the thesis
  (`source "p4-discussion"`) resting on `exceeds_endurance` (`source "p2-loading"`) and
  `fatigue_indicated` (`source "p3-fractography"`), each grounded in its paragraph's premises.

Together: **a whole paper's argument becomes one program the engine runs, and every step is
auditable back to the specific paragraph's bytes it came from.** Driven end to end by
`adj-lang-cli/tests/composition_worked_example_e2e.rs`.

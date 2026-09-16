- **#14986: `chemistry/measuring-tools.adj` — three tools stop being warranted by the ruler sentence, and the balance row stops quoting a sentence the page does not contain.**
  The envelope was the Part A (ruler) sentence, so a `graduated_cylinder`, `balance` or `thermometer`
  answer's primary source was a sentence about a metric ruler, and the other three lab-part sentences
  were header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-15 on the Chemistry LibreTexts lab manual "1: Introducing Measurements in the
  Laboratory (Experiment)" (HTTP 200; a nonsense path on the same host returns a real 404 holding none
  of these spans), with inline tags removed without a space and only ASCII whitespace collapsed.
  - **Each row takes its own lab-part sentence**, which names both the tool and the quantity. Each
    occurs exactly once, inside one innermost `<p>`, scripts set aside and over the whole file, and
    zero times on the nonsense page.
  - **The envelope** is the page's framing sentence — *"In this lab, students will be introduced to
    some common measuring instruments so that they can practice making measurements, and to learn
    about instrument precision."* It names no tool and no quantity. It occurs once in the body and
    **once more in the page's `<head>` metadata**, so its whole-file count is two and the count check
    allows exactly that one head copy.

  ### The balance row quotes a page typo on purpose (closes #15320)

  The page writes:

  > In Part C, an electronic balance and a triple-beam balance **will be to measure** mass in grams (g).

  — the word "used" is missing. The header used to repair that silently, marking the repair with
  brackets (`will be [used] to measure`), under a heading reading "The exact sentences:". Measured,
  **both** the bracketed form and the plain-"used" form occur **zero** times on the page. A repaired
  citation is a paraphrase wearing quotation marks, so the row now carries the page's own wording and
  the bracketed form is gone from the file.

  Same discipline as `earth-science/cloud-types.adj`, which reproduces its page's singular slip ("The
  two main **type** of mid-level clouds") rather than correcting it. A mutant that restores the
  missing "used" is killed.

  ### The composing rule keeps passing, and its proof trail improves

  `chemistry/measuring-tool-si-unit.adj` is a `rule` that joins this table against
  `metrology/si-base-units.adj`. Its test pins the rule's own NIST warrant and a host-level check that
  both libraries contribute a citation — neither of which this change touches — so it stays green
  (verified by running it, 4/4). What changes is for the better: a `thermometer` derivation's fact
  step now cites the Part D sentence about a thermometer instead of the Part A sentence about a ruler.

  ### Pins

  - **Kept:** the direct bind, the reverse bind, and the `microscope` abstention.
  - **Inverted:** `contains("chem.libretexts.org") && contains("\"trust\":\"consensus\"")` — satisfied
    by any LibreTexts citation and by a truncated span. Each answer is now pinned to its own whole
    citations array, closing on both the corroborations `]` and the citations `]` (#14735).
  - **Added:** a per-tool check with negative arms and an assertion that each span names its own tool;
    a test that the balance row quotes the page's wording and that the bracketed repair is gone; a
    table-shape test that also pins the `consensus` tier.

  **10 of 10 mutants killed, two controls:**
  - the ruler row reverted to a bare row that inherits the envelope;
  - the thermometer row reverted to the ruler sentence (the exact defect being fixed);
  - **the balance span "repaired" to restore the page's missing "used"** (#15320);
  - the balance span truncated before its unit;
  - the old envelope restored;
  - the trust tier swapped to `authoritative`;
  - a table-level `cites` re-added;
  - a quantity atom rebound;
  - the envelope naming a tool;
  - the cylinder span truncated mid-sentence.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 2 answers and 1
  abstention; the framing sentence occurs zero times in its output, and so does the ruler sentence,
  which used to ride on every answer.

  The query example needed no change: unlike the four conversions before it, it never described the
  citation shape.

  ### Two things the pre-push security review noted

  It passed with no findings and confirmed the Part C claim with its own fetch — the shipped wording
  occurs once, the grammatical form zero times, the bracketed form zero times — and re-derived the
  composing-rule result with its own repo-wide grep. Two non-findings, both taken:

  - **The stdlib README's catalogue row did not record the conversion**, unlike its #14986 siblings,
    which each note per-row provenance and the framing envelope. Row 109 now does, including the
    `balance` row's deliberate typo.
  - **The header still claimed the spans were "WebFetch-verified TWICE"**, describing the original
    authoring. That line is replaced: these spans were checked against the raw page. This file's own
    Part C quote is the argument for why — a summarizing fetch is exactly what would smooth "will be
    to measure" into "will be used to measure", which is how the repaired quote got shipped in the
    first place.

- **#14986: `transportation/sign-element-color.adj` and `transportation/sign-shape.adj` — each sign's answer carries its own sentence, not another sign's.**
  The two tables cite the same 2009 MUTCD chapters and share three sentences, so they change
  together. In each, the envelope was a row's own sentence:
  - `sign-element-color`'s envelope was the STOP sentence, so the six yield and warning answers
    cited STOP first.
  - `sign-shape`'s envelope was the regulatory default, *"Regulatory signs shall be rectangular
    unless specifically designated otherwise."*, so the STOP answer (an octagon) was grounded first
    by a sentence saying regulatory signs are rectangular.

  Every row now carries **its own sign's sentence as a per-row `source`**, with the chapter that
  sentence was measured on as its `locator`. Both envelopes are now Section 2A.06's framing sentence,
  *"Standardized colors and shapes are specified so that the several classes of traffic signs can be
  promptly recognized."*, which names no sign, colour or shape value. The table-level `cites` are gone.

  ### Why every row restates `locator`

  The framing sentence is on Chapter 2A, and no row's sentence is: STOP, YIELD and the regulatory
  default are on Chapter 2B, and the warning default and NO PASSING ZONE on Chapter 2C. So every row's
  page differs from the envelope's and each restates `locator`; trust is the same, so none restates it
  (`ADJ-TABLES.md` §4).

  ### Measured

  Measured 2026-09-15 on the raw pages, with inline tags removed without a space and only ASCII
  whitespace collapsed:
  - the six sentences (the five row sentences and the envelope) each occur **exactly once** on their
    own page and **zero** times on the other two, each inside one `<p>`;
  - the regulatory default's nearest enclosing `<td>` is the page's layout table; its innermost block
    is Section 2B.02's `<p>`;
  - the MUTCD host answers a nonsense path with HTTP 200, so status codes prove nothing here. The
    content control held: the nonsense page contains none of the six sentences, and a fabricated
    variant of the STOP sentence occurs zero times.

  No sentence changed: every row sentence is byte-identical to the `source` or `cites` it replaces.

  ### The headers

  - Both "Provenance is TABLE-level" paragraphs, which described the old behaviour, are replaced by
    the per-row account and the measurements above.
  - `sign-shape` said the envelope "carries the regulatory default"; that sentence is rewritten.
  - Both "Both pages were CONTENT-verified" claims now say which two pages, since the tables now
    cite three.

  ### Pins

  Both tests keep every behaviour they shipped with, including the yield-background, compressed
  legend-and-border, defeasible-default and abstention tests and their positive controls.
  - **`sign-element-color`:** the STOP pin becomes the whole contiguous citation (sentence, Chapter
    2B, tier, no corroborations). The chapter-attribution test now asks each sign for its own answer
    and pins its sentence-and-chapter pair, with the other two sentences, the envelope and Chapter 2A
    as negative arms. Added: all nine rows each return one answer with their sign's whole citation,
    and a table-shape test.
  - **`sign-shape`:** the STOP pin, which pinned the regulatory default as STOP's primary source, now
    pins STOP's own citation and forbids the regulatory sentence. The chapter-attribution test now
    covers all five rows the same way. The exception-clause needle becomes the warning row's whole
    citation. Added: a table-shape test.

  **15 of 15 mutants killed, two controls.** `sign-element-color` has 8 mutants:
  - the STOP legend row warranted by the YIELD sentence;
  - a warning row's locator swapped to Chapter 2B;
  - the old envelope (STOP at Chapter 2B) restored;
  - a table-level `cites` re-added;
  - a row's block dropped, so it inherits the envelope;
  - a row restating a different trust tier;
  - a colour atom rebound;
  - a sign named in the envelope.

  `sign-shape` has 7:
  - the STOP row warranted by the regulatory default;
  - NO PASSING ZONE's locator swapped to Chapter 2B;
  - the old envelope (the regulatory default at Chapter 2B) restored;
  - YIELD given the tag-stripping space in "(see Figure 2B-1 )";
  - the regulatory row's block dropped;
  - the warning sentence losing its exception clause;
  - a shape named in the envelope.

  The controls are the unmutated suites, green before the first mutant and after the last, with both
  tables byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelopes' wording reaches no answer, and is pinned only in the files.** Every row overrides
  it, so the framing sentence and Chapter 2A occur zero times in either query example's output. The
  four envelope mutants (old envelope restored, and a sign or shape named in it, in each table) were
  killed only by the file-shape tests.

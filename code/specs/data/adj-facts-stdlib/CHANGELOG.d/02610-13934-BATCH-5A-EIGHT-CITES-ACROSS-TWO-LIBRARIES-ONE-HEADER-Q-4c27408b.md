- **#13934 batch 5a: eight `cites` across two libraries, one header quote corrected.**
  `geometry/quadrilateral-types` (+4 — rectangle, rhombus, parallelogram, trapezoid) and
  `astronomy/celestial-objects` (+4 — planet, moon, comet, asteroid).

  **Every sentence was re-verified under a different extractor than the one that first checked it.**
  Two more extractor bugs surfaced after the initial pass, which made those earlier "VERBATIM"
  results stale — they had been checked against a haystack that no longer existed. All twenty
  sentences accepted this session (the four already shipped in batch 4 included) were re-run against
  the corrected extractor and all twenty reproduce.

  *** BUG 8, AND THE FIRST ONE THAT ADDS TEXT RATHER THAN REMOVING IT. *** `<[^>]+>` assumes a tag
  ends at the first `>`, but attribute *values* contain them, and Wikipedia emits
  `data-mw='{"parts":[…]}'`. The regex terminated inside that JSON and spilled the remainder into
  what was being treated as page prose. Every earlier bug DELETED or FUSED text; this one injects
  markup, so a quote could "verify" against something no reader has ever seen. Fixed by scanning
  tags with quote awareness.

  Fixing it immediately **re-broke bug 3** (the adjacent-tag join), because the preceding character
  was derived from the output buffer rather than the raw text, and `aerogens[1])` came out as
  `aerogens [1] )`. That is the fourth time a fix created a new failure mode this session and the
  first to undo an existing fix. Both cases now carry regression probes in the extractor itself.

  *** BUG 9: `figure`/`figcaption` were not block boundaries, so image captions fused into the
  paragraph after them. *** On one page this buried a lead sentence 431 characters inside a block
  beginning "Astatine is here represented by Uraninite…", which made the sentence look **absent**
  when it was merely buried — and it had already been reported here as "genuinely not on the page".
  It affects any page carrying figures.

  ON `rectangle` AND THE `<img alt>` RULE. MathWorld renders inline variables as
  `<img class="inlineformula" alt="a">` — 9px images *of the letters* a and b. Recovering the alt
  reproduces what the page displays. That is **not** true of `circle-parts`' diameter, whose
  `<img alt="pi">` renders a π glyph: there the alt *names* the glyph rather than being it. So alt
  recovery is faithful in one case and a substitution in the other, checkable per page — which
  converts what was recorded last cycle as a blanket MathJax blocker into a per-row question, and
  makes `rectangle` citable while `diameter` stays correctly refused.

  A FIFTH DEFECTIVE HEADER QUOTE: `rectangle`'s read "A closed planar quadrilateral with opposite
  sides…" where the page reads "**A rectangle is** a closed planar quadrilateral…" — it dropped its
  own subject, violating precisely the rule a citation exists to satisfy. Corrected here.

  **But the blanket claim needed qualifying, and `celestial-objects` is why:** all four of its header
  quotes verified CLEAN, as-is, first try. The defect tracks *editorial intervention* — joining
  blocks, eliding with "…", flattening curly quotes, trimming a subject — not header quotes as such.

  `celestial-objects` also **cleared** a host rather than blocking one: `science.nasa.gov` is a
  modern CMS of the kind that is often JS-rendered, and all three of its pages returned 9000+
  characters of body text. The check that separates "the sentence is not there" from "nothing is
  there" earned its keep on the positive side.

  The four new tests are anchored joint-binding pins cut from real CLI output. Two of them span the
  **whole corroboration list**, and the mutation check shows why that is worth doing: a pure
  **reorder** of two middle entries — nothing removed, every sentence still present — reddens the
  whole-list pin while leaving the prefix pin green. No per-sentence `contains` check could catch
  that. All four mutations behave directionally (truncating trapezoid's cite reddens trapezoid and
  leaves rectangle green; truncating asteroid's reddens asteroid and leaves moon green).

  Both `.query.adj` companions parse, run, and abstain correctly. 533 test binaries / 1600 tests
  green, clippy `-D warnings` clean.


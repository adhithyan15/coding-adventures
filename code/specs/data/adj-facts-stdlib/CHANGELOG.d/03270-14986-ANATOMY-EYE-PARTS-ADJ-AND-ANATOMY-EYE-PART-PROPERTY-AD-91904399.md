- **#14986: `anatomy/eye-parts.adj` and `anatomy/eye-part-property.adj` — two eye-parts spans that occurred on no page get the page's no-break spaces, and each eye-part property carries its own sentence.**
  The two tables cite the same NEI "How the Eyes Work" page and share the retina sentence, so they change
  together.

  ### Two row sources the page never wrote

  `eye-parts` was already per-row. Measured 2026-09-15 on the page (HTTP 200; a nonsense path on the
  same host returns 404), with inline tags removed without a space and only ASCII whitespace
  collapsed, two of its row sources occurred **zero** times. The page writes **U+00A0** (NO-BREAK SPACE)
  where they had ordinary spaces:
  - **retina:** twice, on both sides of "retina";
  - **optic nerve:** three times, after "the", "optic" and "nerve".

  Every divergence is a space where the page has U+00A0. With the page's characters, each sentence
  occurs exactly once, and both rows now carry them. That makes five and six U+00A0 spans found this
  cycle.

  The header had said all seven spans "were confirmed verbatim on the page, each occurring EXACTLY
  ONCE"; it now says five held and names the two that did not. A note under the quote blocks names the
  U+00A0 a comment cannot show. The test pinned the ordinary-space forms, so it stayed green while both
  spans were absent from the page; its pins now carry U+00A0.

  ### eye-part-property

  The envelope was the iris row's own sentence, the primary source of all three answers. Every row now
  takes its own sentence as a `source`: cornea, iris, and the retina sentence in its page form. The
  envelope is now the framing sentence `eye-parts` already uses, *"All the different parts of your eyes
  work together to help you see."*, which names no part. The header's "reproduces, byte-for-byte … no
  new WebFetch" paragraph is replaced. The retina quote it called byte-for-byte had ordinary spaces
  where the page has U+00A0.

  ### Pins

  - **`eye-parts`:** every shipped behaviour is kept. The retina and optic-nerve pins carry U+00A0, and
    a new test checks both page forms are in the table while the ordinary-space forms are not in the
    table and reach no answer.
  - **`eye-part-property`:** every shipped behaviour is kept. The locator-and-trust needle becomes the
    iris row's whole contiguous citation. Added: a per-part citation check with negative arms, a retina
    U+00A0 check, and a table-shape test.

  **10 of 10 mutants killed, two controls.** `eye-parts` has 3 mutants:
  - the retina sentence's first U+00A0 restored to a space;
  - the optic-nerve sentence's last U+00A0 restored to a space;
  - the retina row warranted by the optic-nerve sentence.

  `eye-part-property` has 7:
  - the retina row given the ordinary-space form;
  - the iris row warranted by the cornea sentence;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row `locator` added pointing elsewhere;
  - a property atom rebound;
  - a part named in the envelope.

  The controls are the unmutated suites, green before the first mutant and after the last, with both
  tables byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **`eye-part-property`'s envelope wording reaches no answer, and is pinned only in the file.** Every
  row overrides it, so the framing sentence occurs zero times in its query example's output.

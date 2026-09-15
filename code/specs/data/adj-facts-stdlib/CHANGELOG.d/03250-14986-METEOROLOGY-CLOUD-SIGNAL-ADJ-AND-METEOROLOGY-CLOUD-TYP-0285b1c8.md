- **#14986: `meteorology/cloud-signal.adj` and `meteorology/cloud-type.adj` — each cloud's answer carries its own sentence, and a span both tables shipped is repaired.**
  The two tables cite the same NWS Louisville "Cloud Classification" page and shared a defect, so they
  change together. In each, the envelope was a row's own sentence:
  - `cloud-signal`'s envelope was the cirrus value sentence, the primary source of the three stratus
    answers.
  - `cloud-type`'s envelope was the cirrus span, the primary source of the cumulonimbus and stratus
    answers.

  Every row now carries **its own cloud's sentence as a per-row `source`**, and both envelopes are
  now the page's framing sentence, *"Clouds are classified according to their height above and
  appearance (texture) from the ground."*, which names no cloud and no signal. The table-level `cites`
  are gone.

  ### A span both tables shipped that the page never wrote

  The stratus sentence, *"…which may be precipitation-free or may cause periods of light precipitation
  or drizzle."*, was shipped with an ordinary space after "precipitation-free". The page writes a
  **non-breaking space** (U+00A0) there. Rendered with U+00A0 kept, the shipped string occurs **zero**
  times; the page's own string occurs once. A `git grep` of `origin/main`, using `cloud-signal.adj`'s
  two known matches as the positive control, finds the plain-space string shipped in exactly these
  two tables. This is the third U+00A0 span found this cycle, after `long-bone-parts` and
  `muscle-groups`. Both `source` fields now carry the page's character, and both headers' stratus
  quotes carry a note naming it.

  ### `source`, not `cites`

  Measured 2026-09-15 (HTTP 200; a nonsense path on the same host returns 404), with inline tags
  removed without a space and U+00A0 kept:
  - the two-sentence cirrus span occurs once, inside one `<p>`;
  - the cumulonimbus sentence occurs once;
  - the framing envelope occurs once.

  Each row sentence names both its cloud and its value, so each row takes a warrant. Every row's page
  is the envelope's, so rows restate neither locator nor trust (`ADJ-TABLES.md` §4).

  ### The headers

  - **`cloud-signal`:** it said its spans were reproduced "byte-for-byte" with "no new WebFetch", and
    that an ADJ `table` carries ONE provenance envelope. Neither holds, and the paragraph is replaced.
  - **`cloud-type`:** its quotes were "WebFetch-verified before writing". That claim is withdrawn,
    because the stratus quote was not verbatim.

  ### Pins

  Both tests keep every behaviour they shipped with:
  - `cloud-signal`: stratus recalls all three signals, cirrus is recalled for the jet streak, and
    cumulonimbus abstains.
  - `cloud-type`: cirrus recalls its indication, cumulonimbus is recalled for heavy rain and
    thunderstorms, and altocumulus abstains.

  In each, the `contains("weather.gov") && contains(trust)` check (#15209's shape) becomes the whole
  contiguous citation run. Each test adds three checks:
  - every row's answer is one answer, whose primary source is its own cloud's sentence, and which
    carries no other cloud's sentence and not the envelope;
  - the stratus U+00A0 is present, and the ordinary-space string reaches no answer and appears
    nowhere in the table;
  - the table's shape: one row `source` per row, a framing envelope that names no cloud or value, no
    `cites`, no row locator or trust.

  **14 of 14 mutants killed, four controls.** `cloud-signal` has 8 mutants:
  - the U+00A0 restored to a plain space;
  - a stratus row warranted by the cirrus span;
  - a cirrus row warranted by the stratus sentence;
  - the old envelope restored;
  - a table-level `cites` re-added;
  - a row `source` demoted to `cites`;
  - a cloud appended to the envelope;
  - the trust tier flipped.

  `cloud-type` has 6:
  - the U+00A0 restored;
  - cumulonimbus warranted by the cirrus span;
  - the cumulonimbus sentence truncated before its value;
  - the old envelope restored;
  - a row `locator` added pointing elsewhere;
  - a value atom rebound.

  The controls are each unmutated suite, green before the first mutant and after the last. Both files
  were byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced from
  the repo.

  **The envelopes' wording reaches no answer, and is pinned only in the files.** Every row overrides
  it, so the framing sentence occurs zero times in either query example's output. The old-envelope
  and envelope-names-a-cloud mutants were killed only by the file-shape tests. The `cloud-type` query
  example does not ask about stratus, so its output says nothing about the U+00A0 either way; that is
  pinned by the U+00A0 test.

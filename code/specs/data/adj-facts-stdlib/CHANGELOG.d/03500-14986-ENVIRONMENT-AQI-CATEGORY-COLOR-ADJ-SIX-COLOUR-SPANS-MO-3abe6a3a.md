- **#14986: `environment/aqi-category-color.adj` — six colour spans move out of table-level `cites` and into their own rows.**
  The six AirNow spans were table-level `cites`, so **every answer carried all six**: ask what colour a
  `hazardous` day is and the proof offered was the envelope plus the Green, Yellow, Orange, Red and
  Purple spans alongside the Maroon one.

  ### What each row carries now

  Measured 2026-09-15 on AirNow's "AQI Basics" page (HTTP 200; a nonsense path on the same host
  returns a real 404 holding none of these spans), with inline tags removed without a space and only
  ASCII whitespace collapsed.
  - **Each row takes the AirNow span that names its own colour.** Each occurs **exactly once**, scripts
    set aside and over the whole file, and zero times on the nonsense page.
  - **What element holds a row span.** AirNow renders these six rows as a real HTML table, so each
    span's innermost container is one `<tr>`, and the span itself is that row's four `<td><strong>`
    cells read left to right, joined by the inter-cell whitespace this extraction method collapses. No
    single `<td>` holds a whole span, and the only `<div>` involved wraps the entire table — the same
    one for all six rows, so it establishes nothing per span. An earlier draft of this entry said
    "inside one innermost `<div>`"; that measurement's tag list omitted `<tr>`, and it is corrected
    here.
  - **The envelope needed no replacement.** It was already a framing sentence — *"The AQI includes six
    color-coded categories, each corresponding to a range of index values."* — which names no category
    and no colour. That is checked mechanically against the row keys and values rather than by eye.
  - **Nothing moved between pages.** All six `cites` were already on the envelope's own locator, so
    this conversion *relocates* rather than re-sources them, and no row restates a `locator` line or a
    `trust` tier.

  This is the only conversion in the batch where the evidence was already encoded correctly and only
  the *shape* was wrong — which is why it is also the only one whose envelope and locators are
  untouched.

  ### Pins

  - **Kept:** the forward recall, the backward recall from a bound colour, and the full-domain check
    (this table never abstains — every category has a colour).
  - **Inverted:** `contains("airnow.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by
    any AirNow citation, and previously satisfied by one citation object carrying six corroborations.
    Each answer is now pinned to its own whole citations array, closing on both the corroborations `]`
    and the citations `]` (#14735).
  - **Added:** a per-category check with negative arms against the other five spans and the envelope,
    plus an assertion that each span names its own colour; a table-shape test pinning the
    `authoritative` tier and the unchanged envelope.
  - The shape test asserts `!contains("cites ")` rather than `!contains("\n    cites ")`, so a
    row-level corroboration at eight spaces fails it too — a sibling review noted the indent-scoped
    form would miss that.

  **10 of 10 mutants killed, two controls:**
  - the good row reverted to a bare row that inherits the envelope;
  - **the hazardous row's span pulled back up to a table-level `cites`** (the exact shape removed);
  - the moderate row reverted to the good row's span;
  - **the envelope replaced by a row's span** (the defect the other tables in this batch had);
  - the envelope naming a category;
  - the trust tier swapped to `consensus`;
  - a colour atom rebound;
  - the very-unhealthy span truncated before its health alert;
  - the sensitive-groups span truncated mid-sentence;
  - a row restating the locator.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 3 answers and no
  abstentions; the framing sentence occurs zero times in the output, and every corroboration array is
  empty.

  The query example said the engine returns the colour plus "the table's source/locator/trust"; it now
  describes the row's own span with the locator and trust inherited. The README row records the
  conversion, as its #14986 siblings do.

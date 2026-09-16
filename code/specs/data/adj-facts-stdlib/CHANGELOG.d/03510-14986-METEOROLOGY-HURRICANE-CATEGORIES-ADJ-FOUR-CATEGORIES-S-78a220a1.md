- **#14986: `meteorology/hurricane-categories.adj` — four categories stop being warranted by the category-1 sentence.**
  The envelope was the category-1 span, so asking what a category 5 does returned "Catastrophic damage
  will occur…" evidenced by *"Very dangerous winds will produce some damage…"* — a sentence about
  category 1. The other four NHC sentences were header prose that reached no answer.

  ### What each row carries now

  Measured 2026-09-15 on the NOAA/NHC Saffir-Simpson Hurricane Wind Scale page (HTTP 200; a nonsense
  path on the same host returns a real 404 of 16 bytes holding none of these spans), with inline tags
  removed without a space and only ASCII whitespace collapsed.
  - **Each row takes the NHC sentence stating its own damage phrase.** Each occurs **exactly once**,
    inside one `<td>`, scripts set aside and over the whole file, and zero times on the nonsense page.
  - **The envelope** is the page's scale-defining sentence — *"The Saffir-Simpson Hurricane Wind Scale
    estimates potential property damage."* — which names no category and no damage value. It occurs
    once, inside one `<p>`, with no `<head>` copy.

  ### The honest duplicate survives, and is now pinned

  `category_4` and `category_5` share the value `catastrophic_damage`, because the page opens both
  paragraphs with "Catastrophic damage will occur." **Their spans differ**, so each row now cites the
  paragraph actually about it rather than both answers sharing one sentence. The query example shows
  it: the backward `catastrophic_damage` query binds both categories, and each answer carries its own
  paragraph.

  That property needed its own test, because an edit that deduplicated by *value* would leave the
  forward and backward binds passing unchanged — the values are identical either way. Two mutants
  cover both merge directions, and both are killed.

  ### The envelope took two passes to find

  The first candidate hunt scanned only `<p>`/`<li>` sentence-split text and returned nothing but
  disclaimers about hazards the scale does **not** cover — promoting one would have framed the table
  by what it excludes, and I nearly recorded it as blocked. A wider search over whole-body text found
  the real envelope: it follows a `<br><br>` inside a paragraph whose *first* sentence is the
  disclaimer, so the sentence-splitter had dropped it. **When an envelope hunt comes up empty, re-run
  it over whole-body text before concluding the page has no framing sentence.**

  ### Pins

  - **Kept:** the forward binds, the backward bind returning both catastrophic categories, and the
    `category_6` abstention.
  - **Inverted:** `contains("nhc.noaa.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by
    any NHC citation, and previously satisfied by the category-1 span riding on every answer,
    including the category-5 one. Each answer is now pinned to its own whole citations array, closing
    on both the corroborations `]` and the citations `]` (#14735).
  - **Added:** a per-category check whose negative arms assert the other rows' **spans** are absent but
    never the descriptor atom (categories 4 and 5 share that value); a test that the two catastrophic
    rows cite different paragraphs; a table-shape test pinning the tier and the envelope.

  **10 of 10 mutants killed, two controls:**
  - **category_5 pointed at category_4's span** (duplicate-value merge);
  - **category_4 pointed at category_5's span** (the reverse merge);
  - the category_1 row reverted to a bare row that inherits the envelope;
  - category_3 reverted to the category_1 span (the exact defect being fixed);
  - the old envelope restored;
  - the envelope naming a category;
  - a table-level `cites` re-added;
  - the trust tier swapped to `consensus`;
  - a descriptor atom rebound;
  - the category_3 span truncated before its damage clause.

  The controls are the unmutated suite, green before the first mutant and after the last, with the
  file byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer.** The shipped query example returns 7 answers and 1
  abstention; the framing sentence occurs zero times in the output, and every corroboration array is
  empty.

  The query example said the engine returns the descriptor plus "the table's source/locator/trust"; it
  now describes the row's own NHC sentence with the locator and trust inherited. The README row needs
  no change — it describes the axis and three example pairs, not the citation shape.

  **Found while converting: the sibling has the same defect.**
  `meteorology/hurricane-category-home-damage.adj` ships the *same* category-1 sentence as its
  envelope, for five rows drawn from the same five NHC paragraphs. Not converted here — it is its own
  table with its own value atoms — but it is the obvious next one, and its spans are already measured.

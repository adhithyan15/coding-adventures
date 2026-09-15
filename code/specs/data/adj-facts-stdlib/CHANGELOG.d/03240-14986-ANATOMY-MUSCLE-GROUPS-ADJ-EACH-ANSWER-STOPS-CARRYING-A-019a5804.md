- **#14986: `anatomy/muscle-groups.adj` — each answer stops carrying all nine articles' sentences, and one span that was never on its page is repaired.**
  The envelope was the biceps sentence, and the other eight articles' sentences were table-level
  `cites`. So every answer, the gluteus maximus one included, was primarily sourced to the biceps
  article and carried all nine sentences. Each row now carries **its own sentence and its own
  article** as a per-row `source` and `locator`. The envelope is now a sentence from the gluteus
  maximus article that names no muscle and no region: *"The muscle is made up of muscle fascicles
  lying parallel with one another, and are collected together into larger bundles separated by
  fibrous septa."* Every row overrides it.

  ### One span was never on its page

  The pectoralis major row shipped *"…(from Latin pectus 'breast')…"* with an ordinary space between
  "pectus" and "'breast'". The Wikipedia article writes a **non-breaking space** (U+00A0) there.
  Rendered with U+00A0 kept, the shipped string occurs **zero** times, while the article's own string
  occurs once. It is the second U+00A0 found this cycle, after `anatomy/long-bone-parts.adj`'s
  periosteum sentence. A search of the stdlib and the e2e tests found no copy of this span anywhere
  else, with a positive control confirming the search could see the one known copy. The shipped
  `source` now carries the page's character, written in the test as the visible escape `\u{a0}`.

  ### `source`, not `cites`

  Measured 2026-09-15. All nine articles returned HTTP 200, and a nonsense article returned 404.
  Rendered with inline tags removed without a space and U+00A0 kept, each of the nine sentences
  occurs **exactly once** in its own article, and each names both its muscle and the row's region.
  So each row takes a warrant, as in `lung-lobe-names` and `long-bone-parts`. Each row also
  restates `locator`, because its article differs from the envelope's; `ADJ-TABLES.md` §4 omits a
  row locator only when it is the same.

  ### The header

  - **False premise:** *"An ADJ `table` carries ONE provenance envelope"* is replaced.
  - **Pectoralis quote:** the header's pectoralis quote now carries a note naming the U+00A0 it
    cannot show.

  ### Pins

  Four of the shipped tests pinned whole citation runs through the table-level corroboration list
  that every answer carried. Each is carried forward, for the defect it guards, onto that row's own
  primary source:
  - **deltoid** keeps its page's `[1]` footnote marker;
  - **quadriceps** keeps the IPA-and-alias parenthetical its header once deleted;
  - **biceps** keeps its Latin gloss;
  - **the `arm` reverse answer**, which pinned "all eight corroborations in order", a description of
    the defect being removed, now asserts two answers, each with its own article and neither
    carrying another muscle's sentence.

  The loose `contains("en.wikipedia.org") && contains(trust)` check (#15209's shape) becomes whole
  contiguous runs. Three tests are added:
  - every muscle's answer is one answer, whose primary citation is its own sentence and article;
  - the pectoralis U+00A0, with the ordinary-space string reaching no answer and appearing nowhere
    in the table;
  - the table's shape: nine row `source`+`locator` pairs, one framing envelope naming no muscle or
    region, no `cites`, no row trust.

  **14 of 14 mutants killed, two controls.** The mutants:
  - the pectoralis U+00A0 replaced by a plain space;
  - deltoid's `[1]` removed;
  - quadriceps' parenthetical removed;
  - biceps' Latin gloss removed;
  - the deltoid and pectoralis sentences swapped;
  - a row locator pointed at another muscle's article;
  - a row's `source` and `locator` deleted;
  - a row `source` demoted to `cites`;
  - a table-level `cites` re-added;
  - the old biceps envelope restored;
  - a row `trust` added;
  - the trust tier flipped;
  - a region atom rebound;
  - a muscle appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output. The two envelope
  mutants were killed only by the file-shape test, and within it the "names no muscle" check was
  caught first by the envelope-equality assertion. The query example does not ask about pectoralis
  major, so its output says nothing about the U+00A0 either way; that is pinned by its own test.

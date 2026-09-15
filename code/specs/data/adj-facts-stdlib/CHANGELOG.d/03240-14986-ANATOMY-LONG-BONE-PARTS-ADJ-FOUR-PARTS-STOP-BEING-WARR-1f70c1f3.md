- **#14986: `anatomy/long-bone-parts.adj` — four parts stop being warranted by the diaphysis line, and one span that was never on the page is repaired.**
  The envelope was *"Diaphysis: Also known as the shaft."*, so it was the primary source of the
  epiphysis, metaphysis, periosteum and epiphyseal-plate answers. Each row now carries its own
  sentence as a per-row `source`. The envelope is now *"Long bones evolve via endochondral
  ossification."*, from the same section, which names no part and which every row overrides.

  ### One span was never on the page

  The periosteum row shipped *"The periosteum surrounds the bone surface."* with an ordinary space
  between "surrounds" and "the". The page writes `surrounds&#x000a0;the`, a **non-breaking space**
  (U+00A0). Rendered with U+00A0 kept, the shipped string occurs **zero** times and the page's own
  string occurs once. A renderer that collapses all whitespace, such as Python's `str.split()`,
  shows the two as identical and reported the shipped string present. That is the rendering my own
  first check used. The shipped `source` now carries the page's character. The test writes it as
  the visible escape `\u{a0}` and asserts that the ordinary-space string reaches no answer and
  appears nowhere in the table.

  ### `source`, not `cites`

  Measured 2026-09-15 on the NCBI Bookshelf StatPearls "Anatomy, Bones" page (NBK537199; HTTP 200;
  a nonsense NBK id returns 404). Diaphysis, Epiphysis and Metaphysis are each **one**
  `<li><div>` whose "Term:" head is inline in the same element. That is plant-tropisms'
  single-element shape, not a heading or a `<dt>`/`<dd>` weld. The epiphyseal-plates and periosteum
  sentences name their subject in running prose. Every sentence names its own part, so each row
  takes a warrant, as in `anatomy/lung-lobe-names.adj`. Each row sentence and the envelope occur
  exactly once.

  ### The header

  - **False premise:** *"An ADJ `table` carries ONE provenance envelope"* is replaced.
  - **Stale citation line:** *"carrying the table's citation"* now reads *"carrying its own row's
    citation"*.
  - **"Character-for-character" claim:** the provenance section's quotes are now qualified to name
    the one they cannot show, the U+00A0.
  - **Row-line comments:** the trailing `%` comments on each row, which put `...` inside quotation
    marks, are dropped. Each row's sentence is now in its own block.
  - **Earlier quotes:** the truth table and the "Why these tokens" paragraph quote "surrounds the
    bone surface" with a plain space. They now carry a note that the page has U+00A0 there.

  ### Pins

  The shipped test's `contains("ncbi.nlm.nih.gov/books/NBK537199") && contains(trust)` check
  (#15209's shape) is replaced by whole contiguous runs, one per part. The recalls and the tendon
  abstention are kept. Three tests are added:
  - every part's answer is one answer, whose primary source is its own sentence, and which carries
    no other part's sentence and not the framing envelope;
  - the periosteum sentence carries exactly one U+00A0 and differs from the old string only there,
    and the old string reaches no answer and appears nowhere in the table;
  - the table's shape: five row `source`s, one framing envelope naming no part, no row locator or
    trust, no `cites`.

  **13 of 13 mutants killed, two controls.** The mutants:
  - the U+00A0 replaced by a plain space;
  - epiphysis warranted by the diaphysis sentence;
  - the metaphysis and epiphyseal-plate sentences swapped;
  - a row's `source` deleted;
  - a row `source` demoted to `cites`;
  - the old envelope restored;
  - a row `locator` added pointing elsewhere;
  - a row `trust` downgraded;
  - the envelope locator repointed;
  - the trust tier flipped;
  - a description atom rebound;
  - the metaphysis sentence truncated;
  - a part appended to the envelope.

  The controls are the unmutated suite, green before the first mutant and after the last. The file
  was byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  **The envelope's wording reaches no answer, and is pinned only in the file.** Every row overrides
  it, so the framing sentence occurs zero times in the query example's output. The two envelope
  mutants were killed only by the file-shape test, and within it the "envelope names no part" loop
  was caught first by the envelope-equality assertion.

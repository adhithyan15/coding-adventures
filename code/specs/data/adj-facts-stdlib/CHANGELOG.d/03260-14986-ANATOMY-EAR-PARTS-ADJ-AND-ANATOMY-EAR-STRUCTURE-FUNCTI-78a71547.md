- **#14986: `anatomy/ear-parts.adj` and `anatomy/ear-structure-function.adj` — each ear structure's answer carries the sentences that place it, not the cochlea's.**
  Both tables cite NIDCD's "How Do We Hear?" and shipped the **same envelope**: *"The bones in the
  middle ear amplify, or increase, the sound vibrations and send them to the cochlea, a snail-shaped
  structure filled with fluid, in the inner ear."* So it was the primary source of all five
  `ear-parts` answers, including the ear canal's, and of all three `ear-structure-function` answers.
  That sentence names no individual ossicle.

  ### What each row carries now

  Measured 2026-09-15 (HTTP 200; a nonsense path on the same host returns 404), with inline tags
  removed without a space and only ASCII whitespace collapsed. Each sentence below occurs **exactly
  once**. The path-of-sound sentences sit in `<li>` items, the framing sentence in a `<p>`.

  **`ear-parts`:**
  - **cochlea → inner_ear** takes the cochlea sentence as its `source`, since it names both.
  - **ear_canal → outer_ear** `cites` *"Sound waves enter the outer ear and travel through a narrow
    passageway called the ear canal, which leads to the eardrum."* The canal's region is read off
    that sentence, not stated outright, so it corroborates rather than warrants.
  - **malleus, incus, stapes → middle_ear** each `cites` *"…three tiny bones in the middle ear."* and
    *"These bones are called the malleus, incus, and stapes."* "These bones" names them only through
    the sentence before it.

  **`ear-structure-function`:** each ossicle `cites` three sentences in page order: the two bone
  sentences, then the amplify sentence. No single sentence names a bone and what it does.

  Both envelopes are now the page's framing sentence, *"Hearing depends on a series of complex steps
  that change sound waves in the air into electrical signals."*, which names no structure, region or
  function. It stays the primary source of every `cites` row. Every row's page is the envelope's, so
  no row restates `locator` or `trust` (`ADJ-TABLES.md` §4).

  ### The headers

  - **`ear-parts`:** it said an ADJ `table` carries ONE provenance envelope, holding "the single
    strongest span". That paragraph is replaced by the per-row account. Its per-row quotes each
    measured at exactly one occurrence and are unchanged.
  - **`ear-structure-function`:** it said its spans were reproduced "byte-for-byte" with "no new
    WebFetch". That is replaced by the measurement.

  ### Pins

  Both tests keep every behaviour they shipped with: the forward, reverse and three-answer backward
  recalls, and the pinna and ear-canal abstentions. Each loose `contains("nidcd.nih.gov") &&
  contains(trust)` check (#15209's shape) becomes a whole contiguous citation run. Added:
  - every structure's answer is one answer carrying its measured citation shape, with the other
    rows' sentences as negative arms;
  - each ossicle's answer carries the framing envelope, then all three chained sentences in page
    order;
  - a table-shape test for each table.

  **14 of 14 mutants killed, two controls.** `ear-parts` has 8 mutants:
  - the cochlea's `source` demoted to `cites`;
  - the ear canal's `cites` promoted to a `source`;
  - the incus losing its middle-ear antecedent sentence;
  - the stapes' sentences in the wrong order;
  - the old envelope (the cochlea sentence) restored;
  - a table-level `cites` re-added;
  - a region atom rebound;
  - a region named in the envelope.

  `ear-structure-function` has 6:
  - an ossicle losing the amplify sentence;
  - the amplify sentence moved first;
  - an ossicle promoted to a `source` of the amplify sentence;
  - the old envelope restored;
  - a `cites` locator repointed;
  - a bone named in the envelope.

  The controls are the unmutated suites, green before the first mutant and after the last, with both
  tables byte-identical afterwards. It's a local scratch harness, so that count can't be reproduced
  from the repo.

  In both tables the framing envelope reaches answers: it is the primary source of every `cites` row.
  The contiguous citation pins cover it there.

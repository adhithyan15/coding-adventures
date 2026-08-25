### Added - Spanish A1 exam inventory: three more PCIC dimensions

- Enumerate the PCIC **functional inventory** (54 A1 points), the **general and
  specific notions** (36 + 77), and the **orthography inventory** (21) into
  `core/exam-inventory-es-a1.json`, restated in our own words at the granularity
  of the source's numbered subsections. The file grows from 85 to **273 points**.
- Flip `communicative-functions` and `lexicon` to `complete`. Both PCIC
  inventories publish separate A1 and A2 columns, so the A1 boundary is closed by
  the source rather than by our judgement.
- Keep `phonology-orthography` **partial**, and say exactly why in its note. The
  orthography inventory does split A1 from A2 and is fully enumerated here; the
  pronunciation-and-prosody inventory does not — it presents its five sections as
  one undivided A1-A2 band with no per-item level marking. There is no A1-only
  pronunciation boundary to restate, and inventing one is the editorial promotion
  HL20 forbids. The inventory therefore stays partial overall, and its
  `exam-inventory` backlog item is not suppressed.
- **Measured Spanish A1 exam coverage falls from 85/85 (100%) to 223/273 (82%),
  with 50 unmapped points.** This is the intended, honest outcome and not a
  regression: no lesson was retired and no probe was loosened, the grammar
  dimension is still 85/85 inside the new total, and 138 of the 188 new points
  map to atoms the corpus really introduces. The old 100% measured one content
  dimension of four.
- Every one of the 50 unmapped points carries a `note` naming the source exponent
  the corpus does not introduce, and a new assertion enforces that. The gaps are
  real and nameable: clothing, cinema and music, the internet and dictating an
  e-mail address, police and fire, the alphabet, capitalisation, and every
  punctuation mark except the question and exclamation pair.
- No atom id was invented: all 138 probes were validated against the atoms the
  Spanish track actually introduces before the file was committed.
- Spanish A1 consequently joins the `exam-point` planner queue for the first
  time, and the corpus-wide uncovered-point total moves 146 → 196.

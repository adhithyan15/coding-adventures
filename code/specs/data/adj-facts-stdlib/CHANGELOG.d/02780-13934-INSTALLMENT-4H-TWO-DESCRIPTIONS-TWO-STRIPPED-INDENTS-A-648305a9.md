- **#13934 installment 4h: two descriptions, two stripped indents, and a headline that understates
  the round.** 4 value lines, 10 row lines and 161 comment lines across **3** `.adj` files —
  `geometry/solid-vertices-edges` (two values), `biology/genetic-code` and `biology/start-codon` —
  plus three e2e test files. **Eight mutations redden.** Contiguity **60 → 58** over the full
  population of 563; the two that left are the two Platonic values, member for member, and none
  entered.

  **The triage was wrong about all four, in two groups and in the same direction**: it filed each
  behind a held question when reading the page showed they needed none. That is the round's lesson,
  and it is the standing rule — read each candidate's page before repairing it — paying for itself
  twice.

  ### Two values that described a table instead of quoting one

  `solid-vertices-edges` held author-composed prose ABOUT the page — *"MathWorld's Platonic Solid
  table gives, for each solid, its number of vertices: tetrahedron 4, cube 8, …"*. Accurate, and not
  a quotation of anything: **it names the source in the third person, which no verbatim span of a
  page ever does.** A field whose contract is a contiguous span of its `locator` held a summary, and
  no reader could tell without fetching.

  The census filed both behind the table-row question, assuming the counts live only in the page's
  table. They do not — MathWorld states them in ordinary prose:

  > The ordered number of faces for the Platonic solids are 4, 6, 8, 12, 20 (OEIS A053016; in the
  > order tetrahedron, cube, octahedron, dodecahedron, icosahedron), which is also the ordered
  > number of vertices (in the order tetrahedron, octahedron, cube, icosahedron, dodecahedron).

  Neither new `source` touches the page's table, so neither decides whether a table row is a
  verbatim span. The vertices sentence gives its numbers in **face** order and states the **vertex**
  order separately, so each row is an inference the sentence licenses outright rather than a per-row
  verbatim claim; every row names the position it reads.

  **A claim this installment does not make.** 4g's five repairs are byte-exact against RAW HTML.
  These two are not and cannot be: the page's own bytes carry three newlines inside the vertices
  sentence and two inside the edges sentence, because the HTML source wraps the single paragraph
  they share. They are verbatim under the extractor's whitespace collapse — the standard every
  currently-verbatim value in this stdlib already meets — and that is **weaker** than 4g's claim.
  The two look alike and are not; the general question is filed on #14111.

  ### Two values that were not table dumps at all

  `genetic-code` and `start-codon` were filed as "table dumps", which is shorthand for a lost cause.
  NCBI serves the genetic-code table inside a `<pre>` that indents **line 1 by four spaces and every
  later line by two**, so all five data columns land at offset 11. The shipped values had all 12 of
  those characters stripped. So neither is a dump of table cells: each is a genuine contiguous
  multi-line span with its indentation normalised away — 4f's NBSP defect, not 4g's weld.

  Both headers **already claimed byte-fidelity**. `start-codon` said the value `reproduces,
  byte-for-byte, the SAME NCBI "Genetic Codes" page`; `genetic-code` called its field a "5-line
  block copied VERBATIM". Both were false, and had been for as long as they existed, because **no
  instrument here compares a value to raw HTML** — the sweep compares against the extractor's
  output, which normalises, and the CI pins compare the engine's output to the field, which is the
  field to itself.

  Restoring the whitespace decided no held question: whitespace inside a `<pre>` is **rendered**
  whitespace, so carrying it is the same call as carrying a page's U+00A0. That is *not* the case
  filed on #14111 the same day, where newlines appear inside an ordinary wrapped paragraph — source
  formatting that renders as one space, and the owner's call. The two look identical in a diff and
  are opposite in kind.

  ### The repair shipped a fresh instance of its own defect, for the third installment running

  The first attempt restored **8** of the 12 stripped characters and left line 1 flush. Measured
  data-column offsets:

  ```
  the page          11, 11, 11, 11, 11   aligned
  before 4h          7,  9,  9,  9,  9   off by 2
  the first attempt  7, 11, 11, 11, 11   OFF BY 4
  ```

  **It made the misalignment worse**, in the change whose entire subject is that block's whitespace.
  And unlike 4f's and 4g's versions of this, the damage was not confined to prose: `genetic-code`'s
  header tells the reader to decode **by column**, and decoding `atg` that way yielded `T` from the
  shipped bytes where the page gives `M`. Security review caught it. The four spaces are now
  restored, the columns are at 11, and the pins were **regenerated from the corrected fields**
  rather than patched.

  The prose describing the fix was wrong in the same direction — "false by eight characters" was 12,
  and *"indents every line after the first by TWO SPACES"* is precisely the misreading that produced
  the incomplete repair. Also corrected: "five newlines" in the MathWorld note is
  **three** (I had counted a slice that ran past the sentence).

  ### The headline number understates this round

  The sweep still calls both NCBI values NOT CONTIGUOUS, because `extract_v4.blocks()` splits on
  newlines and a five-line `<pre>` span is five blocks to it. **They are byte-exact against the page
  and still fail the screen.** Contiguity therefore moves 60 → 58, not 56 — and the two repairs the
  headline cannot see are the two verified most strongly, against raw HTML. **Round 7 sharpened what
  "most strongly" means here, and the first version of this sentence overclaimed.** A substring test
  with a one-character negative control does NOT discriminate: the first attempt at the indent
  repair was *also* a byte-exact, unique substring of the page, differing only in that its span
  began four characters into line 1. **Three** properties separate them, measured rather than
  argued: the match begins at a line boundary, all five data columns land at one offset, and `atg`
  decodes to `M` on the `AAs` line. Substring and uniqueness hold for the defective value too and
  separate nothing. Round 7 credited uniqueness with power it lacks; round 8 corrected that by
  naming anchoring as the *only* separator, denying power to two arms that have it — the same error
  in opposite directions, one round apart. Both are checked by `tools/verify_ncbi_pre_span.py`,
  committed in this package so the claim is reproducible rather than reported (#14444). A number is
  not a finding; this one is an instrument limitation wearing a number's clothes — and a check can
  be one too.

  Four pins added, because **none of these four values was pinned at all**. Eight mutants killed,
  not four: each NCBI value also gets an INDENT mutant that strips only the eight interior spaces on
  lines 2-5, and an ANCHOR mutant that removes only line 1's four — the shape of the defect review
  caught, which no mutant covered until round 8. Each of those two strips exactly the characters
  whose absence defines one defect and nothing else, which is the point: a whole-value mutation
  would redden the pin too, but by accident rather than on purpose, and it could not tell the two
  failure shapes apart.

  ### What is now held, and one correction to my own grouping

  `chemistry/atomic-weights:73` was investigated and is **held**, correcting this effort's earlier
  grouping of it with the two NCBI values. CIAAW's page has no tabs anywhere in the table — its
  cells are fused (`6Ccarbon` + U+00A0 + `12.011 ± 0.002`) — the value invents tabs between them,
  and the two element rows it welds **are not adjacent**: nitrogen sits between carbon and oxygen.
  That is the invented-separator table-row class 4f established belongs to the owner, not a stripped
  indent.

  Also this round, and not shipped in any installment: the census's controls now name their subject
  by **content** rather than by line number. Its own comment recorded that a control had gone stale
  by naming a coordinate five installments running, and 4g hit the same class from the other side
  with a line-keyed set-diff. A fragment that matches nothing — or two things — now raises. It paid
  immediately: one 4g value sits a line lower than a hardcoded coordinate would have named.


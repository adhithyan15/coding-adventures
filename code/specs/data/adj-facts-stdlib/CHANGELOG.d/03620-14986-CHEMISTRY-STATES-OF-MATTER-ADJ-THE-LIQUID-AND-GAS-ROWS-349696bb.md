- **#14986: `chemistry/states-of-matter.adj` -- the liquid and gas rows stop being warranted by the solid sentence.**
  The envelope was the `solid` span, so asking what a liquid or a gas does in a container returned its property
  evidenced by *"A solid holds its shape and the volume of a solid is fixed by the shape of the solid."* — a
  sentence about a different state — while those two rows' own sentences reached no answer at all.

  ### What each row carries now

  Measured 2026-09-16 UTC against the NASA Glenn Research Center K-12 "State of Matter" page at the `locator`,
  tags removed without inserting a space, only ASCII whitespace collapsed. This is a text measurement, not a
  byte-provenance check — see the ceiling note below.
  - The envelope and all three row spans occur **exactly once** on the page and **zero** times against a real
    404 control on the same host.
  - Every shipped span is **pure ASCII**; the page carries no non-ASCII on any line this table quotes.
  - Each row span names its own state and states the container behaviour its atom compresses.

  ### The liquid row was quoted with an ellipsis, and that is the substantive fix

  The header quoted the liquid span as *"A liquid will take the shape of its container ... a liquid has a
  fixed volume"*. On the page those are **two non-adjacent sentences**, separated by a third:

  > In microgravity, a liquid forms a ball inside a free surface.

  An elided quotation is not verbatim. Shipping it as a row `source` would have put a citation on a row that
  cannot be found on its own page — the #15354 shape in another costume. The row now ships the **contiguous**
  sentence as its `source`:

  > A liquid will take the shape of its container with a free surface in a gravitational field.

  That sentence supports `takes_shape_of_container` and says nothing about volume, so the fixed-volume half
  rides as the row's own `cites`:

  > Regardless of gravity, a liquid has a fixed volume.

  Both were extracted from the page's bytes, never retyped, and each occurs exactly once on it.

  **ADJ-A9 requires a `cites` to carry its own `locator`.** The first attempt emitted a bare one and the file
  stopped parsing (`Expected "locator", got "}"`), which also broke the cross-directory dependent that imports
  this table — three tests red. Counted across the corpus afterwards: **209 `cites` in all, 87 of them inside a
  `row` block after this change, and zero bare ones anywhere**. I had asserted the opposite from reading
  `lower.rs` alone — but lowering accepting the *variant* says nothing about what the *parser* requires to
  build it (`ast.rs:984`; the requirement is enforced by the adapter's missing-child error on the second
  STRING, not by the comment line above it). The converter now runs the built CLI over its own output before
  writing.

  *Two counts were published for this before the right one: I first said 85, and a review said 86. Both were
  needle artifacts — 85 came from an indent-anchored grep, 86 from a different block rule. 87 is a count of
  `cites` lines inside `row (…) {` blocks across all 723 files, after this change.*

  ### What this table claims, and its ceiling

  Nothing in it carries a `quote … at <byte_offset> snapshot "<sha256>"` annotation — the repo's
  byte-provenance mechanism. The spans are labelled with the page they were read from and nothing more, so
  **this table claims no more than `source_labeled`**, and the header now says so. Wording implying byte-level
  extraction ("extracted from the page's bytes") has been narrowed to what was done: read out of the fetched
  page rather than retyped.

  ### The envelope must frame matter and name no state

  The new envelope is the page's framing sentence, *"We call this property of matter the phase of the
  matter."* The shape arm forbids `solid`, `liquid`, `gas` **and `plasma`** — the fourth state this table
  deliberately excludes, so a frame naming it would frame a member of the taxonomy while naming no row key.

  Forbidding is not framing, so the arm also **requires** the envelope to name `matter`: this page's own
  navigation strings ("+ Text Only Site + Non-Flash Version + Contact Glenn") and off-subject prose
  ("Hydrogen has 1 proton and 1 electron.") name no state and frame nothing.

  ### Pins

  - **Inverted:** `contains("grc.nasa.gov") && contains("\"trust\":\"authoritative\"")` — satisfied by any
    NASA citation, constraining no sentence text, and before this change satisfied by the solid span riding on
    the gas answer. Each answer is now pinned to its own whole serialised citation object (#14735).
  - **`liquid` was never queried by this test.** That is precisely why the stitched quotation went unnoticed.
    It is queried now, by the test and by the query companion.
  - **Added:** whole-span negative arms (the three sentences share "shape", "volume", "container", so a short
    needle matches more than one); a row-header-anchored span check; a span-supports-its-row arm; and an arm
    pinning the liquid corroboration as **non-empty**.
  - **Fixed a race I introduced:** two tests both called the scratch helper with the same tags from loops, so
    they derived identical temp paths in one process and could delete each other's files. Scratch directories
    are now unique per call. An intermittent test is worse than a failing one.

  ### The query example, reported as it runs

  **4** queries, **3** answers, **1** abstention (`plasma`); **3** citations arrays, envelope wording in
  **zero** answers, each span **2×** (one answer across the two rendering surfaces), **4** empty and
  **2 non-empty** corroboration occurrences.

  **The 2 is deliberate and breaks this batch's pattern.** Every previous conversion reported *0 non-empty
  corroborations*, because relocating a table-level `cites` to its row emptied the array. Here the liquid row
  **gains** one: real page text that corroborates the atom without stating it, carried as `cites` rather than
  welded into the source. Reusing the previous shard's sentence would have been false.

  ### Downstream

  `geology/earth-layer-matter-behavior.adj` imports this table and its rule hard-codes the old envelope as its
  own `source`. Corpus scan of 723 files: **exactly two** carry that sentence — this table and that one. Its
  three tests pass unchanged, because its citation pin is host-only (`pubs.usgs.gov` + `grc.nasa.gov`) and
  cannot see which sentence is carried. Tracked as **#15359** rather than bundled here: a rule that derives a
  behaviour for *any* layer should not cite the sentence about **solids** as its warrant.

  *An earlier draft of this entry said the defect was "filed separately" when no issue existed — a claim the
  reader could not check. A security review caught it; the issue is now real and cited by number.*

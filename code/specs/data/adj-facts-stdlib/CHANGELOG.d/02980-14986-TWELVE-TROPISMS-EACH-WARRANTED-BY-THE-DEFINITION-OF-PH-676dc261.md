- **#14986: twelve tropisms, each warranted by the definition of phototropism.**
  `biology/plant-tropisms.adj` carried the PHOTOTROPISM line as its envelope, so a recall of
  `traumatotropism` came back proved by *"Phototropism: movement or growth in response to lights or
  colors of light"*. Each row now carries its own definition line; the envelope carries the page's
  definition of a **tropism**, which warrants no row. One page, so no row restates `locator` or
  `trust`.

  ### Is a "Term: definition" line one element, or a weld?

  Checked before anything was built on it, because that shape is usually a `<dt>`/`<dd>` pair that an
  extractor has joined — which would make it the held #13934 question, as it did for
  `geography/landforms.adj` (a thesaurus: term line, definition line) and `language/idiom-meaning.adj`
  (numbered heading, meaning beneath). Here it is **not**: each definition is ONE rendered line, a
  single block element with inline markup inside it.

  A raw-HTML grep for *"lights or colors"* finds **zero** occurrences, which looked alarming for
  about a minute. The markup sits between the words; the zero was the instrument.

  ### What the page's own wording forced

  - **Two variant heads**, kept as written: *"Gravitropism (sometimes referred to as geotropism): is
    movement…"* and *"Electrotropism, or galvanotropism: the movement…"*. A probe that required a bare
    `Gravitropism:` head reported both rows as having **no definition line at all** — the probe's
    prefix test, not the page. A mutant that normalises the head to the bare form reddens.

    For electrotropism the probe was not the only problem. **The header's shipped quote was
    non-verbatim**: it read *"Electrotropism/Galvanotropism: the movement…"*, a form that occurs zero
    times on the page, raw or rendered, under a header promising "the verbatim span that states it".
    So this row is a repair, not just a move. The header's claim that those seven rows were
    "WebFetch-verified, including a targeted second pass re-fetching all seven new terms' raw
    definition text directly" is removed rather than restated — that pass is what produced the quote.
  - **Reference markers trimmed.** The chemotropism line ends *"…in response to chemicals[8]"*; every
    span stops at the last word before the first bracket. That leaves a verbatim PREFIX of the page's
    own line — nothing reworded, and what is dropped is a footnote marker, never content. A mutant
    that restores the bracket reddens.

  ### Pins

  **19 of 19 mutants killed, one control.** All twelve rows broken one at a time; warranting
  `traumatotropism` with the phototropism line; restoring the `[8]`; normalising the gravitropism
  head; rebinding a stimulus; dropping a row's source so it inherits the envelope; repointing the
  locator. The control is the unmutated file.

  ### The envelope gap every prior entry disclosed is closed here

  Six entries in this cascade have said the same thing: once every row overrides `source`, the
  envelope's wording is unreachable from any answer, so fabricating it leaves the suite green —
  disclosed rather than implied. True of the **output**, and needlessly true of the **file**. The
  envelope test now reads the shipped `.adj` and asserts the span literally, so a drift from the page
  is a failure rather than a disclosed gap. That mutant is the 19th kill, not a control.

  **And the `trust` gap does NOT apply here** — a sentence claiming it did was written before it was
  checked. This table declares `consensus`, which is not `lower.rs:2622`'s `Authoritative` default, so
  **deleting the envelope's `trust` line reddens all six tests**. Inheritance is pinned here, not
  assumed. Measured both ways: deleting reddens, and flipping the tier to `authoritative` reddens.

  ### Three of my own errors, each caught by a guard rather than by care

  1. **The substring anchor, again.** The converter replaced the envelope using a 4-space anchor,
     which is a substring of the 8-space row line it had just written — so the framing span landed
     inside the PHOTOTROPISM ROW. The `rainforest-layer` entry below records this exact bug
     ("matched as a **substring** … Line anchored now"). Now anchored on the preceding newline.
  2. **A span retyped instead of copied.** The hydrotropism needle in the test was typed from memory
     and drifted from the shipped line; the test failed, and the fix takes every needle from the
     `.adj` itself. The converter's own docstring warns against exactly this.
  3. **A no-op mutant.** The per-row mutation replaced the word "response", which the aerotropism
     line does not contain (*"Aerotropism: the growth of plants towards or away from a source of
     wind"*) — so that mutant would have changed nothing and scored a free kill. The harness's
     no-op assert caught it; mutations now alter the final word, whatever it is.


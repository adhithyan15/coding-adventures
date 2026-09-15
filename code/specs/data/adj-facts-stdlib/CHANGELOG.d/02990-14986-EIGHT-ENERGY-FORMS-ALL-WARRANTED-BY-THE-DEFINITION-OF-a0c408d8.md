- **#14986: eight energy forms, all warranted by the definition of chemical energy.**
  `physics/energy-forms.adj` carried the CHEMICAL sentence as its envelope, so a recall of
  `electrical` came back proved by *"Chemical energy is energy stored in the bonds of atoms and
  molecules."* Each row now carries its own defining sentence. One page, so no row restates `locator`
  or `trust`.

  ### The same page, and the opposite answer for its sibling

  `physics/energy-form-family.adj` cites this exact page and is **not** convertible: there the
  potential/kinetic grouping lives in a SECTION HEADING, and no sentence assigns a form to a family
  (measured and recorded on #15139, which is why that table was withdrawn from the convertible
  queue). This table asks what each form **is**, which the page states in prose, one sentence per
  form.

  Worth stating because the earlier triage kept trying to classify by page or by envelope shape: the
  question is per **table**, not per page. The same article can ground one relation in prose and leave
  another to a heading.

  ### One variant head

  *"Thermal energy, or heat, is the energy that comes from atoms and molecules moving in a
  substance."* — not *"Thermal energy is…"*. A probe requiring the plain head reported the row as
  having no defining sentence at all, the same prefix-test failure as `gravitropism` and
  `electrotropism` in #15176. A mutant that normalises the head reddens.

  ### Pins

  **14 of 14 mutants killed, one control** (the unmutated file): all eight rows broken one at a time,
  warranting `electrical` with the chemical sentence, normalising the thermal head, rebinding a token,
  dropping a row's source, repointing the locator, and **fabricating the envelope** — which is a kill
  here rather than a disclosed gap, because the envelope test reads the shipped `.adj` and asserts the
  span literally, as #15176 established.

  Measured: all eight spans and the framing sentence occur **exactly once** on the cached page, each
  row span names its form and contains its token, near-miss controls score zero.

  ### One abstention that is not honest, disclosed rather than left

  Security review found it: the page names **nine** forms and this table has eight. `sound` abstains
  even though the same page defines it, once, in prose — *"Sound is energy moving through substances
  in longitudinal (compression or rarefaction) waves."* That is a false negative against the table's
  own source, and the file's header presented its abstention behaviour as honest without saying so.
  The sibling `energy-form-family.adj` already discloses the same gap.

  The header now states it. **Adding the row is not folded in**: it changes what this table claims,
  which does not belong in a provenance conversion, so it is filed instead.

  Also from review: a pre-existing `out.contains("eia.gov")` would have accepted
  `www.eia.gov.evil.example`; it now pins the full locator, which the row assertions already did. The suite was run
  with `RUSTFLAGS="-Dwarnings"`, which is how CI compiles it — a local `cargo test` does not, and
  #15176 failed five jobs on a warning that a plain local run had reported as green.


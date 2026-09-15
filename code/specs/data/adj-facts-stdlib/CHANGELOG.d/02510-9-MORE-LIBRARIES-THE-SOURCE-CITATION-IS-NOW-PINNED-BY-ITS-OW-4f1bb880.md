- **9 more libraries: the `source` citation is now pinned by its own e2e test.** These are the
  fragment-only libraries the previous batch (#13922) deliberately refused, because each is loaded by
  TWO OR THREE tests — siblings import them as dependencies — and guessing which test should carry the
  pin is exactly how a wrong citation landed in `facts_statesofmatter_e2e.rs`. Test-side only; no
  library content changed.

  The disambiguation is principled rather than convenient, and was VERIFIED rather than assumed: every
  one of these libraries has its OWN dedicated test among its owners, so the pin goes there. The other
  owners import it as a dependency and are not responsible for its provenance.

  *** A PIN A SIBLING CAN SATISFY IS NOT A PIN. *** Measured on `main`: 97 of 362 libraries (27%) share
  a `source` citation BYTE-IDENTICALLY with a sibling, across 45 duplicated sentences, and 19 are
  CO-LOADED with their twin in the same test. For those, an anchored assertion is satisfied by either
  library — truncate this one's citation and the sibling's copy still makes the test pass. That is the
  fragment-pin failure one level up: an assertion that cannot distinguish the passing case from the
  failing case. So the predicate now has three clauses — anchored, owner-scoped, AND unique among
  co-loaded libraries.

  That clause is not decorative: it removed a library from this batch. `civics/chamber-branch.adj` is
  skipped, because its citation is byte-identical to `civics/congress-chamber.adj` and its own test
  loads both. It needs a different assertion — a row-and-citation joint binding — and is left for
  bespoke work rather than given a pin that would look like coverage without being it. The batch is 9,
  not 10.

  OWNERSHIP IS NOW PATH-KEYED, NOT BASENAME-KEYED. A basename-keyed map collapses
  `chemistry/states-of-matter.adj` and `physics/states-of-matter.adj` into one entry and silently drops
  the other — the same collision that produced the wrong citation in #13922, and which reappeared TWICE
  MORE in the census tooling written to measure the damage. Tests name libraries three ways and all
  three are now resolved: a domain-qualified literal, a `<domain>.join("file.adj")` receiver, and a bare
  basename accepted ONLY when that basename is globally unique.

  Falsifiability verified by mutation on five of the nine, two mutations each: truncating the citation
  at its old fragment point, and appending text the source never says. All ten mutations fail their
  suite. The harness mutates the `source "` LINE specifically and PRINTS the line it changed — most
  `.adj` files also quote their citation in the literate `%` header, so a naive first-occurrence
  replacement edits a comment, the test correctly still passes, and the run reads as "the pin does not
  work" when the opposite is true. A falsification harness that silently mutates the wrong text reports
  the OPPOSITE of the truth.

  533 test binaries / 1592 tests green, clippy -D warnings clean. After this, 57 + 9 = 66 libraries
  carry an anchored pin, 1 remains fragment-only (`chamber-branch`), and 291 have no citation assertion
  at all. See issues #13916 and #13918.


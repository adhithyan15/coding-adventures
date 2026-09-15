- `physics/energy-form-family.adj` — **eight rows that carried no span of their own now cite the
  page's own definition of their form**, and the table is deliberately *not* converted to per-row
  `source` (#14986). The measurement is the reason.

  **The page states no row's family in a sentence.** The value column is `potential` / `kinetic`,
  and the cited EIA page assigns those by **structure**: a "Potential energy" heading with four
  forms listed under it, a "Kinetic energy" heading with the rest. Measured 2026-09-14 (HTTP 200,
  75,012 chars), with the envelope sentence found verbatim as a positive control and a fabricated
  sentence reported absent as a negative one: for **seven of the eight** forms there is **no
  sentence naming both the form and a family**. The eighth, `motion`, is a false positive — *"Kinetic
  energy is the motion of waves, electrons, atoms, molecules, substances, and objects."* defines
  kinetic energy and happens to contain the word "motion"; it does not say motion energy is kinetic.

  A per-row `source` asserts that the span warrants the row. None of these warrants a family, so
  attaching one would claim more than was measured — the same block `earth-science/water-cycle.adj`
  carries, and list-under-a-heading is #13934's held question besides.

  **What is fixed, needing no undecided machinery:** each row gains a row-level `cites` holding the
  page's own definition. Before this, seven of the eight rows carried **no span of their own at
  all** — a query about `chemical` came back with only the two-family sentence; it now also carries
  *"Chemical energy is energy stored in the bonds of atoms and molecules."*

  Each of the eight occurs **exactly once** on the page, each inside a `<p>`, none names another
  row's form, and — the check that keeps them honestly `cites` — **none mentions `potential` or
  `kinetic`**. That last property is asserted, not explained: if a future edit swaps in a span that
  *does* state a family, the suite fails and says to reconsider conversion, rather than quietly
  allowing it. The envelope is unchanged and was already right: it names no row key.

  The old citation assertion — `contains("eia.gov") && contains("\"trust\":\"authoritative\"")` — was
  the #15139 two-loose-needles shape, now one contiguous span here. **That shape is not rare and
  calling this "the third found" would mislead:** measured over `code/packages/rust/adj-lang-cli/
  tests/`, with the needle *a single assert of* `contains(X) && contains("\"trust\":…)`, **321 of
  the test files carry it, 323 occurrences**. This change fixes one of them. In `circuit-parts`
  that exact shape let a locator naming a file which never existed be swapped for an archive URL
  with no test noticing, because the host string is a substring of both. 10 of 10 mutants killed, green baseline before and after, file verified
  byte-identical afterwards. Local scratch harness, so that count is not reproducible from the repo.


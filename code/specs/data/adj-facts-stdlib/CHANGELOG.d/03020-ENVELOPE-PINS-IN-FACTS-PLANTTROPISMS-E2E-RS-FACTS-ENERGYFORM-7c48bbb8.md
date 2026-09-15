- Envelope pins in `facts_planttropisms_e2e.rs`, `facts_energyforms_e2e.rs` and
  `facts_speleothemaltname_e2e.rs` now run through the envelope's `locator` VALUE and its
  `trust` tier, where they stopped at `\n    locator` — asserting that a locator follows the
  envelope source and never what that locator is. Security review found the short form on
  `anatomy/brain-parts.adj` (#15181); these three entries are its siblings, and each of them
  said it had "closed the envelope gap".

  WHAT WAS ACTUALLY MEASURED, because the first reading of that finding was wrong and is
  worth recording as such. The obvious mutant — repoint the envelope locator at an unrelated
  shipped page — is **KILLED by the pins these three already shipped**, on all three tables,
  along with a tier downgrade. It is killed for a reason none of the three states: no row in
  any of them overrides `locator` or `trust`, so both envelope fields are inherited by every
  row (12, 8 and 8) and reach every answer, where the existing per-row citation assertions
  catch a repoint. `brain-parts` is the one table of the four where all fifteen rows supply
  their own locator — which is exactly why its envelope locator reached no answer and went
  unpinned there. The defect was real in the table review found it in, and was not live here.

  So the mutant that shows these pins are load-bearing is a two-step one, and it is the shape
  this cascade is moving every table toward: give each row the envelope's current locator (a
  pure refactor — byte-identical output, and it is run as a control, green in both arms), then
  repoint the envelope. SURVIVES the shipped pin, KILLS the repaired one, 3 of 3 tables. Nine
  single-step mutants across the three tables (repoint, downgrade, and a fabricate-the-source
  negative arm) kill under both, and are recorded here as what they are: not evidence for this
  change.

  No `.adj` content changed — this is test-side only, and the tiers, locators and spans it
  pins are the ones already shipped.


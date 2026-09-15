- `astronomy/dwarf-planet-criterion-status.adj` (new) — a sibling to the already-shipped
  `planet-criterion.adj` (`planet_criterion(criterion, requirement)`, the three IAU requirements a
  body must meet to count as a full planet). That table's own header already quotes, verbatim, a NASA
  sentence used only to justify EXCLUDING `dwarf_planet` as a row: "Dwarf planets like Pluto were
  defined as objects that orbit the Sun, and are nearly round, but have not been able to clear their
  orbit of debris." New `dwarf_planet_criterion_status(criterion, status)` table decodes that same
  sentence's per-criterion status instead: orbit → met, roundness → met, cleared_orbit → not_met. Full
  3/3 coverage, no abstention needed among the three criteria — the sentence's own leading/trailing
  clause structure fixes each status. New e2e test file `facts_dwarfplanetcriterionstatus_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the not-met criterion, honest abstention
  on a non-criterion atom). No manifest objective, matching `planet-criterion.adj`'s own precedent.
  Second slice from the fresh astronomy/ domain sweep begun after physics/ was fully exhausted.

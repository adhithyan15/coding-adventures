- `earth-science/speleothem-alt-name.adj` (new) — a `table` recording the other names a cave
  formation goes by: `speleothem_alt_name(speleothem, alt_name)`, eight rows across four grounding
  sentences. `column` → `pillar`; `coralloid` → `corallite`; `frozen_waterfall` →
  `petrified_waterfall`, `cascades`, `rivers`, `glaciers`, `organ_pipes` (five names licensed by one
  sentence); and `drapery` → `cave_bacon_when_characteristic_layers_present`. SEVENTH cave/karst
  library and the first on the naming axis, same shape as `astronomy/space-rock-alt-name.adj`, which
  reads its pairs from the same kind of apposition. The BACKWARD direction is the one that matters:
  a reader who meets "organ pipes" on a cave tour needs to resolve it back to the frozen waterfall.

  THE HEADLINE ABSTENTION IS `cave_popcorn`, AND THE REASON IS THAT THE SOURCE CONTRADICTS ITSELF.
  The page offers it as a synonym — "Coralloid (or corallite or cave popcorn) is a catchall term
  describing knobby, nodular, botryoidal, or corallike speleothems." — and then makes it a MEMBER of
  the category instead: "Coralloids include cave popcorn, grapes, knobstone, coral, cauliflower,
  globularites, and grapefruit." A thing cannot be both another name for coralloids and one of
  several kinds of coralloid. `corallite` ships because it appears only in the parenthetical and
  carries no such conflict. Picking whichever reading suited the table would be choosing an answer
  and then finding a citation for it, which is the exact failure this stdlib exists to prevent.
  `grapes` abstains for the adjacent reason: it appears only in the list of things coralloids
  INCLUDE, and membership is a different relation this table does not pretend to hold.

  Bare `cave_bacon` also abstains. The source makes that name CONDITIONAL — bacon "instead of
  drapery, when the characteristic layers are present" — so the condition rides inside the atom,
  following the placement rule `veto-override.adj` and `karst-process-zone.adj` already apply, and a
  query for the unconditional name finds nothing because asking for it is asking what a drapery is
  ALWAYS called. The source's own plurals are kept as atoms (`cascades`, `rivers`, `glaciers`);
  singularising them would be a silent edit of the citation.

  A LIMITATION OF THIS TABLE IN PARTICULAR, STATED IN ITS HEADER: provenance here is TABLE-level, so
  the `source` envelope and every `cites` corroboration cover every row. Most libraries in this
  stdlib are grounded in one or two sentences, where that is harmless; this one is grounded in four,
  covering four unrelated speleothems, so an answer about `column` also carries the coralloid,
  flowstone and bacon sentences. Nothing cited is false and the grounding sentence is always among
  them, but the reader must match sentence to row. IN `--explain` OUTPUT IT IS SHARPER: explain
  renders only the PRIMARY `source` and drops corroborations entirely, so the `corallite` row is
  displayed under the column/pillar sentence, which does not ground it. That is a property of every
  multi-sentence table in this stdlib rather than of this one — `heredity-term.adj` (6
  corroborations, 7 rows), `bill-stage-successor.adj` (6, 7) and `checks-and-balances.adj` (4, 5)
  all behave the same way — so it is filed rather than worked around here, since fixing it changes
  how explain renders provenance for a dozen shipped libraries at once. Until then the JSON output
  is the authoritative view. A single table cannot avoid this by splitting its envelope: two `table`
  blocks sharing a relation name are rejected as `DuplicateTable`, so one relation means one primary
  source. The alternative was four single-purpose tables,
  which would fragment one lookup into four and make the backward query impossible to ask in a
  single goal; one table with a stated limitation is the better trade. Row-level provenance is
  tracked as issue #13893.

  METHOD NOTE, worth recording because it changed the contents of this table: every sentence above
  was taken from a RAW TEXT EXTRACTION of the page rather than from a fetch summary. Summarised
  readings of this same page disagreed with each other about how many times "bacon" occurs and
  whether a given sentence existed at all, and none of them surfaced the `cave_popcorn`
  contradiction — on a summary's account, popcorn would have shipped as a row the page contradicts.

  New `speleothem-alt-name.query.adj` and `facts_speleothemaltname_e2e.rs` (6 tests: the five-name
  group with its verbatim sentence and citation, the reverse lookup in both directions, the bacon
  condition kept inside the atom with a negative assertion that the bare name is never stated, the
  unconditional-name abstention, the self-contradiction abstention with a negative assertion that
  popcorn never resolves to coralloid, and abstention on a singly-named formation and on a category
  member). Every negative assertion was mutation-tested: adding the refused rows makes all five
  abstentions bind. Abstention assertions require `"reason":"no_grounded_support"` rather than bare
  `"abstained":true`, and the abstain queries are written in VARIABLE form because a fully-bound
  query that matches nothing produces no recall entry at all. New manifest objective
  `adj.science.3to5.speleothem_alt_name`.


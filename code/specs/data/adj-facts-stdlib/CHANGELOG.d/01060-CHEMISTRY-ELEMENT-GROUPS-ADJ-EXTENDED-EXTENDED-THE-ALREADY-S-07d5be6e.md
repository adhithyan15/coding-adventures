- `chemistry/element-groups.adj` (extended) — extended the already-shipped
  `element_group_family(element, family)` table from 14 to 28 rows. Each of the five families'
  own already-cited Wikipedia sentence, quoted in full in the header since this table first
  shipped, named MORE member elements than had ever been turned into rows — only the first
  three of each family had been. Added rubidium, caesium, francium (alkali_metal); strontium,
  barium, radium (alkaline_earth_metal); iodine, astatine, tennessine (halogen); krypton,
  xenon, radon (noble_gas); cobalt (transition_metal) — the fourth extend-pattern shape
  (revisit an already-shipped table's own header for material named but never turned into
  rows/values), already proven on `pronoun-type.adj`/`contraction.adj`/`brain-parts.adj`, this
  time across FIVE keys in one table rather than one. Deliberately did NOT add `oganesson`
  even though the noble_gas sentence names it: the source hedges it as a noble gas only "in
  some cases" (its predicted chemistry is debated), so unlike every other name in the same
  sentences that clause does not support a firm row — a considered exclusion, not an
  oversight. The reverse-binding query on `noble_gas` now recalls all six shipped noble gases
  instead of three; the query file and new e2e test
  `chemistry_element_group_family_extension_recalls_newly_added_elements` were updated/added
  accordingly. No new manifest objective (same library, same objective, 168 total unchanged).

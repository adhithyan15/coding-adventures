- `chemistry/acids-bases.adj` (extended) — extended the already-shipped `acid_or_base(substance,
  classification)` table from 12 to 16 rows. This table's own header had ALWAYS explicitly stated
  that the source's acid column also lists HClO3 (chloric_acid) and its base column also lists
  RbOH (rubidium_hydroxide), CsOH (caesium_hydroxide), and Sr(OH)2 (strontium_hydroxide),
  explicitly noting they were "omitted only to keep the set balanced at six acids and six bases --
  nothing about them was uncertain." Since the header itself already confirmed these four are
  grounded in the same already-cited LibreTexts table, adding them is a pure addition sharing the
  existing acid/base classification -- the same header-revisit extend-pattern shape already proven
  on `element-groups.adj`. WebFetch re-verified the full source table live, TWO separate passes,
  both confirming all sixteen rows byte-identical. New e2e test
  `chemistry_acid_or_base_recalls_the_four_substances_added_this_cycle`. No new manifest objective
  (this library has never had one, matching the no-manifest precedent already established for
  sibling chemistry tables).

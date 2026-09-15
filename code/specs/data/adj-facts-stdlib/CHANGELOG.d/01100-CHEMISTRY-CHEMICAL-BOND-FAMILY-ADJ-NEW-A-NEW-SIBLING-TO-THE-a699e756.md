- `chemistry/chemical-bond-family.adj` (new) — a new sibling to the already-shipped
  `chemical-bonds.adj` (which recalls only each bond's single defining TOKEN): a new
  `bond_family(bond, family)` table names which of the source's two families -- PRIMARY (strong)
  or SECONDARY (weak) -- each of the four bond types falls into. Discovered via a header-revisit:
  `chemical-bonds.adj`'s own header already quotes the LibreTexts sentence classifying all four
  bonds ("Primary bonding (ionic, covalent and metallic) is strong ... In contrast, secondary
  bonding is weak ..."), but that table's header explicitly says "the family split is not part of
  this table" -- a deliberate single-axis scope decision, not a missing citation. Since the fact
  is cleanly grounded in the SAME already-cited sentence, it earns its own sibling table with a
  DIFFERENT schema, the same "sibling table, different schema" shape already used for
  `comet-part.adj`/`comet-tail-type.adj` and `lung-lobes.adj`/`lung-lobe-names.adj`.
  WebFetch re-verified the sentence live, TWO separate passes, both byte-identical. The
  reverse-binding query on `primary` is a genuine one-to-many recall (ionic ; covalent ;
  metallic). New e2e test `facts_bondfamily_e2e.rs`. No manifest objective added, matching
  `chemical-bonds.adj`'s own precedent (not wired into the loop's manifest coverage tracking).

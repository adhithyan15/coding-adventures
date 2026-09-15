- `anatomy/tooth-part-property.adj` (new) — a sibling to the already-shipped `tooth-parts.adj`
  (`tooth_part_role(part, role)`, dentin → beneath_enamel, cementum → covers_roots). That table's
  own already-quoted MedlinePlus/StatPearls spans also state a descriptive property for two
  parts -- a fact the part/role schema had no room for: "...dentin, a substance harder than
  bone." and "...cementum (calcified material covering the roots of teeth)". New
  `tooth_part_property(part, property)` table decodes those spans as their own rows:
  dentin → harder_than_bone, cementum → calcified_material. No new WebFetch -- reuses the same
  already-cited sentences. Honest abstention on `enamel`, `crown`, `pulp`, `root`, whose own
  quotes never supply a second, descriptive property beyond the role itself. New e2e test file
  `facts_toothpartproperty_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention). No manifest objective, matching `tooth-parts.adj`'s own precedent. LAST of
  the 3 MODERATE anatomy/ fallback candidates -- closes out the anatomy/ domain sweep entirely
  (both STRONG and MODERATE tiers now fully addressed). 108th content slice overall.

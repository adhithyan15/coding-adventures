- `anatomy/skin-layer-alt-name.adj` (new) — a sibling to the already-shipped `skin-layers.adj`
  (`skin_layer_property(layer, property)`) and `skin-layer-function.adj`
  (`skin_layer_function(layer, function)`). One of those tables' own already-quoted NCI SEER spans
  also names an everyday alternate name for a layer -- a fact neither the property nor the function
  schema had room for: "The subcutis is also known as the hypodermis or subcutaneous layer, and
  functions as both an insulator...". New `skin_layer_alt_name(layer, alt_name)` table decodes that
  span as its own row: subcutaneous → hypodermis. No new WebFetch -- reuses the same already-cited
  NCI SEER sentence. Honest abstention on `epidermis`, a real, already-tabled skin layer whose own
  quote never supplies an alternate name (likewise `dermis`). New e2e test file
  `facts_skinlayeraltname_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching the sibling tables' own precedent. First of the 3
  MODERATE anatomy/ fallback candidates, taken up after the STRONG candidate set was exhausted.
  106th content slice overall.

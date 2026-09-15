- `anatomy/skin-layer-function.adj` (new) — a sibling to the already-shipped `skin-layers.adj`
  (`skin_layer_property(layer, property)`, epidermis → outermost, dermis → thickest,
  subcutaneous → fat). That table maps each layer to a single positional/compositional
  descriptor, but two of that table's own already-quoted NCI SEER spans also state what the layer
  actually DOES — a fact the layer/property schema had no room for: "...protects the body from the
  environment" (epidermis) and "...functions as both an insulator...and as a shock-absorber..."
  (subcutaneous). New `skin_layer_function(layer, function)` table decodes those spans as their own
  rows: epidermis → protects_body, subcutaneous → insulator, subcutaneous → shock_absorber. No new
  WebFetch -- reuses the same already-cited NCI SEER sentences already quoted in the parent table's
  header. Honest abstention on `dermis`, a real, already-tabled skin layer whose own quote states
  only its location and thickness, never a function. New e2e test file
  `facts_skinlayerfunction_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `skin-layers.adj`'s own precedent. TENTH and LAST of
  the original STRONG anatomy/ domain-sweep candidates — closes out the STRONG set. 105th content
  slice overall.

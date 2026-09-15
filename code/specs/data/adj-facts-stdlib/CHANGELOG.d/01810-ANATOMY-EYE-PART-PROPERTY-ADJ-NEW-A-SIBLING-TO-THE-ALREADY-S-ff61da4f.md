- `anatomy/eye-part-property.adj` (new) — a sibling to the already-shipped `eye-parts.adj`
  (`eye_part_function(part, function)`, cornea/pupil/iris/lens/retina/optic_nerve →
  bends_light/lets_in_light/controls_light/focuses_light/turns_light_into_signals/
  carries_signals_to_brain). That table's own header already quotes, verbatim, per-row NEI spans
  naming each part's FUNCTION — but three of those same quoted spans also carry a parenthetical or
  descriptive clause naming a PROPERTY of the part (what it IS, not what it does), a fact the
  function schema had no room for: cornea → dome_shaped ("shaped like a dome"), iris →
  colored_part_of_eye ("the colored part of the eye"), retina → light_sensitive_layer ("a
  light-sensitive layer of tissue"). New `eye_part_property(part, property)` table decodes each as
  its own row. Honest abstention on lens, a real already-tabled eye part whose own quote states
  only a function, no descriptive property (same for pupil/optic_nerve, deliberately left unrowed).
  New e2e test file `facts_eyepartproperty_e2e.rs` (3 tests: forward recall with citation, backward
  recall, honest abstention). No manifest objective, matching `eye-parts.adj`'s own precedent.
  Second slice from the anatomy/ domain sweep. 97th content slice overall.

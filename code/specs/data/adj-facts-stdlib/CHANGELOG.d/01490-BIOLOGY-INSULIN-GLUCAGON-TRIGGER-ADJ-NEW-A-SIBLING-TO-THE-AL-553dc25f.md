- `biology/insulin-glucagon-trigger.adj` (new) — a sibling to the already-shipped
  `hormone-glands.adj` (`hormone_gland(hormone, gland)`, WHICH gland secretes a hormone —
  insulin/glucagon → pancreas, etc.). That table's own header already quotes, verbatim, the
  trigger condition for the pancreas's two glucose-regulating hormones, and buried in those two
  spans, alongside the gland the parent table already captures, is a second fact the
  hormone→gland schema had no room for: WHAT blood-glucose condition triggers each hormone's
  release. New `secretion_trigger(hormone, blood_glucose_level)` table: insulin → high, glucagon
  → low. Honestly narrow — abstains on every other hormone in `hormone-glands.adj`, since the
  source never ties them to a blood-glucose trigger. New e2e test file
  `facts_insulinglucagontrigger_e2e.rs` (3 tests: forward recall with citation, backward recall of
  the low-glucose hormone, honest abstention on a hormone with no glucose trigger). No manifest
  objective, matching `hormone-glands.adj`'s own precedent of not having one. Third slice from the
  fresh biology/ sweep tranche, queued after `muscle-striation.adj`.

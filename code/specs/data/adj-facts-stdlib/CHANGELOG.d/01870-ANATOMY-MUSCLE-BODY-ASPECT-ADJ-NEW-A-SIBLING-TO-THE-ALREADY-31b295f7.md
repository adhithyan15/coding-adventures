- `anatomy/muscle-body-aspect.adj` (new) — a sibling to the already-shipped `muscle-groups.adj`
  (`muscle_region(muscle, region)`, biceps_brachii/triceps_brachii → arm, rectus_abdominis →
  abdomen, sartorius/quadriceps → thigh, gastrocnemius → leg, etc.). That table's own header
  already quotes, verbatim, Wikipedia sentences that name each muscle's body region — but three of
  those same quoted spans also name the muscle's spatial aspect within that region (anterior/
  posterior compartment, or ventral aspect), a fact the muscle/region schema had no room for. New
  `muscle_body_aspect(muscle, aspect)` table decodes those three spans as their own rows:
  rectus_abdominis → ventral, sartorius → anterior, gastrocnemius → posterior. SCOPE NOTE: the
  originally sweep-flagged candidate proposed 6 rows (adding biceps_brachii → anterior,
  triceps_brachii → posterior, quadriceps → anterior), but on fresh verification those three
  muscles' own quoted spans use everyday words like "front"/"back" rather than the anatomical
  terms this table's atoms are built from — per this stdlib's own established precedent that an
  atom must echo the source's OWN chosen word, not a same-meaning paraphrase, those three are
  honest abstentions instead, cutting the candidate down to 3 verbatim-grounded rows. New e2e test
  file `facts_musclebodyaspect_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on quadriceps). No manifest objective, matching `muscle-groups.adj`'s own
  `trust consensus` precedent. Eighth slice from the anatomy/ domain sweep — steady at 176
  objectives. 103rd content slice overall.

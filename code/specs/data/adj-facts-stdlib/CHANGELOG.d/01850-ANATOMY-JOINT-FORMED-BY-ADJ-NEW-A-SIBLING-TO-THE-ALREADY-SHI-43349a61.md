- `anatomy/joint-formed-by.adj` (new) — a sibling to the already-shipped `joint-types.adj`
  (`joint_example(joint_type, example)`, hinge/pivot/condyloid/saddle/planar/ball_and_socket →
  a representative joint the source names for each shape, e.g. pivot → atlantoaxial). That table's
  own header already quotes, verbatim, three StatPearls sentences that each name a joint type's
  representative example joint AND, in the same sentence, the specific bones that meet to form
  that joint — a fact the joint_type/example schema had no room for. New `joint_formed_by
  (joint_type, bone)` table decodes those named bones as their own rows (6 total, across the three
  joint types whose quoted span names the forming bones): pivot → atlas, axis; condyloid →
  distal_metacarpals, proximal_phalanges; saddle → trapezium, first_metacarpal. Hinge, planar, and
  ball_and_socket are deliberately not rowed here, since their own quoted spans name only an
  example joint, never the bones that form it. Honest abstention on `hinge`, a real, already-tabled
  joint type whose own quote names no forming bones. New e2e test file `facts_jointformedby_e2e.rs`
  (3 tests: 2-answer forward recall with citation, backward recall, honest abstention). No manifest
  objective, matching `joint-types.adj`'s own precedent. Sixth slice from the anatomy/ domain sweep
  — steady at 176 objectives. 101st content slice overall.

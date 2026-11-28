# DB-E57 — Classify block-reclamation fixture fuzz results

The DB05c3b1 independent rerun found a well-shaped block fixture whose
declared result disagrees with the block model. The case
`multiple-incomplete-blocks-and-capacity` lists a one-page erase prefix
in step 7. The generated input changed the expected program marker
for physical page 2 from false to true. The model correctly refused it,
but the fuzz callback declared the refusal a failing test. The original
replay input has SHA-256
`7eaac4384b22f9697cff0e8cb3ee70acc815265fe7dc4edb2d29caa2e999e15e`.

For arbitrary fixture bytes, a shape or model refusal is an expected
outcome. A model-accepted fixture must replay against the real device.
Reuse the canonical test's runtime replay for this branch. Keep the
canonical fixture test strict on both shape and model agreement.

Add a deterministic regression that reaches the same well-shaped
incorrect-marker declaration and checks model refusal. Exercise the
wrong declaration in the fuzz seed corpus and preserve runtime replay
for accepted seeds. Demonstrate that a deliberate runtime disagreement
makes the model-accepted seed fail at the intended comparison.

Keep the three shared media documents and runtime source byte-identical.
Repeat the block fixture and history fuzz campaigns, replay the original
failure, and rerun the canonical suite, race and vet. DB05c3b1 remains
pending until this validation gate can be rerun cleanly.

### Added — full-mock structure manifests and measured deficits (#12213)

- Allow each level mock to declare a safe-relative JSON manifest, while keeping
  legacy omissions readable and measurable during migration.
- Validate the manifest's language, level, mock identity, four skill timings,
  and exact task-shape part set, including one selected published variant;
  bind each part to nonempty marked spans in the learner paper, rubric, and
  answer key. These are structural checks; human validation still owns quality.
- Require distinct, present learner paper, rubric, answer key, and structure
  manifest before a timed mock counts as complete in the completion plan.
- Add `mock-assessment/<language>/<level>` work items and a projection that
  keeps absent task-shape inventories explicit as prerequisites. Declared
  manifests cannot enlarge the grandfathered artifact ceiling.
- Test malformed part sets and identities, missing paper and manifest paths,
  and the real corpus's currently incomplete mock evidence. The required
  artifact CLI now refuses malformed declared manifests and incomplete paper
  bindings, without weakening its existing gates.

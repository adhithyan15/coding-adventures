### Fixed: reuse immutable corpus measurements and reject invalid mock levels before reads

- Measure script closure, metalanguage and root-slug splits once per test file; compute the clean planner comparison and valid mock CLI run before timed assertions. Preserve every live-corpus assertion and mutation test.
- Reject unknown Spanish mock levels before loading vocabulary or lessons, with a regression against an absent corpus.

### Fixed: reuse immutable corpus measurements and reject invalid mock levels before reads

- Measure script closure, metalanguage and root-slug splits once per test file; compute the clean planner comparison, modality manifest and its committed owners, and valid mock CLI run before timed assertions. Preserve every live-corpus assertion, live modality CLI check and mutation test. Use Windows junctions for the modality directory-link rejection fixture.
- Reject unknown Spanish mock levels before loading vocabulary or lessons, with a regression against an absent corpus.
- Build the modality manifest once per CLI invocation and reuse it for emitted owner bytes, preserving all byte, identity, reconstruction and recovery checks.
- Give the live full-corpus modality CLI regression a measured 120-second integration budget; ordinary test and hook limits remain 30 seconds.

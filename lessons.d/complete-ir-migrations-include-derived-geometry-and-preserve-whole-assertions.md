---
category: Rust
---

# Complete IR migrations include derived geometry and preserve whole assertions

The first current-main field migration covered semantic Wardley constructors in
integration tests, but omitted private test modules and derived layout fields.
The broad affected suite therefore failed compilation on fourteen paint fixture
constructors before tests ran. New path/arrowhead/outline and link label fields
also describe intentional geometry changes; fixed default placeholders cannot
establish complete expected layout or scene behavior.

Audit semantic and derived types across both integration and private test files.
Preserve whole-IR/scene equality and build new expected geometry from independently
declared source coordinates and incoming semantics, never by calling production
layout to populate expected values. Classify compilation failure separately from
the seven narrow source/typed controls that really compiled and passed.

For long validators, a zero-live-child snapshot is only a current observation.
Consume the producer and inspect its saved command results before reporting what
executed. The stopped broad preflight had completed metadata0, despite an earlier
empty-results observation; it had not executed tests or lint.

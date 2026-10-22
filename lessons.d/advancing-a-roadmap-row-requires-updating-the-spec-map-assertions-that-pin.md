---
category: Testing & coverage
---

# Advancing a roadmap row requires updating the spec-map assertions that pin its status

The Forme roadmap correctly moved FM-B073 from `active` to `done` and FM-B074
from `ready` to `active`, but `test_forme_spec_map.py` still required the old
literal rows. Package tests could not catch this repository-contract drift, so
the Linux release job failed before product validation. Whenever roadmap state
changes, search repository tests for the affected item IDs and run the complete
spec-map suite before pushing.

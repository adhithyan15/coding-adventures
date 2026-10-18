---
category: Testing & coverage
---

# Update boxed fixture contents before repeating full Rust validation

CLOC41 execution at49b57b79 passed all248 executable harnesses and14 doctest
targets, then failed the shared all-target strict check. Two new native-test
fixture branches replaced an existing Box with Box::new, triggering
clippy::replace_box under -D warnings. The aggregate remains UNACCEPTED;
the CLI strict command was not reached. The receipt and raw diagnostics stay
unchanged, and passing tests do not erase the lint failure.

Assign each replacement expression through the existing box instead. Preserve
the exact fixture values, IDs, cases, assertions and production bytes; verify
the two-edit inverse. Run both strict checks before repeating the complete
affected target set at the corrected committed head. Keep earlier execution
attribution separate from the successor. Do not suppress the lint or weaken
tests. Miri remains DEFERRED/PENDING; its frozen inputs are untouched.

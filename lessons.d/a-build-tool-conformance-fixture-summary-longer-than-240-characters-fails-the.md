---
category: CI & GitHub Actions
---

# a build-tool conformance fixture summary longer than 240 characters fails the repo-wide schema check, so run test_build_tool_conformance_schema after editing a case

**What went wrong.** #18791 rewrote the shared case
`code/specs/fixtures/build-tool-v1/cases/resolution-rust-field-aware.json` so it
pins every Cargo link-table rule. As the case grew, its `summary` was rewritten to
name every rule, ending at 262 characters. `code/specs/fixtures/build-tool-v1/schema.json`
caps `summary` at 240 (`"maxLength": 240`). Each implementation's own fixture
replay ignores `summary`, so all eleven build-tool suites passed locally. The
repo-wide "Repo-wide metadata contracts" CI step failed instead, in
`test_build_tool_conformance_schema.BuildToolConformanceSchemaTests.test_checked_in_examples_are_formally_valid`:
"... is too long".

**The fix.** Shorten the summary to 228 characters. The case's rules are pinned by its
manifest and expected edges, not by its prose.

**Do differently.** After editing any `build-tool-v1` case, run
`python3 -m unittest discover -s code/scripts/tests -p 'test_build_tool_conformance_schema.py'`
before pushing. Passing native readers prove the case's behaviour, not that it
matches its schema. Keep a summary to one clause per rule family, not one per rule.

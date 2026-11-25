# warrant-profile-algorithm-v1

Positive and negative cases for the Warrant **algorithm profile** header and the
`realizes` binding field ([VER33](../../VER33-warrant-algorithm-definition-language.md)
section 4, backlog ADL-001a).  One file per case; the expected outcome is the
first line.

| Files | First line | Driven through |
|---|---|---|
| `pos-*.warrant`, `neg-*.warrant` | `-- expect: ok` or `-- expect: W0xxx` | the guarded parser, then `check_warrant` |
| `bind-*.bindings` | `# expect: ok` or `# expect: W0xxx`, plus `# ops:` and `# model:` naming the documents | `parse_evidence_bindings`, then `validate_realizations` |
| `bind-ops*.warrant`, `bind-model*.warrant` | none (supporting documents) | loaded by the binding cases |

Every new diagnostic (`W0801` to `W0807`) has at least one negative case, and the
pure cases pin that a document without the clause is untouched.  The runner is
`code/packages/rust/warrant-grammar/tests/profile_algorithm.rs` (it also fails if a
code loses its last case).  Header *grammar* cases live in the
[syntax corpus](../warrant-syntax-v1/README.md).

This is diagnostics evidence, not generated-test evidence: nothing here is a
`warrant-corpus-v2` stream, and nothing yet uses a realization.

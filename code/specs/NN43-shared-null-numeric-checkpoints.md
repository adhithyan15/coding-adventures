# NN43: Shared Null Numeric Checkpoint Rejections

Status: implemented with direct local four-native-suite evidence; actual selected
required CI and confirmed exact-head merge remain pending.

## Prioritized reliability boundary

NN42 merged in PR #18230 at exact head `a34090527bbe`, squash `705defb19b2`,
after all applicable checks and actual twelve-direction native interchange
passed. Reprioritize reliability, prerequisites, portability and smallest complete
delivery: finish the observed shared null-cell gap before the next Ruby port.
Go's standard JSON decoder can silently map null float cells to zero; NN41
already guards and tests it locally. All four native frameworks must now earn
the same rejection decisions from one language-neutral fixture.

## Fixture-only contract extension

Extend the existing NN40 `checkpoints.json` parsed-document corpus without
changing its root schema, contract identifier or prior 49 documents. Add six
explicit rejected cases: `null-weight-cell`, `null-weight-cell-mixed`,
`null-bias-cell`, `null-bias-cell-mixed`, `null-weight-cell-later-layer`,
`null-bias-cell-later-layer`. Use otherwise valid, connected shapes and metadata,
including rectangular multi-coordinate and second-layer cases, so null is the
only defect. No executable mutation descriptions, new format fields, text-parser
duplicate-key/nesting promises, runtime paths, or training algorithm changes.
The corpus then has 55 cases: 3 accepted controls and 52 rejected documents.

## Native tests and evidence

Python, TypeScript, Rust and Go package suites load the identical fixture through
their public native loaders. Pin all six IDs, reject expectations, the total
inventory and accepted controls in every suite. Keep the complete original
corpus, numerical/finite-difference/ownership/atomicity tests and actual NN42
twelve-direction exchange. A loader rejecting everything must not pass.

Run native numerical/boundary/corpus tests, production builds, coverage, lint,
race checks where supported, examples/resume, links/lessons and diff checks.
Use the existing fixture-directory required-conformance selection rule; no new
workflow or gate semantics are needed. Local exact-copy minimal Rust validation
must identify its boundary; actual root Rust workspace CI remains mandatory.
Update lesson, curriculum, current corpus-count documentation and package
changelogs. Preserve historical NN40/NN41 delivery evidence as historical.
Independent security review precedes every push. Publish one ready PR and wait
for exact-head CI/CodeQL/selected conformance and verified merge before completion.

Local evidence: Python 123 tests and TypeScript 120 tests at 100% library
coverage, Go library 96.9%/CLI 84% plus race/vet/build and native resume, Rust
10 integration tests and README doctest/fmt/Clippy/release example in the
documented exact-copy workspace. Existing loaders reject every new case;
no production loader or training change was needed.

# Validation receipt

## October 25 recovery

Existing PR #18174 was replayed onto main
`d6ce5d38efeca48e0aa6d5366102c756b6d508ea` after the externally verified
PR #18147 merge. Original history and its exact published head
`3007c3fa8cae9bd4738afd2ab2e342e913b422e1` remain preserved locally. Only
the parity journal and roadmap conflicted. The validator, falsification tests,
official corpus, provenance, schemas and renderer vectors retain their original
Git blobs; the normative addition states the existing CI toolchain obligation.
The full private audit archive and unpublished GFM draft remain separate.

On Windows amd64 at runtime-source commit
`23e5bd02f04f8da7b462630b939006af03c39eca`:

- The direct validator accepts all 652 official examples and sixteen renderer
  cases. All fourteen falsification tests pass. Coverage measures all 327
  statements and 170 branches, with zero missing statements or partial branches
  (100% statement and branch coverage).
- All thirteen Venture-routing tests pass. Before the two setup predicate
  additions, the Venture-only truth-table case failed for both Node and npm.
  The independent review also falsified comment-only and other-job decoys and
  a changed npm pin. All nineteen CI-registry tests and ten parity-reporter
  tests pass. Ruff lint/format checks, compileall and lesson validation pass.
- The complete Go build-tool suite passes across all fifteen packages,
  including the real PowerShell registry-generator round trip, with no skip
  flag. Coverage is 84.6% overall and 62.5% for the main package; this is not full build-tool
  qualification. Vet and a temporary-output executable build pass. Its real
  unforced plan evaluates 45 Starlark BUILD files, discovers 5,384 packages and
  passes orphan/build-file checks with the existing 38 pending exemptions.
  It selects no native package for this specs/scripts/workflow change; the
  unconditional foundation validation is therefore essential.
- The full collision-checked package report has 1,559 identities and 4,846
  established-lane slots across fifteen languages, with zero collisions or
  unknown buckets. OCaml remains emerging with five packages. The public
  delivery journal and private operational graph are checked independently;
  each preserves complete, acyclic dependencies and one active implementation.
- A compiler built from this checkout with `cargo build -p mosaic-compile
  --offline` emitted actual Venture React and Electron projects. Each passes
  `npm install --ignore-scripts`, its one generated Vitest interaction test and
  its TypeScript/Vite production build with npm 11.13.0 and Node 24.16.0.
  Inherited long-PATH command lookup required a task-local short PATH, including
  the bundled `pwsh` for Go tests; no global environment or source workaround
  was added. Hosted Node 22 and the full native backend matrix remain CI work.
- Generated React's npm audit reports zero vulnerabilities. Generated Electron
  reports three vulnerable dependency packages: one high and two critical in
  `concurrently`, `electron` and `shell-quote`. The emitter/dependency source is
  unchanged from main; this observation is retained for separate dependency
  maintenance. No audit override, forced dependency update or test skip was
  introduced, and a clean Electron dependency audit is not claimed.

Independent review found no new authority, permission, credential, workflow
event, parser behavior or dependency change in the CI repair. Final-head hosted
checks and conflict-free exact-head auto-merge enrollment remain required; no
manual merge is authorized. The earlier publication receipt below is history,
including its former three-PR ceiling and former host-dependent test skip.

## Original publication receipt

Validated on 2026-10-24 from implementation revision `b2376c0e0d` with
always-run CI selection hardened at `6d7c4b2f17`.

## Normative artifacts

- `spec.json`: 140,487 bytes, 652 examples, SHA-256
  `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`.
- The bytes match the published
  `https://spec.commonmark.org/0.31.2/spec.json` artifact and all sixteen
  byte-identical package-local copies found in this checkout.
- The canonical semantic digest is
  `0153ea34b32feb95ffca06fd6c5b543673aa803aabfbef719192b6e8be122d57`.
- `renderer-cases.json`: sixteen cases, all nineteen TE00 node types, canonical
  semantic digest
  `fa211cec61ae33764ad388e1a6ee4f03b94c6c87665c0b426a5dbe3b1b711828`.
- All three schemas parse as JSON and are closed against additional object
  properties. The dependency-free validator enforces their stricter byte,
  cross-field, containment, and digest invariants.

## Focused verification

The following checks passed:

```text
python3 code/scripts/commonmark_conformance.py
  validated 652 CommonMark 0.31.2 examples and 16 renderer cases

find code/packages -type f -size 140487c -print0 | xargs -0 shasum -a 256
  16 package-local copies, all d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20

python3 -m unittest discover -s code/scripts/tests -p test_commonmark_conformance.py
  Ran 14 tests ... OK

python3 -m unittest discover -s code/scripts/tests -p test_ci_gate_registry.py
  Ran 19 tests ... OK

coverage run --branch -m unittest discover -s code/scripts/tests \
  -p test_commonmark_conformance.py
coverage report -m code/scripts/commonmark_conformance.py
  327 statements, 170 branches, 100% statement and branch coverage

ruff check code/scripts/commonmark_conformance.py \
  code/scripts/tests/test_commonmark_conformance.py
ruff format --check code/scripts/commonmark_conformance.py \
  code/scripts/tests/test_commonmark_conformance.py
python3 -m compileall -q code/scripts/commonmark_conformance.py \
  code/scripts/tests/test_commonmark_conformance.py
python3 code/scripts/lessons.py validate
git diff --check
```

The always-run `Repo-wide metadata contracts` CI job executes both the direct
validator and the complete dedicated unittest command. The external CI-registry
regression scopes itself to the unconditional `contracts` job, rejects job- or
step-level conditions and shell control flow, and requires each exact executable
line once. Package-diff planning that selects no native build therefore cannot
silently omit this foundation.

The Go build planner evaluated all 86 Starlark BUILD files, discovered 5,359 packages,
selected no native package build for this specs-and-scripts-only change, and
reported the orphan checks clean. A public-API cross-check built the TypeScript
`document-ast-to-html` package and matched all sixteen vectors exactly; the
generated `dist` directory was removed afterward.

The parity state graph contained 1,541 unique items, complete dependency
references, no cycle, and no merged item depending on unfinished work. The
operator-set active parity PR ceiling remained three.

## Security and dependency review

The production validator imports only Python standard-library modules. It does
not use the network, environment, subprocesses, plugins, browsers, or
fixture-selected paths. Reads stop one byte beyond explicit transport ceilings
before decoding or hashing. JSON rejects duplicate keys, ambiguous numeric
forms, byte-order marks, invalid UTF-8, excessive nesting, and non-scalar
Unicode. Direct decoded callers are bounded for depth, item count, aggregate
UTF-8 text, aliases, and cycles before semantic validation. Exact raw and
canonical semantic digests prevent a local fixture from normalizing drift into
acceptance.

AST validation closes fields and node variants, caps nodes, depth, children,
text, and expected output, enforces block/inline containment and list
consistency, rejects nested links, and requires complete TE00 node coverage.
Raw HTML remains inert fixture text. The validator performs no rendering or
execution. Manual review found no credential, filesystem-write, process,
network, deserialization-execution, path-traversal, or unbounded-work surface.
No third-party dependency was introduced; the optional `bandit` executable was
not installed in this environment.

## Scope

This receipt covers only the neutral fixture, schemas, validator, and tests. It
does not grant native parser or renderer conformance credit. Package reader
migrations and language-specific parity remain separate reviewed work items.

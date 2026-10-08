# CI Gate Registry — v1

## Overview

The **CI gate registry** is a checked-in, declarative description of which
GitHub Actions jobs a change actually needs. The build tool evaluates it during
the planning pass and emits one boolean per gated job, which
`.github/workflows/ci.yml` consumes as a job-level `if:` condition.

Its one job is to answer, per pull request: *does this change require this
check?* A Ruby compiled-grammar regeneration pipeline has nothing to say about a
human-languages curriculum PR, and should not run on one.

## Motivation

Before this registry, `ci.yml` carried eleven "always-on contract consumer"
jobs. None of them declared `needs:`, so they were scheduled *ahead of* the
`detect` job — the job whose entire purpose is to work out what needs to run.

Measured across 35 successful pull-request runs (2026-09-01):

| Quantity | Value |
| --- | --- |
| Mean wall-clock per run | ~91 min |
| Total execution across all 16 jobs | ~48 min |
| `detect` queue time before starting | 43 min (median) |
| `build` execution | 8.1 min median, 17 min p90 |

About **70% of pull-request wall-clock was queueing, not compute**. Each run
claimed 16 concurrent runner slots against a saturated account-wide ceiling, so
every unnecessary job lengthened the queue for every other run in flight. The
cheapest available speedup was therefore not to make jobs faster, but to stop
starting the ones a change does not need.

This is the same principle already recorded in `lessons.md`: *"intensive checks
belong on main, not PRs — fast PR iteration matters more than 100% per-PR
coverage."* The registry generalizes it from a case-by-case judgment into
something the planner enforces.

## Registry file

`code/specs/data/ci-gates.json`.

JSON, not YAML: the build-tool Go module has zero third-party dependencies and
no `go.sum`, and JSON is already the grain of every other machine-read artifact
in the repo (`build-plan.json`, `.build-cache.json`, the conformance corpus).

```json
{
  "schema_version": 1,
  "gates": {
    "<job-id>": {
      "description": "One line: what this job proves.",
      "packages": ["<language>/<name>", "<language>/programs/<name>"],
      "paths": ["code/fixtures/example/**", "code/scripts/example.py"]
    }
  }
}
```

- `<job-id>` MUST be the literal job key in `.github/workflows/ci.yml`.
- `packages` are qualified package names in the build tool's convention:
  `code/packages/<lang>/<dir>` becomes `<lang>/<dir>`, and
  `code/programs/<lang>/<dir>` becomes `<lang>/programs/<dir>`
  (`internal/discovery.inferPackageName`).
- `paths` are repo-root-relative globs matched with `internal/globmatch`, which
  supports `**` for "zero or more complete path segments".
- Both lists may be empty individually, but a gate with neither can never fire
  and is rejected at load time.

## Why every gate needs BOTH lists

This is the subtlety that makes a package-only registry wrong.

`internal/gitdiff.MapFilesToPackages` maps a changed file to a package only when
the file lies *under* that package's directory, and `sharedPrefixes` in
`main.go` is an empty slice. Consequently, changes under `code/specs/**`,
`code/fixtures/**`, `code/grammars/**`, and `code/scripts/**` map to **zero**
packages and never appear in `affected_packages`.

Nearly every gated job is a *staleness* check whose input lives in exactly those
trees:

- the D18F/D18P/D18Q/D18T jobs read manifests under `code/fixtures/`
- `ruby-grammar-regen-check` reads `.tokens`/`.grammar` sources under `code/grammars/`
- the PHY00/PHY01 Dart tests import fixtures by relative path *out of* their own
  package directory, into `code/specs/fixtures/phy00-phy01-v1/`

A package-only gate would skip the D18F job on a PR that changed only the D18F
manifest — precisely the drift that job exists to catch. The `paths` clause is
load-bearing, not decoration.

### Native consumers of a shared build-tool fixture

Scheduling the `contracts-build-tool-conformance` job for a shared fixture
change validates the neutral corpus but does not execute every native build-tool
consumer. For the exact path
`code/specs/fixtures/build-tool-v1/cases/discovery-language-registry.json`, the
build planner MUST seed all direct native consumers as changed package roots
before affected/prerequisite closure, both on its detect platform and in every
platform-specific build-plan override. The current direct consumers are
`dotnet/programs/build-tool-csharp`, `dotnet/programs/build-tool-fsharp`, and
`<language>/programs/build-tool` for `go`, `haskell`, `lua`, `perl`, `python`,
`ruby`, `rust`, `swift`, and `typescript`.

This is an exact fixture-to-consumer relation, not a general `code/specs/`
shared prefix or a forced full build. It applies to additions, modifications,
deletions, and renames of that path; ordinary changed package roots are united
with the fixture consumers. If a registered consumer is missing from the
discovered package set, planning MUST fail rather than silently omit its native
test. A new direct consumer of this fixture MUST extend the relation and its
drift test in the same change. The resulting affected plan and toolchain flags,
not merely the gate verdict, are the evidence that native tests can run.
For an explicitly single-language invocation, only consumers in that language
are seeded and checked; all-language CI retains the full twelve-consumer check.

The exact flat case family
`code/specs/fixtures/build-tool-v1/cases/ci-gate-selection-*.json` has a
separate direct-native-consumer relation: `go/programs/build-tool` (whose
`go test ./...` BUILD front runs `internal/cigates`) and
`python/programs/build-tool` (whose BUILD front runs the Python fixture
suite), plus `dotnet/programs/build-tool-csharp` and
`dotnet/programs/build-tool-fsharp` (whose native .NET test fronts replay the
neutral cases independently). Any changed path matching this family, including
the deleted source of a rename, MUST seed those four package roots before affected/prerequisite
closure on detect and every platform override. Ordinary changed roots are
united with these roots; sibling fixture domains and nested/lookalike paths
MUST NOT trigger them. Explicit C#, F#, Go, or Python single-language plans
seed only their own consumer; other single-language plans seed neither. An applicable
registered consumer absent from discovery MUST fail planning without writing
an incomplete plan. The emitted affected set and .NET/Go/Python toolchain flags
MUST demonstrate native scheduling with `force=false`. A new direct reader of
this family must extend the relation and its drift test together.

The exact flat case family
`code/specs/fixtures/build-tool-v1/cases/toolchain-detection-*.json` MUST
likewise seed every direct native toolchain-detection reader before affected
and prerequisite closure. Its current consumers are
`dotnet/programs/build-tool-csharp`, `dotnet/programs/build-tool-fsharp`, and
`<language>/programs/build-tool` for Elixir, Go, Haskell, Lua, Perl, Python,
Ruby, Rust, Swift, and TypeScript. The relation applies on detect and Linux,
macOS, and Windows build-plan overrides for added, modified, deleted, and
renamed fixture paths, including the deleted source of a rename. Only a flat
filename with a nonempty stem after `toolchain-detection-` and a final `.json`
extension belongs to this family; nested, case-varied, backup-suffixed, and
sibling fixture paths MUST NOT seed these roots. Ordinary changed package
roots are united with the fixture roots; the change MUST NOT force a full
build. An explicit single-language plan seeds and validates only its own
applicable consumer. Missing applicable registered consumers MUST abort
planning before any partial plan is emitted. The unforced affected sets and
toolchain flags MUST demonstrate scheduling on all three platforms; C# and
F# remain distinct package roots but share the canonical `dotnet` toolchain
flag. Every new direct native reader MUST extend this relation and a
source-reference drift test in the same change.

The flat `code/specs/fixtures/build-tool-v1/cases/source-collection-*.json`
family has three bounded direct-native-consumer relations. The seven
package-local cases (neither `repository-` nor `shared-input-` after the
`source-collection-` prefix) MUST seed `dotnet/programs/build-tool-csharp`,
`dotnet/programs/build-tool-fsharp`, and `<language>/programs/build-tool` for
Elixir, Go, Haskell, Lua, Perl, Python, Ruby, Rust, Swift, and TypeScript. Rust
currently names only two of those seven cases; selecting its front for the
whole local subfamily is intentional conservative over-selection. The nine
`source-collection-repository-*.json` cases MUST seed only C#, F#, and Swift
build-tool roots. The four `source-collection-shared-input-*.json` cases MUST
seed only C# and F# build-tool roots. These relations record native test
readers, not a claim that every front replays every fixture in its subfamily.
The shared `contracts-build-tool-conformance` gate remains responsible for
all cases, including cases with no native reader.

Only a flat filename with a nonempty stem after its most specific prefix and
a final `.json` extension belongs to one of these subfamilies. Classify
`repository-` and `shared-input-` before the package-local fallback; empty
subfamily stems, nested paths, backslash spellings, wrong case, backups, and
sibling fixture families MUST NOT seed these roots. The relation applies to
added, modified, deleted, and renamed paths, including the deleted source of
a rename. On detect and all Linux, macOS, and Windows plan overrides, union
the applicable native roots with ordinary changed package roots before
affected/prerequisite closure without forcing a full build. Explicit
single-language plans seed and validate only readers in that language.
Missing applicable registered roots MUST fail before an incomplete plan is
written. The unforced affected set and toolchain flags MUST demonstrate
native scheduling; C# and F# are separate roots sharing the `dotnet` flag.
A new direct native reader or changed native fixture roster MUST update the
relation and its source-reference drift test together.

The exact flat `code/specs/fixtures/build-tool-v1/cases/graph-*.json` and
`code/specs/fixtures/build-tool-v1/cases/diff-selection-*.json` families have
eight and twelve checked cases respectively. Any added, modified, deleted,
or renamed path in either family, including the deleted source of a rename,
MUST seed their eleven direct native readers before affected/prerequisite
closure: `dotnet/programs/build-tool-csharp`,
`dotnet/programs/build-tool-fsharp`, and `<language>/programs/build-tool` for
Java, Kotlin, Dart, OCaml, Go, Haskell, Perl, Python, and Swift. Java, Kotlin,
Dart, and OCaml tests discover `cases/*.json` by decoded `graph` and
`diff_selection` domains and pin all twenty IDs; they are real native readers
even without literal fixture filenames. OCaml is still an emerging lane, and
these fixtures exercise process-free cores only in Java/Kotlin/Dart/OCaml, not
complete build-tool front doors or neutral adapters.

Classify raw Git paths without OS cleanup. Only the exact prefix, a nonempty
flat stem, and a final lower-case `.json` extension belong to either family;
nested paths, backslash spellings, empty stems, case variants, backups, and
sibling fixture domains MUST NOT seed these roots. Ordinary changed package
roots are united with the native readers, without forcing a full build.
Detect and Linux, macOS, and Windows plan overrides MUST schedule applicable
readers. Explicit single-language planning seeds and validates only that
language's reader. A missing applicable discovered root MUST fail before any
partial plan is emitted. Unforced affected sets and toolchain flags MUST
demonstrate scheduling; C# and F# are distinct roots sharing `dotnet`.
New native readers or case rosters MUST update the exact relation and a
source-reference drift test together, including dynamic domain enumeration.

The twenty-six flat `code/specs/fixtures/build-tool-v1/cases/resolution-*.json`
cases have a heterogeneous native reader relation. A changed case MUST seed
only its direct test fronts, before affected/prerequisite closure. The following
abbreviations denote `<language>/programs/build-tool`: G=Go, H=Haskell,
L=Lua, P=Perl, Y=Python, B=Ruby, R=Rust, S=Swift, T=TypeScript.

| Case stem after `resolution-` | Direct readers |
| --- | --- |
| `build-deps-comment` | G, H, B |
| `dart-field-aware`, `dotnet-cross-language-field-aware`, `haskell-field-aware` | G, H, Y |
| `dotnet-csharp-field-aware`, `dotnet-fsharp-field-aware`, `gradle-java-field-aware`, `gradle-kotlin-field-aware` | G, H, Y |
| `ecosystem-scoped-aliases` | G, Y |
| `elixir-field-aware`, `go-field-aware`, `perl-field-aware`, `ruby-field-aware`, `swift-field-aware`, `typescript-field-aware` | G, H |
| `elixir-program-package` | B, R |
| `elixir-self-edge` | R |
| `lua-cycle`, `lua-field-aware`, `lua-program-package`, `python-diamond`, `python-field-aware`, `rust-field-aware` | H |
| `lua-utf8`, `lua-invalid-utf8` | G, H, L, P, B, R, S, T |
| `ocaml-field-aware` | G |

The case-to-reader map MUST match the checked-in resolution corpus exactly:
new valid flat cases are unknown, not neutral-only, until classified. Reader
drift checks MUST include Haskell's dynamically assembled Gradle/.NET case
names and Rust's package-local `src/resolver.rs` test module. Classify raw Git
paths without OS cleanup; nested or backslash paths, empty stems, case
variants, backups, and sibling domains do not match. Added, modified, deleted,
and renamed paths (including the deleted source) participate. Ordinary package
changes unite with the fixture roots without forcing a full build. Explicit
language filters seed only applicable readers; a missing applicable root or
unknown valid case MUST fail before a partial plan is emitted. Linux, macOS,
and Windows plans MUST expose the selected affected roots and toolchain flags.

The eleven flat `code/specs/fixtures/build-tool-v1/cases/hashing-cache-*.json`
cases have this exact native reader relation. C and F are the distinct
`dotnet/programs/build-tool-csharp` and `dotnet/programs/build-tool-fsharp`
fronts; L, G, P, Y, B, S, and T denote the Lua, Go, Perl, Python, Ruby,
Swift, and TypeScript `<language>/programs/build-tool` fronts.

| Case stem after `hashing-cache-` | Direct readers |
| --- | --- |
| `corrupt` | C, F, L, Y, T |
| `hit` | C, F, L, Y |
| `missing` | C, F, L, G, P, Y, B, S |
| `dependency-change-after`, `dependency-order-before`, `failed-prior-record`, `local-boundary-union` | C, F, L |
| `shared-input-conduit-after`, `shared-input-conduit-before`, `shared-input-sha256-native-after`, `shared-input-sha256-native-before` | C, F, L |

C#/F# enumerate the entire checked glob; Lua lists all eleven; Python
constructs its three state names dynamically. The relation MUST track those
test-source references, including dynamic readers, and the checked corpus
roster. New valid flat cases MUST fail closed until classified, not silently
fall back to the neutral gate alone. Added, modified, deleted, and renamed
paths (including the deleted source) participate, but nested, backslash,
case-varied, and backup lookalikes do not. The selector MUST keep ordinary
changed package roots, honor an explicit language filter, and seed only
applicable native readers on Linux, macOS, and Windows without forcing a
full build. Missing applicable roots or unknown cases MUST fail before a
partial plan is emitted. C and F share the `dotnet` toolchain, while each
remains a separately selected native test front. Some readers assert digest
slices rather than the full neutral cache-decision result.

The twenty-two flat `code/specs/fixtures/build-tool-v1/cases/validation-*.json`
cases have the following closed native validator relation. C/F are the distinct
`dotnet/programs/build-tool-csharp` and `dotnet/programs/build-tool-fsharp`
fronts; E/H/L/P/Y/B/R/S/T denote the Elixir, Haskell, Lua, Perl, Python,
Ruby, Rust, Swift, and TypeScript `<language>/programs/build-tool` fronts;
G denotes Go.

| Case stem after `validation-` | Direct readers |
| --- | --- |
| `orphan-crates-clean`, `orphan-crates-unlisted`, `orphan-exemptions-invalid`, `orphan-exemptions-stale` | C, F, E, H, L, P, Y, B, R, S, T |
| `tracked-artifacts-aliases`, `tracked-artifacts-clean`, `tracked-artifacts-forbidden`, `tracked-artifacts-invalid`, `tracked-artifacts-unicode-boundaries` | C, F, E, H, L, P, Y, B, R, S, T |
| `orphan-package-root-exemptions-invalid`, `orphan-package-root-exemptions-stale`, `orphan-package-roots-clean`, `orphan-package-roots-unlisted`, `lua-windows-sibling-parity-absent` | G |
| `clean-build`, `clean-full`, `dependency-oracles`, `identity-manifest-ambiguous`, `missing-build`, `path-unsafe`, `starlark-declarations-invalid`, `toolchain-unsupported` | neutral gate only |

This map MUST match the checked corpus and native test sources, including
Perl's dynamically constructed fixture names. A new valid flat case is
unknown, not implicitly neutral-only, and MUST fail before a partial plan.
Added, modified, deleted, and renamed paths (including the deleted source)
participate; nested paths, backslash spellings, empty stems, case variants,
backup files, and other fixture domains do not. Ordinary changed package
roots unite with direct readers without forcing a full build. Explicit
language filters select only applicable readers; missing applicable roots
fail before planning. Linux, macOS, and Windows plans MUST expose unforced
affected roots and toolchain flags; C/F share `dotnet` but remain separate
fronts. Neutral-only cases still run the shared fixture gate.

The five flat `code/specs/fixtures/build-tool-v1/cases/plan-*.json` cases
have this closed direct native plan-reader relation. Y denotes
`python/programs/build-tool`; T denotes `typescript/programs/build-tool`.

| Case stem after `plan-` | Direct readers |
| --- | --- |
| `replace-existing` | Y |
| `portable-package-path` | T |
| `affected-empty`, `affected-null`, `future-version` | neutral gate only |

The relation MUST match the checked five-case corpus and native test-source
references; the neutral conformance runner is not a native BUILD reader. A
new valid flat case is unknown and MUST fail before a partial plan is emitted.
Added, modified, deleted, and renamed paths (including the deleted source)
participate. Nested paths, backslash spellings, empty stems, case variants,
backup files, and sibling fixture domains do not. Ordinary changed package
roots unite with direct readers without forcing a full build. An explicit
language filter selects only applicable readers; a missing applicable BUILD
root fails atomically. Linux, macOS, and Windows plans MUST expose the
unforced affected roots and Python/TypeScript toolchain flags. The three
neutral-only cases still run the shared fixture gate.

## Evaluation

A gate is **required** when ANY of the following holds:

1. `force` is set, or the affected set is `null` (git diff unavailable)
2. the run is a main-branch push
3. `.github/workflows/ci.yml` changed
4. `code/specs/data/ci-gates.json` changed
5. `packages` intersects the affected closure
6. any changed file matches a `paths` glob

Rules 1 and 2 preserve the existing "main forces everything" behavior, so a gate
that is wrong on a pull request is still caught on merge. Rules 3 and 4 are
self-tests: a change to the gating machinery runs everything it gates, mirroring
the `workflow_changed()` escape hatch already present in the six
`code/scripts/*_ci_acceptance.py` scripts.

Rule 5 uses the **affected closure** — changed packages plus their transitive
dependents, as computed by `directedgraph.AffectedNodes` — so listing only the
packages a job directly exercises is sufficient; the graph supplies the rest.

### Closure compiler native Windows acceptance

The Windows general package-test step is conditional on
`build-windows-os-suites` (or another native toolchain requirement); a Rust
toolchain flag alone does not run that step. The gate MUST include affected
package `rust/programs/closurec` so compiler and dependency changes execute its
Windows publication and ACL regressions. Its path clause MUST also include
`code/specs/CV02-checked-bounded-provenance-graphs.md`. That exact specification
path MUST also seed `rust/programs/closurec` as a changed package root on Linux,
macOS and Windows, so a specification-only change selects both its native test
command and the Rust toolchain. Planning MUST fail if that consumer is missing
from discovered packages. An explicitly non-Rust single-language invocation
does not seed or require the Rust consumer. Union this root with ordinary package
edits without forcing unrelated packages; deleted or renamed source paths still
trigger the exact relation. Near-matching specification paths do not.
Both gate clauses are exercised through the real evaluator. Emitted-plan tests
must verify the compiler, Rust toolchain and Windows step gate together for a
specification-only diff; an unrelated Rust affected set must not enable the gate.
An OS-labelled green job whose compiler test step was skipped is insufficient
native acceptance evidence. Linux/macOS ordinary package tests already execute
on their selected matrix legs.

### Portable evaluation boundary

The registry decision is a process-free build-tool domain named
`ci_gate_selection`. Its input is a validated in-memory registry, a nullable
affected-package set, a nullable changed-file list, and `force`. Its result is
one record per gate, sorted by gate id, containing the id, the boolean verdict,
and the deterministic `run_` output name.

`null` and an empty list are deliberately different. A `null` affected set or
changed-file list means change detection was unavailable and MUST fail open;
an empty list means change detection succeeded and found nothing. Implementations
MUST evaluate every gate and MUST NOT omit false verdicts.

When neither snapshot is `null`, `force` is false, and no changed path is one
of the fixed gating-machinery sentinels below, evaluation applies one fixed
operation-wide ceiling of 50,000,000 path-selection work units. After registry,
identifier, output-name, and glob validation, but before package intersection
or any matcher call, implementations MUST deduplicate identical path patterns
and preflight every distinct pattern/changed-file pair. A pair first costs the
Unicode-scalar lengths plus one separator unit for every literal leading and
trailing path segment outside the pattern's first-to-last wildcard segment. If
those literal segment bounds cannot match the path, the pair is rejected
without a glob call. Every remaining candidate additionally costs
`(pattern Unicode-scalar count + 1) * (changed-file Unicode-scalar count + 1)`.
The full candidate product is charged even when a package intersection or an
earlier path match could determine the verdict. Checked arithmetic is required:
exactly 50,000,000 units proceeds, while overflow or any larger total returns
an empty result with exactly one error diagnostic
`CI_GATE_MATCH_LIMIT_EXCEEDED` and performs zero matcher calls. A successful
oracle expectation MUST reject an adapter error rather than treating it as an
alternative outcome.

Force, either `null` snapshot, and the fixed gating-machinery sentinels are
reviewed run-all paths: they return every gate as true before match-work
preflight. Invalid registries and invalid globs retain precedence over both
that bypass and the ceiling. A limit error is a hard planning failure; the
front door MUST write neither a partial plan nor partial gate outputs.

A CI adapter MAY respond to exactly `CI_GATE_MATCH_LIMIT_EXCEEDED` by starting a
second, explicit operation with `force=true`. This is a conservative run-all
fallback, not recovery of the failed operation: the adapter MUST discard any
first-operation output, MUST make the fallback visible in its log, and MUST NOT
retry any other diagnostic this way. The portable core and first operation keep
the ceiling, zero matcher calls, empty result, and no-partial-plan contract
unchanged.

The portable core owns exact package intersection, the repository-relative glob
grammar implemented by `internal/globmatch`, output-name mapping, and these
fixed machinery sentinels:

- `.github/workflows/ci.yml`
- `code/specs/data/ci-gates.json`
- `code/programs/go/build-tool/internal/cigates/`
- `code/programs/go/build-tool/internal/globmatch/`
- `code/programs/go/build-tool/internal/gitdiff/`
- `code/programs/go/build-tool/main.go`

Registry file I/O, Git diff acquisition, dependency-graph construction,
`$GITHUB_OUTPUT`, workflow scheduling, and branch-protection policy remain
outside this pure boundary. The language-neutral fixtures validate a closed
registry before evaluation; native registry loaders remain responsible for
rejecting future schema versions, invalid ids or scopes, missing descriptions,
gates with neither packages nor paths, and ids whose hyphen-to-underscore
mapping would collide on the same output name.

The exact-main front-door audit at
`8fe279a38603d7a53147624d65d6ecf288585199` found:

| Front door | Native `ci_gate_selection` | Delivery owner |
|---|---:|---|
| C# / F# shared engine | no | `build-tool-csharp-fsharp-ci-gate-selection-conformance` |
| Elixir | no | `build-tool-elixir-ci-gate-selection-conformance` |
| Go | yes; consumes all neutral cases | this corpus/oracle tranche |
| Haskell | no | `build-tool-haskell-ci-gate-selection-conformance` |
| Lua | no | `build-tool-lua-ci-gate-selection-conformance` |
| Perl | no | `build-tool-perl-ci-gate-selection-conformance` |
| Python | no | `build-tool-python-ci-gate-selection-conformance` |
| Ruby | no | `build-tool-ruby-ci-gate-selection-conformance` |
| Rust | no | `build-tool-rust-ci-gate-selection-conformance` |
| Swift | no | `build-tool-swift-ci-gate-selection-conformance` |
| TypeScript | no | `build-tool-typescript-ci-gate-selection-conformance` |

For the C#/F# owner, the portable evaluator lives in the C# build-tool
assembly, separate from graph/diff selection. The F# build tool exposes an
explicit language-native facade over that reviewed shared engine; both native
test projects independently replay the complete neutral `ci_gate_selection`
corpus and reject result/diagnostic drift. The evaluator accepts only inert
in-memory records, validates the full registry before any run-all shortcut,
and never acquires Git, filesystem, process, workflow, or output authority.
Register both native fixture readers in the Go build-plan selector in the same
change, so a fixture-only diff schedules their actual package tests on all
supported CI platforms.

Java/Kotlin, Dart, and OCaml remain owned by their existing build-tool creation
and promotion items; the final CI gate aggregate depends on those owners as
well as every explicit current-front-door leaf.

### Python process-free adoption

The Python build tool's `ci_gate_selection` owner exposes a typed pure function
over a caller-supplied, already-validated registry and nullable in-memory
affected-package and changed-file snapshots. The function returns every gate
in ID order, including false verdicts, or one stable
`CI_GATE_MATCH_LIMIT_EXCEEDED` diagnostic with no partial gate list. It uses the
existing portable `glob_match` implementation, not host path matching. Python
`len()` counts Unicode scalars for the preflight; the complete operation-wide
charge is computed before the first matcher invocation, even if an early gate
or package intersection would otherwise decide the result. The entire checked-in
`ci-gate-selection/*.json` neutral case set must be discovered by ID and replayed
through this production function in package-local tests. Fixture decoding and
any registry file I/O remain in tests or future reviewed front-door adapters,
not in the pure operation. This adoption does not mark a neutral execution
adapter ready.

### Fail open

Every ambiguity resolves to `true`. A malformed registry is a hard error at plan
time rather than a silent all-`false`. The asymmetry is deliberate: a false
positive wastes one job, a false negative lets a regression through. Per
`lessons.md`, *"when a gate is derived by pattern-matching a build script, the
failure mode is silence — a package that matches nothing is indistinguishable
from a package that passed."*

The validated match-work ceiling is likewise a hard error rather than an
ambiguous selection. Continuing would permit attacker-shaped matching work,
and emitting partial outputs would make the gate set internally inconsistent.

## Outputs

The evaluated map is published two ways.

**In the build plan.** An optional top-level `ci_jobs` object:

```json
{ "ci_jobs": { "ruby-grammar-regen-check": false, "d18f-message-conformance": true } }
```

This is an additive optional field. `build-plan-v1.schema.json` sets
`additionalProperties: true`, and `build-plan-v1.md` lists adding an optional
top-level field as NOT requiring a schema version bump — the same treatment
`platform_overrides` and `shards` received.

**As GitHub Actions step outputs.** One line per gate on `$GITHUB_OUTPUT`:

```
run_ruby_grammar_regen_check=false
run_d18f_message_conformance=true
```

Job ids are lowercased with `-` replaced by `_` and prefixed with `run_`,
because Actions output names cannot contain hyphens. The `run_` prefix keeps
them distinct from the existing `needs_<lang>` toolchain flags, which
`internal/validator.validateCIFullBuildToolchains` asserts on separately.

## Interaction with branch protection

Skipping is safe. The `ci-gate` job already runs `if: always()` and treats a
dependency result of `skipped` as passing:

```bash
case "$r" in
  success|skipped) ;;
  *) echo "::error::a required job did not pass (result: $r)"; fail=1 ;;
esac
```

`CI gate` remains the single required pull-request context, so no branch
protection settings change. A gated job that skips reports `skipped`, and the
gate stays green.

## Relationship to the existing acceptance scripts

Six scripts under `code/scripts/` (`venture_windows_ci_acceptance.py` and five
`mosaic_*_ci_acceptance.py`) already implement this pattern by hand, each with a
hardcoded package set and its own unit tests. They are the prior art this
registry generalizes.

They are intentionally left in place for now. Their tests assert on literal
`ci.yml` substrings, so folding them in is a mechanical but wide change that
belongs in its own pull request rather than riding along with a performance fix.

## Adding a job

### Rust compiled grammars: reconciled lanes

The VM-067 check regenerates the reconciled MacroOct and Nib `_grammar.rs`
artifacts and rejects a byte diff. Its registry entry covers both languages'
lexer/parser packages and canonical sources plus the Rust grammar-tools library
and CLI. The job is intentionally limited to confirmed clean artifacts; issue
#14202 tracks the other Rust grammars whose current output is stale. Widen this
one job after reconciling another lane instead of adding one queued job per
language.


1. Add the job to `.github/workflows/ci.yml` with `needs: detect` and
   `if: needs.detect.outputs.run_<job_id> == 'true'`.
2. Add its entry to `code/specs/data/ci-gates.json`, listing every package it
   runs and every non-package path it reads.
3. Add the job to `ci-gate`'s `needs:` list and its result loop.
4. `code/scripts/tests/test_ci_gate_registry.py` fails if steps 1 and 2
   disagree, so a gate name that does not match a job — or a gated job with no
   registry entry — cannot merge.

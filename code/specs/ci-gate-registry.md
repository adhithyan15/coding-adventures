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

Java/Kotlin, Dart, and OCaml remain owned by their existing build-tool creation
and promotion items; the final CI gate aggregate depends on those owners as
well as every explicit current-front-door leaf.

### Fail open

Every ambiguity resolves to `true`. A malformed registry is a hard error at plan
time rather than a silent all-`false`. The asymmetry is deliberate: a false
positive wastes one job, a false negative lets a regression through. Per
`lessons.md`, *"when a gate is derived by pattern-matching a build script, the
failure mode is silence — a package that matches nothing is indistinguishable
from a package that passed."*

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

1. Add the job to `.github/workflows/ci.yml` with `needs: detect` and
   `if: needs.detect.outputs.run_<job_id> == 'true'`.
2. Add its entry to `code/specs/data/ci-gates.json`, listing every package it
   runs and every non-package path it reads.
3. Add the job to `ci-gate`'s `needs:` list and its result loop.
4. `code/scripts/tests/test_ci_gate_registry.py` fails if steps 1 and 2
   disagree, so a gate name that does not match a job — or a gated job with no
   registry entry — cannot merge.

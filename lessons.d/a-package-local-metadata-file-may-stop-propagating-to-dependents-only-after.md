---
category: BUILD files & dependency management
---

# A package-local metadata file may stop propagating to dependents only after proving no dependent reads it

**What went wrong.** The Go build tool mapped every changed file under a
shell-BUILD package to that package and then rebuilt all of its transitive
dependents. A PR that only added `rust/bounded-json/required_capabilities.json`
and a `CHANGELOG.md` entry selected 556 packages; a ten-branch split of one
capability PR would have cost about 2,200 package builds, and the Windows
job's 300-minute step limit does not survive sets that large. None of those
dependents could read either file.

**The fix.** `code/programs/go/build-tool/internal/pkgmeta` names the
package-root files a package's own BUILD may check but no dependent reads:
`CHANGELOG.md`, `CHANGELOG.d/*.md`, `required_capabilities.json`,
`effect_certificate.json`, `assurance_report.json` and `evidence/**` data
(`.json`, `.cnf`, `.drat`). A package whose only changes are additions or
edits of those files is selected with its prerequisites but not its
dependents or Cargo dev-dependency triggers
(`package_metadata.go`). The shared contract pins it as the
`metadata_policy: "package_local"` diff-selection variant
(`package-metadata-selection-*.json`). The bounded-json case now selects 1
package.

**What review caught.** The first version listed `README.md`, but 644 BUILDs
run `pip install -e ../<dep>` and hatchling reads the dependency's
`readme = "README.md"`: deleting `python/html-renderer/README.md` broke
`python/programs/pipeline-visualizer` while only the renderer was selected.
"Only the package itself reads it" must include the dependents' *packaging*,
and an existence requirement is broken by a deletion even when no content is
read, so deletions, renames and type changes (Git `D`/`T`, unknown status)
always propagate now. It also classified local roots from the unfiltered path
list while ordinary roots used the platform-filtered one, so a metadata
change beside a `BUILD_windows` edit selected nothing on Linux and macOS.
Compute both sets from one list. And an allowlist closed only by name let
`evidence/run.sh` be "metadata"; it is now closed by type, and
`-validate-build-files` refuses any BUILD, script, Cargo path or Rust
`#[path]`/`include!` that executes a metadata-class location.

**What consumes what** (the evidence, so the next person can re-check it):

- Own package only: `rust/ct-compare` and `rust/gf256` BUILDs verify their own
  certificate, assurance report and `evidence/`; Rust/Go/OCaml analyzers and
  many tests read their own `required_capabilities.json`.
- Dependents, for existence only: packaging file lists (gemspec `files`,
  `.cabal` `extra-doc-files`, Perl `MANIFEST`, npm `files`) name
  `CHANGELOG.md` and sometimes `required_capabilities.json`; only a deletion
  can break them, and deletions propagate. Dependents read `README.md`
  content (hatchling `readme`), so README is not metadata.
- Other packages, none of them a dependent of the owner:
  `rust/required-capabilities-compiler` reads
  `rust/weather-agent-e2e/required_capabilities.json` (a declared
  repository-boundary input, so it still propagates from the compiler);
  `rust/ca-assurance-report` tests read `rust/gf256`'s evidence and report;
  `rust/ca-capability-analyzer` tests read `rust/ct-compare`.
- Scheduled by path, not by package: `code/scripts/assumption_catalogue.py`
  (every `assurance_report.json`), the changelog checkers, and the
  `contracts-capability-cage` gate (`code/**/required_capabilities.json`).
- Lookalikes that DO cross packages and therefore stay ordinary source:
  `rust/gf256/evidence.bindings` (read by `rust/warrant-grammar` and
  `rust/warrant-ir`) and nested `tests/fixtures/README.md` (`rust/html-lexer`
  reads `rust/html-parser`'s). `c/neural-framework`'s CMake
  `install(FILES ../<dep>/README.md ...)` names dependency READMEs, but
  neither its BUILD nor CI runs `cmake --install`.

**Do differently.** Before adding a file to the list, grep the whole tree for
its basename and for `../<pkg>/<file>` spellings, then check each reader with
the build graph: a reader that is a transitive dependent of the owner means
the file must keep propagating. Before making a dependent read another
package's metadata file (an install step, a test fixture, an
`include_str!("../../dep/README.md")`), take that file off the list in
`internal/pkgmeta`, the oracle in `code/scripts/build_tool_conformance.py`,
and `code/specs/12-build-system.md` in the same PR. When in doubt, leave a
file propagating: a wrongly-propagating file costs CI time, a wrongly-local
one lets a broken dependent merge.

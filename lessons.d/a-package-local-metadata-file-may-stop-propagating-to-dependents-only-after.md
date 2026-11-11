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
`README.md`, `CHANGELOG.md`, `CHANGELOG.d/**`, `required_capabilities.json`,
`effect_certificate.json`, `assurance_report.json` and `evidence/**`. A
package whose only changes are those files is selected with its prerequisites
but not its dependents or Cargo dev-dependency triggers
(`package_metadata.go`). The shared contract pins it as the
`metadata_policy: "package_local"` diff-selection variant
(`package-metadata-selection-*.json`). The bounded-json case now selects 1
package.

**What consumes what** (the evidence, so the next person can re-check it):

- Own package only: `rust/ct-compare` and `rust/gf256` BUILDs verify their own
  certificate, assurance report and `evidence/`; Rust/Go/OCaml analyzers and
  many tests read their own `required_capabilities.json`; READMEs are read by
  their own package's packaging or `include_str!`.
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

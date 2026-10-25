# Recovery validation — October 25, 2026

Existing PR #18183 was recovered from `231de32b81482caebbdf484387263d86b13b2602`
onto `9000beddee61a136cf5c0c6c5787f81d3d717834`. The production provider is
unchanged: Git blob `efead53ef959a6c836afa58d56a51368e97bf3b6`.
The runtime source reviewed and tested is commit
`80a01e81ca1e1034f1658109a284f4f45bcc0c7f`; subsequent changes document evidence
and lifecycle state. A later macOS-only BUILD environment repair is described
below; it leaves the provider and test source unchanged.

## macOS runner-image linker follow-up

Exact head `e04acc841ef2de4f61a56b5ae671a3927c81a38f`, run 40228623570,
job 121277004429 failed on image macos-26-arm64 20261013.0424.1. Dependency
linking rejected `arm64e.x1` in Command Line Tools SDK27 stubs before provider
tests. This matches [runner-images #14940](https://github.com/actions/runner-images/issues/14940)
and the [merged Acton repair](https://github.com/actonlang/acton/pull/3283).

The BUILD repair selects `DEVELOPER_DIR=/Library/Developer/CommandLineTools`
only for Darwin's Cabal command and descendants. It changes no SDK files,
global tool selection, compiler/dependency versions, coverage or test set.
Three executable shell tests establish Darwin selection, preservation of the
parent/non-Darwin environment and propagation of Cabal failures. The Darwin
test failed before the repair and all three pass afterwards. Ruff check/format
and six Haskell-capability checks pass. The actual Unix BUILD again passes
on Windows with GHC9.4.8/Cabal3.10.3.0: 13 examples, zero failures, HPC enabled.
These results do not establish macOS linking success; fresh hosted CI remains
required. The prior production HPC measurements below remain source-identical.

## Native package

- Windows amd64, GHC 9.4.8, Cabal 3.10.3.0, using the versioned GHCup shim.
- The original global `--enable-coverage` command reproduced the hosted
  pre-compilation failure for vector 0.13.2.0. Narrowing vector exposed another
  internal-library failure and older attoparsec's incompatible ghc-prim bound.
  No experimental dependency constraints are retained.
- The named project-package `coverage: True` setting instruments both the
  library and test executable. `cabal test all` passes 13 examples over all
  359 corpus records, including independent corruption and ownership witnesses.
- Nonempty production-only HPC: **447/447 expressions and 61/61 alternatives**.
  Top-level declarations are **17/39**, a separate metric. Test-reader coverage
  is not substituted for production coverage.
- `cabal check` reports no warnings or errors. A build with `-Wall -Werror`
  passes. The actual Unix BUILD command passes with the same Cabal version,
  through a verified short root junction to avoid Windows build-path limits.
  Its arguments were not wrapped or replaced.
- The Windows BUILD front prints its existing explicit Haskell skip. This is
  not native CI qualification. The oversized-file IO rejection branch is not
  exercised by the in-memory oversized-transport witness.

## Shared build tool and repository checks

- All **15 Go build-tool packages** pass with **84.6% aggregate coverage**;
  the main package measures **62.8%**. Go vet and executable build pass.
  Real PowerShell generator tests run without a skip flag.
- The Go provider's actual tests and vet pass, with **99.0% coverage**.
- The unforced planner evaluates **45 Starlark BUILD files**, discovers
  **5,389 packages**, selects **14 affected/prerequisite/dependent packages**
  from two changed packages, and passes both orphan gates with **37 existing
  pending exemptions**. Planning is not native execution of those fourteen.
- Python checks: **19 CI-registry**, **6 Haskell-capability** and **10 parity
  reporter** tests pass. Lesson validation: **1,540 shards, zero problems**.
- Final structural parity: **1,562 identities / 4,850 slots / fifteen established
  lanes**, no collisions or unknown buckets; OCaml remains emerging with five
  packages. Inventory is not native qualification.
- All package/program sources and relevant fixtures/configuration are present
  for native validation. Unrelated human-language curriculum files are omitted
  from this checkout after a complete checkout exhausted disk space. Authored
  edits and test receipts were preserved; no curriculum validation is claimed.

Independent source/security review passed the runtime source at `80a01e81ca1`.
Final documentation/lifecycle delta review and fresh hosted checks remain
required before exact-head auto-merge enrollment. No compiler, VM, module
loading, filesystem, process, network, credential or host authority is added to
the production provider.

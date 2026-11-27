# vos-link

The **host tool** of the verified-OS document pipeline (backlog VOS-033, spec
[`code/specs/VOS02-interface-contracts-and-profile-linking.md`](../../../specs/VOS02-interface-contracts-and-profile-linking.md)
sections 2, 6.8, 10 and 10.1).  Crate class **H**: it reads files and writes exactly one.
Everything that decides anything is the pure library
[`vos-manifest`](../../../packages/rust/vos-manifest).

## What it does

An implementation of a contract package (VOS02 R1-R4) does not write its own
`.warrant`.  The document is **generated**: the contract package's contract block, the
implementation's hand-written block, and the obligation blocks the implementation carries,
each between marker comments with its SHA-256, so one text has one author and nothing can
drift without a failing check.

```text
vos-link gen-doc   code/packages/rust/<impl> [--root <repo>]   # writes code/specs/<impl>.warrant
vos-link check-doc code/packages/rust/<impl> [--root <repo>]   # exit 1 on any drift or rule failure
vos-link check-component code/packages/rust/<impl> [--root <repo>]   # is the crate a complete component? (VOS04 5.6)
vos-link gen-contract    code/packages/rust/<contract> [--root <repo>]   # writes warrant/lemmas.table (VOS-039b-1)
vos-link check-contract  code/packages/rust/<contract> [--root <repo>]   # lemmas, mutants, config mutants, the table
vos-link check-publish   [--root <repo>]                                # every evidence-only vos-* crate says publish = false (VOS-D104)
```

| Exit | Meaning |
|---|---|
| 0 | no findings |
| 1 | findings (a rule failed on bytes that were read and admitted) |
| 2 | no verdict (an unreadable or oversized file, a symlink, a path outside the repository, a usage error) |

## `gen-build` and `check-component` (VOS-039a, VOS04 section 5.6)

`gen-build` writes the canonical region of `BUILD` and `BUILD_windows` (lines 1 to 9 of VOS04 5.2, the
Linux tarpaulin coverage gate, then `check-component`, then a marker; your own lines go below it) from
`vos.toml`, `evidence.bindings`, the generated document and `warrant/sabotage.plan`, and the tolerance
script from `excused-controls`.  `check-component` then judges the crate: a missing file (`CC02`), a BUILD
file that does not begin with exactly what `gen-build` writes (`CC03`; a text lint was bypassable by
single-line tricks such as `echo`, `|| true` or a commented `"Linux"`, so the lines are reproduced, not
recognised), a bad or incomplete sabotage plan or tolerance script (`CC04`), a missing build-tool row
(`CC05`, read inside the Go slice with comments and raw strings removed), an `-any` crate without
`publish = false` (`CC06`), a missing or too-low `coverage-floor` (`CC07`) and an evidence manifest that
is not the document's claims times the tiers (`CC08`: counts, controls that PASS, whole-id excuses).
For kind `any` the oracle must be shown to survive (`CC11`, VOS-039c, VOS-D080): the `Impl` field outside the cone of `build` is exactly one, the LAST, and numeric, every state-producing operation (an `op` of the generated document that returns `Impl` and takes one; read from the document, not from the optional `ret=`/`args=` of the bindings) lists the fault `field-flip-last`, and the sabotage plan attacks each with a claim of the contract's `any` tier that calls it alone.  The claims themselves live in the contract (`insert_oracle_*`, `remove_oracle_*`, VOS10 9.3); the build proves that they fail under the fault.
Since VOS-039b-1 a **contract crate** (kind `contract`) is a component too: `gen-build` writes a shorter,
identical `BUILD` and `BUILD_windows` (tests, shape check, capability analyzer, `check-contract`,
`check-component`; no environment variable, no python), and `check-component` judges its file set (`CC02`),
the BUILD files byte for byte (`CC03`) and the `vosContractRow` build-tool row (`CC05`).  Fixtures and other
kinds stay out of scope (`CC01`).
Since VOS-040c a contract whose `vos.toml` says `sat = "model"` gets one more generated line, `vos-model-sat check .`
(release), between `check-contract` and `check-component`; `CK11` (in `check-contract`) checks that line and a
well-formed, current `warrant/lemmas.sat` with one row per lemma per width and at least one counted obligation (files only: the lane
depends on this crate, so it cannot be called from here, and the BUILD line does the solving), and the absent key is clean.
An implementation that makes any SAT claim fails `CC12` (`sat` key, `warrant/lemmas.sat`, or a BUILD line naming the lane).

## Bounds against ceilings, contract-crate closure and the publish sweep (VOS-039b-2b, VOS-D112)

`check-component` also runs **CC09** on every implementation: the `bound` rows of the generated document
against the ceilings of `contract.vos`, per component.  Which ceiling an `op` is compared with is DERIVED from its
name (the ceiling `X` itself, else `X` with one `_out` or `_img` suffix stripped, else it is ghost-only, like
`alpha_w` and `cfg_n`), never declared.  A bound that is not an integer literal, a bound above its ceiling, a
missing `steps` or `heap` row (production and `any`) and a ceiling operation nobody realises fail; `depth` is
compared when present and noted when absent.  For a **contract** crate **CC10** requires no dependency, build
script, proc-macro (`proc-macro` or `crate-type`) or `[patch]`/`[replace]`/`[workspace]` table at all (the dependency half of VOS-D112; `vos-shape-check` SH19 (h) closes the attribute half).

`vos-link check-publish [--root R]` sweeps the direct `vos-*` crates of `code/packages/rust` and
`code/programs/rust`: any crate whose kind is not `production`, `reference` or `contract` must say `publish = false`
(`PB01`; `PB02` unreadable TOML; a symlinked crate is no verdict).  A kind-exempt crate must also be registered in the
build tool's `repository_inputs.go` (`vosImplementationRow` or `vosContractRow`), else `PB03`.  It runs in this crate's BUILD.  The publish policy of
the exempt kinds is undecided (release gate, VOS-D104).

## `gen-contract` and `check-contract` (VOS-039b-1, VOS04 section 5.6, VOS02 D-GD11)

The generic form of what the retired `vos-sched-contract-check` crate did by hand.  `check-contract` builds one
Warrant document in memory (the contract block, the generated `cfg_<KEY>()` models of the `config` rows of
`contract.vos`, then `warrant/lemmas.part`) and evaluates **every lemma over every value of its binders**
(`Bool` and `U8`, at most 400,000 cases).  It then replaces one model of the contract block, by the span the
Warrant parser reports, with the single `model` of `warrant/mutants/<lemma>.part` and requires that lemma to
**fail** (an evaluation error is not a kill).  For each `config` key it requires that some lemma calls
`cfg_<KEY>()` and that the tool's own mutants V-1 and V+1 each break one.  What it saw is written by
`gen-contract` into `warrant/lemmas.table` and compared byte for byte (`CK09`); the table is never parsed.
Codes: `CK01` inputs, `CK02` type check, `CK03` budget and domain, `CK04` a lemma fails, `CK05` mutant set,
`CK06` mutant form, `CK07` a lemma survives, `CK08` config, `CK09` the table, `CK10` a Rust enum and its Warrant
`variant` render to different lines (generated from both sides and compared; the lines are rows of the table), `CK11` the model-SAT lane's files (VOS-040c; see below).  On the real scheduler contract
(N = 8, ten lemmas) it takes about 10 s wall (57 s CPU, 8 workers) in a debug build; this crate's tests use a toy contract with two
threads (`tests/fixtures/contracts/vos-fixture-contract`) so they stay fast.

## Inputs

```text
<impl>/vos.toml                     kind = production | reference | any, contract = "<pkg>", obligations = [tiers] (any, canonical, policy, representation, safety)
<impl>/warrant/impl.part            the hand-written implementation block
<contract>/vos.toml                 kind = "contract"
<contract>/contract.vos             the tiers and the instance (parsed with the vos_contract grammar)
<contract>/warrant/contract.part    the contract block
<contract>/warrant/obligations-<tier>.part
<contract>/warrant/lemmas.part              optional: admitted, never embedded, never required (VOS02 R1.4)
```

## Checks (VOS02 10.1, D-GD8)

`GD01`-`GD05` judge the inputs; `CD01`-`CD08` judge the document: marker skeleton (a
grammar, not a scanner), digests, copies equal the contract's parts, `warrant-grammar`
parse and type check, a weakened claim with an obligation's name (`claim_sha256`),
`carries` **computed from the parsed claims**, the R6 rules a document alone decides
(`-any` carries no `canonical`; production carries it and no `any` block; every field of
`Impl` in the cone of `build`; a reference has no `bound`), and byte-for-byte regeneration.
`gen-doc` runs all but regeneration on what it is about to write and writes nothing on a
finding.

### The representation tier (VOS02 2.7, VOS-D061)

An implementation whose state has history that `alpha` forgets (a rotation pointer) carries the
contract's `representation` tier **instead of** `canonical`: eight claims in the contract package,
over `rep(m, h)` and `hist(s)`, models the implementation supplies.  `vos-link` checks no claim of
the implementation's (a syntactic lint over authored claims does not converge): a production crate
carries exactly one of the two tiers and no `any`, an `any` crate carries neither, a missing or
mistyped `rep`/`hist` is a CD04 type error, and a tampered claim is a digest mismatch.  The mode
is derived from `carries` (`mode representation` in the header); there is no `mode` key.

## What a pass means

The R6 cone rule is a **structural lint** (a taint walk over `build`: a field set from a
constant, `x - x`, `* 0`, `bit_and(.., 0)` or an ignored argument is outside the cone); it is
enforced at document level, not proof that the state has no oracle.  `kind`, `contract` and
`obligations` are author-declared until the registry (VOS-D030) and `check-profile`
(VOS-034) exist, so a production crate that lists only `["canonical"]` or an oracle crate
that declares `reference` is not caught here.  What closes the residual risk, an oracle write
path, is the bindings check (VOS-D060, required before any image profile), SH13/SH16 and the
registry.  Every pass prints this as a `note:` line (a representation carrier gets the representation note instead: the cone is not applied there, and the tier does not separate kinds).

## Safety

Paths have no `..` and no symlink (the Windows `\\?\` prefix of a canonical path is dropped before comparing); untrusted strings (file names, `vos.toml` values, document headers) are quoted and escaped in every report; the temporary file name is unique per process and call; files are lstat'ed, opened `O_NOFOLLOW | O_NONBLOCK` and
re-checked on the open handle; caps are checked before reading; the one write goes through a
fresh temporary file and a rename.  The Warrant analysis runs on a 256 MiB stack because
`warrant-grammar`'s checker overflows a small stack on a long expression chain (VOS-D057).

## Tests

`cargo test` (from this directory): `gen_and_check` runs the real `vos-sched-contract`
package through four fixture implementations (`tests/fixtures/impls`) and compares the
output with a golden file (`VOS_LINK_BLESS=1 cargo test` accepts an intended change; the value must be exactly `1` and it is refused when `CI` is set);
`negative_controls` covers NC1, NC2, NC4, NC4b, NC5, NC17 and the document-level NC18;
`paths_and_io` the caps and path containment; `cli` the exit codes; `bounds` (7): CC09's pure comparison and the derived mapping; `publish` (21): `check-publish`'s controls, the real tree and the command line; `component` (84 tests; 9 are CC09 controls): a fixture crate generated by `gen-build`, then one negative control per rule and per bypass of the first draft, each asserting its own code and text; `contract` (58 tests; 5 are CC10 controls): the toy contract generated by `gen-contract`, then one negative control per `CK` code and `check-component` on a contract crate, including the `echo` and `|| true` tricks, which fail by byte inequality.  The symlink, FIFO and permission controls (NC18) are Unix-only
(`cfg(unix)`); on Windows the path logic is covered by pure-string tests of the verbatim-prefix
handling and the lstat checks.  Line coverage is above
95% (llvm-cov, hand-instrumented).  The fixtures are not workspace members.

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
```

| Exit | Meaning |
|---|---|
| 0 | no findings |
| 1 | findings (a rule failed on bytes that were read and admitted) |
| 2 | no verdict (an unreadable or oversized file, a symlink, a path outside the repository, a usage error) |

## Inputs

```text
<impl>/vos.toml                     kind = production | reference | any, contract = "<pkg>", obligations = [tiers],
                                    mode = "representation" (optional; absent = canonical; never on kind any)
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

### Representation mode (VOS02 2.7, VOS-D061)

`mode = "representation"` (production or reference; a `GD05` on kind any, on a contract package,
on an unknown value, and on `canonical` in `obligations`) replaces the canonical and cone rules of
CD07 by **CD07-R**: no `canonical` and no `any` claim and no `-any` name; `build` is an `Impl(..)`
with at least one field outside its cone (else "declare canonical"); a model `rep` that is an
`Impl(..)` whose every field depends on its parameters; the claims `rep_init`, `rep_alpha`, one
`<op>_rep` per operation of `contract.vos` and at least one `<op>_det`; none mentions `build`;
every operation in them is applied to `rep(..)`; no right-hand side of `==` in a `_det` claim
calls an operation anywhere (a `_det` is a conjunction of `==` equations).  Each required claim must be
about its operation (`<op>_rep`/`<op>_det` call it on a `rep(..)` state; `rep_init` mentions `init` and
`rep`; `rep_alpha` applies alpha); no binder has a refinement; every `Impl`-typed argument of an
operation is `rep(..)` or a real image chain; every `rep(..)` takes distinct quantifier variables
(`rep_init` excepted); `rep` has a parameter per field and the history depends on a parameter the cone
fields do not.  All of it is syntactic: a lint, with human review, SH13/SH16, D030 and D060 as the
authority.  The report header prints `mode representation`, and each representation
claim is printed as `impl-own rep <name> claim_sha256=<hex>` for the future lock.  The fixture
`tests/fixtures/impls/vos-fixture-rep` is the reference shape.

## What a pass means

The R6 cone rule is a **structural lint** (a taint walk over `build`: a field set from a
constant, `x - x`, `* 0`, `bit_and(.., 0)` or an ignored argument is outside the cone); it is
enforced at document level, not proof that the state has no oracle.  `kind`, `contract` and
`obligations` are author-declared until the registry (VOS-D030) and `check-profile`
(VOS-034) exist, so a production crate that lists only `["canonical"]` or an oracle crate
that declares `reference` is not caught here.  What closes the residual risk, an oracle write
path, is the bindings check (VOS-D060, required before any image profile), SH13/SH16 and the
registry.  Every pass prints this as a `note:` line (a representation crate gets the CD07-R note instead:
the same limits, and the claim that a mislabelled oracle nothing moves passes the lint).

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
`paths_and_io` the caps and path containment; `cli` the exit codes.  The symlink, FIFO and permission controls (NC18) are Unix-only
(`cfg(unix)`); on Windows the path logic is covered by pure-string tests of the verbatim-prefix
handling and the lstat checks.  Line coverage is above
95% (llvm-cov, hand-instrumented).  The fixtures are not workspace members.

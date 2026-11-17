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
`paths_and_io` the caps and path containment; `cli` the exit codes.  The symlink, FIFO and permission controls (NC18) are Unix-only
(`cfg(unix)`); on Windows the path logic is covered by pure-string tests of the verbatim-prefix
handling and the lstat checks.  Line coverage is above
95% (llvm-cov, hand-instrumented).  The fixtures are not workspace members.

# pif-check

The release gate for Proof Interchange Format claims
([VER32](../../../specs/VER32-proof-interchange-format.md) section 5, backlog item
PIF-003). It is part of the trusted base under the id `tcb.pif.gate-binding`, so it is
written like the kernel: literate comments, `#![forbid(unsafe_code)]`, no third-party
dependencies, fail closed, no panics on any input.

The kernel (`lean-kernel`) decides whether a declaration is well typed. It does **not** decide
which files it was shown or which theorem is accepted for which claim. This crate does.

```text
 digest list ──► resolver ──► reader ──► kernel ──► gate checks ──► report
 (sign-off)      SHA-256      lean-kernel pif::read   goal digest,
                 verified     (trusted)               footprint, buckets
```

## What is here (PIF-003a)

| Module | Role |
|---|---|
| `resolver` | `Source` (untrusted: a directory of `<64 hex>.pif` files, or memory), `Resolver` (verifies SHA-256 of every file before use: `DigestMismatch`, R-IMP-2 / E5). No network, no search path, symbolic links refused, size capped before reading |
| `control` | The chain, sign-off and allow-list files, one strict line format; roles and the shape rules E1 to E4 |
| `chain` | `load_chain`: for each file, resolve, check the allow-list (E4), read with the right `FileClass` and `GateInputs` (imports = digests of all earlier files, E2), fold the environment digests (E3, 8.3) |
| `cli`, `report`, `json` | `pif-check --store <dir> --chain <file> [--signoff <file>] [--allow <file>]`; JSON report on stdout; exit 0 accepted, 1 refused, 2 usage |

### Chain file

```text
pif-chain 1
prelude sha256:<64 hex>
adl     sha256:<64 hex>        (zero or more)
library sha256:<64 hex>        (zero or more)
model   sha256:<64 hex>
axioms  sha256:<64 hex>        (optional)
claim   sha256:<64 hex>        (last; not on the allow-list by design, E4)
```

Roles must appear in that order; the prelude comes first; a claim file anywhere but last is
`ClaimNotLast`. The role fixes the reader's `FileClass`: prelude, adl, model and axioms are
*reviewed* (R-NAM-3, ASCII names), a library is not, the claim gets the strictest rules.

### Sign-off file

VER31 does not define a sign-off file yet; this is the gate's reading of what E1, E6, G6 and
PIF-020 need one to carry:

```text
pif-signoff 1
base <role> sha256:<hex>      the signed base chain, one line per file (E1)
base-environment <hex>        the base environment digest the signer recorded (E6)
goal <claim-id> <hex>         a signed goal digest (checked from PIF-003b)
goals-sha256 <hex>            the signed aggregate (PIF-020)
```

With `--signoff`, the base chain comes **from the sign-off**; the chain file may list only the
claim, or repeat the base exactly.

### Digests

`file_sha256` is the SHA-256 of the exact bytes. The environment digest folds over the chain:
`H0 = SHA-256("pif-env-1\n")`, `Hi = SHA-256(H(i-1) || file_sha256_i)`, with `||` the
concatenation of the **raw 32-byte** values (VER32 8.3 does not say raw or hex; raw is the
choice made here). The base digest folds over everything but the claim file; the full digest
continues over it. Digests print as 64 lowercase hex digits.

### Vetting the base, the allow-list and the sign-off (security review)

* **The base must be vetted.** Without `--allow` or `--signoff` the gate refuses (`"ok":false`,
  error `Unvetted`, exit 3) unless `--unsafe-no-allow` is given. The report's top-level
  `"base_trust"` is `"signed"`, `"allow-listed"` or `"unvetted"`.
* **A sign-off is complete or refused.** It must carry `base-environment`, at least one `goal`
  line (unique, G3-valid ids) and `goals-sha256`. The report's `"signoff"` object says what was
  checked (`base_environment`, `goals`, `aggregate`: `checked`, `pending` or `absent`).
* **At least one claim file** is required unless `--allow-no-claim` is given.
* **Allow-list lines are `<role> <digest>`.** The role is bound, so an allow-listed model cannot be
  loaded as a `library` to skip the reviewed-file checks. The environment fold does not include
  roles (VER32 8.3 is unchanged); the **sign-off's signature must cover the whole sign-off, roles
  included**, since the `base <role> <digest>` lines are what fix each file's role.
  Note for a VER32 follow-up: E1 should say the signed base chain records roles, and 8.3 that
  roles are covered by the sign-off, not by the environment digest.
* **`--withdrawn-logic n,n,...`** is gate configuration passed to the reader (R-VER-1); an empty
  list withdraws none.
* Control files and store entries must be regular files (no symbolic links, FIFOs, devices).
## Goals and the canonical term encoding (PIF-003b)

A claim file binds a claim id to a theorem (`goal "<id>" <theorem> sha256:<hex>`). The kernel
checked the proof; the gate decides whether the theorem's *statement* is the signed one.

| Module | Role |
|---|---|
| `cte` | The canonical term encoding of VER32 3.6 in two forms. The **binding CTE** (binder names and info erased, levels syntactic, 1 MiB cap) is computed **from the kernel store**; the **display CTE** keeps names. `Dterm` holds a term as a tree, built and emitted with explicit stacks (no recursion on attacker-controlled depth), with a strict parser for the display form |
| `goals` | G4/G5: recompute each goal digest from the theorem's type and compare it with the record's (a hint); the **goals list** (one line per claim: claim id, one space, display CTE, LF) parsed and checked with `erase(display) = binding`; sign-off goal digests; the **goal-signing aggregate** (PIF-020); R-GOL-4 again from the stored goal term |

The worked example's goal digest, recomputed from the kernel store, is
`bf1fa0a0d828b019965f46efa44ff2627bf54a59558a1bc2959b52c161f74676`, the value in VER32 4.6.

### Pinned byte layout

VER32 gives the CTE grammar and says STRING and NAT are "written exactly as in PIF (4.1)".
This crate writes a NAT in decimal with no leading zero, and a STRING in double quotes with
printable ASCII 0x21 to 0x7E as itself except `"` and `\` (`\"`, `\\`) and every other
character (space, controls, non-ASCII) as `\u{h}` in lowercase hex without leading zeros. The
parser accepts exactly those spellings, so a name has one encoding. A numeric name component
prints as digits (`<"A",3>`), a string component `"3"` as `<"A","3">`.

### Goals list and aggregate

```text
gates.and_comm Pd(<"a">,C<"Bool">[],Pd(<"b">,C<"Bool">[],A(A(A(C<"Eq">[s(z)],...
```

The aggregate is `SHA-256` over the lines `claim-id goal-digest base-environment-digest`
(digests as 64 lowercase hex digits), each ended by LF, sorted bytewise. VER32 does not fix
the terminator or the digest spelling; this is the choice made here. `pif-check --goals
<file>` adds the list; the report prints each claim's `goal_digest` and `goals_sha256`.

## Running

```sh
cargo test -p pif-check
cargo run -q -p pif-check -- --store ./store --chain chain.txt --allow allow.txt
```

## Not yet here

Goal digests and the canonical term encoding (PIF-003b), footprints, axiom classification
and the full report (PIF-003c), and the printer `pif-pp` with its round trips (PIF-003d).
`pif-eval` and `pif-fmt` of PIF-003 are separate untrusted tools and are not part of this crate.

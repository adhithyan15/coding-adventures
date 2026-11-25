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

## What is here

PIF-003 in four parts: 003a resolver, chain and CLI; 003b canonical term encoding and goal
digests; 003c footprints, classification and the report; 003d the printer, the round trips
and G7.

### Resolver, chain and CLI (PIF-003a)

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

### Sign-off comparison and limits (security review)

`goals_sha256` is always printed (the aggregate the gate computed); `goals_sha256_verified` is
true only when a sign-off supplied an aggregate to compare with. A goals list may have at most
100,000 lines, and claim lookups use hash maps, so a hostile list cannot make the gate quadratic.
## Footprints, classification and the report (PIF-003c)

| Module | Role |
|---|---|
| `footprint` | The axioms a theorem's type **and** value depend on, transitively, over the whole chain; a constant's footprint includes itself (G2). Explicit stacks, a visited set over names and hash-consed expressions, deterministic order. Inductives contribute constructors and recursor, recursors their rules |
| `registry` | The axiom registry file and its exact-match rule (A2): `(name, universe parameters, binding-CTE digest of the type, environment-prefix digest)`. Builds the reader's `GateInputs::axiom_registry` per file, with that file's prefix |
| `claims` | The per-claim report fields: claim id, theorem, goal digest, base and full environment digests, footprint with classes and buckets, kernel crate version and commit, certificate id `pif:sha256:<claim file digest>`, `rungs: "pending"` |

**Classification today.** A `Warrant.Assume.*` or `Src.*` axiom that exactly matches a
registry entry is a *registered* (c) assumption (a `Src.*` one is never bucket (a), and the
report says "relative to Lean + these axioms; consistency with the kernel logic not
established"). **Every other axiom, including `propext`, `Classical.choice` and
`Quot.sound`, is an unclassified (c) assumption**: A1 needs a validated `axiom.*` `prim`
role, roles are kernel slice 4, and until then the gate fails closed.

`footprint::Footprinter` memoises per-node footprints, so `claim_reports` walks the shared prelude
once however many claims there are.

### Registry file

```text
pif-registry 1
axiom <name CTE> <params> <type digest> <prefix digest>
axiom <"Warrant","Assume","gates","nand"> - <64 hex> <64 hex>
axiom <"Src","hol","select"> <"u">,<"v"> <64 hex> <64 hex>
```

`<params>` is `-` or comma-separated name CTEs. Only `Warrant.Assume.*` and `Src.*` names are
accepted. `pif-check --registry <file>`.

### Kernel version and commit

`claims::KERNEL_VERSION` is written by hand (a cargo dependency does not expose its version
at compile time) and a test compares it with `lean-kernel`'s manifest. The commit is read
from the build environment variable `PIF_KERNEL_COMMIT` and is `"unknown"` without it.

## The printer `pif-pp`, the round trips and G7 (PIF-003d)

A person signs a *printed* goal and reviews *printed* definitions, so the printer decides
what they see (`tcb.pif.printer`). VER32 lets it stay outside the trusted base only if the
gate checks that printing loses nothing. The gate does:

| Module | Role |
|---|---|
| `printer` | Terms to text: `forall (a : T), ...`, `fun (x : T) => ...`, `A -> B` for an unused `_` binder, `let x : T := v; b`, `Sort l`, constants with `.{levels}`, bound variables as `#n`. Deterministic, iterative, refuses shadowed binders, unprintable names, keywords as names, literals and projections |
| `ppparse` | The **independent** parser of that syntax (a stack machine; shares only the `Dterm` data type with the printer) |
| `filemodel`, `filetext` | A file as a person reads it: decoded **from the PIF text with the file's own binder names** (not through the kernel store, which keeps the first-interned names), printed one line per declaration, and parsed back |
| `printcheck` | `parse(print(G)) = G` for every goal; `parse(print(F)) = F` for every prelude, ADL, model and axioms file; and the decoded file must agree with what the kernel accepted, declaration by declaration (binding CTE of every type and value, kinds, hints, parameters, the set of declared names) |
| `names` | G7: every name in a goal's display CTE is in the ASCII class; binder names are single-component, pairwise distinct (`_` exempt) and not constants of the environment |
| `pp_cli` | `pif-pp file <path>`, `pif-pp check <path>`, `pif-pp goal <display CTE>` |

`load_chain` runs the file round trip on every reviewed base file, and `check_goals` runs G7
and the goal round trip on every goals-list line (and, with no list, the constant and
universe-parameter part of G7 on the stored goal term).

**Why the parser is hand-written and not generated by `grammar-tools`.** For a trusted gate it
is the same conflict as VER32 Q1: a generated parser puts a lexer engine, a parser engine and
a grammar compiler into the trusted base, and the code that turns a parse tree into a
de Bruijn term would still be hand-written. The syntax is small and unambiguous, the parser
is a stack machine, and the round trip is the differential. The grammar in `printer.rs` is
written so a `.grammar` file can be derived from it if `grammar-tools` is later audited to
that level.

**Shadowing.** Variables print as `#n`, so a repeated binder name cannot change a meaning; the
rule is a readability and review rule. The printer refuses a repeated name when the text can
then refer to either binder (the new one is used in its body, or the old one is used inside
the new scope). Two binders that are never referred to inside the inner scope are allowed:
the committed prelude writes every arrow as a binder named `a`, so `Prop -> Prop -> Prop` is
two unused `a` binders. G7 is stricter for goals: all binder names are distinct.

## Running

```sh
cargo test -p pif-check
cargo run -q -p pif-check -- --store ./store --chain chain.txt --allow allow.txt
cargo run -q -p pif-check --bin pif-pp -- file ../../specs/data/pif/prelude.pif
```

## Not yet here

* The VER31 rungs (`rungs` prints `"pending"`) and the `axiom.*` role bindings of A1
  (kernel slice 4): every axiom in a footprint is an unclassified (c) assumption until then.
* `pif-eval` and `pif-fmt` of PIF-003 are separate untrusted tools and are not part of this crate.

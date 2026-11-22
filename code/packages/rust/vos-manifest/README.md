# vos-manifest

The **pure parsers and document assembly** of the verified-OS program (backlog VOS-033,
spec [`code/specs/VOS02-interface-contracts-and-profile-linking.md`](../../../specs/VOS02-interface-contracts-and-profile-linking.md)
sections 2, 6.8, 10 and 10.1).  Crate class **P** (parser, VOS01 section 1.6): bytes in,
values out, no filesystem, no clock, no environment.  The program
[`vos-link`](../../../programs/rust/vos-link) owns every file and calls this library.

## What it does

| Module | Input | Output |
|---|---|---|
| `contract` | `contract.vos` bytes | a `Contract` (tiers, `config`, ceilings), its **canonical text** and digest; caps, word classes, sorting, one spelling |
| `part` | a `.part` / `impl.part` | an admitted `Part`: ASCII, LF only, no control byte, no marker anywhere in a line, within caps |
| `document` | admitted parts | `assemble` writes the generated `code/specs/<impl>.warrant`; `parse_layout` reads its marker skeleton back |
| `warrant` | a document or a block of claims | every claim with its `claim_sha256`, the `assume` rows, `bound` rows and the shape of `build` (via `warrant-grammar`) |
| `digest`, `caps`, `grammar` | | SHA-256 hex, the VOS02 section 6.8 caps, the grammar pairs |

## No hand-written lexers or parsers

Both text formats are token + parser grammars under `code/grammars/`:

* [`vos_contract`](../../../grammars/vos_contract): `contract.vos`.
* [`vos_document`](../../../grammars/vos_document): the seven marker comments of a
  generated document and the order they come in.  Any other line that mentions
  `-- vos02:` is a token with no place in the grammar, so a forged, repeated, missing,
  moved or unknown marker is a parse error.

They are read by the generic `GrammarLexer` / `GrammarParser`.  Warrant text is read only by
`warrant-grammar`.  What a grammar cannot say is a small, named post-parse check (word
classes, `u64` range, sorted tiers, canonical re-emit).

## Usage

```rust
use vos_manifest::contract::parse_contract;
use vos_manifest::document::{assemble, parse_layout, AssembleInput};
use vos_manifest::part::admit_part;
use vos_manifest::Tier;

let contract = parse_contract(&std::fs::read("contract.vos")?)?;
assert_eq!(contract.emit().as_bytes(), std::fs::read("contract.vos")?.as_slice());

let c = admit_part(&std::fs::read("warrant/contract.part")?)?;
let i = admit_part(&std::fs::read("warrant/impl.part")?)?;
let s = admit_part(&std::fs::read("warrant/obligations-safety.part")?)?;
let doc = assemble(&AssembleInput {
    impl_package: "vos-ready-bitmap",
    contract_package: "vos-ready-contract",
    contract: &c,
    implementation: &i,
    obligations: &[(Tier::Safety, &s)],
})?;
let layout = parse_layout(doc.as_bytes())?;
assert_eq!(layout.contract.marker_sha256.as_deref(), Some(c.digest().as_str()));
```

`vos_manifest::warrant::bound_facts(text)` lists a document's `op` names and `bound` rows from the parsed tree (a bound
is `Some` only for an integer literal); `vos-link check-component` CC09 compares them with the contract's ceilings.

## `claim_sha256`

The identity of a claim (VER29 section 5.3): the SHA-256 of the claim's real tokens joined
by single spaces.  Comments and layout (`Indent`, `Dedent`, `Newline`) are not part of it.
VER29 slice 2 is not implemented, so the definition lives here (backlog VOS-D056); a pinned
golden digest in `tests/warrant.rs` was computed independently of the code.

## Fail closed, and the one known limit

Every function is total.  Byte caps are checked before a byte is lexed; a file over a cap is
reported as a cap (`is_cap()`), which `vos-link` turns into "no verdict".

**Known limit.** `warrant-grammar`'s checker recurses over the expression tree, and a chain
of a few hundred `and`s in a single expression overflows a 2 MiB debug stack (backlog
VOS-D057).  The Warrant token budget (65,536) bounds the depth; `vos-link` runs the analysis
on a 256 MiB stack, which covers that bound.  The library itself does not spawn threads.

## Tests

`cargo test -p vos-manifest` runs four suites: `grammars` (grammar-tools validation and the
engine facts the grammars rely on), `contract`, `parts_and_documents` (VOS02 negative control
NC17: forged, duplicated, missing and reordered markers; CR and control bytes; caps) and
`warrant` (claim digests, trailer, `build` cone).  Line coverage is above 95% (llvm-cov,
hand-instrumented).

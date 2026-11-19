# typescript-warrant-ir

`typescript-warrant-ir` is a small Rust development tool that connects the
versioned TypeScript AST parser to the existing Warrant Intermediate
Representation (WIR). It admits one bounded family of typed scalar functions,
lowers their actual AST nodes and operator tokens, runs the existing WIR type
checker, and lets existing WIR proof tests send those expressions through the
bit-blaster and SAT solver.

It is not a TypeScript compiler, is not a runtime dependency, and does not prove
arbitrary TypeScript. The first source fixture contains three bit helpers over
an unsigned 64-bit mask and a slot in `0..=63`. The adapter refuses all syntax
outside its documented subset. `BigInt(slot)` is accepted only for the direct
slot parameter; `~` is admitted only under a finite mask; and the only shift is
`1n << BigInt(slot)`. The parser currently accepts some `return` line breaks
that JavaScript treats as automatic semicolons, so this adapter checks original
trivia and refuses those cases itself.

```rust
use typescript_warrant_ir::lower_typescript_bytes;

let source = br#"
export function bit_is_set(mask: bigint, slot: number): boolean {
  return (mask & (1n << BigInt(slot))) !== 0n;
}
export function set_bit(mask: bigint, slot: number): bigint {
  return mask | (1n << BigInt(slot));
}
export function clear_bit(mask: bigint, slot: number): bigint {
  return mask & ~(1n << BigInt(slot));
}
"#;
let typed_wir = lower_typescript_bytes(source).expect("admitted source");
assert_eq!(typed_wir.functions.len(), 3);
```

The fixture integration tests remain fixture-only. The development example
`source-proof` accepts an actual source path, an expected repository-relative or
absolute path, and its expected SHA-256; it reads and validates those exact
bytes before parsing. For example, from the Rust workspace:

```sh
cargo run -p typescript-warrant-ir --example source-proof -- \
  --source ../typescript/game-input-frame/src/digital.ts \
  --expected-path code/packages/typescript/game-input-frame/src/digital.ts \
  --sha256 <expected-sha256> --query healthy:Proved
```

The JSON response uses schema `typescript-warrant-ir/source-proof/v1` and
records the input path, resolved physical path, byte count, SHA-256, parser
profile, solver identity, WIR widths, expected and actual solver verdicts, and
(for a mutant) the complete decimal mask and slot witness. The mutant model's
`expected` value is the independent reference and `predicted` is the changed
helper's hand-evaluated result; neither is a public reducer execution. The
consumer separately reports that execution and compares the complete frame.
A mutant query must receive a caller-created mutated source path and digest plus
`--origin-path`, `--origin-sha256`, and `--mutation` equal to the selected query
name. Healthy queries refuse all origin and mutation flags. Supported query names are
`healthy:Proved`, `bit_is_set_not_equal:Counterexample`, `set_bit_or:Counterexample`,
and `clear_bit_complement:Counterexample`. The CLI never edits or substitutes
source bytes. Guarded mutant queries assert the public reducer call precondition
inside WIR before SAT. The CLI does not itself prove or replay the record-based
reducer; the consumer owns that separate public replay evidence.

See [VER09c](../../../specs/VER09c-typescript-source-wir-bridge.md) for the
complete subset, bounds, evidence obligations, and claim limits.

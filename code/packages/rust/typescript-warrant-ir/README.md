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

The source-bound integration proof reads the checked-in fixture with
`include_bytes!`, pins its path and SHA-256, proves each helper against an
independent per-bit reference, and admits supported wrong-operator mutants to
SAT. It decodes each witness and runs the original and mutated TypeScript bytes
under Node. This proves only those fixture helpers over their bounded domains;
it does not prove a future production reducer. GBL-050a must bind the consumer's
own `digital.ts` and replay witnesses through its public reducer.

See [VER09c](../../../specs/VER09c-typescript-source-wir-bridge.md) for the
complete subset, bounds, evidence obligations, and claim limits.

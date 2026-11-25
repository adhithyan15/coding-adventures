# pif-writer

An **untrusted producer library** for the Proof Interchange Format
([VER32](../../../specs/VER32-proof-interchange-format.md), backlog item PIF-004a). It lets a
program build a PIF file without hand-numbering records: you ask for names, levels and
expressions and get typed handles; you declare axioms, definitions, theorems, opaques,
inductive blocks and goals; `Writer::to_text` prints the canonical file.

```rust
use pif_writer::{BinderInfo, Level, Writer};

let mut w = Writer::new("native", "my producer 0.1").unwrap();
let prop = w.sort(Level::ZERO);
let p = w.name("P").unwrap();
w.axiom(p, &[], prop).unwrap();                 // axiom P : Prop
let pc = w.constant(p, &[]);
let x = w.name("x").unwrap();
let v0 = w.var(0);
let ty = w.pi(BinderInfo::Default, x, pc, pc);  // P -> P
let val = w.lam(BinderInfo::Default, x, pc, v0);
let id = w.name("id").unwrap();
w.theorem(id, &[], ty, val).unwrap();           // theorem id : P -> P := fun x => x
print!("{}", w.to_text());
```

## Where it fits

```text
 pif-writer (untrusted)  ->  PIF text  ->  lean_kernel::pif::read (trusted)  ->  kernel
```

The reader rechecks every byte, so this crate is never a trust anchor: a bug here can cause a
file to be rejected, not a false statement to be accepted. The converse promise is tested with
the real reader: for well-formed input the output is accepted.

## Canonical form

* dense, backward-only indices (a handle exists only after its record was emitted);
* maximal sharing: each distinct name, level and expression is emitted once, so asking again
  returns the same handle and emits nothing (binder names and infos count as part of the node);
* post-order, in creation order: the text is a pure function of the call sequence, so the same
  calls give byte-identical output (no hash table is ever iterated);
* the grammar's exact spacing and escaping (`\u{20}` for a space, lowercase hex, `\"` and
  `\\`), the `features` line (distinct, ascending; `lits`, `quotient` and `mutual` are added
  automatically), `import` lines and `end <count>`.

The VER32 4.6 worked example, rebuilt with this crate, is byte-identical to the fixture
`lean-kernel/tests/fixtures/worked-example.pif` (`tests/worked_example.rs`). The fixture's
numbering is creation order, so the test makes its calls in that order; for example the name
`b` is created after the constants of `Bool` and not before.

## Refusals

Builders that take strings return `Result` and refuse what the reader would reject: bad name
components (R-NAM-1), names over 32 components or 4096 decoded bytes (R-LIM-1, the reader's `name_bytes` cap), non-ASCII components when the writer is `ascii_names_only()` (R-NAM-3,
for reviewed files such as the prelude), the anonymous declaration name, duplicate universe
parameters, bad claim ids and digests, non-canonical numerals. `var` is bounded by the
kernel's `MAX_BVAR_INDEX`. What the crate does **not** check is typing: whether a theorem
proves its type is the kernel's business.

## Tests

`cargo test -p pif-writer`: unit tests of spelling, sharing and refusals; the worked example;
and property-style tests over 300 generated files (random levels, propositions, identity and K
proofs), each read by `lean_kernel::pif::read` under reviewed-file rules.

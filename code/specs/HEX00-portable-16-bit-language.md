# HEX00 — a portable 16-bit rung after Nib and Oct

Status: design, 2026-09-27. No Hex frontend or executable Hex compiler is
implemented by this document.

## Decision and learning goal

Hex is one source language with separate Z80 and Intel 8086 targets. The name
marks a 16-bit value as the next teaching step after Nib's 4-bit and Oct's
8-bit words. A learner can use the same `u16` source operation and inspect how
the 8086 performs it with a word instruction while the Z80 sometimes needs a
register-pair operation or several byte instructions. The targets share a
language contract; they do not share machine-code bytes or an ABI.

This is a **new** language rather than silently treating Oct source or 8008
ROMs as an 8080, Z80, or 8086 program. Oct remains an 8008 language. A later
Oct v2 may target 8080/Z80 independently; Hex's 16-bit type and stack-local
contract are a distinct increase in complexity.

| Candidate ladder | Benefit | Cost | Decision |
|---|---|---|---|
| Oct v2 for 8080/Z80, then an 8086-only language | Each source maps closely to one machine family | Two grammars and no common source program to reveal native versus synthesized word operations | Keep Oct v2 as a separate future extension |
| One Hex source with Z80 and 8086 targets | A single lesson and test corpus expose the two implementations of the same 16-bit behavior | Z80 needs multi-instruction word operations and both targets need real stack/call backends | **Choose for VM-072**, with narrow v0 semantics and staged proofs |

## What the repository can execute today

| Component | Z80 | Intel 8086 |
|---|---|---|
| Rust behavioral simulator | `z80-simulator`: 64 KiB memory, registers, flags, I/O, bounded runs and Python differential tests | `intel8086-simulator`: segmented 1 MiB memory, ModRM, registers, flags, I/O, bounded runs and Python differential tests |
| Python behavioral simulator | `z80-simulator` | `intel-8086-simulator` |
| Gate-level model | Rust and Python `z80-gatelevel` | Rust and Python `intel8086-gatelevel` |
| Rust encoder | `z80-encoder` exposes the small backend-used subset; `z80-simulator::encoding` contains more helpers | `intel8086-encoder` exposes the small backend-used subset; `intel8086-simulator::encoding` contains more helpers |
| Rust CIR backend | `z80-backend`: 8-bit constant in A, then HALT | `intel8086-backend`: 16-bit constant in AX, then HLT |

The four affected Rust simulator/backend suites passed locally when this
design was written. Their successful constant-return tests prove the wiring,
not Hex arithmetic, branches, stack locals, or calls. Existing simulators
can execute those instruction families, but both CIR backends still reject
the corresponding operations. The older `intel8086-backend` spec describes a
former curated simulator subset; the simulator itself and its current README
now implement the full specified Python-oracle surface. Reconcile that
documentation separately (VM-075).

The Z80 extends the 8080 instruction encoding, with flag differences that
matter to condition lowering. It is an 8-bit processor with 16-bit register
pairs, not an 8086 equivalent. Intel 8086 uses different machine code and
segmented addressing. Hex therefore compiles each target independently and
compares **source-level results**, never output bytes.

## Hex v0 source contract

Hex v0 uses the repository's grammar tools. Checked-in
`code/grammars/hex/hex.tokens` and `hex.grammar` will be the only grammar
authority; Rust lexer/parser wrappers will load those files. The syntax
follows Oct's braces, semicolons, `fn`, `let`, `if`, `else`, `while`, and
`return`, so the new lesson focuses on word width and memory discipline.

```hex
fn sum_to(n: u16) -> u16 {
    let sum: u16 = 0;
    let i: u16 = 1;
    while i <= n {
        sum = sum + i;
        i = i + 1;
    }
    return sum;
}

fn main() -> u16 {
    return sum_to(5); // 15 in HL on Z80, AX on 8086
}
```

- Types: `u8`, `u16`, and `bool`. `bool` is distinct and contains only
  `false`/`true`. Variables and parameters require annotations. Locals must
  be assigned before reading. An unsuffixed integer literal takes an expected
  `u8` or `u16` type from a declaration, return, argument, or the typed other
  operand of a binary expression, and must fit that width. With no such
  context it defaults to `u16` and must fit `0..65535`. An outer expected
  width propagates through a binary expression, so `let x: u8 = 1 + 2;`
  checks both literals as `u8`. Two untyped literals default to `u16` only
  when neither operand nor the enclosing expression supplies a width.
- `u8` arithmetic wraps modulo 256; `u16` arithmetic wraps modulo 65536.
  `+`, `-`, `&`, `|`, `^`, and `~` preserve operand width. Mixed-width binary
  expressions are errors. `u16(x)` zero-extends a `u8`; `u8(x)` keeps the low
  eight bits of a `u16`. Casts also accept an input already of the destination
  type as an identity. The cast's argument is checked without borrowing the
  destination's expected type, so `u8(0x1234)` first gives the literal its
  default `u16` type and then produces `0x34`. No implicit narrowing or
  signed interpretation.
- `==`, `!=`, `<`, `<=`, `>`, and `>=` compare unsigned operands of one
  width and return `bool`. `!`, `&&`, and `||` require `bool`; the latter two
  short-circuit left to right. `if` and `while` require `bool`.
- Functions have fixed, typed parameters and one `u8`, `u16`, `bool`, or
  `void` result. Calls may be nested; recursion and indirect calls are
  excluded from v0 so maximum stack usage can be computed from the call graph.
- A program has exactly one zero-argument `main` returning `u8`, `u16`, or
  `bool`. Its return value is the observable result; v0 has no console,
  operating-system calls, port I/O,
  globals, arrays, pointers, dynamic allocation, interrupts, or inline
  assembly. These require separate memory and device contracts in later
  versions. Multiplication, division, shifts, and signed types are also
  deferred rather than silently synthesized.

The compiler must reject a constant outside its type, a missing return on
any reachable non-void path, uninitialized reads, incompatible operands,
recursion, unbounded static frame requirements, and unsupported target
operations with precise source positions. A bounded simulator run that
exhausts its step budget is a failure, not a passing result.

## Execution profile and ABI

Both targets use a 64 KiB v0 **logical** arena: code occupies offsets
`0x0000..0x7FFF`; stack starts at `0xFFFE` and grows down toward `0x8000`.
The linker rejects code above the lower half, a maximum acyclic call-chain
frame footprint that enters the code half, and target-relative branches or
addresses that cannot be encoded. Calls save a return address, and a callee
restores its frame before returning. This fixed profile makes stack capacity
visible to a beginner and avoids accidental code/stack overlap.

For the 8086 profile, startup sets `CS = DS = SS = 0`, `IP = 0`, and
`SP = 0xFFFE`; all v0 addresses are near offsets in segment zero. The full
20-bit physical address space and far calls are deliberately deferred. A
`u16` return is in AX, `u8`/`bool` in AL. For the Z80 profile, startup sets
`PC = 0` and `SP = 0xFFFE`; a `u16` return is in HL, `u8`/`bool` in A.
Both targets encode returned `false` as zero and `true` as one, with no
other boolean register values. The target-specific exit sequence halts while
preserving `main`'s result register for simulator inspection. Internal
functions use actual CALL/RET rather
than the present backends' `ret_*`-as-HALT shortcut.

All `u16` values stored on the stack use little-endian byte order, matching
both targets. Stack-frame slot placement, saved registers, and argument
passing are target-specific and must be pinned by executable call tests before
the first compiler implementation claims support. No object produced for one
target may be fed to the other target's simulator as a compatibility proof.

## Compiler and acceptance ladder

Hex will follow the shared frontend path: grammar tools → typed Hex AST →
IIR → existing inference/specialization → CIR → target backend → target ROM.
This keeps one source/type contract and two machine-code emitters. Each rung
is a separate reviewable PR. H0 passes frontend diagnostics and reference
execution while rejecting machine-code emission. H1–H4 are delivered only
after **both** Rust simulators execute their generated ROM under a finite step
limit and the expected result is asserted. Python simulators and gate-level
models are independent oracles for selected discriminating cases, not
substitutes for running the emitted ROM.

As in Oct, shared IIR may use wider storage slots, but every operation must
carry or enforce its source width before a value is observed. In particular,
H2 must test `u16` wrap and complement on both emitted targets; a widened
intermediate that accidentally returns `65537` instead of `1` is a failed
compiler even when the simulator itself is correct. Other LANG backends gain
Hex support only after they also satisfy these width and execution tests.

| Rung | New work | Required proof |
|---|---|---|
| H0 | Grammar, AST, type checker, IIR emission, exact diagnostics, reference interpreter; reject unsupported target emission | Parse/type errors and interpreter results for all cases below |
| H1 | Entry ABI, `u8`/`u16` constants, explicit conversions, return and halt; Z80 gains a two-byte constant return | `main` returns `0`, `42`, `0x1234`, `false`/`true` as 0/1, and `u8(0x1234)` yields `0x34` |
| H2 | Stack locals, width-correct arithmetic/bitwise operations and unsigned comparisons; Z80 synthesizes operations it lacks natively | `0xffff + 2 == 1`; zero-extend a `u8` local holding 255, then add 1 to get 256; `0xffff > 1`; local reassignment |
| H3 | Conditional and loop branches with short-circuiting | taken/untaken branches, a finite loop, and a trace that executes a dynamic `&&` right-hand branch when its left side is true but skips the same branch when its left side is false |
| H4 | Direct calls, parameters, frames, and static stack-depth analysis | `sum_to(5) == 15`, two nested calls, and compile-time recursion rejection |

The first **language-runs-on-both** claim belongs to H4. Earlier rungs may
advertise their exact subset, never the whole Hex v0 language. Each rung
adds source-to-ROM-to-simulator tests to the LANG matrix or an equally
CI-executed conformance suite, pins errors as well as successful results,
and preserves the existing Nib/Oct suites. After H4, memory arrays, port
I/O, 8086 far pointers, and Z80-specific instructions can be designed as
explicit later extensions rather than accidental differences in v0.
Because H3 has no effectful expressions, its short-circuit proof inspects
the emitted instruction trace as well as the result; a later effectful
extension needs an observable right-hand-side regression of its own.

## Open implementation decisions

The design fixes language behavior and the entry/result ABI. The first
backend implementation PR must choose and document the exact callee-saved
registers, parameter layout, and frame-pointer convention for each target,
then test nested calls before accepting them. It must also measure generated
code size and maximum frame use against the 32 KiB code/stack partitions.
If simulator and gate-level models disagree on an emitted instruction, log
the smallest ROM and defer promotion of that rung until resolved.

## References

- `OCT00-oct-language.md` — preceding 8008 language and corrected 8080
  portability boundary.
- `z80-backend.md`, `intel8086-backend.md`, and their current Rust package
  tests — present compiler limits.
- `07k-z80-simulator.md`, `07m-intel-8086-simulator.md`, and current Rust
  simulator completion tests — executable targets.
- Zilog, [Z80 Family CPU User Manual](https://www.zilog.com/docs/z80/z80cpu_um.pdf)
  — primary instruction/flag reference. The repository's simulator and
  backend tests remain the execution evidence for this project.

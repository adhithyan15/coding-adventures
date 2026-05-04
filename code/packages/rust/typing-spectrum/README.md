# `typing-spectrum`

**LANG22** — compilation strategy advisor for the optional-typing spectrum.

`typing-spectrum` bridges the type-tier information produced by
[`iir-type-checker`] and the compilation decisions made by `aot-core`,
`aot-with-pgo`, and `jit-core`.

## The optional-typing spectrum

LANG22 §"The optional-typing spectrum" classifies every LANG program
somewhere on a continuum from "fully typed" to "untyped":

| LANG22 label | `TypingTier` | Examples |
|---|---|---|
| Tier A — fully typed | `FullyTyped` | Tetrad, Algol, Rust, C+ |
| Tier B — partially typed | `Partial(fraction)` | TypeScript, Sorbet-Ruby, Hack |
| Tier C — untyped | `Untyped` | Twig, MRI Ruby, Python (pre-mypy) |

The same compilation pipeline serves all three tiers; what changes is
*when and how* the pipeline specialises.

## The five compilation modes

| Mode | Description | Deopt? | `.ldp` in? | `.ldp` out? |
|------|-------------|--------|-----------|------------|
| `TreeWalking` | Interpret `IIRModule` directly | No | No | No |
| `AotNoProfile` | Typed → native; untyped → runtime calls | **No** | No | No |
| `AotWithPgo` | Speculation + deopt anchors from `.ldp` | Yes | Yes | No |
| `Jit` | Tier-up from interpreter on call-count | Yes | No | Yes |
| `JitThenAotWithPgo` | JIT → save `.ldp` → AOT-PGO at release | Yes | Yes | Yes |

## Quick start

```rust
use interpreter_ir::module::IIRModule;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use iir_type_checker::infer_and_check;
use typing_spectrum::advisory::advise;
use typing_spectrum::mode::CompilationMode;

// Step 1: build your IIRModule (from a language frontend).
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    "i64",
    vec![
        IIRInstr::new("add", Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i64"),
        IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i64"),
    ],
);
let mut module = IIRModule::new("math", "tetrad");
module.add_or_replace(fn_);

// Step 2: infer + validate types.
infer_and_check(&mut module);

// Step 3: get the compilation advisory.
let adv = advise(&module);

// Fully-typed → AOT-no-profile (no JIT warmup needed).
assert_eq!(adv.recommended_mode, CompilationMode::AotNoProfile);
assert!(!adv.requires_deopt());
println!("{}", adv.summary());
```

## Canonical type names

LANG22 defines a universal vocabulary every LANG backend agrees on.
Map language-specific annotations to canonical IIR strings:

```rust
use typing_spectrum::canonical::map_frontend_type;

// Twig
assert_eq!(map_frontend_type("int",     "twig"),       Some("i64"));
// TypeScript
assert_eq!(map_frontend_type("number",  "typescript"), Some("f64"));
// Ruby / Sorbet
assert_eq!(map_frontend_type("Integer", "ruby"),       Some("i64"));
// Python / mypy
assert_eq!(map_frontend_type("None",    "python"),     Some("nil"));
```

## JIT promotion thresholds

```rust
use typing_spectrum::threshold::JitPromotionThreshold;
use iir_type_checker::tier::TypingTier;

let t = JitPromotionThreshold::for_tier(&TypingTier::Untyped);
assert_eq!(t.call_count, 100); // wait for 100 calls before JIT-compiling
```

## Stack position

```
typing-spectrum
 ├─ iir-type-checker  (provides TypingTier + infer_and_check)
 └─ interpreter-ir    (provides IIRModule, IIRFunction, IIRInstr)
```

## Related specs

- **LANG22** `code/specs/LANG22-typing-spectrum-aot-jit.md`
- **LANG11** `jit-profiling-insights` — consumes type observations this crate describes
- **LANG04** `aot-core` — acts on `CompilationMode::AotNoProfile` advisories
- **ldp-format** — the `.ldp` binary format this crate's advisories reference

# iir-linker

Static linker for `IIRModule`s — implements the LANG33 module system at the
InterpreterIR level.

## What it does

`iir-linker` takes a set of `IIRModule`s (each produced by a separate
compilation unit) and merges them into one self-contained module that can be
executed by `vm-core` or lowered to BEAM / WASM / JVM / CLR by the
`iir-to-*` backends.

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
use interpreter_ir::module_exports::{IIRExport, IIRImport};
use iir_linker::link;

// Module "math" exports "add".
let mut math = IIRModule::new("math", "twig");
math.entry_point = None;
math.add_or_replace(IIRFunction::new(
    "add",
    vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    "i64",
    vec![IIRInstr::new("ret_void", None, vec![], "void")],
));
math.exports.push(IIRExport::new("add"));

// Module "app" imports "add".
let mut app = IIRModule::new("app", "twig");
app.add_or_replace(IIRFunction::new(
    "main", vec![], "void",
    vec![IIRInstr::new("ret_void", None, vec![], "void")],
));
app.imports.push(IIRImport::new("math", "add", "any"));

let merged = link(&[math, app]).unwrap();
assert!(merged.get_function("add").is_some());
assert!(merged.get_function("main").is_some());
assert!(merged.imports.is_empty()); // self-contained
```

## API

| Function | Purpose |
|----------|---------|
| `link(modules)` | Full link: resolve + type-check + merge. Returns all errors. |
| `link_strict(modules)` | Fail-fast variant: returns the first error. |
| `verify_imports(module, providers)` | Pre-flight: check imports without merging. |
| `IIRLinker::new().link(modules)` | Struct API (same as `link`). |

## Renaming and function references

When two modules define the same private function name, the first module
keeps the bare name and later ones are renamed to `"<module>::name"`.
Renaming is only safe if every *reference* to the old name follows, so the
linker rewrites each module's function-name operands through that module's
rename map (its own renamed functions plus its imports):

| Op | Operand rewritten |
|----|-------------------|
| `call` | `srcs[0]` — callee name (`Operand::Var`) |
| `alloc_closure` | `srcs[0]` — closure body name (`Operand::Str`) |
| `task_spawn` | `srcs[0]` — spawned function name |
| `group_spawn` | `srcs[1]` — spawned function name |

The merged `entry_point` is mapped the same way.  Data operands such as
`const(Str(..))`, `global_load`/`global_store` names and `catch` kinds are
left alone.  This matters for Twig in particular: every module numbers its
lambdas from `__lambda_0`, so without the `alloc_closure` rewrite a second
module's closures would run the first module's lambda body (issue #18421).

The `fn` operand of `task_spawn` and `group_spawn` is a function *name*,
never a register holding a closure (LANG28 pins this).  The rewrite matches
operands by their text, so a register in that slot that happened to share
a renamed function's name would be renamed too.

## One owner per name

Renaming only works if every name the merge pass sees has exactly one
meaning.  Three inputs would break that, so `link` rejects them up front
(issue #18429) instead of quietly running the wrong function:

| Input | Why it is ambiguous | Error |
|-------|---------------------|-------|
| modules `left` and `right` both export `foo` | exports keep their bare name, so the merged module would hold two `foo`s and every `call foo` would run `left`'s | `ConflictingExport` |
| two input modules are both named `util` | import rewrites are keyed by module name, so each would get the other's imports | `DuplicateModuleName` |
| `app` has `entry_point = "start"` but no `start` | the bare name would bind to some other module's private `start` | `UnresolvedEntryPoint` |

Things that are *not* errors: the same private name in several modules
(renamed apart), one module holding two functions of the same name (Twig's
user `main` beside its synthesized `main` — references bind to the first),
and export aliases with distinct public names.

## Error types

```text
LinkError::Unresolved            — import has no matching export
LinkError::TypeMismatch          — import type annotations don't match export
LinkError::DuplicateExport       — one module exports the same public name twice
LinkError::ConflictingExport     — two different modules export the same public name
LinkError::DuplicateModuleName   — two input modules share a name
LinkError::UnresolvedEntryPoint  — entry point neither defined nor imported by its module
LinkError::UndeclaredCall        — call instruction targets neither local nor import
```

## Where it fits

```
Language Frontend
    ↓  IIRModule (per compilation unit)
iir-linker::link
    ↓  merged IIRModule
vm-core / iir-to-beam / iir-to-wasm / iir-to-jvm-class-file / iir-to-cil-bytecode
```

## Spec

`code/specs/LANG33-module-system.md`

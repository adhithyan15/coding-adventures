# twig-module-driver

**LANG56** — Multi-file module resolver for the Twig language.

Reads a root `.tw` file, resolves `(import …)` declarations recursively, compiles
each module, and links them into a single `IIRModule` ready for `twig-vm`.

## Why this exists

Every previous LANG milestone compiled a **single source string** to a single
`IIRModule`.  The self-hosted Twig compiler spans multiple files
(`compiler/lexer.tw`, `compiler/parser.tw`, `compiler/codegen.tw`, …).  This
crate bridges the gap by implementing:

1. **Resolve** — convert `(import compiler/lexer)` to an absolute file path.
2. **Compile** — run `twig-ir-compiler` on each source file.
3. **Link** — merge all `IIRModule`s with `iir-linker::link`.

## Module naming

Import names use **slash-separated paths** relative to a search root.  The `.tw`
extension is implicit:

| Import name | File |
|-------------|------|
| `compiler/lexer` | `<root>/compiler/lexer.tw` |
| `stdlib/io` | `<root>/stdlib/io.tw` |
| `utils` | `<root>/utils.tw` |

## Quick start

```rust
use twig_module_driver::compile_module_tree;
use std::path::Path;

// Compile a single-file program (no module declaration needed).
let module = compile_module_tree(Path::new("main.tw"), &[]).unwrap();

// Compile a multi-file program with an extra search root.
let stdlib_root = Path::new("/usr/local/share/twig/stdlib");
let module = compile_module_tree(
    Path::new("compiler/main.tw"),
    &[stdlib_root],
).unwrap();

// Then run with twig-vm:
// let result = twig_vm::run(&module).unwrap();
```

Or use the convenience wrappers in `twig-vm`:

```rust
use twig_vm::run_file;
use std::path::Path;

let result = run_file(Path::new("main.tw")).unwrap();
```

## Architecture

```
root.tw  →  BFS discovery
              │
              ├── lib_a.tw  →  parse
              │    └── shared.tw  →  parse
              └── lib_b.tw  →  parse
                   └── (shared.tw already visited)
              │
              ▼ DFS cycle detection
              │
              ▼ collect all fn names → extern_fns
              │
              ├── compile root.tw   (with extern_fns)
              ├── compile lib_a.tw  (with extern_fns)
              ├── compile lib_b.tw  (with extern_fns)
              └── compile shared.tw (with extern_fns)
              │
              ▼ iir_linker::link → IIRModule
```

## Design decisions (LANG56 v1)

- **Global-merge linking** — the linker merges all functions into one module.
  Per-function import checking (a module may only call functions it explicitly
  imported) is deferred to LANG57.

- **Two-phase BFS+DFS** — discovery is pure BFS (each file parsed once); cycle
  detection is a separate iterative DFS with three-colour marking.  Mixing cycle
  detection into BFS produces false positives for shared dependencies (two modules
  both importing the same library would look like a cycle to a BFS `pending`-set
  check).

- **Extern injection** — cross-module calls compile correctly because all function
  names from all modules are pre-registered as "externs" in the compiler.  The
  `iir_linker` resolves the actual call targets at link time.

- **Root module** — the root `.tw` file's `entry_point = Some("main")` is preserved.
  All other modules have their `entry_point` cleared so the linker uses the root's
  `main` as the program entry.

## Dependencies

- `twig-parser` — lex + parse `.tw` source
- `twig-ir-compiler` — lower Twig AST to `IIRModule`
- `iir-linker` — merge `[IIRModule]` into one
- `interpreter-ir` — `IIRModule`, `IIRExport` types

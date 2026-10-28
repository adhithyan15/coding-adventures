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

Inside the link, each file's **module name** is its file stem (`lexer` for
`compiler/lexer.tw`).  The linker rejects two modules with the same name,
so when two files share a stem (`a/util.tw`, `b/util.tw`) the later one in
discovery order gets a numeric suffix (`util~2`).  The name only appears as
the prefix of renamed private functions, e.g. `util~2::helper`.

## Exported names must be unique

Twig calls a function in another module by its bare name, and the linker
keeps every exported function's bare name.  So two modules may not export
the same name: `(module left (export foo))` next to
`(module right (export foo))` fails with
`ModuleDriverError::Link([LinkError::ConflictingExport { .. }])` instead of
quietly running `left`'s `foo` everywhere (issue #18429).  Same-named
*private* functions are fine — the linker renames them apart.

`main` is the one exception the driver handles for you.  A library's
entry point is cleared (only the root's `main` runs), and any export of
`main` from a library is withdrawn along with it.  That lets a file such as
`compiler/main.tw`, which exports `main` so it can run on its own, be
imported by a test root that also exports `main`.

## A module sees only what its imports export

Inside a module, a bare name from elsewhere resolves only if one of the
module's **direct** imports lists it in `(export …)` (issue #18432):

```scheme
; lib.tw
(module lib (export lib-val))
(define (helper) 1)          ; private
(define (lib-val) (helper))

; main.tw
(module main (import lib))
(lib-val)                    ; ok — exported by a direct import
(helper)                     ; compile error: unbound name "helper"
```

The same goes for a function *value* (`(let ((f helper)) …)`) and for the
exports of a module you only import indirectly — import it yourself.  A
function the module defines shadows an imported one of the same name.

Two layers enforce this.  The IR compiler gets only the visible names as
externs, so anything else is an `unbound name` compile error with a source
position.  The driver also declares each visible name as an `IIRImport`,
and the linker rejects any function reference that is neither defined nor
imported (`LinkError::UndeclaredCall`).  Before #18432 every function name
in the whole tree was an extern of every module, and the linker bound the
bare name to whichever function owned it — another module's private
`helper` included.

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
              ▼ per module: defined fns, exported fns
              │
              ├── compile root.tw   (externs = exports of lib_a, lib_b)
              ├── compile lib_a.tw  (externs = exports of shared)
              ├── compile lib_b.tw  (externs = exports of shared)
              └── compile shared.tw (externs = none)
              │     + one IIRImport per extern the module does not define
              │
              ▼ iir_linker::link → IIRModule
```

## Design decisions (LANG56 v1)

- **Global-merge linking** — the linker merges all functions into one module.
  Import checking is per module: a module may name exactly the exported
  functions of its direct imports (#18432).

- **Two-phase BFS+DFS** — discovery is pure BFS (each file parsed once); cycle
  detection is a separate iterative DFS with three-colour marking.  Mixing cycle
  detection into BFS produces false positives for shared dependencies (two modules
  both importing the same library would look like a cycle to a BFS `pending`-set
  check).

- **Extern injection** — cross-module calls compile because each module's
  visible names (its direct imports' exports) are pre-registered as "externs"
  in the compiler, and declared as `IIRImport`s so `iir_linker` can resolve
  and check them at link time.  (Until #18432 *every* function name in the
  tree was an extern of every module, private ones included.)

- **Root module** — the root `.tw` file's `entry_point = Some("main")` is preserved.
  All other modules have their `entry_point` cleared so the linker uses the root's
  `main` as the program entry.

## Dependencies

- `twig-parser` — lex + parse `.tw` source
- `twig-ir-compiler` — lower Twig AST to `IIRModule`
- `iir-linker` — merge `[IIRModule]` into one
- `interpreter-ir` — `IIRModule`, `IIRExport` types

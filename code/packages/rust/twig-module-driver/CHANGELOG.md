# Changelog — twig-module-driver

## [0.1.0] — 2026-05-14

### New crate (LANG56 — Multi-File Module Driver)

First release.  Implements the file-level driver that turns a root `.tw` source
file (and all its transitive imports) into a single, fully-linked `IIRModule`
ready for `twig-vm::run`.

#### `compile_module_tree(root_path, search_roots) -> Result<IIRModule, ModuleDriverError>`

Four-phase pipeline:

1. **Discovery** — BFS from `root_path`, reading and parsing each `.tw` file
   encountered.  Import names (e.g. `"compiler/lexer"`) are resolved to absolute
   paths by scanning the requesting file's directory and then `search_roots` in
   order.  Each module is parsed exactly once (canonical-path dedup).

2. **Cycle detection** — Iterative DFS with three-colour marking (White / Grey /
   Black) on the adjacency graph built during discovery.  A back-edge to a Grey
   node (module currently on the DFS stack) triggers `CircularImport`.  This is
   separate from discovery so that shared dependencies (two modules both importing
   the same library) do not produce false positives.

3. **Compilation with externs** — Every top-level function name from every
   discovered module is collected and injected into the compiler as "externs" via
   the new `twig_ir_compiler::compile_program_with_externs`.  This allows
   cross-module calls to compile to `call` instructions rather than failing with
   "unbound name".

4. **Linking** — `iir_linker::link(&[IIRModule])` merges all compiled modules into
   one self-contained `IIRModule`.  The root module keeps `entry_point = Some("main")`; all library modules have their `entry_point` cleared before linking.

#### `resolve_import(import_name, search_roots, requesting_file) -> Option<PathBuf>`

Converts a slash-separated module name (e.g. `"stdlib/io"`) to a canonical file
path by searching the requesting file's directory first, then each search root.
The `.tw` extension is always appended.

#### `ModuleDriverError` variants

- `Io { path, error }` — could not read a source file
- `Parse { path, error }` — source file contained a syntax error
- `Compile { path, error }` — source file failed the IR compiler
- `UnresolvedImport { import_name, searched }` — no `.tw` file found in any root
- `CircularImport { cycle_member }` — import graph contains a cycle
- `Link(Vec<LinkError>)` — `iir_linker::link` failed (usually a name collision)

#### Tests

13 unit tests covering:

- `resolve_import_finds_file_in_sibling_dir`
- `resolve_import_uses_explicit_search_root`
- `resolve_import_returns_none_for_missing`
- `single_file_no_imports` — backward-compat: plain Twig programs work unchanged
- `single_file_with_module_decl_exports_populates_iirexport` — compiler-level export check
- `two_file_import_library_function_callable` — root calls lib function post-link
- `three_file_chain_transitive_import` — root → lib-a → lib-b transitivity
- `shared_dependency_compiled_once` — shared lib compiled exactly once (no dup-fn error)
- `unresolved_import_returns_error`
- `circular_import_returns_error`
- `export_only_lists_declared_names` — compiler-level export filter check
- `empty_module_no_panic`
- `library_module_has_no_entry_point`

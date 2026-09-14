# Lazy synthetic-function emission breaks exact module-function-count assertions

BA2 (BASIC multi-item PRINT) appends two synthetic helper functions
(`__basic_print_uint`/`__basic_print_int`) to the module whenever a `PRINT`
renders a value. A pre-existing test (`compiles_def_fn_into_sibling_function`)
compiled a program that PRINTs and asserted `m.functions.len() == 2` (main +
the DEF FN sibling) — which silently became 4. It passed on a stale local test
binary but failed `build (ubuntu-latest)` on fresh CI (cf. the closurec
stale-binary lesson). LESSON: when a frontend change adds module-level functions
conditionally, grep the whole crate for `functions.len()` / `functions.iter().count()`
assertions and any downstream consumer (e.g. `basic-dap`) before pushing; assert
the *named* functions you care about (`functions[0].name == "main"`,
`.iter().find(|f| f.name == "FNS")`) rather than a brittle total count. Always
re-run the FULL crate test suite (not `--lib` alone, and not a possibly-cached
binary) after changing module assembly.

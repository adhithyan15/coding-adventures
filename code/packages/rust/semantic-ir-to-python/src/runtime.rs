//! Import header for the SIR Python runtime.
//!
//! The runtime semantics no longer live inline.  They ship in the published
//! `coding-adventures-sir-runtime-core` package (see `code/specs/sir-runtime.md`);
//! every emitted module imports them instead of pasting a prelude into the file.
//!
//! The import aliases each helper back to the historical `_sir_*` name the
//! emitter already uses, so `emit.rs` is unchanged — only the *source* of the
//! helpers moved from an inlined blob to an imported library.  `_sir_plus` etc.
//! map to the core's clean names (`add`, `sub`, …); `_sir_print` maps to the
//! core's `sir_print`.

pub const RUNTIME: &str = r##"# ── SIR runtime (imported from coding-adventures-sir-runtime-core) ──
from coding_adventures_sir_runtime_core import (
    truthy as _sir_truthy,
    intern as _sir_intern,
    apply as _sir_apply,
    make_closure as _sir_make_closure,
    global_set as _sir_global_set,
    global_get as _sir_global_get,
    global_get_static as _sir_global_get_static,
    call_builtin as _sir_call_builtin,
    builtin_closure as _sir_builtin_closure,
    add as _sir_plus,
    sub as _sir_minus,
    mul as _sir_times,
    div as _sir_divide,
    eq as _sir_eq,
    lt as _sir_lt,
    gt as _sir_gt,
    cons as _sir_cons,
    car as _sir_car,
    cdr as _sir_cdr,
    is_null as _sir_is_null,
    is_pair as _sir_is_pair,
    is_number as _sir_is_number,
    is_symbol as _sir_is_symbol,
    sir_print as _sir_print,
    to_display as _sir_to_display,
)
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_non_empty_ends_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_imports_from_core_package() {
        assert!(RUNTIME.contains("from coding_adventures_sir_runtime_core import"));
        // No inline prelude any more.
        assert!(!RUNTIME.contains("class Symbol"));
        assert!(!RUNTIME.contains("def _sir_truthy"));
    }

    #[test]
    fn runtime_aliases_every_helper_the_emitter_uses() {
        for alias in &[
            "truthy as _sir_truthy",
            "intern as _sir_intern",
            "apply as _sir_apply",
            "make_closure as _sir_make_closure",
            "global_set as _sir_global_set",
            "global_get as _sir_global_get",
            "global_get_static as _sir_global_get_static",
            "call_builtin as _sir_call_builtin",
            "builtin_closure as _sir_builtin_closure",
            "add as _sir_plus",
            "sub as _sir_minus",
            "mul as _sir_times",
            "div as _sir_divide",
            "eq as _sir_eq",
            "lt as _sir_lt",
            "gt as _sir_gt",
            "cons as _sir_cons",
            "car as _sir_car",
            "cdr as _sir_cdr",
            "is_null as _sir_is_null",
            "is_pair as _sir_is_pair",
            "is_number as _sir_is_number",
            "is_symbol as _sir_is_symbol",
            "sir_print as _sir_print",
            "to_display as _sir_to_display",
        ] {
            assert!(RUNTIME.contains(alias), "runtime missing alias `{}`", alias);
        }
    }
}

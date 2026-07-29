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

/// The OOP-runtime import header, appended **only** when a module uses an
/// object-orientation feature (classes/modules/instance vars/class vars/
/// constants or reflective `is_a?`-style dispatch).  Pure non-OOP modules
/// never gain a dependency on this package.  Each helper is aliased to a
/// `_sir_oop_*` name the emitter uses; see `code/specs/sir-runtime.md`.
pub const RUNTIME_OOP: &str = r##"# ── SIR OOP runtime (imported from coding-adventures-sir-runtime-oop) ──
from coding_adventures_sir_runtime_oop import (
    define_class as _sir_oop_define_class,
    ivar_get as _sir_oop_ivar_get,
    ivar_set as _sir_oop_ivar_set,
    cvar_get as _sir_oop_cvar_get,
    cvar_set as _sir_oop_cvar_set,
    call_method as _sir_oop_call_method,
    call_new as _sir_oop_call_new,
    call_super as _sir_oop_call_super,
    call_class_method as _sir_oop_call_class_method,
    def_method as _sir_oop_def_method,
    def_class_method as _sir_oop_def_class_method,
    include_module as _sir_oop_include_module,
    extend_module as _sir_oop_extend_module,
    current_self as _sir_oop_current_self,
    sym_to_proc as _sir_oop_sym_to_proc,
    case_eq as _sir_oop_case_eq,
)
"##;

/// The exception-runtime import header, appended **only** when a module uses
/// the `Exceptions` feature (a `try`/`rescue` or a `raise`).  Pure
/// non-throwing modules never gain a dependency on this package.  Each helper
/// is aliased to a `_sir_exc_*` name the emitter uses; see
/// `code/specs/sir-runtime.md`.
pub const RUNTIME_EXC: &str = r##"# ── SIR exception runtime (imported from coding-adventures-sir-runtime-exceptions) ──
from coding_adventures_sir_runtime_exceptions import (
    raise_error as _sir_exc_raise_error,
    rescue_matches as _sir_exc_rescue_matches,
    register_ancestry as _sir_exc_register_ancestry,
)
"##;

/// The pairs-runtime import header, appended **only** when a module uses the
/// `Pairs` feature (a `cons`/`car`/`cdr`/`pair?` builtin).  The cons-pair value
/// type now lives in its own `coding-adventures-sir-runtime-pairs` package
/// (core re-exports it for back-compat); pure non-pair modules never gain a
/// dependency on it.  Each helper keeps the emitter's historical `_sir_*` name;
/// see `code/specs/sir-runtime.md`.
pub const RUNTIME_PAIRS: &str = r##"# ── SIR pairs runtime (imported from coding-adventures-sir-runtime-pairs) ──
from coding_adventures_sir_runtime_pairs import (
    cons as _sir_cons,
    car as _sir_car,
    cdr as _sir_cdr,
    is_pair as _sir_is_pair,
)
"##;

/// The regex-runtime import header, appended **only** when a module calls the
/// `regex` builtin (a Ruby `/pat/flags` literal).  Provides a target-native
/// compiler with Ruby→Python flag translation; see `code/specs/sir-runtime.md`.
pub const RUNTIME_REGEX: &str = r##"# ── SIR regex runtime (imported from coding-adventures-sir-runtime-regex) ──
from coding_adventures_sir_runtime_regex import (
    compile as _sir_regex_compile,
    is_match as _sir_regex_is_match,
    match_data as _sir_regex_match_data,
)
"##;

/// The shell-runtime import header, appended **only** when a module calls the
/// `backtick` builtin (a Ruby `` `cmd` `` literal).  Provides a thin
/// subprocess wrapper that runs the command via the system shell and returns
/// its stdout; see `code/specs/sir-runtime.md`.
pub const RUNTIME_SHELL: &str = r##"# ── SIR shell runtime (imported from coding-adventures-sir-runtime-shell) ──
from coding_adventures_sir_runtime_shell import backtick as _sir_shell_backtick
"##;

/// The range-runtime import header, appended **only** when a module calls the
/// `range` builtin (a Ruby `a..b` / `a...b` literal).  Provides the first-class
/// `Range` value type (Python's `range` is half-open and integer-only and can't
/// model the inclusive or begin/endless forms); see `code/specs/sir-runtime.md`.
pub const RUNTIME_RANGE: &str = r##"# ── SIR range runtime (imported from coding-adventures-sir-runtime-range) ──
from coding_adventures_sir_runtime_range import range as _sir_range
"##;

pub const RUNTIME: &str = r##"# ── SIR runtime (imported from coding-adventures-sir-runtime-core) ──
from coding_adventures_sir_runtime_core import (
    truthy as _sir_truthy,
    intern as _sir_intern,
    apply as _sir_apply,
    make_closure as _sir_make_closure,
    as_lambda as _sir_as_lambda,
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
    ne as _sir_ne,
    lt as _sir_lt,
    gt as _sir_gt,
    le as _sir_le,
    ge as _sir_ge,
    is_null as _sir_is_null,
    is_number as _sir_is_number,
    is_symbol as _sir_is_symbol,
    sir_print as _sir_print,
    sir_puts as _sir_puts,
    to_display as _sir_to_display,
)

# Default-parameter sentinel (P2c).  SIR defaults are *call-time* and may
# reference *earlier* params, so Python's native def-time defaults are wrong
# for our model.  Instead a defaulted param's native default is this unique
# sentinel object: callers may omit the argument (it then binds the sentinel),
# and the function body's resolve-prologue replaces any still-sentinel param
# with its default expression — evaluated in the body, where earlier params
# are already in scope.  A fresh `object()` is distinct from every real value
# (including `None`), so it can never collide with a legitimate argument.
_SIR_MISSING = object()
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
            "as_lambda as _sir_as_lambda",
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
            "is_null as _sir_is_null",
            "is_number as _sir_is_number",
            "is_symbol as _sir_is_symbol",
            "sir_print as _sir_print",
            "sir_puts as _sir_puts",
            "to_display as _sir_to_display",
        ] {
            assert!(RUNTIME.contains(alias), "runtime missing alias `{}`", alias);
        }
    }

    #[test]
    fn oop_and_exc_runtime_import_their_packages() {
        assert!(RUNTIME_OOP.contains("from coding_adventures_sir_runtime_oop import"));
        // Mixin helpers (MX2) are aliased for the `__include__`/`__extend__` arms.
        assert!(RUNTIME_OOP.contains("include_module as _sir_oop_include_module"));
        assert!(RUNTIME_OOP.contains("extend_module as _sir_oop_extend_module"));
        assert!(RUNTIME_EXC.contains("from coding_adventures_sir_runtime_exceptions import"));
        assert!(RUNTIME_EXC.contains("raise_error as _sir_exc_raise_error"));
        assert!(RUNTIME_EXC.contains("rescue_matches as _sir_exc_rescue_matches"));
        assert!(RUNTIME_EXC.contains("register_ancestry as _sir_exc_register_ancestry"));
        assert!(RUNTIME_OOP.ends_with('\n'));
        assert!(RUNTIME_EXC.ends_with('\n'));
    }

    #[test]
    fn pairs_moved_to_dedicated_package() {
        // cons/car/cdr/pair? now ship in the pairs package, not core.
        assert!(RUNTIME_PAIRS.contains("from coding_adventures_sir_runtime_pairs import"));
        for alias in &[
            "cons as _sir_cons",
            "car as _sir_car",
            "cdr as _sir_cdr",
            "is_pair as _sir_is_pair",
        ] {
            assert!(RUNTIME_PAIRS.contains(alias), "pairs runtime missing `{}`", alias);
            assert!(!RUNTIME.contains(alias), "core runtime should not still alias `{}`", alias);
        }
        assert!(RUNTIME_PAIRS.ends_with('\n'));
    }
}

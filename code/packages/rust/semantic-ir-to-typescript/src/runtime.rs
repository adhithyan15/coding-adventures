//! Import header for the SIR TypeScript runtime.
//!
//! The runtime semantics no longer live inline.  They ship in the published
//! `@coding-adventures/sir-runtime-core` package (see `code/specs/sir-runtime.md`);
//! every emitted module imports them under the `__Sir` namespace instead of
//! pasting a `namespace __Sir { … }` block into the file.
//!
//! `import * as __Sir` binds both the value helpers (`__Sir.add`, `__Sir.truthy`,
//! `__Sir.cons`, …) and the types (`__Sir.Val`, `__Sir.Sym`, `__Sir.Pair`,
//! `__Sir.Closure`) the emitter references, so generated code keeps its
//! `__Sir.*` call sites — only the *source* of the runtime moved from an inlined
//! blob to an imported library.

/// The import header emitted at the top of every artifact.
pub const RUNTIME: &str = r##"import * as __Sir from "@coding-adventures/sir-runtime-core";
"##;

/// The OOP-runtime import, emitted **only** when a module uses an
/// object-orientation feature (classes/modules/instance vars/class
/// vars/constants or reflective `is_a?`-style dispatch).  Pure
/// non-OOP modules never gain a dependency on this package.  Bound as
/// `__SirOop` so the emitter's `__SirOop.*` call sites resolve; see
/// `code/specs/sir-runtime.md`.
pub const RUNTIME_OOP: &str = r##"import * as __SirOop from "@coding-adventures/sir-runtime-oop";
"##;

/// The exception-runtime import, emitted **only** when a module uses the
/// `Exceptions` feature (a `try/catch` or a `raise`).  Pure non-throwing
/// modules never gain a dependency on this package.  Bound as `__SirExc`
/// so the emitter's `__SirExc.*` call sites resolve; see
/// `code/specs/sir-runtime.md`.
pub const RUNTIME_EXC: &str = r##"import * as __SirExc from "@coding-adventures/sir-runtime-exceptions";
"##;

/// The pairs-runtime import, emitted **only** when a module uses the `Pairs`
/// feature (a `cons`/`car`/`cdr`/`pair?` builtin).  The cons-pair value type
/// now lives in its own `@coding-adventures/sir-runtime-pairs` package (core
/// re-exports it for back-compat); pure non-pair modules never gain a
/// dependency on it.  Bound as `__SirPairs` so the emitter's `__SirPairs.*`
/// call sites resolve; see `code/specs/sir-runtime.md`.
pub const RUNTIME_PAIRS: &str = r##"import * as __SirPairs from "@coding-adventures/sir-runtime-pairs";
"##;

/// The regex-runtime import, emitted **only** when a module calls the `regex`
/// builtin (a Ruby `/pat/flags` literal).  Bound as `__SirRegex`; provides a
/// native `RegExp` compiler with Ruby→JS flag translation.  See
/// `code/specs/sir-runtime.md`.
pub const RUNTIME_REGEX: &str = r##"import * as __SirRegex from "@coding-adventures/sir-runtime-regex";
"##;

/// The shell-runtime import, emitted **only** when a module calls the
/// `backtick` builtin (a Ruby `` `cmd` `` literal).  Bound as `__SirShell`;
/// runs the command via the system shell and returns its stdout.  See
/// `code/specs/sir-runtime.md`.
pub const RUNTIME_SHELL: &str = r##"import * as __SirShell from "@coding-adventures/sir-runtime-shell";
"##;

/// The range-runtime import, emitted **only** when a module calls the `range`
/// builtin (a Ruby `a..b` / `a...b` literal).  Bound as `__SirRange`; provides
/// the first-class `Range` value type (JavaScript has no range type at all).
/// See `code/specs/sir-runtime.md`.
pub const RUNTIME_RANGE: &str = r##"import * as __SirRange from "@coding-adventures/sir-runtime-range";
"##;

/// The symbolic-expression runtime import, emitted **only** when a module
/// uses the SIR23 symbolic/pattern domain (`Feature::SymbolicExpr` or
/// `Feature::PatternMatching` — a `SymApply`/`SymPatternBlank`/
/// `SymPatternNamed`/`SymRule`/`SymReplaceAll` node).  Pure non-symbolic
/// modules (e.g. a MATLAB program) never gain a dependency on it.  Bound as
/// `__SirSym` so the emitter's `__SirSym.*` call sites resolve; see
/// `code/specs/SIR23-symbolic-pattern-semantic-ir.md` §"Backend impact".
pub const RUNTIME_SYM: &str = r##"import * as __SirSym from "@coding-adventures/sir-runtime-symbolic";
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_non_empty_and_terminates_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_imports_core_namespace() {
        assert!(RUNTIME.contains(
            r#"import * as __Sir from "@coding-adventures/sir-runtime-core";"#
        ));
        // No inline runtime any more.
        assert!(!RUNTIME.contains("namespace __Sir"));
        assert!(!RUNTIME.contains("export function truthy"));
    }

    #[test]
    fn oop_and_exc_imports_bind_their_namespaces() {
        assert!(RUNTIME_OOP.contains(
            r#"import * as __SirOop from "@coding-adventures/sir-runtime-oop";"#
        ));
        assert!(RUNTIME_EXC.contains(
            r#"import * as __SirExc from "@coding-adventures/sir-runtime-exceptions";"#
        ));
        assert!(RUNTIME_OOP.ends_with('\n'));
        assert!(RUNTIME_EXC.ends_with('\n'));
    }

    #[test]
    fn pairs_import_binds_its_namespace() {
        assert!(RUNTIME_PAIRS.contains(
            r#"import * as __SirPairs from "@coding-adventures/sir-runtime-pairs";"#
        ));
        assert!(RUNTIME_PAIRS.ends_with('\n'));
        // cons/car/cdr no longer come from the core namespace import.
        assert!(!RUNTIME.contains("cons"));
    }

    #[test]
    fn symbolic_import_binds_its_namespace() {
        assert!(RUNTIME_SYM.contains(
            r#"import * as __SirSym from "@coding-adventures/sir-runtime-symbolic";"#
        ));
        assert!(RUNTIME_SYM.ends_with('\n'));
    }
}

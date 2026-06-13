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
}

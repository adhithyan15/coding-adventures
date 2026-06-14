//! `SourceLoc` — one source position attached to one IIR instruction.
//!
//! # Why this lives in `interpreter-ir`, not in a frontend crate
//!
//! Every LANG-pipeline frontend (Twig, Lispy, BASIC, Brainfuck, …) lowers
//! its AST to the same [`crate::IIRFunction`].  Every dev-tool that
//! consumes IIR — the LSP, the in-process debugger, the AOT codegen that
//! emits DWARF / PDB, the line-coverage instrumentation pass — needs the
//! same answer to the same question:
//!
//! > "Which `(line, column)` in the user's source produced
//! >  `instructions[i]`?"
//!
//! The answer is the i-th element of [`IIRFunction::source_map`].  Putting
//! the type here means every frontend records positions with the same
//! shape and every consumer reads them with the same shape — no per-
//! frontend bespoke representation, no per-consumer adapter.
//!
//! # The lockstep invariant
//!
//! `source_map[i]` corresponds to `instructions[i]`; the two vectors are
//! kept in **lockstep** — same length, parallel indexing.  A frontend
//! that emits an instruction must, in the same operation, push a
//! `SourceLoc` for it.  See the `FnCtx::emit` helper in
//! [`twig_ir_compiler::compiler`] for the canonical pattern.
//!
//! Lockstep is the simplest representation for downstream consumers — no
//! interpolation, no holes, no "previous mapped instr" lookups.  An
//! empty `source_map` is permitted only on functions where positions are
//! genuinely unknown (legacy callers, hand-built test fixtures).
//!
//! # Indexing — 1-based
//!
//! Every frontend in this repo emits 1-based line/column numbers (line
//! 1, column 1 is the first character of the source).  Editors,
//! compilers, and language-server protocols all converge on 1-based;
//! storing 0-based here would force every consumer to add 1 on the way
//! out.
//!
//! # The synthetic sentinel
//!
//! Some IIR instructions have no exact source counterpart — the trailing
//! `ret` synthesised by `Compiler::compile` for a program with no value-
//! producing top-level expression, for instance.  By convention these
//! synthetic instructions are tagged with **`SourceLoc::SYNTHETIC`**
//! (line `0`, column `0`).  Consumers can `if loc == SourceLoc::SYNTHETIC`
//! to suppress the entry from coverage reports / debugger line-step
//! tables.
//!
//! Frontends that *can* attribute a synthetic instruction back to a real
//! source construct (e.g. the closing `)` of a `(define ...)`) should
//! prefer that real position over the sentinel — the sentinel is the
//! "no information available" fallback, not the default.

// ---------------------------------------------------------------------------
// SourceLoc
// ---------------------------------------------------------------------------

/// One source position attached to one IIR instruction.
///
/// Stored in [`IIRFunction::source_map`](crate::IIRFunction::source_map)
/// in lockstep with [`IIRFunction::instructions`](crate::IIRFunction::instructions):
/// `source_map[i]` is the position that produced `instructions[i]`.
///
/// Coordinates are 1-based.  The sentinel [`SourceLoc::SYNTHETIC`]
/// (line `0`, column `0`) marks instructions with no real source
/// counterpart — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLoc {
    /// 1-based line number from the source frontend.  `0` means
    /// synthetic — see [`SourceLoc::SYNTHETIC`].
    pub line: u32,
    /// 1-based column number from the source frontend.  `0` means
    /// synthetic when paired with `line == 0`.
    pub column: u32,
}

impl SourceLoc {
    /// Sentinel for instructions synthesised by the compiler with no
    /// real source counterpart.
    ///
    /// `line == 0` AND `column == 0` is the contract — both fields zero.
    /// Consumers (LSP, debugger, coverage) should treat this as "skip".
    pub const SYNTHETIC: SourceLoc = SourceLoc { line: 0, column: 0 };

    /// Build a `SourceLoc` from any pair of integer types convertible
    /// into `u32`.
    ///
    /// The parser surfaces positions as `usize`; the type cast happens
    /// here so callers don't sprinkle `as u32` everywhere.  Values
    /// larger than `u32::MAX` are clamped to `u32::MAX` — a 4-billion-
    /// line source file is not a real-world input, and saturating is
    /// safer than wrapping.
    pub fn new(line: usize, column: usize) -> Self {
        SourceLoc {
            line: line.min(u32::MAX as usize) as u32,
            column: column.min(u32::MAX as usize) as u32,
        }
    }

    /// Is this the synthetic-instruction sentinel?
    pub fn is_synthetic(self) -> bool {
        self.line == 0 && self.column == 0
    }
}

impl std::fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_synthetic() {
            write!(f, "<synthetic>")
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps_oversize_values() {
        // A pathological caller passing usize::MAX should not panic;
        // clamp to u32::MAX so the SourceLoc remains constructible.
        let loc = SourceLoc::new(usize::MAX, usize::MAX);
        assert_eq!(loc.line, u32::MAX);
        assert_eq!(loc.column, u32::MAX);
    }

    #[test]
    fn new_passes_through_normal_values() {
        let loc = SourceLoc::new(42, 7);
        assert_eq!(loc.line, 42);
        assert_eq!(loc.column, 7);
    }

    #[test]
    fn synthetic_sentinel_is_zero_zero() {
        assert_eq!(SourceLoc::SYNTHETIC.line, 0);
        assert_eq!(SourceLoc::SYNTHETIC.column, 0);
        assert!(SourceLoc::SYNTHETIC.is_synthetic());
    }

    #[test]
    fn non_synthetic_loc_reports_not_synthetic() {
        let loc = SourceLoc::new(1, 1);
        assert!(!loc.is_synthetic());
    }

    #[test]
    fn line_only_zero_is_still_synthetic_only_when_column_also_zero() {
        // line=0 column=5 is NOT synthetic by contract — both must be 0.
        // (Frontends should never emit such a value, but if they do, we
        // treat it as a real position so the sentinel stays unique.)
        let loc = SourceLoc { line: 0, column: 5 };
        assert!(!loc.is_synthetic());
    }

    #[test]
    fn display_formats_normal_loc() {
        let loc = SourceLoc::new(3, 14);
        assert_eq!(format!("{loc}"), "3:14");
    }

    #[test]
    fn display_formats_synthetic_loc() {
        assert_eq!(format!("{}", SourceLoc::SYNTHETIC), "<synthetic>");
    }

    #[test]
    fn copy_clone_eq_hash_derived() {
        // SourceLoc is small (8 bytes) and stored in lockstep with
        // instructions — Copy + Clone + Eq + Hash all matter for
        // downstream consumers.  This test is a smoke check that the
        // derives compile and behave sensibly.
        use std::collections::HashSet;
        let a = SourceLoc::new(1, 1);
        let b = a;
        assert_eq!(a, b);
        let mut set: HashSet<SourceLoc> = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}

//! `IIRModule` — the top-level container for an InterpreterIR program.
//!
//! A module holds all functions compiled from a single source file (or REPL
//! session).  It is the unit handed to `vm-core.execute()` and
//! `jit-core.execute_with_jit()`.
//!
//! REPL sessions mutate the module incrementally: each new input appends new
//! `IIRFunction` objects or replaces existing ones by name.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//!
//! let mut module = IIRModule::new("hello.bas", "basic");
//! assert_eq!(module.entry_point, Some("main".to_string()));
//! // Clear entry_point so the empty-module case validates cleanly.
//! module.entry_point = None;
//! assert!(module.validate().is_empty());
//! ```

use crate::instr::Operand;
use crate::function::IIRFunction;
use crate::module_exports::{IIRExport, IIRImport};

// ---------------------------------------------------------------------------
// The string ceiling (LANG83 PR5a)
// ---------------------------------------------------------------------------

/// The most errors of one kind a string-literal check reports in full; the
/// rest are counted in one closing line (LANG83 PR5b security re-review,
/// L2-b).
///
/// Every column runs these checks on modules it did not write. One message
/// per offending operand, each repeating the function's name, made the error
/// text O(name length × operands): a 120 KB serialised module produced
/// 402 MB of it. With the cap, and names cut to [`MAX_ERROR_NAME_CHARS`], the
/// text of [`IIRModule::str_literal_errors`] is bounded whatever the module.
pub const MAX_LITERAL_ERRORS: usize = 16;

/// The most characters of a function, op or module name an error shows.
pub const MAX_ERROR_NAME_CHARS: usize = 64;

/// A name for an error message: quoted and escaped like `{:?}`, and cut to
/// [`MAX_ERROR_NAME_CHARS`] characters with its full length in bytes noted.
fn error_name(name: &str) -> String {
    if name.chars().count() <= MAX_ERROR_NAME_CHARS {
        return format!("{name:?}");
    }
    let head: String = name.chars().take(MAX_ERROR_NAME_CHARS).collect();
    format!("{head:?}… ({} bytes)", name.len())
}

/// Collects at most [`MAX_LITERAL_ERRORS`] messages and counts the rest. A
/// message is built only when it is kept, so a module with a million bad
/// operands costs a count, not a million strings.
#[derive(Default)]
struct BoundedErrors {
    errors: Vec<String>,
    omitted: usize,
}

impl BoundedErrors {
    fn push(&mut self, message: impl FnOnce() -> String) {
        if self.errors.len() < MAX_LITERAL_ERRORS {
            self.errors.push(message());
        } else {
            self.omitted += 1;
        }
    }

    fn finish(mut self, kind: &str) -> Vec<String> {
        if self.omitted > 0 {
            self.errors.push(format!("{kind}: and {} more like these", self.omitted));
        }
        self.errors
    }
}
//
// An IIR `str` is an immutable sequence of bytes, and its length is bounded.
// Every backend stores strings in a host structure with a hard size limit,
// and the tightest of them sets the bound everyone shares:
//
// | Host                    | Its own limit                          |
// |-------------------------|----------------------------------------|
// | .NET `System.String`    | `0x3FFFFFDF` = 1 073 741 791 UTF-16 units |
// | JVM array / `String`    | just under 2³¹                         |
// | WASM `[i32 len][bytes]` | `i32` header, 4 GiB of linear memory   |
// | native / LLVM block     | `i64` header                           |
//
// `STR_MAX_BYTES` = 2³⁰ − 64 = 1 073 741 760 is the largest round value
// below the .NET limit. It is also far enough from every `i64` that adding
// two lengths can never overflow.
//
// A module may *lower* the ceiling with `IIRModule::str_max_bytes`. A
// frontend sets it from its resource budget, and the conformance suite sets
// it to 16 so it can test the trap without allocating a gigabyte on nine
// backends. Only `str_concat` can grow a string past it, and it raises
// `Trap.Overflow` when it would. A `str_const` longer than the ceiling is
// refused by `IIRModule::str_ceiling_errors`, which `IIRModule::validate` and
// every column's own validator call (#18800).
//
//     ceiling = str_max_bytes.unwrap_or(STR_MAX_BYTES)
//     str_concat a, b  traps Trap.Overflow  iff  len a > ceiling − len b
//
// The comparison is written `len a > ceiling − len b` rather than
// `len a + len b > ceiling` because the subtraction cannot overflow
// (`len b ≤ ceiling`), while the sum, on a host with 32-bit lengths, could.

/// The largest string any IIR module may hold, in bytes: 2³⁰ − 64.
///
/// See the table above for why this value. A module can lower it with
/// [`IIRModule::str_max_bytes`]; it can never raise it.
///
/// ```
/// use interpreter_ir::STR_MAX_BYTES;
/// assert_eq!(STR_MAX_BYTES, (1 << 30) - 64);
/// assert!(STR_MAX_BYTES < 0x3FFF_FFDF); // .NET's System.String limit
/// ```
pub const STR_MAX_BYTES: u32 = (1 << 30) - 64;

/// Top-level container for an InterpreterIR program.
#[derive(Debug, Clone)]
pub struct IIRModule {
    /// A human-readable name, typically the source file path.
    pub name: String,

    /// All functions in the program, in definition order.
    pub functions: Vec<IIRFunction>,

    /// Name of the function to call when the module is executed.
    ///
    /// `None` means no automatic entry point (useful for libraries).
    pub entry_point: Option<String>,

    /// Source language identifier.
    ///
    /// Used by tooling for display only; not interpreted by `vm-core`.
    pub language: String,

    /// Functions this module makes visible to other modules (LANG33).
    ///
    /// Empty list = export nothing (a pure program, not a library).
    /// Backends use this list to populate their export sections.
    /// Backward compatible: existing modules with no exports work identically.
    pub exports: Vec<IIRExport>,

    /// Functions this module requires from other modules (LANG33).
    ///
    /// Resolved by `iir-linker::link` during static linking or by the backend
    /// at module-load time for dynamic linking.
    /// Backward compatible: existing modules with no imports work identically.
    pub imports: Vec<IIRImport>,

    /// The exact source-language release this module was compiled against
    /// (LANG84 C4.1), such as `Some("5.38.0")` for a Perl module.
    ///
    /// A release-versioned frontend resolves every compile to **one** row of
    /// its language's `code/grammars/<lang>/releases.csv` and records that
    /// row's ID here, so a module says which grammar produced it:
    ///
    /// ```text
    ///   perl source ──► perl5.38.0.{tokens,grammar} ──► IIRModule { release: Some("5.38.0"), .. }
    ///   twig source ──► twig.{tokens,grammar}       ──► IIRModule { release: None, .. }
    /// ```
    ///
    /// `None` means the producing frontend has no release inventory (or the
    /// module was built by hand). Backends never read this field: two modules
    /// that differ only in `release` lower to identical code, which is what
    /// lets LANG84's CI group releases into IIR identity classes. It is not
    /// part of the binary wire format (`serialise`), so a deserialised module
    /// has `None`, like the transient profiling fields. Read `None` as
    /// "unknown", never as a language's default release: a Perl module that
    /// went through `serialise` also has `None`.
    pub release: Option<String>,

    /// The module's string ceiling in bytes (LANG83 PR5a), or `None` for the
    /// registry-wide [`STR_MAX_BYTES`].
    ///
    /// A frontend lowers it from its resource budget (LANG84 C5), and a test
    /// lowers it so that `str_concat`'s `Trap.Overflow` is cheap to reach. A
    /// value above `STR_MAX_BYTES` is malformed IR, reported by
    /// [`IIRModule::validate`]. Every backend reads the effective value through
    /// [`IIRModule::str_ceiling`], never this field directly.
    ///
    /// Unlike `release`, the field is part of the binary wire format: since
    /// #18800 `serialise` writes it (format 1.2, only when it is `Some`), and
    /// `iir-linker` gives a linked program the smallest ceiling of its
    /// modules. Before, both silently reset it to `None`, the registry
    /// ceiling, which is looser than any `Some`.
    pub str_max_bytes: Option<u32>,
}

impl IIRModule {
    /// Create a new, empty module.
    pub fn new(name: impl Into<String>, language: impl Into<String>) -> Self {
        IIRModule {
            name:        name.into(),
            functions:   Vec::new(),
            entry_point: Some("main".to_string()),
            language:    language.into(),
            exports:     Vec::new(),
            imports:     Vec::new(),
            release:     None,
            str_max_bytes: None,
        }
    }

    /// The effective string ceiling in bytes: `str_max_bytes`, or
    /// [`STR_MAX_BYTES`] when the module does not lower it (LANG83 PR5a).
    ///
    /// A malformed value above `STR_MAX_BYTES` is clamped to it here, so a
    /// backend that lowers an unvalidated module still never exceeds the
    /// registry ceiling.
    ///
    /// ```
    /// use interpreter_ir::{IIRModule, STR_MAX_BYTES};
    /// let mut module = IIRModule::new("m", "iir");
    /// assert_eq!(module.str_ceiling(), STR_MAX_BYTES);
    /// module.str_max_bytes = Some(16);
    /// assert_eq!(module.str_ceiling(), 16);
    /// module.str_max_bytes = Some(u32::MAX);
    /// assert_eq!(module.str_ceiling(), STR_MAX_BYTES);
    /// ```
    pub fn str_ceiling(&self) -> u32 {
        self.str_max_bytes.map_or(STR_MAX_BYTES, |n| n.min(STR_MAX_BYTES))
    }

    /// LANG83 PR5b: every `Operand::Bytes` that is not `str_const`'s literal
    /// (`BytesOperand`), and nothing else.
    ///
    /// A raw-byte literal is a string value. In a name position it would be
    /// read as a name on some tiers and refused on others: the JIT used to lift
    /// `call_builtin [Bytes(b"record"), …]` to a call of the builtin `record`,
    /// which vm-core refused (PR5b security review, L2). So the module is
    /// malformed, and [`IIRModule::str_literal_errors`] — the check every
    /// column runs — reports it.
    pub fn bytes_operand_errors(&self) -> Vec<String> {
        let mut errors = BoundedErrors::default();
        for fn_ in &self.functions {
            for instr in &fn_.instructions {
                for (idx, src) in instr.srcs.iter().enumerate() {
                    if matches!(src, Operand::Bytes(_)) && !(instr.op == "str_const" && idx == 0) {
                        errors.push(|| {
                            format!(
                                "BytesOperand: function {}: op {} operand {idx} is a raw-byte \
                                 literal, which only str_const's literal may be",
                                error_name(&fn_.name),
                                error_name(&instr.op)
                            )
                        });
                    }
                }
            }
        }
        errors.finish("BytesOperand")
    }

    /// The string-literal checks every column runs before it compiles or runs
    /// a module: [`IIRModule::str_ceiling_errors`] (#18800) and
    /// [`IIRModule::bytes_operand_errors`] (LANG83 PR5b). [`IIRModule::validate`]
    /// includes both.
    pub fn str_literal_errors(&self) -> Vec<String> {
        let mut errors = self.str_ceiling_errors();
        errors.extend(self.bytes_operand_errors());
        errors
    }

    /// The LANG83 PR5a string-ceiling checks, and nothing else (empty = the
    /// module is within its ceiling).
    ///
    /// | Error | When |
    /// |-------|------|
    /// | `StrMaxBytes` | `str_max_bytes` is above [`STR_MAX_BYTES`]: malformed IR |
    /// | `StrTooLong`  | a `str_const` literal is longer than [`IIRModule::str_ceiling`] |
    ///
    /// Why a literal past the ceiling is refused rather than trapped: no
    /// backend could hold it, and the contract says `str_concat` is the only
    /// op that may *discover* an over-long string at run time. A literal is
    /// known before the program runs, so the module is malformed.
    ///
    /// [`IIRModule::validate`] includes these errors. Every column also calls
    /// this method directly, where it already refuses a module (#18800): the
    /// WASM, JVM, CLR, BEAM and LLVM validators, `twig-aot`'s native
    /// preparation, and `vm-core`'s load-time check. Before that, nothing
    /// called `validate` on the way to a backend, so a 17-byte literal in a
    /// module whose ceiling is 16 compiled and ran everywhere. The columns call
    /// only these checks, not the whole of `validate`: its other checks
    /// (duplicate names, the entry point) are not every column's contract.
    ///
    /// ```
    /// use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand, STR_MAX_BYTES};
    /// let mut module = IIRModule::new("m", "iir");
    /// module.functions.push(IIRFunction::new("main", vec![], "void", vec![
    ///     IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("12345".into())], "str"),
    ///     IIRInstr::new("ret_void", None, vec![], "void"),
    /// ]));
    /// assert!(module.str_ceiling_errors().is_empty());
    /// module.str_max_bytes = Some(4);
    /// assert!(module.str_ceiling_errors()[0].starts_with("StrTooLong"));
    /// module.str_max_bytes = Some(STR_MAX_BYTES + 1);
    /// assert!(module.str_ceiling_errors()[0].starts_with("StrMaxBytes"));
    /// ```
    pub fn str_ceiling_errors(&self) -> Vec<String> {
        // A ceiling above the registry's is malformed (at most one error).
        let mut max_error = Vec::new();
        if let Some(max) = self.str_max_bytes {
            if max > STR_MAX_BYTES {
                max_error.push(format!(
                    "StrMaxBytes: module {} sets str_max_bytes = {max}, above the registry \
                     ceiling STR_MAX_BYTES = {STR_MAX_BYTES}",
                    error_name(&self.name)
                ));
            }
        }
        let mut errors = BoundedErrors::default();
        // So is a literal longer than the (clamped) ceiling.
        let ceiling = self.str_ceiling() as usize;
        for fn_ in &self.functions {
            for instr in fn_.instructions.iter().filter(|i| i.op == "str_const") {
                // A `Str` or a `Bytes` literal (LANG83 PR5b): both are bytes.
                if let Some(literal) = instr.srcs.first().and_then(Operand::as_lit_bytes) {
                    if literal.len() > ceiling {
                        errors.push(|| {
                            format!(
                                "StrTooLong: function {}: a {}-byte str_const exceeds the module's \
                                 string ceiling of {ceiling} bytes",
                                error_name(&fn_.name),
                                literal.len()
                            )
                        });
                    }
                }
            }
        }
        max_error.extend(errors.finish("StrTooLong"));
        max_error
    }

    // -----------------------------------------------------------------------
    // Lookup
    // -----------------------------------------------------------------------

    /// Return a shared reference to the `IIRFunction` with the given name, or
    /// `None` if not found.
    pub fn get_function(&self, fn_name: &str) -> Option<&IIRFunction> {
        self.functions.iter().find(|f| f.name == fn_name)
    }

    /// Return a mutable reference to the `IIRFunction` with the given name,
    /// or `None` if not found.
    ///
    /// The VM's dispatch loop uses this to update profiling counters in place.
    pub fn get_function_mut(&mut self, fn_name: &str) -> Option<&mut IIRFunction> {
        self.functions.iter_mut().find(|f| f.name == fn_name)
    }

    /// Return all function names in definition order.
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.iter().map(|f| f.name.as_str()).collect()
    }

    // -----------------------------------------------------------------------
    // Mutation (used by REPL incremental compilation)
    // -----------------------------------------------------------------------

    /// Append `fn_` or replace an existing function with the same name.
    ///
    /// Called by the REPL integration (LANG08) when the user redefines a
    /// function in a later input.
    pub fn add_or_replace(&mut self, fn_: IIRFunction) {
        if let Some(pos) = self.functions.iter().position(|f| f.name == fn_.name) {
            self.functions[pos] = fn_;
        } else {
            self.functions.push(fn_);
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Return a list of validation error strings (empty = valid).
    ///
    /// Checks:
    /// - No duplicate function names
    /// - Entry point function exists (if `entry_point` is set)
    /// - No instruction branches to an undefined label within its function
    /// - Every export refers to a function that exists in this module (LANG33)
    /// - No two exports have the same `public_name()` (LANG33)
    /// - Every `catch`'s `try_end_label`/`handler_label` resolve to a `label`
    ///   in the same function, and `handler_label` is immediately followed by
    ///   a `landingpad` (AOT00 T2 — see `crate::opcodes::is_exception_op`)
    ///
    /// **Imports are not validated here** — import resolution requires peer
    /// modules and is the linker's job (`iir-linker::verify_imports`).
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for fn_ in &self.functions {
            if !seen.insert(&fn_.name) {
                errors.push(format!("duplicate function name: {:?}", fn_.name));
            }
        }

        // LANG83 PR5b: a raw-byte literal anywhere but `str_const`'s literal.
        errors.extend(self.bytes_operand_errors());

        if let Some(ep) = &self.entry_point {
            if self.get_function(ep).is_none() {
                errors.push(format!(
                    "entry_point {:?} not found in module functions",
                    ep
                ));
            }
        }

        for fn_ in &self.functions {
            // `label` position lookup: name -> instruction index. Used both for
            // the branch-target check below and the AOT00 T2 `catch`/`landingpad`
            // structural checks (which need to know not just that a label is
            // *defined*, but what instruction immediately follows it).
            let label_positions: std::collections::HashMap<&str, usize> = fn_
                .instructions
                .iter()
                .enumerate()
                .filter(|(_, i)| i.op == "label")
                .filter_map(|(idx, i)| Some((i.srcs.first()?.as_var()?, idx)))
                .collect();

            for instr in &fn_.instructions {
                if matches!(instr.op.as_str(), "jmp" | "jmp_if_true" | "jmp_if_false") {
                    if let Some(label) = instr.srcs.last().and_then(|s| s.as_var()) {
                        if !label_positions.contains_key(label) {
                            errors.push(format!(
                                "function {:?}: branch to undefined label {:?}",
                                fn_.name, label
                            ));
                        }
                    }
                }

                // ── AOT00 T2: `catch(kind, try_end_label, handler_label)` ──────
                // Both label operands must resolve to a `label` in this function
                // (same requirement as a `jmp` target), and `handler_label` must
                // be immediately followed by a `landingpad` — the handler's entry
                // point, which binds the in-flight exception. A `catch` whose
                // handler isn't a real landing pad can never be lowered, so this
                // is caught here rather than surfacing as a confusing backend
                // error later.
                if instr.op == "catch" {
                    let try_end = instr.srcs.get(1).and_then(|s| s.as_var());
                    let handler = instr.srcs.get(2).and_then(|s| s.as_var());

                    match try_end {
                        Some(label) if !label_positions.contains_key(label) => {
                            errors.push(format!(
                                "function {:?}: catch refers to undefined try_end_label {:?}",
                                fn_.name, label
                            ));
                        }
                        None => errors.push(format!(
                            "function {:?}: catch is missing its try_end_label operand",
                            fn_.name
                        )),
                        _ => {}
                    }

                    match handler {
                        Some(label) => match label_positions.get(label) {
                            None => errors.push(format!(
                                "function {:?}: catch refers to undefined handler_label {:?}",
                                fn_.name, label
                            )),
                            Some(&label_idx) => {
                                let landingpad_idx = label_idx + 1;
                                let is_landingpad = fn_
                                    .instructions
                                    .get(landingpad_idx)
                                    .is_some_and(|i| i.op == "landingpad");
                                if !is_landingpad {
                                    errors.push(format!(
                                        "function {:?}: handler_label {:?} is not immediately \
                                         followed by a landingpad instruction",
                                        fn_.name, label
                                    ));
                                }
                            }
                        },
                        None => errors.push(format!(
                            "function {:?}: catch is missing its handler_label operand",
                            fn_.name
                        )),
                    }
                }
            }
        }

        // ── LANG83 PR5a: the string ceiling ─────────────────────────────────
        // Shared with every column's own validator (#18800), so it lives in
        // its own method; see `str_ceiling_errors`.
        errors.extend(self.str_ceiling_errors());

        // ── LANG33: export checks ────────────────────────────────────────────
        let fn_names: std::collections::HashSet<&str> =
            self.functions.iter().map(|f| f.name.as_str()).collect();

        let mut public_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for export in &self.exports {
            if !fn_names.contains(export.function_name.as_str()) {
                errors.push(format!(
                    "ExportNotFound: exported function {:?} not found in module {:?}",
                    export.function_name, self.name
                ));
            }
            let pn = export.public_name().to_string();
            if !public_names.insert(pn.clone()) {
                errors.push(format!(
                    "DuplicateExport: public name {:?} is exported more than once in module {:?}",
                    pn, self.name
                ));
            }
        }

        errors
    }
}

impl std::fmt::Display for IIRModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IIRModule({:?}, language={:?}, functions={:?}, entry={:?})",
            self.name,
            self.language,
            self.function_names(),
            self.entry_point,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::IIRFunction;
    use crate::instr::{IIRInstr, Operand};

    fn make_main() -> IIRFunction {
        IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![IIRInstr::new("ret_void", None, vec![], "void")],
        )
    }

    #[test]
    fn a_new_module_names_no_release() {
        // LANG84 C4.1: only a release-versioned frontend sets `release`.
        let mut module = IIRModule::new("p.pl", "perl");
        assert_eq!(module.release, None);
        module.release = Some("5.38.0".into());
        assert_eq!(module.clone().release.as_deref(), Some("5.38.0"));
    }

    #[test]
    fn a_new_module_uses_the_registry_string_ceiling() {
        let module = IIRModule::new("m", "iir");
        assert_eq!(module.str_max_bytes, None);
        assert_eq!(module.str_ceiling(), STR_MAX_BYTES);
        assert_eq!(STR_MAX_BYTES, 1_073_741_760);
    }

    fn module_with_literal(literal: &str, max: Option<u32>) -> IIRModule {
        let mut module = IIRModule::new("m", "iir");
        module.str_max_bytes = max;
        module.functions.push(IIRFunction::new(
            "main",
            vec![],
            "void",
            vec![
                IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str(literal.into())], "str"),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        ));
        module
    }

    #[test]
    fn validate_accepts_a_literal_exactly_at_the_ceiling() {
        assert!(module_with_literal("0123456789abcdef", Some(16)).validate().is_empty());
        // The ceiling counts bytes, not characters: "é" is two bytes.
        assert!(module_with_literal("é", Some(2)).validate().is_empty());
    }

    #[test]
    fn validate_refuses_a_literal_over_the_ceiling() {
        let errors = module_with_literal("0123456789abcdefg", Some(16)).validate();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("StrTooLong"), "{errors:?}");
        let errors = module_with_literal("é", Some(1)).validate();
        assert!(errors[0].contains("2-byte"), "{errors:?}");
    }

    /// LANG83 PR5b: a raw-byte literal is measured like a `Str` one, so the
    /// ceiling cannot be dodged by spelling a literal as bytes.
    #[test]
    fn a_raw_byte_literal_counts_against_the_ceiling() {
        let mut module = module_with_literal("", Some(2));
        module.functions[0].instructions[0].srcs[0] = Operand::Bytes(vec![0xFF, 0x00]);
        assert!(module.str_ceiling_errors().is_empty());
        module.functions[0].instructions[0].srcs[0] = Operand::Bytes(vec![0xFF, 0x00, 0x80]);
        let errors = module.str_ceiling_errors();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("StrTooLong") && errors[0].contains("3-byte"), "{errors:?}");
    }

    /// LANG83 PR5b: `Operand::Bytes` is legal only as `str_const`'s literal.
    #[test]
    fn validate_refuses_a_raw_byte_operand_outside_str_const() {
        let mut module = module_with_literal("", None);
        module.functions[0].instructions[0].srcs[0] = Operand::Bytes(vec![0xFF]);
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        module.functions[0].instructions.insert(
            1,
            IIRInstr::new("global_load", Some("g".into()), vec![Operand::Bytes(b"x".to_vec())], "i64"),
        );
        // A second operand of `str_const` is not its literal either.
        module.functions[0].instructions.insert(
            1,
            IIRInstr::new(
                "str_const",
                Some("t".into()),
                vec![Operand::Str("a".into()), Operand::Bytes(vec![1])],
                "str",
            ),
        );
        let errors = module.validate();
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors.iter().all(|e| e.starts_with("BytesOperand")), "{errors:?}");
        assert!(errors[0].contains("\"str_const\" operand 1"), "{errors:?}");
        assert!(errors[1].contains("\"global_load\" operand 0"), "{errors:?}");
    }

    /// `str_literal_errors` — what every column runs — is the ceiling checks
    /// plus `BytesOperand` (PR5b security review L2), and `validate` reports
    /// each of them.
    #[test]
    fn str_literal_errors_is_the_ceiling_and_the_bytes_operand_checks() {
        let mut module = module_with_literal("0123456789abcdefg", Some(16));
        module.functions[0].instructions.insert(
            1,
            IIRInstr::new("call_builtin", None, vec![Operand::Bytes(b"record".to_vec())], "void"),
        );
        let literal = module.str_literal_errors();
        assert_eq!(literal.len(), 2, "{literal:?}");
        assert!(literal[0].starts_with("StrTooLong") && literal[1].starts_with("BytesOperand"), "{literal:?}");
        assert_eq!(module.bytes_operand_errors(), literal[1..].to_vec());
        assert!(literal.iter().all(|e| module.validate().contains(e)));
        assert!(module_with_literal("ok", None).str_literal_errors().is_empty());
    }

    /// PR5b security re-review (L2-b): the string-literal error text is
    /// bounded whatever the module. A long function name times many bad
    /// operands used to give one message per operand, each repeating the
    /// name: here 2 000 × 2 000 characters, about 8 MB of text (the review's
    /// 20 000 × 20 000 reproducer made 402 MB from a 120 KB stream). The
    /// module goes through the wire format first, as an untrusted one would.
    #[test]
    fn string_literal_error_text_is_bounded() {
        let name = "f".repeat(2_000);
        let mut bytes = IIRModule::new("m", "iir");
        bytes.functions.push(IIRFunction::new(&name, vec![], "void", vec![IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Bytes(vec![]); 2_000],
            "void",
        )]));
        let mut long = IIRModule::new(&"m".repeat(2_000), "iir");
        long.str_max_bytes = Some(0);
        long.functions.push(IIRFunction::new(
            &name,
            vec![],
            "void",
            (0..2_000).map(|_| IIRInstr::new("str_const", Some("s".into()), vec![Operand::Str("x".into())], "str")).collect(),
        ));
        for module in [bytes, long] {
            let module = crate::serialise::deserialise(&crate::serialise::serialise(&module)).unwrap();
            let errors = module.str_literal_errors();
            let text: usize = errors.iter().map(String::len).sum();
            assert!(errors.len() <= MAX_LITERAL_ERRORS + 2, "{} errors", errors.len());
            assert!(text < 8 * 1024, "{text} bytes of error text");
            let last = errors.last().unwrap();
            assert!(last.ends_with(&format!("and {} more like these", 2_000 - MAX_LITERAL_ERRORS)), "{last}");
            assert!(errors[0].contains("… (2000 bytes)"), "{}", errors[0]);
        }
    }

    #[test]
    fn validate_refuses_a_ceiling_above_the_registry_one() {
        let errors = module_with_literal("", Some(STR_MAX_BYTES + 1)).validate();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("StrMaxBytes"), "{errors:?}");
        assert!(module_with_literal("", Some(STR_MAX_BYTES)).validate().is_empty());
        // `str_ceiling` clamps, so a backend never goes above the registry value.
        assert_eq!(module_with_literal("", Some(u32::MAX)).str_ceiling(), STR_MAX_BYTES);
    }

    /// `str_ceiling_errors` is exactly the ceiling part of `validate`: the
    /// backends call it on its own (#18800), so it must not report the
    /// other checks (here: the entry point `"lib"` that does not exist), and
    /// `validate` must still report everything it reports.
    #[test]
    fn str_ceiling_errors_is_the_ceiling_part_of_validate() {
        let mut module = module_with_literal("0123456789abcdefg", Some(16));
        module.entry_point = Some("lib".into());
        let ceiling = module.str_ceiling_errors();
        assert_eq!(ceiling.len(), 1, "{ceiling:?}");
        assert!(ceiling[0].starts_with("StrTooLong"), "{ceiling:?}");
        let all = module.validate();
        assert!(all.iter().any(|e| e.contains("entry_point")), "{all:?}");
        assert!(ceiling.iter().all(|e| all.contains(e)), "{all:?}");
        // Both malformations at once: the ceiling above the registry one is
        // clamped, so a 17-byte literal is then within it.
        module.str_max_bytes = Some(STR_MAX_BYTES + 1);
        let ceiling = module.str_ceiling_errors();
        assert_eq!(ceiling.len(), 1, "{ceiling:?}");
        assert!(ceiling[0].starts_with("StrMaxBytes"), "{ceiling:?}");
        // A literal in any function is checked, and a non-literal operand
        // (a `Var`) is not a literal.
        module.str_max_bytes = Some(1);
        module.functions.push(IIRFunction::new("g", vec![], "void", vec![
            IIRInstr::new("str_const", Some("t".into()), vec![Operand::Str("xy".into())], "str"),
            IIRInstr::new("str_const", Some("u".into()), vec![Operand::Var("not_a_literal".into())], "str"),
        ]));
        let ceiling = module.str_ceiling_errors();
        assert_eq!(ceiling.len(), 2, "{ceiling:?}");
        assert!(ceiling[1].contains("\"g\""), "{ceiling:?}");
        assert!(module_with_literal("", None).str_ceiling_errors().is_empty());
    }

    #[test]
    fn new_module_has_main_entry() {
        let module = IIRModule::new("test.bas", "basic");
        assert_eq!(module.entry_point, Some("main".to_string()));
        assert!(module.functions.is_empty());
    }

    #[test]
    fn validate_empty_module_no_main() {
        let module = IIRModule::new("test.bas", "basic");
        let errors = module.validate();
        // entry_point = "main" but no function named "main" → one error
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("main"));
    }

    #[test]
    fn validate_module_with_main_is_clean() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.add_or_replace(make_main());
        assert!(module.validate().is_empty());
    }

    #[test]
    fn add_or_replace_replaces_existing() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.add_or_replace(make_main());
        let mut new_main = make_main();
        new_main.call_count = 42;
        module.add_or_replace(new_main);
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].call_count, 42);
    }

    #[test]
    fn validate_catches_duplicate_names() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        module.functions.push(make_main());
        module.functions.push(make_main()); // duplicate
        let errors = module.validate();
        assert!(errors.iter().any(|e| e.contains("duplicate")));
    }

    #[test]
    fn validate_catches_undefined_label() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "loop_fn",
            vec![],
            "void",
            vec![IIRInstr::new(
                "jmp",
                None,
                vec![Operand::Var("loop_start".into())],
                "void",
            )],
        );
        module.functions.push(fn_);
        let errors = module.validate();
        assert!(errors.iter().any(|e| e.contains("loop_start")));
    }

    // ── AOT00 T2: catch/landingpad structural validation ──────────────────────

    /// Build `try { <body...> } catch <kind> -> handler { landingpad(dest) <handler_body...> }`
    /// as a flat IIR instruction stream, using labels `try_end`/`handler` — the
    /// shape every `validate_catch_*` test below starts from.
    fn make_fn_with_try_catch(landingpad_follows_handler: bool) -> IIRFunction {
        let mut instructions = vec![
            IIRInstr::new(
                "catch",
                None,
                vec![
                    Operand::Str("Trap".into()),
                    Operand::Var("try_end".into()),
                    Operand::Var("handler".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("try_end".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("handler".into())], "void"),
        ];
        if landingpad_follows_handler {
            instructions.push(IIRInstr::new(
                "landingpad",
                Some("exc".into()),
                vec![],
                "ref<exception>",
            ));
        }
        instructions.push(IIRInstr::new("ret_void", None, vec![], "void"));

        IIRFunction::new("f", vec![], "void", instructions)
    }

    #[test]
    fn validate_accepts_well_formed_try_catch() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        module.functions.push(make_fn_with_try_catch(true));
        assert!(module.validate().is_empty());
    }

    #[test]
    fn validate_catches_missing_landingpad_after_handler_label() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        module.functions.push(make_fn_with_try_catch(false));
        let errors = module.validate();
        assert!(
            errors.iter().any(|e| e.contains("landingpad")),
            "expected a landingpad error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_catches_undefined_handler_label() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "f",
            vec![],
            "void",
            vec![
                IIRInstr::new(
                    "catch",
                    None,
                    vec![
                        Operand::Str("Trap".into()),
                        Operand::Var("try_end".into()),
                        Operand::Var("nowhere".into()),
                    ],
                    "void",
                ),
                IIRInstr::new("label", None, vec![Operand::Var("try_end".into())], "void"),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        );
        module.functions.push(fn_);
        let errors = module.validate();
        assert!(errors.iter().any(|e| e.contains("nowhere")));
    }

    #[test]
    fn validate_catches_undefined_try_end_label() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.entry_point = None;
        let fn_ = IIRFunction::new(
            "f",
            vec![],
            "void",
            vec![
                IIRInstr::new(
                    "catch",
                    None,
                    vec![
                        Operand::Str("Trap".into()),
                        Operand::Var("nowhere".into()),
                        Operand::Var("handler".into()),
                    ],
                    "void",
                ),
                IIRInstr::new("label", None, vec![Operand::Var("handler".into())], "void"),
                IIRInstr::new("landingpad", Some("exc".into()), vec![], "ref<exception>"),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ],
        );
        module.functions.push(fn_);
        let errors = module.validate();
        assert!(errors.iter().any(|e| e.contains("nowhere")));
    }

    #[test]
    fn get_function_mut_allows_updating_call_count() {
        let mut module = IIRModule::new("test.bas", "basic");
        module.add_or_replace(make_main());
        module.get_function_mut("main").unwrap().call_count = 5;
        assert_eq!(module.get_function("main").unwrap().call_count, 5);
    }
}

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

use crate::function::IIRFunction;
use crate::module_exports::{IIRExport, IIRImport};

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
        }
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

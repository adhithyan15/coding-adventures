//! # FLOW-MATIC → IIR compiler
//!
//! Lowers a parsed FLOW-MATIC program (the generic `GrammarASTNode` CST from
//! [`coding_adventures_flow_matic_parser`]) into an [`interpreter_ir::IIRModule`]
//! so FLOW-MATIC runs on every execution backend the LANG VM AOT chain targets
//! (NativeAOT / LLVM / WASM / JVM / CLR / VM / JIT). See
//! [PL09](../../../specs/PL09-codegen.md).
//!
//! ## This slice
//!
//! FLOW-MATIC is a file/record data-flow language; its **control flow**,
//! **scalar-field moves**, and **record output** lower over the existing
//! backend-portable primitives (no filesystem — see PL09 D4):
//!
//! | FLOW-MATIC | IIR |
//! | --- | --- |
//! | operation `(n)` | `label op_n` |
//! | `COMPARE a WITH b` | `cmp_gt`/`cmp_eq`/`cmp_lt` into three flag vars |
//! | `IF GREATER/EQUAL/LESS GO TO OPERATION n` | `jmp_if_true flag, op_n` |
//! | `OTHERWISE GO TO OPERATION n` | `jmp op_n` |
//! | `JUMP TO OPERATION n` | `jmp op_n` |
//! | `MOVE field TO field` | `mov` between the fields' registers |
//! | `WRITE-ITEM handle` | print the file's fields via `__fm_print_int`+`putchar` |
//! | `STOP` | `ret 0` |
//! | `INPUT`/`OUTPUT`/`HSP` (file declarations) | no-op |
//!
//! Each file-qualified field (`PRODUCT-NO (A)`) is a distinct `i64` register
//! initialised to 0. `READ-ITEM` (which needs the EOF-aware input capability),
//! `TRANSFER`, `TEST`/`REWIND`/`CLOSE-OUT`, and the `END OF DATA` loop condition
//! are a later rung; a program that reaches one gets a clean
//! [`CompileError::Unsupported`], never wrong output.

use coding_adventures_flow_matic_parser::try_parse_flow_matic;
use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::BTreeSet;

/// A FLOW-MATIC → IIR compilation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The source did not parse.
    Parse(String),
    /// A construct not yet lowered in this slice (e.g. record/file I/O).
    Unsupported(String),
    /// The CST was shaped unexpectedly (a malformed node).
    Malformed(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "FLOW-MATIC parse error: {m}"),
            CompileError::Unsupported(m) => write!(f, "FLOW-MATIC not yet lowered to IIR: {m}"),
            CompileError::Malformed(m) => write!(f, "malformed FLOW-MATIC CST: {m}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile FLOW-MATIC `source` into an [`IIRModule`] with a single `main`
/// returning `i64` (its return value is the process exit code).
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, CompileError> {
    let ast = try_parse_flow_matic(source).map_err(CompileError::Parse)?;
    let mut comp = Compiler::default();
    comp.emit_program(&ast)?;

    let mut main = IIRFunction::new("main", vec![], "i64", comp.instrs);
    // Every instruction we emit is statically typed (no `"any"` hints); the
    // `"void"` control-flow hints otherwise make the auto-inference return
    // PartiallyTyped, so we assert FullyTyped like the BASIC/Brainfuck frontends.
    main.type_status = FunctionTypeStatus::FullyTyped;

    let mut module = IIRModule::new(module_name, "flow-matic");
    module.functions.push(main);
    // WRITE-ITEM prints numeric fields through the synthesized recursive
    // digit-print helpers; append them (before user code is irrelevant — `call`
    // resolves by name) only when a WRITE-ITEM actually rendered a value.
    if comp.needs_print {
        for func in print_helper_functions() {
            module.functions.push(func);
        }
    }
    module.entry_point = Some("main".to_string());
    Ok(module)
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Every file-qualified field register, collected up front (also used by
    /// `WRITE-ITEM` to find a file's fields).
    fields: BTreeSet<String>,
    /// Set when a `WRITE-ITEM` emits a call to the digit-print helpers, so they
    /// are appended to the module.
    needs_print: bool,
    /// Unique-label counter for `WRITE-ITEM` record separators.
    write_counter: usize,
}

impl Compiler {
    fn emit(&mut self, op: &str, dest: Option<&str>, srcs: Vec<Operand>, type_hint: &str) {
        self.instrs.push(IIRInstr::new(op, dest.map(str::to_string), srcs, type_hint));
    }

    fn emit_program(&mut self, program: &GrammarASTNode) -> Result<(), CompileError> {
        // Pre-pass: every file-qualified field becomes an i64 register, zeroed
        // at entry (FLOW-MATIC fields start empty; a real value would arrive via
        // READ-ITEM, a later rung).
        collect_fields(program, &mut self.fields);
        for field in self.fields.clone() {
            self.emit("const", Some(&field), vec![Operand::Int(0)], "i64");
        }
        // Zero the three shared comparison flags at entry too, so an
        // `IF … GO TO` that is reachable without a dominating `COMPARE` (e.g. a
        // lone IF, or a jump into a block whose flag was set on another path)
        // reads a defined `0` (false → no branch) rather than an undefined
        // register — which `module.validate()` and the backend validators do
        // NOT catch, and which would miscompile downstream.
        for flag in [CMP_GT, CMP_EQ, CMP_LT] {
            self.emit("const", Some(flag), vec![Operand::Int(0)], "i64");
        }

        // Each operation becomes a labelled block; clauses run in order and
        // control falls through to the next operation unless a clause jumps.
        for stmt in child_nodes(program, "statement") {
            let op_num = first_token(stmt, "NUMBER")
                .ok_or_else(|| CompileError::Malformed("operation without a number".into()))?;
            self.emit("label", None, vec![Operand::Var(op_label(&op_num))], "void");
            for clause in child_nodes(stmt, "clause") {
                self.emit_clause(clause)?;
            }
        }

        // A trailing `ret 0` guarantees `main` returns even if the last
        // operation falls off the end without a STOP.
        self.emit_ret_zero();
        Ok(())
    }

    fn emit_clause(&mut self, clause: &GrammarASTNode) -> Result<(), CompileError> {
        // `clause` wraps exactly one specific *_clause node.
        let inner = clause
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .ok_or_else(|| CompileError::Malformed("empty clause".into()))?;

        match inner.rule_name.as_str() {
            // File declarations bear no runtime effect in this slice.
            "input_clause" | "output_clause" | "hsp_clause" => Ok(()),

            "compare_clause" => {
                let fields = child_nodes(inner, "field");
                if fields.len() != 2 {
                    return Err(CompileError::Malformed("COMPARE needs two fields".into()));
                }
                let a = read_field(fields[0])?;
                let b = read_field(fields[1])?;
                // Set the three-way flags the following IF clauses read.
                self.emit("cmp_gt", Some(CMP_GT), vec![Operand::Var(a.clone()), Operand::Var(b.clone())], "i64");
                self.emit("cmp_eq", Some(CMP_EQ), vec![Operand::Var(a.clone()), Operand::Var(b.clone())], "i64");
                self.emit("cmp_lt", Some(CMP_LT), vec![Operand::Var(a), Operand::Var(b)], "i64");
                Ok(())
            }

            "if_clause" => {
                let flag = self.condition_flag(inner)?;
                let target = read_target(inner)?;
                self.emit(
                    "jmp_if_true",
                    None,
                    vec![Operand::Var(flag), Operand::Var(target)],
                    "void",
                );
                Ok(())
            }

            "otherwise_clause" => {
                let target = read_target(inner)?;
                self.emit("jmp", None, vec![Operand::Var(target)], "void");
                Ok(())
            }

            "jump_clause" => {
                let target = read_target(inner)?;
                self.emit("jmp", None, vec![Operand::Var(target)], "void");
                Ok(())
            }

            "move_clause" => {
                let fields = child_nodes(inner, "field");
                if fields.len() != 2 {
                    return Err(CompileError::Malformed("MOVE needs two fields".into()));
                }
                let src = read_field(fields[0])?;
                let dst = read_field(fields[1])?;
                self.emit("mov", Some(&dst), vec![Operand::Var(src)], "i64");
                Ok(())
            }

            "stop_clause" => {
                self.emit_ret_zero();
                Ok(())
            }

            "write_item_clause" => {
                // WRITE-ITEM <handle> writes the file's record to stdout: its
                // fields (those qualified by the handle's letter — `FILE-C` →
                // `C`), space-separated, then a newline.
                let handle = first_token(inner, "NAME").ok_or_else(|| {
                    CompileError::Malformed("WRITE-ITEM without a file handle".into())
                })?;
                let qualifier = qualifier_of_handle(&handle);
                let suffix = format!("__{}", sanitise(&qualifier));
                let record: Vec<String> =
                    self.fields.iter().filter(|f| f.ends_with(&suffix)).cloned().collect();
                self.needs_print = true;
                for (i, field) in record.iter().enumerate() {
                    if i > 0 {
                        self.emit_putchar(b' ' as i64);
                    }
                    // call __fm_print_int(field) — the return value is unused.
                    let ret = format!("_w{}", self.write_counter);
                    self.write_counter += 1;
                    self.emit(
                        "call",
                        Some(&ret),
                        vec![Operand::Var("__fm_print_int".into()), Operand::Var(field.clone())],
                        "i64",
                    );
                }
                self.emit_putchar(b'\n' as i64); // the record terminator
                Ok(())
            }

            // Record READ (needs the EOF-aware input capability, D4) and tape
            // control are a later rung.
            other => Err(CompileError::Unsupported(format!(
                "the {} (record input / tape control is a later rung)",
                verb_name(other)
            ))),
        }
    }

    /// `putchar(byte)` — a `const` then the host `putchar` builtin.
    fn emit_putchar(&mut self, byte: i64) {
        let t = format!("_ch{}", self.write_counter);
        self.write_counter += 1;
        self.emit("const", Some(&t), vec![Operand::Int(byte)], "i64");
        self.emit("call_builtin", None, vec![Operand::Var("putchar".into()), Operand::Var(t)], "void");
    }

    /// The flag register a given `if_clause`'s condition reads.
    fn condition_flag(&self, if_clause: &GrammarASTNode) -> Result<String, CompileError> {
        let cond = child_node(if_clause, "condition")
            .ok_or_else(|| CompileError::Malformed("IF without a condition".into()))?;
        // A condition is a keyword (GREATER/EQUAL/LESS) or the END OF DATA test.
        let words: Vec<String> = child_tokens(cond)
            .into_iter()
            .filter(|(k, _)| k == "KEYWORD")
            .map(|(_, v)| v)
            .collect();
        match words.first().map(String::as_str) {
            Some("GREATER") => Ok(CMP_GT.to_string()),
            Some("EQUAL") => Ok(CMP_EQ.to_string()),
            Some("LESS") => Ok(CMP_LT.to_string()),
            Some("END") => Err(CompileError::Unsupported(
                "IF END OF DATA (the file end-of-data loop is a later rung)".into(),
            )),
            _ => Err(CompileError::Malformed("unrecognised IF condition".into())),
        }
    }

    fn emit_ret_zero(&mut self) {
        self.emit("const", Some(RET_ZERO), vec![Operand::Int(0)], "i64");
        self.emit("ret", None, vec![Operand::Var(RET_ZERO.to_string())], "i64");
    }
}

// Fixed register names. The three comparison flags are reused per COMPARE — an
// IF always immediately follows the COMPARE it reads, so there is no lifetime
// overlap between distinct comparisons.
const CMP_GT: &str = "_cmp_gt";
const CMP_EQ: &str = "_cmp_eq";
const CMP_LT: &str = "_cmp_lt";
const RET_ZERO: &str = "_ret0";

// ---------------------------------------------------------------------------
// CST helpers
// ---------------------------------------------------------------------------

fn child_nodes<'a>(n: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(x) if x.rule_name == rule => Some(x),
            _ => None,
        })
        .collect()
}

fn child_node<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(n, rule).into_iter().next()
}

fn child_tokens(n: &GrammarASTNode) -> Vec<(String, String)> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => {
                Some((t.effective_type_name().to_string(), t.value.clone()))
            }
            _ => None,
        })
        .collect()
}

fn first_token(n: &GrammarASTNode, type_name: &str) -> Option<String> {
    child_tokens(n).into_iter().find(|(k, _)| k == type_name).map(|(_, v)| v)
}

/// The IIR register name for a `field` node (`NAME LPAREN NAME RPAREN`): its
/// data-name qualified by its file, sanitised to a register-safe identifier.
fn read_field(field: &GrammarASTNode) -> Result<String, CompileError> {
    let names: Vec<String> = child_tokens(field)
        .into_iter()
        .filter(|(k, _)| k == "NAME")
        .map(|(_, v)| v)
        .collect();
    match names.as_slice() {
        [data, file] => Ok(format!("fld_{}__{}", sanitise(data), sanitise(file))),
        _ => Err(CompileError::Malformed("field must be NAME ( NAME )".into())),
    }
}

/// The `op_<n>` label for a `target` node (`OPERATION NUMBER`).
fn read_target(clause: &GrammarASTNode) -> Result<String, CompileError> {
    let target = child_node(clause, "target")
        .ok_or_else(|| CompileError::Malformed("clause without a GO TO target".into()))?;
    let n = first_token(target, "NUMBER")
        .ok_or_else(|| CompileError::Malformed("GO TO target without an operation number".into()))?;
    Ok(op_label(&n))
}

/// Recursively collect every `field` node's register name.
fn collect_fields(node: &GrammarASTNode, out: &mut BTreeSet<String>) {
    if node.rule_name == "field" {
        if let Ok(name) = read_field(node) {
            out.insert(name);
        }
    }
    for c in &node.children {
        if let ASTNodeOrToken::Node(n) = c {
            collect_fields(n, out);
        }
    }
}

fn op_label(num: &str) -> String {
    format!("op_{num}")
}

fn sanitise(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn verb_name(rule: &str) -> &str {
    match rule {
        "transfer_clause" => "TRANSFER",
        "read_item_clause" => "READ-ITEM",
        "write_item_clause" => "WRITE-ITEM",
        "test_clause" => "TEST",
        "rewind_clause" => "REWIND",
        "closeout_clause" => "CLOSE-OUT",
        other => other,
    }
}

/// The field qualifier a `WRITE-ITEM`/`READ-ITEM` file handle refers to: `FILE-C`
/// → `C` (the letter fields are qualified by). A handle without the `FILE-`
/// prefix is used as-is.
fn qualifier_of_handle(handle: &str) -> String {
    let up = handle.to_ascii_uppercase();
    up.strip_prefix("FILE-").unwrap_or(&up).to_string()
}

/// The synthesized recursive digit-print helpers a `WRITE-ITEM` calls:
/// `__fm_print_int(n)` dispatches the sign then calls `__fm_print_mag(m)`, which
/// prints an `i64` magnitude most-significant-digit first via `putchar`. Both
/// return `i64` 0 (the convention the print backends expect). Mirrors the
/// Dartmouth BASIC frontend's `__basic_print_int`/`__basic_print_uint`.
fn print_helper_functions() -> Vec<IIRFunction> {
    fn mk(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
        IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
    }
    fn var(name: &str) -> Operand {
        Operand::Var(name.to_string())
    }

    // __fm_print_mag(m): print m (>= 0), high digits first, recursively.
    let mag_body = vec![
        mk("const", Some("ten"), vec![Operand::Int(10)], "i64"),
        mk("div", Some("t"), vec![var("m"), var("ten")], "i64"),
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        mk("cmp_ne", Some("more"), vec![var("t"), var("zero")], "i64"),
        // Single-digit m (t == 0): skip the recursion.
        mk("jmp_if_false", None, vec![var("more"), var("mag_last")], "void"),
        mk("call", Some("_r"), vec![var("__fm_print_mag"), var("t")], "i64"),
        mk("label", None, vec![var("mag_last")], "void"),
        mk("mod", Some("d"), vec![var("m"), var("ten")], "i64"),
        mk("const", Some("c0"), vec![Operand::Int(b'0' as i64)], "i64"),
        mk("add", Some("c"), vec![var("d"), var("c0")], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("c")], "void"),
        mk("const", Some("z"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z")], "i64"),
    ];

    // __fm_print_int(n): print the sign, then the magnitude.
    let int_body = vec![
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        mk("cmp_lt", Some("neg"), vec![var("n"), var("zero")], "i64"),
        mk("jmp_if_false", None, vec![var("neg"), var("int_pos")], "void"),
        mk("const", Some("minus"), vec![Operand::Int(b'-' as i64)], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("minus")], "void"),
        mk("sub", Some("mag"), vec![var("zero"), var("n")], "i64"),
        mk("call", Some("_rn"), vec![var("__fm_print_mag"), var("mag")], "i64"),
        mk("jmp", None, vec![var("int_done")], "void"),
        mk("label", None, vec![var("int_pos")], "void"),
        mk("call", Some("_rp"), vec![var("__fm_print_mag"), var("n")], "i64"),
        mk("label", None, vec![var("int_done")], "void"),
        mk("const", Some("z2"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z2")], "i64"),
    ];

    let mut mag = IIRFunction::new("__fm_print_mag", vec![("m".into(), "i64".into())], "i64", mag_body);
    mag.type_status = FunctionTypeStatus::FullyTyped;
    let mut int = IIRFunction::new("__fm_print_int", vec![("n".into(), "i64".into())], "i64", int_body);
    int.type_status = FunctionTypeStatus::FullyTyped;
    vec![int, mag]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(module: &IIRModule) -> Vec<String> {
        module.functions[0]
            .instructions
            .iter()
            .map(|i| i.op.clone())
            .collect()
    }

    #[test]
    fn compiles_compare_branch_stop_and_validates() {
        // op_1 compares a field with itself (→ equal), IF EQUAL jumps to op_3,
        // OTHERWISE would jump to op_2; both targets exist so the module is valid.
        let src = "\
(0) INPUT INVENTORY FILE-A ; OUTPUT PRICED FILE-C .
(1) COMPARE PRODUCT-NO (A) WITH PRODUCT-NO (A) ;
    IF EQUAL GO TO OPERATION 3 ; OTHERWISE GO TO OPERATION 2 .
(2) STOP .
(3) STOP . (END)";
        let module = compile_source(src, "inv").unwrap();
        // Well-formed for the AOT chain: single main, entry set, valid.
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.entry_point.as_deref(), Some("main"));
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());

        let os = ops(&module);
        // A field-init const, four operation labels, the three-way compare, the
        // conditional and unconditional jumps, and the STOP rets are all present.
        assert!(os.contains(&"label".to_string()));
        assert!(os.contains(&"cmp_eq".to_string()));
        assert!(os.contains(&"jmp_if_true".to_string()));
        assert!(os.contains(&"jmp".to_string()));
        assert!(os.contains(&"ret".to_string()));

        // Four operation labels (op_0..op_3).
        let labels: Vec<&str> = module.functions[0]
            .instructions
            .iter()
            .filter(|i| i.op == "label")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Var(v)) => Some(v.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, vec!["op_0", "op_1", "op_2", "op_3"]);
    }

    #[test]
    fn move_copies_between_field_registers() {
        let src = "\
(0) INPUT A FILE-A ; OUTPUT B FILE-B .
(1) MOVE UNIT-PRICE (A) TO UNIT-PRICE (B) ; STOP .";
        let module = compile_source(src, "m").unwrap();
        assert!(module.validate().is_empty());
        // The MOVE lowers to a `mov` between the two field registers.
        let mov = module.functions[0]
            .instructions
            .iter()
            .find(|i| i.op == "mov")
            .expect("a mov");
        assert_eq!(mov.dest.as_deref(), Some("fld_UNIT_PRICE__B"));
        assert!(matches!(mov.srcs.first(), Some(Operand::Var(v)) if v == "fld_UNIT_PRICE__A"));
    }

    #[test]
    fn jump_to_operation_is_an_unconditional_branch() {
        let src = "(0) JUMP TO OPERATION 1 .\n(1) STOP .";
        let module = compile_source(src, "j").unwrap();
        assert!(module.validate().is_empty());
        let jmp = module.functions[0].instructions.iter().find(|i| i.op == "jmp").unwrap();
        assert!(matches!(jmp.srcs.first(), Some(Operand::Var(v)) if v == "op_1"));
    }

    #[test]
    fn record_io_is_unsupported_not_wrong() {
        // READ-ITEM needs the file runtime — a later rung. Clean error, not a
        // miscompile.
        let src = "(0) READ-ITEM FILE-A ; STOP .";
        let err = compile_source(src, "r").unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("READ-ITEM"));
    }

    #[test]
    fn digit_print_helper_renders_every_shape() {
        // Exercise __fm_print_int directly (WRITE-ITEM can't reach non-zero
        // fields until READ lands): build a `main` that prints one constant, run
        // it on the JIT, and check the bytes. Covers 0, single/multi-digit, and
        // the negative sign path — the correctness the WRITE record image needs.
        use jit_core::core::JITCore;
        use jit_core::GenericCirJit;
        use std::sync::{Arc, Mutex};
        use vm_core::core::VMCore;
        use vm_core::value::Value;

        fn print(n: i64) -> String {
            let mut instrs = vec![
                IIRInstr::new("const", Some("x".into()), vec![Operand::Int(n)], "i64"),
                IIRInstr::new(
                    "call",
                    Some("_r".into()),
                    vec![Operand::Var("__fm_print_int".into()), Operand::Var("x".into())],
                    "i64",
                ),
                IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
                IIRInstr::new("ret", None, vec![Operand::Var("z".into())], "i64"),
            ];
            let _ = &mut instrs;
            let mut main = IIRFunction::new("main", vec![], "i64", instrs);
            main.type_status = FunctionTypeStatus::FullyTyped;
            let mut module = IIRModule::new("t", "flow-matic");
            module.functions.push(main);
            for f in print_helper_functions() {
                module.functions.push(f);
            }
            module.entry_point = Some("main".into());
            assert!(module.validate().is_empty(), "{:?}", module.validate());

            let mut vm = VMCore::new();
            let chars: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            {
                let chars = Arc::clone(&chars);
                vm.builtins_mut().register("putchar", move |args| {
                    let b = args.first().and_then(|v| v.as_i64()).unwrap_or(0);
                    chars.lock().unwrap().push(b as u8);
                    Ok(Value::Null)
                });
            }
            let backend = GenericCirJit::new();
            let mut jit = JITCore::new(&mut vm, Box::new(backend));
            jit.execute_with_jit(&mut vm, &mut module, "main", &[]).unwrap();
            let bytes = chars.lock().unwrap().clone();
            String::from_utf8(bytes).unwrap()
        }

        assert_eq!(print(0), "0");
        assert_eq!(print(7), "7");
        assert_eq!(print(42), "42");
        assert_eq!(print(100), "100");
        assert_eq!(print(-7), "-7");
        assert_eq!(print(-1234), "-1234");
    }

    #[test]
    fn if_without_a_preceding_compare_reads_a_defined_flag() {
        // A lone IF (no COMPARE) must not read an undefined flag register — the
        // flags are zeroed at entry, so this IF sees false and falls through.
        let src = "(0) IF EQUAL GO TO OPERATION 1 .\n(1) STOP .";
        let module = compile_source(src, "lone_if").unwrap();
        assert!(module.validate().is_empty());
        // The three flag registers are const-initialised before any label.
        let first_ops: Vec<&str> = module.functions[0]
            .instructions
            .iter()
            .take_while(|i| i.op != "label")
            .filter(|i| i.op == "const")
            .filter_map(|i| i.dest.as_deref())
            .collect();
        assert!(first_ops.contains(&"_cmp_eq"), "flags zeroed at entry: {first_ops:?}");
    }

    #[test]
    fn end_of_data_condition_is_unsupported() {
        let src = "(0) IF END OF DATA GO TO OPERATION 1 .\n(1) STOP .";
        let err = compile_source(src, "e").unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }
}

//! # Brainfuck → InterpreterIR compiler (BF04)
//!
//! This module is the bridge between the Brainfuck grammar AST and the
//! [`interpreter_ir`] bytecode that [`vm_core`] executes.
//!
//! ## Architecture
//!
//! ```text
//! Brainfuck source
//!        │
//!        ▼  brainfuck::parse_brainfuck()
//! GrammarASTNode
//!        │
//!        ▼  compile_to_iir()   ← THIS MODULE
//! IIRModule  (one function: "main", no params, FULLY_TYPED)
//!        │
//!        ▼  vm-core or jit-core
//! execution result
//! ```
//!
//! ## Why one flat function?
//!
//! Brainfuck has no functions, no scopes, no parameters — just a sequence
//! of commands and (possibly nested) loops.  Everything compiles into a
//! single `IIRFunction` named `"main"` with no parameters.  Loops become
//! `label` + `jmp_if_*` pairs within that function; the IIR's named-label
//! control flow handles arbitrary nesting without extra bookkeeping.
//!
//! ## Why fixed register names instead of SSA?
//!
//! The IIR's frame model is plain mutable registers: assigning to the same
//! name overwrites the slot.  Using SSA-style unique names per write would
//! break programs where a skip block (loop body not taken) leaves a name
//! undefined.  The IIR has no phi-nodes to paper over this, so we use four
//! fixed register names (`ptr`, `v`, `c`, `k`) and overwrite them freely —
//! exactly what a hand-written interpreter would do.
//!
//! ## Type hints
//!
//! Every emitted instruction carries a concrete `type_hint`:
//! - `"u8"` for cell values and cell-arithmetic immediates
//! - `"u32"` for pointer values and pointer-arithmetic immediates
//! - `"void"` for control-flow / side-effect ops
//!
//! This makes the resulting `IIRFunction` `FULLY_TYPED`, so the JIT
//! (BF05) can tier up on first call without waiting for the profiler.
//!
//! ## Command → IIR mapping
//!
//! | BF command | IIR emitted |
//! |---|---|
//! | `>` | `const k 1 u32` + `add ptr ptr k u32` |
//! | `<` | `const k 1 u32` + `sub ptr ptr k u32` |
//! | `+` | `load_mem v ptr u8` + `const k 1 u8` + `add v v k u8` + `store_mem ptr v u8` |
//! | `-` | `load_mem v ptr u8` + `const k 1 u8` + `sub v v k u8` + `store_mem ptr v u8` |
//! | `.` | `load_mem v ptr u8` + `call_builtin putchar v void` |
//! | `,` | `call_builtin getchar () u8` → `v` + `store_mem ptr v u8` |
//! | `[`…`]` | label loop_N_start + load_mem c ptr u8 + jmp_if_false c loop_N_end + body + jmp loop_N_start + label loop_N_end |

use interpreter_ir::{
    function::{FunctionTypeStatus, IIRFunction},
    instr::{IIRInstr, Operand},
    module::IIRModule,
};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ---------------------------------------------------------------------------
// Register names (fixed allocation — see module doc for rationale)
// ---------------------------------------------------------------------------

/// The data-pointer register.  Contains the current tape index as a `u32`.
const PTR: &str = "ptr";

/// Scratch register for cell values.  Written by `load_mem`, read by `.`/`,`.
const VAL: &str = "v";

/// Loop-condition register.  Written at each loop guard, read by `jmp_if_false`.
const COND: &str = "c";

/// Immediate-scratch register for the constant `1` used in `+`, `-`, `>`, `<`.
const IMM: &str = "k";

/// Total number of named registers + headroom for vm-core internals.
const REGISTER_COUNT: usize = 8; // 4 named + 4 headroom

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Compile a Brainfuck source string into a single-function [`IIRModule`].
///
/// This is the primary entry point: it lexes, parses, and compiles the source
/// in one call, returning an `IIRModule` ready for [`vm_core::core::VMCore`].
///
/// # Errors
///
/// Returns an error string if the parser rejects the source (e.g. unmatched
/// brackets).
///
/// # Example
///
/// ```
/// use brainfuck_iir_compiler::compile_source;
///
/// let module = compile_source("+++", "demo").unwrap();
/// assert_eq!(module.functions.len(), 1);
/// assert_eq!(module.entry_point, Some("main".to_string()));
/// ```
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    let ast = brainfuck::parser::parse_brainfuck(source)?;
    Ok(compile_to_iir(&ast, module_name))
}

/// Compile a parsed Brainfuck [`GrammarASTNode`] into a single-function
/// [`IIRModule`].
///
/// The returned module contains exactly one function named `"main"` with no
/// parameters and `return_type = "void"`.  Its `type_status` is
/// [`FunctionTypeStatus::FullyTyped`] because every emitted instruction
/// carries a concrete `type_hint`.
///
/// # Example
///
/// ```
/// use brainfuck_iir_compiler::compile_source;
///
/// let module = compile_source(">+<", "test").unwrap();
/// let f = &module.functions[0];
/// assert_eq!(f.type_status, interpreter_ir::function::FunctionTypeStatus::FullyTyped);
/// ```
pub fn compile_to_iir(ast: &GrammarASTNode, module_name: &str) -> IIRModule {
    let mut compiler = Compiler::new();
    compiler.emit_program(ast);
    let fn_ = compiler.finish();
    IIRModule {
        name: module_name.to_string(),
        functions: vec![fn_],
        entry_point: Some("main".to_string()),
        language: "brainfuck".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Internal compiler
// ---------------------------------------------------------------------------

/// Stateful AST walker that accumulates [`IIRInstr`]s.
///
/// The compiler keeps:
/// - `instrs` — the ordered instruction list being built.
/// - `loop_counter` — depth-first index for unique loop labels.
struct Compiler {
    instrs: Vec<IIRInstr>,
    loop_counter: usize,
}

impl Compiler {
    fn new() -> Self {
        Self {
            instrs: Vec::new(),
            loop_counter: 0,
        }
    }

    // ------------------------------------------------------------------
    // Low-level emit helper
    // ------------------------------------------------------------------

    fn emit(
        &mut self,
        op: &str,
        dest: Option<&str>,
        srcs: Vec<Operand>,
        type_hint: &str,
    ) {
        self.instrs.push(IIRInstr::new(
            op,
            dest.map(|s| s.to_string()),
            srcs,
            type_hint,
        ));
    }

    // ------------------------------------------------------------------
    // Program-level emitters
    // ------------------------------------------------------------------

    /// Walk the program AST, emit prologue, body, and epilogue.
    fn emit_program(&mut self, ast: &GrammarASTNode) {
        // Prologue: initialise the data pointer to cell 0.
        self.emit("const", Some(PTR), vec![Operand::Int(0)], "u32");

        for child in &ast.children {
            if let ASTNodeOrToken::Node(node) = child {
                self.emit_node(node);
            }
        }

        // Epilogue: explicit return to cleanly terminate the interpreter frame.
        self.emit("ret_void", None, vec![], "void");
    }

    /// Produce the finished [`IIRFunction`].
    fn finish(self) -> IIRFunction {
        IIRFunction {
            name: "main".to_string(),
            params: vec![],
            return_type: "void".to_string(),
            instructions: self.instrs,
            register_count: REGISTER_COUNT,
            type_status: FunctionTypeStatus::FullyTyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: vec![],
        }
    }

    // ------------------------------------------------------------------
    // AST dispatch
    // ------------------------------------------------------------------

    fn emit_node(&mut self, node: &GrammarASTNode) {
        match node.rule_name.as_str() {
            "instruction" => {
                // `instruction = loop | command` — one child.
                for child in &node.children {
                    if let ASTNodeOrToken::Node(child_node) = child {
                        self.emit_node(child_node);
                    }
                }
            }
            "loop" => self.emit_loop(node),
            "command" => self.emit_command(node),
            other => {
                // Defensive: unknown rule means the grammar drifted.
                panic!("unexpected AST rule: {other:?}");
            }
        }
    }

    // ------------------------------------------------------------------
    // Per-command emitters
    // ------------------------------------------------------------------

    fn emit_command(&mut self, node: &GrammarASTNode) {
        let tok = first_token(node).expect("command node has no token");
        match tok.value.as_str() {
            ">" => self.emit_ptr_shift(true),
            "<" => self.emit_ptr_shift(false),
            "+" => self.emit_cell_mutation(true),
            "-" => self.emit_cell_mutation(false),
            "." => self.emit_output(),
            "," => self.emit_input(),
            other => panic!("unknown brainfuck command: {other:?}"),
        }
    }

    /// Compile `>` (right=true) or `<` (right=false) — pointer shift.
    ///
    /// ```text
    /// const  k  1      u32
    /// add/sub ptr ptr k u32
    /// ```
    fn emit_ptr_shift(&mut self, right: bool) {
        self.emit("const", Some(IMM), vec![Operand::Int(1)], "u32");
        let op = if right { "add" } else { "sub" };
        self.emit(
            op,
            Some(PTR),
            vec![Operand::Var(PTR.into()), Operand::Var(IMM.into())],
            "u32",
        );
    }

    /// Compile `+` (inc=true) or `-` (inc=false) — cell mutation.
    ///
    /// ```text
    /// load_mem  v  ptr     u8
    /// const     k  1       u8
    /// add/sub   v  v k     u8
    /// store_mem    ptr v   u8
    /// ```
    fn emit_cell_mutation(&mut self, inc: bool) {
        self.emit(
            "load_mem",
            Some(VAL),
            vec![Operand::Var(PTR.into())],
            "u8",
        );
        self.emit("const", Some(IMM), vec![Operand::Int(1)], "u8");
        let op = if inc { "add" } else { "sub" };
        self.emit(
            op,
            Some(VAL),
            vec![Operand::Var(VAL.into()), Operand::Var(IMM.into())],
            "u8",
        );
        self.emit(
            "store_mem",
            None,
            vec![Operand::Var(PTR.into()), Operand::Var(VAL.into())],
            "u8",
        );
    }

    /// Compile `.` — write current cell to stdout via `putchar`.
    ///
    /// ```text
    /// load_mem    v  ptr      u8
    /// call_builtin   putchar v void
    /// ```
    fn emit_output(&mut self) {
        self.emit(
            "load_mem",
            Some(VAL),
            vec![Operand::Var(PTR.into())],
            "u8",
        );
        self.emit(
            "call_builtin",
            None,
            vec![
                Operand::Var("putchar".into()),
                Operand::Var(VAL.into()),
            ],
            "void",
        );
    }

    /// Compile `,` — read one byte from stdin via `getchar`.
    ///
    /// ```text
    /// call_builtin  v  getchar   u8
    /// store_mem        ptr v     u8
    /// ```
    fn emit_input(&mut self) {
        self.emit(
            "call_builtin",
            Some(VAL),
            vec![Operand::Var("getchar".into())],
            "u8",
        );
        self.emit(
            "store_mem",
            None,
            vec![Operand::Var(PTR.into()), Operand::Var(VAL.into())],
            "u8",
        );
    }

    // ------------------------------------------------------------------
    // Loop emitter
    // ------------------------------------------------------------------

    /// Compile `[body]` into a structured loop.
    ///
    /// The emitted shape matches what `ir-to-wasm-compiler` expects for
    /// structured-loop recognition (BF05):
    ///
    /// ```text
    /// label   loop_N_start
    /// load_mem c ptr           u8    ; load current cell as loop guard
    /// jmp_if_false c loop_N_end      ; exit if cell == 0
    /// ... body ...
    /// jmp     loop_N_start           ; unconditional back-edge
    /// label   loop_N_end
    /// ```
    ///
    /// Brainfuck semantics ("loop while cell ≠ 0") are preserved by the
    /// top-of-loop guard: each iteration re-reads the cell, so any body
    /// that moves the pointer and changes the current cell will be
    /// re-tested before the next iteration.
    fn emit_loop(&mut self, node: &GrammarASTNode) {
        let id = self.loop_counter;
        self.loop_counter += 1;
        let start = format!("loop_{id}_start");
        let end = format!("loop_{id}_end");

        // Entry label + per-iteration guard.
        self.emit("label", None, vec![Operand::Var(start.clone())], "void");
        self.emit("load_mem", Some(COND), vec![Operand::Var(PTR.into())], "u8");
        self.emit(
            "jmp_if_false",
            None,
            vec![Operand::Var(COND.into()), Operand::Var(end.clone())],
            "void",
        );

        // Body: skip bracket tokens, emit only rule-based children.
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                self.emit_node(child_node);
            }
        }

        // Unconditional back-edge — the next guard decides whether to continue.
        self.emit("jmp", None, vec![Operand::Var(start)], "void");
        self.emit("label", None, vec![Operand::Var(end)], "void");
    }
}

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

/// Return the first [`lexer::token::Token`] descendant of `node`, or `None`.
fn first_token(node: &GrammarASTNode) -> Option<lexer::token::Token> {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(tok) => return Some(tok.clone()),
            ASTNodeOrToken::Node(inner) => {
                if let Some(tok) = first_token(inner) {
                    return Some(tok);
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(src: &str) -> IIRModule {
        compile_source(src, "test").expect("parse should succeed")
    }

    fn instrs(src: &str) -> Vec<IIRInstr> {
        compile(src).functions.into_iter().next().unwrap().instructions
    }

    // --- Module structure ---

    #[test]
    fn empty_program_has_prologue_and_epilogue() {
        let m = compile("");
        let f = &m.functions[0];
        // prologue: const ptr 0 u32
        // epilogue: ret_void
        assert_eq!(f.instructions.len(), 2);
        assert_eq!(f.instructions[0].op, "const");
        assert_eq!(f.instructions.last().unwrap().op, "ret_void");
    }

    #[test]
    fn module_entry_point_is_main() {
        assert_eq!(compile("").entry_point, Some("main".to_string()));
    }

    #[test]
    fn function_name_is_main() {
        assert_eq!(compile("").functions[0].name, "main");
    }

    #[test]
    fn function_has_no_params() {
        assert!(compile("").functions[0].params.is_empty());
    }

    #[test]
    fn function_is_fully_typed() {
        assert_eq!(
            compile("").functions[0].type_status,
            FunctionTypeStatus::FullyTyped
        );
    }

    #[test]
    fn module_language_is_brainfuck() {
        assert_eq!(compile("").language, "brainfuck");
    }

    #[test]
    fn module_name_forwarded() {
        let m = compile_source("+", "mymod").unwrap();
        assert_eq!(m.name, "mymod");
    }

    // --- Pointer shift `>` / `<` ---

    #[test]
    fn right_emits_const_and_add() {
        let i = instrs(">");
        // prologue const ptr 0, then const k 1, add ptr ptr k, then ret_void
        assert_eq!(i[1].op, "const");
        assert_eq!(i[1].dest, Some(IMM.into()));
        assert_eq!(i[2].op, "add");
        assert_eq!(i[2].dest, Some(PTR.into()));
        assert_eq!(i[2].type_hint, "u32");
    }

    #[test]
    fn left_emits_const_and_sub() {
        let i = instrs("<");
        assert_eq!(i[1].op, "const");
        assert_eq!(i[2].op, "sub");
        assert_eq!(i[2].dest, Some(PTR.into()));
        assert_eq!(i[2].type_hint, "u32");
    }

    // --- Cell mutation `+` / `-` ---

    #[test]
    fn inc_emits_four_instructions() {
        let i = instrs("+");
        // prologue + 4 cell-mutation instrs + epilogue
        assert_eq!(i[1].op, "load_mem");
        assert_eq!(i[2].op, "const");
        assert_eq!(i[3].op, "add");
        assert_eq!(i[4].op, "store_mem");
        assert_eq!(i[3].type_hint, "u8");
    }

    #[test]
    fn dec_emits_sub_not_add() {
        let i = instrs("-");
        assert_eq!(i[3].op, "sub");
        assert_eq!(i[3].type_hint, "u8");
    }

    #[test]
    fn inc_store_has_no_dest() {
        let i = instrs("+");
        assert_eq!(i[4].dest, None); // store_mem has no dest
    }

    // --- Output `.` ---

    #[test]
    fn dot_emits_load_mem_and_call_builtin() {
        let i = instrs(".");
        assert_eq!(i[1].op, "load_mem");
        assert_eq!(i[2].op, "call_builtin");
        assert_eq!(i[2].type_hint, "void");
    }

    #[test]
    fn dot_call_builtin_has_putchar_as_first_src() {
        let i = instrs(".");
        assert_eq!(i[2].srcs[0], Operand::Var("putchar".into()));
    }

    // --- Input `,` ---

    #[test]
    fn comma_emits_call_builtin_getchar_and_store() {
        let i = instrs(",");
        assert_eq!(i[1].op, "call_builtin");
        assert_eq!(i[1].srcs[0], Operand::Var("getchar".into()));
        assert_eq!(i[1].type_hint, "u8");
        assert_eq!(i[2].op, "store_mem");
    }

    // --- Loop `[...]` ---

    #[test]
    fn empty_loop_emits_label_load_jmp_label() {
        let i = instrs("[]");
        // prologue, label start, load_mem c, jmp_if_false, jmp back, label end, epilogue
        assert_eq!(i[1].op, "label");
        assert_eq!(i[2].op, "load_mem");
        assert_eq!(i[2].dest, Some(COND.into()));
        assert_eq!(i[3].op, "jmp_if_false");
        assert_eq!(i[4].op, "jmp");
        assert_eq!(i[5].op, "label");
    }

    #[test]
    fn loop_labels_are_consistent() {
        let i = instrs("[]");
        let start_label = match &i[1].srcs[0] {
            Operand::Var(s) => s.clone(),
            _ => panic!("expected Var"),
        };
        let jmp_label = match &i[4].srcs[0] {
            Operand::Var(s) => s.clone(),
            _ => panic!("expected Var"),
        };
        assert_eq!(start_label, jmp_label);
        assert!(start_label.contains("start"));
    }

    #[test]
    fn nested_loops_get_distinct_ids() {
        let i = instrs("[[]]");
        // outer label_0_start, inner label_1_start, etc.
        let outer_start = match &i[1].srcs[0] {
            Operand::Var(s) => s.clone(),
            _ => panic!(),
        };
        // Find next "label" after the first one
        let inner_start = i.iter().skip(2).find(|instr| instr.op == "label").map(|instr| match &instr.srcs[0] {
            Operand::Var(s) => s.clone(),
            _ => panic!(),
        }).unwrap();
        assert_ne!(outer_start, inner_start);
    }

    // --- Complex programs ---

    #[test]
    fn hello_world_compiles_without_error() {
        let src = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.";
        compile_source(src, "hw").unwrap();
    }

    #[test]
    fn compile_source_propagates_parse_error_on_unmatched_bracket() {
        let result = compile_source("[", "bad");
        assert!(result.is_err());
    }

    #[test]
    fn all_instrs_have_concrete_type_hint() {
        let i = instrs("+>-<.[,]");
        for instr in &i {
            assert!(
                !instr.type_hint.is_empty() && instr.type_hint != "any",
                "instruction {}: has ambiguous type_hint {:?}",
                instr.op,
                instr.type_hint
            );
        }
    }

    #[test]
    fn multiple_commands_accumulate_correctly() {
        // `>><` should produce 2 right-shifts + 1 left-shift = 6 body instrs
        // (each shift is 2 instrs: const + add/sub)
        let i = instrs(">><");
        // prologue=1, body=(2+2+2)=6, epilogue=1 → total 8
        assert_eq!(i.len(), 8);
    }
}

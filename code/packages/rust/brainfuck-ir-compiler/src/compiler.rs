//! # Brainfuck IR Compiler — translates a Brainfuck AST into IR.
//!
//! This module walks the `GrammarASTNode` produced by the Brainfuck parser
//! and emits IR instructions for each node. It also builds the first two
//! segments of the source map chain:
//!
//! - **Segment 1: SourceToAst** — source positions → AST node IDs
//! - **Segment 2: AstToIr** — AST node IDs → IR instruction IDs
//!
//! ## Register allocation
//!
//! Brainfuck needs very few registers:
//!
//! ```text
//! v0 = tape base address  (pointer to the start of the tape)
//! v1 = tape pointer offset (current cell index, 0-based)
//! v2 = temporary (cell value for loads/stores)
//! v3 = temporary (for bounds checks)
//! v4 = temporary (for syscall arguments)
//! v5 = max pointer value  (tape_size - 1, for bounds checks)
//! v6 = zero constant      (for bounds checks)
//! ```
//!
//! ## Compilation mapping
//!
//! | Command | IR Output |
//! |---------|-----------|
//! | `>`     | `ADD_IMM v1, v1, 1` |
//! | `<`     | `ADD_IMM v1, v1, -1` |
//! | `+`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
//! | `-`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
//! | `.`     | `LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1` |
//! | `,`     | `SYSCALL 2; STORE_BYTE v4, v0, v1` |
//! | `[`/`]` | `LABEL loop_N_start; LOAD_BYTE; BRANCH_Z loop_N_end; ...; JUMP loop_N_start; LABEL loop_N_end` |

use compiler_ir::{
    IrOp, IrOperand, IrInstruction, IrDataDecl, IrProgram, IdGenerator,
};
use compiler_source_map::{SourceMapChain, SourcePosition};
use parser::grammar_parser::{GrammarASTNode, ASTNodeOrToken};
use crate::build_config::BuildConfig;

// ===========================================================================
// Register constants
// ===========================================================================

/// v0: base address of the tape
const REG_TAPE_BASE: usize = 0;
/// v1: current cell offset (0-based index)
const REG_TAPE_PTR: usize = 1;
/// v2: temporary for cell values
const REG_TEMP: usize = 2;
/// v3: temporary for bounds checks
const REG_TEMP2: usize = 3;
/// v4: syscall argument register
const REG_SYS_ARG: usize = 4;
/// v5: tape_size - 1 (max valid pointer, for upper bounds check)
const REG_MAX_PTR: usize = 5;
/// v6: constant 0 (for lower bounds check)
const REG_ZERO: usize = 6;

// ===========================================================================
// Syscall numbers
// ===========================================================================

/// Syscall 1 = write byte in v4 to stdout
const SYSCALL_WRITE: i64 = 1;
/// Syscall 2 = read byte from stdin into v4
const SYSCALL_READ: i64 = 2;
/// Syscall 10 = halt with exit code in v4
const SYSCALL_EXIT: i64 = 10;

// ===========================================================================
// CompileResult
// ===========================================================================

/// The result of compiling a Brainfuck AST.
#[derive(Debug)]
pub struct CompileResult {
    /// The compiled IR program.
    pub program: IrProgram,
    /// Source map chain with `SourceToAst` and `AstToIr` segments filled in.
    pub source_map: SourceMapChain,
}

// ===========================================================================
// compile — public entry point
// ===========================================================================

/// Compile a Brainfuck `GrammarASTNode` into an `IrProgram`.
///
/// The `filename` parameter is used in source map entries to identify which
/// file the source positions refer to.
///
/// # Errors
///
/// Returns an error if:
/// - The AST root is not a `"program"` node
/// - `config.tape_size` is 0
/// - An unexpected AST node type is encountered
///
/// # Example
///
/// ```no_run
/// use brainfuck_ir_compiler::compiler::compile;
/// use brainfuck_ir_compiler::build_config::release_config;
/// use brainfuck::parser::parse_brainfuck;
///
/// let ast = parse_brainfuck("+.").unwrap();
/// let result = compile(&ast, "test.bf", release_config()).unwrap();
/// assert!(!result.program.instructions.is_empty());
/// ```
pub fn compile(
    ast: &GrammarASTNode,
    filename: &str,
    config: BuildConfig,
) -> Result<CompileResult, String> {
    // Validate inputs
    if ast.rule_name != "program" {
        return Err(format!(
            "expected 'program' AST node, got {:?}",
            ast.rule_name
        ));
    }
    if config.tape_size == 0 {
        return Err("invalid tape_size 0: must be positive".to_string());
    }

    let mut c = Compiler {
        config: config.clone(),
        filename: filename.to_string(),
        id_gen: IdGenerator::new(),
        node_id_gen: 0,
        program: IrProgram::new("_start"),
        source_map: SourceMapChain::new(),
        loop_count: 0,
    };

    // Add tape data declaration
    c.program.add_data(IrDataDecl {
        label: "tape".to_string(),
        size: config.tape_size,
        init: 0,
    });

    // Emit prologue (_start label + register setup)
    c.emit_prologue();

    // Compile the program body
    c.compile_program(ast)?;

    // Emit epilogue (HALT + optional __trap_oob)
    c.emit_epilogue();

    Ok(CompileResult {
        program: c.program,
        source_map: c.source_map,
    })
}

// ===========================================================================
// Internal compiler state
// ===========================================================================

struct Compiler {
    config: BuildConfig,
    filename: String,
    id_gen: IdGenerator,
    node_id_gen: usize,
    program: IrProgram,
    source_map: SourceMapChain,
    loop_count: usize,
}

impl Compiler {
    // ── ID management ──────────────────────────────────────────────────────

    fn next_node_id(&mut self) -> usize {
        let id = self.node_id_gen;
        self.node_id_gen += 1;
        id
    }

    // ── Instruction emission ───────────────────────────────────────────────

    /// Emit an instruction and return its ID.
    fn emit(&mut self, opcode: IrOp, operands: Vec<IrOperand>) -> i64 {
        let id = self.id_gen.next();
        self.program.add_instruction(IrInstruction::new(opcode, operands, id));
        id
    }

    /// Emit a label instruction (labels have ID -1 — they produce no machine code).
    fn emit_label(&mut self, name: &str) {
        self.program.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label(name.to_string())],
            -1,
        ));
    }

    // ── Helpers for register operands ──────────────────────────────────────

    fn reg(idx: usize) -> IrOperand { IrOperand::Register(idx) }
    fn imm(val: i64) -> IrOperand   { IrOperand::Immediate(val) }
    fn lbl(name: &str) -> IrOperand { IrOperand::Label(name.to_string()) }

    // ── Prologue ───────────────────────────────────────────────────────────

    /// Emit the program prologue: set up the tape base address and pointer.
    ///
    /// ```text
    /// _start:
    ///   LOAD_ADDR  v0, tape    ; v0 = &tape
    ///   LOAD_IMM   v1, 0       ; v1 = 0 (tape pointer starts at cell 0)
    ///   [debug only]
    ///   LOAD_IMM   v5, N-1    ; v5 = tape_size - 1 (upper bound)
    ///   LOAD_IMM   v6, 0       ; v6 = 0 (lower bound constant)
    /// ```
    fn emit_prologue(&mut self) {
        self.emit_label("_start");

        // v0 = &tape
        self.emit(IrOp::LoadAddr, vec![
            Self::reg(REG_TAPE_BASE),
            Self::lbl("tape"),
        ]);

        // v1 = 0
        self.emit(IrOp::LoadImm, vec![
            Self::reg(REG_TAPE_PTR),
            Self::imm(0),
        ]);

        // Debug mode: set up bounds-check registers
        if self.config.insert_bounds_checks {
            // v5 = tape_size - 1
            self.emit(IrOp::LoadImm, vec![
                Self::reg(REG_MAX_PTR),
                Self::imm(self.config.tape_size as i64 - 1),
            ]);
            // v6 = 0
            self.emit(IrOp::LoadImm, vec![
                Self::reg(REG_ZERO),
                Self::imm(0),
            ]);
        }
    }

    // ── Epilogue ───────────────────────────────────────────────────────────

    /// Emit the program epilogue: HALT + optional __trap_oob handler.
    fn emit_epilogue(&mut self) {
        self.emit(IrOp::Halt, vec![]);

        if self.config.insert_bounds_checks {
            self.emit_label("__trap_oob");
            // v4 = 1 (error exit code)
            self.emit(IrOp::LoadImm, vec![
                Self::reg(REG_SYS_ARG),
                Self::imm(1),
            ]);
            // exit(1)
            self.emit(IrOp::Syscall, vec![Self::imm(SYSCALL_EXIT)]);
        }
    }

    // ── AST walking ────────────────────────────────────────────────────────

    /// Compile all instruction children of the `program` node.
    fn compile_program(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                self.compile_node(child_node)?;
            }
            // Tokens at the program level are ignored
        }
        Ok(())
    }

    fn compile_node(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        match node.rule_name.as_str() {
            "instruction" => {
                // An instruction wraps either a loop or a command
                for child in &node.children {
                    if let ASTNodeOrToken::Node(child_node) = child {
                        self.compile_node(child_node)?;
                    }
                }
                Ok(())
            }
            "command" => self.compile_command(node),
            "loop" => self.compile_loop(node),
            other => Err(format!("unexpected AST node type: {:?}", other)),
        }
    }

    // ── Command compilation ────────────────────────────────────────────────

    /// Compile a single Brainfuck command node.
    fn compile_command(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        // Extract the token from the command node
        let tok = extract_token(node)
            .ok_or_else(|| "command node has no token".to_string())?;

        let ast_node_id = self.next_node_id();
        self.source_map.source_to_ast.add(
            SourcePosition {
                file: self.filename.clone(),
                line: tok.line,
                column: tok.column,
                length: 1,
            },
            ast_node_id,
        );

        let mut ir_ids: Vec<i64> = Vec::new();

        match tok.value.as_str() {
            ">" => {
                // RIGHT: move tape pointer right
                if self.config.insert_bounds_checks {
                    ir_ids.extend(self.emit_bounds_check_right());
                }
                let id = self.emit(IrOp::AddImm, vec![
                    Self::reg(REG_TAPE_PTR),
                    Self::reg(REG_TAPE_PTR),
                    Self::imm(1),
                ]);
                ir_ids.push(id);
            }
            "<" => {
                // LEFT: move tape pointer left
                if self.config.insert_bounds_checks {
                    ir_ids.extend(self.emit_bounds_check_left());
                }
                let id = self.emit(IrOp::AddImm, vec![
                    Self::reg(REG_TAPE_PTR),
                    Self::reg(REG_TAPE_PTR),
                    Self::imm(-1),
                ]);
                ir_ids.push(id);
            }
            "+" => {
                // INC: increment current cell
                ir_ids.extend(self.emit_cell_mutation(1));
            }
            "-" => {
                // DEC: decrement current cell
                ir_ids.extend(self.emit_cell_mutation(-1));
            }
            "." => {
                // OUTPUT: write current cell to stdout
                // Load current cell value
                let id1 = self.emit(IrOp::LoadByte, vec![
                    Self::reg(REG_TEMP),
                    Self::reg(REG_TAPE_BASE),
                    Self::reg(REG_TAPE_PTR),
                ]);
                ir_ids.push(id1);
                // Copy to the syscall argument register without depending on v6.
                let id2 = self.emit(IrOp::AddImm, vec![
                    Self::reg(REG_SYS_ARG),
                    Self::reg(REG_TEMP),
                    Self::imm(0),
                ]);
                ir_ids.push(id2);
                // Syscall 1 = write byte
                let id3 = self.emit(IrOp::Syscall, vec![Self::imm(SYSCALL_WRITE)]);
                ir_ids.push(id3);
            }
            "," => {
                // INPUT: read byte from stdin into current cell
                // Syscall 2 = read byte
                let id1 = self.emit(IrOp::Syscall, vec![Self::imm(SYSCALL_READ)]);
                ir_ids.push(id1);
                // Store result (in v4) to current cell
                let id2 = self.emit(IrOp::StoreByte, vec![
                    Self::reg(REG_SYS_ARG),
                    Self::reg(REG_TAPE_BASE),
                    Self::reg(REG_TAPE_PTR),
                ]);
                ir_ids.push(id2);
            }
            other => {
                return Err(format!("unknown command token: {:?}", other));
            }
        }

        self.source_map.ast_to_ir.add(ast_node_id, ir_ids);
        Ok(())
    }

    /// Emit IR for incrementing or decrementing the current cell by `delta`.
    ///
    /// ```text
    /// LOAD_BYTE  v2, v0, v1    ← load current cell
    /// ADD_IMM    v2, v2, delta  ← increment/decrement
    /// AND_IMM    v2, v2, 255    ← mask to byte (if enabled)
    /// STORE_BYTE v2, v0, v1    ← store back
    /// ```
    fn emit_cell_mutation(&mut self, delta: i64) -> Vec<i64> {
        let mut ids = Vec::new();

        // Load current cell value
        let id = self.emit(IrOp::LoadByte, vec![
            Self::reg(REG_TEMP),
            Self::reg(REG_TAPE_BASE),
            Self::reg(REG_TAPE_PTR),
        ]);
        ids.push(id);

        // Add delta
        let id = self.emit(IrOp::AddImm, vec![
            Self::reg(REG_TEMP),
            Self::reg(REG_TEMP),
            Self::imm(delta),
        ]);
        ids.push(id);

        // Mask to byte range (0-255) if enabled
        if self.config.mask_byte_arithmetic {
            let id = self.emit(IrOp::AndImm, vec![
                Self::reg(REG_TEMP),
                Self::reg(REG_TEMP),
                Self::imm(255),
            ]);
            ids.push(id);
        }

        // Store back to cell
        let id = self.emit(IrOp::StoreByte, vec![
            Self::reg(REG_TEMP),
            Self::reg(REG_TAPE_BASE),
            Self::reg(REG_TAPE_PTR),
        ]);
        ids.push(id);

        ids
    }

    // ── Bounds checking ────────────────────────────────────────────────────

    /// Emit bounds check for RIGHT (`>`):
    ///
    /// ```text
    /// CMP_GT    v3, v1, v5        ← is ptr > tape_size - 1?
    /// BRANCH_NZ v3, __trap_oob   ← if so, trap
    /// ```
    fn emit_bounds_check_right(&mut self) -> Vec<i64> {
        let mut ids = Vec::new();
        let id = self.emit(IrOp::CmpGt, vec![
            Self::reg(REG_TEMP2),
            Self::reg(REG_TAPE_PTR),
            Self::reg(REG_MAX_PTR),
        ]);
        ids.push(id);
        let id = self.emit(IrOp::BranchNz, vec![
            Self::reg(REG_TEMP2),
            Self::lbl("__trap_oob"),
        ]);
        ids.push(id);
        ids
    }

    /// Emit bounds check for LEFT (`<`):
    ///
    /// ```text
    /// CMP_LT    v1, v1, v6        ← is ptr < 0?
    /// BRANCH_NZ v1, __trap_oob   ← if so, trap
    /// ```
    fn emit_bounds_check_left(&mut self) -> Vec<i64> {
        let mut ids = Vec::new();
        let id = self.emit(IrOp::CmpLt, vec![
            Self::reg(REG_TAPE_PTR),
            Self::reg(REG_TAPE_PTR),
            Self::reg(REG_ZERO),
        ]);
        ids.push(id);
        let id = self.emit(IrOp::BranchNz, vec![
            Self::reg(REG_TAPE_PTR),
            Self::lbl("__trap_oob"),
        ]);
        ids.push(id);
        ids
    }

    // ── Loop compilation ───────────────────────────────────────────────────

    /// Compile a Brainfuck loop `[body]`.
    ///
    /// ```text
    /// LABEL      loop_N_start
    /// LOAD_BYTE  v2, v0, v1         ← load current cell
    /// BRANCH_Z   v2, loop_N_end    ← skip body if cell == 0
    /// ...compile body...
    /// JUMP       loop_N_start       ← repeat
    /// LABEL      loop_N_end
    /// ```
    fn compile_loop(&mut self, node: &GrammarASTNode) -> Result<(), String> {
        let loop_num = self.loop_count;
        self.loop_count += 1;
        let start_label = format!("loop_{}_start", loop_num);
        let end_label = format!("loop_{}_end", loop_num);

        // Find the LOOP_START token for source mapping
        let ast_node_id = self.next_node_id();
        if let Some(start_line) = node.start_line {
            let start_col = node.start_column.unwrap_or(1);
            self.source_map.source_to_ast.add(
                SourcePosition {
                    file: self.filename.clone(),
                    line: start_line,
                    column: start_col,
                    length: 1,
                },
                ast_node_id,
            );
        }

        let mut ir_ids: Vec<i64> = Vec::new();

        // Emit loop start label
        self.emit_label(&start_label);

        // Load current cell and branch if zero
        let id = self.emit(IrOp::LoadByte, vec![
            Self::reg(REG_TEMP),
            Self::reg(REG_TAPE_BASE),
            Self::reg(REG_TAPE_PTR),
        ]);
        ir_ids.push(id);

        let id = self.emit(IrOp::BranchZ, vec![
            Self::reg(REG_TEMP),
            Self::lbl(&end_label),
        ]);
        ir_ids.push(id);

        // Compile loop body (skip bracket tokens, process child nodes)
        for child in &node.children {
            if let ASTNodeOrToken::Node(child_node) = child {
                self.compile_node(child_node)?;
            }
            // Bracket tokens (LOOP_START, LOOP_END) are skipped
        }

        // Jump back to loop start
        let id = self.emit(IrOp::Jump, vec![Self::lbl(&start_label)]);
        ir_ids.push(id);

        // Emit loop end label
        self.emit_label(&end_label);

        // Record AST → IR mapping for the loop construct
        self.source_map.ast_to_ir.add(ast_node_id, ir_ids);

        Ok(())
    }
}

// ===========================================================================
// Token extraction
// ===========================================================================

/// Extract the leaf token from an AST node.
///
/// The AST structure is:
/// ```text
/// command → Token (leaf node wrapping a single token)
/// ```
///
/// This helper digs through the AST to find the leaf token.
fn extract_token(node: &GrammarASTNode) -> Option<&lexer::token::Token> {
    if node.is_leaf() {
        return node.token();
    }
    for child in &node.children {
        match child {
            ASTNodeOrToken::Token(tok) => return Some(tok),
            ASTNodeOrToken::Node(child_node) => {
                if let Some(tok) = extract_token(child_node) {
                    return Some(tok);
                }
            }
        }
    }
    None
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_config::{debug_config, release_config};
    use brainfuck::parser::parse_brainfuck;
    use compiler_ir::print_ir;
    use compiler_ir::ir_parser::parse_ir;

    // ── Test helpers ──────────────────────────────────────────────────────

    fn compile_source(source: &str, config: BuildConfig) -> Result<CompileResult, String> {
        let ast = parse_brainfuck(source)
            .map_err(|e| format!("parse error: {:?}", e))?;
        compile(&ast, "test.bf", config)
    }

    fn must_compile(source: &str, config: BuildConfig) -> CompileResult {
        compile_source(source, config).expect("compile failed")
    }

    fn count_opcode(program: &IrProgram, opcode: IrOp) -> usize {
        program.instructions.iter().filter(|i| i.opcode == opcode).count()
    }

    fn has_label(program: &IrProgram, name: &str) -> bool {
        program.instructions.iter().any(|i| {
            i.opcode == IrOp::Label
                && i.operands.first() == Some(&IrOperand::Label(name.to_string()))
        })
    }

    // ── Empty program ─────────────────────────────────────────────────────

    #[test]
    fn test_compile_empty_program() {
        let result = must_compile("", release_config());
        assert!(has_label(&result.program, "_start"));
        assert_eq!(count_opcode(&result.program, IrOp::Halt), 1);
        assert_eq!(result.program.version, 1);
        assert_eq!(result.program.entry_label, "_start");
    }

    #[test]
    fn test_compile_empty_program_has_tape_data() {
        let result = must_compile("", release_config());
        assert_eq!(result.program.data.len(), 1);
        assert_eq!(result.program.data[0].label, "tape");
        assert_eq!(result.program.data[0].size, 30000);
        assert_eq!(result.program.data[0].init, 0);
    }

    // ── Single commands ───────────────────────────────────────────────────

    #[test]
    fn test_compile_increment() {
        let result = must_compile("+", release_config());
        assert!(count_opcode(&result.program, IrOp::LoadByte) >= 1);
        assert!(count_opcode(&result.program, IrOp::StoreByte) >= 1);
        assert!(count_opcode(&result.program, IrOp::AndImm) >= 1);
    }

    #[test]
    fn test_compile_increment_no_mask() {
        let mut config = release_config();
        config.mask_byte_arithmetic = false;
        let result = must_compile("+", config);
        assert_eq!(count_opcode(&result.program, IrOp::AndImm), 0);
    }

    #[test]
    fn test_compile_decrement() {
        let result = must_compile("-", release_config());
        // DEC should have ADD_IMM with value -1
        let found = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::AddImm
                && i.operands.len() >= 3
                && i.operands[2] == IrOperand::Immediate(-1)
        });
        assert!(found, "expected ADD_IMM with -1 for DEC");
    }

    #[test]
    fn test_compile_right() {
        let result = must_compile(">", release_config());
        // RIGHT produces: ADD_IMM v1, v1, 1
        let found = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::AddImm
                && i.operands.len() >= 3
                && i.operands[0] == IrOperand::Register(REG_TAPE_PTR)
                && i.operands[2] == IrOperand::Immediate(1)
        });
        assert!(found, "expected ADD_IMM v1, v1, 1 for RIGHT");
    }

    #[test]
    fn test_compile_left() {
        let result = must_compile("<", release_config());
        // LEFT produces: ADD_IMM v1, v1, -1
        let found = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::AddImm
                && i.operands.len() >= 3
                && i.operands[0] == IrOperand::Register(REG_TAPE_PTR)
                && i.operands[2] == IrOperand::Immediate(-1)
        });
        assert!(found, "expected ADD_IMM v1, v1, -1 for LEFT");
    }

    #[test]
    fn test_compile_output() {
        let result = must_compile(".", release_config());
        let found_copy = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::AddImm
                && i.operands.len() == 3
                && i.operands[0] == IrOperand::Register(REG_SYS_ARG)
                && i.operands[1] == IrOperand::Register(REG_TEMP)
                && i.operands[2] == IrOperand::Immediate(0)
        });
        let found = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Syscall
                && i.operands.first() == Some(&IrOperand::Immediate(SYSCALL_WRITE))
        });
        assert!(found_copy, "expected ADD_IMM copy into syscall arg register");
        assert!(found, "expected SYSCALL 1 (write) for OUTPUT");
    }

    #[test]
    fn test_compile_input() {
        let result = must_compile(",", release_config());
        // INPUT: should have SYSCALL 2
        let found = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Syscall
                && i.operands.first() == Some(&IrOperand::Immediate(SYSCALL_READ))
        });
        assert!(found, "expected SYSCALL 2 (read) for INPUT");
    }

    // ── Loop compilation ──────────────────────────────────────────────────

    #[test]
    fn test_compile_simple_loop() {
        let result = must_compile("[-]", release_config());
        assert!(has_label(&result.program, "loop_0_start"));
        assert!(has_label(&result.program, "loop_0_end"));
        assert!(count_opcode(&result.program, IrOp::BranchZ) >= 1);
        assert!(count_opcode(&result.program, IrOp::Jump) >= 1);
    }

    #[test]
    fn test_compile_nested_loops() {
        let result = must_compile("[>[+<-]]", release_config());
        assert!(has_label(&result.program, "loop_0_start"));
        assert!(has_label(&result.program, "loop_1_start"));
    }

    #[test]
    fn test_compile_empty_loop() {
        let result = must_compile("[]", release_config());
        assert!(has_label(&result.program, "loop_0_start"));
        assert!(has_label(&result.program, "loop_0_end"));
    }

    // ── Bounds checking ───────────────────────────────────────────────────

    #[test]
    fn test_compile_bounds_checks_right() {
        let result = must_compile(">", debug_config());
        assert!(count_opcode(&result.program, IrOp::CmpGt) >= 1);
        assert!(count_opcode(&result.program, IrOp::BranchNz) >= 1);
        assert!(has_label(&result.program, "__trap_oob"));
    }

    #[test]
    fn test_compile_bounds_checks_left() {
        let result = must_compile("<", debug_config());
        assert!(count_opcode(&result.program, IrOp::CmpLt) >= 1);
    }

    #[test]
    fn test_compile_no_bounds_checks_release() {
        let result = must_compile("><", release_config());
        assert_eq!(count_opcode(&result.program, IrOp::CmpGt), 0);
        assert_eq!(count_opcode(&result.program, IrOp::CmpLt), 0);
        assert!(!has_label(&result.program, "__trap_oob"));
    }

    // ── Source map ────────────────────────────────────────────────────────

    #[test]
    fn test_source_map_basic() {
        let result = must_compile("+.", release_config());
        // Should have 2 SourceToAst entries (one for +, one for .)
        assert_eq!(
            result.source_map.source_to_ast.entries.len(), 2,
            "expected 2 SourceToAst entries"
        );
        assert_eq!(result.source_map.source_to_ast.entries[0].pos.column, 1);
        assert_eq!(result.source_map.source_to_ast.entries[1].pos.column, 2);
    }

    #[test]
    fn test_source_map_ast_to_ir() {
        let result = must_compile("+", release_config());
        // "+" produces LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE = 4 IR IDs
        assert_eq!(result.source_map.ast_to_ir.entries.len(), 1);
        assert_eq!(
            result.source_map.ast_to_ir.entries[0].ir_ids.len(), 4,
            "expected 4 IR IDs for '+'"
        );
    }

    #[test]
    fn test_source_map_filename() {
        let result = must_compile("+", release_config());
        for entry in &result.source_map.source_to_ast.entries {
            assert_eq!(entry.pos.file, "test.bf");
        }
    }

    #[test]
    fn test_source_map_loop_has_entry() {
        let result = must_compile("[-]", release_config());
        // Loop + command = at least 2 entries
        assert!(
            result.source_map.source_to_ast.entries.len() >= 2,
            "expected at least 2 SourceToAst entries"
        );
    }

    // ── IR printer integration ────────────────────────────────────────────

    #[test]
    fn test_compiled_ir_is_printable() {
        let result = must_compile("+.", release_config());
        let text = print_ir(&result.program);
        assert!(text.contains(".version 1"), "missing .version");
        assert!(text.contains(".data tape 30000 0"), "missing .data");
        assert!(text.contains(".entry _start"), "missing .entry");
        assert!(text.contains("LOAD_BYTE"), "missing LOAD_BYTE");
        assert!(text.contains("HALT"), "missing HALT");
    }

    #[test]
    fn test_compiled_ir_roundtrip() {
        let result = must_compile("++[-].", release_config());
        let text = print_ir(&result.program);
        let parsed = parse_ir(&text).expect("roundtrip parse failed");
        assert_eq!(
            parsed.instructions.len(),
            result.program.instructions.len(),
            "roundtrip instruction count mismatch"
        );
    }

    // ── Complex programs ──────────────────────────────────────────────────

    #[test]
    fn test_compile_hello_world_subset() {
        // ++ [>++++<-] >. — sets cell 0 to 8 and outputs
        let result = must_compile("++++++++[>+++++++++<-]>.", release_config());
        assert!(has_label(&result.program, "loop_0_start"));
        let found_output = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Syscall
                && i.operands.first() == Some(&IrOperand::Immediate(SYSCALL_WRITE))
        });
        assert!(found_output, "expected SYSCALL 1 (output)");
    }

    #[test]
    fn test_compile_cat_program() {
        let result = must_compile(",[.,]", release_config());
        let found_read = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Syscall
                && i.operands.first() == Some(&IrOperand::Immediate(SYSCALL_READ))
        });
        let found_write = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Syscall
                && i.operands.first() == Some(&IrOperand::Immediate(SYSCALL_WRITE))
        });
        assert!(found_read, "expected SYSCALL 2 (read)");
        assert!(found_write, "expected SYSCALL 1 (write)");
    }

    // ── Custom tape size ──────────────────────────────────────────────────

    #[test]
    fn test_custom_tape_size() {
        let mut config = release_config();
        config.tape_size = 1000;
        let result = must_compile("", config);
        assert_eq!(result.program.data[0].size, 1000);
    }

    // ── Instruction ID uniqueness ─────────────────────────────────────────

    #[test]
    fn test_instruction_ids_are_unique() {
        let result = must_compile("++[>+<-].", release_config());
        let mut seen = std::collections::HashSet::new();
        for instr in &result.program.instructions {
            if instr.id == -1 {
                continue; // labels have -1
            }
            assert!(
                seen.insert(instr.id),
                "duplicate instruction ID: {}",
                instr.id
            );
        }
    }

    // ── Error cases ───────────────────────────────────────────────────────

    #[test]
    fn test_compile_invalid_ast_root() {
        use parser::grammar_parser::GrammarASTNode;
        let ast = GrammarASTNode {
            rule_name: "not_a_program".to_string(),
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        let result = compile(&ast, "test.bf", release_config());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 'program'"));
    }

    #[test]
    fn test_compile_zero_tape_size() {
        let ast = parse_brainfuck("").unwrap();
        let mut config = release_config();
        config.tape_size = 0;
        let result = compile(&ast, "test.bf", config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("tape_size"));
    }
}

//! # dartmouth-basic-ir-compiler
//!
//! Lowers a Dartmouth BASIC AST (from `dartmouth-basic-parser`) into a
//! target-independent [`IrProgram`] ready for any backend (GE-225, WASM, JVM, CIL).
//!
//! ## Historical context
//!
//! Dartmouth BASIC was the world's first general-purpose language designed for
//! shared, time-sliced hardware.  In 1964, John Kemeny and Thomas Kurtz ran
//! the first BASIC programs on a GE-225 mainframe at Dartmouth College.
//! Students typed programs on Teletype terminals and received printed output
//! within seconds — a radical departure from overnight batch processing.
//!
//! The language was intentionally simple: 17 statement types, floating-point
//! only, 1-based arrays, and line numbers as the sole control-flow mechanism.
//! GOTO and GOSUB were not bugs — they were the entire control structure, and
//! they worked perfectly for programs that fit on a single teletype page.
//!
//! ## V1 scope
//!
//! This first version compiles the integer subset needed for meaningful programs:
//!
//! - `REM`           — no-op comment
//! - `LET`           — variable assignment (scalar only) with all arithmetic operators
//! - `PRINT`         — string literals and numeric expressions
//! - `GOTO`          — unconditional jump to a line number
//! - `IF … THEN`     — conditional jump; all six relational operators
//! - `FOR … TO [STEP]` — pre-test counted loop
//! - `NEXT`          — ends the innermost FOR loop
//! - `END` / `STOP`  — halt execution
//!
//! ## Virtual register layout
//!
//! Every BASIC variable gets a fixed virtual register.  The GE-225 backend
//! assigns each register a dedicated spill slot.  This fixed layout ensures
//! that GOTO targets and loop iterations always read/write the correct register:
//!
//! ```text
//! v0        — syscall argument (char code to print for SYSCALL 1)
//! v1–v26    — BASIC scalar variables A–Z  (v1=A, v2=B, …, v26=Z)
//! v27–v286  — BASIC two-character variables A0–Z9
//!             v27=A0, v28=A1, …, v36=A9, v37=B0, …, v286=Z9
//! v287+     — expression temporaries (fresh register per intermediate value)
//! ```
//!
//! ## Label conventions
//!
//! ```text
//! _start         — program entry point
//! _line_N        — BASIC line number N (GOTO/IF target)
//! _for_N_check   — top of FOR loop N's pre-test
//! _for_N_end     — label after FOR loop N's body (NEXT target)
//! _pnum_N_pos    — non-negative branch in print-number routine N
//! _pnum_N_sK     — skip-digit-K label in print-number routine N
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use dartmouth_basic_ir_compiler::compile_basic_source;
//!
//! let result = compile_basic_source("10 LET A = 5\n20 END\n").unwrap();
//! assert!(!result.program.instructions.is_empty());
//! assert_eq!(result.var_regs["A"], 1);   // A lives in v1
//! ```

use coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic;
use compiler_ir::{IdGenerator, IrInstruction, IrOp, IrOperand, IrProgram};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::HashMap;

// ===========================================================================
// GE-225 character codes
// ===========================================================================
//
// The GE-225 typewriter uses 6-bit character codes stored in the N register.
// These codes are specific to the GE-225 teletype, not ASCII.  They are
// given below in decimal (matches the octal values in the GE-225 manual):
//
//   '0'=0  '1'=1  '2'=2  '3'=3  '4'=4  '5'=5  '6'=6  '7'=7
//   '8'=8  '9'=9  '/'=11 'A'=17 'B'=18 'C'=19 'D'=20 'E'=21
//   'F'=22 'G'=23 'H'=24 'I'=25  '-'=27  '.'=32
//   'J'=33 'K'=34 'L'=35 'M'=36 'N'=37 'O'=38 'P'=39 'Q'=40
//   'R'=41  '$'=43  ' '=48 'S'=50 'T'=51 'U'=52 'V'=53 'W'=54
//   'X'=55 'Y'=56 'Z'=57
//   Carriage return = 0o37 = 31

/// Carriage-return code for the GE-225 typewriter (octal 037 = decimal 31).
const GE225_CARRIAGE_RETURN: i64 = 0o37;

/// GE-225 minus-sign code (octal 033 = decimal 27).
const GE225_MINUS_CODE: i64 = 0o33;

/// Look up the GE-225 typewriter code for a single ASCII character.
///
/// Uppercase letters, digits, space, and basic punctuation are supported.
/// Lowercase letters are silently uppercased.
/// Returns `None` for characters with no GE-225 equivalent.
///
/// # Example
/// ```
/// use dartmouth_basic_ir_compiler::ascii_to_ge225;
/// assert_eq!(ascii_to_ge225('H'), Some(0o30));   // 24
/// assert_eq!(ascii_to_ge225('h'), Some(0o30));   // lowercase ok
/// assert_eq!(ascii_to_ge225('@'), None);          // no GE-225 code
/// ```
pub fn ascii_to_ge225(ch: char) -> Option<i64> {
    match ch.to_ascii_uppercase() {
        '0' => Some(0o00), '1' => Some(0o01), '2' => Some(0o02),
        '3' => Some(0o03), '4' => Some(0o04), '5' => Some(0o05),
        '6' => Some(0o06), '7' => Some(0o07), '8' => Some(0o10),
        '9' => Some(0o11), '/' => Some(0o13),
        'A' => Some(0o21), 'B' => Some(0o22), 'C' => Some(0o23),
        'D' => Some(0o24), 'E' => Some(0o25), 'F' => Some(0o26),
        'G' => Some(0o27), 'H' => Some(0o30), 'I' => Some(0o31),
        '-' => Some(0o33), '.' => Some(0o40),
        'J' => Some(0o41), 'K' => Some(0o42), 'L' => Some(0o43),
        'M' => Some(0o44), 'N' => Some(0o45), 'O' => Some(0o46),
        'P' => Some(0o47), 'Q' => Some(0o50), 'R' => Some(0o51),
        '$' => Some(0o53), ' ' => Some(0o60),
        'S' => Some(0o62), 'T' => Some(0o63), 'U' => Some(0o64),
        'V' => Some(0o65), 'W' => Some(0o66), 'X' => Some(0o67),
        'Y' => Some(0o70), 'Z' => Some(0o71),
        _ => None,
    }
}

// ===========================================================================
// Fixed virtual register indices
// ===========================================================================
//
// BASIC variables map to a fixed set of virtual registers.  This design means
// that GOTO-based control flow always finds the right register — there is no
// "live range" problem because every register is live for the entire program.

/// v0 — syscall argument (loaded with a char code before every SYSCALL 1).
const REG_SYSCALL_ARG: usize = 0;

/// v1 is the base index for single-letter BASIC variables A–Z.
/// A→v1, B→v2, …, Z→v26.
const VAR_BASE: usize = 1;

/// v27 is the base for two-character variables A0–Z9.
/// A0→v27, A1→v28, …, A9→v36, B0→v37, …, Z9→v286.
const VAR_LETTER_DIGIT_BASE: usize = 27;

/// v287+ are expression temporaries — fresh register per sub-expression.
const TEMP_BASE: usize = 287;

/// Map a BASIC scalar variable name to its fixed virtual register index.
///
/// Single-letter names (A–Z) map to v1–v26.
/// Two-character names (A0–Z9) map to v27–v286.
///
/// # Examples
/// ```
/// use dartmouth_basic_ir_compiler::scalar_reg;
/// assert_eq!(scalar_reg("A"), 1);
/// assert_eq!(scalar_reg("Z"), 26);
/// assert_eq!(scalar_reg("A0"), 27);
/// assert_eq!(scalar_reg("Z9"), 286);
/// ```
pub fn scalar_reg(name: &str) -> usize {
    // Guard against empty or non-alphabetic input: the first character must be
    // A-Z (or a-z, which we normalise to uppercase).  An empty slice or a
    // name whose first character is not a letter would cause an index-out-of-
    // bounds panic at `upper[0]`, so we reject those inputs explicitly.
    let upper: Vec<char> = name.to_ascii_uppercase().chars().collect();
    assert!(
        !upper.is_empty() && upper[0].is_ascii_alphabetic(),
        "scalar_reg: name must start with A-Z, got {:?}",
        name
    );
    if upper.len() == 1 {
        // A–Z → v1–v26
        VAR_BASE + (upper[0] as usize - 'A' as usize)
    } else {
        // A0–Z9 → v27–v286
        let letter_idx = upper[0] as usize - 'A' as usize;
        let digit_idx  = upper[1].to_digit(10).unwrap_or(0) as usize;
        VAR_LETTER_DIGIT_BASE + letter_idx * 10 + digit_idx
    }
}

/// Syscall number for "print the character in v0".
const SYSCALL_PRINT_CHAR: i64 = 1;

// ===========================================================================
// Public result and error types
// ===========================================================================

/// The outputs of a successful Dartmouth BASIC compilation.
///
/// # Example
/// ```rust
/// use dartmouth_basic_ir_compiler::compile_basic_source;
///
/// let result = compile_basic_source("10 LET A = 5\n20 END\n").unwrap();
/// assert!(!result.program.instructions.is_empty());
/// // Variable A lives in v1
/// assert_eq!(result.var_regs["A"], 1);
/// ```
#[derive(Debug)]
pub struct CompileResult {
    /// The compiled IR program containing all instructions.
    pub program: IrProgram,
    /// Maps BASIC variable names (e.g. `"A"`, `"B3"`) to their virtual
    /// register indices.  After execution, the value of BASIC variable A
    /// is the value stored in the memory slot for register `var_regs["A"]`.
    pub var_regs: HashMap<String, usize>,
}

/// Error returned when BASIC compilation fails.
///
/// # Fields
/// - `message` — human-readable description of what went wrong
///
/// # Examples
///
/// ```rust
/// use dartmouth_basic_ir_compiler::compile_basic_source;
///
/// // GOSUB is not supported in V1
/// let err = compile_basic_source("10 GOSUB 100\n20 END\n");
/// assert!(err.is_err());
/// ```
#[derive(Debug, Clone)]
pub struct CompileError {
    /// Human-readable description of the error.
    pub message: String,
}

impl CompileError {
    fn new(msg: impl Into<String>) -> Self {
        CompileError { message: msg.into() }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompileError: {}", self.message)
    }
}

impl std::error::Error for CompileError {}

// ===========================================================================
// Character encoding enum
// ===========================================================================

/// How PRINT statement character codes are emitted.
///
/// - `Ge225`  — emit GE-225 6-bit typewriter codes (for the GE-225 backend)
/// - `Ascii`  — emit standard ASCII byte values (for WASM/WASI `fd_write`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharEncoding {
    /// GE-225 6-bit typewriter codes.  Digits 0–9 have codes 0–9.
    Ge225,
    /// Standard ASCII.  Digit '0' has code 48, '-' has code 45.
    Ascii,
}

// ===========================================================================
// Public entry points
// ===========================================================================

/// Compile Dartmouth BASIC source text to an [`IrProgram`].
///
/// This is the simplest entry point.  It parses the source, compiles with
/// GE-225 character encoding and 32-bit integer width, and returns the
/// result.
///
/// # Arguments
///
/// - `source` — Dartmouth BASIC source text, e.g. `"10 LET A = 5\n20 END\n"`
///
/// # Returns
///
/// A [`CompileResult`] containing the IR program and variable register map.
///
/// # Errors
///
/// Returns a [`CompileError`] if the program uses an unsupported V1 feature
/// (GOSUB, DIM, INPUT, DEF FN, exponentiation, NEXT without FOR, etc.).
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ir_compiler::compile_basic_source;
///
/// let result = compile_basic_source("10 LET A = 5\n20 END\n").unwrap();
/// // The IR program starts with a _start label
/// assert!(!result.program.instructions.is_empty());
/// ```
pub fn compile_basic_source(source: &str) -> Result<CompileResult, CompileError> {
    compile_basic_with_options(source, CharEncoding::Ge225, 32)
}

/// Compile Dartmouth BASIC source text with explicit options.
///
/// # Arguments
///
/// - `source`        — BASIC source text
/// - `char_encoding` — character encoding for PRINT statements
/// - `int_bits`      — signed integer width in bits (controls digit count for PRINT of numbers)
///
/// # Errors
///
/// Returns [`CompileError`] for unsupported features or malformed AST nodes.
///
/// # Example
///
/// ```rust
/// use dartmouth_basic_ir_compiler::{compile_basic_with_options, CharEncoding};
///
/// // Compile for a 20-bit target (GE-225 has 20-bit words)
/// let result = compile_basic_with_options(
///     "10 PRINT 42\n20 END\n",
///     CharEncoding::Ge225,
///     20,
/// ).unwrap();
/// assert!(!result.program.instructions.is_empty());
/// ```
pub fn compile_basic_with_options(
    source: &str,
    char_encoding: CharEncoding,
    int_bits: u32,
) -> Result<CompileResult, CompileError> {
    if int_bits < 2 {
        return Err(CompileError::new(format!(
            "int_bits must be >= 2; got {int_bits}"
        )));
    }
    // i64 supports a maximum shift of 63 bits.  Allowing int_bits > 63 would
    // cause `1i64 << (int_bits - 1)` to overflow (panic in debug, UB in
    // release) inside `emit_print_number`.  Reject it here before we even
    // construct the compiler so the caller gets a clear, actionable error
    // rather than a panic or a garbled program.
    if int_bits > 63 {
        return Err(CompileError::new(format!(
            "int_bits must be <= 63 for i64 arithmetic; got {int_bits}"
        )));
    }
    let ast = parse_dartmouth_basic(source);
    let compiler = Compiler::new(char_encoding, int_bits);
    compiler.compile(&ast)
}

// ===========================================================================
// FOR-loop stack record
// ===========================================================================

/// State kept on the for-loop stack while compiling a FOR statement.
///
/// Pushed when FOR is encountered; popped when the matching NEXT is compiled.
///
/// ```text
/// _for_N_check:          ← check_label (jump back here from NEXT)
///     CMP_GT v_cmp, v_var, v_limit
///     BRANCH_NZ v_cmp, _for_N_end
///     <loop body>
/// NEXT:
///     ADD v_var, v_var, v_step    ← increment
///     JUMP _for_N_check           ← back-edge
/// _for_N_end:             ← end_label (BRANCH_NZ target)
/// ```
struct ForRecord {
    /// The BASIC variable name of the loop counter (e.g. `"I"`).
    var_name: String,
    /// Virtual register index of the loop counter.
    var_reg: usize,
    /// Virtual register holding the upper limit (TO expression result).
    /// Stored for future use (e.g. negative-step backward iteration in V2).
    #[allow(dead_code)]
    limit_reg: usize,
    /// Virtual register holding the step value (STEP expression, or 1).
    step_reg: usize,
    /// Label at the top of the pre-test block.
    check_label: String,
    /// Label placed after the loop body (BRANCH_NZ target when var > limit).
    end_label: String,
}

// ===========================================================================
// Internal compiler
// ===========================================================================

struct Compiler {
    program:     IrProgram,
    id_gen:      IdGenerator,
    next_reg:    usize,
    loop_count:  usize,
    label_count: usize,
    for_stack:   Vec<ForRecord>,
    char_encoding: CharEncoding,
    int_bits:    u32,
}

impl Compiler {
    fn new(char_encoding: CharEncoding, int_bits: u32) -> Self {
        Compiler {
            program:     IrProgram::new("_start"),
            id_gen:      IdGenerator::new(),
            next_reg:    TEMP_BASE,
            loop_count:  0,
            label_count: 0,
            for_stack:   Vec::new(),
            char_encoding,
            int_bits,
        }
    }

    // ── Top-level compilation ──────────────────────────────────────────────

    fn compile(mut self, ast: &GrammarASTNode) -> Result<CompileResult, CompileError> {
        // Emit the program entry label first.
        self.emit_label("_start");

        // Walk the program's children: each "line" node is one BASIC line.
        for child in &ast.children {
            if let ASTNodeOrToken::Node(line_node) = child {
                if line_node.rule_name == "line" {
                    self.compile_line(line_node)?;
                }
            }
        }

        // Epilogue: if the program falls through without an END statement,
        // halt execution cleanly.
        self.emit(IrOp::Halt, vec![]);

        // Build the variable register map.  Every single-letter variable
        // A–Z gets an entry; two-character variables are registered on
        // demand by the compiler but we expose the A-Z set for convenience.
        let mut var_regs = HashMap::new();
        for i in 0..26usize {
            let name = String::from(char::from(b'A' + i as u8));
            var_regs.insert(name, VAR_BASE + i);
        }

        Ok(CompileResult { program: self.program, var_regs })
    }

    // ── Line compilation ──────────────────────────────────────────────────

    /// Compile one numbered BASIC line.
    ///
    /// Emits a `LABEL _line_N` instruction for the line number, then
    /// dispatches the statement inside the line.
    fn compile_line(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        let mut line_num: Option<i64> = None;
        let mut stmt_node: Option<&GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "LINE_NUM" => {
                    line_num = tok.value.parse().ok();
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                    stmt_node = Some(n);
                }
                _ => {}
            }
        }

        if let Some(n) = line_num {
            self.emit_label(&format!("_line_{n}"));
        }
        if let Some(stmt) = stmt_node {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    // ── Statement dispatch ─────────────────────────────────────────────────

    /// Dispatch to the appropriate handler based on the concrete statement rule.
    fn compile_statement(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        for child in &node.children {
            if let ASTNodeOrToken::Node(inner) = child {
                match inner.rule_name.as_str() {
                    "rem_stmt"    => { self.compile_rem(inner)?;  return Ok(()); }
                    "let_stmt"    => { self.compile_let(inner)?;  return Ok(()); }
                    "print_stmt"  => { self.compile_print(inner)?; return Ok(()); }
                    "goto_stmt"   => { self.compile_goto(inner)?; return Ok(()); }
                    "if_stmt"     => { self.compile_if(inner)?;   return Ok(()); }
                    "for_stmt"    => { self.compile_for(inner)?;  return Ok(()); }
                    "next_stmt"   => { self.compile_next(inner)?; return Ok(()); }
                    "end_stmt" | "stop_stmt" => {
                        self.emit(IrOp::Halt, vec![]);
                        return Ok(());
                    }
                    r @ ("gosub_stmt" | "return_stmt" | "dim_stmt"
                        | "def_stmt" | "input_stmt" | "read_stmt"
                        | "data_stmt" | "restore_stmt") => {
                        let keyword = r.trim_end_matches("_stmt").to_uppercase();
                        return Err(CompileError::new(format!(
                            "'{keyword}' is not supported in V1 of the compiled pipeline"
                        )));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    // ── REM ───────────────────────────────────────────────────────────────

    /// Compile a REM statement into a COMMENT instruction.
    ///
    /// COMMENT produces no machine code on any backend.  It is emitted purely
    /// for human-readable IR output and source-level debugging.
    fn compile_rem(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        // Collect remark text from non-keyword token children.
        let mut parts: Vec<String> = Vec::new();
        for child in &node.children {
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.effective_type_name() != "KEYWORD" {
                    parts.push(tok.value.clone());
                }
            }
        }
        let text = if parts.is_empty() {
            "REM".to_string()
        } else {
            parts.join(" ").trim().to_string()
        };
        self.emit(IrOp::Comment, vec![IrOperand::Label(text)]);
        Ok(())
    }

    // ── LET ───────────────────────────────────────────────────────────────

    /// Compile `LET var = expr` — evaluate the expression into a fresh register,
    /// then copy it into the variable's fixed register via `ADD_IMM v_var, v_val, 0`.
    fn compile_let(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        let mut var_node:  Option<&GrammarASTNode> = None;
        let mut expr_node: Option<&GrammarASTNode> = None;
        let mut seen_eq = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "EQ" => {
                    seen_eq = true;
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "variable" && !seen_eq {
                        var_node = Some(n);
                    } else if seen_eq && expr_node.is_none() {
                        expr_node = Some(n);
                    }
                }
                _ => {}
            }
        }

        let var_node  = var_node.ok_or_else(|| CompileError::new("malformed LET: no variable"))?;
        let expr_node = expr_node.ok_or_else(|| CompileError::new("malformed LET: no expression"))?;

        let var_name = self.extract_var_name(var_node)
            .ok_or_else(|| CompileError::new("malformed LET: could not extract variable name"))?;
        let v_var = scalar_reg(&var_name);
        let v_val = self.compile_expr(expr_node)?;

        // Copy expression result into the variable's register.
        // ADD_IMM v_var, v_val, 0  →  v_var = v_val + 0 = v_val
        self.emit(IrOp::AddImm, vec![
            IrOperand::Register(v_var),
            IrOperand::Register(v_val),
            IrOperand::Immediate(0),
        ]);
        Ok(())
    }

    // ── PRINT ─────────────────────────────────────────────────────────────

    /// Compile a PRINT statement.
    ///
    /// Supports:
    /// - String literals (each character → LOAD_IMM + SYSCALL 1)
    /// - Numeric expressions (decimal digit-extraction loop)
    /// - Comma-separated mixtures
    /// - Bare PRINT (just a carriage return)
    ///
    /// A carriage-return code is always appended at the end, matching the
    /// original DTSS terminal behaviour.
    fn compile_print(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        // Find the print_list child (may be absent for bare PRINT).
        let print_list = node.children.iter().find_map(|c| {
            if let ASTNodeOrToken::Node(n) = c {
                if n.rule_name == "print_list" { Some(n) } else { None }
            } else {
                None
            }
        });

        // Carriage-return code: 0o37 for GE-225, '\n' (10) for ASCII.
        let cr: i64 = match self.char_encoding {
            CharEncoding::Ge225  => GE225_CARRIAGE_RETURN,
            CharEncoding::Ascii  => '\n' as i64,
        };

        match print_list {
            None => {
                // Bare PRINT → just emit a carriage return.
                self.emit_print_code(cr);
            }
            Some(pl) => {
                // Walk each print_item (print_sep/comma nodes are skipped).
                for item in &pl.children {
                    if let ASTNodeOrToken::Node(n) = item {
                        if n.rule_name == "print_item" {
                            self.compile_print_item(n)?;
                        }
                    }
                }
                self.emit_print_code(cr);
            }
        }
        Ok(())
    }

    /// Compile one item from a PRINT list: a string literal or a numeric
    /// expression.
    fn compile_print_item(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "STRING" => {
                    // Strip surrounding quotes from the string literal.
                    let text = tok.value.trim_matches('"').trim_matches('\'');
                    for ch in text.chars() {
                        let code = match self.char_encoding {
                            CharEncoding::Ascii => ch as i64,
                            CharEncoding::Ge225 => {
                                ascii_to_ge225(ch).ok_or_else(|| {
                                    CompileError::new(format!(
                                        "character {:?} has no GE-225 typewriter equivalent; \
                                         V1 supports A-Z, 0-9, space, and basic punctuation",
                                        ch
                                    ))
                                })?
                            }
                        };
                        self.emit_print_code(code);
                    }
                    return Ok(());
                }
                ASTNodeOrToken::Node(n) => {
                    // Numeric expression: compile to register, emit digit sequence.
                    let v_val = self.compile_expr(n)?;
                    self.emit_print_number(v_val);
                    return Ok(());
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Emit `LOAD_IMM v0, code; SYSCALL 1, v0` to print one character.
    fn emit_print_code(&mut self, code: i64) {
        self.emit(IrOp::LoadImm, vec![
            IrOperand::Register(REG_SYSCALL_ARG),
            IrOperand::Immediate(code),
        ]);
        self.emit(IrOp::Syscall, vec![
            IrOperand::Immediate(SYSCALL_PRINT_CHAR),
            IrOperand::Register(REG_SYSCALL_ARG),
        ]);
    }

    /// Emit an unrolled decimal digit-extraction routine that prints the
    /// signed integer stored in `v_val`.
    ///
    /// Algorithm:
    ///
    /// 1. Copy v_val into a scratch register `r_work`.
    /// 2. If `r_work < 0`: print '-' then negate (`r_work = 0 − r_work`).
    /// 3. For each power of ten above the units place (unrolled):
    ///    a. `r_dig = r_work / power`
    ///    b. `r_work = r_work − r_dig * power`   (= r_work mod power)
    ///    c. If `r_dig == 0 AND not_yet_started`: skip (leading-zero suppression).
    ///    d. Else: print digit; mark started.
    /// 4. Print `r_work` (units digit, handles value 0 correctly).
    ///
    /// The number of digit positions is `len(str(2^(int_bits−1) − 1))`:
    /// - `int_bits=32` → 2,147,483,647 → 10 digits → 9 powers
    /// - `int_bits=20` → 524,287       →  6 digits → 5 powers
    ///
    /// Every power-of-ten constant emitted as `LOAD_IMM` must fit in the
    /// target machine's signed word — this is why `int_bits` matters.
    fn emit_print_number(&mut self, v_val: usize) {
        let label_id = self.label_count;
        self.label_count += 1;

        // Allocate all scratch registers up front.
        let r_work    = self.new_reg();  // working copy of the magnitude
        let r_zero    = self.new_reg();  // constant 0
        let r_one     = self.new_reg();  // constant 1
        let r_started = self.new_reg();  // 1 once the first non-zero digit is printed
        let r_dig     = self.new_reg();  // current digit (reused per position)
        let r_mttmp   = self.new_reg();  // temp: r_dig * power (for mod computation)
        let r_pow     = self.new_reg();  // power of ten (reused per position)
        let r_is_neg  = self.new_reg();  // 1 if the original value was negative

        // Initialise scratch registers.
        self.emit(IrOp::AddImm,  vec![IrOperand::Register(r_work),    IrOperand::Register(v_val), IrOperand::Immediate(0)]);
        self.emit(IrOp::LoadImm, vec![IrOperand::Register(r_zero),    IrOperand::Immediate(0)]);
        self.emit(IrOp::LoadImm, vec![IrOperand::Register(r_one),     IrOperand::Immediate(1)]);
        self.emit(IrOp::LoadImm, vec![IrOperand::Register(r_started), IrOperand::Immediate(0)]);

        // Sign handling: if r_work < 0, print '-' then negate.
        let minus_code: i64 = match self.char_encoding {
            CharEncoding::Ascii => '-' as i64,
            CharEncoding::Ge225 => GE225_MINUS_CODE,
        };
        let l_pos = format!("_pnum_{label_id}_pos");
        self.emit(IrOp::CmpLt, vec![
            IrOperand::Register(r_is_neg),
            IrOperand::Register(r_work),
            IrOperand::Register(r_zero),
        ]);
        self.emit(IrOp::BranchZ, vec![
            IrOperand::Register(r_is_neg),
            IrOperand::Label(l_pos.clone()),
        ]);
        // Print '-'
        self.emit_print_code(minus_code);
        // Negate: r_work = 0 − r_work
        let r_neg = self.new_reg();
        self.emit(IrOp::Sub, vec![
            IrOperand::Register(r_neg),
            IrOperand::Register(r_zero),
            IrOperand::Register(r_work),
        ]);
        self.emit(IrOp::AddImm, vec![
            IrOperand::Register(r_work),
            IrOperand::Register(r_neg),
            IrOperand::Immediate(0),
        ]);
        self.emit_label(&l_pos);

        // For ASCII, digits need to be offset by 48 to reach '0'-'9'.
        // For GE-225, typewriter codes 0–9 already map to '0'–'9'.
        let digit_offset: i64 = match self.char_encoding {
            CharEncoding::Ascii => 48,
            CharEncoding::Ge225 => 0,
        };

        // Compute the power-of-ten positions from int_bits:
        //   max_value  = 2^(int_bits-1) − 1
        //   digit_count = number of decimal digits in max_value
        //   powers = [10^(digit_count-1), …, 10]
        let max_val: i64 = (1i64 << (self.int_bits - 1)) - 1;
        let digit_count = max_val.to_string().len();
        let powers: Vec<i64> = (1..digit_count)
            .rev()
            .map(|i| 10i64.pow(i as u32))
            .collect();

        // Unrolled digit extraction: one iteration per power of ten.
        for (pos_idx, power) in powers.iter().enumerate() {
            let l_skip = format!("_pnum_{label_id}_s{pos_idx}");

            // Extract digit and remainder.
            //   r_dig   = r_work / power
            //   r_mttmp = r_dig * power
            //   r_work  = r_work − r_mttmp   (= r_work mod power)
            self.emit(IrOp::LoadImm, vec![IrOperand::Register(r_pow), IrOperand::Immediate(*power)]);
            self.emit(IrOp::Div,     vec![IrOperand::Register(r_dig), IrOperand::Register(r_work), IrOperand::Register(r_pow)]);
            self.emit(IrOp::Mul,     vec![IrOperand::Register(r_mttmp), IrOperand::Register(r_dig), IrOperand::Register(r_pow)]);
            self.emit(IrOp::Sub,     vec![IrOperand::Register(r_work), IrOperand::Register(r_work), IrOperand::Register(r_mttmp)]);

            // Leading-zero suppression: skip if digit == 0 AND not yet started.
            // Trick: r_dig + r_started == 0 iff both are zero.
            let r_sum = self.new_reg();
            self.emit(IrOp::Add, vec![
                IrOperand::Register(r_sum),
                IrOperand::Register(r_dig),
                IrOperand::Register(r_started),
            ]);
            self.emit(IrOp::BranchZ, vec![
                IrOperand::Register(r_sum),
                IrOperand::Label(l_skip.clone()),
            ]);

            // Print this digit (add digit_offset to convert to printable code).
            self.emit(IrOp::AddImm, vec![
                IrOperand::Register(REG_SYSCALL_ARG),
                IrOperand::Register(r_dig),
                IrOperand::Immediate(digit_offset),
            ]);
            self.emit(IrOp::Syscall, vec![
                IrOperand::Immediate(SYSCALL_PRINT_CHAR),
                IrOperand::Register(REG_SYSCALL_ARG),
            ]);
            // Mark started = 1 (copy r_one into r_started).
            self.emit(IrOp::AddImm, vec![
                IrOperand::Register(r_started),
                IrOperand::Register(r_one),
                IrOperand::Immediate(0),
            ]);
            self.emit_label(&l_skip);
        }

        // Units digit: always print (correctly handles the value 0).
        self.emit(IrOp::AddImm, vec![
            IrOperand::Register(REG_SYSCALL_ARG),
            IrOperand::Register(r_work),
            IrOperand::Immediate(digit_offset),
        ]);
        self.emit(IrOp::Syscall, vec![
            IrOperand::Immediate(SYSCALL_PRINT_CHAR),
            IrOperand::Register(REG_SYSCALL_ARG),
        ]);
    }

    // ── GOTO ──────────────────────────────────────────────────────────────

    /// Compile `GOTO lineno` — emit an unconditional JUMP to `_line_N`.
    fn compile_goto(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        let lineno = self.extract_lineno(node)?;
        self.emit(IrOp::Jump, vec![
            IrOperand::Label(format!("_line_{lineno}"))
        ]);
        Ok(())
    }

    // ── IF … THEN ─────────────────────────────────────────────────────────

    /// Compile `IF expr relop expr THEN lineno`.
    ///
    /// All six relational operators are supported:
    ///
    /// | Operator | IR opcodes emitted             |
    /// |----------|-------------------------------|
    /// | `<`      | CMP_LT + BRANCH_NZ            |
    /// | `>`      | CMP_GT + BRANCH_NZ            |
    /// | `=`      | CMP_EQ + BRANCH_NZ            |
    /// | `<>`     | CMP_NE + BRANCH_NZ            |
    /// | `<=`     | CMP_GT + NOT-bool + BRANCH_NZ |
    /// | `>=`     | CMP_LT + NOT-bool + BRANCH_NZ |
    ///
    /// The NOT-bool idiom:  `v_out = (v_in + (-1)) & 1`
    ///
    /// When v_in = 1: `(1-1) & 1 = 0`
    /// When v_in = 0: `(0-1) & 1 = (-1 in two's complement) & 1 = 1`
    fn compile_if(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        let mut exprs:       Vec<&GrammarASTNode> = Vec::new();
        let mut relop_value: Option<String>        = None;
        let mut lineno:      Option<i64>           = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "NUMBER" => {
                    lineno = tok.value.parse().ok();
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "relop" {
                        // Extract the operator token from the relop node.
                        relop_value = n.children.iter().find_map(|c| {
                            if let ASTNodeOrToken::Token(t) = c { Some(t.value.clone()) } else { None }
                        });
                    } else if matches!(n.rule_name.as_str(), "expr" | "term" | "power" | "unary" | "primary") {
                        exprs.push(n);
                    }
                }
                _ => {}
            }
        }

        let lineno = lineno.ok_or_else(|| CompileError::new("malformed IF: no target line number"))?;
        let relop  = relop_value.ok_or_else(|| CompileError::new("malformed IF: no relational operator"))?;
        if exprs.len() < 2 {
            return Err(CompileError::new("malformed IF: fewer than 2 expression operands"));
        }

        let v_lhs = self.compile_expr(exprs[0])?;
        let v_rhs = self.compile_expr(exprs[1])?;
        let v_cmp = self.new_reg();
        let label = format!("_line_{lineno}");

        match relop.as_str() {
            "<" => {
                self.emit(IrOp::CmpLt, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_cmp), IrOperand::Label(label)]);
            }
            ">" => {
                self.emit(IrOp::CmpGt, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_cmp), IrOperand::Label(label)]);
            }
            "=" => {
                self.emit(IrOp::CmpEq, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_cmp), IrOperand::Label(label)]);
            }
            "<>" => {
                self.emit(IrOp::CmpNe, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_cmp), IrOperand::Label(label)]);
            }
            "<=" => {
                // LE = NOT GT
                self.emit(IrOp::CmpGt, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                let v_flipped = self.not_bool(v_cmp);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_flipped), IrOperand::Label(label)]);
            }
            ">=" => {
                // GE = NOT LT
                self.emit(IrOp::CmpLt, vec![IrOperand::Register(v_cmp), IrOperand::Register(v_lhs), IrOperand::Register(v_rhs)]);
                let v_flipped = self.not_bool(v_cmp);
                self.emit(IrOp::BranchNz, vec![IrOperand::Register(v_flipped), IrOperand::Label(label)]);
            }
            other => {
                return Err(CompileError::new(format!("unknown relational operator: {other:?}")));
            }
        }
        Ok(())
    }

    /// Flip a boolean register: 0 → 1, 1 → 0.
    ///
    /// Implementation:  `v_out = (v_in + (-1)) & 1`
    ///
    /// ```text
    /// v_in = 1: (1 - 1) & 1 = 0 & 1 = 0  ✓
    /// v_in = 0: (0 - 1) & 1 = (-1) & 1
    ///           In two's complement -1 = 0xFF…FF; 0xFF…FF & 1 = 1  ✓
    /// ```
    fn not_bool(&mut self, v_in: usize) -> usize {
        let v_sub = self.new_reg();
        let v_out = self.new_reg();
        self.emit(IrOp::AddImm, vec![
            IrOperand::Register(v_sub),
            IrOperand::Register(v_in),
            IrOperand::Immediate(-1),
        ]);
        self.emit(IrOp::AndImm, vec![
            IrOperand::Register(v_out),
            IrOperand::Register(v_sub),
            IrOperand::Immediate(1),
        ]);
        v_out
    }

    // ── FOR … TO [STEP] ───────────────────────────────────────────────────

    /// Compile a FOR statement.
    ///
    /// Emits:
    /// 1. Evaluate start, limit, and step expressions.
    /// 2. `ADD_IMM v_var, v_start, 0` — initialise loop counter.
    /// 3. `_for_N_check:` label.
    /// 4. `CMP_GT v_cmp, v_var, v_limit; BRANCH_NZ v_cmp, _for_N_end` — pre-test.
    ///
    /// Pushes a `ForRecord` for the matching NEXT to pop.
    fn compile_for(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        let mut var_name: Option<String>            = None;
        let mut exprs:    Vec<&GrammarASTNode>      = Vec::new();
        let mut has_step = false;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "NAME" => {
                    var_name = Some(tok.value.to_ascii_uppercase());
                }
                ASTNodeOrToken::Token(tok)
                    if tok.effective_type_name() == "KEYWORD"
                       && tok.value.to_ascii_uppercase() == "STEP" =>
                {
                    has_step = true;
                }
                ASTNodeOrToken::Node(n)
                    if matches!(n.rule_name.as_str(), "expr" | "term" | "power" | "unary" | "primary") =>
                {
                    exprs.push(n);
                }
                _ => {}
            }
        }

        let var_name = var_name.ok_or_else(|| CompileError::new("malformed FOR: no variable"))?;
        if exprs.len() < 2 {
            return Err(CompileError::new("malformed FOR: expected at least start and limit expressions"));
        }

        let v_var   = scalar_reg(&var_name);
        let v_start = self.compile_expr(exprs[0])?;
        let v_limit = self.compile_expr(exprs[1])?;

        // Step: use STEP expression if provided, otherwise default to 1.
        let v_step = if has_step && exprs.len() >= 3 {
            self.compile_expr(exprs[2])?
        } else {
            let reg = self.new_reg();
            self.emit(IrOp::LoadImm, vec![
                IrOperand::Register(reg),
                IrOperand::Immediate(1),
            ]);
            reg
        };

        // Initialise loop variable: v_var = v_start.
        self.emit(IrOp::AddImm, vec![
            IrOperand::Register(v_var),
            IrOperand::Register(v_start),
            IrOperand::Immediate(0),
        ]);

        let loop_num    = self.loop_count;
        self.loop_count += 1;
        let check_label = format!("_for_{loop_num}_check");
        let end_label   = format!("_for_{loop_num}_end");

        self.emit_label(&check_label);

        // Pre-test: exit if v_var > v_limit.
        let v_cmp = self.new_reg();
        self.emit(IrOp::CmpGt, vec![
            IrOperand::Register(v_cmp),
            IrOperand::Register(v_var),
            IrOperand::Register(v_limit),
        ]);
        self.emit(IrOp::BranchNz, vec![
            IrOperand::Register(v_cmp),
            IrOperand::Label(end_label.clone()),
        ]);

        self.for_stack.push(ForRecord {
            var_name,
            var_reg:     v_var,
            limit_reg:   v_limit,
            step_reg:    v_step,
            check_label,
            end_label,
        });
        Ok(())
    }

    // ── NEXT ──────────────────────────────────────────────────────────────

    /// Compile a NEXT statement.
    ///
    /// Pops the innermost FOR record, verifies the variable name matches,
    /// emits the increment (`ADD v_var, v_var, v_step`), the backward jump
    /// (`JUMP _for_N_check`), and the loop end label (`_for_N_end:`).
    fn compile_next(&mut self, node: &GrammarASTNode) -> Result<(), CompileError> {
        if self.for_stack.is_empty() {
            return Err(CompileError::new("NEXT without matching FOR"));
        }

        // Extract the optional variable name from NEXT (e.g. `NEXT I`).
        let next_var: Option<String> = node.children.iter().find_map(|c| {
            if let ASTNodeOrToken::Token(tok) = c {
                if tok.effective_type_name() == "NAME" {
                    Some(tok.value.to_ascii_uppercase())
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Check that the variable matches (if given) before popping.
        {
            let rec = self.for_stack.last().unwrap();
            if let Some(ref nv) = next_var {
                if *nv != rec.var_name {
                    return Err(CompileError::new(format!(
                        "NEXT {nv} does not match innermost FOR {}",
                        rec.var_name
                    )));
                }
            }
        }

        let rec = self.for_stack.pop().unwrap();

        // Increment: v_var += v_step.
        self.emit(IrOp::Add, vec![
            IrOperand::Register(rec.var_reg),
            IrOperand::Register(rec.var_reg),
            IrOperand::Register(rec.step_reg),
        ]);

        // Jump back to pre-test.
        self.emit(IrOp::Jump, vec![
            IrOperand::Label(rec.check_label)
        ]);

        // End label: BRANCH_NZ target when counter exceeds limit.
        self.emit_label(&rec.end_label);
        Ok(())
    }

    // ── Expression compilation ─────────────────────────────────────────────

    /// Recursively compile an expression node and return its result register.
    ///
    /// The grammar's precedence tower:
    /// ```text
    /// expr   → term { (+ | -) term }
    /// term   → power { (* | /) power }
    /// power  → unary [^ unary]          (^ unsupported in V1)
    /// unary  → - unary | primary
    /// primary → variable | NUMBER | ( expr )
    /// ```
    fn compile_expr(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        match node.rule_name.as_str() {
            "primary"  => self.compile_primary(node),
            "unary"    => self.compile_unary(node),
            "expr" | "term" => self.compile_binop_chain(node),
            "power"    => self.compile_power(node),
            "variable" => self.compile_variable_expr(node),
            _ => {
                // Single-child pass-through for wrapper rules.
                let ast_children: Vec<&GrammarASTNode> = node.children.iter().filter_map(|c| {
                    if let ASTNodeOrToken::Node(n) = c { Some(n) } else { None }
                }).collect();
                if ast_children.len() == 1 {
                    self.compile_expr(ast_children[0])
                } else {
                    Err(CompileError::new(format!(
                        "unexpected expression node: {:?}", node.rule_name
                    )))
                }
            }
        }
    }

    /// Compile a `primary` node: number literal, variable, or `(expr)`.
    fn compile_primary(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "NUMBER" => {
                    let val: i64 = tok.value.parse::<f64>()
                        .map(|f| f as i64)
                        .map_err(|_| CompileError::new(format!("invalid number literal: {:?}", tok.value)))?;
                    let v = self.new_reg();
                    self.emit(IrOp::LoadImm, vec![
                        IrOperand::Register(v),
                        IrOperand::Immediate(val),
                    ]);
                    return Ok(v);
                }
                ASTNodeOrToken::Node(n) => {
                    if n.rule_name == "variable" {
                        return self.compile_variable_expr(n);
                    }
                    return self.compile_expr(n);
                }
                _ => {}
            }
        }
        Err(CompileError::new("empty primary expression"))
    }

    /// Compile a `variable` reference.
    ///
    /// Array element access (e.g. `A(I)`) is not supported in V1.
    fn compile_variable_expr(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        // Array access: detected by the presence of an LPAREN child.
        let has_paren = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "LPAREN")
        });
        if has_paren {
            return Err(CompileError::new("array element access is not supported in V1"));
        }
        let name = self.extract_var_name(node)
            .ok_or_else(|| CompileError::new("could not extract variable name from AST"))?;
        Ok(scalar_reg(&name))
    }

    /// Compile a `unary` node (optional leading minus).
    fn compile_unary(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        let mut has_minus = false;
        let mut inner:     Option<&GrammarASTNode> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "MINUS" => {
                    has_minus = true;
                }
                ASTNodeOrToken::Node(n) => {
                    inner = Some(n);
                }
                _ => {}
            }
        }

        let inner = inner.ok_or_else(|| CompileError::new("empty unary expression"))?;
        let v_inner = self.compile_expr(inner)?;

        if !has_minus {
            return Ok(v_inner);
        }

        // Negate: v_result = 0 − v_inner
        let v_zero   = self.new_reg();
        let v_result = self.new_reg();
        self.emit(IrOp::LoadImm, vec![IrOperand::Register(v_zero), IrOperand::Immediate(0)]);
        self.emit(IrOp::Sub, vec![
            IrOperand::Register(v_result),
            IrOperand::Register(v_zero),
            IrOperand::Register(v_inner),
        ]);
        Ok(v_result)
    }

    /// Compile a `power` node.
    ///
    /// The `^` operator is not supported in V1; if detected, a `CompileError`
    /// is returned.  Otherwise, the single unary child is compiled.
    fn compile_power(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        let has_caret = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "CARET")
        });
        if has_caret {
            return Err(CompileError::new("the ^ (power) operator is not supported in V1"));
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                return self.compile_expr(n);
            }
        }
        Err(CompileError::new("empty power expression"))
    }

    /// Compile a left-associative binary operator chain.
    ///
    /// Used for both `expr` (`+`/`-`) and `term` (`*`/`/`).
    /// Children are interleaved: `[operand, op_token, operand, op_token, …]`.
    fn compile_binop_chain(&mut self, node: &GrammarASTNode) -> Result<usize, CompileError> {
        let mut operands:  Vec<usize>  = Vec::new();
        let mut operators: Vec<String> = Vec::new();

        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok)
                    if matches!(
                        tok.effective_type_name(),
                        "PLUS" | "MINUS" | "STAR" | "SLASH"
                    ) =>
                {
                    operators.push(tok.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    operands.push(self.compile_expr(n)?);
                }
                _ => {}
            }
        }

        if operands.is_empty() {
            return Err(CompileError::new(format!(
                "empty {} expression", node.rule_name
            )));
        }

        let mut result = operands[0];
        for (i, op) in operators.iter().enumerate() {
            let rhs   = operands[i + 1];
            let v_out = self.new_reg();
            let opcode = match op.as_str() {
                "+" => IrOp::Add,
                "-" => IrOp::Sub,
                "*" => IrOp::Mul,
                "/" => IrOp::Div,
                other => return Err(CompileError::new(format!(
                    "unknown binary operator: {other:?}"
                ))),
            };
            self.emit(opcode, vec![
                IrOperand::Register(v_out),
                IrOperand::Register(result),
                IrOperand::Register(rhs),
            ]);
            result = v_out;
        }
        Ok(result)
    }

    // ── Helper utilities ──────────────────────────────────────────────────

    /// Allocate and return the next fresh expression-temporary register.
    fn new_reg(&mut self) -> usize {
        let r = self.next_reg;
        self.next_reg += 1;
        r
    }

    /// Append one instruction to the program with a fresh unique ID.
    fn emit(&mut self, opcode: IrOp, operands: Vec<IrOperand>) {
        let id = self.id_gen.next();
        self.program.add_instruction(IrInstruction::new(opcode, operands, id));
    }

    /// Append a LABEL pseudo-instruction (labels use ID = -1).
    fn emit_label(&mut self, name: &str) {
        self.program.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label(name.to_string())],
            -1,
        ));
    }

    /// Extract the variable name string from a `variable` AST node.
    fn extract_var_name(&self, node: &GrammarASTNode) -> Option<String> {
        for child in &node.children {
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.effective_type_name() == "NAME" {
                    return Some(tok.value.to_ascii_uppercase());
                }
            }
        }
        None
    }

    /// Extract an integer line number from a statement node.
    fn extract_lineno(&self, node: &GrammarASTNode) -> Result<i64, CompileError> {
        for child in &node.children {
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.effective_type_name() == "NUMBER" {
                    return tok.value.parse()
                        .map_err(|_| CompileError::new(format!(
                            "invalid line number: {:?}", tok.value
                        )));
                }
            }
        }
        Err(CompileError::new(format!(
            "could not find line number in {}", node.rule_name
        )))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compile a source snippet and return the result.
    fn compile(source: &str) -> CompileResult {
        compile_basic_source(source).expect("compilation should succeed")
    }

    // ── Basic compilation ────────────────────────────────────────────────

    #[test]
    fn test_empty_program_has_halt() {
        // A program that falls through without END should still HALT.
        let result = compile("10 REM EMPTY\n");
        assert!(result.program.instructions.iter().any(|i| i.opcode == IrOp::Halt),
                "program must end with HALT");
    }

    #[test]
    fn test_start_label_emitted() {
        let result = compile("10 END\n");
        assert!(result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Label
                && i.operands == vec![IrOperand::Label("_start".to_string())]
        }), "_start label must be emitted");
    }

    #[test]
    fn test_line_label_emitted() {
        let result = compile("10 END\n");
        assert!(result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Label
                && i.operands == vec![IrOperand::Label("_line_10".to_string())]
        }), "_line_10 label must be emitted");
    }

    #[test]
    fn test_var_regs_a_to_z() {
        let result = compile("10 END\n");
        // Spot-check a few variable registers.
        assert_eq!(result.var_regs["A"], 1);
        assert_eq!(result.var_regs["B"], 2);
        assert_eq!(result.var_regs["Z"], 26);
    }

    // ── scalar_reg ──────────────────────────────────────────────────────

    #[test]
    fn test_scalar_reg_single_letter() {
        assert_eq!(scalar_reg("A"), 1);
        assert_eq!(scalar_reg("Z"), 26);
        assert_eq!(scalar_reg("a"), 1); // lowercase OK
    }

    #[test]
    fn test_scalar_reg_two_char() {
        assert_eq!(scalar_reg("A0"), 27);
        assert_eq!(scalar_reg("A9"), 36);
        assert_eq!(scalar_reg("B0"), 37);
        assert_eq!(scalar_reg("Z9"), 286);
    }

    // ── ascii_to_ge225 ──────────────────────────────────────────────────

    #[test]
    fn test_ge225_digits() {
        for d in 0u8..=9 {
            assert_eq!(
                ascii_to_ge225(char::from(b'0' + d)),
                Some(d as i64),
                "digit {} should map to GE-225 code {}", d, d
            );
        }
    }

    #[test]
    fn test_ge225_uppercase_letters() {
        assert_eq!(ascii_to_ge225('A'), Some(0o21));
        assert_eq!(ascii_to_ge225('Z'), Some(0o71));
    }

    #[test]
    fn test_ge225_lowercase_uppercased() {
        assert_eq!(ascii_to_ge225('h'), Some(0o30)); // H
    }

    #[test]
    fn test_ge225_unsupported_char() {
        assert_eq!(ascii_to_ge225('@'), None);
        assert_eq!(ascii_to_ge225('!'), None);
    }

    // ── REM ─────────────────────────────────────────────────────────────

    #[test]
    fn test_rem_emits_comment() {
        let result = compile("10 REM THIS IS A REMARK\n20 END\n");
        let has_comment = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Comment
        });
        assert!(has_comment, "REM should emit a COMMENT instruction");
    }

    // ── LET ─────────────────────────────────────────────────────────────

    #[test]
    fn test_let_emits_load_imm_and_add_imm() {
        // LET A = 5 → LOAD_IMM vN, 5; ADD_IMM v1, vN, 0
        let result = compile("10 LET A = 5\n20 END\n");
        let instrs = &result.program.instructions;
        let has_load_imm = instrs.iter().any(|i| i.opcode == IrOp::LoadImm);
        let has_add_imm  = instrs.iter().any(|i| i.opcode == IrOp::AddImm);
        assert!(has_load_imm, "LET should emit LOAD_IMM for the literal");
        assert!(has_add_imm,  "LET should emit ADD_IMM to copy into variable register");
    }

    #[test]
    fn test_let_variable_register() {
        // A is always v1; after LET A = 7 the ADD_IMM destination should be v1.
        let result = compile("10 LET A = 7\n20 END\n");
        let copy = result.program.instructions.iter().find(|i| {
            i.opcode == IrOp::AddImm
                && i.operands.first() == Some(&IrOperand::Register(1))
        });
        assert!(copy.is_some(), "ADD_IMM should target v1 (variable A)");
    }

    #[test]
    fn test_let_arithmetic_expression() {
        // LET B = A + 1 should emit ADD (not just LOAD_IMM)
        let result = compile("10 LET A = 3\n20 LET B = A + 1\n30 END\n");
        let has_add = result.program.instructions.iter().any(|i| i.opcode == IrOp::Add);
        assert!(has_add, "LET with + expression should emit ADD");
    }

    #[test]
    fn test_let_multiply() {
        let result = compile("10 LET A = 3\n20 LET B = A * 2\n30 END\n");
        let has_mul = result.program.instructions.iter().any(|i| i.opcode == IrOp::Mul);
        assert!(has_mul, "LET with * expression should emit MUL");
    }

    #[test]
    fn test_let_divide() {
        let result = compile("10 LET A = 10\n20 LET B = A / 2\n30 END\n");
        let has_div = result.program.instructions.iter().any(|i| i.opcode == IrOp::Div);
        assert!(has_div, "LET with / expression should emit DIV");
    }

    // ── PRINT ────────────────────────────────────────────────────────────

    #[test]
    fn test_print_string_emits_syscalls() {
        let result = compile("10 PRINT \"HELLO\"\n20 END\n");
        let syscall_count = result.program.instructions.iter()
            .filter(|i| i.opcode == IrOp::Syscall)
            .count();
        // "HELLO" = 5 chars + 1 carriage-return = 6 syscalls
        assert_eq!(syscall_count, 6, "PRINT \"HELLO\" should emit 6 SYSCALL instructions");
    }

    #[test]
    fn test_bare_print_emits_one_syscall() {
        // PRINT with no arguments → just a carriage return
        let result = compile("10 PRINT\n20 END\n");
        let syscall_count = result.program.instructions.iter()
            .filter(|i| i.opcode == IrOp::Syscall)
            .count();
        assert_eq!(syscall_count, 1, "bare PRINT should emit exactly 1 SYSCALL");
    }

    #[test]
    fn test_print_number_emits_div_and_mul() {
        // Printing a numeric expression requires digit extraction
        let result = compile("10 LET A = 42\n20 PRINT A\n30 END\n");
        let has_div = result.program.instructions.iter().any(|i| i.opcode == IrOp::Div);
        let has_mul = result.program.instructions.iter().any(|i| i.opcode == IrOp::Mul);
        assert!(has_div, "PRINT number should emit DIV for digit extraction");
        assert!(has_mul, "PRINT number should emit MUL for remainder computation");
    }

    // ── GOTO ─────────────────────────────────────────────────────────────

    #[test]
    fn test_goto_emits_jump() {
        let result = compile("10 GOTO 30\n20 END\n30 END\n");
        let has_jump = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::Jump
                && i.operands == vec![IrOperand::Label("_line_30".to_string())]
        });
        assert!(has_jump, "GOTO 30 should emit JUMP _line_30");
    }

    // ── IF … THEN ────────────────────────────────────────────────────────

    #[test]
    fn test_if_lt_emits_cmp_lt_branch_nz() {
        let result = compile("10 LET A = 3\n20 IF A < 5 THEN 40\n30 END\n40 END\n");
        let has_cmp_lt  = result.program.instructions.iter().any(|i| i.opcode == IrOp::CmpLt);
        let has_branch  = result.program.instructions.iter().any(|i| i.opcode == IrOp::BranchNz);
        assert!(has_cmp_lt, "IF A < 5 should emit CMP_LT");
        assert!(has_branch, "IF A < 5 should emit BRANCH_NZ");
    }

    #[test]
    fn test_if_le_uses_not_bool() {
        // <= is implemented as NOT(>) which requires ADD_IMM(-1) + AND_IMM(1)
        let result = compile("10 LET A = 3\n20 IF A <= 5 THEN 40\n30 END\n40 END\n");
        let has_and_imm = result.program.instructions.iter().any(|i| i.opcode == IrOp::AndImm);
        assert!(has_and_imm, "IF A <= 5 should use not-bool (AND_IMM)");
    }

    #[test]
    fn test_if_eq_emits_cmp_eq() {
        let result = compile("10 LET A = 3\n20 IF A = 3 THEN 40\n30 END\n40 END\n");
        let has_cmp_eq = result.program.instructions.iter().any(|i| i.opcode == IrOp::CmpEq);
        assert!(has_cmp_eq, "IF A = 3 should emit CMP_EQ");
    }

    #[test]
    fn test_if_ne_emits_cmp_ne() {
        let result = compile("10 LET A = 3\n20 IF A <> 5 THEN 40\n30 END\n40 END\n");
        let has_cmp_ne = result.program.instructions.iter().any(|i| i.opcode == IrOp::CmpNe);
        assert!(has_cmp_ne, "IF A <> 5 should emit CMP_NE");
    }

    // ── FOR / NEXT ───────────────────────────────────────────────────────

    #[test]
    fn test_for_next_emits_loop_structure() {
        let result = compile(
            "10 FOR I = 1 TO 10\n20 NEXT I\n30 END\n"
        );
        let instrs = &result.program.instructions;
        // Must have: check label, CMP_GT (exit test), BRANCH_NZ, ADD (increment), JUMP (back-edge)
        assert!(instrs.iter().any(|i| i.opcode == IrOp::CmpGt),
                "FOR should emit CMP_GT for the pre-test");
        assert!(instrs.iter().any(|i| i.opcode == IrOp::BranchNz),
                "FOR should emit BRANCH_NZ to skip past the loop body");
        assert!(instrs.iter().any(|i| i.opcode == IrOp::Add),
                "NEXT should emit ADD for the increment");
        assert!(instrs.iter().any(|i| i.opcode == IrOp::Jump),
                "NEXT should emit JUMP back to the check label");
    }

    #[test]
    fn test_for_with_step() {
        let result = compile(
            "10 FOR I = 0 TO 10 STEP 2\n20 NEXT I\n30 END\n"
        );
        // The step expression (2) should be compiled as a LOAD_IMM.
        let has_load_imm_2 = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::LoadImm && i.operands.get(1) == Some(&IrOperand::Immediate(2))
        });
        assert!(has_load_imm_2, "STEP 2 should emit LOAD_IMM 2");
    }

    #[test]
    fn test_next_without_for_errors() {
        let err = compile_basic_source("10 NEXT I\n20 END\n");
        assert!(err.is_err(), "NEXT without FOR should return an error");
    }

    // ── END / STOP ───────────────────────────────────────────────────────

    #[test]
    fn test_end_emits_halt() {
        let result = compile("10 END\n");
        let halt_count = result.program.instructions.iter()
            .filter(|i| i.opcode == IrOp::Halt)
            .count();
        // At minimum 1 from END, plus the epilogue HALT.
        assert!(halt_count >= 1, "END should emit at least one HALT");
    }

    #[test]
    fn test_stop_emits_halt() {
        let result = compile("10 STOP\n");
        assert!(result.program.instructions.iter().any(|i| i.opcode == IrOp::Halt),
                "STOP should emit HALT");
    }

    // ── Unsupported features ─────────────────────────────────────────────

    #[test]
    fn test_gosub_returns_error() {
        let err = compile_basic_source("10 GOSUB 100\n20 END\n100 END\n");
        assert!(err.is_err(), "GOSUB should return CompileError");
        assert!(err.unwrap_err().message.contains("GOSUB"),
                "error message should mention GOSUB");
    }

    #[test]
    fn test_power_operator_returns_error() {
        // The ^ operator is not supported in V1
        let err = compile_basic_source("10 LET A = 2 ^ 3\n20 END\n");
        assert!(err.is_err(), "^ operator should return CompileError");
    }

    #[test]
    fn test_int_bits_too_small_errors() {
        let err = compile_basic_with_options("10 END\n", CharEncoding::Ge225, 1);
        assert!(err.is_err(), "int_bits < 2 should return error");
    }

    #[test]
    fn test_int_bits_too_large_errors() {
        // int_bits > 63 would overflow i64 inside emit_print_number.
        // The public API must reject it before constructing the compiler.
        let err = compile_basic_with_options("10 END\n", CharEncoding::Ge225, 64);
        assert!(err.is_err(), "int_bits > 63 should return error");
        let msg = err.unwrap_err().message;
        assert!(msg.contains("63"), "error should mention the upper bound of 63");
    }

    #[test]
    #[should_panic(expected = "scalar_reg: name must start with A-Z")]
    fn test_scalar_reg_empty_panics() {
        // An empty string would previously cause an index-out-of-bounds panic
        // with a confusing message.  Now we assert early with a clear message.
        scalar_reg("");
    }

    // ── char_encoding ASCII ──────────────────────────────────────────────

    #[test]
    fn test_ascii_encoding_uses_newline_for_cr() {
        let result = compile_basic_with_options(
            "10 PRINT\n20 END\n",
            CharEncoding::Ascii,
            32,
        ).unwrap();
        // Bare PRINT emits LOAD_IMM v0, 10 (newline in ASCII).
        let has_newline = result.program.instructions.iter().any(|i| {
            i.opcode == IrOp::LoadImm
                && i.operands == vec![
                    IrOperand::Register(0),
                    IrOperand::Immediate(10),
                ]
        });
        assert!(has_newline, "ASCII encoding should use newline (10) for carriage-return");
    }

    // ── Instruction count sanity ─────────────────────────────────────────

    #[test]
    fn test_non_empty_program() {
        let result = compile("10 LET A = 5\n20 END\n");
        assert!(!result.program.instructions.is_empty(),
                "compiled program should have instructions");
    }

    #[test]
    fn test_multi_line_program_compiles() {
        let result = compile(concat!(
            "10 LET I = 1\n",
            "20 LET I = I + 1\n",
            "30 IF I < 5 THEN 20\n",
            "40 END\n"
        ));
        assert!(!result.program.instructions.is_empty());
    }

    #[test]
    fn test_unary_minus() {
        // LET A = -5 should emit LOAD_IMM 0 and SUB for negation
        let result = compile("10 LET A = -5\n20 END\n");
        let has_sub = result.program.instructions.iter().any(|i| i.opcode == IrOp::Sub);
        assert!(has_sub, "unary minus should emit SUB (0 - operand)");
    }
}

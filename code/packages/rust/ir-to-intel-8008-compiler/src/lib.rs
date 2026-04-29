//! Intel 8008 code generator — translates `IrProgram` into assembly text.
//!
//! # Intel 8008 Architecture
//!
//! The Intel 8008 (1972) is the world's first single-chip 8-bit microprocessor.
//! It has a small but capable register file:
//!
//! - **A** — Accumulator: 8-bit implicit result register for all ALU ops.
//!           Every arithmetic and logical instruction uses A and places the
//!           result in A.  A is scratch — not preserved across operations.
//! - **B, C, D, E** — Four 8-bit general-purpose data registers.
//! - **H, L** — High and Low bytes of the 14-bit memory address register.
//!              `H:L = (H << 8) | L`.  Used for all memory ops via M.
//! - **M** — Pseudo-register: the memory byte at address H:L.
//!
//! ## Physical register assignment
//!
//! Virtual IR registers map to 8008 hardware registers:
//!
//! | IR register | Physical | Role |
//! |-------------|----------|------|
//! | v0 | B | constant zero, preloaded to 0 at `_start` |
//! | v1 | C | scratch / return value |
//! | v2 | D | 1st local / 1st argument |
//! | v3 | E | 2nd local / 2nd argument |
//! | v4 | H | 3rd local (careful: H is also memory high byte) |
//! | v5 | L | 4th local (careful: L is also memory low byte) |
//!
//! ## Dangerous opcode conflicts
//!
//! Three `MOV A, {reg}` encodings conflict with other instructions:
//!
//! - `MOV A, C` = 0x79 → **IN 7** (reads input port 7, not register C!)
//! - `MOV A, H` = 0x7C → **JMP** (unconditional 3-byte jump!)
//! - `MOV A, M` = 0x7E → **CAL** (subroutine call!)
//!
//! The compiler uses the safe group-10 ALU path:
//!
//! ```text
//! MVI  A, 0     ; prime accumulator
//! ADD  C        ; A = 0 + C = C  (group-10, always a register read)
//! ```
//!
//! # Example
//!
//! ```
//! use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
//! use ir_to_intel_8008_compiler::{IrToIntel8008Compiler, IrValidator};
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(1), IrOperand::Immediate(42)],
//!     1,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
//!
//! let errors = IrValidator.validate(&prog);
//! assert!(errors.is_empty());
//! let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
//! assert!(asm.contains("MVI  C, 42"));
//! assert!(asm.contains("HLT"));
//! ```

use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

pub mod codegen;

// ---------------------------------------------------------------------------
// Assembly indentation
// ---------------------------------------------------------------------------

const INDENT: &str = "    ";

// ---------------------------------------------------------------------------
// Physical register table
// ---------------------------------------------------------------------------
//
// Virtual register index → 8008 physical register name.
// v0=B (zero constant), v1=C (scratch/return), v2=D..v5=L.
// A is the implicit accumulator and is never an IR register.

fn preg(vreg_index: usize) -> &'static str {
    match vreg_index {
        0 => "B", // constant zero
        1 => "C", // scratch / return value
        2 => "D", // 1st local / 1st arg
        3 => "E", // 2nd local / 2nd arg
        4 => "H", // 3rd local / 3rd arg  (careful: H is memory high byte)
        5 => "L", // 4th local / 4th arg  (careful: L is memory low byte)
        _ => "B", // fallback for unmapped (should not happen on valid IR)
    }
}

/// Emit the shortest safe sequence that loads physical register `reg` into A.
///
/// # The Dangerous Conflicts
///
/// Three `MOV A, {reg}` encodings in group-01 are **not** register copies:
///
/// - `MOV A, C` = 0x79 → **IN 7**: reads input port 7 (SSS=001 in group-01 = IN)
/// - `MOV A, H` = 0x7C → **JMP**: 3-byte unconditional jump (SSS=100 in group-01)
/// - `MOV A, M` = 0x7E → **CAL**: 3-byte subroutine call (SSS=110 in group-01)
///
/// # Safe Workaround
///
/// In group-10 (ALU), SSS field is always a register/memory read — no conflicts.
/// We clear A with `MVI A, 0` then use `ADD {reg}` so A = 0 + reg = reg.
fn load_a(reg: &str) -> Vec<String> {
    match reg {
        "A" => vec![], // already in accumulator
        "C" | "H" | "M" => {
            // Dangerous: MOV A, C → IN 7 / MOV A, H → JMP / MOV A, M → CAL
            // Fix: use group-10 ALU path where SSS is always a register read.
            vec![
                format!("{INDENT}MVI  A, 0"),
                format!("{INDENT}ADD  {reg}"),
            ]
        }
        _ => vec![format!("{INDENT}MOV  A, {reg}")],
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// A single hardware-constraint violation found during validation.
#[derive(Debug, Clone)]
pub struct IrValidationError {
    /// Machine-readable rule identifier (e.g. `"no_word_ops"`).
    pub rule: &'static str,
    /// Human-readable description of the violation.
    pub message: String,
}

impl std::fmt::Display for IrValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.message)
    }
}

/// Intel 8008 hardware-constraint validator.
///
/// Validates an `IrProgram` against the Intel 8008's physical limitations.
/// All errors are collected in a single pass so the caller sees every
/// problem at once.
///
/// # Validation rules
///
/// | Rule | Constraint |
/// |------|------------|
/// | `no_word_ops` | `LOAD_WORD` and `STORE_WORD` are forbidden |
/// | `register_count` | At most 6 distinct virtual register indices (v0–v5) |
/// | `imm_range` | `LOAD_IMM` / `ADD_IMM` immediates must fit in `u8` (0–255) |
/// | `syscall_whitelist` | SYSCALL ∈ {3,4} ∪ {11–16} ∪ {20–27} ∪ {40–63} |
/// | `static_data` | Total data declarations ≤ 8 191 bytes |
pub struct IrValidator;

impl IrValidator {
    /// Validate `program` and return all hardware-constraint violations.
    ///
    /// Returns an empty `Vec` if the program is compatible with the 8008.
    pub fn validate(&self, program: &IrProgram) -> Vec<IrValidationError> {
        let mut errors = Vec::new();

        // Rule: static_data — total IrDataDecl sizes ≤ 8 191 bytes.
        // (8 KB RAM region: addresses 0x2000–0x3FFE)
        let total_data: usize = program.data.iter().map(|d| d.size).sum();
        if total_data > 8191 {
            errors.push(IrValidationError {
                rule: "static_data",
                message: format!(
                    "total static data {} bytes exceeds 8 191-byte 8008 RAM limit",
                    total_data
                ),
            });
        }

        // Per-instruction rules.
        let mut seen_regs = std::collections::HashSet::new();

        for instr in &program.instructions {
            let op = instr.opcode;

            // Rule: no_word_ops — LOAD_WORD and STORE_WORD impossible.
            if op == IrOp::LoadWord || op == IrOp::StoreWord {
                errors.push(IrValidationError {
                    rule: "no_word_ops",
                    message: format!(
                        "{} is not supported on the 8008 (no 16-bit memory bus)",
                        op
                    ),
                });
            }

            // Rule: imm_range — LOAD_IMM / ADD_IMM immediates must fit in u8.
            if op == IrOp::LoadImm || op == IrOp::AddImm {
                for operand in &instr.operands {
                    if let IrOperand::Immediate(v) = operand {
                        if *v < 0 || *v > 255 {
                            errors.push(IrValidationError {
                                rule: "imm_range",
                                message: format!(
                                    "{}: immediate {} does not fit in u8 (valid range 0–255)",
                                    op, v
                                ),
                            });
                        }
                    }
                }
            }

            // Rule: syscall_whitelist — valid 8008 SYSCALL numbers.
            if op == IrOp::Syscall {
                if let Some(IrOperand::Immediate(n)) = instr.operands.first() {
                    let n = *n;
                    let valid = n == 3
                        || n == 4
                        || (11..=16).contains(&n)
                        || (20..=27).contains(&n)
                        || (40..=63).contains(&n);
                    if !valid {
                        errors.push(IrValidationError {
                            rule: "syscall_whitelist",
                            message: format!(
                                "SYSCALL {}: not a valid 8008 intrinsic \
                                 (valid: 3,4,11–16,20–27,40–63)",
                                n
                            ),
                        });
                    }
                }
            }

            // Collect register indices for register_count rule.
            for operand in &instr.operands {
                if let IrOperand::Register(idx) = operand {
                    seen_regs.insert(*idx);
                }
            }
        }

        // Rule: register_count — at most 6 distinct virtual registers (v0–v5).
        for &idx in &seen_regs {
            if idx > 5 {
                errors.push(IrValidationError {
                    rule: "register_count",
                    message: format!(
                        "virtual register v{} exceeds the 8008 physical \
                         register limit (v0–v5 only; v6+ requires spilling)",
                        idx
                    ),
                });
            }
        }

        errors
    }
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

/// Translates a validated `IrProgram` into Intel 8008 assembly text.
///
/// This is a one-pass, instruction-by-instruction translator. It holds a
/// `label_count` used to produce unique local labels for comparison
/// materialisation and the parity intrinsic.
pub struct IrToIntel8008Compiler;

impl IrToIntel8008Compiler {
    /// Translate `program` into Intel 8008 assembly text.
    ///
    /// Runs `IrValidator::validate` first. If any violations are found,
    /// returns `Err(errors)`.
    ///
    /// The output is a multi-line string, one instruction per line:
    /// - `ORG 0x0000` origin directive at the top
    /// - Labels at column 0 with a colon suffix
    /// - Instructions indented with 4 spaces
    pub fn compile(&self, program: &IrProgram) -> Result<String, Vec<IrValidationError>> {
        let errors = IrValidator.validate(program);
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(CodeGen::new().generate(program))
    }
}

// ---------------------------------------------------------------------------
// Internal code generator
// ---------------------------------------------------------------------------

/// One-pass Intel 8008 assembly generator (internal).
///
/// Holds a `label_count` for generating unique local labels inside
/// comparison materialisation sequences.
struct CodeGen {
    label_count: usize,
}

impl CodeGen {
    fn new() -> Self {
        Self { label_count: 0 }
    }

    /// Generate assembly for a full `IrProgram`.
    fn generate(&mut self, program: &IrProgram) -> String {
        let mut lines: Vec<String> = Vec::new();
        // Every 8008 program begins at ROM address 0x0000.
        lines.push(format!("{INDENT}ORG 0x0000"));
        for instr in &program.instructions {
            lines.extend(self.emit(instr));
        }
        lines.join("\n") + "\n"
    }

    /// Generate a unique local label suffix.
    fn next_label(&mut self) -> String {
        let lbl = format!("cmp_{}", self.label_count);
        self.label_count += 1;
        lbl
    }

    /// Emit assembly lines for a single IR instruction.
    fn emit(&mut self, instr: &IrInstruction) -> Vec<String> {
        match instr.opcode {
            IrOp::Label    => self.emit_label(instr),
            IrOp::LoadImm  => self.emit_load_imm(instr),
            IrOp::LoadAddr => self.emit_load_addr(instr),
            IrOp::LoadByte => self.emit_load_byte(instr),
            IrOp::StoreByte => self.emit_store_byte(instr),
            IrOp::Add      => self.emit_binop("ADD", instr),
            IrOp::AddImm   => self.emit_add_imm(instr),
            IrOp::Sub      => self.emit_binop("SUB", instr),
            IrOp::And      => self.emit_binop("ANA", instr),
            IrOp::AndImm   => self.emit_and_imm(instr),
            IrOp::CmpEq    => self.emit_cmp("CMP_EQ", "JTZ", false, instr),
            IrOp::CmpNe    => self.emit_cmp("CMP_NE", "JTZ", true, instr),
            IrOp::CmpLt    => self.emit_cmp_carry("CMP_LT", false, instr),
            IrOp::CmpGt    => self.emit_cmp_carry("CMP_GT", true, instr),
            IrOp::BranchZ  => self.emit_branch("JTZ", instr),
            IrOp::BranchNz => self.emit_branch("JFZ", instr),
            IrOp::Jump     => self.emit_jump(instr),
            IrOp::Call     => self.emit_call(instr),
            IrOp::Ret      => self.emit_ret(),
            IrOp::Halt     => self.emit_halt(),
            IrOp::Syscall  => self.emit_syscall(instr),
            IrOp::Nop      => self.emit_nop(),
            IrOp::Comment  => self.emit_comment(instr),
            op => vec![format!("{INDENT}; unsupported opcode: {}", op)],
        }
    }

    // -----------------------------------------------------------------------
    // LABEL name → name:
    // -----------------------------------------------------------------------
    //
    // Labels sit at column 0 with a colon suffix.

    fn emit_label(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Label(name)) = instr.operands.first() {
            vec![format!("{}:", name)]
        } else {
            vec![]
        }
    }

    // -----------------------------------------------------------------------
    // LOAD_IMM Rdst, imm → MVI Rdst, imm
    // -----------------------------------------------------------------------
    //
    // MVI (Move Immediate) loads an 8-bit constant into any named register.
    // Always a 2-byte instruction: opcode + byte operand.
    //
    // Example: LOAD_IMM v1, 42  →  MVI C, 42

    fn emit_load_imm(&self, instr: &IrInstruction) -> Vec<String> {
        match (instr.operands.first(), instr.operands.get(1)) {
            (Some(IrOperand::Register(dst)), Some(IrOperand::Immediate(imm))) => {
                vec![format!("{INDENT}MVI  {}, {}", preg(*dst), imm)]
            }
            _ => vec![format!("{INDENT}; LOAD_IMM: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // LOAD_ADDR Rdst, symbol → MVI H, hi(symbol) / MVI L, lo(symbol)
    // -----------------------------------------------------------------------
    //
    // Static variables live in the 8008 RAM region (0x2000–0x3FFE). To access
    // RAM the code generator must load the 14-bit address into H:L.
    //
    // The destination register operand is ignored — on the 8008, addresses must
    // live in H:L. The assembler resolves `hi(symbol)` and `lo(symbol)`.
    //
    // Example: LOAD_ADDR v1, counter
    //   →  MVI H, hi(counter)
    //      MVI L, lo(counter)

    fn emit_load_addr(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Label(name)) = instr.operands.get(1) {
            vec![
                format!("{INDENT}MVI  H, hi({})", name),
                format!("{INDENT}MVI  L, lo({})", name),
            ]
        } else {
            vec![format!("{INDENT}; LOAD_ADDR: label operand expected")]
        }
    }

    // -----------------------------------------------------------------------
    // LOAD_BYTE Rdst, Rbase, Rzero → MVI A, 0 / ADD M / MOV Rdst, A
    // -----------------------------------------------------------------------
    //
    // The 8008 reads RAM through the M pseudo-register. H:L must point to
    // the target address (set by the preceding LOAD_ADDR).
    //
    // IMPORTANT: `MOV A, M` = 0x7E = CAL in group-01! We must use the
    // safe group-10 ALU path via `_load_a("M")`: MVI A, 0; ADD M.
    //
    // Example: LOAD_BYTE v1, v1, v0  →  MVI A, 0; ADD M; MOV C, A

    fn emit_load_byte(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Register(dst)) = instr.operands.first() {
            let rdst = preg(*dst);
            let mut lines = load_a("M"); // MVI A, 0; ADD M  (safe group-10 path)
            if rdst != "A" {
                lines.push(format!("{INDENT}MOV  {rdst}, A"));
            }
            lines
        } else {
            vec![format!("{INDENT}; LOAD_BYTE: missing destination register")]
        }
    }

    // -----------------------------------------------------------------------
    // STORE_BYTE Rsrc, Rbase, Rzero → MOV A, Rsrc / MOV M, A
    // -----------------------------------------------------------------------
    //
    // Writes a byte to RAM at H:L (set by the preceding LOAD_ADDR).
    //
    // Example: STORE_BYTE v1, v1, v0  →  MOV A, C; MOV M, A
    // (but MOV A, C = IN 7, so must use safe load_a("C"))

    fn emit_store_byte(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Register(src)) = instr.operands.first() {
            let rsrc = preg(*src);
            let mut lines = load_a(rsrc);
            lines.push(format!("{INDENT}MOV  M, A"));
            lines
        } else {
            vec![format!("{INDENT}; STORE_BYTE: missing source register")]
        }
    }

    // -----------------------------------------------------------------------
    // ADD/SUB/AND Rdst, Ra, Rb → load A, Ra / OP Rb / MOV Rdst, A
    // -----------------------------------------------------------------------
    //
    // Generic three-register binop. The 8008 instruction names:
    //   ADD  r  : A ← A + r
    //   SUB  r  : A ← A − r  (CY = borrow)
    //   ANA  r  : A ← A AND r  (CY = 0)
    //
    // We emit: load_a(Ra); OP Rb; MOV Rdst, A.

    fn emit_binop(&self, op_mnemonic: &str, instr: &IrInstruction) -> Vec<String> {
        match (
            instr.operands.first(),
            instr.operands.get(1),
            instr.operands.get(2),
        ) {
            (
                Some(IrOperand::Register(dst)),
                Some(IrOperand::Register(ra)),
                Some(IrOperand::Register(rb)),
            ) => {
                let rdst = preg(*dst);
                let rra = preg(*ra);
                let rrb = preg(*rb);
                let mut lines = load_a(rra);
                lines.push(format!("{INDENT}{op_mnemonic}  {rrb}"));
                lines.push(format!("{INDENT}MOV  {rdst}, A"));
                lines
            }
            _ => vec![format!("{INDENT}; {op_mnemonic}: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // ADD_IMM Rdst, Ra, imm → load_a(Ra) / ADI imm / MOV Rdst, A
    // -----------------------------------------------------------------------
    //
    // Special case imm == 0 → register copy: load_a(Ra); MOV Rdst, A.
    // For imm != 0: load_a(Ra); ADI imm; MOV Rdst, A.
    //
    // ADI = Add Immediate: A ← A + d8.

    fn emit_add_imm(&self, instr: &IrInstruction) -> Vec<String> {
        match (
            instr.operands.first(),
            instr.operands.get(1),
            instr.operands.get(2),
        ) {
            (
                Some(IrOperand::Register(dst)),
                Some(IrOperand::Register(src)),
                Some(IrOperand::Immediate(imm)),
            ) => {
                let rdst = preg(*dst);
                let rsrc = preg(*src);
                let mut lines = load_a(rsrc);
                if *imm != 0 {
                    lines.push(format!("{INDENT}ADI  {imm}"));
                }
                lines.push(format!("{INDENT}MOV  {rdst}, A"));
                lines
            }
            _ => vec![format!("{INDENT}; ADD_IMM: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // AND_IMM Rdst, Ra, imm → load_a(Ra) / ANI imm / MOV Rdst, A
    // -----------------------------------------------------------------------
    //
    // ANI = AND Immediate: A ← A AND d8.

    fn emit_and_imm(&self, instr: &IrInstruction) -> Vec<String> {
        match (
            instr.operands.first(),
            instr.operands.get(1),
            instr.operands.get(2),
        ) {
            (
                Some(IrOperand::Register(dst)),
                Some(IrOperand::Register(src)),
                Some(IrOperand::Immediate(imm)),
            ) => {
                let rdst = preg(*dst);
                let rsrc = preg(*src);
                let mut lines = load_a(rsrc);
                lines.push(format!("{INDENT}ANI  {imm}"));
                lines.push(format!("{INDENT}MOV  {rdst}, A"));
                lines
            }
            _ => vec![format!("{INDENT}; AND_IMM: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // CMP_EQ / CMP_NE — equality comparison (Z flag)
    // -----------------------------------------------------------------------
    //
    // The 8008 CMP instruction sets Z=1 iff Ra == Rb (A − Rb == 0).
    // We use an optimistic-load approach:
    //
    //   CMP_EQ Rdst, Ra, Rb:
    //     load_a(Ra)
    //     CMP  Rb          ; Z=1 iff Ra == Rb
    //     MVI  Rdst, 1     ; assume equal (true)
    //     JTZ  cmp_N_done  ; Z → keep 1
    //     MVI  Rdst, 0     ; not equal → overwrite
    //   cmp_N_done:
    //
    //   CMP_NE: swap the initial values (0 first, then 1).
    //     `neg_eq=true` means: result for equal case is 0 (i.e. CMP_NE).

    fn emit_cmp(
        &mut self, _op_name: &str, jmp_if: &str, neg_eq: bool, instr: &IrInstruction,
    ) -> Vec<String> {
        match (
            instr.operands.first(),
            instr.operands.get(1),
            instr.operands.get(2),
        ) {
            (
                Some(IrOperand::Register(dst)),
                Some(IrOperand::Register(ra)),
                Some(IrOperand::Register(rb)),
            ) => {
                let rdst = preg(*dst);
                let rra = preg(*ra);
                let rrb = preg(*rb);
                let done = self.next_label();
                // For CMP_EQ: assume=1, then overwrite with 0 if not equal.
                // For CMP_NE: assume=0, then overwrite with 1 if not equal.
                let (assume, overwrite) = if neg_eq { (0, 1) } else { (1, 0) };
                let mut lines = load_a(rra);
                lines.push(format!("{INDENT}CMP  {rrb}"));
                lines.push(format!("{INDENT}MVI  {rdst}, {assume}"));
                lines.push(format!("{INDENT}{jmp_if}  {done}"));
                lines.push(format!("{INDENT}MVI  {rdst}, {overwrite}"));
                lines.push(format!("{done}:"));
                lines
            }
            _ => vec![format!("{INDENT}; CMP: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // CMP_LT / CMP_GT — unsigned carry-based comparison
    // -----------------------------------------------------------------------
    //
    // 8008 CMP/SUB: CY=1 iff A < Rb (unsigned borrow).
    // JTC = jump if carry true (CY=1).
    //
    //   CMP_LT Rdst, Ra, Rb:
    //     load_a(Ra)
    //     CMP  Rb          ; CY=1 iff Ra < Rb (borrow)
    //     MVI  Rdst, 1     ; assume less-than
    //     JTC  cmp_N_done  ; CY → keep 1
    //     MVI  Rdst, 0
    //   cmp_N_done:
    //
    //   CMP_GT: swap operands (Ra > Rb ⟺ Rb < Ra).

    fn emit_cmp_carry(
        &mut self, _op_name: &str, swap: bool, instr: &IrInstruction,
    ) -> Vec<String> {
        match (
            instr.operands.first(),
            instr.operands.get(1),
            instr.operands.get(2),
        ) {
            (
                Some(IrOperand::Register(dst)),
                Some(IrOperand::Register(ra)),
                Some(IrOperand::Register(rb)),
            ) => {
                let rdst = preg(*dst);
                // For CMP_GT: load Rb into A and compare with Ra (swap operands).
                let (lhs, rhs) = if swap {
                    (preg(*rb), preg(*ra))
                } else {
                    (preg(*ra), preg(*rb))
                };
                let done = self.next_label();
                let mut lines = load_a(lhs);
                lines.push(format!("{INDENT}CMP  {rhs}"));
                lines.push(format!("{INDENT}MVI  {rdst}, 1"));
                lines.push(format!("{INDENT}JTC  {done}"));
                lines.push(format!("{INDENT}MVI  {rdst}, 0"));
                lines.push(format!("{done}:"));
                lines
            }
            _ => vec![format!("{INDENT}; CMP_LT/GT: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // BRANCH_Z Rcond, lbl → load_a(Rcond) / CPI 0 / JTZ lbl
    // BRANCH_NZ Rcond, lbl → load_a(Rcond) / CPI 0 / JFZ lbl
    // -----------------------------------------------------------------------
    //
    // CPI 0 = compare A with 0. Sets Z=1 iff A == 0.
    // JTZ = jump if Z=1 (result was zero).
    // JFZ = jump if Z=0 (result was non-zero).

    fn emit_branch(&self, jmp: &str, instr: &IrInstruction) -> Vec<String> {
        match (instr.operands.first(), instr.operands.get(1)) {
            (Some(IrOperand::Register(reg)), Some(IrOperand::Label(lbl))) => {
                let rn = preg(*reg);
                let mut lines = load_a(rn);
                lines.push(format!("{INDENT}CPI  0"));
                lines.push(format!("{INDENT}{jmp}  {lbl}"));
                lines
            }
            _ => vec![format!("{INDENT}; BRANCH: missing or invalid operands")],
        }
    }

    // -----------------------------------------------------------------------
    // JUMP lbl → JMP lbl
    // -----------------------------------------------------------------------
    //
    // JMP is the 8008 unconditional branch — a 3-byte instruction.

    fn emit_jump(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Label(lbl)) = instr.operands.first() {
            vec![format!("{INDENT}JMP  {lbl}")]
        } else {
            vec![format!("{INDENT}; JUMP: missing label operand")]
        }
    }

    // -----------------------------------------------------------------------
    // CALL lbl → CAL lbl
    // -----------------------------------------------------------------------
    //
    // CAL (Call subroutine): push PC+3 onto the 8-level hardware stack and
    // jump to the subroutine address. The callee returns via RFC.

    fn emit_call(&self, instr: &IrInstruction) -> Vec<String> {
        if let Some(IrOperand::Label(lbl)) = instr.operands.first() {
            vec![format!("{INDENT}CAL  {lbl}")]
        } else {
            vec![format!("{INDENT}; CALL: missing label operand")]
        }
    }

    // -----------------------------------------------------------------------
    // RET → MVI A, 0 / ADD C / RFC
    // -----------------------------------------------------------------------
    //
    // The Oct calling convention places the return value in v1=C.
    // Before returning, we copy C to A (the 8008 return-value register).
    //
    // ⚠️  We CANNOT emit `MOV A, C` — it encodes as `IN 7` (group-01 SSS=001)!
    // Fix: use the ALU path:
    //   MVI A, 0   ; prime accumulator
    //   ADD C      ; A = 0 + C = C  (group-10, SSS=001 correctly reads reg C)
    //   RFC        ; return unconditionally (CY=0, so RFC always returns)

    fn emit_ret(&self) -> Vec<String> {
        // _REG_RESULT = "C" (v1)
        vec![
            format!("{INDENT}MVI  A, 0"),
            format!("{INDENT}ADD  C"),
            format!("{INDENT}RFC"),
        ]
    }

    // -----------------------------------------------------------------------
    // HALT → HLT
    // -----------------------------------------------------------------------
    //
    // HLT (0xFF) halts the Intel 8008 processor.

    fn emit_halt(&self) -> Vec<String> {
        vec![format!("{INDENT}HLT")]
    }

    // -----------------------------------------------------------------------
    // NOP → comment (8008 has no true NOP)
    // -----------------------------------------------------------------------
    //
    // The Intel 8008 has no dedicated NOP instruction. The closest equivalent
    // is `MOV A, A` (self-copy), but we emit a comment instead.

    fn emit_nop(&self) -> Vec<String> {
        vec![format!("{INDENT}; NOP (no-op; omitted on 8008)")]
    }

    // -----------------------------------------------------------------------
    // COMMENT → ; text
    // -----------------------------------------------------------------------

    fn emit_comment(&self, instr: &IrInstruction) -> Vec<String> {
        match instr.operands.first() {
            Some(IrOperand::Label(text)) => vec![format!("{INDENT}; {text}")],
            Some(IrOperand::Immediate(n)) => vec![format!("{INDENT}; {n}")],
            _ => vec![format!("{INDENT};")],
        }
    }

    // -----------------------------------------------------------------------
    // SYSCALL → inline 8008 hardware intrinsic
    // -----------------------------------------------------------------------
    //
    // The Oct IR uses SYSCALL to represent 10 hardware intrinsics:
    //   3  = adc(a, b)  → ADC E (a=D, b=E, result→C)
    //   4  = sbb(a, b)  → SBB E (a=D, b=E, result→C)
    //   11 = rlc(a)     → RLC   (a=D, result→C)
    //   12 = rrc(a)     → RRC   (a=D, result→C)
    //   13 = ral(a)     → RAL   (a=D, result→C)
    //   14 = rar(a)     → RAR   (a=D, result→C)
    //   15 = carry()    → ACI 0 trick; result→C
    //   16 = parity(a)  → ORA A + branch; a=D, result→C
    //   20+p = in(p)    → IN p  (p ∈ 0–7); result→C
    //   40+p = out(p,v) → OUT p (p ∈ 0–23); v=D

    fn emit_syscall(&mut self, instr: &IrInstruction) -> Vec<String> {
        let num = match instr.operands.first() {
            Some(IrOperand::Immediate(n)) => *n,
            _ => return vec![format!("{INDENT}; SYSCALL: missing number operand")],
        };

        match num {
            3  => self.emit_syscall_adc(),
            4  => self.emit_syscall_sbb(),
            11 => self.emit_syscall_rotate("RLC"),
            12 => self.emit_syscall_rotate("RRC"),
            13 => self.emit_syscall_rotate("RAL"),
            14 => self.emit_syscall_rotate("RAR"),
            15 => self.emit_syscall_carry(),
            16 => self.emit_syscall_parity(),
            20..=27 => self.emit_syscall_in((num - 20) as usize),
            40..=63 => self.emit_syscall_out((num - 40) as usize),
            _  => vec![format!("{INDENT}; SYSCALL {num}: unrecognised")],
        }
    }

    /// SYSCALL 3 — adc(a, b): A ← D + E + CY → result in C.
    ///
    /// a is in v2=D, b is in v3=E.
    ///
    /// ```text
    /// MOV  A, D      ; load a into accumulator
    /// ADC  E         ; A = a + b + CY
    /// MOV  C, A      ; result in v1=C
    /// ```
    fn emit_syscall_adc(&self) -> Vec<String> {
        vec![
            format!("{INDENT}MOV  A, D"),   // v2=D = arg0
            format!("{INDENT}ADC  E"),       // v3=E = arg1
            format!("{INDENT}MOV  C, A"),   // v1=C = result
        ]
    }

    /// SYSCALL 4 — sbb(a, b): A ← D − E − CY → result in C.
    ///
    /// ```text
    /// MOV  A, D
    /// SBB  E
    /// MOV  C, A
    /// ```
    fn emit_syscall_sbb(&self) -> Vec<String> {
        vec![
            format!("{INDENT}MOV  A, D"),
            format!("{INDENT}SBB  E"),
            format!("{INDENT}MOV  C, A"),
        ]
    }

    /// SYSCALL 11–14 — rlc/rrc/ral/rar: rotate D → result in C.
    ///
    /// All four rotations: load D into A, apply rotation, store in C.
    ///
    /// ```text
    /// MOV  A, D
    /// RLC            ; (or RRC, RAL, RAR)
    /// MOV  C, A
    /// ```
    fn emit_syscall_rotate(&self, mnemonic: &str) -> Vec<String> {
        vec![
            format!("{INDENT}MOV  A, D"),
            format!("{INDENT}{mnemonic}"),
            format!("{INDENT}MOV  C, A"),
        ]
    }

    /// SYSCALL 15 — carry(): materialise CY into C.
    ///
    /// ACI 0 trick: `A ← A + 0 + CY = CY` (after priming A = 0).
    ///
    /// ```text
    /// MVI  A, 0
    /// ACI  0         ; A = 0 + 0 + CY = CY
    /// MOV  C, A
    /// ```
    fn emit_syscall_carry(&self) -> Vec<String> {
        vec![
            format!("{INDENT}MVI  A, 0"),
            format!("{INDENT}ACI  0"),
            format!("{INDENT}MOV  C, A"),
        ]
    }

    /// SYSCALL 16 — parity(a): materialise P flag from D into C.
    ///
    /// ORA A refreshes all flags without changing A.
    /// JFP (Jump if Parity False) = jump if P=0 (odd parity).
    ///
    /// ```text
    /// MOV  A, D
    /// ORA  A         ; refresh P flag
    /// MVI  C, 0      ; assume odd parity
    /// JFP  par_N
    /// MVI  C, 1      ; even parity
    /// par_N:
    /// ```
    fn emit_syscall_parity(&mut self) -> Vec<String> {
        let done = self.next_label();
        vec![
            format!("{INDENT}MOV  A, D"),
            format!("{INDENT}ORA  A"),
            format!("{INDENT}MVI  C, 0"),
            format!("{INDENT}JFP  {done}"),
            format!("{INDENT}MVI  C, 1"),
            format!("{done}:"),
        ]
    }

    /// SYSCALL 20+p — in(p): read input port p → result in C.
    ///
    /// ```text
    /// IN   p
    /// MOV  C, A
    /// ```
    fn emit_syscall_in(&self, port: usize) -> Vec<String> {
        vec![
            format!("{INDENT}IN   {port}"),
            format!("{INDENT}MOV  C, A"),
        ]
    }

    /// SYSCALL 40+p — out(p, val): write D to output port p.
    ///
    /// The value to write is staged in v2=D.
    ///
    /// ```text
    /// MOV  A, D
    /// OUT  p
    /// ```
    fn emit_syscall_out(&self, port: usize) -> Vec<String> {
        vec![
            format!("{INDENT}MOV  A, D"),
            format!("{INDENT}OUT  {port}"),
        ]
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrDataDecl, IrInstruction, IrOp, IrOperand, IrProgram};

    fn simple_prog() -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        p
    }

    fn prog_with(instr: IrInstruction) -> IrProgram {
        let mut p = IrProgram::new("_start");
        p.add_instruction(instr);
        p.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        p
    }

    // -----------------------------------------------------------------------
    // preg / load_a helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_preg_maps_correctly() {
        assert_eq!(preg(0), "B");
        assert_eq!(preg(1), "C");
        assert_eq!(preg(2), "D");
        assert_eq!(preg(3), "E");
        assert_eq!(preg(4), "H");
        assert_eq!(preg(5), "L");
    }

    #[test]
    fn test_load_a_a_is_noop() {
        assert!(load_a("A").is_empty());
    }

    #[test]
    fn test_load_a_safe_registers_use_mov() {
        let b = load_a("B");
        assert_eq!(b.len(), 1);
        assert!(b[0].contains("MOV  A, B"));
    }

    #[test]
    fn test_load_a_dangerous_registers_use_alu_path() {
        for reg in &["C", "H", "M"] {
            let lines = load_a(reg);
            assert_eq!(lines.len(), 2, "expected 2 lines for {reg}");
            assert!(lines[0].contains("MVI  A, 0"), "for {reg}: {}", lines[0]);
            assert!(lines[1].contains(&format!("ADD  {reg}")), "for {reg}: {}", lines[1]);
        }
    }

    // -----------------------------------------------------------------------
    // Validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_empty_program_passes() {
        assert!(IrValidator.validate(&simple_prog()).is_empty());
    }

    #[test]
    fn test_validate_rejects_load_word() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.rule == "no_word_ops"));
    }

    #[test]
    fn test_validate_rejects_store_word() {
        let prog = prog_with(IrInstruction::new(
            IrOp::StoreWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "no_word_ops"));
    }

    #[test]
    fn test_validate_rejects_immediate_too_large() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(256)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "imm_range"));
    }

    #[test]
    fn test_validate_rejects_negative_immediate() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(-1)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "imm_range"));
    }

    #[test]
    fn test_validate_allows_u8_immediate() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(255)],
            1,
        ));
        assert!(IrValidator.validate(&prog).is_empty());
    }

    #[test]
    fn test_validate_rejects_invalid_syscall() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "syscall_whitelist"));
    }

    #[test]
    fn test_validate_allows_valid_syscalls() {
        for &n in &[3i64, 4, 11, 12, 13, 14, 15, 16, 20, 27, 40, 63] {
            let prog = prog_with(IrInstruction::new(
                IrOp::Syscall,
                vec![IrOperand::Immediate(n)],
                1,
            ));
            let errors = IrValidator.validate(&prog);
            assert!(
                errors.iter().all(|e| e.rule != "syscall_whitelist"),
                "SYSCALL {n} should be allowed"
            );
        }
    }

    #[test]
    fn test_validate_rejects_high_register() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(6), IrOperand::Immediate(1)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "register_count"));
    }

    #[test]
    fn test_validate_rejects_excess_static_data() {
        let mut prog = IrProgram::new("_start");
        prog.data.push(IrDataDecl { label: "buf".to_string(), size: 8192, init: 0 });
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let errors = IrValidator.validate(&prog);
        assert!(errors.iter().any(|e| e.rule == "static_data"));
    }

    // -----------------------------------------------------------------------
    // Compilation tests — structural checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_compile_starts_with_org() {
        let asm = IrToIntel8008Compiler.compile(&simple_prog()).unwrap();
        assert!(asm.starts_with("    ORG 0x0000\n"));
    }

    #[test]
    fn test_compile_ends_with_newline() {
        let asm = IrToIntel8008Compiler.compile(&simple_prog()).unwrap();
        assert!(asm.ends_with('\n'));
    }

    #[test]
    fn test_halt_emits_hlt() {
        let asm = IrToIntel8008Compiler.compile(&simple_prog()).unwrap();
        assert!(asm.contains("HLT"));
    }

    #[test]
    fn test_load_imm_emits_mvi() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(42)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("MVI  C, 42"), "asm was: {asm}");
    }

    #[test]
    fn test_label_emits_at_column_zero() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("my_label".to_string())],
            -1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("my_label:"), "asm was: {asm}");
        // Label must be at column 0 (not indented)
        assert!(asm.lines().any(|l| l == "my_label:"));
    }

    #[test]
    fn test_load_addr_emits_mvi_h_l() {
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadAddr,
            vec![
                IrOperand::Register(1),
                IrOperand::Label("counter".to_string()),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("MVI  H, hi(counter)"), "asm: {asm}");
        assert!(asm.contains("MVI  L, lo(counter)"), "asm: {asm}");
    }

    #[test]
    fn test_load_byte_uses_safe_alu_path() {
        // LOAD_BYTE v1, v1, v0  → must NOT emit MOV A, M (= CAL!)
        let prog = prog_with(IrInstruction::new(
            IrOp::LoadByte,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Register(0),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        // Safe path: MVI A, 0; ADD M
        assert!(asm.contains("MVI  A, 0"), "asm: {asm}");
        assert!(asm.contains("ADD  M"), "asm: {asm}");
        // Must NOT contain the dangerous MOV A, M
        assert!(!asm.contains("MOV  A, M"), "dangerous MOV A, M in asm: {asm}");
    }

    #[test]
    fn test_add_emits_load_add_mov() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Add,
            vec![
                IrOperand::Register(1), // dst = v1 = C
                IrOperand::Register(2), // Ra  = v2 = D
                IrOperand::Register(3), // Rb  = v3 = E
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("MOV  A, D"), "asm: {asm}");
        assert!(asm.contains("ADD  E"), "asm: {asm}");
        assert!(asm.contains("MOV  C, A"), "asm: {asm}");
    }

    #[test]
    fn test_add_imm_zero_is_copy() {
        let prog = prog_with(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(1), // dst
                IrOperand::Register(2), // src
                IrOperand::Immediate(0),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        // imm == 0 → no ADI
        assert!(!asm.contains("ADI"), "asm should not have ADI for imm=0: {asm}");
        assert!(asm.contains("MOV  A, D"), "asm: {asm}");
        assert!(asm.contains("MOV  C, A"), "asm: {asm}");
    }

    #[test]
    fn test_cmp_eq_emits_materialization_sequence() {
        let prog = prog_with(IrInstruction::new(
            IrOp::CmpEq,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(2),
                IrOperand::Register(3),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("CMP  E"), "asm: {asm}");
        assert!(asm.contains("JTZ"), "asm: {asm}");
        assert!(asm.contains("MVI  C, 1"), "asm: {asm}");
        assert!(asm.contains("MVI  C, 0"), "asm: {asm}");
    }

    #[test]
    fn test_cmp_ne_has_swapped_values() {
        let prog = prog_with(IrInstruction::new(
            IrOp::CmpNe,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(2),
                IrOperand::Register(3),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        // CMP_NE: assume=0, overwrite=1
        let lines: Vec<&str> = asm.lines().collect();
        let mvi_lines: Vec<&str> = lines.iter()
            .filter(|l| l.contains("MVI  C,"))
            .copied()
            .collect();
        // First MVI should be 0 (assume not equal), second should be 1
        assert!(mvi_lines[0].contains("0"), "first MVI for CMP_NE should be 0: {}", mvi_lines[0]);
        assert!(mvi_lines[1].contains("1"), "second MVI for CMP_NE should be 1: {}", mvi_lines[1]);
    }

    #[test]
    fn test_cmp_lt_emits_jtc() {
        let prog = prog_with(IrInstruction::new(
            IrOp::CmpLt,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(2),
                IrOperand::Register(3),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("JTC"), "asm: {asm}");
    }

    #[test]
    fn test_cmp_gt_swaps_operands() {
        // CMP_GT v1, v2, v3: Ra=v2=D, Rb=v3=E
        // Should swap: load E into A, CMP D
        let prog = prog_with(IrInstruction::new(
            IrOp::CmpGt,
            vec![
                IrOperand::Register(1), // dst
                IrOperand::Register(2), // Ra = D
                IrOperand::Register(3), // Rb = E
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        // For CMP_GT, we load Rb (E) into A and compare with Ra (D)
        assert!(asm.contains("MOV  A, E"), "CMP_GT should load Rb(E) not Ra(D): {asm}");
        assert!(asm.contains("CMP  D"), "CMP_GT should compare with Ra(D): {asm}");
    }

    #[test]
    fn test_branch_z_emits_cpi_jtz() {
        let prog = prog_with(IrInstruction::new(
            IrOp::BranchZ,
            vec![
                IrOperand::Register(1),
                IrOperand::Label("loop_start".to_string()),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("CPI  0"), "asm: {asm}");
        assert!(asm.contains("JTZ  loop_start"), "asm: {asm}");
    }

    #[test]
    fn test_branch_nz_emits_jfz() {
        let prog = prog_with(IrInstruction::new(
            IrOp::BranchNz,
            vec![
                IrOperand::Register(2),
                IrOperand::Label("loop_back".to_string()),
            ],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("JFZ  loop_back"), "asm: {asm}");
    }

    #[test]
    fn test_jump_emits_jmp() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Jump,
            vec![IrOperand::Label("target".to_string())],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("JMP  target"), "asm: {asm}");
    }

    #[test]
    fn test_call_emits_cal() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Call,
            vec![IrOperand::Label("my_fn".to_string())],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("CAL  my_fn"), "asm: {asm}");
    }

    #[test]
    fn test_ret_uses_safe_alu_path() {
        let prog = prog_with(IrInstruction::new(IrOp::Ret, vec![], 1));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        // Must NOT contain MOV A, C (= IN 7!)
        assert!(!asm.contains("MOV  A, C"), "dangerous MOV A, C in asm: {asm}");
        // Must use safe ALU path: MVI A, 0; ADD C; RFC
        assert!(asm.contains("MVI  A, 0"), "asm: {asm}");
        assert!(asm.contains("ADD  C"), "asm: {asm}");
        assert!(asm.contains("RFC"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_adc() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(3)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("ADC  E"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_sbb() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(4)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("SBB  E"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_rlc() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(11)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("RLC"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_carry_uses_aci_0() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(15)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("ACI  0"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_parity() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(16)],
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("ORA  A"), "asm: {asm}");
        assert!(asm.contains("JFP"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_in_port_3() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(23)], // 20+3 = port 3
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("IN   3"), "asm: {asm}");
    }

    #[test]
    fn test_syscall_out_port_5() {
        let prog = prog_with(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(45)], // 40+5 = port 5
            1,
        ));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("OUT  5"), "asm: {asm}");
    }

    #[test]
    fn test_comparison_labels_are_unique() {
        // Two CMP_EQ instructions must produce different local labels.
        let mut prog = IrProgram::new("_start");
        for id in 1..=3 {
            prog.add_instruction(IrInstruction::new(
                IrOp::CmpEq,
                vec![
                    IrOperand::Register(1),
                    IrOperand::Register(2),
                    IrOperand::Register(3),
                ],
                id,
            ));
        }
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 99));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("cmp_0:"), "asm: {asm}");
        assert!(asm.contains("cmp_1:"), "asm: {asm}");
        assert!(asm.contains("cmp_2:"), "asm: {asm}");
    }

    #[test]
    fn test_nop_emits_comment() {
        let prog = prog_with(IrInstruction::new(IrOp::Nop, vec![], 1));
        let asm = IrToIntel8008Compiler.compile(&prog).unwrap();
        assert!(asm.contains("; NOP"), "asm: {asm}");
    }
}

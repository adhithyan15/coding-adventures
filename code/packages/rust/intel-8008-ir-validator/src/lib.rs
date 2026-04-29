//! # intel-8008-ir-validator — Hardware-constraint validation for the Intel 8008.
//!
//! The Intel 8008 (1972) is the world's first commercially available 8-bit
//! microprocessor.  It targets real hardware that imposes constraints the Oct
//! type checker cannot know about:
//!
//! - **8-level hardware call stack** — only 7 levels available for user CAL
//!   (level 0 is always the current PC)
//! - **Only 4 user-data registers**: B, C, D, E (accumulator A is scratch)
//! - **8-bit immediates only** — MVI, ADI, etc. all take 8-bit literals (0–255)
//! - **SYSCALL numbers restricted** — only hardware-supported intrinsics allowed
//! - **8 KB RAM region** — at most 8 191 static bytes (addresses 0x2000–0x3FFF)
//! - **No 16-bit memory bus** — LOAD_WORD / STORE_WORD are impossible
//!
//! This validator answers: "Can this IR program run on real 8008 hardware?"
//! It collects **all** violations in one pass so the programmer sees every
//! problem at once rather than fixing them one by one.
//!
//! ## Validation rules
//!
//! | Rule               | Constraint                                                |
//! |--------------------|-----------------------------------------------------------|
//! | `no_word_ops`      | LOAD_WORD and STORE_WORD opcodes are forbidden            |
//! | `static_ram`       | Sum of all data declaration sizes ≤ 8 191 bytes           |
//! | `call_depth`       | Static call-graph DFS depth ≤ 7                           |
//! | `register_count`   | Distinct virtual register indices ≤ 6                     |
//! | `imm_range`        | Every LOAD_IMM and ADD_IMM immediate fits in u8 (0–255)   |
//! | `syscall_whitelist`| SYSCALL numbers ∈ {3,4} ∪ {11–16} ∪ {20–27} ∪ {40–63} |
//!
//! ## Quick start
//!
//! ```
//! use compiler_ir::{IrProgram, IrInstruction, IrOp, IrOperand};
//! use intel_8008_ir_validator::IrValidator;
//!
//! let mut prog = IrProgram::new("_start");
//! prog.add_instruction(IrInstruction::new(
//!     IrOp::LoadImm,
//!     vec![IrOperand::Register(0), IrOperand::Immediate(42)],
//!     0,
//! ));
//! prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
//!
//! let validator = IrValidator;
//! let errors = validator.validate(&prog);
//! assert!(errors.is_empty(), "should pass all checks");
//! ```

use std::collections::{HashMap, HashSet};
use std::fmt;

use compiler_ir::{IrOp, IrOperand, IrProgram};

// ===========================================================================
// Hardware constants — the "walls" every 8008 program must fit inside
// ===========================================================================

/// Maximum bytes of static RAM.
///
/// The 8008 RAM region spans 0x2000–0x3FFF (8 192 bytes total).  We
/// reserve the last byte (0x3FFF) as a guard, giving a practical limit of
/// 8 191 usable bytes.  Programs with more static data overflow into ROM
/// space, which is read-only — crash.
const MAX_RAM_BYTES: usize = 8191;

/// Maximum allowed call-graph depth.
///
/// The 8008 hardware push-down stack has 8 slots.  Slot 0 is permanently
/// occupied by the current PC.  That leaves 7 slots for CAL instructions.
/// An 8th nested call wraps the stack, silently corrupting return addresses.
///
/// Think of it as 8 plates: the bottom plate is always there, you can
/// stack 7 more.  The 8th plate slides the bottom one out — catastrophic.
const MAX_CALL_DEPTH: usize = 7;

/// Maximum number of distinct virtual register indices.
///
/// The 8008 has 7 named registers: A, B, C, D, E, H, L.
///
/// ```text
/// A      = accumulator — scratch for ALU; NOT a persistent data register
/// H, L   = 14-bit address register — reserved for LOAD_ADDR / LOAD_BYTE /
///           STORE_BYTE sequences; NOT user-addressable
/// B, C, D, E = 4 user data registers (persistent across instructions)
/// ```
///
/// The Oct calling convention maps virtual registers to physical registers:
///
/// ```text
/// v0  → B   (constant zero, preloaded at _start)
/// v1  → A   (scratch / return value — lives in the accumulator)
/// v2  → C   (1st local / 1st argument slot)
/// v3  → D   (2nd local / 2nd argument slot)
/// v4  → E   (3rd local / 3rd argument slot)
/// v5  → ??? (4th local — tight; H/L are reserved)
/// ```
///
/// Any virtual register index ≥ 6 cannot be assigned a physical home
/// without spilling to RAM, which the current code generator does not
/// support.
const MAX_VIRTUAL_REGISTERS: usize = 6;

/// Minimum immediate value for LOAD_IMM and ADD_IMM.
///
/// All 8008 immediate instructions (MVI, ADI, ACI, SUI, ANI, XRI, ORI,
/// CPI) encode their literal operand in a single 8-bit byte following the
/// opcode.  Values below 0 are out of range for the unsigned byte field.
const MIN_IMM: i64 = 0;

/// Maximum immediate value for LOAD_IMM and ADD_IMM.
///
/// The 8-bit immediate field can hold at most 255 (0xFF).  A value of 256
/// would require two bytes — impossible to encode in a single MVI or ADI.
const MAX_IMM: i64 = 255;

// ===========================================================================
// ValidationDiagnostic — one violation found by the validator
// ===========================================================================

/// A single hardware-constraint violation detected by [`IrValidator`].
///
/// Each diagnostic carries:
/// - `rule`    — the short identifier for which check fired
///              (`"no_word_ops"`, `"static_ram"`, `"call_depth"`,
///               `"register_count"`, `"imm_range"`, `"syscall_whitelist"`)
/// - `message` — a human-readable description and suggested fix
///
/// # Display
///
/// ```
/// use intel_8008_ir_validator::ValidationDiagnostic;
/// let d = ValidationDiagnostic {
///     rule: "imm_range".to_string(),
///     message: "LOAD_IMM immediate 300 is out of range".to_string(),
/// };
/// assert_eq!(d.to_string(), "[imm_range] LOAD_IMM immediate 300 is out of range");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    /// Short rule identifier, e.g. `"syscall_whitelist"`.
    pub rule: String,
    /// Human-readable description of the violation.
    pub message: String,
}

impl fmt::Display for ValidationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.rule, self.message)
    }
}

// ===========================================================================
// valid_syscalls — the set of 8008-supported SYSCALL numbers
// ===========================================================================

/// Build the set of SYSCALL numbers that have a valid 8008 hardware lowering.
///
/// The 8008 intrinsic SYSCALL numbers and their hardware counterparts:
///
/// ```text
///  3       → adc(a, b)   — ADC r instruction
///  4       → sbb(a, b)   — SBB r instruction
/// 11       → rlc(a)      — RLC (rotate A left circular)
/// 12       → rrc(a)      — RRC (rotate A right circular)
/// 13       → ral(a)      — RAL (rotate A left through carry)
/// 14       → rar(a)      — RAR (rotate A right through carry)
/// 15       → carry()     — ACI 0 trick to materialise carry flag
/// 16       → parity(a)   — ORA A + conditional branch for parity
/// 20..=27  → in(p)       — IN p instruction (p = syscall# - 20, ports 0–7)
/// 40..=63  → out(p, v)   — OUT p instruction (p = syscall# - 40, ports 0–23)
/// ```
///
/// Any number outside this set has no 8008 assembly instruction to lower
/// it to and is rejected.
fn valid_syscalls() -> HashSet<i64> {
    let mut set = HashSet::new();
    set.insert(3); // adc
    set.insert(4); // sbb
    for i in 11..=16 { // rlc, rrc, ral, rar, carry, parity
        set.insert(i);
    }
    for i in 20..=27 { // in(0)..in(7)
        set.insert(i);
    }
    for i in 40..=63 { // out(0)..out(23)
        set.insert(i);
    }
    set
}

// ===========================================================================
// IrValidator — the main validator type
// ===========================================================================

/// Validates an [`IrProgram`] against all Intel 8008 hardware constraints.
///
/// The validator runs six independent checks in a single pass (plus a DFS
/// for call depth).  All violations are accumulated and returned together
/// so the programmer can fix everything at once rather than discovering
/// failures one at a time.
///
/// # Usage
///
/// ```
/// use compiler_ir::{IrProgram, IrInstruction, IrOp};
/// use intel_8008_ir_validator::IrValidator;
///
/// let prog = IrProgram::new("_start");
/// let errors = IrValidator.validate(&prog);
/// if errors.is_empty() {
///     println!("Program is feasible on Intel 8008 hardware.");
/// } else {
///     for e in &errors {
///         println!("{}", e);
///     }
/// }
/// ```
pub struct IrValidator;

impl IrValidator {
    /// Run all hardware-constraint checks on `program`.
    ///
    /// Checks are independent — a failure in one does not prevent the
    /// others from running.  This gives the programmer a complete picture
    /// of everything that must be fixed.
    ///
    /// Returns an empty `Vec` when the program passes all six checks and
    /// can proceed to code generation.
    pub fn validate(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let mut errors = Vec::new();
        errors.extend(self.check_no_word_ops(program));
        errors.extend(self.check_static_ram(program));
        errors.extend(self.check_call_depth(program));
        errors.extend(self.check_register_count(program));
        errors.extend(self.check_imm_range(program));
        errors.extend(self.check_syscall_whitelist(program));
        errors
    }

    // -----------------------------------------------------------------------
    // Rule 1: No 16-bit memory operations
    // -----------------------------------------------------------------------
    //
    // The 8008 is an 8-bit CPU with an 8-bit data bus.  Every memory access
    // moves exactly one byte via the M pseudo-register (memory at address H:L).
    // There is no 16-bit move instruction, no double-byte fetch.
    //
    // LOAD_WORD / STORE_WORD would require two separate byte-wide accesses with
    // manually managed H:L addressing — which is not what those IR opcodes
    // represent.  If 16-bit quantities are needed, split them into two
    // LOAD_BYTE/STORE_BYTE pairs at consecutive addresses (low byte + high byte).
    //
    // We emit at most one error per forbidden opcode type (not one per
    // occurrence) — the programmer knows to eliminate all instances.

    fn check_no_word_ops(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let mut errors = Vec::new();
        let mut seen_load_word = false;
        let mut seen_store_word = false;

        for instr in &program.instructions {
            if instr.opcode == IrOp::LoadWord && !seen_load_word {
                errors.push(ValidationDiagnostic {
                    rule: "no_word_ops".to_string(),
                    message: concat!(
                        "LOAD_WORD is not supported on Intel 8008 — the CPU has an 8-bit data bus ",
                        "and no 16-bit memory instruction.  Replace with two LOAD_BYTE instructions ",
                        "at consecutive addresses (low byte + high byte)."
                    ).to_string(),
                });
                seen_load_word = true;
            } else if instr.opcode == IrOp::StoreWord && !seen_store_word {
                errors.push(ValidationDiagnostic {
                    rule: "no_word_ops".to_string(),
                    message: concat!(
                        "STORE_WORD is not supported on Intel 8008 — the CPU has an 8-bit data bus ",
                        "and no 16-bit memory instruction.  Replace with two STORE_BYTE instructions ",
                        "at consecutive addresses (low byte + high byte)."
                    ).to_string(),
                });
                seen_store_word = true;
            }
        }
        errors
    }

    // -----------------------------------------------------------------------
    // Rule 2: Static RAM usage ≤ 8 191 bytes
    // -----------------------------------------------------------------------
    //
    // The 8008 backend maps static variables into the RAM region starting at
    // 0x2000.  The region runs to 0x3FFF, giving 8 192 bytes total.  We
    // reserve the last byte as a guard, so the practical limit is 8 191.
    //
    // Each Oct `static u8` variable occupies exactly 1 byte, so this limit
    // is equivalent to "at most 8 191 static variables".
    //
    // Analogy: the RAM is an 8 KB apartment.  Each `static` is a piece of
    // furniture.  Exceeding 8 191 pieces means some land outside the apartment
    // in read-only ROM space — crash.

    fn check_static_ram(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let total: usize = program.data.iter().map(|d| d.size).sum();
        if total > MAX_RAM_BYTES {
            return vec![ValidationDiagnostic {
                rule: "static_ram".to_string(),
                message: format!(
                    "Static RAM usage {} bytes exceeds the Intel 8008 limit of {} bytes \
                     (RAM region 0x2000–0x3FFE).  \
                     Reduce data declarations by at least {} bytes.",
                    total,
                    MAX_RAM_BYTES,
                    total - MAX_RAM_BYTES,
                ),
            }];
        }
        vec![]
    }

    // -----------------------------------------------------------------------
    // Rule 3: Call graph depth ≤ 7
    // -----------------------------------------------------------------------
    //
    // The 8008 hardware stack is an 8-slot circular push-down register.
    // Slot 0 always holds the current PC — it is never free.  That leaves
    // 7 slots available for nested CAL (call) instructions.
    //
    // Think of it as a stack of 8 plates: one plate is permanently on the
    // bottom (current PC).  You can stack 7 more; an 8th CAL slides the
    // bottom plate out, corrupting the return chain.
    //
    // The validator builds a static call graph from LABEL and CALL opcodes,
    // then runs a DFS to find the longest chain.  Recursive call graphs are
    // caught first and always rejected — the 8008 has no runtime overflow check.
    //
    // Pass 1 — cycle detection (DFS with gray/black coloring):
    //   gray  = node is on the current DFS path
    //   black = node has been fully explored
    //   If we see a gray node again, we found a back edge → cycle.
    //
    // Pass 2 — maximum depth DFS:
    //   Each branch gets its OWN copy of the visited set (like the Python
    //   `visited | {node}` pattern) so we measure the max depth across all
    //   root-to-leaf paths independently.

    fn check_call_depth(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        // ---- Build the call graph: label → [callee, ...] ----
        let mut call_graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut current_label: Option<String> = None;

        for instr in &program.instructions {
            match instr.opcode {
                IrOp::Label => {
                    if let Some(IrOperand::Label(name)) = instr.operands.first() {
                        current_label = Some(name.clone());
                        call_graph.entry(name.clone()).or_default();
                    }
                }
                IrOp::Call => {
                    if let (Some(ref caller), Some(IrOperand::Label(callee))) =
                        (&current_label, instr.operands.first())
                    {
                        call_graph
                            .entry(caller.clone())
                            .or_default()
                            .push(callee.clone());
                        call_graph.entry(callee.clone()).or_default();
                    }
                }
                _ => {}
            }
        }

        // ---- Cycle detection ----
        //
        // DFS with three-colour marking:
        //   white (absent from both sets) = not yet visited
        //   gray  (in `visiting`)         = on the current DFS path
        //   black (in `visited`)          = fully explored
        //
        // A back-edge (white→gray) signals a cycle.  We extract the cycle
        // from the current path for the error message.
        let all_labels: Vec<String> = call_graph.keys().cloned().collect();
        let mut visiting: HashSet<String> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for label in &all_labels {
            if let Some(cycle) = find_cycle(
                label,
                &call_graph,
                &mut visiting,
                &mut visited,
                &mut path,
            ) {
                let cycle_str = cycle.join(" -> ");
                return vec![ValidationDiagnostic {
                    rule: "call_depth".to_string(),
                    message: format!(
                        "Recursive call graphs are not supported on the Intel 8008 \
                         — the 8-level hardware stack wraps without overflow detection, \
                         silently corrupting return addresses.  \
                         Found cycle: {}.  \
                         Refactor the recursion into an iterative loop.",
                        cycle_str,
                    ),
                }];
            }
        }

        // ---- Maximum depth DFS ----
        //
        // Count call *edges* (not nodes) from the deepest-reaching label.
        // Depth 0 = a label that makes no calls; depth N = N nested CALs.
        // Each branch gets its own copy of the visited set so we correctly
        // measure the maximum depth across all paths (not just one path).
        let max_depth = all_labels.iter().map(|label| {
            let visited_copy = HashSet::new();
            dfs_depth(label, &call_graph, 0, &visited_copy)
        }).max().unwrap_or(0);

        if max_depth > MAX_CALL_DEPTH {
            return vec![ValidationDiagnostic {
                rule: "call_depth".to_string(),
                message: format!(
                    "Call graph depth {} exceeds the Intel 8008 hardware stack limit of \
                     {} nested calls (8-level push-down stack; level 0 = current PC).  \
                     Reduce nesting by inlining functions or restructuring the call graph.",
                    max_depth,
                    MAX_CALL_DEPTH,
                ),
            }];
        }

        vec![]
    }

    // -----------------------------------------------------------------------
    // Rule 4: Virtual register count ≤ 6
    // -----------------------------------------------------------------------
    //
    // The 8008 has 7 named registers: A, B, C, D, E, H, L.
    // Of these:
    //
    //   A      = accumulator — scratch, used implicitly by all ALU ops
    //   H, L   = 14-bit memory address register — reserved for LOAD_ADDR /
    //             LOAD_BYTE / STORE_BYTE; NOT user-addressable
    //   B, C, D, E = 4 user data registers, persistent across instructions
    //
    // The Oct calling convention maps 6 virtual registers (v0–v5) to physical
    // registers.  Any virtual register index ≥ 6 cannot be assigned a physical
    // home without register spilling to RAM, which the current code generator
    // does not support.

    fn check_register_count(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let mut seen: HashSet<usize> = HashSet::new();

        for instr in &program.instructions {
            for operand in &instr.operands {
                if let IrOperand::Register(idx) = operand {
                    seen.insert(*idx);
                }
            }
        }

        let count = seen.len();
        if count > MAX_VIRTUAL_REGISTERS {
            return vec![ValidationDiagnostic {
                rule: "register_count".to_string(),
                message: format!(
                    "Program uses {} distinct virtual registers but Intel 8008 supports \
                     at most {} (v0–v5 mapping to B, A, C, D, E and one spare; \
                     H and L are reserved for memory addressing).  \
                     Reduce local variable count or avoid functions with \
                     4 locals that call other functions.",
                    count,
                    MAX_VIRTUAL_REGISTERS,
                ),
            }];
        }
        vec![]
    }

    // -----------------------------------------------------------------------
    // Rule 5: Immediate values in LOAD_IMM / ADD_IMM ∈ [0, 255]
    // -----------------------------------------------------------------------
    //
    // All 8008 immediate instructions (MVI, ADI, ACI, SUI, ANI, XRI, ORI,
    // CPI) encode their literal operand in a single byte following the opcode.
    // This means the immediate must fit in 8 bits: 0–255.
    //
    //   LOAD_IMM lowers to:  MVI Rdst, imm
    //   ADD_IMM  lowers to:  MOV A, Ra;  ADI imm;  MOV Rdst, A
    //
    // A value of 256 requires two bytes — impossible in a single MVI/ADI.
    // Negative values are also out of range for the unsigned byte field.
    //
    // We report every out-of-range occurrence individually so the programmer
    // can fix them all at once.

    fn check_imm_range(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let mut errors = Vec::new();

        for instr in &program.instructions {
            if instr.opcode != IrOp::LoadImm && instr.opcode != IrOp::AddImm {
                continue;
            }
            for operand in &instr.operands {
                if let IrOperand::Immediate(val) = operand {
                    if *val < MIN_IMM || *val > MAX_IMM {
                        errors.push(ValidationDiagnostic {
                            rule: "imm_range".to_string(),
                            message: format!(
                                "{} immediate {} is out of range for Intel 8008.  \
                                 Valid range is [{}, {}] \
                                 (u8: one byte, matching MVI/ADI instruction format).  \
                                 Split large values or use a register load sequence.",
                                instr.opcode, val, MIN_IMM, MAX_IMM,
                            ),
                        });
                    }
                }
            }
        }
        errors
    }

    // -----------------------------------------------------------------------
    // Rule 6: SYSCALL numbers in the 8008 whitelist
    // -----------------------------------------------------------------------
    //
    // The Oct IR uses SYSCALL to represent hardware intrinsic operations.
    // Each SYSCALL number maps to a specific inline instruction sequence in
    // the 8008 code generator.  SYSCALL numbers outside the whitelist have
    // no defined 8008 assembly lowering and cannot be compiled.
    //
    // Whitelist (from OCT00 specification):
    //
    //    3  → adc(a, b)  ←  ADC r     (add with carry)
    //    4  → sbb(a, b)  ←  SBB r     (subtract with borrow)
    //   11  → rlc(a)     ←  RLC       (rotate left circular)
    //   12  → rrc(a)     ←  RRC       (rotate right circular)
    //   13  → ral(a)     ←  RAL       (rotate left through carry)
    //   14  → rar(a)     ←  RAR       (rotate right through carry)
    //   15  → carry()    ←  MVI A,0; ACI 0  (materialise carry flag)
    //   16  → parity(a)  ←  ORA A; JFP ...  (materialise parity flag)
    //  20–27 → in(p)     ←  IN p      (input port p, p ∈ 0–7)
    //  40–63 → out(p,v)  ←  OUT p     (output port p, p ∈ 0–23)
    //
    // Each unique invalid number is reported only once.

    fn check_syscall_whitelist(&self, program: &IrProgram) -> Vec<ValidationDiagnostic> {
        let whitelist = valid_syscalls();
        let mut errors = Vec::new();
        let mut seen_bad: HashSet<i64> = HashSet::new();

        for instr in &program.instructions {
            if instr.opcode != IrOp::Syscall {
                continue;
            }
            if let Some(IrOperand::Immediate(num)) = instr.operands.first() {
                if !whitelist.contains(num) && !seen_bad.contains(num) {
                    seen_bad.insert(*num);
                    errors.push(ValidationDiagnostic {
                        rule: "syscall_whitelist".to_string(),
                        message: format!(
                            "SYSCALL {} is not a valid Intel 8008 intrinsic.  \
                             Valid syscall numbers are: \
                             3–4 (adc/sbb), 11–16 (rotations/carry/parity), \
                             20–27 (in ports 0–7), 40–63 (out ports 0–23).  \
                             Check the Oct intrinsic call that produced SYSCALL {}.",
                            num, num,
                        ),
                    });
                }
            }
        }
        errors
    }
}

// ===========================================================================
// DFS helpers — used by check_call_depth
// ===========================================================================

/// Detect a cycle in `graph` using three-colour DFS.
///
/// Colors:
///   - **white** (absent from both sets): not yet visited
///   - **gray** (`visiting`): currently on the DFS path
///   - **black** (`visited`): fully explored
///
/// Returns the cycle path as `Some(vec![a, b, c, a])` if one is found,
/// or `None` if the graph is acyclic.
///
/// # Arguments
///
/// * `node`     — current node being explored
/// * `graph`    — adjacency list: label → [callee, ...]
/// * `visiting` — set of gray nodes (current DFS path)
/// * `visited`  — set of black nodes (fully explored)
/// * `path`     — the current DFS path for cycle extraction
fn find_cycle(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    // If node is gray, we found a back edge → cycle.
    if visiting.contains(node) {
        // Extract the cycle from `path`: find where the cycle starts.
        if let Some(start_idx) = path.iter().position(|n| n == node) {
            let mut cycle: Vec<String> = path[start_idx..].to_vec();
            cycle.push(node.to_string()); // close the loop: A → ... → A
            return Some(cycle);
        }
        return Some(vec![node.to_string(), node.to_string()]);
    }

    // If node is already fully explored, no cycle through this node.
    if visited.contains(node) {
        return None;
    }

    // Mark gray and push to path.
    visiting.insert(node.to_string());
    path.push(node.to_string());

    // Explore children.  Clone the list to avoid borrowing `graph` and
    // `visiting`/`visited`/`path` simultaneously.
    let children: Vec<String> = graph.get(node).cloned().unwrap_or_default();
    for child in &children {
        if let Some(cycle) = find_cycle(child, graph, visiting, visited, path) {
            return Some(cycle);
        }
    }

    // Mark black and pop from path.
    path.pop();
    visiting.remove(node);
    visited.insert(node.to_string());
    None
}

/// Compute the maximum call depth reachable from `node`.
///
/// Depth is the number of call *edges* traversed, not the number of nodes.
///   - Depth 0: `node` makes no calls.
///   - Depth N: there exists a chain of N nested CALs from `node`.
///
/// Each branch of the DFS receives its own copy of `visited` so we
/// correctly measure the maximum depth across all root-to-leaf paths
/// independently.  (If we shared `visited`, one branch would prune paths
/// that a sibling branch explores more deeply.)
///
/// # Arguments
///
/// * `node`    — current node
/// * `graph`   — adjacency list: label → [callee, ...]
/// * `depth`   — number of call edges traversed so far
/// * `visited` — set of nodes already on this path (prevents re-entry)
fn dfs_depth(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    depth: usize,
    visited: &HashSet<String>,
) -> usize {
    // If we've already visited this node on this path, stop to avoid loops.
    // (Cycles were caught earlier; this guard protects against shared-label
    // programs that reference the same callee from multiple call sites.)
    if visited.contains(node) {
        return depth;
    }

    // Create a new visited set for this node (copy-on-extend pattern,
    // matching the Python `visited | {node}` semantics).
    let mut new_visited = visited.clone();
    new_visited.insert(node.to_string());

    let children: Vec<String> = graph.get(node).cloned().unwrap_or_default();
    if children.is_empty() {
        return depth;
    }

    children.iter()
        .map(|child| dfs_depth(child, graph, depth + 1, &new_visited))
        .max()
        .unwrap_or(depth)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrDataDecl, IrInstruction, IrProgram};

    // Helper: build a minimal valid program with a HALT
    fn minimal_program() -> IrProgram {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        prog
    }

    // Helper: add a LABEL+CALL chain of `depth` levels starting from entry
    fn call_chain(depth: usize) -> IrProgram {
        // Creates: _start → fn1 → fn2 → ... → fn{depth}
        // Call depth in edges = `depth`.
        let mut prog = IrProgram::new("_start");

        // Emit label for _start
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        // Emit CALL to fn1 from _start
        if depth > 0 {
            prog.add_instruction(IrInstruction::new(
                IrOp::Call,
                vec![IrOperand::Label("fn1".to_string())],
                0,
            ));
        }

        // For depth d, emit fn{i} → fn{i+1} for i in 1..(d-1)
        for i in 1..depth {
            let name = format!("fn{}", i);
            prog.add_instruction(IrInstruction::new(
                IrOp::Label,
                vec![IrOperand::Label(name.clone())],
                -1,
            ));
            let callee = format!("fn{}", i + 1);
            prog.add_instruction(IrInstruction::new(
                IrOp::Call,
                vec![IrOperand::Label(callee)],
                i as i64,
            ));
        }

        // Emit the deepest function (no calls)
        if depth > 0 {
            let name = format!("fn{}", depth);
            prog.add_instruction(IrInstruction::new(
                IrOp::Label,
                vec![IrOperand::Label(name)],
                -1,
            ));
            prog.add_instruction(IrInstruction::new(IrOp::Ret, vec![], depth as i64));
        }

        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], (depth + 1) as i64));
        prog
    }

    // ------------------------------------------------------------------
    // Acceptance tests — valid programs must produce no errors
    // ------------------------------------------------------------------

    #[test]
    fn accepts_empty_program() {
        let prog = IrProgram::new("_start");
        assert!(IrValidator.validate(&prog).is_empty());
    }

    #[test]
    fn accepts_minimal_halt_program() {
        let prog = minimal_program();
        assert!(IrValidator.validate(&prog).is_empty());
    }

    #[test]
    fn accepts_load_imm_boundary_values() {
        // 0 and 255 are both valid u8 immediates
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(0)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(255)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        assert!(IrValidator.validate(&prog).is_empty());
    }

    #[test]
    fn accepts_add_imm_boundary_values() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(0),
                IrOperand::Immediate(1),
            ],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Immediate(128),
            ],
            1,
        ));
        assert!(IrValidator.validate(&prog).is_empty());
    }

    #[test]
    fn accepts_valid_syscall_numbers() {
        // All boundary SYSCALL numbers in the whitelist
        let valid_nums: Vec<i64> = vec![3, 4, 11, 16, 20, 27, 40, 63];
        for num in valid_nums {
            let mut prog = IrProgram::new("_start");
            prog.add_instruction(IrInstruction::new(
                IrOp::Syscall,
                vec![IrOperand::Immediate(num)],
                0,
            ));
            let errors = IrValidator.validate(&prog);
            assert!(
                errors.is_empty(),
                "SYSCALL {} should be valid but got: {:?}",
                num, errors
            );
        }
    }

    #[test]
    fn accepts_call_depth_at_limit() {
        // Exactly 7 nested calls — at the limit, must pass
        let prog = call_chain(7);
        let errors = IrValidator.validate(&prog);
        assert!(
            errors.is_empty(),
            "depth-7 chain should be valid but got: {:?}", errors
        );
    }

    #[test]
    fn accepts_max_register_count() {
        // Exactly 6 distinct registers (v0–v5) — at the limit
        let mut prog = IrProgram::new("_start");
        for i in 0..6 {
            prog.add_instruction(IrInstruction::new(
                IrOp::LoadImm,
                vec![IrOperand::Register(i), IrOperand::Immediate(i as i64)],
                i as i64,
            ));
        }
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 6));
        let errors = IrValidator.validate(&prog);
        assert!(
            errors.is_empty(),
            "6 registers should be valid but got: {:?}", errors
        );
    }

    #[test]
    fn accepts_ram_at_limit() {
        // Exactly 8 191 bytes — at the limit
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl {
            label: "buf".to_string(),
            size: 8191,
            init: 0,
        });
        assert!(IrValidator.validate(&prog).is_empty());
    }

    // ------------------------------------------------------------------
    // Rejection tests — invalid programs must produce specific errors
    // ------------------------------------------------------------------

    #[test]
    fn rejects_load_word() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "no_word_ops");
        assert!(errors[0].message.contains("LOAD_WORD"));
    }

    #[test]
    fn rejects_store_word() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::StoreWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "no_word_ops");
        assert!(errors[0].message.contains("STORE_WORD"));
    }

    #[test]
    fn rejects_both_word_ops_two_errors() {
        // Both LOAD_WORD and STORE_WORD → 2 distinct no_word_ops errors
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::StoreWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        let word_errors: Vec<_> = errors.iter().filter(|e| e.rule == "no_word_ops").collect();
        assert_eq!(word_errors.len(), 2);
    }

    #[test]
    fn rejects_ram_over_limit() {
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl {
            label: "big_buf".to_string(),
            size: 8192,
            init: 0,
        });
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "static_ram");
        assert!(errors[0].message.contains("8192 bytes"));
        assert!(errors[0].message.contains("1 bytes")); // must reduce by 1
    }

    #[test]
    fn rejects_ram_multiple_decls_over_limit() {
        let mut prog = IrProgram::new("_start");
        prog.add_data(IrDataDecl { label: "a".to_string(), size: 5000, init: 0 });
        prog.add_data(IrDataDecl { label: "b".to_string(), size: 4000, init: 0 });
        // 9 000 total > 8 191
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "static_ram");
    }

    #[test]
    fn rejects_load_imm_over_255() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(256)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "imm_range");
        assert!(errors[0].message.contains("256"));
        assert!(errors[0].message.contains("LOAD_IMM"));
    }

    #[test]
    fn rejects_load_imm_negative() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(-1)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "imm_range");
        assert!(errors[0].message.contains("-1"));
    }

    #[test]
    fn rejects_add_imm_over_255() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::AddImm,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(0),
                IrOperand::Immediate(300),
            ],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "imm_range");
        assert!(errors[0].message.contains("300"));
        assert!(errors[0].message.contains("ADD_IMM"));
    }

    #[test]
    fn rejects_too_many_registers() {
        // 7 distinct registers (v0–v6) — one over the limit
        let mut prog = IrProgram::new("_start");
        for i in 0..7 {
            prog.add_instruction(IrInstruction::new(
                IrOp::LoadImm,
                vec![IrOperand::Register(i), IrOperand::Immediate(0)],
                i as i64,
            ));
        }
        let errors = IrValidator.validate(&prog);
        let reg_errors: Vec<_> = errors.iter().filter(|e| e.rule == "register_count").collect();
        assert_eq!(reg_errors.len(), 1);
        assert!(reg_errors[0].message.contains('7'));
    }

    #[test]
    fn rejects_deep_call_graph() {
        // 8 nested calls — one over the limit of 7
        let prog = call_chain(8);
        let errors = IrValidator.validate(&prog);
        let depth_errors: Vec<_> = errors.iter().filter(|e| e.rule == "call_depth").collect();
        assert_eq!(depth_errors.len(), 1);
        assert!(depth_errors[0].message.contains('8'));
    }

    #[test]
    fn rejects_recursive_call_graph() {
        // a → b → a  (cycle — always rejected on 8008)
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("a".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Call,
            vec![IrOperand::Label("b".to_string())],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("b".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Call,
            vec![IrOperand::Label("a".to_string())],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        let depth_errors: Vec<_> = errors.iter().filter(|e| e.rule == "call_depth").collect();
        assert_eq!(depth_errors.len(), 1);
        assert!(depth_errors[0].message.contains("cycle") || depth_errors[0].message.contains("Recursive"));
    }

    #[test]
    fn rejects_self_recursive_call() {
        // foo → foo  (self-recursion)
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("foo".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Call,
            vec![IrOperand::Label("foo".to_string())],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        let depth_errors: Vec<_> = errors.iter().filter(|e| e.rule == "call_depth").collect();
        assert_eq!(depth_errors.len(), 1);
    }

    #[test]
    fn rejects_invalid_syscall() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "syscall_whitelist");
        assert!(errors[0].message.contains("99"));
    }

    #[test]
    fn rejects_syscall_5_invalid() {
        // 5 is not in the whitelist (3,4 are; 6–10 are not)
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(5)],
            0,
        ));
        let errors = IrValidator.validate(&prog);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule, "syscall_whitelist");
    }

    #[test]
    fn rejects_multiple_invalid_syscalls_reported_individually() {
        // SYSCALL 5 and SYSCALL 10 — both invalid, reported separately
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(5)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(10)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        let sc_errors: Vec<_> = errors.iter().filter(|e| e.rule == "syscall_whitelist").collect();
        assert_eq!(sc_errors.len(), 2);
    }

    #[test]
    fn invalid_syscall_deduplicated() {
        // Same invalid number appearing twice — report only once
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            1,
        ));
        let errors = IrValidator.validate(&prog);
        let sc_errors: Vec<_> = errors.iter().filter(|e| e.rule == "syscall_whitelist").collect();
        assert_eq!(sc_errors.len(), 1, "duplicate invalid SYSCALL should be reported only once");
    }

    #[test]
    fn multiple_errors_accumulated_in_one_pass() {
        // LOAD_WORD + bad imm + bad syscall — all reported together
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadWord,
            vec![IrOperand::Register(0), IrOperand::Register(1), IrOperand::Register(2)],
            0,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(300)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            2,
        ));
        let errors = IrValidator.validate(&prog);
        assert!(errors.len() >= 3, "expected ≥3 errors, got: {:?}", errors);
    }

    #[test]
    fn validation_diagnostic_display_format() {
        let d = ValidationDiagnostic {
            rule: "imm_range".to_string(),
            message: "test message".to_string(),
        };
        assert_eq!(d.to_string(), "[imm_range] test message");
    }

    #[test]
    fn validation_diagnostic_equality() {
        let a = ValidationDiagnostic { rule: "r".to_string(), message: "m".to_string() };
        let b = ValidationDiagnostic { rule: "r".to_string(), message: "m".to_string() };
        let c = ValidationDiagnostic { rule: "r".to_string(), message: "other".to_string() };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn call_depth_zero_no_calls() {
        // A program with labels but no CALL instructions has depth 0 → valid
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 0));
        assert!(IrValidator.validate(&prog).is_empty());
    }
}

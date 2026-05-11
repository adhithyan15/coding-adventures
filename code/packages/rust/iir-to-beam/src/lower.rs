//! IIR → BEAM lowering pass.
//!
//! This module converts an [`interpreter_ir::IIRModule`] into a
//! [`ir_to_beam::BEAMModule`] that can then be serialized by
//! [`ir_to_beam::encode_beam`].
//!
//! # BEAM execution model (mini-primer)
//!
//! BEAM (Bogdan/Björn Erlang Abstract Machine) is a **register machine**.
//! Each function body has a set of **x-registers** (x0, x1, x2, …) used
//! for arguments and temporaries, plus a separate y-register stack for
//! callee-saved values (not used here).
//!
//! Unlike WASM or JVM (which are stack machines), BEAM instructions name
//! their source and destination registers explicitly:
//!
//! ```text
//! move   {i,42}   {x,0}          % load integer 42 into x0
//! gc_bif2 erlang:+/2 … {x,0} {x,1} {x,2}  % x2 = x0 + x1
//! return                         % return value is in x0
//! ```
//!
//! Arithmetic is performed via **BIFs** (Built-In Functions).  `gc_bif2`
//! calls a two-argument BIF and may trigger garbage collection
//! (hence the `live` operand that tells the GC how many x-registers are
//! live at the call site).
//!
//! # Function layout
//!
//! Every function in BEAM bytecode follows a fixed preamble:
//!
//! ```text
//! {label, L_fi}.                          ← func_info header label
//! {func_info, {a,Module}, {a,Name}, Arity}.
//! {label, L_entry}.                       ← this is what ExpT exports
//!   … translated IIR instructions …
//! ```
//!
//! The export table (`ExpT`) records `(function_atom, arity, L_entry)` so the
//! BEAM loader knows which label to jump to when calling the function.
//!
//! # Register allocation strategy
//!
//! IIR uses named SSA variables (`"a"`, `"result"`, …).  BEAM uses numbered
//! x-registers.  This pass uses the simplest possible allocation:
//!
//! 1. Function parameters are assigned x0, x1, x2, … in order.
//! 2. Every distinct variable name encountered (in `dest` or `Var` srcs) gets
//!    the next available register.  The same name always maps to the same
//!    register within one function.
//!
//! This is not optimal (it may allocate registers for variables that are
//! dead), but it is correct and simple enough for a v1 backend.  A register
//! allocator with liveness analysis can be added later without changing the
//! interface.
//!
//! # Label numbering
//!
//! A single global counter increments across all functions.  Each function
//! consumes two labels for its preamble (func_info label + entry label), then
//! one per `label` IIR instruction, plus one synthetic label per comparison
//! and per conditional branch instruction.
//!
//! Reserving a globally-unique counter is required by the BEAM loader:
//! label numbers must be unique across the entire code section, not just
//! within one function.
//!
//! # Comparison synthesis
//!
//! IIR comparison operators (`cmp_eq`, `cmp_lt`, …) produce a boolean result
//! stored in a register.  BEAM has no "compare and store boolean" instruction —
//! it only has conditional branch instructions (`is_eq_exact`, `is_lt`, …).
//!
//! We synthesize a boolean with three instructions + one synthetic label:
//!
//! ```text
//! move  {i,0}  {x,rd}          % assume false
//! is_eq_exact {f,synth}  {x,a}  {x,b}   % branch to synth if a != b (false)
//! move  {i,1}  {x,rd}          % actually equal: overwrite with true
//! label {u,synth}              % both paths converge here
//! ```
//!
//! This is the standard pattern in BEAM compilers for boolean-producing
//! comparisons.

use std::collections::HashMap;

use interpreter_ir::{IIRModule, Operand};
use ir_to_beam::encoder::{
    BEAMExport, BEAMImport, BEAMInstruction, BEAMModule, BEAMOperand,
};

use crate::validate::validate_for_beam;

// ===========================================================================
// BEAM opcode constants
// ===========================================================================
//
// These are the single-byte opcode numbers defined in the BEAM instruction
// set (OTP beam_opcodes.h).  We only use a subset here.

/// `{label, {u,N}}` — defines label N at this position in the code stream.
const OP_LABEL: u8 = 1;
/// `{func_info, {a,Mod}, {a,Fun}, {u,Arity}}` — function entry metadata.
/// The BEAM loader uses this to build the function-info table for stack traces.
const OP_FUNC_INFO: u8 = 2;
/// `{int_code_end}` — end of the code section.
/// Must be the very last instruction in the entire code stream.
const OP_INT_CODE_END: u8 = 3;
/// `{call, {u,Arity}, {f,Label}}` — local function call.
/// Saves the return address on the call stack and jumps to Label.
const OP_CALL: u8 = 4;
/// `{return}` — return from the current function.
/// The return value must already be in x0.
const OP_RETURN: u8 = 19;
/// `{jump, {f,Label}}` — unconditional branch to Label.
const OP_JUMP: u8 = 36;
/// `{is_eq_exact, {f,Fail}, A, B}` — fall through if A == B; branch to Fail if not.
/// Note: the branch is taken on *failure* (when not equal), not on success.
const OP_IS_EQ_EXACT: u8 = 43;
/// `{is_ne_exact, {f,Fail}, A, B}` — fall through if A != B; branch to Fail if not.
const OP_IS_NE_EXACT: u8 = 44;
/// `{is_lt, {f,Fail}, A, B}` — fall through if A < B; branch to Fail if not.
/// That is: branch to Fail when A >= B.
const OP_IS_LT: u8 = 47;
/// `{is_ge, {f,Fail}, A, B}` — fall through if A >= B; branch to Fail if not.
/// That is: branch to Fail when A < B.
const OP_IS_GE: u8 = 48;
/// `{move, Src, Dst}` — copy one register or immediate into another.
const OP_MOVE: u8 = 64;
/// `{gc_bif1, {f,Fail}, {u,Live}, Bif, Arg, Dst}` — one-argument BIF call.
/// `{f,0}` as Fail means "no handler; raise exception on error".
const OP_GC_BIF1: u8 = 124;
/// `{gc_bif2, {f,Fail}, {u,Live}, Bif, Arg1, Arg2, Dst}` — two-argument BIF call.
const OP_GC_BIF2: u8 = 125;

// ===========================================================================
// IIRBeamConfig
// ===========================================================================

/// Configuration for the IIR → BEAM lowering pass.
///
/// Currently only the BEAM module name is configurable.  Erlang module names
/// are atoms — they must be lowercase identifiers, e.g. `"myapp"`, `"calc"`.
#[derive(Debug, Clone)]
pub struct IIRBeamConfig {
    /// The Erlang module name (atom).  Must not be empty.
    pub module_name: String,
}

impl Default for IIRBeamConfig {
    fn default() -> Self {
        Self { module_name: "iir_module".to_string() }
    }
}

impl IIRBeamConfig {
    /// Create a config with the given module name.
    ///
    /// # Example
    /// ```
    /// use iir_to_beam::IIRBeamConfig;
    /// let cfg = IIRBeamConfig::new("myapp");
    /// assert_eq!(cfg.module_name, "myapp");
    /// ```
    pub fn new(module_name: impl Into<String>) -> Self {
        Self { module_name: module_name.into() }
    }
}

// ===========================================================================
// IIRBeamError
// ===========================================================================

/// Errors that can occur during IIR → BEAM lowering.
#[derive(Debug)]
pub enum IIRBeamError {
    /// The module failed pre-flight validation (see [`validate_for_beam`]).
    ValidationFailed(Vec<String>),
    /// An IIR opcode that is not supported by this BEAM backend.
    UnsupportedOp { function: String, op: String },
    /// An instruction has a type that cannot be lowered.
    UnsupportedType { function: String, type_hint: String },
    /// A branch targets a label name that has no definition in the function.
    UndefinedLabel { function: String, label: String },
    /// A `Var` operand references a variable name that was never defined.
    UndefinedVariable { function: String, name: String },
    /// An operand has an unexpected shape (e.g. Bool where Int was expected).
    InvalidOperand { function: String, detail: String },
}

impl std::fmt::Display for IIRBeamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "function {function:?}: unsupported op {op:?}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(f, "function {function:?}: unsupported type {type_hint:?}")
            }
            Self::UndefinedLabel { function, label } => {
                write!(f, "function {function:?}: undefined label {label:?}")
            }
            Self::UndefinedVariable { function, name } => {
                write!(f, "function {function:?}: undefined variable {name:?}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "function {function:?}: invalid operand: {detail}")
            }
        }
    }
}

impl std::error::Error for IIRBeamError {}

// ===========================================================================
// _AtomTable — insertion-ordered, 1-based
// ===========================================================================

/// Maintains the BEAM atom table with insertion-order, 1-based indices.
///
/// BEAM atoms are 1-based: the first atom interned has index 1 (index 0 is
/// the atom `nil` in some OTP implementations, but in this file format the
/// first explicitly declared atom has index 1).  Interning the same string
/// twice always returns the same index.
///
/// The module name MUST be the first atom interned (index 1) — the BEAM
/// loader identifies a module by its first atom-table entry.
struct AtomTable {
    /// Ordered list of atom strings (0-indexed internally; 1-based in BEAM).
    atoms: Vec<String>,
    /// Map from atom string to its 1-based index.
    index: HashMap<String, u32>,
}

impl AtomTable {
    fn new() -> Self {
        Self { atoms: Vec::new(), index: HashMap::new() }
    }

    /// Intern `atom`, returning its 1-based BEAM index.
    /// If the atom is already present, returns the existing index.
    fn intern(&mut self, atom: &str) -> u32 {
        if let Some(&idx) = self.index.get(atom) {
            return idx;
        }
        self.atoms.push(atom.to_string());
        // 1-based index: first atom → 1, second → 2, …
        let idx = self.atoms.len() as u32;
        self.index.insert(atom.to_string(), idx);
        idx
    }

    /// Return all atoms in insertion order (used to build BEAMModule.atoms).
    fn all(&self) -> &[String] {
        &self.atoms
    }
}

// ===========================================================================
// _ImportTable — deduplicating, 0-based indices
// ===========================================================================

/// Maintains the BEAM import table (ImpT chunk) with deduplication.
///
/// Each import is a `(module_atom_idx, function_atom_idx, arity)` triple.
/// The gc_bif1/gc_bif2 instructions reference imports by their **0-based**
/// index into this table via an `{a, import_idx}` operand.
///
/// Why 0-based? The `gc_bif` operand tag is `A` (atom), but its value is
/// the import-table slot index, not an atom table index.  BEAM documentation
/// is inconsistent here; in practice, Erlang/OTP uses 0-based import indices
/// in `gc_bif` instructions.
struct ImportTable {
    imports: Vec<BEAMImport>,
    /// Map from (module_idx, fn_idx, arity) to 0-based slot index.
    index: HashMap<(u32, u32, u32), u32>,
}

impl ImportTable {
    fn new() -> Self {
        Self { imports: Vec::new(), index: HashMap::new() }
    }

    /// Intern an import, returning its 0-based slot index.
    fn intern(&mut self, module_idx: u32, fn_idx: u32, arity: u32) -> u32 {
        let key = (module_idx, fn_idx, arity);
        if let Some(&idx) = self.index.get(&key) {
            return idx;
        }
        let idx = self.imports.len() as u32;
        self.imports.push(BEAMImport {
            module_atom_index: module_idx,
            function_atom_index: fn_idx,
            arity,
        });
        self.index.insert(key, idx);
        idx
    }
}

// ===========================================================================
// lower_iir_to_beam
// ===========================================================================

/// Lower an `IIRModule` to a `BEAMModule` ready for encoding.
///
/// This is the main entry point for the IIR BEAM backend.
///
/// # Algorithm
///
/// 1. Validate the module; return `Err(ValidationFailed)` on errors.
/// 2. Build the atom table: module name first (index 1), then intern names
///    on demand as functions, parameters, and BIF names are encountered.
/// 3. For each function, run **Pass 1**: assign x-registers to all variable
///    names (params first, then dests/var-srcs in instruction order).  Also
///    scan for `label` instructions and assign globally-unique BEAM label
///    numbers.
/// 4. For each function, run **Pass 2**: emit the func_info preamble and
///    then translate each IIR instruction to BEAM instructions.
/// 5. Append `INT_CODE_END`.
/// 6. Build the `BEAMModule` with the assembled instruction stream, atom
///    table, import table, and export table.
///
/// # Errors
///
/// Returns [`IIRBeamError::ValidationFailed`] if validation fails.
/// Other error variants are returned for malformed instruction operands or
/// undefined variable/label references (which should not occur for well-
/// formed IIR produced by a correct frontend).
pub fn lower_iir_to_beam(
    module: &IIRModule,
    config: &IIRBeamConfig,
) -> Result<BEAMModule, IIRBeamError> {
    // ── Step 1: pre-flight validation ──────────────────────────────────────
    let errs = validate_for_beam(module);
    if !errs.is_empty() {
        return Err(IIRBeamError::ValidationFailed(errs));
    }

    // ── Step 2: atom table ─────────────────────────────────────────────────
    //
    // The module name MUST be the very first atom (index 1).  The BEAM loader
    // identifies a module by its first atom-table entry.
    let mut atoms = AtomTable::new();
    let mod_atom = atoms.intern(&config.module_name);
    // "erlang" is the module that owns all arithmetic BIFs.
    let erlang_atom = atoms.intern("erlang");

    // Intern the BIF names.  We do this upfront so that even if no arithmetic
    // is used, the atoms are in the table (they cost nothing).
    let atom_plus   = atoms.intern("+");
    let atom_minus  = atoms.intern("-");
    let atom_times  = atoms.intern("*");
    let atom_div    = atoms.intern("div");
    let atom_rem    = atoms.intern("rem");
    let atom_band   = atoms.intern("band");
    let atom_bor    = atoms.intern("bor");
    let atom_bxor   = atoms.intern("bxor");
    let atom_bnot   = atoms.intern("bnot");
    let atom_bsl    = atoms.intern("bsl");
    let atom_bsr    = atoms.intern("bsr");

    // ── Step 3: import table ───────────────────────────────────────────────
    let mut imports = ImportTable::new();

    // Pre-register all arithmetic BIFs so their import indices are stable.
    // Even if a specific BIF is never called, pre-registering is harmless —
    // it only adds entries to the ImpT chunk, which the loader silently ignores
    // if they are unreferenced.
    let import_add  = imports.intern(erlang_atom, atom_plus,  2); // erlang:+/2
    let import_sub  = imports.intern(erlang_atom, atom_minus, 2); // erlang:-/2
    let import_mul  = imports.intern(erlang_atom, atom_times, 2); // erlang:*/2
    let import_div  = imports.intern(erlang_atom, atom_div,   2); // erlang:div/2
    let import_rem  = imports.intern(erlang_atom, atom_rem,   2); // erlang:rem/2
    let import_neg  = imports.intern(erlang_atom, atom_minus, 1); // erlang:-/1
    let import_and  = imports.intern(erlang_atom, atom_band,  2); // erlang:band/2
    let import_or   = imports.intern(erlang_atom, atom_bor,   2); // erlang:bor/2
    let import_xor  = imports.intern(erlang_atom, atom_bxor,  2); // erlang:bxor/2
    let import_not  = imports.intern(erlang_atom, atom_bnot,  1); // erlang:bnot/1
    let import_shl  = imports.intern(erlang_atom, atom_bsl,   2); // erlang:bsl/2
    let import_shr  = imports.intern(erlang_atom, atom_bsr,   2); // erlang:bsr/2

    // ── Step 4: first pass over all functions ─────────────────────────────
    //
    // We need to know:
    //   a) Each function's entry label (so `call fn_name` can reference it).
    //   b) Each function's register map (so `call fn_name, args…` knows which
    //      registers the arguments are in).
    //
    // Both require a scan of the function before emitting any instructions.

    // Global label counter.  Starts at 1 and increments across all functions.
    // Each function reserves two labels for its preamble (func_info + entry),
    // then one per IIR `label` instruction + one per synthetic label.
    let mut label_counter: u32 = 0;

    // Per-function metadata collected in the first pass.
    struct FnMeta {
        fn_atom: u32,      // BEAM atom index for the function name
        arity: u32,        // number of parameters
        entry_label: u32,  // BEAM label to call/export
        reg_map: HashMap<String, u8>, // variable name → x-register index
        iir_label_map: HashMap<String, u32>, // IIR label name → BEAM label number
        next_reg: u8,      // first register after all assigned ones
    }

    let mut fn_metas: Vec<FnMeta> = Vec::with_capacity(module.functions.len());

    for func in &module.functions {
        let fn_atom = atoms.intern(&func.name);

        // Assign the func_info label and entry label.
        label_counter += 1;
        let _fi_label = label_counter; // used for {label, L_fi}
        label_counter += 1;
        let entry_label = label_counter; // used for {label, L_entry} and ExpT

        // Build the register map for this function.
        //
        // Erlang calling convention: the first `arity` x-registers hold the
        // function arguments when the function is entered.  We assign x0, x1,
        // … to the parameters first so that callers can move arguments into
        // the correct registers before calling.
        let mut reg_map: HashMap<String, u8> = HashMap::new();
        let mut next_reg: u8 = 0;

        // Parameters → x0, x1, …
        for (param_name, _param_type) in &func.params {
            reg_map.insert(param_name.clone(), next_reg);
            next_reg += 1;
        }

        // Walk instruction dests and Var sources to assign registers for all
        // SSA variables.  We do this in instruction order so that the register
        // number is predictable (useful for debugging).
        //
        // Note: we also need to allocate registers for variables referenced as
        // Var sources before they appear as dests (e.g. `load_reg v → rd`).
        for instr in &func.instructions {
            // Source Var operands
            for src in &instr.srcs {
                if let Operand::Var(name) = src {
                    if !reg_map.contains_key(name.as_str()) {
                        reg_map.insert(name.clone(), next_reg);
                        next_reg += 1;
                    }
                }
            }
            // Destination
            if let Some(dest) = &instr.dest {
                if !reg_map.contains_key(dest.as_str()) {
                    reg_map.insert(dest.clone(), next_reg);
                    next_reg += 1;
                }
            }
        }

        // Build the label map: scan for `label` instructions and assign
        // globally-unique BEAM label numbers.
        let mut iir_label_map: HashMap<String, u32> = HashMap::new();
        for instr in &func.instructions {
            if instr.op == "label" {
                if let Some(Operand::Var(name)) = instr.srcs.first() {
                    if !iir_label_map.contains_key(name.as_str()) {
                        label_counter += 1;
                        iir_label_map.insert(name.clone(), label_counter);
                    }
                }
            }
        }

        fn_metas.push(FnMeta {
            fn_atom,
            arity: func.params.len() as u32,
            entry_label,
            reg_map,
            iir_label_map,
            next_reg,
        });
    }

    // ── Step 5: second pass — emit BEAM instructions ──────────────────────

    // Build a map from function name to entry label for `call` instructions.
    let fn_entry_map: HashMap<&str, u32> = module.functions.iter()
        .zip(fn_metas.iter())
        .map(|(f, meta)| (f.name.as_str(), meta.entry_label))
        .collect();

    let mut instrs: Vec<BEAMInstruction> = Vec::new();
    let mut exports: Vec<BEAMExport> = Vec::new();

    // Reset the label counter for the preamble labels (we already counted
    // them above; now we re-emit them in order).
    //
    // Implementation note: we stored the entry_label in fn_metas, so we
    // derive the fi_label as entry_label - 1.
    for (func, meta) in module.functions.iter().zip(fn_metas.iter()) {
        let fi_label = meta.entry_label - 1;

        // ── Preamble: {label, L_fi}; {func_info, Mod, Fun, Arity}; {label, L_entry}
        //
        // This triple is required by the BEAM loader.  `func_info` provides
        // the human-readable function signature used in stack traces and error
        // messages.  `L_fi` is not exported; `L_entry` is.
        instrs.push(BEAMInstruction::new(OP_LABEL, vec![BEAMOperand::u(fi_label as u64)]));
        instrs.push(BEAMInstruction::new(OP_FUNC_INFO, vec![
            BEAMOperand::a(mod_atom),
            BEAMOperand::a(meta.fn_atom),
            BEAMOperand::u(meta.arity as u64),
        ]));
        instrs.push(BEAMInstruction::new(OP_LABEL, vec![BEAMOperand::u(meta.entry_label as u64)]));

        // The function's entry label is what callers jump to and what the
        // export table points at.
        exports.push(BEAMExport {
            function_atom_index: meta.fn_atom,
            arity: meta.arity,
            label: meta.entry_label,
        });

        // ── Translate each IIR instruction ──────────────────────────────────

        // `live` = number of live x-registers at gc_bif call sites.
        // We conservatively use next_reg (all assigned registers may be live).
        let live = meta.next_reg as u64;

        // Synthetic label counter for comparisons and conditional branches.
        // These labels are generated on demand and must be globally unique.
        // We'll increment the outer label_counter inside the loop.
        // (We can't borrow `label_counter` mutably inside a closure here,
        //  so we use a raw counter variable.)
        let reg_map = &meta.reg_map;
        let iir_label_map = &meta.iir_label_map;
        let fn_name = &func.name;

        // Helper closure: look up a variable register or return an error.
        // (We define this as a macro-like helper to avoid borrow issues.)
        macro_rules! var_reg {
            ($name:expr) => {{
                match reg_map.get($name.as_str()) {
                    Some(&r) => r,
                    None => return Err(IIRBeamError::UndefinedVariable {
                        function: fn_name.clone(),
                        name: $name.to_string(),
                    }),
                }
            }};
        }

        // Helper: resolve an operand to a BEAM x-register.
        macro_rules! operand_reg {
            ($op:expr) => {{
                match $op {
                    Operand::Var(name) => var_reg!(name),
                    _ => return Err(IIRBeamError::InvalidOperand {
                        function: fn_name.clone(),
                        detail: format!(
                            "expected variable operand, got {:?}", $op
                        ),
                    }),
                }
            }};
        }

        // Helper: resolve an IIR label name to a BEAM label number.
        // We use `&*$name` to coerce both `String` and `&str` to `&str`,
        // avoiding the unstable `str::as_str()` method (stabilized after 1.94).
        macro_rules! resolve_label {
            ($name:expr) => {{
                match iir_label_map.get(&*$name) {
                    Some(&lbl) => lbl,
                    None => return Err(IIRBeamError::UndefinedLabel {
                        function: fn_name.clone(),
                        label: $name.to_string(),
                    }),
                }
            }};
        }

        for instr in &func.instructions {
            match instr.op.as_str() {

                // ── const Int / Bool ────────────────────────────────────────
                //
                // IIR `const` loads an immediate value into a destination
                // register.  BEAM uses `move {i,value} {x,rd}`.
                //
                // Booleans are represented as integers: false = 0, true = 1.
                // This matches Erlang's conventional "success flag" idiom and
                // makes subsequent is_eq_exact/is_ne_exact tests simple.
                "const" => {
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "const instruction must have a dest".into(),
                        }),
                    };
                    let value: u64 = match instr.srcs.first() {
                        Some(Operand::Int(n)) => *n as u64,
                        Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
                        Some(other) => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!(
                                "const operand must be Int or Bool, got {:?}", other
                            ),
                        }),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "const instruction has no source operand".into(),
                        }),
                    };
                    instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                        BEAMOperand::i(value),   // {i,val} — integer immediate
                        BEAMOperand::x(rd),      // {x,rd}  — destination register
                    ]));
                }

                // ── Binary arithmetic: add, sub, mul, div, mod ──────────────
                //
                // Pattern: gc_bif2 {f,0} {u,live} {a,import_idx} {x,r1} {x,r2} {x,rd}
                //
                // {f,0}         = no explicit failure label; let BEAM raise badarith.
                // {u,live}      = number of live registers (for GC root scanning).
                // {a,import_idx}= index into the import table (0-based).
                // {x,r1},{x,r2} = source registers.
                // {x,rd}        = destination register.
                "add" | "sub" | "mul" | "div" | "mod" => {
                    let import_idx = match instr.op.as_str() {
                        "add" => import_add,
                        "sub" => import_sub,
                        "mul" => import_mul,
                        "div" => import_div,
                        "mod" => import_rem,
                        _ => unreachable!(),
                    };
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }),
                    };
                    let r1 = operand_reg!(&instr.srcs[0]);
                    let r2 = operand_reg!(&instr.srcs[1]);
                    instrs.push(BEAMInstruction::new(OP_GC_BIF2, vec![
                        BEAMOperand::f(0),
                        BEAMOperand::u(live),
                        BEAMOperand::a(import_idx),
                        BEAMOperand::x(r1),
                        BEAMOperand::x(r2),
                        BEAMOperand::x(rd),
                    ]));
                }

                // ── Unary arithmetic: neg, not ──────────────────────────────
                //
                // Pattern: gc_bif1 {f,0} {u,live} {a,import_idx} {x,r} {x,rd}
                //
                // gc_bif1 is the same as gc_bif2 but with only one source
                // register.  `erlang:-/1` is unary minus; `erlang:bnot/1` is
                // bitwise NOT.
                "neg" | "not" => {
                    let import_idx = match instr.op.as_str() {
                        "neg" => import_neg,
                        "not" => import_not,
                        _ => unreachable!(),
                    };
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }),
                    };
                    let r = operand_reg!(&instr.srcs[0]);
                    instrs.push(BEAMInstruction::new(OP_GC_BIF1, vec![
                        BEAMOperand::f(0),
                        BEAMOperand::u(live),
                        BEAMOperand::a(import_idx),
                        BEAMOperand::x(r),
                        BEAMOperand::x(rd),
                    ]));
                }

                // ── Binary bitwise: and, or, xor, shl, shr ─────────────────
                "and" | "or" | "xor" | "shl" | "shr" => {
                    let import_idx = match instr.op.as_str() {
                        "and" => import_and,
                        "or"  => import_or,
                        "xor" => import_xor,
                        "shl" => import_shl,
                        "shr" => import_shr,
                        _ => unreachable!(),
                    };
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }),
                    };
                    let r1 = operand_reg!(&instr.srcs[0]);
                    let r2 = operand_reg!(&instr.srcs[1]);
                    instrs.push(BEAMInstruction::new(OP_GC_BIF2, vec![
                        BEAMOperand::f(0),
                        BEAMOperand::u(live),
                        BEAMOperand::a(import_idx),
                        BEAMOperand::x(r1),
                        BEAMOperand::x(r2),
                        BEAMOperand::x(rd),
                    ]));
                }

                // ── Comparisons: cmp_eq, cmp_ne, cmp_lt, cmp_le, cmp_gt, cmp_ge
                //
                // BEAM has no "compare and produce boolean" instruction.  We
                // synthesize it with the three-instruction pattern:
                //
                //   move {i,0} {x,rd}                     ← assume false
                //   <conditional_branch> {f,synth} A B    ← branch if cond is FALSE
                //   move {i,1} {x,rd}                     ← condition was TRUE: set 1
                //   label {u,synth}                       ← both paths meet here
                //
                // The conditional branch has inverted semantics: it jumps to
                // `synth` when the condition FAILS.  Since we want `rd=1` when
                // the condition holds, we let the fall-through do the overwrite.
                //
                // For cmp_gt/cmp_le/cmp_ge, we swap operands to reuse is_lt/is_ge.
                "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }),
                    };
                    let r1 = operand_reg!(&instr.srcs[0]);
                    let r2 = operand_reg!(&instr.srcs[1]);

                    // Allocate a synthetic label for the false branch to converge.
                    label_counter += 1;
                    let synth = label_counter;

                    // Step A: pre-load 0 (false) into rd.
                    instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                        BEAMOperand::i(0),
                        BEAMOperand::x(rd),
                    ]));

                    // Step B: conditional branch.
                    //   The branch operand is {f,synth}: jump to synth when
                    //   the condition is NOT satisfied.
                    match instr.op.as_str() {
                        "cmp_eq" => {
                            // is_eq_exact {f,synth} {x,r1} {x,r2}
                            // → falls through if r1 == r2, branches if r1 != r2
                            instrs.push(BEAMInstruction::new(OP_IS_EQ_EXACT, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r1),
                                BEAMOperand::x(r2),
                            ]));
                        }
                        "cmp_ne" => {
                            // is_ne_exact {f,synth} {x,r1} {x,r2}
                            // → falls through if r1 != r2, branches if r1 == r2
                            instrs.push(BEAMInstruction::new(OP_IS_NE_EXACT, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r1),
                                BEAMOperand::x(r2),
                            ]));
                        }
                        "cmp_lt" => {
                            // is_lt {f,synth} {x,r1} {x,r2}
                            // → falls through if r1 < r2, branches if r1 >= r2
                            instrs.push(BEAMInstruction::new(OP_IS_LT, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r1),
                                BEAMOperand::x(r2),
                            ]));
                        }
                        "cmp_le" => {
                            // We want: true when r1 <= r2, i.e. r2 >= r1.
                            // is_ge {f,synth} {x,r2} {x,r1}
                            // → falls through if r2 >= r1, branches if r2 < r1
                            instrs.push(BEAMInstruction::new(OP_IS_GE, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r2),  // swapped: r2 >= r1 ⟺ r1 <= r2
                                BEAMOperand::x(r1),
                            ]));
                        }
                        "cmp_gt" => {
                            // We want: true when r1 > r2, i.e. r2 < r1.
                            // is_lt {f,synth} {x,r2} {x,r1}
                            // → falls through if r2 < r1, branches if r2 >= r1
                            instrs.push(BEAMInstruction::new(OP_IS_LT, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r2),  // swapped: r2 < r1 ⟺ r1 > r2
                                BEAMOperand::x(r1),
                            ]));
                        }
                        "cmp_ge" => {
                            // is_ge {f,synth} {x,r1} {x,r2}
                            // → falls through if r1 >= r2, branches if r1 < r2
                            instrs.push(BEAMInstruction::new(OP_IS_GE, vec![
                                BEAMOperand::f(synth),
                                BEAMOperand::x(r1),
                                BEAMOperand::x(r2),
                            ]));
                        }
                        _ => unreachable!(),
                    }

                    // Step C: fall-through path — condition was satisfied.
                    instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                        BEAMOperand::i(1),
                        BEAMOperand::x(rd),
                    ]));

                    // Step D: convergence label — both paths meet here.
                    instrs.push(BEAMInstruction::new(
                        OP_LABEL, vec![BEAMOperand::u(synth as u64)],
                    ));
                }

                // ── label ───────────────────────────────────────────────────
                //
                // An IIR `label` instruction marks a target for branches.
                // We look up the pre-assigned BEAM label number and emit
                // {label, {u,N}}.
                "label" => {
                    if let Some(Operand::Var(name)) = instr.srcs.first() {
                        let lbl = resolve_label!(name);
                        instrs.push(BEAMInstruction::new(
                            OP_LABEL, vec![BEAMOperand::u(lbl as u64)],
                        ));
                    }
                }

                // ── jmp ─────────────────────────────────────────────────────
                //
                // Unconditional jump: `{jump, {f,Label}}`.
                "jmp" => {
                    if let Some(Operand::Var(name)) = instr.srcs.first() {
                        let lbl = resolve_label!(name);
                        instrs.push(BEAMInstruction::new(
                            OP_JUMP, vec![BEAMOperand::f(lbl)],
                        ));
                    }
                }

                // ── jmp_if_true ─────────────────────────────────────────────
                //
                // Branch to target if cond != 0 (truthy).
                //
                // Synthesis:
                //   is_eq_exact {f,fall_synth} {x,cond} {i,0}  ← jump to fall_synth if cond == 0
                //   jump {f,target}                             ← cond != 0: take the branch
                //   label {u,fall_synth}                        ← cond == 0: continue here
                "jmp_if_true" => {
                    let cond_reg = operand_reg!(&instr.srcs[0]);
                    let target_name = match instr.srcs.last() {
                        Some(Operand::Var(name)) if instr.srcs.len() >= 2 => name.as_str(),
                        _ => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_true: expected (cond_var, label_var)".into(),
                        }),
                    };
                    let target_lbl = resolve_label!(target_name);
                    label_counter += 1;
                    let fall_synth = label_counter;

                    instrs.push(BEAMInstruction::new(OP_IS_EQ_EXACT, vec![
                        BEAMOperand::f(fall_synth),  // branch here if cond == 0
                        BEAMOperand::x(cond_reg),
                        BEAMOperand::i(0),
                    ]));
                    instrs.push(BEAMInstruction::new(
                        OP_JUMP, vec![BEAMOperand::f(target_lbl)],
                    ));
                    instrs.push(BEAMInstruction::new(
                        OP_LABEL, vec![BEAMOperand::u(fall_synth as u64)],
                    ));
                }

                // ── jmp_if_false ─────────────────────────────────────────────
                //
                // Branch to target if cond == 0 (falsy).
                //
                // Synthesis:
                //   is_ne_exact {f,fall_synth} {x,cond} {i,0}  ← jump to fall_synth if cond != 0
                //   jump {f,target}                             ← cond == 0: take the branch
                //   label {u,fall_synth}                        ← cond != 0: continue here
                "jmp_if_false" => {
                    let cond_reg = operand_reg!(&instr.srcs[0]);
                    let target_name = match instr.srcs.last() {
                        Some(Operand::Var(name)) if instr.srcs.len() >= 2 => name.as_str(),
                        _ => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_false: expected (cond_var, label_var)".into(),
                        }),
                    };
                    let target_lbl = resolve_label!(target_name);
                    label_counter += 1;
                    let fall_synth = label_counter;

                    instrs.push(BEAMInstruction::new(OP_IS_NE_EXACT, vec![
                        BEAMOperand::f(fall_synth),  // branch here if cond != 0
                        BEAMOperand::x(cond_reg),
                        BEAMOperand::i(0),
                    ]));
                    instrs.push(BEAMInstruction::new(
                        OP_JUMP, vec![BEAMOperand::f(target_lbl)],
                    ));
                    instrs.push(BEAMInstruction::new(
                        OP_LABEL, vec![BEAMOperand::u(fall_synth as u64)],
                    ));
                }

                // ── ret ─────────────────────────────────────────────────────
                //
                // Return a value from the current function.
                //
                // Erlang calling convention: the return value must be in x0.
                // If the value is already in x0 (e.g. for single-arg functions),
                // the move is a no-op (same src = same dst), but we emit it
                // unconditionally for simplicity.
                "ret" => {
                    let r = operand_reg!(&instr.srcs[0]);
                    if r != 0 {
                        // Move the return value to x0 only if not already there.
                        instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                            BEAMOperand::x(r),
                            BEAMOperand::x(0),
                        ]));
                    }
                    instrs.push(BEAMInstruction::new(OP_RETURN, vec![]));
                }

                // ── ret_void ─────────────────────────────────────────────────
                //
                // Return without a value.  BEAM `return` always returns x0,
                // but the caller is expected to ignore it for void functions.
                "ret_void" => {
                    instrs.push(BEAMInstruction::new(OP_RETURN, vec![]));
                }

                // ── call ────────────────────────────────────────────────────
                //
                // Call a function within the same module.
                //
                // IIR `call` layout: op="call", dest=Some(result_var),
                //   srcs=[Var(fn_name), Var(arg0), Var(arg1), …]
                //
                // BEAM calling convention:
                //   1. Move args into x0, x1, …, x(arity-1).
                //   2. `call {u,arity} {f,entry_label}`.
                //   3. The return value is in x0; move it to the result register.
                "call" => {
                    // srcs[0] = function name, srcs[1..] = arguments
                    let callee_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "call: first source must be Var(function_name)".into(),
                        }),
                    };

                    let entry_lbl = match fn_entry_map.get(callee_name) {
                        Some(&lbl) => lbl,
                        None => return Err(IIRBeamError::UndefinedLabel {
                            function: fn_name.clone(),
                            label: callee_name.to_string(),
                        }),
                    };

                    let arg_srcs = &instr.srcs[1..];
                    let arity = arg_srcs.len() as u32;

                    // Move each argument into x0, x1, …
                    // We collect the (arg_reg, target_x) pairs first to avoid
                    // aliasing issues (moving x1 → x0 then x0 → x1 would be
                    // wrong if we emit eagerly).  For a simple sequential move
                    // into fresh positions this is fine, but being explicit is
                    // clearer.
                    for (i, src) in arg_srcs.iter().enumerate() {
                        let arg_reg = operand_reg!(src);
                        let target_x = i as u8;
                        if arg_reg != target_x {
                            instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                                BEAMOperand::x(arg_reg),
                                BEAMOperand::x(target_x),
                            ]));
                        }
                    }

                    // Emit the call.
                    instrs.push(BEAMInstruction::new(OP_CALL, vec![
                        BEAMOperand::u(arity as u64),
                        BEAMOperand::f(entry_lbl),
                    ]));

                    // Move the return value (x0) into the destination register.
                    if let Some(dest) = &instr.dest {
                        let rd = var_reg!(dest);
                        if rd != 0 {
                            instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                                BEAMOperand::x(0),
                                BEAMOperand::x(rd),
                            ]));
                        }
                    }
                }

                // ── load_reg ─────────────────────────────────────────────────
                //
                // `load_reg v → rd` copies variable v's register into rd.
                // In BEAM: `move {x,v_reg} {x,rd}`.
                "load_reg" => {
                    let rd = match &instr.dest {
                        Some(name) => var_reg!(name),
                        None => return Err(IIRBeamError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "load_reg must have a dest".into(),
                        }),
                    };
                    let v_reg = operand_reg!(&instr.srcs[0]);
                    instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                        BEAMOperand::x(v_reg),
                        BEAMOperand::x(rd),
                    ]));
                }

                // ── store_reg ────────────────────────────────────────────────
                //
                // `store_reg v, src` writes src's value into v's register.
                // In BEAM: `move {x,src_reg} {x,v_reg}`.
                "store_reg" => {
                    let v_reg = operand_reg!(&instr.srcs[0]);
                    let src_reg = operand_reg!(&instr.srcs[1]);
                    instrs.push(BEAMInstruction::new(OP_MOVE, vec![
                        BEAMOperand::x(src_reg),
                        BEAMOperand::x(v_reg),
                    ]));
                }

                // ── type_assert ──────────────────────────────────────────────
                //
                // `type_assert` is a frontend hint that a variable has a given
                // type.  BEAM is dynamically typed at runtime (types are checked
                // by BIFs), so there is nothing to emit here.  We drop it.
                "type_assert" => {
                    // Intentionally empty — type assertions are checked at
                    // runtime by the BEAM BIFs, not by explicit instructions.
                }

                // ── Unsupported ops ──────────────────────────────────────────
                //
                // These are caught by validate_for_beam but listed here for
                // exhaustiveness.  If validation was skipped somehow, we
                // return an error rather than silently ignoring them.
                other => {
                    return Err(IIRBeamError::UnsupportedOp {
                        function: fn_name.clone(),
                        op: other.to_string(),
                    });
                }
            }
        }
    }

    // ── Step 6: INT_CODE_END ───────────────────────────────────────────────
    //
    // Every BEAM code section must end with `int_code_end`.  The BEAM loader
    // uses this sentinel to know where the instruction stream stops.
    instrs.push(BEAMInstruction::new(OP_INT_CODE_END, vec![]));

    // ── Step 7: assemble the BEAMModule ───────────────────────────────────

    // label_count = highest label used + 1 (the BEAM loader uses this to
    // pre-allocate its internal label table).
    let label_count = label_counter + 1;

    Ok(BEAMModule {
        name: config.module_name.clone(),
        atoms: atoms.all().to_vec(),
        instructions: instrs,
        imports: imports.imports,
        exports,
        locals: vec![],
        label_count,
        max_opcode: 0, // let encode_beam derive from the instruction stream
        instruction_set_version: 0,
        extra_chunks: vec![],
    })
}

// ===========================================================================
// Unit tests (in-module)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule};

    fn single_fn(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
        }
    }

    fn default_cfg() -> IIRBeamConfig {
        IIRBeamConfig::new("testmod")
    }

    #[test]
    fn module_name_set_in_atoms() {
        let m = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let beam = lower_iir_to_beam(&m, &default_cfg()).unwrap();
        assert_eq!(beam.atoms[0], "testmod");
    }

    #[test]
    fn exports_entry() {
        let m = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let beam = lower_iir_to_beam(&m, &default_cfg()).unwrap();
        assert_eq!(beam.exports.len(), 1);
        assert_eq!(beam.exports[0].arity, 0);
    }

    #[test]
    fn last_instr_is_int_code_end() {
        let m = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let beam = lower_iir_to_beam(&m, &default_cfg()).unwrap();
        assert_eq!(beam.instructions.last().unwrap().opcode, OP_INT_CODE_END);
    }

    #[test]
    fn preamble_is_label_funcinfo_label() {
        let m = single_fn(vec![IIRInstr::new("ret_void", None, vec![], "void")]);
        let beam = lower_iir_to_beam(&m, &default_cfg()).unwrap();
        assert_eq!(beam.instructions[0].opcode, OP_LABEL);
        assert_eq!(beam.instructions[1].opcode, OP_FUNC_INFO);
        assert_eq!(beam.instructions[2].opcode, OP_LABEL);
    }
}

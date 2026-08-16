//! # wasm-validator
//!
//! WebAssembly 1.0 module validator.
//!
//! Validates a parsed [`WasmModule`] for semantic correctness before execution.
//! The validator checks structural properties that the parser cannot enforce:
//!
//! - Type indices are in bounds (function type references, block types).
//! - Function indices are in bounds (calls, element segments, exports).
//! - Export names are unique.
//! - Memory and table counts do not exceed their bounded caps
//!   (`wasm_execution::MAX_MEMORIES`/`MAX_TABLES` -- W16/task #96 raised
//!   these from WASM 1.0's original hardcoded "at most 1").
//! - Data segment memory indices are valid.
//! - Element segment table indices are valid.
//! - Every function body is instruction-level well-typed (WASM06/W02 Phase
//!   2) -- see the [`type_check`] module for the abstract-interpretation
//!   algorithm.
//!
//! ## Why Validate?
//!
//! A WASM binary can be syntactically correct (well-formed LEB128, valid
//! section ordering) but semantically wrong (references a type index that
//! does not exist, or an instruction sequence that pops a type nothing put
//! there). The parser only checks syntax; the validator checks semantics.
//! This separation of concerns keeps both passes simple.
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

use std::collections::HashSet;
use wasm_types::{ExternalKind, ImportTypeInfo, WasmModule};

mod type_check;

// ──────────────────────────────────────────────────────────────────────────────
// Error Type
// ──────────────────────────────────────────────────────────────────────────────

/// An error detected during module validation.
///
/// Each variant carries a human-readable description of what went wrong.
///
/// | Variant               | Example cause                                |
/// |-----------------------|----------------------------------------------|
/// | TypeIndexOutOfBounds  | Function references type index 5, but only   |
/// |                       | 3 types are defined.                         |
/// | FuncIndexOutOfBounds  | Export references function 10, but only 7    |
/// |                       | functions exist (imports + module-defined).   |
/// | DuplicateExport       | Two exports share the name "memory".         |
/// | TooManyMemories       | Module exceeds MAX_MEMORIES.                 |
/// | TooManyTables         | Module exceeds MAX_TABLES.                   |
/// | InvalidDataSegment    | Data segment references memory index 1.      |
/// | InvalidElement | Element segment references table index 1.    |
/// | Other                 | Catch-all for additional validation errors.  |
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// A type index exceeds the number of entries in the type section.
    TypeIndexOutOfBounds(String),
    /// A function index exceeds the total function count.
    FuncIndexOutOfBounds(String),
    /// Two or more exports share the same name.
    DuplicateExport(String),
    /// The module's total memory count exceeds `wasm_execution::MAX_MEMORIES`.
    TooManyMemories(String),
    /// The module's total table count exceeds `wasm_execution::MAX_TABLES`.
    TooManyTables(String),
    /// A data segment references an invalid memory index.
    InvalidDataSegment(String),
    /// An element segment references an invalid table index.
    InvalidElement(String),
    /// A catch-all for other validation failures.
    Other(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::TypeIndexOutOfBounds(m) => write!(f, "TypeIndexOutOfBounds: {}", m),
            ValidationError::FuncIndexOutOfBounds(m) => write!(f, "FuncIndexOutOfBounds: {}", m),
            ValidationError::DuplicateExport(m) => write!(f, "DuplicateExport: {}", m),
            ValidationError::TooManyMemories(m) => write!(f, "TooManyMemories: {}", m),
            ValidationError::TooManyTables(m) => write!(f, "TooManyTables: {}", m),
            ValidationError::InvalidDataSegment(m) => write!(f, "InvalidDataSegment: {}", m),
            ValidationError::InvalidElement(m) => {
                write!(f, "InvalidElement: {}", m)
            }
            ValidationError::Other(m) => write!(f, "ValidationError: {}", m),
        }
    }
}

impl std::error::Error for ValidationError {}

// ──────────────────────────────────────────────────────────────────────────────
// Validated Module
// ──────────────────────────────────────────────────────────────────────────────

/// A validated WASM module — proof that the module passed validation.
///
/// This is a newtype wrapper around [`WasmModule`]. Its existence in the
/// type system guarantees that `validate()` was called and succeeded.
/// Downstream code (the runtime) can accept `ValidatedModule` instead of
/// `WasmModule` to ensure validation is never accidentally skipped.
///
/// The wrapped module is deliberately private (task #100, security
/// review): an earlier version exposed it as `pub module: WasmModule`,
/// which meant any crate depending on `wasm-validator` could construct
/// `ValidatedModule { module: attacker_controlled }` directly with a
/// struct literal, skipping `validate()` entirely and defeating the very
/// guarantee this type exists to provide (e.g. `wasm-runtime::
/// instantiate()` requiring `&ValidatedModule` -- see its own doc
/// comment -- offered no real protection while this field was public).
/// [`ValidatedModule::module`] is the only way to reach the wrapped
/// value now; the only way to construct a `ValidatedModule` at all is
/// [`validate`] succeeding. This is enforced by the compiler, not a
/// convention -- the struct literal that used to be the bypass is now a
/// compile error:
///
/// ```compile_fail
/// use wasm_types::WasmModule;
/// use wasm_validator::ValidatedModule;
///
/// let bypass = ValidatedModule { module: WasmModule::default() };
/// ```
#[derive(Debug, Clone)]
pub struct ValidatedModule {
    module: WasmModule,
}

impl ValidatedModule {
    /// The underlying parsed module, proven to have passed [`validate`].
    pub fn module(&self) -> &WasmModule {
        &self.module
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Validation
// ──────────────────────────────────────────────────────────────────────────────

/// Validate a parsed WASM module for semantic correctness.
///
/// Returns a [`ValidatedModule`] on success, or a [`ValidationError`]
/// describing the first problem found.
///
/// # Checks performed
///
/// 1. **Memory count** -- at most `wasm_execution::MAX_MEMORIES`
///    (imports + module); each memory's `min`/`max` also capped at the
///    real spec's 65536-page ceiling.
/// 2. **Table count** -- at most `wasm_execution::MAX_TABLES`
///    (imports + module); each table's `min` also capped at this
///    interpreter's own `MAX_TABLE_ELEMENTS` resource limit.
/// 3. **Function type indices** -- Every entry in the function section must
///    reference a valid index in the type section.
/// 4. **Import type indices** -- Function imports must reference valid types.
/// 5. **Code/function count match** -- The code section must have exactly as
///    many entries as the function section.
/// 6. **Export uniqueness** -- No two exports may share the same name.
/// 7. **Export indices** -- Every export must reference a valid entity.
/// 8. **Data segment validity** -- Memory index must be 0 (data-segment
///    application still only ever targets memory 0, W16).
/// 9. **Element segment validity** -- Table index must be a real,
///    in-bounds table (task #96 -- element-segment application already
///    indexes by the real table, so this generalized beyond "must be 0").
/// 10. **Start function** -- If present, must be a valid function index.
/// 11. **Instruction-level type checking** -- Every function body's
///     instruction sequence is well-typed (WASM06/W02 Phase 2): no stack
///     underflow, no type mismatches, correct local/global indices and
///     mutability, memory instructions only when a memory exists. See
///     [`type_check`] for the algorithm.
///
/// # Example
///
/// ```rust
/// use wasm_types::WasmModule;
/// use wasm_validator::validate;
///
/// let module = WasmModule::default();
/// let validated = validate(&module).expect("empty module is valid");
/// ```
pub fn validate(module: &WasmModule) -> Result<ValidatedModule, ValidationError> {
    // Count imported memories and tables.
    let imported_memories = module
        .imports
        .iter()
        .filter(|i| i.kind == ExternalKind::Memory)
        .count();
    let imported_tables = module
        .imports
        .iter()
        .filter(|i| i.kind == ExternalKind::Table)
        .count();
    let imported_functions = module
        .imports
        .iter()
        .filter(|i| i.kind == ExternalKind::Function)
        .count();
    let imported_globals = module
        .imports
        .iter()
        .filter(|i| i.kind == ExternalKind::Global)
        .count();

    // ── Check 1: Memory count ≤ MAX_MEMORIES ───────────────────────────
    //
    // Multi-memory proposal, W16, task #85: WASM 1.0's own hardcoded "at
    // most 1" cap is gone; `wasm_execution::MAX_MEMORIES` is a real,
    // bounded cap on total memory count (imported + declared) instead --
    // see its own doc comment for why 64.
    let total_memories = imported_memories + module.memories.len();
    if total_memories > wasm_execution::MAX_MEMORIES {
        return Err(ValidationError::TooManyMemories(format!(
            "at most {} memories allowed, found {} ({} imported + {} declared)",
            wasm_execution::MAX_MEMORIES,
            total_memories,
            imported_memories,
            module.memories.len()
        )));
    }

    // ── Check 1b: Memory limits ≤ the real spec's 65536-page ceiling ────
    //
    // Security review (task #96): a REAL WASM spec structural-validation
    // rule (not implementation-defined, unlike Check 2b below) -- a
    // memory's `min`/`max` may never exceed 2^16 pages (the entire 32-bit
    // address space at 64KiB/page), the identical bound `LinearMemory::
    // grow()` already enforces at runtime. Previously unchecked at
    // validation time: a module declaring an over-cap `min` would reach
    // `LinearMemory::new`'s eager `vec![0u8; min * PAGE_SIZE]` allocation
    // unvalidated -- e.g. a single `(memory 100000)` attempts a ~6.4GB
    // allocation before ever running a single instruction.
    //
    // A per-memory cap alone isn't enough once `MAX_MEMORIES` (64) is
    // multiplied in: 64 memories each at the spec's own 65536-page max
    // would still total ~256GB of eager allocation from one small module,
    // all through the fully-intended `validate()` path -- no bypass
    // needed (2nd round finding). So this also tracks a running total
    // across every memory (imported + declared) and caps the SUM at the
    // same 65536-page bound: still permits any single spec-valid memory
    // at its true max, just prevents many of them from being max-size
    // simultaneously.
    let mut total_memory_pages: u64 = 0;
    for (i, mt) in module.memories.iter().enumerate() {
        if mt.limits.min > 65536 || mt.limits.max.is_some_and(|m| m > 65536) {
            return Err(ValidationError::Other(format!(
                "memory #{i}: limits (min={}, max={:?}) exceed the spec maximum of 65536 pages",
                mt.limits.min, mt.limits.max
            )));
        }
        total_memory_pages += mt.limits.min as u64;
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Memory(mt) = &imp.type_info {
            if mt.limits.min > 65536 || mt.limits.max.is_some_and(|m| m > 65536) {
                return Err(ValidationError::Other(format!(
                    "imported memory {}.{}: limits (min={}, max={:?}) exceed the spec maximum of 65536 pages",
                    imp.module_name, imp.name, mt.limits.min, mt.limits.max
                )));
            }
            total_memory_pages += mt.limits.min as u64;
        }
    }
    if total_memory_pages > 65536 {
        return Err(ValidationError::Other(format!(
            "total declared memory across all memories is {total_memory_pages} pages, exceeding the aggregate cap of 65536 pages"
        )));
    }

    // ── Check 2: Table count ≤ MAX_TABLES ───────────────────────────────
    //
    // Multi-table (task #96): WASM 1.0's own hardcoded "at most 1" cap is
    // gone; `wasm_execution::MAX_TABLES` is a real, bounded cap instead --
    // see its own doc comment for why this needs no companion storage-
    // layer work (unlike W16's multi-memory, table storage was already a
    // `Vec` and already index-aware end to end).
    let total_tables = imported_tables + module.tables.len();
    if total_tables > wasm_execution::MAX_TABLES {
        return Err(ValidationError::TooManyTables(format!(
            "at most {} tables allowed, found {} ({} imported + {} declared)",
            wasm_execution::MAX_TABLES,
            total_tables,
            imported_tables,
            module.tables.len()
        )));
    }

    // ── Check 2b: Table limits ≤ MAX_TABLE_ELEMENTS ─────────────────────
    //
    // Security review (task #96): unlike Check 1b above, this is NOT a
    // real spec requirement -- WASM's own spec allows a table `min` up to
    // `2^32 - 1` -- it's this interpreter's own implementation-defined
    // resource limit, matching `Table::new`'s eager `vec![None; min]`
    // allocation cost. See `MAX_TABLE_ELEMENTS`'s own doc comment for the
    // full reasoning, including why raising `MAX_TABLES` from 1 to 64
    // (this same task) made an unvalidated `min` a real amplified DoS
    // vector worth closing now rather than leaving as a pre-existing gap.
    //
    // Same aggregate-vs-per-item gap as Check 1b above (2nd round
    // finding): 64 tables each at `MAX_TABLE_ELEMENTS` would still total
    // ~5.1GB from one small module. Tracks a running total across every
    // table (imported + declared) and caps the SUM at the same
    // `MAX_TABLE_ELEMENTS` bound too -- still permits any single table at
    // its full implementation-defined max, just not many of them at once.
    let mut total_table_elements: u64 = 0;
    for (i, tt) in module.tables.iter().enumerate() {
        if tt.limits.min > wasm_execution::MAX_TABLE_ELEMENTS {
            return Err(ValidationError::Other(format!(
                "table #{i}: declared minimum {} elements exceeds this interpreter's resource limit of {}",
                tt.limits.min,
                wasm_execution::MAX_TABLE_ELEMENTS
            )));
        }
        total_table_elements += tt.limits.min as u64;
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Table(tt) = &imp.type_info {
            if tt.limits.min > wasm_execution::MAX_TABLE_ELEMENTS {
                return Err(ValidationError::Other(format!(
                    "imported table {}.{}: declared minimum {} elements exceeds this interpreter's resource limit of {}",
                    imp.module_name, imp.name, tt.limits.min, wasm_execution::MAX_TABLE_ELEMENTS
                )));
            }
            total_table_elements += tt.limits.min as u64;
        }
    }
    if total_table_elements > wasm_execution::MAX_TABLE_ELEMENTS as u64 {
        return Err(ValidationError::Other(format!(
            "total declared elements across all tables is {total_table_elements}, exceeding the aggregate cap of {}",
            wasm_execution::MAX_TABLE_ELEMENTS
        )));
    }

    // ── Check 3: Import type indices ────────────────────────────────────
    for (i, imp) in module.imports.iter().enumerate() {
        if let ImportTypeInfo::Function(type_idx) = &imp.type_info {
            if *type_idx as usize >= module.types.len() {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "import #{} ({}.{}) references type index {}, but only {} types exist",
                    i,
                    imp.module_name,
                    imp.name,
                    type_idx,
                    module.types.len()
                )));
            }
        }
    }

    // ── Check 4: Function type indices ──────────────────────────────────
    for (i, &type_idx) in module.functions.iter().enumerate() {
        if type_idx as usize >= module.types.len() {
            return Err(ValidationError::TypeIndexOutOfBounds(format!(
                "function #{} references type index {}, but only {} types exist",
                i,
                type_idx,
                module.types.len()
            )));
        }
    }

    // ── Check 5: Code/function count match ──────────────────────────────
    if module.code.len() != module.functions.len() {
        return Err(ValidationError::Other(format!(
            "code section has {} entries but function section has {} entries",
            module.code.len(),
            module.functions.len()
        )));
    }

    // ── Check 6: Export uniqueness ──────────────────────────────────────
    let mut export_names = HashSet::new();
    for exp in &module.exports {
        if !export_names.insert(&exp.name) {
            return Err(ValidationError::DuplicateExport(format!(
                "duplicate export name: \"{}\"",
                exp.name
            )));
        }
    }

    // ── Check 7: Export indices ─────────────────────────────────────────
    let total_functions = imported_functions + module.functions.len();
    let total_globals = imported_globals + module.globals.len();

    for exp in &module.exports {
        match exp.kind {
            ExternalKind::Function => {
                if (exp.index as usize) >= total_functions {
                    return Err(ValidationError::FuncIndexOutOfBounds(format!(
                        "export \"{}\" references function index {}, but only {} functions exist",
                        exp.name, exp.index, total_functions
                    )));
                }
            }
            ExternalKind::Memory => {
                if (exp.index as usize) >= total_memories {
                    return Err(ValidationError::InvalidDataSegment(format!(
                        "export \"{}\" references memory index {}, but only {} memories exist",
                        exp.name, exp.index, total_memories
                    )));
                }
            }
            ExternalKind::Table => {
                if (exp.index as usize) >= total_tables {
                    return Err(ValidationError::InvalidElement(format!(
                        "export \"{}\" references table index {}, but only {} tables exist",
                        exp.name, exp.index, total_tables
                    )));
                }
            }
            ExternalKind::Global => {
                if (exp.index as usize) >= total_globals {
                    return Err(ValidationError::Other(format!(
                        "export \"{}\" references global index {}, but only {} globals exist",
                        exp.name, exp.index, total_globals
                    )));
                }
            }
        }
    }

    // ── Check 8: Data segments ──────────────────────────────────────────
    //
    // Deliberately NOT widened alongside Check 1's memory-count cap
    // (multi-memory, W16, task #85): `wasm-runtime::instantiate()` only
    // ever applies a data segment to memory 0, regardless of
    // `seg.memory_index` (see `code/specs/
    // W16-wasm-multi-memory-first-slice.md`'s "What does NOT change").
    // Accepting a segment targeting a non-zero index here would let it
    // silently land on the WRONG memory at instantiation time instead of
    // being rejected -- keeping this check at "must be 0" means a module
    // using this real, spec-legal (but not yet supported HERE) feature
    // fails loudly at validation instead.
    for (i, seg) in module.data.iter().enumerate() {
        if seg.memory_index != 0 || (total_memories == 0 && !module.data.is_empty()) {
            if total_memories == 0 {
                return Err(ValidationError::InvalidDataSegment(format!(
                    "data segment #{} references memory, but no memory is declared",
                    i
                )));
            }
            if seg.memory_index != 0 {
                return Err(ValidationError::InvalidDataSegment(format!(
                    "data segment #{} references memory index {}, but only index 0 is valid",
                    i, seg.memory_index
                )));
            }
        }
    }

    // ── Check 9: Element segments ───────────────────────────────────────
    //
    // Multi-table (task #96): generalized to a real bounds check against
    // `total_tables`, unlike W16's data-segment check (which deliberately
    // stayed "must be 0") -- safe here because `wasm-runtime::
    // instantiate()`'s element-segment application already indexes by the
    // real `elem.table_index` (`tables.get_mut(elem.table_index as
    // usize)`), so there is no silent-misapplication risk to guard
    // against.
    for (i, elem) in module.elements.iter().enumerate() {
        if total_tables == 0 {
            return Err(ValidationError::InvalidElement(format!(
                "element segment #{} references a table, but no table is declared",
                i
            )));
        }
        if elem.table_index as usize >= total_tables {
            return Err(ValidationError::InvalidElement(format!(
                "element segment #{} references table index {}, but only {} tables exist",
                i, elem.table_index, total_tables
            )));
        }
        // Validate function indices within element segments.
        for &func_idx in &elem.function_indices {
            if (func_idx as usize) >= total_functions {
                return Err(ValidationError::FuncIndexOutOfBounds(format!(
                    "element segment #{} references function index {}, \
                     but only {} functions exist",
                    i, func_idx, total_functions
                )));
            }
        }
    }

    // ── Check 10: Start function ────────────────────────────────────────
    if let Some(start_idx) = module.start {
        if (start_idx as usize) >= total_functions {
            return Err(ValidationError::FuncIndexOutOfBounds(format!(
                "start function index {} is out of bounds (only {} functions exist)",
                start_idx, total_functions
            )));
        }
    }

    // ── Check 11: Instruction-level type checking (WASM06/W02 Phase 2) ──
    type_check::type_check_module(module)?;

    Ok(ValidatedModule {
        module: module.clone(),
    })
}

// ───────────────────────────────────────────────────────────────────────���──────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_types::*;

    #[test]
    fn empty_module_is_valid() {
        let module = WasmModule::default();
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn valid_module_with_function() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B],
            }],
            exports: vec![Export {
                name: "square".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_bad_type_index() {
        let module = WasmModule {
            types: vec![],
            functions: vec![99], // index 99 does not exist
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x0B],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TypeIndexOutOfBounds(_)));
    }

    #[test]
    fn rejects_duplicate_exports() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            functions: vec![0, 0],
            code: vec![
                FunctionBody {
                    locals: vec![],
                    code: vec![0x0B],
                },
                FunctionBody {
                    locals: vec![],
                    code: vec![0x0B],
                },
            ],
            exports: vec![
                Export {
                    name: "dup".to_string(),
                    kind: ExternalKind::Function,
                    index: 0,
                },
                Export {
                    name: "dup".to_string(),
                    kind: ExternalKind::Function,
                    index: 1,
                },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::DuplicateExport(_)));
    }

    /// Multi-memory (W16, task #85) raised the cap from 1 to
    /// `wasm_execution::MAX_MEMORIES`, so a module needs MORE than that
    /// many memories to still be rejected -- proving the cap MOVED, not
    /// disappeared.
    #[test]
    fn rejects_too_many_memories() {
        let module = WasmModule {
            memories: (0..=wasm_execution::MAX_MEMORIES)
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false })
                .collect(),
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyMemories(_)));
    }

    /// A module with exactly `MAX_MEMORIES` (previously rejected under
    /// the old hardcoded "at most 1" WASM-1.0 cap) now validates
    /// successfully -- the concrete case this whole spec exists to fix.
    #[test]
    fn accepts_up_to_max_memories() {
        let module = WasmModule {
            memories: (0..wasm_execution::MAX_MEMORIES)
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false })
                .collect(),
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    /// Security review (task #96): a real WASM spec rule, not an
    /// implementation-defined heuristic -- a memory's `min` may never
    /// exceed 2^16 pages. Previously unchecked at validation time, so an
    /// over-cap `min` reached `LinearMemory::new`'s eager allocation
    /// unvalidated.
    #[test]
    fn rejects_a_memory_declaring_more_than_65536_pages() {
        let module = WasmModule {
            memories: vec![MemoryType { limits: Limits { min: 65537, max: None }, shared: false }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn accepts_a_memory_declaring_exactly_65536_pages() {
        let module = WasmModule {
            memories: vec![MemoryType { limits: Limits { min: 65536, max: None }, shared: false }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    /// Security review, 2nd round (task #96): a per-memory cap alone
    /// isn't enough once `MAX_MEMORIES` (64) is multiplied in -- 64
    /// memories each individually under the per-memory 65536-page cap
    /// can still sum to far more aggregate allocation than any single
    /// spec-valid memory would ever need. Two memories each at the full
    /// 65536-page cap sum to 131072, over the aggregate cap, even though
    /// neither one alone would be rejected by Check 1b's per-item check.
    #[test]
    fn rejects_memories_whose_combined_pages_exceed_the_aggregate_cap() {
        let module = WasmModule {
            memories: vec![
                MemoryType { limits: Limits { min: 65536, max: None }, shared: false },
                MemoryType { limits: Limits { min: 1, max: None }, shared: false },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_bad_export_func_index() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x0B],
            }],
            exports: vec![Export {
                name: "bad".to_string(),
                kind: ExternalKind::Function,
                index: 99,
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::FuncIndexOutOfBounds(_)));
    }

    #[test]
    fn rejects_code_function_count_mismatch() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            functions: vec![0, 0], // 2 functions
            code: vec![FunctionBody {
                // but only 1 body
                locals: vec![],
                code: vec![0x0B],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)));
    }

    #[test]
    fn rejects_bad_start_function() {
        let module = WasmModule {
            start: Some(99),
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::FuncIndexOutOfBounds(_)));
    }

    // ── Additional validation tests ──────────────────────────────────

    #[test]
    fn valid_start_function() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            start: Some(0),
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    /// Multi-table (task #96) raised the cap from 1 to
    /// `wasm_execution::MAX_TABLES`, so a module needs MORE than that
    /// many tables to still be rejected -- proving the cap MOVED, not
    /// disappeared.
    fn rejects_too_many_tables() {
        let module = WasmModule {
            tables: (0..=wasm_execution::MAX_TABLES)
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None } })
                .collect(),
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyTables(_)));
    }

    /// A module with exactly `MAX_TABLES` (previously rejected under the
    /// old hardcoded "at most 1" WASM-1.0 cap) now validates
    /// successfully.
    #[test]
    fn accepts_up_to_max_tables() {
        let module = WasmModule {
            tables: (0..wasm_execution::MAX_TABLES)
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None } })
                .collect(),
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    /// Security review (task #96): raising `MAX_TABLES` from 1 to 64
    /// amplified a pre-existing gap -- a table's declared `min` was never
    /// bounds-checked before reaching `Table::new`'s eager
    /// `vec![None; min]` allocation. Unlike memory's real spec-mandated
    /// 65536-page ceiling, this is an implementation-defined resource
    /// limit (real WASM allows a table `min` up to `2^32 - 1`).
    #[test]
    fn rejects_a_table_declaring_more_than_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS + 1, max: None },
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn accepts_a_table_declaring_exactly_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS, max: None },
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    /// Security review, 2nd round (task #96): same aggregate-vs-per-item
    /// gap as memory's Check 1b above -- two tables each under the
    /// per-table `MAX_TABLE_ELEMENTS` cap can still sum past it, even
    /// though neither is rejected individually by Check 2b's per-item
    /// check.
    #[test]
    fn rejects_tables_whose_combined_elements_exceed_the_aggregate_cap() {
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS, max: None },
                },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None } },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_bad_import_type_index() {
        let module = WasmModule {
            types: vec![],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "func".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(99),
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TypeIndexOutOfBounds(_)));
    }

    #[test]
    fn valid_import_type_index() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "func".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_data_segment_no_memory() {
        let module = WasmModule {
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidDataSegment(_)));
    }

    #[test]
    fn rejects_data_segment_bad_memory_index() {
        let module = WasmModule {
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
            }],
            data: vec![DataSegment {
                memory_index: 1, // only index 0 is valid
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidDataSegment(_)));
    }

    #[test]
    fn valid_data_segment() {
        let module = WasmModule {
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01, 0x02],
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_element_segment_no_table() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            elements: vec![Element {
                table_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![0],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidElement(_)));
    }

    #[test]
    fn rejects_element_segment_bad_table_index() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: 10, max: None },
            }],
            elements: vec![Element {
                table_index: 1, // only 0 is valid
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![0],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidElement(_)));
    }

    #[test]
    fn rejects_element_segment_bad_func_index() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: 10, max: None },
            }],
            elements: vec![Element {
                table_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![99], // out of bounds
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::FuncIndexOutOfBounds(_)));
    }

    #[test]
    fn valid_element_segment() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: 10, max: None },
            }],
            elements: vec![Element {
                table_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![0],
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_export_bad_memory_index() {
        let module = WasmModule {
            exports: vec![Export {
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                index: 0,
            }],
            // No memories exist
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidDataSegment(_)));
    }

    #[test]
    fn rejects_export_bad_table_index() {
        let module = WasmModule {
            exports: vec![Export {
                name: "tbl".to_string(),
                kind: ExternalKind::Table,
                index: 0,
            }],
            // No tables exist
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidElement(_)));
    }

    #[test]
    fn rejects_export_bad_global_index() {
        let module = WasmModule {
            exports: vec![Export {
                name: "g".to_string(),
                kind: ExternalKind::Global,
                index: 0,
            }],
            // No globals exist
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)));
    }

    #[test]
    fn valid_export_memory() {
        let module = WasmModule {
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
                shared: false,
            }],
            exports: vec![Export {
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                index: 0,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn valid_export_global() {
        let module = WasmModule {
            globals: vec![Global {
                global_type: GlobalType {
                    value_type: ValueType::I32,
                    mutable: false,
                },
                init_expr: vec![0x41, 0x00, 0x0B],
            }],
            exports: vec![Export {
                name: "g".to_string(),
                kind: ExternalKind::Global,
                index: 0,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn valid_module_with_imports_counted() {
        // Imported function + module function = 2 total functions
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "imported".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(0),
            }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            exports: vec![Export {
                name: "local_fn".to_string(),
                kind: ExternalKind::Function,
                index: 1, // index 0 is import, index 1 is module-defined
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn imported_memory_counts_toward_limit() {
        // 1 imported + MAX_MEMORIES declared = MAX_MEMORIES + 1, over the
        // cap -- proves the imported one counts toward the SAME limit as
        // declared memories, not a separate budget.
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                type_info: ImportTypeInfo::Memory(MemoryType {
                    limits: Limits { min: 1, max: None },
                    shared: false,
                }),
            }],
            memories: (0..wasm_execution::MAX_MEMORIES)
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false })
                .collect(),
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyMemories(_)));
    }

    #[test]
    fn imported_table_counts_toward_limit() {
        // 1 imported + MAX_TABLES declared = MAX_TABLES + 1, over the cap
        // -- proves the imported one counts toward the SAME limit as
        // declared tables, not a separate budget.
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "tbl".to_string(),
                kind: ExternalKind::Table,
                type_info: ImportTypeInfo::Table(TableType {
                    element_type: 0x70,
                    limits: Limits { min: 1, max: None },
                }),
            }],
            tables: (0..wasm_execution::MAX_TABLES)
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None } })
                .collect(),
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyTables(_)));
    }

    #[test]
    fn validation_error_display() {
        let cases = vec![
            (ValidationError::TypeIndexOutOfBounds("test".into()), "TypeIndexOutOfBounds: test"),
            (ValidationError::FuncIndexOutOfBounds("test".into()), "FuncIndexOutOfBounds: test"),
            (ValidationError::DuplicateExport("test".into()), "DuplicateExport: test"),
            (ValidationError::TooManyMemories("test".into()), "TooManyMemories: test"),
            (ValidationError::TooManyTables("test".into()), "TooManyTables: test"),
            (ValidationError::InvalidDataSegment("test".into()), "InvalidDataSegment: test"),
            (ValidationError::InvalidElement("test".into()), "InvalidElement: test"),
            (ValidationError::Other("test".into()), "ValidationError: test"),
        ];
        for (err, expected) in cases {
            assert_eq!(format!("{}", err), expected);
        }
    }

    #[test]
    fn validation_error_is_error_trait() {
        let err = ValidationError::Other("test".into());
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn validated_module_contains_module() {
        let module = WasmModule::default();
        let validated = validate(&module).unwrap();
        assert_eq!(validated.module.types.len(), 0);
    }

    #[test]
    fn multiple_valid_exports() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0, 0],
            code: vec![
                FunctionBody { locals: vec![], code: vec![0x0B] },
                FunctionBody { locals: vec![], code: vec![0x0B] },
            ],
            exports: vec![
                Export { name: "a".to_string(), kind: ExternalKind::Function, index: 0 },
                Export { name: "b".to_string(), kind: ExternalKind::Function, index: 1 },
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }
}

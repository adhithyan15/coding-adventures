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
use wasm_types::{ExternalKind, ImportTypeInfo, ValueType, WasmModule};

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
    // W21 (exceptions proposal): tags, same "imports first, then
    // module-defined, in declaration order" combined index-space
    // convention as every other kind above.
    let imported_tags = module
        .imports
        .iter()
        .filter(|i| i.kind == ExternalKind::Tag)
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

    // ── Check 1a: min <= max for every memory (and, below, every table) ─
    //
    // A real, spec-mandated structural rule this repo never actually
    // checked before W25 (found chasing `memory64.wast`'s own
    // `"size minimum must not be greater than maximum"` `assert_invalid`
    // case -- a pre-existing gap for 32-bit memories too, not something
    // memory64 introduced). Applies identically regardless of `is64`.
    for (i, mt) in module.memories.iter().enumerate() {
        if mt.limits.max.is_some_and(|m| mt.limits.min > m) {
            return Err(ValidationError::Other(format!(
                "memory #{i}: size minimum must not be greater than maximum (min={}, max={:?})",
                mt.limits.min, mt.limits.max
            )));
        }
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Memory(mt) = &imp.type_info {
            if mt.limits.max.is_some_and(|m| mt.limits.min > m) {
                return Err(ValidationError::Other(format!(
                    "imported memory {}.{}: size minimum must not be greater than maximum (min={}, max={:?})",
                    imp.module_name, imp.name, mt.limits.min, mt.limits.max
                )));
            }
        }
    }

    // ── Check 1b: Memory limits ≤ the real spec's own ceiling ───────────
    //
    // Security review (task #96): a REAL WASM spec structural-validation
    // rule (not implementation-defined, unlike Check 2b below) -- a
    // 32-bit memory's `min`/`max` may never exceed 2^16 pages (the entire
    // 32-bit address space at 64KiB/page), the identical bound
    // `LinearMemory::grow()` already enforces at runtime. Previously
    // unchecked at validation time: a module declaring an over-cap `min`
    // would reach `LinearMemory::new`'s eager `vec![0u8; min * PAGE_SIZE]`
    // allocation unvalidated -- e.g. a single `(memory 100000)` attempts a
    // ~6.4GB allocation before ever running a single instruction.
    //
    // W25 (memory64): a 64-bit (`is64`) memory's OWN spec ceiling is
    // `2^48` pages, not `2^16` -- verified live against `memory64.wast`'s
    // own real `assert_invalid` boundary (`0x1_0000_0000_0001` invalid,
    // `0x1_0000_0000_0000` valid). This is a much larger number than any
    // real system will ever actually allocate, but VALIDATION does not
    // allocate -- a module that only ever DECLARES such a memory (never
    // instantiates it) is genuinely spec-valid and must validate
    // successfully; only an actual instantiation attempt hits this
    // interpreter's own separate, much smaller practical resource limit
    // (`wasm-execution::MAX_MEMORY64_INITIAL_PAGES`, enforced by
    // `LinearMemory::new_with_is64` in `wasm-runtime::instantiate`, not
    // here).
    //
    // A per-memory cap alone isn't enough once `MAX_MEMORIES` (64) is
    // multiplied in: 64 memories each at the 32-bit spec's own
    // 65536-page max would still total ~256GB of eager allocation from
    // one small module, all through the fully-intended `validate()` path
    // -- no bypass needed (2nd round finding). So this also tracks a
    // running total across every 32-bit memory (imported + declared) and
    // caps the SUM at the same 65536-page bound: still permits any
    // single spec-valid 32-bit memory at its true max, just prevents many
    // of them from being max-size simultaneously. `is64` memories are
    // deliberately EXCLUDED from this specific aggregate (it was
    // calibrated for the 32-bit case, where the spec ceiling and the safe
    // allocation ceiling are the SAME number, 65536 -- that's no longer
    // true for `is64`, whose much larger spec ceiling would make this
    // aggregate reject an individually-valid large `is64` declaration
    // that nothing has even tried to instantiate yet); `is64` memories
    // get their own, separate aggregate-and-per-memory practical cap at
    // actual instantiation time instead (`wasm-runtime::instantiate`'s
    // own `total_is64_pages` tracking, right where the real allocation
    // risk lives).
    let mut total_memory_pages: u64 = 0;
    for (i, mt) in module.memories.iter().enumerate() {
        let ceiling: u64 = if mt.is64 { 1u64 << 48 } else { 65536 };
        if mt.limits.min > ceiling || mt.limits.max.is_some_and(|m| m > ceiling) {
            return Err(ValidationError::Other(format!(
                "memory #{i}: limits (min={}, max={:?}) exceed the spec maximum of {ceiling} pages",
                mt.limits.min, mt.limits.max
            )));
        }
        if !mt.is64 {
            total_memory_pages += mt.limits.min;
        }
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Memory(mt) = &imp.type_info {
            let ceiling: u64 = if mt.is64 { 1u64 << 48 } else { 65536 };
            if mt.limits.min > ceiling || mt.limits.max.is_some_and(|m| m > ceiling) {
                return Err(ValidationError::Other(format!(
                    "imported memory {}.{}: limits (min={}, max={:?}) exceed the spec maximum of {ceiling} pages",
                    imp.module_name, imp.name, mt.limits.min, mt.limits.max
                )));
            }
            if !mt.is64 {
                total_memory_pages += mt.limits.min;
            }
        }
    }
    if total_memory_pages > 65536 {
        return Err(ValidationError::Other(format!(
            "total declared memory across all 32-bit memories is {total_memory_pages} pages, exceeding the aggregate cap of 65536 pages"
        )));
    }

    // ── Check 1c: min <= max for every table (W25) ──────────────────────
    //
    // Same real, spec-mandated rule as Check 1a above, applied to tables
    // too -- this repo never checked it for tables either before W25.
    for (i, tt) in module.tables.iter().enumerate() {
        if tt.limits.max.is_some_and(|m| tt.limits.min > m) {
            return Err(ValidationError::Other(format!(
                "table #{i}: size minimum must not be greater than maximum (min={}, max={:?})",
                tt.limits.min, tt.limits.max
            )));
        }
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Table(tt) = &imp.type_info {
            if tt.limits.max.is_some_and(|m| tt.limits.min > m) {
                return Err(ValidationError::Other(format!(
                    "imported table {}.{}: size minimum must not be greater than maximum (min={}, max={:?})",
                    imp.module_name, imp.name, tt.limits.min, tt.limits.max
                )));
            }
        }
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
    // real spec requirement for a 32-bit table -- WASM's own spec allows a
    // 32-bit table `min` up to `2^32 - 1` -- it's this interpreter's own
    // implementation-defined resource limit, matching `Table::new`'s eager
    // `vec![None; min]` allocation cost. See `MAX_TABLE_ELEMENTS`'s own doc
    // comment for the full reasoning, including why raising `MAX_TABLES`
    // from 1 to 64 (this same task) made an unvalidated `min` a real
    // amplified DoS vector worth closing now rather than leaving as a
    // pre-existing gap.
    //
    // W26 (table64 proposal): an `is64` table's REAL spec ceiling is
    // `u64::MAX` (verified live against the reference interpreter's own
    // `interpreter/valid/valid.ml::check_tabletype` -- table64's ceiling is
    // NOT the same `2^48`-page bound memory64 uses; tables aren't measured
    // in byte-multiplying "pages", so the proposal imposes no equivalent
    // artificial cap). No `u64` value can ever exceed `u64::MAX`, so this
    // per-item check is unconditionally satisfied for `is64` tables --
    // skipped explicitly below (not silently applied and coincidentally
    // passing) -- and `is64` tables are excluded from the 32-bit aggregate
    // for the same reason `W25`'s Check 1b excludes `is64` memories: the
    // aggregate is calibrated for the 32-bit case, where the practical
    // allocation ceiling and this check's bound coincide; that's no longer
    // true once `is64` can legitimately declare a `min` this interpreter
    // will refuse to actually allocate (see `Table::new_with_is64`'s own
    // separate, real practical cap at INSTANTIATION time instead).
    //
    // Same aggregate-vs-per-item gap as Check 1b above (2nd round
    // finding): 64 tables each at `MAX_TABLE_ELEMENTS` would still total
    // ~5.1GB from one small module. Tracks a running total across every
    // 32-bit table (imported + declared) and caps the SUM at the same
    // `MAX_TABLE_ELEMENTS` bound too -- still permits any single table at
    // its full implementation-defined max, just not many of them at once.
    let mut total_table_elements: u64 = 0;
    for (i, tt) in module.tables.iter().enumerate() {
        if !tt.is64 && tt.limits.min > wasm_execution::MAX_TABLE_ELEMENTS as u64 {
            return Err(ValidationError::Other(format!(
                "table #{i}: declared minimum {} elements exceeds this interpreter's resource limit of {}",
                tt.limits.min,
                wasm_execution::MAX_TABLE_ELEMENTS
            )));
        }
        if !tt.is64 {
            total_table_elements += tt.limits.min;
        }
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Table(tt) = &imp.type_info {
            if !tt.is64 && tt.limits.min > wasm_execution::MAX_TABLE_ELEMENTS as u64 {
                return Err(ValidationError::Other(format!(
                    "imported table {}.{}: declared minimum {} elements exceeds this interpreter's resource limit of {}",
                    imp.module_name, imp.name, tt.limits.min, wasm_execution::MAX_TABLE_ELEMENTS
                )));
            }
            if !tt.is64 {
                total_table_elements += tt.limits.min;
            }
        }
    }
    if total_table_elements > wasm_execution::MAX_TABLE_ELEMENTS as u64 {
        return Err(ValidationError::Other(format!(
            "total declared elements across all 32-bit tables is {total_table_elements}, exceeding the aggregate cap of {}",
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
        // W21 (exceptions proposal): a tag import's type index must be in
        // bounds, AND (a real spec rule, not this repo's own invention --
        // `tag.wast`'s own two "non-empty tag result type" assert_invalid
        // cases probe exactly this) the referenced function type's
        // `results` must be empty. `grade_assert_invalid` only checks that
        // SOME rejection happens, never the message text, so the exact
        // wording here is not corpus-load-bearing -- still worded
        // precisely for a real reader/future maintainer.
        if let ImportTypeInfo::Tag(type_idx) = &imp.type_info {
            let ty = module.types.get(*type_idx as usize).ok_or_else(|| {
                ValidationError::TypeIndexOutOfBounds(format!(
                    "import #{} ({}.{}) (tag) references type index {}, but only {} types exist",
                    i,
                    imp.module_name,
                    imp.name,
                    type_idx,
                    module.types.len()
                ))
            })?;
            if !ty.results.is_empty() {
                return Err(ValidationError::Other(format!(
                    "import #{} ({}.{}) (tag): non-empty tag result type",
                    i, imp.module_name, imp.name
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

    // ── Check 4b: Tag type indices, and "tag result type must be empty" ─
    //
    // W21 (exceptions proposal): mirrors Check 4 above for module-defined
    // tags (`module.tags: Vec<u32>`, type indices only, imports handled by
    // Check 3 above), plus the same non-empty-result-type rule.
    for (i, &type_idx) in module.tags.iter().enumerate() {
        let ty = module.types.get(type_idx as usize).ok_or_else(|| {
            ValidationError::TypeIndexOutOfBounds(format!(
                "tag #{} references type index {}, but only {} types exist",
                i,
                type_idx,
                module.types.len()
            ))
        })?;
        if !ty.results.is_empty() {
            return Err(ValidationError::Other(format!(
                "tag #{i}: non-empty tag result type"
            )));
        }
    }

    // ── Check 4c: ConcreteFuncRef type indices inside declared signatures ─
    //
    // W11 addendum, security-review round: `wasm-wast-parser` accepts a
    // BARE NUMERIC `(ref null N)`/`ref.null N` -- unlike a `$name`
    // reference (always assigned an in-range index at declaration time),
    // a numeric literal has no such guarantee, and `resolve_idx` itself
    // never bounds-checks a plain number against anything. Check 4 above
    // already bounds-checks a FUNCTION's own type index this same way;
    // this closes the analogous gap for a `ValueType::ConcreteFuncRef`
    // index embedded ANYWHERE a declared signature can carry one: a
    // function's own params/results (`module.types`), a global's value
    // type (both declared and imported), and a function body's locals.
    // Deliberately does NOT scan `module.struct_types`' own field types --
    // this crate's `wasm-wast-parser` has no struct-type text-format
    // declarations at all (see `ValueType::ConcreteFuncRef`'s own doc
    // comment), so no TEXT-format module can put one there; scanning it
    // anyway would require the same struct-vs-func index disambiguation
    // `wasm-validator::type_check`'s `0xD0` handler already documents as
    // out of scope for this addendum.
    // Returns the out-of-range index, if `vt` is a `ConcreteFuncRef` whose
    // index is `>= types_len` -- `None` for every other `ValueType`
    // (including an in-range `ConcreteFuncRef`).
    fn out_of_range_concrete_func_ref(vt: &ValueType, types_len: usize) -> Option<u32> {
        match vt {
            ValueType::ConcreteFuncRef(idx) if *idx as usize >= types_len => Some(*idx),
            _ => None,
        }
    }
    for ty in &module.types {
        for vt in ty.params.iter().chain(ty.results.iter()) {
            if let Some(idx) = out_of_range_concrete_func_ref(vt, module.types.len()) {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "a function signature's ref.null references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
        }
    }
    for (i, g) in module.globals.iter().enumerate() {
        if let Some(idx) = out_of_range_concrete_func_ref(&g.global_type.value_type, module.types.len()) {
            return Err(ValidationError::TypeIndexOutOfBounds(format!(
                "global #{i}'s ref.null references type index {idx}, but only {} types exist",
                module.types.len()
            )));
        }
    }
    for imp in &module.imports {
        if let ImportTypeInfo::Global(gt) = &imp.type_info {
            if let Some(idx) = out_of_range_concrete_func_ref(&gt.value_type, module.types.len()) {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "imported global {}.{}'s ref.null references type index {idx}, but only {} types exist",
                    imp.module_name,
                    imp.name,
                    module.types.len()
                )));
            }
        }
    }
    for (i, body) in module.code.iter().enumerate() {
        for vt in &body.locals {
            if let Some(idx) = out_of_range_concrete_func_ref(vt, module.types.len()) {
                return Err(ValidationError::TypeIndexOutOfBounds(format!(
                    "function #{i}'s local ref.null references type index {idx}, but only {} types exist",
                    module.types.len()
                )));
            }
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
    // W21 (exceptions proposal): same combined-index-space convention.
    let total_tags = imported_tags + module.tags.len();

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
            ExternalKind::Tag => {
                if (exp.index as usize) >= total_tags {
                    return Err(ValidationError::Other(format!(
                        "export \"{}\" references tag index {}, but only {} tags exist",
                        exp.name, exp.index, total_tags
                    )));
                }
            }
        }
    }

    // ── Check 8: Data segments ──────────────────────────────────────────
    //
    // Widened (real corpus vendoring pass -- `address0.wast`/`address1.wast`
    // and over a dozen other files use a non-zero `memory_index`, e.g.
    // `(data (memory $mem1) (i32.const 0) "...")`) from the prior "must be
    // 0" rule to a real bounds check against `total_memories`, mirroring
    // every other multi-memory check in this file (`memory.init`/
    // `memory.fill`/etc. in `type_check.rs` already bounds-check the same
    // way). `wasm-runtime::instantiate()` now applies each ACTIVE segment
    // to its own `seg.memory_index`, not unconditionally memory 0 -- see
    // that function's own doc comment.
    //
    // A PASSIVE segment (`is_passive`, task #95) carries no real memory
    // reference at all -- `seg.memory_index` is kept `0`/unset by
    // convention (see `DataSegment.memory_index`'s own doc comment) and
    // is never applied to any memory at instantiation time, so it must
    // never be bounds-checked against `total_memories` here: a passive
    // segment is legal in a module that declares NO memory whatsoever
    // (`token.wast`'s own `(data $l "a")` with no `(memory ...)` anywhere
    // in the module -- the bytes just sit there until some OTHER module's
    // `memory.init` copies from it). Skipping passive segments entirely
    // was a real, pre-existing bug this same pass fixed: the old check's
    // `total_memories == 0 && !module.data.is_empty()` branch didn't look
    // at `is_passive` at all, so a passive-only, memory-less module was
    // wrongly rejected here.
    for (i, seg) in module.data.iter().enumerate() {
        if seg.is_passive {
            continue;
        }
        if (seg.memory_index as usize) >= total_memories {
            return Err(ValidationError::InvalidDataSegment(format!(
                "data segment #{} references memory index {}, but only {} memories exist",
                i, seg.memory_index, total_memories
            )));
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
    //
    // Passive segments (task #97): `elem.table_index` is meaningless
    // (unset, `0` by convention) for a passive segment -- it names no
    // real target table at declaration time, `table.init` supplies one
    // per-call instead -- so a module can declare a passive element
    // segment (and later `table.init`/`elem.drop` it) with ZERO tables
    // declared at all, same "a module with zero memories can still
    // declare a passive data segment" allowance task #95 established for
    // `memory.init`/`data.drop`. The table-existence/bounds checks below
    // apply only to ACTIVE segments, which really do target a real table
    // at instantiation time.
    for (i, elem) in module.elements.iter().enumerate() {
        if !elem.is_passive {
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
        }
        // Validate function indices within element segments. A `None`
        // entry (`ref.null`, task #97) names no function at all, so
        // there is nothing to bounds-check for it.
        for idx in elem.function_indices.iter().flatten() {
            if (*idx as usize) >= total_functions {
                return Err(ValidationError::FuncIndexOutOfBounds(format!(
                    "element segment #{} references function index {}, \
                     but only {} functions exist",
                    i, idx, total_functions
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

    // ── Check 4c: ConcreteFuncRef bounds (W11 addendum, security-review round) ──
    //
    // A bare NUMERIC `(ref null N)` has no declaration-time guarantee its
    // index is in range (unlike a `$name` reference) -- `wasm-wast-parser`
    // will happily produce a `ConcreteFuncRef` with an out-of-range index,
    // so `validate` itself must catch it.

    #[test]
    fn rejects_out_of_range_concrete_func_ref_in_function_result() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(99)] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![], code: vec![0x0B] }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TypeIndexOutOfBounds(_)), "{err:?}");
    }

    #[test]
    fn accepts_in_range_concrete_func_ref_in_function_result() {
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },
                FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(0)] },
            ],
            functions: vec![1],
            code: vec![FunctionBody { locals: vec![], code: vec![0xD0, 0x63, 0x00, 0x0B] }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_out_of_range_concrete_func_ref_in_global_type() {
        let module = WasmModule {
            globals: vec![Global {
                global_type: GlobalType { value_type: ValueType::ConcreteFuncRef(7), mutable: false },
                init_expr: vec![0xD0, 0x63, 0x07, 0x0B],
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TypeIndexOutOfBounds(_)), "{err:?}");
    }

    #[test]
    fn rejects_out_of_range_concrete_func_ref_in_local() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![ValueType::ConcreteFuncRef(5)], code: vec![0x0B] }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TypeIndexOutOfBounds(_)), "{err:?}");
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
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false })
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
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false })
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
            memories: vec![MemoryType { limits: Limits { min: 65537, max: None }, shared: false, is64: false }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn accepts_a_memory_declaring_exactly_65536_pages() {
        let module = WasmModule {
            memories: vec![MemoryType { limits: Limits { min: 65536, max: None }, shared: false, is64: false }],
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
                MemoryType { limits: Limits { min: 65536, max: None }, shared: false, is64: false },
                MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false },
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
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false })
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
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false })
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
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64 + 1, max: None },
             is64: false,}],
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
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
             is64: false,}],
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
                    limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64, max: None },
                    is64: false,
                },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    // ── table64 (W26) Check 2b is64-awareness ────────────────────────────

    /// table64's own real spec ceiling is `u64::MAX` (verified live
    /// against the reference interpreter's `check_tabletype`), NOT the
    /// same `2^48`-page bound memory64 uses -- see code/specs/
    /// W26-wasm-table64-first-slice.md. An `is64` table whose `min` is far
    /// past this interpreter's own `MAX_TABLE_ELEMENTS` implementation
    /// resource limit must still validate successfully (only actual
    /// instantiation enforces a practical cap, via
    /// `Table::new_with_is64`).
    #[test]
    fn accepts_an_is64_table_declaring_far_more_than_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: u64::MAX, max: None }, is64: true }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module));
    }

    /// A plain (`is64: false`) table declaring more than `MAX_TABLE_ELEMENTS`
    /// must still be rejected exactly as before -- this slice's `is64`
    /// branch must not accidentally loosen the existing 32-bit check.
    #[test]
    fn still_rejects_a_32bit_table_declaring_more_than_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: wasm_execution::MAX_TABLE_ELEMENTS as u64 + 1, max: None },
                is64: false,
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    /// An `is64` table's `min` must NOT be added to the 32-bit aggregate --
    /// a huge `is64` table's `min` alongside a small, otherwise-fine 32-bit
    /// table must not spuriously trip the 32-bit aggregate cap.
    #[test]
    fn is64_table_min_is_excluded_from_the_32bit_aggregate() {
        let module = WasmModule {
            tables: vec![
                TableType { element_type: 0x70, limits: Limits { min: u64::MAX, max: None }, is64: true },
                TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false },
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module));
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
                is_passive: false,
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
                is64: false,
            }],
            data: vec![DataSegment {
                memory_index: 1, // out of bounds -- only memory 0 exists
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01],
                is_passive: false,
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
                is64: false,
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01, 0x02],
                is_passive: false,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    // Real corpus vendoring pass (`address0.wast`/`address1.wast` and
    // over a dozen other files, see `wasm-conformance`'s CHANGELOG):
    // Check 8 now accepts an active data segment targeting any IN-BOUNDS
    // memory index, not just 0 -- `wasm-runtime::instantiate()` was
    // widened in the same pass to actually apply it to that memory.
    #[test]
    fn valid_data_segment_targets_non_zero_memory_in_multi_memory_module() {
        let module = WasmModule {
            memories: vec![
                MemoryType { limits: Limits { min: 0, max: None }, shared: false, is64: false },
                MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false },
            ],
            data: vec![DataSegment {
                memory_index: 1,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: vec![0x01, 0x02],
                is_passive: false,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    // Real bug this same pass fixed: a PASSIVE segment (`token.wast`'s own
    // `(data $l "a")` with no `(memory ...)` anywhere in the module) has no
    // real memory reference at all and must never be bounds-checked --
    // legal even when the module declares zero memories.
    #[test]
    fn valid_passive_data_segment_in_module_with_no_memory() {
        let module = WasmModule {
            data: vec![DataSegment {
                memory_index: 0, // unset-by-convention; irrelevant when passive
                offset_expr: vec![],
                data: vec![0x01, 0x02],
                is_passive: true,
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
                function_indices: vec![Some(0)],
                is_passive: false,
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
             is64: false,}],
            elements: vec![Element {
                table_index: 1, // only 0 is valid
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![Some(0)],
                is_passive: false,
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
             is64: false,}],
            elements: vec![Element {
                table_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![Some(99)], // out of bounds
                is_passive: false,
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
             is64: false,}],
            elements: vec![Element {
                table_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                function_indices: vec![Some(0)],
                is_passive: false,
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
                is64: false,
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
                    is64: false,
                }),
            }],
            memories: (0..wasm_execution::MAX_MEMORIES)
                .map(|_| MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false })
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
                 is64: false,}),
            }],
            tables: (0..wasm_execution::MAX_TABLES)
                .map(|_| TableType { element_type: 0x70, limits: Limits { min: 1, max: None }, is64: false })
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

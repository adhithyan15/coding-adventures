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
use std::rc::Rc;
use wasm_types::{CanonicalGroup, ExternalKind, ImportTypeInfo, ValueType, WasmModule};

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
    /// This module's own canonical type-group forms (W34 first slice:
    /// `code/specs/W34-wasm-gc-canonical-type-equivalence.md`) -- one
    /// entry per flat type-section index, `None` for a type this slice
    /// does not yet canonicalize (any type belonging to a
    /// `rec_group_size > 1` group -- multi-member De Bruijn numbering is a
    /// later slice's job, see the spec's own "Recommended slice
    /// decomposition"). See [`wasm_types::canonicalize_types`]'s own doc
    /// comment for the algorithm and its termination argument.
    ///
    /// Computed exactly once, here in [`validate`], immediately after
    /// `type_check::type_check_module` confirms (among everything else)
    /// that the module's declared `sub` chains are acyclic --
    /// canonicalization's own termination argument depends on that
    /// ordering guarantee already holding (see `canonicalize_types`'s doc
    /// comment). `ValidatedModule`'s `module` field is already private,
    /// with [`validate`] the only constructor (a struct-literal bypass is
    /// a compile error -- see this struct's own doc comment above); this
    /// field inherits that exact same guarantee for free, by construction
    /// -- there is no code path that can produce a `ValidatedModule`
    /// (and therefore no path that can produce a `canonical_types` value)
    /// without going through `validate()` first.
    canonical_types: Vec<Option<(Rc<CanonicalGroup>, u32)>>,
}

impl ValidatedModule {
    /// The underlying parsed module, proven to have passed [`validate`].
    pub fn module(&self) -> &WasmModule {
        &self.module
    }

    /// This flat type-section index's own canonical type-group form (W34
    /// first slice), or `None` if this slice doesn't canonicalize it yet
    /// (out of range, or a `rec_group_size > 1` member -- see
    /// [`wasm_types::canonicalize_types`]'s own doc comment).
    pub fn canonical_type_at(&self, idx: u32) -> Option<(Rc<CanonicalGroup>, u32)> {
        self.canonical_types.get(idx as usize).cloned().flatten()
    }

    /// Whether flat type-section indices `i` and `j` are canonically
    /// equivalent per this slice's own (singleton-groups-only) algorithm --
    /// `false`, conservatively, whenever EITHER side isn't canonicalized
    /// yet (out of range, or a `rec_group_size > 1` member), never a wrong
    /// `true`. This slice does not wire this into any validator/execution
    /// call site itself (that's a later slice, per the spec's own "Wiring
    /// into within-module checks" section) -- it exists here purely so the
    /// mechanism is directly testable end-to-end through the real
    /// `validate()` entry point, not just through the lower-level
    /// `wasm_types::canonicalize_types` free function.
    pub fn canonically_equivalent(&self, i: u32, j: u32) -> bool {
        wasm_types::canonical_types_equivalent(&self.canonical_types, i, j)
    }

    /// The whole per-flat-index canonical-type table (W34 third slice) --
    /// exposed as a slice so a downstream crate that needs to CARRY this
    /// data further (`wasm-runtime::instantiate`, threading it into
    /// `WasmInstance::canonical_types` for `wasm-execution`'s own runtime
    /// dispatch -- see that crate's own doc comments) can clone it once,
    /// rather than reconstructing an equivalent Vec index-by-index via
    /// repeated [`Self::canonical_type_at`] calls. Read-only: there is no
    /// way to construct a `ValidatedModule` (and therefore no way to reach
    /// this slice) other than [`validate`] succeeding, the same guarantee
    /// [`Self::module`] already relies on.
    pub fn canonical_types(&self) -> &[Option<(Rc<CanonicalGroup>, u32)>] {
        &self.canonical_types
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
///    (imports + module). A table's declared `min` is NOT capped here
///    (gap 2 of the W-next `elem.wast`/`table.wast` investigation pass --
///    the real spec places no ceiling on merely declaring an oversized
///    table; this interpreter's own `MAX_TABLE_ELEMENTS` resource limit
///    is enforced only at actual instantiation/allocation time instead,
///    in `wasm-runtime::instantiate`).
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

    // ── (Former) Check 2b: 32-bit table `min` ≤ MAX_TABLE_ELEMENTS ──────
    //
    // REMOVED (gap 2 of the W-next `elem.wast`/`table.wast` investigation
    // pass, `code/specs/W07-wasm-post-mvp-epics.md`'s addendum) -- this
    // check used to reject, AT STRUCTURAL VALIDATION TIME, any 32-bit
    // table whose declared `min` exceeded this interpreter's own
    // `MAX_TABLE_ELEMENTS` resource-limit heuristic (both per-table and,
    // below, summed across every 32-bit table). Its own doc comment
    // already said as much -- "unlike Check 1b above, this is NOT a real
    // spec requirement... it's this interpreter's own implementation-
    // defined resource limit" -- but that comment stopped short of the
    // conclusion table.wast's real corpus makes unavoidable: an
    // implementation MAY refuse to actually ALLOCATE an oversized table,
    // but the real spec places no ceiling on merely DECLARING one, and a
    // module that only declares (never instantiates) such a table is
    // genuinely valid. `table.wast`'s own `(module definition (table
    // 0xffff_ffff funcref))` -- a bare, unwrapped directive (no
    // `assert_invalid` around it) -- is exactly this: the official
    // testsuite itself asserts this declaration validates. `wasm-
    // validator` rejecting it here was a real conformance bug, not a
    // defensible implementation choice, exactly the same "declare freely,
    // refuse only at real allocation time" shape `is64` tables and BOTH
    // `is64`/32-bit MEMORIES already use elsewhere in this same function
    // (Check 1b above defers a 64-bit memory's own, much larger, spec
    // ceiling to `wasm-runtime::instantiate`'s `LinearMemory::
    // new_with_is64`; `is64` tables already deferred to `Table::
    // new_with_is64` the same way -- this fix just extends that same
    // discipline to 32-bit tables' `min`, closing the inconsistency
    // rather than leaving it as a 32-bit-only special case).
    //
    // Not a DoS regression: `Table::new_with_is64` (the constructor
    // EVERY declared table already goes through in `wasm-runtime::
    // instantiate`, `is64` or not) already has its own real,
    // unconditional per-table `MAX_TABLE_ELEMENTS` cap, returning a
    // graceful `TrapError` instead of attempting the allocation -- see
    // that constructor's own doc comment ("the cap is checked
    // UNCONDITIONALLY, not only when `is64`"). This structural check was
    // always redundant with that one for the single-table case; the only
    // real gap closed by moving instead of just deleting is the
    // AGGREGATE case (many individually-under-cap tables summing past
    // it) -- `wasm-runtime::instantiate`'s own table-aggregate check
    // (previously `is64`-only, generalized by this same fix to cover
    // every table) now closes that for 32-bit tables too, the same place
    // the equivalent `is64` aggregate already lived.
    //
    // The real spec-mandated "`min` <= `max`" rule (Check 1c above) is
    // UNCHANGED and still enforced here at validation time -- only the
    // extra, non-spec resource-limit heuristic moved.

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
    // Returns the out-of-range index, if `vt` is a `ConcreteFuncRef` (or,
    // W32 second slice, its non-null counterpart `NonNullConcreteFuncRef`
    // -- same func-type index space, same `0..types_len` bound, see that
    // variant's own doc comment) whose index is `>= types_len` -- `None`
    // for every other `ValueType` (including an in-range one).
    //
    // W32 second slice: real corpus regression found (not a pre-existing
    // gap this slice merely exposed by coincidence) -- `wasm-wast-parser`
    // happily produces a `NonNullConcreteFuncRef` with an out-of-range
    // index (`resolve_idx` on a bare numeric atom just parses the digits,
    // it does not bounds-check them, exactly like `ConcreteFuncRef`'s own
    // doc comment on this function's test already explains for the
    // nullable case), and this repo's real corpus (`ref.wast`'s own
    // `(module (type $type-func-param-invalid (func (param (ref
    // 1)))))"unknown type"` etc.) genuinely exercises the NON-null form
    // of exactly the check this function already performed for the
    // nullable one -- without this arm, five `ref.wast` `assert_invalid`
    // cases that used to pass only because non-null `(ref $t)` was
    // entirely unparseable (a lucky parse-failure `Pass`, not a real
    // check) would silently validate instead once this slice's `(ref $t)`
    // parsing landed.
    fn out_of_range_concrete_func_ref(vt: &ValueType, types_len: usize) -> Option<u32> {
        match vt {
            ValueType::ConcreteFuncRef(idx) | ValueType::NonNullConcreteFuncRef(idx) if *idx as usize >= types_len => Some(*idx),
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
    //
    // W34 first slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`),
    // updated by the third slice: `type_check_module` itself now computes
    // and returns this module's own canonicalized type-group forms as a
    // side product of Check 11 -- it needs them internally anyway (wired
    // into `is_assignable`/`call_indirect`'s static checks via
    // `ModuleContext`/`TypeContext`, see that crate-internal module's own
    // doc comments), computed right after `check_type_subtyping` (which
    // runs `check_type_subtyping_is_acyclic` as its own first step) has
    // confirmed the module's `sub`/`rec` reference ordering is well-founded
    // -- see `wasm_types::canonicalize_types`'s own doc comment for why
    // that ordering guarantee matters even though this function itself
    // never recurses. Reusing the same computation here (rather than
    // calling `canonicalize_types` a second time) keeps this a true
    // "computed exactly once per module" cache, matching `ValidatedModule::
    // canonical_types`'s own doc comment.
    let canonical_types = type_check::type_check_module(module)?;

    Ok(ValidatedModule {
        module: module.clone(),
        canonical_types,
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

    // ── W-addendum 2026-09-01 pass: memarg/sub-opcode LEB128 strictness ─────
    //
    // Real corpus bugs (`binary-leb128.wast`'s memarg align/offset overlong
    // and out-of-range `assert_malformed` cases, and its 0xFC sub-opcode
    // overlong case): a memory instruction's `align`/`offset` immediates,
    // and a `0xFC`/`0xFD`-prefixed instruction's sub-opcode immediate, were
    // all decoded via the native-64-bit-budget `decode_unsigned` instead of
    // a width-bounded decode, so a 6+-byte or high-bit-set encoding of a
    // small value parsed successfully instead of being rejected. Each
    // negative case here has a matching positive-control case proving the
    // FIX didn't also reject the ordinary minimal encoding of the exact
    // same value.

    /// A minimal single-function module with one declared memory, whose
    /// only function body is the given code -- the common shape every
    /// memarg test below needs. `access_i32` picks a natural-alignment-2
    /// memory op (`i32.load`) so `align` values up to `2` are legal.
    fn module_with_memory_and_code(is64: bool, code: Vec<u8>) -> WasmModule {
        WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64 }],
            code: vec![FunctionBody { locals: vec![], code }],
            ..Default::default()
        }
    }

    /// `i32.load` with `align` encoded as an overlong (6-byte, one more
    /// than the `ceil(32/7) = 5`-byte budget) LEB128 of the otherwise
    /// perfectly legal value `2` -- exactly `binary-leb128.wast`'s
    /// "alignment 2 with one byte too many" case (adapted from the raw
    /// binary form to a directly-constructed `WasmModule`, since this
    /// crate validates bytecode either way).
    #[test]
    fn memarg_align_overlong_leb128_is_rejected() {
        let code = vec![
            0x41, 0x00, // i32.const 0
            0x28, // i32.load
            0x82, 0x80, 0x80, 0x80, 0x80, 0x00, // align 2, overlong (6 bytes)
            0x00, // offset 0
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(false, code);
        assert!(validate(&module).is_err(), "overlong memarg align must be rejected");
    }

    /// The exact same `align`/`offset`/instruction shape as the previous
    /// test, but with `align` encoded MINIMALLY (1 byte) -- must still
    /// validate fine. Guards against the fix over-rejecting.
    #[test]
    fn memarg_align_minimal_leb128_still_parses() {
        let code = vec![
            0x41, 0x00, // i32.const 0
            0x28, // i32.load
            0x02, // align 2, minimal
            0x00, // offset 0
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(false, code);
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    /// `i32.load` with `offset` encoded overlong (6 bytes) on a plain
    /// (32-bit) memory -- `binary-leb128.wast`'s "offset 2 with one byte
    /// too many" case.
    #[test]
    fn memarg_offset_overlong_leb128_is_rejected_on_32bit_memory() {
        let code = vec![
            0x41, 0x00, // i32.const 0
            0x28, // i32.load
            0x02, // align 2, minimal
            0x82, 0x80, 0x80, 0x80, 0x80, 0x00, // offset 2, overlong (6 bytes)
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(false, code);
        assert!(validate(&module).is_err(), "overlong memarg offset on a 32-bit memory must be rejected");
    }

    /// `i32.load` with `align` encoded in the full 5-byte `u32` budget but
    /// with an out-of-range high bit set (`\x82\x80\x80\x80\x10` decodes
    /// bit 32, one past the 32-bit width) -- `binary-leb128.wast`'s
    /// "alignment 2 with unused bits set" case.
    #[test]
    fn memarg_align_out_of_range_high_bit_is_rejected() {
        let code = vec![
            0x41, 0x00, // i32.const 0
            0x28, // i32.load
            0x82, 0x80, 0x80, 0x80, 0x10, // align 2, bit 32 spuriously set
            0x00, // offset 0
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(false, code);
        assert!(validate(&module).is_err(), "out-of-range memarg align must be rejected");
    }

    /// `binary_leb128_64.wast`'s own pair of cases, the reason `offset`
    /// can't just be blanket-narrowed to 32 bits: on an `is64` (memory64)
    /// memory, `offset` genuinely needs the full 64-bit budget. A 10-byte
    /// encoding of `2^64 - 1` must parse fine...
    #[test]
    fn memarg_offset_widens_to_64_bits_on_is64_memory() {
        let code = vec![
            0x42, 0x00, // i64.const 0 (address operand is i64 for an is64 memory)
            0x28, // i32.load
            0x02, // align 2, minimal
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, // offset 2^64 - 1
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(true, code);
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    /// ...but `2^64` itself (one bit further -- out of range even for the
    /// full 64-bit budget) must still be rejected.
    #[test]
    fn memarg_offset_out_of_range_even_at_64_bits_on_is64_memory() {
        let code = vec![
            0x42, 0x00, // i64.const 0
            0x28, // i32.load
            0x02, // align 2, minimal
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02, // offset 2^64
            0x1A, // drop
            0x0B, // end
        ];
        let module = module_with_memory_and_code(true, code);
        assert!(validate(&module).is_err(), "offset 2^64 must be rejected even on an is64 memory");
    }

    /// `binary-leb128.wast`'s "i64_trunc_sat_f64_u with 6 bytes" case: the
    /// `0xFC`-prefixed sub-opcode is a `u32` LEB128, same overlong rule as
    /// every other `u32` field.
    #[test]
    fn fc_prefixed_sub_opcode_overlong_leb128_is_rejected() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![ValueType::F64], results: vec![ValueType::I64] }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x20, 0x00, // local.get 0
                    0xFC, 0x87, 0x80, 0x80, 0x80, 0x80, 0x00, // i64.trunc_sat_f64_u (sub-opcode 7), overlong (6 bytes)
                    0x0B, // end
                ],
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_err(), "overlong 0xFC sub-opcode must be rejected");
    }

    /// Same instruction, sub-opcode encoded minimally (1 byte) -- must
    /// still validate fine.
    #[test]
    fn fc_prefixed_sub_opcode_minimal_leb128_still_parses() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![ValueType::F64], results: vec![ValueType::I64] }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x20, 0x00, // local.get 0
                    0xFC, 0x07, // i64.trunc_sat_f64_u (sub-opcode 7), minimal
                    0x0B, // end
                ],
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    // ── W-addendum 2026-09-01 pass: data count section required for
    //    memory.init/data.drop (`binary.wast`'s own two `assert_malformed`
    //    cases -- a different root cause than the LEB128 strictness gaps
    //    above, but found and fixed in the same pass) ──────────────────────

    /// `memory.init` with `missing_data_count_section: true` must be
    /// rejected, even when the referenced data segment index is perfectly
    /// in-bounds -- the data-count-section check fires FIRST.
    #[test]
    fn memory_init_without_data_count_section_is_rejected() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
            data: vec![DataSegment { memory_index: 0, offset_expr: vec![0x41, 0x00, 0x0B], data: vec![], is_passive: false }],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x41, 0x00, // i32.const 0 (dest)
                    0x41, 0x00, // i32.const 0 (src)
                    0x41, 0x00, // i32.const 0 (len)
                    0xFC, 0x08, 0x00, 0x00, // memory.init 0 0
                    0x0B, // end
                ],
            }],
            missing_data_count_section: true,
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(format!("{err}").contains("data count section"), "{err:?}");
    }

    /// The same module, but with `missing_data_count_section: false` (as a
    /// real binary WOULD set it, had it declared §12) -- `memory.init`
    /// must validate fine.
    #[test]
    fn memory_init_with_data_count_section_parses_fine() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            memories: vec![MemoryType { limits: Limits { min: 1, max: None }, shared: false, is64: false }],
            data: vec![DataSegment { memory_index: 0, offset_expr: vec![0x41, 0x00, 0x0B], data: vec![], is_passive: false }],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0xFC, 0x08, 0x00, 0x00, 0x0B],
            }],
            missing_data_count_section: false,
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    /// `data.drop` with `missing_data_count_section: true` must be
    /// rejected, same as `memory.init` above.
    #[test]
    fn data_drop_without_data_count_section_is_rejected() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            functions: vec![0],
            data: vec![DataSegment { memory_index: 0, offset_expr: vec![], data: vec![], is_passive: true }],
            code: vec![FunctionBody { locals: vec![], code: vec![0xFC, 0x09, 0x00, 0x0B] }],
            missing_data_count_section: true,
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(format!("{err}").contains("data count section"), "{err:?}");
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

    // ── W32 first slice: `NullRef <: StructRef(_)` ──────────────────────────
    //
    // `wasm-wast-parser` has no struct-type TEXT-format declarations at all
    // (see `ValueType::StructRef`'s own doc comment), so this direction of
    // the bottom-type lattice -- unlike the func/extern/exn-hierarchy ones,
    // covered via WAT text in `tests/type_check.rs` -- needs a directly
    // constructed `WasmModule`, matching this file's own
    // `accepts_in_range_concrete_func_ref_in_function_result` pattern just
    // above.

    #[test]
    fn accepts_nullref_flowing_into_a_structref_result() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::StructRef(0)] }],
            functions: vec![0],
            // ref.null none (0xD0 0x71) -- pushes NullRef; end.
            code: vec![FunctionBody { locals: vec![], code: vec![0xD0, 0x71, 0x0B] }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    // ── W32 second slice: `NonNullStructRef(i) <: StructRef(i) <: Anyref` ────
    //
    // Same "no struct-type TEXT-format declarations exist" limitation as the
    // first slice's `NullRef <: StructRef(_)` tests just above -- a
    // `NonNullStructRef`-typed LOCAL, read via `local.get`, is this crate's
    // only way to get a statically-`Known(NonNullStructRef(_))` value onto
    // the stack without a real `struct.new` instruction (not implemented by
    // this repo's execution engine yet -- irrelevant here, since this is a
    // pure STATIC type-check, never actually run).

    #[test]
    fn accepts_non_null_structref_flowing_into_a_structref_result_same_index() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::StructRef(0)] }],
            functions: vec![0],
            // local.get 0; end -- local 0 is a NonNullStructRef(0).
            code: vec![FunctionBody { locals: vec![ValueType::NonNullStructRef(0)], code: vec![0x20, 0x00, 0x0B] }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    #[test]
    fn accepts_non_null_structref_flowing_into_an_anyref_result() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::Anyref] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![ValueType::NonNullStructRef(0)], code: vec![0x20, 0x00, 0x0B] }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    #[test]
    fn rejects_structref_flowing_into_a_non_null_structref_result() {
        // The reverse direction never holds -- a NULLABLE `StructRef` carries
        // no static guarantee it's actually non-null, so it cannot stand in
        // for the non-null slot (the exact asymmetry this slice's own
        // "Verification plan" calls out).
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::NonNullStructRef(0)] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![ValueType::StructRef(0)], code: vec![0x20, 0x00, 0x0B] }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_non_null_structref_flowing_into_a_mismatched_index() {
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::StructRef(1)] }],
            functions: vec![0],
            code: vec![FunctionBody { locals: vec![ValueType::NonNullStructRef(0)], code: vec![0x20, 0x00, 0x0B] }],
            ..Default::default()
        };
        assert!(validate(&module).is_err());
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

    /// Gap 2 of the W-next `elem.wast`/`table.wast` investigation pass
    /// (`code/specs/W07-wasm-post-mvp-epics.md`'s addendum): a 32-bit
    /// table declaring far more than this interpreter's own
    /// `MAX_TABLE_ELEMENTS` resource-limit heuristic must still validate
    /// successfully -- the real spec allows a 32-bit table `min` up to
    /// `2^32 - 1`, and merely DECLARING a table never allocates anything.
    /// `table.wast`'s own real corpus case, `(module definition (table
    /// 0xffff_ffff funcref))` (`u32::MAX`, no `max`), is exactly this
    /// shape -- a bare, unwrapped directive the official testsuite itself
    /// asserts must validate. This replaces the OLD `rejects_a_table_
    /// declaring_more_than_max_table_elements` test, which asserted the
    /// opposite (now-corrected) behavior. The practical resource limit is
    /// still enforced for real, just moved to actual instantiation time
    /// (`Table::new_with_is64` in `wasm-execution`, called from
    /// `wasm-runtime::instantiate`) -- see that constructor's own tests
    /// for the "declares fine, fails to instantiate" half of this story.
    #[test]
    fn accepts_a_32bit_table_declaring_far_more_than_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: u32::MAX as u64, max: None },
                is64: false,
            }],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module));
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

    /// The former aggregate check (many individually-under-cap 32-bit
    /// tables summing past `MAX_TABLE_ELEMENTS`) also moved to
    /// `wasm-runtime::instantiate` alongside the per-table check above --
    /// validation alone must accept this now, same reasoning as the
    /// single-table case.
    #[test]
    fn accepts_tables_whose_combined_elements_exceed_the_old_aggregate_cap() {
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
        assert!(validate(&module).is_ok(), "{:?}", validate(&module));
    }

    // ── table64 (W26): is64 tables already worked this way ──────────────

    /// table64's own real spec ceiling is `u64::MAX` (verified live
    /// against the reference interpreter's `check_tabletype`), NOT the
    /// same `2^48`-page bound memory64 uses -- see code/specs/
    /// W26-wasm-table64-first-slice.md. An `is64` table whose `min` is far
    /// past this interpreter's own `MAX_TABLE_ELEMENTS` implementation
    /// resource limit must still validate successfully (only actual
    /// instantiation enforces a practical cap, via
    /// `Table::new_with_is64`). Since gap 2's fix above, 32-bit tables now
    /// share this exact same "declare freely, cap only at instantiation"
    /// treatment -- this test's own assertion was already correct before
    /// that fix; it's kept to confirm the fix didn't regress it.
    #[test]
    fn accepts_an_is64_table_declaring_far_more_than_max_table_elements() {
        let module = WasmModule {
            tables: vec![TableType { element_type: 0x70, limits: Limits { min: u64::MAX, max: None }, is64: true }],
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
                is_declarative: false,
                item_exprs: vec![],
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
                is_declarative: false,
                item_exprs: vec![],
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
                is_declarative: false,
                item_exprs: vec![],
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
                is_declarative: false,
                item_exprs: vec![],
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

    // ── W33 first slice: GC `sub`/`final` nominal subtyping (`code/specs/
    // W33-wasm-gc-recursive-type-subtyping.md`) ─────────────────────────────

    #[test]
    fn accepts_a_valid_nominal_subtype_chain_matching_the_real_corpus_shape() {
        // Mirrors `type-subtyping.wast`'s own "Subsumption" 3-cycle
        // exactly (lines 68-73): `$t1 (func (param i32 (ref $t3)))`, `$t2
        // (sub $t1 (func (param i32 (ref $t2))))` (self-referencing),
        // `$t3 (sub $t2 (func (param i32 (ref $t1))))` -- contravariant
        // params satisfied at every declared `sub` step via the real
        // subtype chain (`$t3 <: $t2 <: $t1`), not mere structural
        // equality (the params at each step are DIFFERENT concrete
        // indices).
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::I32, ValueType::NonNullConcreteFuncRef(2)], results: vec![] }, // $t1
                FuncType { params: vec![ValueType::I32, ValueType::NonNullConcreteFuncRef(1)], results: vec![] }, // $t2 sub $t1
                FuncType { params: vec![ValueType::I32, ValueType::NonNullConcreteFuncRef(0)], results: vec![] }, // $t3 sub $t2
            ],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(1), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    #[test]
    fn rejects_a_sub_declaration_whose_parent_is_final() {
        // `type-subtyping.wast` lines 780-786: a type with NO `sub` clause
        // at all defaults to final -- declaring a further sub of it must
        // be rejected.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping::default(), // final by default, no `sub` clause
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_sub_declaration_whose_explicit_final_parent_forecloses_it() {
        // `type-subtyping.wast` lines 796-802: `(sub final (func))`
        // followed by an attempted `(sub $t (func))` — an EXPLICIT
        // `final` forecloses just as much as the implicit default above.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: true, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn accepts_a_sub_of_a_non_final_parent() {
        // The positive counterpart of the two finality tests above: an
        // OPEN (non-final) parent's sub declaration must be accepted.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn rejects_a_sub_declaration_with_mismatched_arity() {
        // `type-subtyping.wast` lines 944-949: `$f0 (sub (func))`, `$f1
        // (sub $f0 (func (param i32)))` -- the GC proposal's function
        // subtyping rule requires INVARIANT arity; adding a param is
        // rejected even though the parent is open (non-final).
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![ValueType::I32], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_sub_declaration_violating_param_contravariance() {
        // A declared `sub` whose param goes the WRONG way (covariant
        // instead of contravariant) must be rejected: the parent func
        // type's param is `(ref $wide)`; the declared "sub" narrows it to
        // `(ref $narrow)`, a STRICT nominal subtype of `$wide` -- but
        // params must WIDEN (or stay equal), never narrow, in a real
        // subtype, so this must be rejected even though `$narrow <:
        // $wide` genuinely holds (it's just the wrong position for it).
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },                                     // 0: $wide (marker, final=false)
                FuncType { params: vec![], results: vec![] },                                     // 1: $narrow, sub $wide
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(0)], results: vec![] },  // 2: parent func type -- param (ref $wide)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(1)], results: vec![] },  // 3: declared "sub" of #2 -- param (ref $narrow), WRONG direction
            ],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(2), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_cyclic_sub_declaration() {
        // Security review finding (W33 first slice): `(rec (type $t1 (sub
        // $t2 (func))) (type $t2 (sub $t1 (func))))` -- two types
        // declared as mutual `sub`s of EACH OTHER. Each individual link
        // structurally checks out fine in isolation (empty/empty func
        // shapes, invariant arity trivially satisfied both ways), so
        // without a dedicated cycle check this validates successfully
        // and makes `func_type_is_nominal_subtype(0, 1)` AND `(1, 0)`
        // both `true` -- two independently-declared, differently-indexed
        // types becoming mutually interchangeable, exactly the
        // "canonical equivalence between unrelated types" this slice's
        // own scope says must stay unimplemented (a wrong ACCEPT here is
        // a real soundness risk).
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { supertype: Some(1), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_self_cyclic_sub_declaration() {
        // The degenerate 1-cycle: a type declared as its OWN supertype.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn rejects_a_longer_cyclic_sub_chain() {
        // A 3-cycle: t0 sub t2, t1 sub t0, t2 sub t1 -- no individual
        // link is a self-reference, but the whole chain loops.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }; 3],
            type_subtyping: vec![
                TypeSubtyping { supertype: Some(2), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(1), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    #[test]
    fn accepts_a_module_with_multiple_independent_acyclic_sub_chains() {
        // Regression guard for the cycle checker itself: it must not
        // reject legitimate, SEPARATE (non-interacting) acyclic chains
        // sharing one module's type section, and its "already proven
        // acyclic" (BLACK) memoization must not cause it to skip
        // checking a later, independent chain.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }; 4],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(2), is_final: false, ..Default::default() },
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok());
    }

    #[test]
    fn call_argument_accepts_a_declared_nominal_subtype_concrete_func_ref() {
        // Integration test for `is_assignable`'s new W33 arms via a real
        // `call`: `$f1`'s function type takes a `(ref $t1)` param; `$f2`'s
        // takes a `(ref $t2)` param where `$t2 sub $t1`. `$f2`'s body
        // calls `$f1` passing its OWN param straight through -- valid
        // because `(ref $t2) <: (ref $t1)` per the declared chain, the
        // exact mechanic `type-subtyping.wast`'s "Subsumption" section
        // tests via real function bodies, not just type-section shape.
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },                                      // 0: $t1 (final=false)
                FuncType { params: vec![], results: vec![] },                                      // 1: $t2 sub $t1
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(0)], results: vec![] },   // 2: $f1's type -- param (ref $t1)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(1)], results: vec![] },   // 3: $f2's type -- param (ref $t2)
            ],
            type_subtyping: vec![
                TypeSubtyping { supertype: None, is_final: false, ..Default::default() },
                TypeSubtyping { supertype: Some(0), is_final: false, ..Default::default() },
                TypeSubtyping::default(),
                TypeSubtyping::default(),
            ],
            functions: vec![2, 3],
            code: vec![
                FunctionBody { locals: vec![], code: vec![0x0B] }, // $f1: drop param, done
                FunctionBody { locals: vec![], code: vec![0x20, 0x00, 0x10, 0x00, 0x0B] }, // $f2: local.get 0; call 0; end
            ],
            ..Default::default()
        };
        assert!(validate(&module).is_ok(), "{:?}", validate(&module).unwrap_err());
    }

    #[test]
    fn call_argument_rejects_an_unrelated_concrete_func_ref() {
        // The negative counterpart: `$t1`/`$t2` are declared with NO `sub`
        // relationship between them at all (two independent, final types)
        // AND a genuinely different shape (`$t2` takes an `i32` param
        // `$t1` doesn't) -- passing a `(ref $t2)` where `(ref $t1)` is
        // expected must be rejected. This is deliberately NOT the same as
        // the positive test's types (no `sub` declared here), proving the
        // assignability arms require a REAL declared chain OR real
        // canonical equivalence, not just "any two concrete func ref
        // types."
        //
        // W34 third slice (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`):
        // this test USED to give `$t1`/`$t2` the SAME empty shape, on the
        // premise that byte-identical-but-undeclared-`sub` types stay
        // unrelated -- exactly the gap this slice closes (see the real GC
        // proposal's own "subtyping is nominal modulo canonicalization"
        // rule). That premise is now WRONG (two structurally-identical
        // types genuinely ARE the same canonical type, `sub` or not), so
        // this test's shapes were changed to be genuinely different
        // instead of merely un-declared-as-related, preserving its real
        // purpose (an honest reclassification, not a regression -- see
        // `call_argument_accepts_a_canonically_equivalent_but_nominally_
        // unrelated_concrete_func_ref` just below for the case this test
        // used to, incorrectly, also cover).
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },                                      // 0: $t1
                FuncType { params: vec![ValueType::I32], results: vec![] },                        // 1: $t2 (genuinely different shape, unrelated to $t1)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(0)], results: vec![] },   // 2: $f1's type -- param (ref $t1)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(1)], results: vec![] },   // 3: $f2's type -- param (ref $t2)
            ],
            functions: vec![2, 3],
            code: vec![
                FunctionBody { locals: vec![], code: vec![0x0B] },
                FunctionBody { locals: vec![], code: vec![0x20, 0x00, 0x10, 0x00, 0x0B] }, // local.get 0; call 0; end
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::Other(_)), "{err:?}");
    }

    /// W34 third slice: the positive case the PREVIOUS version of
    /// `call_argument_rejects_an_unrelated_concrete_func_ref` (just above)
    /// used to (incorrectly, per the real GC proposal) also reject --
    /// `$t1`/`$t2` are byte-identical (`(func)`, no params/results) and
    /// declare NO `sub` relationship at all, yet are canonically the SAME
    /// type, so a `(ref $t2)` argument flowing into a `(ref $t1)` param at
    /// a real `call` site must now be ACCEPTED. This is the direct,
    /// end-to-end (`validate()`, not just `is_assignable`/`canonicalize_
    /// types` in isolation) proof this slice's own wiring reaches a real
    /// validation decision.
    #[test]
    fn call_argument_accepts_a_canonically_equivalent_but_nominally_unrelated_concrete_func_ref() {
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![] },                                      // 0: $t1
                FuncType { params: vec![], results: vec![] },                                      // 1: $t2 (canonically == $t1, no `sub` declared)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(0)], results: vec![] },   // 2: $f1's type -- param (ref $t1)
                FuncType { params: vec![ValueType::NonNullConcreteFuncRef(1)], results: vec![] },   // 3: $f2's type -- param (ref $t2)
            ],
            functions: vec![2, 3],
            code: vec![
                FunctionBody { locals: vec![], code: vec![0x0B] },
                FunctionBody { locals: vec![], code: vec![0x20, 0x00, 0x10, 0x00, 0x0B] }, // local.get 0; call 0; end
            ],
            ..Default::default()
        };
        validate(&module).expect("canonically equivalent concrete func refs must be assignable even with no declared `sub` chain");
    }

    // ────────────────────────────────────────────────────────────────────
    // W34 first slice: canonical type-group equivalence, wired through the
    // real `validate()` entry point (not just `wasm_types::canonicalize_
    // types` directly -- see that crate's own extensive unit tests for the
    // mechanism in isolation).
    // ────────────────────────────────────────────────────────────────────

    #[test]
    fn validate_computes_canonical_type_for_a_self_referencing_singleton() {
        // `type-rec.wast` line 4 shape: `(type (func (param (ref 0))
        // (result (ref 0))))`.
        let module = WasmModule {
            types: vec![FuncType { params: vec![ValueType::ConcreteFuncRef(0)], results: vec![ValueType::ConcreteFuncRef(0)] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        let validated = validate(&module).unwrap();
        assert!(validated.canonical_type_at(0).is_some());
        assert!(validated.canonically_equivalent(0, 0));
        // Out of range never panics, just reports "not canonicalized."
        assert_eq!(validated.canonical_type_at(99), None);
    }

    #[test]
    fn validate_canonical_types_compare_equal_across_two_independently_validated_modules() {
        // Cross-module comparability, exercised through the real
        // `validate()` path this time -- two SEPARATE modules, isomorphic
        // shape at different flat indices, no shared numbering.
        let module_a = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![ValueType::I32] }],
            type_subtyping: vec![TypeSubtyping::default()],
            ..Default::default()
        };
        let padding = FuncType { params: vec![], results: vec![] };
        let module_b = WasmModule {
            types: vec![padding.clone(), padding, FuncType { params: vec![], results: vec![ValueType::I32] }],
            type_subtyping: vec![TypeSubtyping::default(); 3],
            ..Default::default()
        };
        let validated_a = validate(&module_a).unwrap();
        let validated_b = validate(&module_b).unwrap();
        assert_eq!(validated_a.canonical_type_at(0), validated_b.canonical_type_at(2));
        assert_ne!(validated_a.canonical_type_at(0), validated_b.canonical_type_at(0));
    }

    #[test]
    fn validate_computes_canonical_types_for_a_real_multi_member_rec_group() {
        // W34 second slice: a real multi-member `rec` group now
        // canonicalizes through the actual `validate()` entry point too,
        // not just `wasm_types::canonicalize_types` directly -- `$h`/`$k`,
        // `type-rec.wast` lines 15-18, `$h -> $k` (`Rec(1)`), `$k -> $h`
        // (`Rec(0)`).
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![ValueType::ConcreteFuncRef(1)], results: vec![] },
                FuncType { params: vec![], results: vec![ValueType::ConcreteFuncRef(0)] },
            ],
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 2, rec_group_position: 1, ..Default::default() },
            ],
            ..Default::default()
        };
        let validated = validate(&module).unwrap();
        assert!(validated.canonical_type_at(0).is_some());
        assert!(validated.canonical_type_at(1).is_some());
        // The two members are NOT canonically equivalent to each other
        // (different bodies -- `$h` takes a param, `$k` returns a
        // result), but each is a stable, self-consistent identity.
        assert!(!validated.canonically_equivalent(0, 1));
        assert!(validated.canonically_equivalent(0, 0));
    }

    #[test]
    fn validate_rejects_canonicalizing_an_internally_inconsistent_rec_group_claim() {
        // A hand-built module whose two "sibling" entries disagree about
        // their own group's size must still VALIDATE fine if nothing else
        // is wrong with it (canonicalization failing is not the same as
        // the module being ill-typed), but canonicalizes to `None` rather
        // than guessing.
        let module = WasmModule {
            types: vec![FuncType { params: vec![], results: vec![] }, FuncType { params: vec![], results: vec![] }],
            type_subtyping: vec![
                TypeSubtyping { rec_group_size: 2, rec_group_position: 0, ..Default::default() },
                TypeSubtyping { rec_group_size: 3, rec_group_position: 1, ..Default::default() },
            ],
            ..Default::default()
        };
        let validated = validate(&module).unwrap();
        assert_eq!(validated.canonical_type_at(0), None);
        assert_eq!(validated.canonical_type_at(1), None);
        assert!(!validated.canonically_equivalent(0, 1));
    }

    #[test]
    fn validate_canonically_equivalent_is_false_for_genuinely_different_shapes() {
        let module = WasmModule {
            types: vec![
                FuncType { params: vec![], results: vec![ValueType::I32] },
                FuncType { params: vec![], results: vec![ValueType::I64] },
            ],
            type_subtyping: vec![TypeSubtyping::default(); 2],
            ..Default::default()
        };
        let validated = validate(&module).unwrap();
        assert!(!validated.canonically_equivalent(0, 1));
    }
}

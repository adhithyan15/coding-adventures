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
//! - Memory and table counts do not exceed 1 (WASM 1.0 restriction).
//! - Data segment memory indices are valid.
//! - Element segment table indices are valid.
//!
//! ## Why Validate?
//!
//! A WASM binary can be syntactically correct (well-formed LEB128, valid
//! section ordering) but semantically wrong (references a type index that
//! does not exist). The parser only checks syntax; the validator checks
//! semantics. This separation of concerns keeps both passes simple.
//!
//! ## Validation vs. Type Checking
//!
//! A full WASM validator would also do stack-based type checking of every
//! instruction in every function body. This implementation focuses on
//! module-level structural validation, which is sufficient for running
//! well-formed modules produced by standard compilers (Clang, Rust, etc.).
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

use std::collections::HashSet;
use wasm_types::{ExternalKind, ImportTypeInfo, WasmModule};

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
/// | TooManyMemories       | Module declares 2 memories (max is 1).       |
/// | TooManyTables         | Module declares 2 tables (max is 1).         |
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
    /// The module declares more than one memory (not allowed in WASM 1.0).
    TooManyMemories(String),
    /// The module declares more than one table (not allowed in WASM 1.0).
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
#[derive(Debug, Clone)]
pub struct ValidatedModule {
    /// The underlying parsed module.
    pub module: WasmModule,
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
/// 1. **Memory count** -- WASM 1.0 allows at most 1 memory (imports + module).
/// 2. **Table count** -- WASM 1.0 allows at most 1 table (imports + module).
/// 3. **Function type indices** -- Every entry in the function section must
///    reference a valid index in the type section.
/// 4. **Import type indices** -- Function imports must reference valid types.
/// 5. **Code/function count match** -- The code section must have exactly as
///    many entries as the function section.
/// 6. **Export uniqueness** -- No two exports may share the same name.
/// 7. **Export indices** -- Every export must reference a valid entity.
/// 8. **Data segment validity** -- Memory indices must be 0 (only one memory).
/// 9. **Element segment validity** -- Table indices must be 0 (only one table).
/// 10. **Start function** -- If present, must be a valid function index.
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

    // ── Check 1: Memory count ≤ 1 ──────────────────────────────────────
    let total_memories = imported_memories + module.memories.len();
    if total_memories > 1 {
        return Err(ValidationError::TooManyMemories(format!(
            "WASM 1.0 allows at most 1 memory, found {} ({} imported + {} declared)",
            total_memories,
            imported_memories,
            module.memories.len()
        )));
    }

    // ── Check 2: Table count ≤ 1 ───────────────────────────────────────
    let total_tables = imported_tables + module.tables.len();
    if total_tables > 1 {
        return Err(ValidationError::TooManyTables(format!(
            "WASM 1.0 allows at most 1 table, found {} ({} imported + {} declared)",
            total_tables,
            imported_tables,
            module.tables.len()
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
    for (i, elem) in module.elements.iter().enumerate() {
        if total_tables == 0 {
            return Err(ValidationError::InvalidElement(format!(
                "element segment #{} references a table, but no table is declared",
                i
            )));
        }
        if elem.table_index != 0 {
            return Err(ValidationError::InvalidElement(format!(
                "element segment #{} references table index {}, but only index 0 is valid",
                i, elem.table_index
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

    #[test]
    fn rejects_too_many_memories() {
        let module = WasmModule {
            memories: vec![
                MemoryType {
                    limits: Limits { min: 1, max: None },
                },
                MemoryType {
                    limits: Limits { min: 1, max: None },
                },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyMemories(_)));
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
    fn rejects_too_many_tables() {
        let module = WasmModule {
            tables: vec![
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: 1, max: None },
                },
                TableType {
                    element_type: 0x70,
                    limits: Limits { min: 1, max: None },
                },
            ],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyTables(_)));
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
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "mem".to_string(),
                kind: ExternalKind::Memory,
                type_info: ImportTypeInfo::Memory(MemoryType {
                    limits: Limits { min: 1, max: None },
                }),
            }],
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
            }],
            ..Default::default()
        };
        let err = validate(&module).unwrap_err();
        assert!(matches!(err, ValidationError::TooManyMemories(_)));
    }

    #[test]
    fn imported_table_counts_toward_limit() {
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
            tables: vec![TableType {
                element_type: 0x70,
                limits: Limits { min: 1, max: None },
            }],
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

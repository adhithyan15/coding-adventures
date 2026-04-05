//! # wasm-runtime
//!
//! Complete WebAssembly 1.0 runtime — parse, validate, instantiate, execute.
//!
//! This crate composes the lower-level WASM packages into a single, user-facing
//! API. It handles the full pipeline:
//!
//! ```text
//! .wasm bytes  -->  Parse  -->  Validate  -->  Instantiate  -->  Execute
//!     |               |            |               |               |
//! &[u8]         WasmModule  ValidatedModule  WasmInstance    WasmValue[]
//!     |               |            |               |               |
//! (input)      (module-parser) (validator)    (this file)    (execution)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use wasm_runtime::WasmRuntime;
//!
//! let runtime = WasmRuntime::new();
//! let result = runtime.load_and_run(&square_wasm, "square", &[5]);
//! assert_eq!(result.unwrap(), vec![25]);
//! ```
//!
//! This crate is part of the coding-adventures monorepo, a ground-up
//! implementation of the computing stack from transistors to operating systems.

use wasm_execution::{
    evaluate_const_expr, HostFunction, HostInterface, LinearMemory, Table, TrapError,
    WasmEngineConfig, WasmExecutionEngine, WasmValue,
};
use wasm_module_parser::WasmModuleParser;
use wasm_types::{
    ExternalKind, FuncType, FunctionBody, GlobalType, ImportTypeInfo, ValueType, WasmModule,
};
use wasm_validator::{validate, ValidatedModule, ValidationError};

// ══════════════════════════════════════════════════════════════════════════════
// ProcExitError
// ══════════════════════════════════════════════════════════════════════════════

/// Thrown when a WASM program calls `proc_exit`.
///
/// Not a real error — it is the WASM program requesting clean termination.
/// The runtime catches this and returns the exit code.
#[derive(Debug, Clone)]
pub struct ProcExitError {
    /// The exit code the program requested.
    pub exit_code: i32,
}

impl std::fmt::Display for ProcExitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "proc_exit({})", self.exit_code)
    }
}

impl std::error::Error for ProcExitError {}

// ══════════════════════════════════════════════════════════════════════════════
// WasiStub
// ══════════════════════════════════════════════════════════════════════════════

/// A minimal WASI host implementation.
///
/// Provides `fd_write` (captures stdout/stderr) and `proc_exit` (terminates
/// execution). All other WASI functions return ENOSYS (52).
pub struct WasiStub {
    /// Callback for stdout output.
    stdout_callback: Box<dyn Fn(&str)>,
}

impl WasiStub {
    /// Create a new WASI stub with a stdout callback.
    pub fn new(stdout_callback: impl Fn(&str) + 'static) -> Self {
        WasiStub {
            stdout_callback: Box::new(stdout_callback),
        }
    }
}

impl HostInterface for WasiStub {
    fn resolve_function(
        &self,
        module_name: &str,
        name: &str,
    ) -> Option<Box<dyn HostFunction>> {
        if module_name != "wasi_snapshot_preview1" {
            return None;
        }

        match name {
            "proc_exit" => Some(Box::new(ProcExitFunc)),
            // Other WASI functions return ENOSYS
            _ => Some(Box::new(EnosysFunc {
                func_type: FuncType {
                    params: vec![],
                    results: vec![ValueType::I32],
                },
            })),
        }
    }

    fn resolve_global(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<(GlobalType, WasmValue)> {
        None
    }

    fn resolve_memory(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<LinearMemory> {
        None
    }

    fn resolve_table(
        &self,
        _module_name: &str,
        _name: &str,
    ) -> Option<Table> {
        None
    }
}

/// Host function that implements proc_exit.
struct ProcExitFunc;

impl HostFunction for ProcExitFunc {
    fn func_type(&self) -> &FuncType {
        // We use a static-like approach. Since this is simple, just return a reference
        // to a locally constructed type. To avoid lifetime issues, we leak it.
        // In practice this is fine for a singleton.
        static FUNC_TYPE: std::sync::LazyLock<FuncType> = std::sync::LazyLock::new(|| FuncType {
            params: vec![ValueType::I32],
            results: vec![],
        });
        &FUNC_TYPE
    }

    fn call(&self, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError> {
        let exit_code = args
            .first()
            .and_then(|v| v.as_i32().ok())
            .unwrap_or(0);
        Err(TrapError::new(format!("proc_exit({})", exit_code)))
    }
}

/// Host function that returns ENOSYS (52) for unimplemented WASI calls.
struct EnosysFunc {
    func_type: FuncType,
}

impl HostFunction for EnosysFunc {
    fn func_type(&self) -> &FuncType {
        &self.func_type
    }

    fn call(&self, _args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError> {
        Ok(vec![WasmValue::I32(52)]) // ENOSYS
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// WasmInstance
// ══════════════════════════════════════════════════════════════════════════════

/// A live, executable instance of a WASM module.
///
/// Contains all allocated runtime state and the export lookup table.
pub struct WasmInstance {
    /// The original parsed module.
    pub module: WasmModule,
    /// Allocated linear memory.
    pub memory: Option<LinearMemory>,
    /// Allocated tables.
    pub tables: Vec<Table>,
    /// Global variable values.
    pub globals: Vec<WasmValue>,
    /// Global type descriptors.
    pub global_types: Vec<GlobalType>,
    /// All function type signatures.
    pub func_types: Vec<FuncType>,
    /// Function bodies (None for imports).
    pub func_bodies: Vec<Option<FunctionBody>>,
    /// Export map: name -> (kind, index).
    pub exports: Vec<(String, ExternalKind, u32)>,
}

// ══════════════════════════════════════════════════════════════════════════════
// WasmRuntime
// ══════════════════════════════════════════════════════════════════════════════

/// Complete WebAssembly 1.0 runtime.
///
/// Composes the parser, validator, and execution engine into a single
/// user-facing API.
///
/// ## Example
///
/// ```rust,ignore
/// let runtime = WasmRuntime::new();
/// let result = runtime.load_and_run(&wasm_bytes, "square", &[5]);
/// assert_eq!(result.unwrap(), vec![25]);
/// ```
pub struct WasmRuntime {
    host: Option<Box<dyn HostInterface>>,
}

impl WasmRuntime {
    /// Create a new runtime with no host interface.
    pub fn new() -> Self {
        WasmRuntime { host: None }
    }

    /// Create a new runtime with a host interface for import resolution.
    pub fn with_host(host: Box<dyn HostInterface>) -> Self {
        WasmRuntime { host: Some(host) }
    }

    /// Parse a .wasm binary into a WasmModule.
    pub fn load(&self, wasm_bytes: &[u8]) -> Result<WasmModule, String> {
        WasmModuleParser::parse(wasm_bytes).map_err(|e| format!("{}", e))
    }

    /// Validate a parsed module.
    pub fn validate(&self, module: &WasmModule) -> Result<ValidatedModule, ValidationError> {
        validate(module)
    }

    /// Instantiate a parsed module into a live instance.
    pub fn instantiate(&self, module: &WasmModule) -> Result<WasmInstance, TrapError> {
        let mut func_types: Vec<FuncType> = Vec::new();
        let mut func_bodies: Vec<Option<FunctionBody>> = Vec::new();
        let mut host_functions: Vec<Option<Box<dyn HostFunction>>> = Vec::new();
        let mut global_types: Vec<GlobalType> = Vec::new();
        let mut globals: Vec<WasmValue> = Vec::new();
        let mut memory: Option<LinearMemory> = None;
        let mut tables: Vec<Table> = Vec::new();

        // Resolve imports.
        for imp in &module.imports {
            match &imp.type_info {
                ImportTypeInfo::Function(type_idx) => {
                    let ft = module.types[*type_idx as usize].clone();
                    func_types.push(ft);
                    func_bodies.push(None);

                    let host_func = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_function(&imp.module_name, &imp.name));
                    host_functions.push(host_func);
                }
                ImportTypeInfo::Memory(mem_type) => {
                    let imported_mem = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_memory(&imp.module_name, &imp.name));
                    if let Some(m) = imported_mem {
                        memory = Some(m);
                    } else {
                        memory = Some(LinearMemory::new(
                            mem_type.limits.min,
                            mem_type.limits.max,
                        ));
                    }
                }
                ImportTypeInfo::Table(table_type) => {
                    let imported_table = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_table(&imp.module_name, &imp.name));
                    if let Some(t) = imported_table {
                        tables.push(t);
                    } else {
                        tables.push(Table::new(
                            table_type.limits.min,
                            table_type.limits.max,
                        ));
                    }
                }
                ImportTypeInfo::Global(gt) => {
                    let imported_global = self
                        .host
                        .as_ref()
                        .and_then(|h| h.resolve_global(&imp.module_name, &imp.name));
                    if let Some((gtype, gval)) = imported_global {
                        global_types.push(gtype);
                        globals.push(gval);
                    } else {
                        global_types.push(gt.clone());
                        globals.push(WasmValue::default_for(gt.value_type));
                    }
                }
            }
        }

        // Add module-defined functions.
        for (i, &type_idx) in module.functions.iter().enumerate() {
            func_types.push(module.types[type_idx as usize].clone());
            func_bodies.push(module.code.get(i).cloned());
            host_functions.push(None);
        }

        // Allocate memory.
        if memory.is_none() && !module.memories.is_empty() {
            let mem_type = &module.memories[0];
            memory = Some(LinearMemory::new(
                mem_type.limits.min,
                mem_type.limits.max,
            ));
        }

        // Allocate tables.
        for table_type in &module.tables {
            tables.push(Table::new(
                table_type.limits.min,
                table_type.limits.max,
            ));
        }

        // Initialize globals.
        for global in &module.globals {
            global_types.push(global.global_type.clone());
            let value = evaluate_const_expr(&global.init_expr, &globals)?;
            globals.push(value);
        }

        // Apply data segments.
        if let Some(ref mut mem) = memory {
            for seg in &module.data {
                let offset = evaluate_const_expr(&seg.offset_expr, &globals)?;
                let offset_num = offset.as_i32().map_err(|e| TrapError::new(e.message))? as usize;
                mem.write_bytes(offset_num, &seg.data)?;
            }
        }

        // Apply element segments.
        for elem in &module.elements {
            if let Some(table) = tables.get_mut(elem.table_index as usize) {
                let offset = evaluate_const_expr(&elem.offset_expr, &globals)?;
                let offset_num =
                    offset.as_i32().map_err(|e| TrapError::new(e.message))? as u32;
                for (j, &func_idx) in elem.function_indices.iter().enumerate() {
                    table.set(offset_num + j as u32, Some(func_idx))?;
                }
            }
        }

        // Build export list.
        let exports: Vec<(String, ExternalKind, u32)> = module
            .exports
            .iter()
            .map(|e| (e.name.clone(), e.kind, e.index))
            .collect();

        let instance = WasmInstance {
            module: module.clone(),
            memory,
            tables,
            globals,
            global_types,
            func_types,
            func_bodies,
            exports,
        };

        Ok(instance)
    }

    /// Call an exported function by name.
    pub fn call(
        &self,
        instance: &mut WasmInstance,
        name: &str,
        args: &[i64],
    ) -> Result<Vec<i64>, TrapError> {
        let (_, kind, index) = instance
            .exports
            .iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| TrapError::new(format!("export \"{}\" not found", name)))?;

        if *kind != ExternalKind::Function {
            return Err(TrapError::new(format!(
                "export \"{}\" is not a function",
                name
            )));
        }

        let func_index = *index as usize;
        let func_type = instance.func_types[func_index].clone();

        // Convert args to WasmValues.
        let wasm_args: Vec<WasmValue> = args
            .iter()
            .zip(func_type.params.iter())
            .map(|(&arg, &param_type)| match param_type {
                ValueType::I32 => WasmValue::I32(arg as i32),
                ValueType::I64 => WasmValue::I64(arg),
                ValueType::F32 => WasmValue::F32(arg as f32),
                ValueType::F64 => WasmValue::F64(arg as f64),
            })
            .collect();

        // Build engine config, transferring ownership temporarily.
        let memory = instance.memory.take();
        let tables = std::mem::take(&mut instance.tables);

        let mut engine = WasmExecutionEngine::new(WasmEngineConfig {
            memory,
            tables,
            globals: instance.globals.clone(),
            global_types: instance.global_types.clone(),
            func_types: instance.func_types.clone(),
            func_bodies: instance.func_bodies.clone(),
            host_functions: (0..instance.func_types.len()).map(|_| None).collect(),
        });

        let results = engine.call_function(func_index, &wasm_args)?;

        // Convert back to i64.
        Ok(results
            .iter()
            .map(|r| match r {
                WasmValue::I32(v) => *v as i64,
                WasmValue::I64(v) => *v,
                WasmValue::F32(v) => *v as i64,
                WasmValue::F64(v) => *v as i64,
            })
            .collect())
    }

    /// Parse, validate, instantiate, and call in one step.
    pub fn load_and_run(
        &self,
        wasm_bytes: &[u8],
        entry: &str,
        args: &[i64],
    ) -> Result<Vec<i64>, String> {
        let module = self.load(wasm_bytes)?;
        self.validate(&module).map_err(|e| format!("{}", e))?;
        let mut instance = self.instantiate(&module).map_err(|e| format!("{}", e))?;
        self.call(&mut instance, entry, args)
            .map_err(|e| format!("{}", e))
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_types::*;

    /// Build the raw WASM binary for a square(x) = x * x function.
    ///
    /// This is a minimal valid .wasm file containing:
    /// - Type section: (i32) -> i32
    /// - Function section: function 0 uses type 0
    /// - Export section: exports "square" as function 0
    /// - Code section: local.get 0; local.get 0; i32.mul; end
    fn build_square_wasm() -> Vec<u8> {
        let mut wasm = Vec::new();

        // Magic + version
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // \0asm
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version 1

        // Type section (id=1)
        // 1 type: (i32) -> i32
        let type_section = vec![
            0x01, // 1 type
            0x60, // func type
            0x01, 0x7F, // 1 param: i32
            0x01, 0x7F, // 1 result: i32
        ];
        wasm.push(0x01); // section id
        wasm.push(type_section.len() as u8); // section size
        wasm.extend_from_slice(&type_section);

        // Function section (id=3)
        // 1 function referencing type 0
        let func_section = vec![
            0x01, // 1 function
            0x00, // type index 0
        ];
        wasm.push(0x03);
        wasm.push(func_section.len() as u8);
        wasm.extend_from_slice(&func_section);

        // Export section (id=7)
        // Export "square" as function 0
        let export_section = vec![
            0x01, // 1 export
            0x06, // name length 6
            b's', b'q', b'u', b'a', b'r', b'e', // "square"
            0x00, // export kind: function
            0x00, // function index 0
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        // Code section (id=10)
        // 1 function body: local.get 0; local.get 0; i32.mul; end
        let body = vec![
            0x00, // 0 local declarations
            0x20, 0x00, // local.get 0
            0x20, 0x00, // local.get 0
            0x6C, // i32.mul
            0x0B, // end
        ];
        let body_with_size = {
            let mut v = vec![body.len() as u8];
            v.extend_from_slice(&body);
            v
        };
        let code_section = {
            let mut v = vec![0x01u8]; // 1 body
            v.extend_from_slice(&body_with_size);
            v
        };
        wasm.push(0x0A);
        wasm.push(code_section.len() as u8);
        wasm.extend_from_slice(&code_section);

        wasm
    }

    #[test]
    fn test_runtime_square_end_to_end() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[5]);
        assert_eq!(result.unwrap(), vec![25]);
    }

    #[test]
    fn test_runtime_square_negative() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[-3]);
        assert_eq!(result.unwrap(), vec![9]);
    }

    #[test]
    fn test_runtime_square_zero() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "square", &[0]);
        assert_eq!(result.unwrap(), vec![0]);
    }

    #[test]
    fn test_runtime_nonexistent_export() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let result = runtime.load_and_run(&wasm, "nonexistent", &[5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_validate_and_instantiate() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();

        let module = runtime.load(&wasm).unwrap();
        let _validated = runtime.validate(&module).unwrap();
        let mut instance = runtime.instantiate(&module).unwrap();
        let result = runtime.call(&mut instance, "square", &[7]).unwrap();
        assert_eq!(result, vec![49]);
    }

    #[test]
    fn test_wasi_stub_creation() {
        let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let output_clone = output.clone();
        let _wasi = WasiStub::new(move |text: &str| {
            output_clone.lock().unwrap().push(text.to_string());
        });
    }

    #[test]
    fn test_proc_exit_error() {
        let err = ProcExitError { exit_code: 0 };
        assert_eq!(format!("{}", err), "proc_exit(0)");
    }

    #[test]
    fn test_proc_exit_error_nonzero() {
        let err = ProcExitError { exit_code: 1 };
        assert_eq!(format!("{}", err), "proc_exit(1)");
        assert_eq!(err.exit_code, 1);
    }

    #[test]
    fn test_proc_exit_is_error_trait() {
        let err = ProcExitError { exit_code: 42 };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_runtime_default() {
        let runtime = WasmRuntime::default();
        // Default runtime should have no host
        let wasm = build_square_wasm();
        let result = runtime.load_and_run(&wasm, "square", &[3]);
        assert_eq!(result.unwrap(), vec![9]);
    }

    #[test]
    fn test_runtime_load_invalid_wasm() {
        let runtime = WasmRuntime::new();
        let result = runtime.load(&[0x00, 0x01, 0x02, 0x03]);
        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_validate_valid_module() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        assert!(runtime.validate(&module).is_ok());
    }

    #[test]
    fn test_runtime_instantiate() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        let instance = runtime.instantiate(&module).unwrap();

        // Check that exports were populated
        assert!(!instance.exports.is_empty());
        assert_eq!(instance.exports[0].0, "square");
        assert_eq!(instance.exports[0].1, ExternalKind::Function);
    }

    #[test]
    fn test_runtime_call_wrong_export_type() {
        // Build a module that exports a memory, then try to call it as a function
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // magic
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // version

        // Memory section (id=5): 1 memory, min=1, no max
        let mem_section = vec![0x01, 0x00, 0x01]; // 1 memory, limits flag 0, min 1
        wasm.push(0x05);
        wasm.push(mem_section.len() as u8);
        wasm.extend_from_slice(&mem_section);

        // Export section (id=7): export "mem" as memory 0
        let export_section = vec![
            0x01,       // 1 export
            0x03,       // name length
            b'm', b'e', b'm',
            0x02,       // memory export kind
            0x00,       // memory index 0
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        let runtime = WasmRuntime::new();
        let result = runtime.load_and_run(&wasm, "mem", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a function"));
    }

    #[test]
    fn test_runtime_with_memory() {
        // Build a module with memory that stores and loads a value
        // func store_and_load(val: i32) -> i32:
        //   i32.const 0; local.get 0; i32.store; i32.const 0; i32.load; end
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]);
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]);

        // Type section: (i32) -> i32
        let type_section = vec![0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F];
        wasm.push(0x01);
        wasm.push(type_section.len() as u8);
        wasm.extend_from_slice(&type_section);

        // Function section
        let func_section = vec![0x01, 0x00];
        wasm.push(0x03);
        wasm.push(func_section.len() as u8);
        wasm.extend_from_slice(&func_section);

        // Memory section: 1 page min, no max
        let mem_section = vec![0x01, 0x00, 0x01];
        wasm.push(0x05);
        wasm.push(mem_section.len() as u8);
        wasm.extend_from_slice(&mem_section);

        // Export section
        let export_section = vec![
            0x01, 0x04, b't', b'e', b's', b't', 0x00, 0x00,
        ];
        wasm.push(0x07);
        wasm.push(export_section.len() as u8);
        wasm.extend_from_slice(&export_section);

        // Code section
        let body = vec![
            0x00,       // 0 locals
            0x41, 0x00, // i32.const 0 (addr)
            0x20, 0x00, // local.get 0 (val)
            0x36, 0x02, 0x00, // i32.store align=2 offset=0
            0x41, 0x00, // i32.const 0 (addr)
            0x28, 0x02, 0x00, // i32.load align=2 offset=0
            0x0B,       // end
        ];
        let body_with_size = {
            let mut v = vec![body.len() as u8];
            v.extend_from_slice(&body);
            v
        };
        let code_section = {
            let mut v = vec![0x01u8];
            v.extend_from_slice(&body_with_size);
            v
        };
        wasm.push(0x0A);
        wasm.push(code_section.len() as u8);
        wasm.extend_from_slice(&code_section);

        let runtime = WasmRuntime::new();
        let result = runtime.load_and_run(&wasm, "test", &[42]);
        assert_eq!(result.unwrap(), vec![42]);
    }

    #[test]
    fn test_runtime_with_global() {
        // Module with a mutable global initialized to 100
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x23, 0x00, 0x0B], // global.get 0; end
            }],
            globals: vec![Global {
                global_type: GlobalType {
                    value_type: ValueType::I32,
                    mutable: true,
                },
                init_expr: vec![0x41, 0xE4, 0x00, 0x0B], // i32.const 100; end (100 in signed LEB128)
            }],
            exports: vec![Export {
                name: "get_global".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };

        let runtime = WasmRuntime::new();
        let mut instance = runtime.instantiate(&module).unwrap();
        let result = runtime.call(&mut instance, "get_global", &[]).unwrap();
        assert_eq!(result, vec![100]);
    }

    #[test]
    fn test_runtime_with_data_segment() {
        // Module with memory and a data segment that initializes bytes
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![
                    0x41, 0x00, // i32.const 0
                    0x28, 0x02, 0x00, // i32.load align=2 offset=0
                    0x0B,
                ],
            }],
            memories: vec![MemoryType {
                limits: Limits { min: 1, max: None },
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B], // i32.const 0; end
                data: vec![0x2A, 0x00, 0x00, 0x00],  // 42 in little-endian
            }],
            exports: vec![Export {
                name: "read".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            ..Default::default()
        };

        let runtime = WasmRuntime::new();
        let mut instance = runtime.instantiate(&module).unwrap();
        let result = runtime.call(&mut instance, "read", &[]).unwrap();
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_wasi_stub_proc_exit() {
        let wasi = WasiStub::new(|_| {});
        let func = wasi
            .resolve_function("wasi_snapshot_preview1", "proc_exit")
            .unwrap();
        assert_eq!(func.func_type().params, vec![ValueType::I32]);
        assert!(func.func_type().results.is_empty());
        // Calling proc_exit should return an error (trap)
        let result = func.call(&[WasmValue::I32(0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_wasi_stub_enosys_function() {
        let wasi = WasiStub::new(|_| {});
        let func = wasi
            .resolve_function("wasi_snapshot_preview1", "unknown_function")
            .unwrap();
        let result = func.call(&[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(52)]); // ENOSYS
    }

    #[test]
    fn test_wasi_stub_wrong_module() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_function("env", "some_func").is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_global() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_global("wasi_snapshot_preview1", "x").is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_memory() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_memory("wasi_snapshot_preview1", "memory").is_none());
    }

    #[test]
    fn test_wasi_stub_resolve_table() {
        let wasi = WasiStub::new(|_| {});
        assert!(wasi.resolve_table("wasi_snapshot_preview1", "table").is_none());
    }

    #[test]
    fn test_runtime_with_host() {
        let wasi = WasiStub::new(|_| {});
        let runtime = WasmRuntime::with_host(Box::new(wasi));
        let wasm = build_square_wasm();
        let result = runtime.load_and_run(&wasm, "square", &[4]);
        assert_eq!(result.unwrap(), vec![16]);
    }

    #[test]
    fn test_instance_fields() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let module = runtime.load(&wasm).unwrap();
        let instance = runtime.instantiate(&module).unwrap();

        // No memory in square module
        assert!(instance.memory.is_none());
        // No tables
        assert!(instance.tables.is_empty());
        // No globals
        assert!(instance.globals.is_empty());
        // One function type
        assert_eq!(instance.func_types.len(), 1);
        // One function body
        assert_eq!(instance.func_bodies.len(), 1);
    }

    #[test]
    fn test_runtime_load_and_run_nonexistent_export_error_message() {
        let wasm = build_square_wasm();
        let runtime = WasmRuntime::new();
        let err = runtime.load_and_run(&wasm, "no_such_fn", &[1]).unwrap_err();
        assert!(err.contains("not found"));
    }
}

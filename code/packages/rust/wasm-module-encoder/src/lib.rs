//! Encode `wasm-types::WasmModule` into raw WebAssembly 1.0 bytes.

use std::fmt;

use wasm_leb128::encode_unsigned;
use wasm_types::{
    CustomSection, DataSegment, Element, Export, ExternalKind, FuncType, FunctionBody, Global,
    GlobalType, Import, ImportTypeInfo, Limits, MemoryType, TableType, ValueType, WasmModule,
};

pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
pub const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmEncodeError {
    pub message: String,
}

impl WasmEncodeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WasmEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WasmEncodeError {}

pub fn encode_module(module: &WasmModule) -> Result<Vec<u8>, WasmEncodeError> {
    let mut sections = Vec::new();

    for custom in &module.customs {
        sections.extend(encode_section(0, encode_custom(custom)));
    }
    if !module.types.is_empty() {
        sections.extend(encode_section(
            1,
            encode_vector(&module.types, encode_func_type),
        ));
    }
    if !module.imports.is_empty() {
        sections.extend(encode_section(2, encode_imports(&module.imports)?));
    }
    if !module.functions.is_empty() {
        sections.extend(encode_section(
            3,
            encode_vector(&module.functions, |index| encode_u32(*index)),
        ));
    }
    if !module.tables.is_empty() {
        sections.extend(encode_section(
            4,
            encode_vector(&module.tables, encode_table_type),
        ));
    }
    if !module.memories.is_empty() {
        sections.extend(encode_section(
            5,
            encode_vector(&module.memories, encode_memory_type),
        ));
    }
    if !module.globals.is_empty() {
        sections.extend(encode_section(
            6,
            encode_vector(&module.globals, encode_global),
        ));
    }
    if !module.exports.is_empty() {
        sections.extend(encode_section(
            7,
            encode_vector(&module.exports, encode_export),
        ));
    }
    if let Some(start) = module.start {
        sections.extend(encode_section(8, encode_u32(start)));
    }
    if !module.elements.is_empty() {
        sections.extend(encode_section(
            9,
            encode_vector(&module.elements, encode_element),
        ));
    }
    if !module.code.is_empty() {
        sections.extend(encode_section(10, encode_function_bodies(&module.code)));
    }
    if !module.data.is_empty() {
        sections.extend(encode_section(
            11,
            encode_vector(&module.data, encode_data_segment),
        ));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&WASM_MAGIC);
    bytes.extend_from_slice(&WASM_VERSION);
    bytes.extend(sections);
    Ok(bytes)
}

fn encode_section(section_id: u8, payload: Vec<u8>) -> Vec<u8> {
    let mut bytes = vec![section_id];
    bytes.extend(encode_u32(payload.len() as u32));
    bytes.extend(payload);
    bytes
}

fn encode_u32(value: u32) -> Vec<u8> {
    encode_unsigned(value as u64)
}

fn encode_name(text: &str) -> Vec<u8> {
    let mut bytes = encode_u32(text.len() as u32);
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn encode_vector<T>(values: &[T], mut encode: impl FnMut(&T) -> Vec<u8>) -> Vec<u8> {
    let mut bytes = encode_u32(values.len() as u32);
    for value in values {
        bytes.extend(encode(value));
    }
    bytes
}

fn encode_value_types(types: &[ValueType]) -> Vec<u8> {
    let mut bytes = encode_u32(types.len() as u32);
    bytes.extend(types.iter().map(|value_type| *value_type as u8));
    bytes
}

fn encode_func_type(func_type: &FuncType) -> Vec<u8> {
    let mut bytes = vec![0x60];
    bytes.extend(encode_value_types(&func_type.params));
    bytes.extend(encode_value_types(&func_type.results));
    bytes
}

fn encode_limits(limits: &Limits) -> Vec<u8> {
    let mut bytes = Vec::new();
    match limits.max {
        Some(max) => {
            bytes.push(0x01);
            bytes.extend(encode_u32(limits.min));
            bytes.extend(encode_u32(max));
        }
        None => {
            bytes.push(0x00);
            bytes.extend(encode_u32(limits.min));
        }
    }
    bytes
}

fn encode_memory_type(memory_type: &MemoryType) -> Vec<u8> {
    encode_limits(&memory_type.limits)
}

fn encode_table_type(table_type: &TableType) -> Vec<u8> {
    let mut bytes = vec![table_type.element_type];
    bytes.extend(encode_limits(&table_type.limits));
    bytes
}

fn encode_global_type(global_type: &GlobalType) -> Vec<u8> {
    vec![
        global_type.value_type as u8,
        if global_type.mutable { 0x01 } else { 0x00 },
    ]
}

fn encode_imports(imports: &[Import]) -> Result<Vec<u8>, WasmEncodeError> {
    let mut bytes = encode_u32(imports.len() as u32);
    for import in imports {
        bytes.extend(encode_import(import)?);
    }
    Ok(bytes)
}

fn encode_import(import: &Import) -> Result<Vec<u8>, WasmEncodeError> {
    let mut bytes = Vec::new();
    bytes.extend(encode_name(&import.module_name));
    bytes.extend(encode_name(&import.name));
    bytes.push(import.kind as u8);

    match (&import.kind, &import.type_info) {
        (ExternalKind::Function, ImportTypeInfo::Function(type_index)) => {
            bytes.extend(encode_u32(*type_index));
        }
        (ExternalKind::Table, ImportTypeInfo::Table(table_type)) => {
            bytes.extend(encode_table_type(table_type));
        }
        (ExternalKind::Memory, ImportTypeInfo::Memory(memory_type)) => {
            bytes.extend(encode_memory_type(memory_type));
        }
        (ExternalKind::Global, ImportTypeInfo::Global(global_type)) => {
            bytes.extend(encode_global_type(global_type));
        }
        (ExternalKind::Function, _) => {
            return Err(WasmEncodeError::new(
                "function imports require a function type index",
            ));
        }
        (ExternalKind::Table, _) => {
            return Err(WasmEncodeError::new(
                "table imports require TableType metadata",
            ));
        }
        (ExternalKind::Memory, _) => {
            return Err(WasmEncodeError::new(
                "memory imports require MemoryType metadata",
            ));
        }
        (ExternalKind::Global, _) => {
            return Err(WasmEncodeError::new(
                "global imports require GlobalType metadata",
            ));
        }
    }

    Ok(bytes)
}

fn encode_export(export: &Export) -> Vec<u8> {
    let mut bytes = encode_name(&export.name);
    bytes.push(export.kind as u8);
    bytes.extend(encode_u32(export.index));
    bytes
}

fn encode_global(global: &Global) -> Vec<u8> {
    let mut bytes = encode_global_type(&global.global_type);
    bytes.extend_from_slice(&global.init_expr);
    bytes
}

fn encode_element(element: &Element) -> Vec<u8> {
    let mut bytes = encode_u32(element.table_index);
    bytes.extend_from_slice(&element.offset_expr);
    bytes.extend(encode_u32(element.function_indices.len() as u32));
    for func_index in &element.function_indices {
        bytes.extend(encode_u32(*func_index));
    }
    bytes
}

fn encode_data_segment(segment: &DataSegment) -> Vec<u8> {
    let mut bytes = encode_u32(segment.memory_index);
    bytes.extend_from_slice(&segment.offset_expr);
    bytes.extend(encode_u32(segment.data.len() as u32));
    bytes.extend_from_slice(&segment.data);
    bytes
}

fn encode_function_bodies(bodies: &[FunctionBody]) -> Vec<u8> {
    let mut bytes = encode_u32(bodies.len() as u32);
    for body in bodies {
        bytes.extend(encode_function_body(body));
    }
    bytes
}

fn encode_function_body(body: &FunctionBody) -> Vec<u8> {
    let groups = group_locals(&body.locals);
    let mut payload = encode_u32(groups.len() as u32);
    for (count, value_type) in groups {
        payload.extend(encode_u32(count));
        payload.push(value_type as u8);
    }
    payload.extend_from_slice(&body.code);

    let mut bytes = encode_u32(payload.len() as u32);
    bytes.extend(payload);
    bytes
}

fn group_locals(locals: &[ValueType]) -> Vec<(u32, ValueType)> {
    if locals.is_empty() {
        return Vec::new();
    }

    let mut groups = Vec::new();
    let mut current = locals[0];
    let mut count = 1u32;
    for value_type in &locals[1..] {
        if *value_type == current {
            count += 1;
        } else {
            groups.push((count, current));
            current = *value_type;
            count = 1;
        }
    }
    groups.push((count, current));
    groups
}

fn encode_custom(custom: &CustomSection) -> Vec<u8> {
    let mut bytes = encode_name(&custom.name);
    bytes.extend_from_slice(&custom.data);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_module_parser::WasmModuleParser;
    use wasm_types::WasmModule;

    fn minimal_module() -> WasmModule {
        WasmModule {
            types: vec![FuncType {
                params: vec![ValueType::I32],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            exports: vec![Export {
                name: "identity".to_string(),
                kind: ExternalKind::Function,
                index: 0,
            }],
            code: vec![FunctionBody {
                locals: vec![],
                code: vec![0x20, 0x00, 0x0B],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn encodes_minimal_module_round_trip() {
        let module = minimal_module();
        let encoded = encode_module(&module).unwrap();
        let parsed = WasmModuleParser::parse(&encoded).unwrap();

        assert!(encoded.starts_with(&[WASM_MAGIC, WASM_VERSION].concat()));
        assert_eq!(parsed.types, module.types);
        assert_eq!(parsed.functions, module.functions);
        assert_eq!(parsed.exports, module.exports);
        assert_eq!(parsed.code, module.code);
    }

    #[test]
    fn encodes_memory_data_global_and_start() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![ValueType::I32],
            }],
            functions: vec![0],
            memories: vec![MemoryType {
                limits: Limits {
                    min: 1,
                    max: Some(2),
                },
            }],
            globals: vec![Global {
                global_type: GlobalType {
                    value_type: ValueType::I32,
                    mutable: false,
                },
                init_expr: vec![0x41, 0x2A, 0x0B],
            }],
            exports: vec![
                Export {
                    name: "main".to_string(),
                    kind: ExternalKind::Function,
                    index: 0,
                },
                Export {
                    name: "memory".to_string(),
                    kind: ExternalKind::Memory,
                    index: 0,
                },
            ],
            start: Some(0),
            code: vec![FunctionBody {
                locals: vec![ValueType::I32],
                code: vec![0x41, 0x07, 0x0B],
            }],
            data: vec![DataSegment {
                memory_index: 0,
                offset_expr: vec![0x41, 0x00, 0x0B],
                data: b"Nib".to_vec(),
            }],
            ..Default::default()
        };

        let parsed = WasmModuleParser::parse(&encode_module(&module).unwrap()).unwrap();
        assert_eq!(parsed.memories, module.memories);
        assert_eq!(parsed.globals, module.globals);
        assert_eq!(parsed.start, module.start);
        assert_eq!(parsed.data, module.data);
    }

    #[test]
    fn encodes_imports_table_and_custom_section() {
        let module = WasmModule {
            types: vec![FuncType {
                params: vec![],
                results: vec![],
            }],
            imports: vec![
                Import {
                    module_name: "env".to_string(),
                    name: "f".to_string(),
                    kind: ExternalKind::Function,
                    type_info: ImportTypeInfo::Function(0),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "table".to_string(),
                    kind: ExternalKind::Table,
                    type_info: ImportTypeInfo::Table(TableType {
                        element_type: 0x70,
                        limits: Limits {
                            min: 1,
                            max: Some(4),
                        },
                    }),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "memory".to_string(),
                    kind: ExternalKind::Memory,
                    type_info: ImportTypeInfo::Memory(MemoryType {
                        limits: Limits { min: 1, max: None },
                    }),
                },
                Import {
                    module_name: "env".to_string(),
                    name: "glob".to_string(),
                    kind: ExternalKind::Global,
                    type_info: ImportTypeInfo::Global(GlobalType {
                        value_type: ValueType::I32,
                        mutable: true,
                    }),
                },
            ],
            customs: vec![CustomSection {
                name: "name".to_string(),
                data: vec![0x01, 0x02],
            }],
            ..Default::default()
        };

        let parsed = WasmModuleParser::parse(&encode_module(&module).unwrap()).unwrap();
        assert_eq!(parsed.imports, module.imports);
        assert_eq!(parsed.customs, module.customs);
    }

    #[test]
    fn rejects_invalid_function_import_type() {
        let module = WasmModule {
            imports: vec![Import {
                module_name: "env".to_string(),
                name: "f".to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Memory(MemoryType {
                    limits: Limits { min: 1, max: None },
                }),
            }],
            ..Default::default()
        };

        let err = encode_module(&module).unwrap_err();
        assert!(err.message.contains("function imports require"));
    }
}

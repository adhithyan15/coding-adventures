pub mod codegen;

use std::collections::{BTreeSet, HashMap};
use std::fmt;

use compiler_ir::{IrDataDecl, IrInstruction, IrOp, IrOperand, IrProgram};
use wasm_leb128::{encode_signed, encode_unsigned};
use wasm_opcodes::get_opcode_by_name;
use wasm_types::{
    DataSegment, Export, ExternalKind, FuncType, FunctionBody, Import, ImportTypeInfo, Limits,
    MemoryType, ValueType, WasmModule, BLOCK_TYPE_EMPTY,
};

const SYSCALL_WRITE: i64 = 1;
const SYSCALL_READ: i64 = 2;
const SYSCALL_EXIT: i64 = 10;
const SYSCALL_ARG0: usize = 4;

const WASI_MODULE: &str = "wasi_snapshot_preview1";
const WASI_IOVEC_OFFSET: u32 = 0;
const WASI_COUNT_OFFSET: u32 = 8;
const WASI_BYTE_OFFSET: u32 = 12;
const WASI_SCRATCH_SIZE: u32 = 16;

const REG_SCRATCH: usize = 1;
const REG_VAR_BASE: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmLoweringError {
    pub message: String,
}

impl WasmLoweringError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WasmLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WasmLoweringError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub label: String,
    pub param_count: usize,
    pub export_name: Option<String>,
}

#[derive(Debug, Clone)]
struct FunctionIr {
    label: String,
    instructions: Vec<IrInstruction>,
    signature: FunctionSignature,
    max_reg: usize,
}

#[derive(Debug, Clone)]
struct WasiImport {
    syscall_number: i64,
    name: &'static str,
    func_type: FuncType,
    type_key: &'static str,
}

#[derive(Debug, Clone)]
struct WasiContext {
    function_indices: HashMap<i64, u32>,
    scratch_base: Option<u32>,
}

#[derive(Default)]
pub struct IrToWasmCompiler;

impl IrToWasmCompiler {
    pub fn compile(
        &self,
        program: &IrProgram,
        function_signatures: &[FunctionSignature],
    ) -> Result<WasmModule, WasmLoweringError> {
        let mut signatures = infer_function_signatures_from_comments(program);
        for signature in function_signatures {
            signatures.insert(signature.label.clone(), signature.clone());
        }

        let functions = split_functions(program, &signatures)?;
        let imports = collect_wasi_imports(program)?;
        let (type_indices, types) = build_type_table(&functions, &imports);
        let data_offsets = layout_data(&program.data);
        let scratch_base = if needs_wasi_scratch(program) {
            Some(align_up(total_data_size(&program.data), 4))
        } else {
            None
        };

        let mut module = WasmModule {
            types,
            ..Default::default()
        };

        for import in &imports {
            module.imports.push(Import {
                module_name: WASI_MODULE.to_string(),
                name: import.name.to_string(),
                kind: ExternalKind::Function,
                type_info: ImportTypeInfo::Function(type_indices[import.type_key]),
            });
        }

        let function_index_base = module.imports.len() as u32;
        let mut function_indices = HashMap::new();
        for (index, function) in functions.iter().enumerate() {
            function_indices.insert(function.label.clone(), function_index_base + index as u32);
            module.functions.push(type_indices[&function.label]);
        }

        let mut total_bytes = total_data_size(&program.data);
        if let Some(base) = scratch_base {
            total_bytes = total_bytes.max(base + WASI_SCRATCH_SIZE);
        }

        if needs_memory(program) || scratch_base.is_some() {
            let page_count = ((total_bytes + 65_535) / 65_536).max(1);
            module.memories.push(MemoryType {
                limits: Limits {
                    min: page_count,
                    max: None,
                },
            });
            module.exports.push(Export {
                name: "memory".to_string(),
                kind: ExternalKind::Memory,
                index: 0,
            });

            for decl in &program.data {
                module.data.push(DataSegment {
                    memory_index: 0,
                    offset_expr: const_expr(data_offsets[&decl.label] as i64),
                    data: vec![decl.init; decl.size],
                });
            }
        }

        let wasi_context = WasiContext {
            function_indices: imports
                .iter()
                .enumerate()
                .map(|(index, import)| (import.syscall_number, index as u32))
                .collect(),
            scratch_base,
        };

        for function in &functions {
            let mut lowerer = FunctionLowerer::new(
                function,
                &signatures,
                &function_indices,
                &data_offsets,
                &wasi_context,
            );
            module.code.push(lowerer.lower()?);
            if let Some(export_name) = &function.signature.export_name {
                module.exports.push(Export {
                    name: export_name.clone(),
                    kind: ExternalKind::Function,
                    index: function_indices[&function.label],
                });
            }
        }

        Ok(module)
    }
}

pub fn infer_function_signatures_from_comments(
    program: &IrProgram,
) -> HashMap<String, FunctionSignature> {
    let mut signatures = HashMap::new();
    let mut pending_comment: Option<String> = None;

    for instruction in &program.instructions {
        if instruction.opcode == IrOp::Comment {
            pending_comment = instruction
                .operands
                .first()
                .and_then(label_operand)
                .map(str::to_string);
            continue;
        }

        let Some(label_name) = function_label_name(instruction) else {
            pending_comment = None;
            continue;
        };

        if label_name == "_start" {
            signatures.insert(
                label_name.to_string(),
                FunctionSignature {
                    label: label_name.to_string(),
                    param_count: 0,
                    export_name: Some("_start".to_string()),
                },
            );
        } else if let Some(comment) = pending_comment.as_deref() {
            if let Some(export_name) = label_name.strip_prefix("_fn_") {
                if let Some(param_count) = parse_function_comment(comment, export_name) {
                    signatures.insert(
                        label_name.to_string(),
                        FunctionSignature {
                            label: label_name.to_string(),
                            param_count,
                            export_name: Some(export_name.to_string()),
                        },
                    );
                }
            }
        }

        pending_comment = None;
    }

    signatures
}

fn parse_function_comment(comment: &str, export_name: &str) -> Option<usize> {
    let rest = comment.strip_prefix("function:")?.trim();
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    if close < open || rest[..open].trim() != export_name {
        return None;
    }

    let params = rest[open + 1..close].trim();
    Some(if params.is_empty() {
        0
    } else {
        params
            .split(',')
            .filter(|piece| !piece.trim().is_empty())
            .count()
    })
}

fn build_type_table(
    functions: &[FunctionIr],
    imports: &[WasiImport],
) -> (HashMap<String, u32>, Vec<FuncType>) {
    let mut seen = HashMap::new();
    let mut type_indices = HashMap::new();
    let mut types = Vec::new();

    for import in imports {
        remember_type(
            import.type_key.to_string(),
            import.func_type.clone(),
            &mut seen,
            &mut type_indices,
            &mut types,
        );
    }

    for function in functions {
        remember_type(
            function.label.clone(),
            FuncType {
                params: vec![ValueType::I32; function.signature.param_count],
                results: vec![ValueType::I32],
            },
            &mut seen,
            &mut type_indices,
            &mut types,
        );
    }

    (type_indices, types)
}

fn remember_type(
    key: String,
    ty: FuncType,
    seen: &mut HashMap<String, u32>,
    type_indices: &mut HashMap<String, u32>,
    types: &mut Vec<FuncType>,
) {
    let signature_key = func_type_key(&ty);
    let index = if let Some(index) = seen.get(&signature_key) {
        *index
    } else {
        let index = types.len() as u32;
        types.push(ty);
        seen.insert(signature_key, index);
        index
    };
    type_indices.insert(key, index);
}

fn split_functions(
    program: &IrProgram,
    signatures: &HashMap<String, FunctionSignature>,
) -> Result<Vec<FunctionIr>, WasmLoweringError> {
    let mut functions = Vec::new();
    let mut start_index = None;
    let mut start_label = None::<String>;

    for (index, instruction) in program.instructions.iter().enumerate() {
        let Some(label_name) = function_label_name(instruction) else {
            continue;
        };

        if let (Some(start), Some(label)) = (start_index, &start_label) {
            functions.push(make_function_ir(
                label,
                &program.instructions[start..index],
                signatures,
            )?);
        }

        start_index = Some(index);
        start_label = Some(label_name.to_string());
    }

    if let (Some(start), Some(label)) = (start_index, &start_label) {
        functions.push(make_function_ir(
            label,
            &program.instructions[start..],
            signatures,
        )?);
    }

    Ok(functions)
}

fn make_function_ir(
    label: &str,
    instructions: &[IrInstruction],
    signatures: &HashMap<String, FunctionSignature>,
) -> Result<FunctionIr, WasmLoweringError> {
    let signature = if label == "_start" {
        signatures.get(label).cloned().unwrap_or(FunctionSignature {
            label: label.to_string(),
            param_count: 0,
            export_name: Some("_start".to_string()),
        })
    } else {
        signatures.get(label).cloned().ok_or_else(|| {
            WasmLoweringError::new(format!("missing function signature for {}", label))
        })?
    };

    let mut max_reg = REG_SCRATCH.max(REG_VAR_BASE + signature.param_count.saturating_sub(1));
    let mut has_syscall = false;
    for instruction in instructions {
        if instruction.opcode == IrOp::Syscall {
            has_syscall = true;
        }
        for operand in &instruction.operands {
            if let IrOperand::Register(index) = operand {
                max_reg = max_reg.max(*index);
            }
        }
    }
    if has_syscall {
        max_reg = max_reg.max(SYSCALL_ARG0);
    }

    Ok(FunctionIr {
        label: label.to_string(),
        instructions: instructions.to_vec(),
        signature,
        max_reg,
    })
}

fn collect_wasi_imports(program: &IrProgram) -> Result<Vec<WasiImport>, WasmLoweringError> {
    let mut required = BTreeSet::new();
    for instruction in &program.instructions {
        if instruction.opcode != IrOp::Syscall || instruction.operands.is_empty() {
            continue;
        }
        required.insert(expect_immediate(
            instruction.operands.first(),
            "SYSCALL number",
        )?);
    }

    let ordered = vec![
        WasiImport {
            syscall_number: SYSCALL_WRITE,
            name: "fd_write",
            func_type: FuncType {
                params: vec![ValueType::I32; 4],
                results: vec![ValueType::I32],
            },
            type_key: "wasi::fd_write",
        },
        WasiImport {
            syscall_number: SYSCALL_READ,
            name: "fd_read",
            func_type: FuncType {
                params: vec![ValueType::I32; 4],
                results: vec![ValueType::I32],
            },
            type_key: "wasi::fd_read",
        },
        WasiImport {
            syscall_number: SYSCALL_EXIT,
            name: "proc_exit",
            func_type: FuncType {
                params: vec![ValueType::I32],
                results: vec![],
            },
            type_key: "wasi::proc_exit",
        },
    ];

    let supported = ordered
        .iter()
        .map(|entry| entry.syscall_number)
        .collect::<BTreeSet<_>>();
    let unsupported = required
        .difference(&supported)
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        return Err(WasmLoweringError::new(format!(
            "unsupported SYSCALL number(s): {}",
            unsupported.join(", ")
        )));
    }

    Ok(ordered
        .into_iter()
        .filter(|entry| required.contains(&entry.syscall_number))
        .collect())
}

fn layout_data(decls: &[IrDataDecl]) -> HashMap<String, u32> {
    let mut offsets = HashMap::new();
    let mut cursor = 0;
    for decl in decls {
        offsets.insert(decl.label.clone(), cursor);
        cursor += decl.size as u32;
    }
    offsets
}

fn needs_memory(program: &IrProgram) -> bool {
    !program.data.is_empty()
        || program.instructions.iter().any(|instruction| {
            matches!(
                instruction.opcode,
                IrOp::LoadAddr
                    | IrOp::LoadByte
                    | IrOp::StoreByte
                    | IrOp::LoadWord
                    | IrOp::StoreWord
            )
        })
}

fn needs_wasi_scratch(program: &IrProgram) -> bool {
    program.instructions.iter().any(|instruction| {
        instruction.opcode == IrOp::Syscall
            && instruction
                .operands
                .first()
                .and_then(|operand| match operand {
                    IrOperand::Immediate(value) => Some(*value),
                    _ => None,
                })
                .is_some_and(|value| value == SYSCALL_WRITE || value == SYSCALL_READ)
    })
}

struct FunctionLowerer<'a> {
    function: &'a FunctionIr,
    signatures: &'a HashMap<String, FunctionSignature>,
    function_indices: &'a HashMap<String, u32>,
    data_offsets: &'a HashMap<String, u32>,
    wasi_context: &'a WasiContext,
    label_to_index: HashMap<String, usize>,
    bytes: Vec<u8>,
}

impl<'a> FunctionLowerer<'a> {
    fn new(
        function: &'a FunctionIr,
        signatures: &'a HashMap<String, FunctionSignature>,
        function_indices: &'a HashMap<String, u32>,
        data_offsets: &'a HashMap<String, u32>,
        wasi_context: &'a WasiContext,
    ) -> Self {
        let label_to_index = function
            .instructions
            .iter()
            .enumerate()
            .filter_map(|(index, instruction)| {
                label_name_from_instruction(instruction).map(|label| (label.to_string(), index))
            })
            .collect();

        Self {
            function,
            signatures,
            function_indices,
            data_offsets,
            wasi_context,
            label_to_index,
            bytes: Vec::new(),
        }
    }

    fn lower(&mut self) -> Result<FunctionBody, WasmLoweringError> {
        self.copy_params_into_ir_registers();
        self.emit_region(1, self.function.instructions.len())?;
        self.emit_opcode("end");

        Ok(FunctionBody {
            locals: vec![ValueType::I32; self.function.max_reg + 1],
            code: self.bytes.clone(),
        })
    }

    fn copy_params_into_ir_registers(&mut self) {
        for param_index in 0..self.function.signature.param_count {
            self.emit_opcode("local.get");
            self.emit_u32(param_index as u32);
            self.emit_opcode("local.set");
            self.emit_u32(self.local_index(REG_VAR_BASE + param_index) as u32);
        }
    }

    fn emit_region(&mut self, start: usize, end: usize) -> Result<(), WasmLoweringError> {
        let mut index = start;
        while index < end {
            let instruction = &self.function.instructions[index];

            if instruction.opcode == IrOp::Comment {
                index += 1;
                continue;
            }

            if let Some(label_name) = label_name_from_instruction(instruction) {
                if is_loop_start(label_name) {
                    index = self.emit_loop(index)?;
                    continue;
                }
            }

            if matches!(instruction.opcode, IrOp::BranchZ | IrOp::BranchNz)
                && instruction.operands.len() == 2
                && instruction
                    .operands
                    .get(1)
                    .and_then(label_operand)
                    .is_some_and(is_if_else_label)
            {
                index = self.emit_if(index)?;
                continue;
            }

            if instruction.opcode == IrOp::Label {
                index += 1;
                continue;
            }

            if matches!(
                instruction.opcode,
                IrOp::Jump | IrOp::BranchZ | IrOp::BranchNz
            ) {
                return Err(WasmLoweringError::new(format!(
                    "unexpected unstructured control flow in {}",
                    self.function.label
                )));
            }

            self.emit_simple(instruction)?;
            index += 1;
        }

        Ok(())
    }

    fn emit_if(&mut self, branch_index: usize) -> Result<usize, WasmLoweringError> {
        let branch = &self.function.instructions[branch_index];
        let cond_reg = expect_register(branch.operands.first(), "if condition")?;
        let else_label = expect_label(branch.operands.get(1), "if else label")?;
        let end_label = if let Some(prefix) = else_label.strip_suffix("_else") {
            format!("{}_end", prefix)
        } else {
            format!("{}_end", else_label)
        };

        let else_index = self.require_label_index(&else_label)?;
        let end_index = self.require_label_index(&end_label)?;
        let jump_index = self.find_last_jump_to_label(branch_index + 1, else_index, &end_label)?;

        self.emit_local_get(cond_reg);
        if branch.opcode == IrOp::BranchNz {
            self.emit_opcode("i32.eqz");
        }
        self.emit_opcode("if");
        self.bytes.push(BLOCK_TYPE_EMPTY);

        self.emit_region(branch_index + 1, jump_index)?;

        if else_index + 1 < end_index {
            self.emit_opcode("else");
            self.emit_region(else_index + 1, end_index)?;
        }

        self.emit_opcode("end");
        Ok(end_index + 1)
    }

    fn emit_loop(&mut self, label_index: usize) -> Result<usize, WasmLoweringError> {
        let start_label = label_name_from_instruction(&self.function.instructions[label_index])
            .ok_or_else(|| WasmLoweringError::new("loop lowering expected a start label"))?;
        let end_label = format!(
            "{}_end",
            start_label.strip_suffix("_start").unwrap_or(start_label)
        );

        let end_index = self.require_label_index(&end_label)?;
        let branch_index =
            self.find_first_branch_to_label(label_index + 1, end_index, &end_label)?;
        let backedge_index =
            self.find_last_jump_to_label(branch_index + 1, end_index, start_label)?;
        let branch = &self.function.instructions[branch_index];
        let cond_reg = expect_register(branch.operands.first(), "loop condition")?;

        self.emit_opcode("block");
        self.bytes.push(BLOCK_TYPE_EMPTY);
        self.emit_opcode("loop");
        self.bytes.push(BLOCK_TYPE_EMPTY);

        self.emit_region(label_index + 1, branch_index)?;
        self.emit_local_get(cond_reg);
        if branch.opcode == IrOp::BranchZ {
            self.emit_opcode("i32.eqz");
        }
        self.emit_opcode("br_if");
        self.emit_u32(1);

        self.emit_region(branch_index + 1, backedge_index)?;
        self.emit_opcode("br");
        self.emit_u32(0);
        self.emit_opcode("end");
        self.emit_opcode("end");

        Ok(end_index + 1)
    }

    fn emit_simple(&mut self, instruction: &IrInstruction) -> Result<(), WasmLoweringError> {
        match instruction.opcode {
            IrOp::LoadImm => {
                let dst = expect_register(instruction.operands.first(), "LOAD_IMM dst")?;
                let value = expect_immediate(instruction.operands.get(1), "LOAD_IMM imm")?;
                self.emit_i32_const(value);
                self.emit_local_set(dst);
            }
            IrOp::LoadAddr => {
                let dst = expect_register(instruction.operands.first(), "LOAD_ADDR dst")?;
                let label = expect_label(instruction.operands.get(1), "LOAD_ADDR label")?;
                let offset = self.data_offsets.get(&label).ok_or_else(|| {
                    WasmLoweringError::new(format!("unknown data label: {}", label))
                })?;
                self.emit_i32_const(*offset as i64);
                self.emit_local_set(dst);
            }
            IrOp::LoadByte | IrOp::LoadWord => {
                let dst = expect_register(instruction.operands.first(), "load dst")?;
                let base = expect_register(instruction.operands.get(1), "load base")?;
                let offset = expect_register(instruction.operands.get(2), "load offset")?;
                self.emit_address(base, offset);
                if instruction.opcode == IrOp::LoadByte {
                    self.emit_opcode("i32.load8_u");
                    self.emit_memarg(0, 0);
                } else {
                    self.emit_opcode("i32.load");
                    self.emit_memarg(2, 0);
                }
                self.emit_local_set(dst);
            }
            IrOp::StoreByte | IrOp::StoreWord => {
                let src = expect_register(instruction.operands.first(), "store src")?;
                let base = expect_register(instruction.operands.get(1), "store base")?;
                let offset = expect_register(instruction.operands.get(2), "store offset")?;
                self.emit_address(base, offset);
                self.emit_local_get(src);
                if instruction.opcode == IrOp::StoreByte {
                    self.emit_opcode("i32.store8");
                    self.emit_memarg(0, 0);
                } else {
                    self.emit_opcode("i32.store");
                    self.emit_memarg(2, 0);
                }
            }
            IrOp::Add => self.emit_binary_numeric("i32.add", instruction)?,
            IrOp::AddImm => {
                let dst = expect_register(instruction.operands.first(), "ADD_IMM dst")?;
                let src = expect_register(instruction.operands.get(1), "ADD_IMM src")?;
                let value = expect_immediate(instruction.operands.get(2), "ADD_IMM imm")?;
                self.emit_local_get(src);
                self.emit_i32_const(value);
                self.emit_opcode("i32.add");
                self.emit_local_set(dst);
            }
            IrOp::Sub => self.emit_binary_numeric("i32.sub", instruction)?,
            IrOp::And => self.emit_binary_numeric("i32.and", instruction)?,
            IrOp::AndImm => {
                let dst = expect_register(instruction.operands.first(), "AND_IMM dst")?;
                let src = expect_register(instruction.operands.get(1), "AND_IMM src")?;
                let value = expect_immediate(instruction.operands.get(2), "AND_IMM imm")?;
                self.emit_local_get(src);
                self.emit_i32_const(value);
                self.emit_opcode("i32.and");
                self.emit_local_set(dst);
            }
            IrOp::CmpEq => self.emit_binary_numeric("i32.eq", instruction)?,
            IrOp::CmpNe => self.emit_binary_numeric("i32.ne", instruction)?,
            IrOp::CmpLt => self.emit_binary_numeric("i32.lt_s", instruction)?,
            IrOp::CmpGt => self.emit_binary_numeric("i32.gt_s", instruction)?,
            IrOp::Call => {
                let label = expect_label(instruction.operands.first(), "CALL target")?;
                let signature = self.signatures.get(&label).ok_or_else(|| {
                    WasmLoweringError::new(format!("missing function signature for {}", label))
                })?;
                let function_index = self.function_indices.get(&label).ok_or_else(|| {
                    WasmLoweringError::new(format!("unknown function label: {}", label))
                })?;
                for param_index in 0..signature.param_count {
                    self.emit_local_get(REG_VAR_BASE + param_index);
                }
                self.emit_opcode("call");
                self.emit_u32(*function_index);
                self.emit_local_set(REG_SCRATCH);
            }
            IrOp::Ret | IrOp::Halt => {
                self.emit_local_get(REG_SCRATCH);
                self.emit_opcode("return");
            }
            IrOp::Nop => self.emit_opcode("nop"),
            IrOp::Syscall => self.emit_syscall(instruction)?,
            _ => {
                return Err(WasmLoweringError::new(format!(
                    "unsupported opcode: {}",
                    instruction.opcode
                )))
            }
        }

        Ok(())
    }

    fn emit_syscall(&mut self, instruction: &IrInstruction) -> Result<(), WasmLoweringError> {
        match expect_immediate(instruction.operands.first(), "SYSCALL number")? {
            SYSCALL_WRITE => self.emit_wasi_write(),
            SYSCALL_READ => self.emit_wasi_read(),
            SYSCALL_EXIT => self.emit_wasi_exit(),
            value => Err(WasmLoweringError::new(format!(
                "unsupported SYSCALL number: {}",
                value
            ))),
        }
    }

    fn emit_wasi_write(&mut self) -> Result<(), WasmLoweringError> {
        let scratch_base = self.require_wasi_scratch()?;
        let iovec_ptr = scratch_base + WASI_IOVEC_OFFSET;
        let nwritten_ptr = scratch_base + WASI_COUNT_OFFSET;
        let byte_ptr = scratch_base + WASI_BYTE_OFFSET;

        self.emit_i32_const(byte_ptr as i64);
        self.emit_local_get(SYSCALL_ARG0);
        self.emit_opcode("i32.store8");
        self.emit_memarg(0, 0);

        self.emit_store_const_i32(iovec_ptr, byte_ptr);
        self.emit_store_const_i32(iovec_ptr + 4, 1);

        self.emit_i32_const(1);
        self.emit_i32_const(iovec_ptr as i64);
        self.emit_i32_const(1);
        self.emit_i32_const(nwritten_ptr as i64);
        self.emit_wasi_call(SYSCALL_WRITE)?;
        self.emit_local_set(REG_SCRATCH);
        Ok(())
    }

    fn emit_wasi_read(&mut self) -> Result<(), WasmLoweringError> {
        let scratch_base = self.require_wasi_scratch()?;
        let iovec_ptr = scratch_base + WASI_IOVEC_OFFSET;
        let nread_ptr = scratch_base + WASI_COUNT_OFFSET;
        let byte_ptr = scratch_base + WASI_BYTE_OFFSET;

        self.emit_i32_const(byte_ptr as i64);
        self.emit_i32_const(0);
        self.emit_opcode("i32.store8");
        self.emit_memarg(0, 0);

        self.emit_store_const_i32(iovec_ptr, byte_ptr);
        self.emit_store_const_i32(iovec_ptr + 4, 1);

        self.emit_i32_const(0);
        self.emit_i32_const(iovec_ptr as i64);
        self.emit_i32_const(1);
        self.emit_i32_const(nread_ptr as i64);
        self.emit_wasi_call(SYSCALL_READ)?;
        self.emit_local_set(REG_SCRATCH);

        self.emit_i32_const(byte_ptr as i64);
        self.emit_opcode("i32.load8_u");
        self.emit_memarg(0, 0);
        self.emit_local_set(SYSCALL_ARG0);
        Ok(())
    }

    fn emit_wasi_exit(&mut self) -> Result<(), WasmLoweringError> {
        self.emit_local_get(SYSCALL_ARG0);
        self.emit_wasi_call(SYSCALL_EXIT)?;
        self.emit_i32_const(0);
        self.emit_opcode("return");
        Ok(())
    }

    fn emit_store_const_i32(&mut self, address: u32, value: u32) {
        self.emit_i32_const(address as i64);
        self.emit_i32_const(value as i64);
        self.emit_opcode("i32.store");
        self.emit_memarg(2, 0);
    }

    fn emit_wasi_call(&mut self, syscall_number: i64) -> Result<(), WasmLoweringError> {
        let function_index = self
            .wasi_context
            .function_indices
            .get(&syscall_number)
            .ok_or_else(|| {
                WasmLoweringError::new(format!(
                    "missing WASI import for SYSCALL {}",
                    syscall_number
                ))
            })?;
        self.emit_opcode("call");
        self.emit_u32(*function_index);
        Ok(())
    }

    fn require_wasi_scratch(&self) -> Result<u32, WasmLoweringError> {
        self.wasi_context
            .scratch_base
            .ok_or_else(|| WasmLoweringError::new("SYSCALL lowering requires WASM scratch memory"))
    }

    fn emit_binary_numeric(
        &mut self,
        opcode_name: &str,
        instruction: &IrInstruction,
    ) -> Result<(), WasmLoweringError> {
        let context = instruction.opcode.to_string();
        let dst = expect_register(instruction.operands.first(), &format!("{} dst", context))?;
        let left = expect_register(instruction.operands.get(1), &format!("{} lhs", context))?;
        let right = expect_register(instruction.operands.get(2), &format!("{} rhs", context))?;
        self.emit_local_get(left);
        self.emit_local_get(right);
        self.emit_opcode(opcode_name);
        self.emit_local_set(dst);
        Ok(())
    }

    fn emit_address(&mut self, base_index: usize, offset_index: usize) {
        self.emit_local_get(base_index);
        self.emit_local_get(offset_index);
        self.emit_opcode("i32.add");
    }

    fn emit_local_get(&mut self, reg_index: usize) {
        self.emit_opcode("local.get");
        self.emit_u32(self.local_index(reg_index) as u32);
    }

    fn emit_local_set(&mut self, reg_index: usize) {
        self.emit_opcode("local.set");
        self.emit_u32(self.local_index(reg_index) as u32);
    }

    fn emit_i32_const(&mut self, value: i64) {
        self.emit_opcode("i32.const");
        self.bytes.extend(encode_signed(value));
    }

    fn emit_memarg(&mut self, align: u32, offset: u32) {
        self.emit_u32(align);
        self.emit_u32(offset);
    }

    fn emit_opcode(&mut self, name: &str) {
        self.bytes.push(get_opcode_by_name(name).unwrap().opcode);
    }

    fn emit_u32(&mut self, value: u32) {
        self.bytes.extend(encode_unsigned(value as u64));
    }

    fn local_index(&self, reg_index: usize) -> usize {
        self.function.signature.param_count + reg_index
    }

    fn require_label_index(&self, label: &str) -> Result<usize, WasmLoweringError> {
        self.label_to_index.get(label).copied().ok_or_else(|| {
            WasmLoweringError::new(format!(
                "missing label {} in {}",
                label, self.function.label
            ))
        })
    }

    fn find_first_branch_to_label(
        &self,
        start: usize,
        end: usize,
        label: &str,
    ) -> Result<usize, WasmLoweringError> {
        for index in start..end {
            let instruction = &self.function.instructions[index];
            if !matches!(instruction.opcode, IrOp::BranchZ | IrOp::BranchNz) {
                continue;
            }
            if instruction
                .operands
                .get(1)
                .and_then(label_operand)
                .is_some_and(|name| name == label)
            {
                return Ok(index);
            }
        }
        Err(WasmLoweringError::new(format!(
            "expected branch to {} in {}",
            label, self.function.label
        )))
    }

    fn find_last_jump_to_label(
        &self,
        start: usize,
        end: usize,
        label: &str,
    ) -> Result<usize, WasmLoweringError> {
        for index in (start..end).rev() {
            let instruction = &self.function.instructions[index];
            if instruction.opcode != IrOp::Jump {
                continue;
            }
            if instruction
                .operands
                .first()
                .and_then(label_operand)
                .is_some_and(|name| name == label)
            {
                return Ok(index);
            }
        }
        Err(WasmLoweringError::new(format!(
            "expected jump to {} in {}",
            label, self.function.label
        )))
    }
}

fn const_expr(value: i64) -> Vec<u8> {
    let mut bytes = vec![get_opcode_by_name("i32.const").unwrap().opcode];
    bytes.extend(encode_signed(value));
    bytes.push(get_opcode_by_name("end").unwrap().opcode);
    bytes
}

fn function_label_name(instruction: &IrInstruction) -> Option<&str> {
    let label = label_name_from_instruction(instruction)?;
    if label == "_start" || label.starts_with("_fn_") {
        Some(label)
    } else {
        None
    }
}

fn label_name_from_instruction(instruction: &IrInstruction) -> Option<&str> {
    if instruction.opcode != IrOp::Label {
        return None;
    }
    label_operand(instruction.operands.first()?)
}

fn label_operand(operand: &IrOperand) -> Option<&str> {
    match operand {
        IrOperand::Label(label) => Some(label.as_str()),
        _ => None,
    }
}

fn expect_register(operand: Option<&IrOperand>, context: &str) -> Result<usize, WasmLoweringError> {
    match operand {
        Some(IrOperand::Register(index)) => Ok(*index),
        other => Err(WasmLoweringError::new(format!(
            "{}: expected register, got {:?}",
            context, other
        ))),
    }
}

fn expect_immediate(operand: Option<&IrOperand>, context: &str) -> Result<i64, WasmLoweringError> {
    match operand {
        Some(IrOperand::Immediate(value)) => Ok(*value),
        other => Err(WasmLoweringError::new(format!(
            "{}: expected immediate, got {:?}",
            context, other
        ))),
    }
}

fn expect_label(operand: Option<&IrOperand>, context: &str) -> Result<String, WasmLoweringError> {
    match operand {
        Some(IrOperand::Label(label)) => Ok(label.clone()),
        other => Err(WasmLoweringError::new(format!(
            "{}: expected label, got {:?}",
            context, other
        ))),
    }
}

fn is_loop_start(label: &str) -> bool {
    label
        .strip_prefix("loop_")
        .and_then(|rest| rest.strip_suffix("_start"))
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_if_else_label(label: &str) -> bool {
    label
        .strip_prefix("if_")
        .and_then(|rest| rest.strip_suffix("_else"))
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
}

fn align_up(value: u32, alignment: u32) -> u32 {
    ((value + alignment - 1) / alignment) * alignment
}

fn total_data_size(decls: &[IrDataDecl]) -> u32 {
    decls.iter().map(|decl| decl.size as u32).sum()
}

fn func_type_key(func_type: &FuncType) -> String {
    format!("{:?}=>{:?}", func_type.params, func_type.results)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use brainfuck::parser::parse_brainfuck;
    use brainfuck_ir_compiler::{compile, release_config};
    use wasm_module_encoder::encode_module;
    use wasm_runtime::{WasiConfig, WasiEnv, WasmRuntime};

    use super::*;

    #[test]
    fn lowers_brainfuck_ir_into_wasm_module_with_wasi_imports() {
        let ast = parse_brainfuck(",.").unwrap();
        let compiled = compile(&ast, "echo.bf", release_config()).unwrap();

        let module = IrToWasmCompiler::default()
            .compile(&compiled.program, &[])
            .unwrap();

        assert_eq!(module.memories.len(), 1);
        assert!(module.exports.iter().any(|entry| entry.name == "memory"));
        assert!(module.exports.iter().any(|entry| entry.name == "_start"));
        assert_eq!(
            module
                .imports
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["fd_write", "fd_read"]
        );
    }

    #[test]
    fn runs_lowered_brainfuck_echo_through_runtime() {
        let ast = parse_brainfuck(",.").unwrap();
        let compiled = compile(&ast, "echo.bf", release_config()).unwrap();
        let module = IrToWasmCompiler::default()
            .compile(&compiled.program, &[])
            .unwrap();
        let binary = encode_module(&module).unwrap();

        let output = Arc::new(Mutex::new(String::new()));
        let output_ref = Arc::clone(&output);
        let wasi = WasiEnv::new(WasiConfig {
            stdout_callback: Some(Box::new(move |text| {
                output_ref.lock().unwrap().push_str(text);
            })),
            stdin_callback: Some(Box::new(|requested| {
                let mut bytes = b"Q".to_vec();
                bytes.truncate(requested);
                bytes
            })),
            ..Default::default()
        });
        let runtime = WasmRuntime::with_host(Box::new(wasi));

        let result = runtime.load_and_run(&binary, "_start", &[]).unwrap();

        assert_eq!(result, vec![0]);
        assert_eq!(output.lock().unwrap().as_str(), "Q");
    }

    #[test]
    fn rejects_unsupported_syscall_number() {
        let program = IrProgram {
            instructions: vec![
                IrInstruction::new(IrOp::Label, vec![IrOperand::Label("_start".into())], -1),
                IrInstruction::new(IrOp::Syscall, vec![IrOperand::Immediate(99)], 0),
            ],
            data: vec![],
            entry_label: "_start".into(),
            version: 1,
        };

        let err = IrToWasmCompiler::default()
            .compile(&program, &[])
            .unwrap_err();

        assert!(err.message.contains("unsupported SYSCALL number"));
    }
}

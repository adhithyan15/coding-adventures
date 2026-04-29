//! # ir-to-jvm-class-file
//!
//! This crate is the Rust generic JVM backend for `compiler-ir`.
//!
//! The central idea is simple: keep the generated class extremely ordinary.
//! Virtual registers live in a static `int[]`, data lives in a static `byte[]`,
//! and each callable IR region becomes a plain static JVM method. "Boring" here
//! is a feature, because boring bytecode is easy for both the JVM verifier and
//! GraalVM Native Image to accept.

pub mod codegen;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::CString;
use std::fmt;
use std::fs;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};
use jvm_class_file::{ACC_PUBLIC, ACC_STATIC, ACC_SUPER};

const ACC_PRIVATE: u16 = 0x0002;
const ACC_FINAL: u16 = 0x0010;

const CONSTANT_UTF8: u8 = 1;
const CONSTANT_INTEGER: u8 = 3;
const CONSTANT_CLASS: u8 = 7;
const CONSTANT_FIELDREF: u8 = 9;
const CONSTANT_METHODREF: u8 = 10;
const CONSTANT_NAME_AND_TYPE: u8 = 12;

const OP_NOP: u8 = 0x00;
const OP_ICONST_M1: u8 = 0x02;
const OP_ICONST_0: u8 = 0x03;
const OP_BIPUSH: u8 = 0x10;
const OP_SIPUSH: u8 = 0x11;
const OP_LDC: u8 = 0x12;
const OP_LDC_W: u8 = 0x13;
const OP_ILOAD: u8 = 0x15;
const OP_LLOAD_0: u8 = 0x1a;
const OP_ISTORE: u8 = 0x36;
const OP_ISTORE_0: u8 = 0x3b;
const OP_IALOAD: u8 = 0x2e;
const OP_BALOAD: u8 = 0x33;
const OP_IASTORE: u8 = 0x4f;
const OP_BASTORE: u8 = 0x54;
const OP_POP: u8 = 0x57;
const OP_IADD: u8 = 0x60;
const OP_ISUB: u8 = 0x64;
const OP_IMUL: u8 = 0x68;
const OP_IDIV: u8 = 0x6c;
const OP_ISHL: u8 = 0x78;
const OP_ISHR: u8 = 0x7a;
const OP_IAND: u8 = 0x7e;
const OP_IOR: u8 = 0x80;
const OP_I2B: u8 = 0x91;
const OP_IFEQ: u8 = 0x99;
const OP_IFNE: u8 = 0x9a;
const OP_IF_ICMPEQ: u8 = 0x9f;
const OP_IF_ICMPNE: u8 = 0xa0;
const OP_IF_ICMPLT: u8 = 0xa1;
const OP_IF_ICMPGT: u8 = 0xa3;
const OP_GOTO: u8 = 0xa7;
const OP_IRETURN: u8 = 0xac;
const OP_RETURN: u8 = 0xb1;
const OP_GETSTATIC: u8 = 0xb2;
const OP_PUTSTATIC: u8 = 0xb3;
const OP_INVOKEVIRTUAL: u8 = 0xb6;
const OP_INVOKESTATIC: u8 = 0xb8;
const OP_NEWARRAY: u8 = 0xbc;

const ATYPE_INT: u8 = 10;
const ATYPE_BYTE: u8 = 8;

const DESC_INT_ARRAY: &str = "[I";
const DESC_BYTE_ARRAY: &str = "[B";
const DESC_MAIN: &str = "([Ljava/lang/String;)V";
const DESC_NOARGS_INT: &str = "()I";
const DESC_NOARGS_VOID: &str = "()V";
const DESC_INT_TO_INT: &str = "(I)I";
const DESC_INT_INT_TO_VOID: &str = "(II)V";
const DESC_ARRAYS_FILL_BYTE_RANGE: &str = "([BIIB)V";
const DESC_PRINTSTREAM_WRITE: &str = "(I)V";
const DESC_INPUTSTREAM_READ: &str = "()I";

const MAX_STATIC_DATA_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmBackendError {
    message: String,
}

impl JvmBackendError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for JvmBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JvmBackendError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmBackendConfig {
    pub class_name: String,
    pub class_file_major: u16,
    pub class_file_minor: u16,
    pub emit_main_wrapper: bool,
}

impl JvmBackendConfig {
    pub fn new(class_name: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            class_file_major: 49,
            class_file_minor: 0,
            emit_main_wrapper: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmClassArtifact {
    pub class_name: String,
    pub class_bytes: Vec<u8>,
    pub callable_labels: Vec<String>,
    pub data_offsets: BTreeMap<String, usize>,
}

impl JvmClassArtifact {
    pub fn class_filename(&self) -> String {
        format!("{}.class", self.class_name.replace('.', "/"))
    }
}

#[derive(Debug, Clone)]
struct FieldSpec {
    access_flags: u16,
    name: String,
    descriptor: String,
}

#[derive(Debug, Clone)]
struct MethodSpec {
    access_flags: u16,
    name: String,
    descriptor: String,
    code: Vec<u8>,
    max_stack: u16,
    max_locals: u16,
}

#[derive(Debug, Clone)]
struct CallableRegion {
    name: String,
    start_index: usize,
    end_index: usize,
    instructions: Vec<IrInstruction>,
}

#[derive(Debug, Clone)]
enum BytecodeItem {
    Label(String),
    Raw(Vec<u8>),
    Branch { opcode: u8, label: String },
}

#[derive(Debug, Default)]
struct BytecodeBuilder {
    items: Vec<BytecodeItem>,
}

impl BytecodeBuilder {
    fn mark(&mut self, label: impl Into<String>) {
        self.items.push(BytecodeItem::Label(label.into()));
    }

    fn emit_raw(&mut self, data: Vec<u8>) {
        self.items.push(BytecodeItem::Raw(data));
    }

    fn emit_opcode(&mut self, opcode: u8) {
        self.emit_raw(vec![opcode]);
    }

    fn emit_u1_instruction(&mut self, opcode: u8, operand: u8) {
        self.emit_raw(vec![opcode, operand]);
    }

    fn emit_i1_instruction(&mut self, opcode: u8, operand: i8) {
        self.emit_raw(vec![opcode, operand as u8]);
    }

    fn emit_u2_instruction(&mut self, opcode: u8, operand: u16) {
        let mut bytes = vec![opcode];
        append_u2(&mut bytes, operand);
        self.emit_raw(bytes);
    }

    fn emit_branch(&mut self, opcode: u8, label: impl Into<String>) {
        self.items.push(BytecodeItem::Branch {
            opcode,
            label: label.into(),
        });
    }

    fn assemble(&self) -> Result<Vec<u8>, JvmBackendError> {
        let mut label_offsets = HashMap::<String, usize>::new();
        let mut offset = 0usize;
        for item in &self.items {
            match item {
                BytecodeItem::Label(label) => {
                    label_offsets.insert(label.clone(), offset);
                }
                BytecodeItem::Raw(data) => {
                    offset = offset
                        .checked_add(data.len())
                        .ok_or_else(|| JvmBackendError::new("bytecode size overflow"))?;
                }
                BytecodeItem::Branch { .. } => {
                    offset = offset
                        .checked_add(3)
                        .ok_or_else(|| JvmBackendError::new("bytecode size overflow"))?;
                }
            }
        }

        let mut output = Vec::new();
        let mut offset = 0usize;
        for item in &self.items {
            match item {
                BytecodeItem::Label(_) => {}
                BytecodeItem::Raw(data) => {
                    output.extend_from_slice(data);
                    offset += data.len();
                }
                BytecodeItem::Branch { opcode, label } => {
                    let target = *label_offsets.get(label).ok_or_else(|| {
                        JvmBackendError::new(format!("Unknown bytecode label: {label}"))
                    })?;
                    let branch_offset = isize::try_from(target)
                        .and_then(|target| isize::try_from(offset).map(|here| target - here))
                        .map_err(|_| JvmBackendError::new("bytecode branch offset overflow"))?;
                    if !(i16::MIN as isize..=i16::MAX as isize).contains(&branch_offset) {
                        return Err(JvmBackendError::new(format!(
                            "Branch offset out of range for label {label}: {branch_offset}"
                        )));
                    }
                    output.push(*opcode);
                    output.extend_from_slice(&(branch_offset as i16).to_be_bytes());
                    offset += 3;
                }
            }
        }

        Ok(output)
    }
}

#[derive(Debug, Default)]
struct ConstantPoolBuilder {
    entries: Vec<Vec<u8>>,
    indices: HashMap<String, u16>,
}

impl ConstantPoolBuilder {
    fn count(&self) -> usize {
        self.entries.len() + 1
    }

    fn encode(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|entry| entry.iter().copied())
            .collect()
    }

    fn add(&mut self, key: String, payload: Vec<u8>) -> Result<u16, JvmBackendError> {
        if let Some(index) = self.indices.get(&key) {
            return Ok(*index);
        }
        self.entries.push(payload);
        let index = u16::try_from(self.entries.len())
            .map_err(|_| JvmBackendError::new("constant pool exceeds u16 count"))?;
        self.indices.insert(key, index);
        Ok(index)
    }

    fn utf8(&mut self, value: &str) -> Result<u16, JvmBackendError> {
        let encoded = value.as_bytes();
        let length = u16::try_from(encoded.len()).map_err(|_| {
            JvmBackendError::new(format!("UTF-8 constant {value:?} exceeds 65535 bytes"))
        })?;
        let mut payload = vec![CONSTANT_UTF8];
        append_u2(&mut payload, length);
        payload.extend_from_slice(encoded);
        self.add(format!("Utf8:{value}"), payload)
    }

    fn integer(&mut self, value: i32) -> Result<u16, JvmBackendError> {
        let mut payload = vec![CONSTANT_INTEGER];
        append_i4(&mut payload, value);
        self.add(format!("Integer:{value}"), payload)
    }

    fn class_ref(&mut self, internal_name: &str) -> Result<u16, JvmBackendError> {
        let name_index = self.utf8(internal_name)?;
        let mut payload = vec![CONSTANT_CLASS];
        append_u2(&mut payload, name_index);
        self.add(format!("Class:{internal_name}"), payload)
    }

    fn name_and_type(&mut self, name: &str, descriptor: &str) -> Result<u16, JvmBackendError> {
        let mut payload = vec![CONSTANT_NAME_AND_TYPE];
        append_u2(&mut payload, self.utf8(name)?);
        append_u2(&mut payload, self.utf8(descriptor)?);
        self.add(format!("NameAndType:{name}:{descriptor}"), payload)
    }

    fn field_ref(
        &mut self,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<u16, JvmBackendError> {
        let mut payload = vec![CONSTANT_FIELDREF];
        append_u2(&mut payload, self.class_ref(owner)?);
        append_u2(&mut payload, self.name_and_type(name, descriptor)?);
        self.add(format!("Fieldref:{owner}:{name}:{descriptor}"), payload)
    }

    fn method_ref(
        &mut self,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<u16, JvmBackendError> {
        let mut payload = vec![CONSTANT_METHODREF];
        append_u2(&mut payload, self.class_ref(owner)?);
        append_u2(&mut payload, self.name_and_type(name, descriptor)?);
        self.add(format!("Methodref:{owner}:{name}:{descriptor}"), payload)
    }
}

#[derive(Debug)]
struct JvmClassLowerer<'a> {
    program: &'a IrProgram,
    config: JvmBackendConfig,
    internal_name: String,
    cp: ConstantPoolBuilder,
    data_offsets: BTreeMap<String, usize>,
    fresh_label_id: usize,
    helper_reg_field: &'static str,
    helper_mem_field: &'static str,
    helper_reg_get: &'static str,
    helper_reg_set: &'static str,
    helper_mem_load_byte: &'static str,
    helper_mem_store_byte: &'static str,
    helper_load_word: &'static str,
    helper_store_word: &'static str,
    helper_syscall: &'static str,
}

impl<'a> JvmClassLowerer<'a> {
    fn new(program: &'a IrProgram, config: JvmBackendConfig) -> Self {
        Self {
            program,
            internal_name: config.class_name.replace('.', "/"),
            config,
            cp: ConstantPoolBuilder::default(),
            data_offsets: BTreeMap::new(),
            fresh_label_id: 0,
            helper_reg_field: "__ca_regs",
            helper_mem_field: "__ca_memory",
            helper_reg_get: "__ca_regGet",
            helper_reg_set: "__ca_regSet",
            helper_mem_load_byte: "__ca_memLoadByte",
            helper_mem_store_byte: "__ca_memStoreByte",
            helper_load_word: "__ca_loadWord",
            helper_store_word: "__ca_storeWord",
            helper_syscall: "__ca_syscall",
        }
    }

    fn lower(mut self) -> Result<JvmClassArtifact, JvmBackendError> {
        validate_class_name(&self.config.class_name)?;
        let label_positions = self.collect_labels()?;
        let callable_regions = self.discover_callable_regions(&label_positions)?;
        self.validate_helper_name_collisions(&callable_regions)?;
        self.data_offsets = self.assign_data_offsets()?;
        let reg_count = self.max_register_index() + 1;

        let fields = vec![
            FieldSpec {
                access_flags: ACC_PRIVATE | ACC_STATIC,
                name: self.helper_reg_field.to_string(),
                descriptor: DESC_INT_ARRAY.to_string(),
            },
            FieldSpec {
                access_flags: ACC_PRIVATE | ACC_STATIC,
                name: self.helper_mem_field.to_string(),
                descriptor: DESC_BYTE_ARRAY.to_string(),
            },
        ];

        let mut methods = vec![
            self.build_class_initializer(reg_count)?,
            self.build_reg_get_method()?,
            self.build_reg_set_method()?,
            self.build_mem_load_byte_method()?,
            self.build_mem_store_byte_method()?,
            self.build_load_word_method()?,
            self.build_store_word_method()?,
            self.build_syscall_method()?,
        ];
        for region in &callable_regions {
            methods.push(self.build_callable_method(region, &label_positions)?);
        }
        if self.config.emit_main_wrapper {
            methods.push(self.build_main_method()?);
        }

        let class_bytes = self.encode_class_file(&fields, &methods)?;
        Ok(JvmClassArtifact {
            class_name: self.config.class_name,
            class_bytes,
            callable_labels: callable_regions
                .into_iter()
                .map(|region| region.name)
                .collect(),
            data_offsets: self.data_offsets,
        })
    }

    fn collect_labels(&self) -> Result<HashMap<String, usize>, JvmBackendError> {
        let mut positions = HashMap::new();
        for (index, instruction) in self.program.instructions.iter().enumerate() {
            if instruction.opcode != IrOp::Label {
                continue;
            }
            let label = as_label(instruction.operands.first(), "LABEL operand")?;
            if positions.insert(label.to_string(), index).is_some() {
                return Err(JvmBackendError::new(format!("Duplicate IR label: {label}")));
            }
        }
        Ok(positions)
    }

    fn discover_callable_regions(
        &self,
        label_positions: &HashMap<String, usize>,
    ) -> Result<Vec<CallableRegion>, JvmBackendError> {
        let mut callable_names = BTreeSet::new();
        callable_names.insert(self.program.entry_label.clone());
        for instruction in &self.program.instructions {
            if instruction.opcode == IrOp::Call {
                callable_names
                    .insert(as_label(instruction.operands.first(), "CALL target")?.to_string());
            }
        }
        if !label_positions.contains_key(&self.program.entry_label) {
            return Err(JvmBackendError::new(format!(
                "Entry label not found: {}",
                self.program.entry_label
            )));
        }

        let mut ordered_names: Vec<String> = callable_names.into_iter().collect();
        ordered_names.sort_by_key(|name| label_positions.get(name).copied().unwrap_or(usize::MAX));
        let mut regions = Vec::with_capacity(ordered_names.len());
        for (index, name) in ordered_names.iter().enumerate() {
            let start_index = *label_positions
                .get(name)
                .ok_or_else(|| JvmBackendError::new(format!("Missing callable label: {name}")))?;
            let end_index = ordered_names
                .get(index + 1)
                .and_then(|next| label_positions.get(next).copied())
                .unwrap_or(self.program.instructions.len());
            let instructions = self.program.instructions[start_index..end_index].to_vec();
            regions.push(CallableRegion {
                name: name.clone(),
                start_index,
                end_index,
                instructions,
            });
        }

        let callable_lookup: BTreeSet<String> =
            regions.iter().map(|region| region.name.clone()).collect();
        for region in &regions {
            for instruction in &region.instructions {
                match instruction.opcode {
                    IrOp::Jump | IrOp::BranchZ | IrOp::BranchNz => {
                        let target = as_label(instruction.operands.last(), "branch target")?;
                        let target_index = label_positions.get(target).ok_or_else(|| {
                            JvmBackendError::new(format!("Branch target {target:?} does not exist"))
                        })?;
                        if !((*target_index >= region.start_index)
                            && (*target_index < region.end_index))
                        {
                            return Err(JvmBackendError::new(format!(
                                "Branch target {target:?} escapes callable {:?}",
                                region.name
                            )));
                        }
                    }
                    IrOp::Call => {
                        let target = as_label(instruction.operands.first(), "CALL target")?;
                        if !callable_lookup.contains(target) {
                            return Err(JvmBackendError::new(format!(
                                "CALL target {target:?} is not callable"
                            )));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(regions)
    }

    fn validate_helper_name_collisions(
        &self,
        regions: &[CallableRegion],
    ) -> Result<(), JvmBackendError> {
        let reserved = [
            self.helper_reg_get,
            self.helper_reg_set,
            self.helper_mem_load_byte,
            self.helper_mem_store_byte,
            self.helper_load_word,
            self.helper_store_word,
            self.helper_syscall,
            "<clinit>",
            "main",
        ];
        let collisions: Vec<&str> = reserved
            .iter()
            .copied()
            .filter(|reserved_name| regions.iter().any(|region| region.name == *reserved_name))
            .collect();
        if collisions.is_empty() {
            Ok(())
        } else {
            Err(JvmBackendError::new(format!(
                "Callable labels collide with helper names: {collisions:?}"
            )))
        }
    }

    fn assign_data_offsets(&self) -> Result<BTreeMap<String, usize>, JvmBackendError> {
        let mut offsets = BTreeMap::new();
        let mut offset = 0usize;
        for declaration in &self.program.data {
            offsets.insert(declaration.label.clone(), offset);
            offset = offset
                .checked_add(declaration.size)
                .ok_or_else(|| JvmBackendError::new("static data size overflow"))?;
            if offset > MAX_STATIC_DATA_BYTES {
                return Err(JvmBackendError::new(format!(
                    "Total static data exceeds the JVM backend limit of {MAX_STATIC_DATA_BYTES} bytes"
                )));
            }
        }
        Ok(offsets)
    }

    fn max_register_index(&self) -> usize {
        self.program
            .instructions
            .iter()
            .flat_map(|instruction| instruction.operands.iter())
            .filter_map(|operand| match operand {
                IrOperand::Register(index) => Some(*index),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        self.fresh_label_id += 1;
        format!("__ca_{prefix}_{}", self.fresh_label_id)
    }

    fn field_ref(&mut self, name: &str, descriptor: &str) -> Result<u16, JvmBackendError> {
        self.cp.field_ref(&self.internal_name, name, descriptor)
    }

    fn method_ref(&mut self, name: &str, descriptor: &str) -> Result<u16, JvmBackendError> {
        self.cp.method_ref(&self.internal_name, name, descriptor)
    }

    fn emit_push_int(
        &mut self,
        builder: &mut BytecodeBuilder,
        value: i64,
    ) -> Result<(), JvmBackendError> {
        if value == -1 {
            builder.emit_opcode(OP_ICONST_M1);
            return Ok(());
        }
        if (0..=5).contains(&value) {
            builder.emit_opcode(OP_ICONST_0 + value as u8);
            return Ok(());
        }
        if (i8::MIN as i64..=i8::MAX as i64).contains(&value) {
            builder.emit_i1_instruction(OP_BIPUSH, value as i8);
            return Ok(());
        }
        if (i16::MIN as i64..=i16::MAX as i64).contains(&value) {
            let mut bytes = vec![OP_SIPUSH];
            bytes.extend_from_slice(&(value as i16).to_be_bytes());
            builder.emit_raw(bytes);
            return Ok(());
        }
        let index = self.cp.integer(i32::try_from(value).map_err(|_| {
            JvmBackendError::new(format!("Immediate {value} is outside JVM int range"))
        })?)?;
        if index <= u8::MAX.into() {
            builder.emit_u1_instruction(OP_LDC, index as u8);
        } else {
            builder.emit_u2_instruction(OP_LDC_W, index);
        }
        Ok(())
    }

    fn emit_iload(&self, builder: &mut BytecodeBuilder, index: u8) {
        if index <= 3 {
            builder.emit_opcode(OP_LLOAD_0 + index);
        } else {
            builder.emit_u1_instruction(OP_ILOAD, index);
        }
    }

    fn emit_istore(&self, builder: &mut BytecodeBuilder, index: u8) {
        if index <= 3 {
            builder.emit_opcode(OP_ISTORE_0 + index);
        } else {
            builder.emit_u1_instruction(OP_ISTORE, index);
        }
    }

    fn emit_reg_get(
        &mut self,
        builder: &mut BytecodeBuilder,
        index: usize,
    ) -> Result<(), JvmBackendError> {
        self.emit_push_int(builder, index as i64)?;
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.method_ref(self.helper_reg_get, DESC_INT_TO_INT)?,
        );
        Ok(())
    }

    fn emit_reg_set(
        &mut self,
        builder: &mut BytecodeBuilder,
        index: usize,
        value: i64,
    ) -> Result<(), JvmBackendError> {
        self.emit_push_int(builder, index as i64)?;
        self.emit_push_int(builder, value)?;
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
        );
        Ok(())
    }

    fn build_class_initializer(&mut self, reg_count: usize) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();

        self.emit_push_int(&mut builder, reg_count as i64)?;
        builder.emit_u1_instruction(OP_NEWARRAY, ATYPE_INT);
        builder.emit_u2_instruction(
            OP_PUTSTATIC,
            self.field_ref(self.helper_reg_field, DESC_INT_ARRAY)?,
        );

        let total_bytes: usize = self.program.data.iter().map(|decl| decl.size).sum();
        self.emit_push_int(&mut builder, total_bytes as i64)?;
        builder.emit_u1_instruction(OP_NEWARRAY, ATYPE_BYTE);
        builder.emit_u2_instruction(
            OP_PUTSTATIC,
            self.field_ref(self.helper_mem_field, DESC_BYTE_ARRAY)?,
        );

        for declaration in &self.program.data {
            if declaration.size == 0 || declaration.init == 0 {
                continue;
            }
            let start = *self
                .data_offsets
                .get(&declaration.label)
                .ok_or_else(|| JvmBackendError::new("missing data offset for declaration"))?;
            self.emit_fill_byte_range(&mut builder, start, declaration.size, declaration.init)?;
        }

        builder.emit_opcode(OP_RETURN);
        Ok(MethodSpec {
            access_flags: ACC_STATIC,
            name: "<clinit>".to_string(),
            descriptor: DESC_NOARGS_VOID.to_string(),
            code: builder.assemble()?,
            max_stack: 8,
            max_locals: 0,
        })
    }

    fn build_reg_get_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.field_ref(self.helper_reg_field, DESC_INT_ARRAY)?,
        );
        self.emit_iload(&mut builder, 0);
        builder.emit_opcode(OP_IALOAD);
        builder.emit_opcode(OP_IRETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_reg_get.to_string(),
            descriptor: DESC_INT_TO_INT.to_string(),
            code: builder.assemble()?,
            max_stack: 2,
            max_locals: 1,
        })
    }

    fn build_reg_set_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.field_ref(self.helper_reg_field, DESC_INT_ARRAY)?,
        );
        self.emit_iload(&mut builder, 0);
        self.emit_iload(&mut builder, 1);
        builder.emit_opcode(OP_IASTORE);
        builder.emit_opcode(OP_RETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_reg_set.to_string(),
            descriptor: DESC_INT_INT_TO_VOID.to_string(),
            code: builder.assemble()?,
            max_stack: 3,
            max_locals: 2,
        })
    }

    fn emit_fill_byte_range(
        &mut self,
        builder: &mut BytecodeBuilder,
        start: usize,
        size: usize,
        value: u8,
    ) -> Result<(), JvmBackendError> {
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.field_ref(self.helper_mem_field, DESC_BYTE_ARRAY)?,
        );
        self.emit_push_int(builder, start as i64)?;
        self.emit_push_int(builder, (start + size) as i64)?;
        self.emit_push_int(builder, i64::from(value))?;
        builder.emit_opcode(OP_I2B);
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.cp
                .method_ref("java/util/Arrays", "fill", DESC_ARRAYS_FILL_BYTE_RANGE)?,
        );
        Ok(())
    }

    fn build_mem_load_byte_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.field_ref(self.helper_mem_field, DESC_BYTE_ARRAY)?,
        );
        self.emit_iload(&mut builder, 0);
        builder.emit_opcode(OP_BALOAD);
        self.emit_push_int(&mut builder, 0xff)?;
        builder.emit_opcode(OP_IAND);
        builder.emit_opcode(OP_IRETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_mem_load_byte.to_string(),
            descriptor: DESC_INT_TO_INT.to_string(),
            code: builder.assemble()?,
            max_stack: 2,
            max_locals: 1,
        })
    }

    fn build_mem_store_byte_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.field_ref(self.helper_mem_field, DESC_BYTE_ARRAY)?,
        );
        self.emit_iload(&mut builder, 0);
        self.emit_iload(&mut builder, 1);
        builder.emit_opcode(OP_BASTORE);
        builder.emit_opcode(OP_RETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_mem_store_byte.to_string(),
            descriptor: DESC_INT_INT_TO_VOID.to_string(),
            code: builder.assemble()?,
            max_stack: 3,
            max_locals: 2,
        })
    }

    fn build_load_word_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        self.emit_iload(&mut builder, 0);
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.method_ref(self.helper_mem_load_byte, DESC_INT_TO_INT)?,
        );
        for (shift, extra) in [(8i64, 1i64), (16, 2), (24, 3)] {
            self.emit_iload(&mut builder, 0);
            self.emit_push_int(&mut builder, extra)?;
            builder.emit_opcode(OP_IADD);
            builder.emit_u2_instruction(
                OP_INVOKESTATIC,
                self.method_ref(self.helper_mem_load_byte, DESC_INT_TO_INT)?,
            );
            self.emit_push_int(&mut builder, shift)?;
            builder.emit_opcode(OP_ISHL);
            builder.emit_opcode(OP_IOR);
        }
        builder.emit_opcode(OP_IRETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_load_word.to_string(),
            descriptor: DESC_INT_TO_INT.to_string(),
            code: builder.assemble()?,
            max_stack: 4,
            max_locals: 1,
        })
    }

    fn build_store_word_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        for (shift, extra) in [(0i64, 0i64), (8, 1), (16, 2), (24, 3)] {
            self.emit_iload(&mut builder, 0);
            if extra != 0 {
                self.emit_push_int(&mut builder, extra)?;
                builder.emit_opcode(OP_IADD);
            }
            self.emit_iload(&mut builder, 1);
            if shift != 0 {
                self.emit_push_int(&mut builder, shift)?;
                builder.emit_opcode(OP_ISHR);
            }
            builder.emit_u2_instruction(
                OP_INVOKESTATIC,
                self.method_ref(self.helper_mem_store_byte, DESC_INT_INT_TO_VOID)?,
            );
        }
        builder.emit_opcode(OP_RETURN);
        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_store_word.to_string(),
            descriptor: DESC_INT_INT_TO_VOID.to_string(),
            code: builder.assemble()?,
            max_stack: 4,
            max_locals: 2,
        })
    }

    fn build_syscall_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        let label_read = self.fresh_label("sys_read");
        let label_halt = self.fresh_label("sys_halt");
        let label_have_input = self.fresh_label("sys_have_input");

        self.emit_iload(&mut builder, 0);
        self.emit_push_int(&mut builder, 1)?;
        builder.emit_branch(OP_IF_ICMPNE, label_read.clone());
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.cp
                .field_ref("java/lang/System", "out", "Ljava/io/PrintStream;")?,
        );
        self.emit_reg_get(&mut builder, 4)?;
        self.emit_push_int(&mut builder, 0xff)?;
        builder.emit_opcode(OP_IAND);
        builder.emit_u2_instruction(
            OP_INVOKEVIRTUAL,
            self.cp
                .method_ref("java/io/PrintStream", "write", DESC_PRINTSTREAM_WRITE)?,
        );
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.cp
                .field_ref("java/lang/System", "out", "Ljava/io/PrintStream;")?,
        );
        builder.emit_u2_instruction(
            OP_INVOKEVIRTUAL,
            self.cp
                .method_ref("java/io/PrintStream", "flush", DESC_NOARGS_VOID)?,
        );
        builder.emit_opcode(OP_RETURN);

        builder.mark(label_read.clone());
        self.emit_iload(&mut builder, 0);
        self.emit_push_int(&mut builder, 2)?;
        builder.emit_branch(OP_IF_ICMPNE, label_halt.clone());
        builder.emit_u2_instruction(
            OP_GETSTATIC,
            self.cp
                .field_ref("java/lang/System", "in", "Ljava/io/InputStream;")?,
        );
        builder.emit_u2_instruction(
            OP_INVOKEVIRTUAL,
            self.cp
                .method_ref("java/io/InputStream", "read", DESC_INPUTSTREAM_READ)?,
        );
        self.emit_istore(&mut builder, 1);
        self.emit_iload(&mut builder, 1);
        self.emit_push_int(&mut builder, -1)?;
        builder.emit_branch(OP_IF_ICMPNE, label_have_input.clone());
        self.emit_reg_set(&mut builder, 4, 0)?;
        builder.emit_opcode(OP_RETURN);

        builder.mark(label_have_input);
        self.emit_push_int(&mut builder, 4)?;
        self.emit_iload(&mut builder, 1);
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
        );
        builder.emit_opcode(OP_RETURN);

        builder.mark(label_halt);
        builder.emit_opcode(OP_RETURN);

        Ok(MethodSpec {
            access_flags: ACC_PRIVATE | ACC_STATIC,
            name: self.helper_syscall.to_string(),
            descriptor: "(I)V".to_string(),
            code: builder.assemble()?,
            max_stack: 4,
            max_locals: 2,
        })
    }

    fn build_callable_method(
        &mut self,
        region: &CallableRegion,
        _label_positions: &HashMap<String, usize>,
    ) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();

        for instruction in &region.instructions {
            match instruction.opcode {
                IrOp::Label => {
                    builder.mark(as_label(instruction.operands.first(), "LABEL operand")?);
                }
                IrOp::Comment => {}
                IrOp::Nop => builder.emit_opcode(OP_NOP),
                IrOp::LoadImm => {
                    let dst = as_register(instruction.operands.first(), "LOAD_IMM dst")?;
                    let imm = as_immediate(instruction.operands.get(1), "LOAD_IMM immediate")?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_push_int(&mut builder, imm)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::LoadAddr => {
                    let dst = as_register(instruction.operands.first(), "LOAD_ADDR dst")?;
                    let label = as_label(instruction.operands.get(1), "LOAD_ADDR label")?;
                    let offset = *self.data_offsets.get(label).ok_or_else(|| {
                        JvmBackendError::new(format!("Unknown data label: {label}"))
                    })?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_push_int(&mut builder, offset as i64)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::LoadByte => {
                    let dst = as_register(instruction.operands.first(), "LOAD_BYTE dst")?;
                    let base = as_register(instruction.operands.get(1), "LOAD_BYTE base")?;
                    let offset = as_register(instruction.operands.get(2), "LOAD_BYTE offset")?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_reg_get(&mut builder, base)?;
                    self.emit_reg_get(&mut builder, offset)?;
                    builder.emit_opcode(OP_IADD);
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_mem_load_byte, DESC_INT_TO_INT)?,
                    );
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::StoreByte => {
                    let src = as_register(instruction.operands.first(), "STORE_BYTE src")?;
                    let base = as_register(instruction.operands.get(1), "STORE_BYTE base")?;
                    let offset = as_register(instruction.operands.get(2), "STORE_BYTE offset")?;
                    self.emit_reg_get(&mut builder, base)?;
                    self.emit_reg_get(&mut builder, offset)?;
                    builder.emit_opcode(OP_IADD);
                    self.emit_reg_get(&mut builder, src)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_mem_store_byte, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::LoadWord => {
                    let dst = as_register(instruction.operands.first(), "LOAD_WORD dst")?;
                    let base = as_register(instruction.operands.get(1), "LOAD_WORD base")?;
                    let offset = as_register(instruction.operands.get(2), "LOAD_WORD offset")?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_reg_get(&mut builder, base)?;
                    self.emit_reg_get(&mut builder, offset)?;
                    builder.emit_opcode(OP_IADD);
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_load_word, DESC_INT_TO_INT)?,
                    );
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::StoreWord => {
                    let src = as_register(instruction.operands.first(), "STORE_WORD src")?;
                    let base = as_register(instruction.operands.get(1), "STORE_WORD base")?;
                    let offset = as_register(instruction.operands.get(2), "STORE_WORD offset")?;
                    self.emit_reg_get(&mut builder, base)?;
                    self.emit_reg_get(&mut builder, offset)?;
                    builder.emit_opcode(OP_IADD);
                    self.emit_reg_get(&mut builder, src)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_store_word, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::And => {
                    let dst = as_register(instruction.operands.first(), "binary dst")?;
                    let lhs = as_register(instruction.operands.get(1), "binary lhs")?;
                    let rhs = as_register(instruction.operands.get(2), "binary rhs")?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_reg_get(&mut builder, lhs)?;
                    self.emit_reg_get(&mut builder, rhs)?;
                    match instruction.opcode {
                        IrOp::Add => builder.emit_opcode(OP_IADD),
                        IrOp::Sub => builder.emit_opcode(OP_ISUB),
                        // JVM `imul` is signed integer multiply (truncates to int width).
                        IrOp::Mul => builder.emit_opcode(OP_IMUL),
                        // JVM `idiv` truncates toward zero — matches IrOp::Div semantics.
                        IrOp::Div => builder.emit_opcode(OP_IDIV),
                        IrOp::And => builder.emit_opcode(OP_IAND),
                        _ => unreachable!(),
                    }
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::AddImm | IrOp::AndImm => {
                    let dst = as_register(instruction.operands.first(), "binary-imm dst")?;
                    let src = as_register(instruction.operands.get(1), "binary-imm src")?;
                    let imm = as_immediate(instruction.operands.get(2), "binary-imm imm")?;
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_reg_get(&mut builder, src)?;
                    self.emit_push_int(&mut builder, imm)?;
                    match instruction.opcode {
                        IrOp::AddImm => builder.emit_opcode(OP_IADD),
                        IrOp::AndImm => builder.emit_opcode(OP_IAND),
                        _ => unreachable!(),
                    }
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::CmpEq | IrOp::CmpNe | IrOp::CmpLt | IrOp::CmpGt => {
                    let dst = as_register(instruction.operands.first(), "compare dst")?;
                    let lhs = as_register(instruction.operands.get(1), "compare lhs")?;
                    let rhs = as_register(instruction.operands.get(2), "compare rhs")?;
                    let true_label = self.fresh_label("cmp_true");
                    let done_label = self.fresh_label("cmp_done");
                    let branch_opcode = match instruction.opcode {
                        IrOp::CmpEq => OP_IF_ICMPEQ,
                        IrOp::CmpNe => OP_IF_ICMPNE,
                        IrOp::CmpLt => OP_IF_ICMPLT,
                        IrOp::CmpGt => OP_IF_ICMPGT,
                        _ => unreachable!(),
                    };
                    self.emit_push_int(&mut builder, dst as i64)?;
                    self.emit_reg_get(&mut builder, lhs)?;
                    self.emit_reg_get(&mut builder, rhs)?;
                    builder.emit_branch(branch_opcode, true_label.clone());
                    self.emit_push_int(&mut builder, 0)?;
                    builder.emit_branch(OP_GOTO, done_label.clone());
                    builder.mark(true_label);
                    self.emit_push_int(&mut builder, 1)?;
                    builder.mark(done_label);
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::Jump => {
                    builder.emit_branch(
                        OP_GOTO,
                        as_label(instruction.operands.first(), "JUMP target")?,
                    );
                }
                IrOp::BranchZ | IrOp::BranchNz => {
                    let reg = as_register(instruction.operands.first(), "branch reg")?;
                    let label = as_label(instruction.operands.get(1), "branch target")?;
                    self.emit_reg_get(&mut builder, reg)?;
                    builder.emit_branch(
                        if instruction.opcode == IrOp::BranchZ {
                            OP_IFEQ
                        } else {
                            OP_IFNE
                        },
                        label.to_string(),
                    );
                }
                IrOp::Call => {
                    let label = as_label(instruction.operands.first(), "CALL target")?;
                    self.emit_push_int(&mut builder, 1)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(label, DESC_NOARGS_INT)?,
                    );
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_reg_set, DESC_INT_INT_TO_VOID)?,
                    );
                }
                IrOp::Ret | IrOp::Halt => {
                    self.emit_reg_get(&mut builder, 1)?;
                    builder.emit_opcode(OP_IRETURN);
                }
                IrOp::Syscall => {
                    let number = as_immediate(instruction.operands.first(), "SYSCALL number")?;
                    self.emit_push_int(&mut builder, number)?;
                    builder.emit_u2_instruction(
                        OP_INVOKESTATIC,
                        self.method_ref(self.helper_syscall, "(I)V")?,
                    );
                }
            }
        }

        Ok(MethodSpec {
            access_flags: if region.name == self.program.entry_label {
                ACC_PUBLIC | ACC_STATIC
            } else {
                ACC_PRIVATE | ACC_STATIC
            },
            name: region.name.clone(),
            descriptor: DESC_NOARGS_INT.to_string(),
            code: builder.assemble()?,
            max_stack: 16,
            max_locals: 0,
        })
    }

    fn build_main_method(&mut self) -> Result<MethodSpec, JvmBackendError> {
        let mut builder = BytecodeBuilder::default();
        builder.emit_u2_instruction(
            OP_INVOKESTATIC,
            self.method_ref(&self.program.entry_label, DESC_NOARGS_INT)?,
        );
        builder.emit_opcode(OP_POP);
        builder.emit_opcode(OP_RETURN);
        Ok(MethodSpec {
            access_flags: ACC_PUBLIC | ACC_STATIC,
            name: "main".to_string(),
            descriptor: DESC_MAIN.to_string(),
            code: builder.assemble()?,
            max_stack: 1,
            max_locals: 1,
        })
    }

    fn encode_class_file(
        &mut self,
        fields: &[FieldSpec],
        methods: &[MethodSpec],
    ) -> Result<Vec<u8>, JvmBackendError> {
        let this_class_index = self.cp.class_ref(&self.internal_name)?;
        let super_class_index = self.cp.class_ref("java/lang/Object")?;
        let code_name_index = self.cp.utf8("Code")?;

        let mut field_bytes = Vec::new();
        for field in fields {
            append_u2(&mut field_bytes, field.access_flags);
            append_u2(&mut field_bytes, self.cp.utf8(&field.name)?);
            append_u2(&mut field_bytes, self.cp.utf8(&field.descriptor)?);
            append_u2(&mut field_bytes, 0);
        }

        let mut method_bytes = Vec::new();
        for method in methods {
            method_bytes.extend_from_slice(&self.encode_method(method, code_name_index)?);
        }

        let mut bytes = Vec::new();
        append_u4(&mut bytes, 0xCAFEBABE);
        append_u2(&mut bytes, self.config.class_file_minor);
        append_u2(&mut bytes, self.config.class_file_major);
        append_u2(
            &mut bytes,
            u16::try_from(self.cp.count())
                .map_err(|_| JvmBackendError::new("constant pool exceeds u16 count"))?,
        );
        bytes.extend_from_slice(&self.cp.encode());
        append_u2(&mut bytes, ACC_PUBLIC | ACC_SUPER | ACC_FINAL);
        append_u2(&mut bytes, this_class_index);
        append_u2(&mut bytes, super_class_index);
        append_u2(&mut bytes, 0);
        append_u2(
            &mut bytes,
            u16::try_from(fields.len()).map_err(|_| JvmBackendError::new("too many fields"))?,
        );
        bytes.extend_from_slice(&field_bytes);
        append_u2(
            &mut bytes,
            u16::try_from(methods.len()).map_err(|_| JvmBackendError::new("too many methods"))?,
        );
        bytes.extend_from_slice(&method_bytes);
        append_u2(&mut bytes, 0);
        Ok(bytes)
    }

    fn encode_method(
        &mut self,
        method: &MethodSpec,
        code_name_index: u16,
    ) -> Result<Vec<u8>, JvmBackendError> {
        let mut code_attribute_body = Vec::new();
        append_u2(&mut code_attribute_body, method.max_stack);
        append_u2(&mut code_attribute_body, method.max_locals);
        append_u4(
            &mut code_attribute_body,
            u32::try_from(method.code.len())
                .map_err(|_| JvmBackendError::new("method code exceeds 4 GiB"))?,
        );
        code_attribute_body.extend_from_slice(&method.code);
        append_u2(&mut code_attribute_body, 0);
        append_u2(&mut code_attribute_body, 0);

        let mut code_attribute = Vec::new();
        append_u2(&mut code_attribute, code_name_index);
        append_u4(
            &mut code_attribute,
            u32::try_from(code_attribute_body.len())
                .map_err(|_| JvmBackendError::new("Code attribute exceeds 4 GiB"))?,
        );
        code_attribute.extend_from_slice(&code_attribute_body);

        let mut method_bytes = Vec::new();
        append_u2(&mut method_bytes, method.access_flags);
        append_u2(&mut method_bytes, self.cp.utf8(&method.name)?);
        append_u2(&mut method_bytes, self.cp.utf8(&method.descriptor)?);
        append_u2(&mut method_bytes, 1);
        method_bytes.extend_from_slice(&code_attribute);
        Ok(method_bytes)
    }
}

pub fn lower_ir_to_jvm_class_file(
    program: &IrProgram,
    config: JvmBackendConfig,
) -> Result<JvmClassArtifact, JvmBackendError> {
    JvmClassLowerer::new(program, config).lower()
}

pub fn write_class_file(
    artifact: &JvmClassArtifact,
    output_dir: impl AsRef<Path>,
) -> Result<PathBuf, JvmBackendError> {
    validate_class_name(&artifact.class_name)?;
    let root = if output_dir.as_ref().as_os_str().is_empty() {
        Path::new(".")
    } else {
        output_dir.as_ref()
    };
    let relative_path = validated_output_relative_path(&artifact.class_filename())?;
    let (root_fd, absolute_root) = secure_open_root(root)?;
    let mut open_dirs = vec![root_fd];
    for component in relative_path
        .iter()
        .take(relative_path.components().count().saturating_sub(1))
    {
        let parent_fd = open_dirs.last().expect("root fd should exist");
        mkdir_at(parent_fd, Path::new(component)).map_err(io_err("write"))?;
        let next_fd = open_directory_at(parent_fd, Path::new(component)).map_err(|_| {
            JvmBackendError::new(
                "class_filename contains a symlinked or invalid directory component",
            )
        })?;
        open_dirs.push(next_fd);
    }

    let parent_fd = open_dirs.last().expect("root fd should exist");
    let file_fd = open_output_file_at(
        parent_fd,
        relative_path.file_name().expect("filename should exist"),
    )
    .map_err(|_| JvmBackendError::new("class_filename points at a symlinked or invalid output file"))?;
    let mut output: fs::File = file_fd.into();
    output
        .write_all(&artifact.class_bytes)
        .map_err(io_err("write"))?;
    output.flush().map_err(io_err("write"))?;
    let target = absolute_root.join(&relative_path);
    Ok(target)
}

fn io_err(stage: &'static str) -> impl Fn(std::io::Error) -> JvmBackendError {
    move |err| JvmBackendError::new(format!("[{stage}] {err}"))
}

fn secure_open_root(root: &Path) -> Result<(OwnedFd, PathBuf), JvmBackendError> {
    if let Ok(metadata) = fs::symlink_metadata(root) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(JvmBackendError::new(
                "class_filename contains a symlinked or invalid directory component",
            ));
        }
        let canonical_root = fs::canonicalize(root).map_err(io_err("write"))?;
        let root_fd = open_existing_directory(&canonical_root)?;
        return Ok((root_fd, canonical_root));
    }
    let absolute_root = normalize_absolute_path(root)?;
    let mut current_fd = open_root_directory()?;
    let mut components = absolute_root.components();
    let _ = components.next();
    for component in components {
        let std::path::Component::Normal(name) = component else {
            continue;
        };
        mkdir_at(&current_fd, Path::new(name)).map_err(io_err("write"))?;
        current_fd = open_directory_at(&current_fd, Path::new(name)).map_err(|_| {
            JvmBackendError::new("class_filename contains a symlinked or invalid directory component")
        })?;
    }
    Ok((current_fd, absolute_root))
}

fn normalize_absolute_path(path: &Path) -> Result<PathBuf, JvmBackendError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(io_err("write"))?
            .join(path)
    };
    let mut normalized = PathBuf::from("/");
    for component in absolute.components() {
        match component {
            std::path::Component::RootDir => normalized = PathBuf::from("/"),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::Prefix(_) => {
                return Err(JvmBackendError::new("windows-style paths are not supported"));
            }
        }
    }
    Ok(normalized)
}

fn open_root_directory() -> Result<OwnedFd, JvmBackendError> {
    let root = CString::new("/").expect("root path literal must not contain NUL");
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let raw_fd = unsafe { libc::open(root.as_ptr(), flags) };
    if raw_fd < 0 {
        return Err(JvmBackendError::new(format!(
            "[write] {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

fn open_existing_directory(path: &Path) -> Result<OwnedFd, JvmBackendError> {
    let name = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| JvmBackendError::new("[write] path contains NUL"))?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let raw_fd = unsafe { libc::open(name.as_ptr(), flags) };
    if raw_fd < 0 {
        return Err(JvmBackendError::new(format!(
            "[write] {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

fn mkdir_at(parent_fd: &OwnedFd, component: &Path) -> Result<(), std::io::Error> {
    let name = component_to_c_string(component)?;
    let result = unsafe { libc::mkdirat(parent_fd.as_raw_fd(), name.as_ptr(), 0o755) };
    if result == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        return Ok(());
    }
    Err(error)
}

fn open_directory_at(parent_fd: &OwnedFd, component: &Path) -> Result<OwnedFd, std::io::Error> {
    let name = component_to_c_string(component)?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let raw_fd = unsafe { libc::openat(parent_fd.as_raw_fd(), name.as_ptr(), flags, 0) };
    if raw_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

fn open_output_file_at(
    parent_fd: &OwnedFd,
    file_name: &std::ffi::OsStr,
) -> Result<OwnedFd, std::io::Error> {
    let name = CString::new(file_name.as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))?;
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let raw_fd = unsafe { libc::openat(parent_fd.as_raw_fd(), name.as_ptr(), flags, 0o644) };
    if raw_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

fn component_to_c_string(component: &Path) -> Result<CString, std::io::Error> {
    CString::new(component.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL"))
}

fn validated_output_relative_path(class_filename: &str) -> Result<PathBuf, JvmBackendError> {
    let path = PathBuf::from(class_filename);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(JvmBackendError::new(
            "class_filename escapes the requested classpath root",
        ));
    }
    Ok(path)
}

fn validate_class_name(class_name: &str) -> Result<(), JvmBackendError> {
    if class_name.is_empty() {
        return Err(JvmBackendError::new("class_name must not be empty"));
    }
    for segment in class_name.split('.') {
        let mut chars = segment.chars();
        let first = chars.next().ok_or_else(|| {
            JvmBackendError::new(
                "class_name must be a legal Java binary name made of dot-separated identifiers",
            )
        })?;
        if !is_java_identifier_start(first) || !chars.all(is_java_identifier_part) {
            return Err(JvmBackendError::new(
                "class_name must be a legal Java binary name made of dot-separated identifiers",
            ));
        }
    }
    Ok(())
}

fn is_java_identifier_start(c: char) -> bool {
    c == '_' || c == '$' || c.is_ascii_alphabetic()
}

fn is_java_identifier_part(c: char) -> bool {
    is_java_identifier_start(c) || c.is_ascii_digit()
}

fn as_register(operand: Option<&IrOperand>, context: &str) -> Result<usize, JvmBackendError> {
    match operand {
        Some(IrOperand::Register(index)) => Ok(*index),
        Some(other) => Err(JvmBackendError::new(format!(
            "{context} must be a register, got {other:?}"
        ))),
        None => Err(JvmBackendError::new(format!("{context} is missing"))),
    }
}

fn as_label<'a>(operand: Option<&'a IrOperand>, context: &str) -> Result<&'a str, JvmBackendError> {
    match operand {
        Some(IrOperand::Label(name)) => Ok(name.as_str()),
        Some(other) => Err(JvmBackendError::new(format!(
            "{context} must be a label, got {other:?}"
        ))),
        None => Err(JvmBackendError::new(format!("{context} is missing"))),
    }
}

fn as_immediate(operand: Option<&IrOperand>, context: &str) -> Result<i64, JvmBackendError> {
    match operand {
        Some(IrOperand::Immediate(value)) => Ok(*value),
        Some(other) => Err(JvmBackendError::new(format!(
            "{context} must be an immediate, got {other:?}"
        ))),
        None => Err(JvmBackendError::new(format!("{context} is missing"))),
    }
}

fn append_u2(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn append_u4(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

fn append_i4(buffer: &mut Vec<u8>, value: i32) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrDataDecl, IrInstruction, IrOperand};
    use jvm_class_file::parse_class_file;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn lowers_simple_program_to_parseable_class_file() {
        let artifact =
            lower_ir_to_jvm_class_file(&simple_program(), JvmBackendConfig::new("demo.Example"))
                .unwrap();
        let parsed = parse_class_file(&artifact.class_bytes).unwrap();
        assert_eq!(parsed.this_class_name, "demo/Example");
        assert!(parsed.find_method("_start", Some("()I")).is_some());
        assert!(parsed
            .find_method("main", Some("([Ljava/lang/String;)V"))
            .is_some());
        assert!(parsed.find_method("__ca_syscall", Some("(I)V")).is_some());
    }

    #[test]
    fn writes_class_file_using_classpath_layout() {
        let artifact =
            lower_ir_to_jvm_class_file(&simple_program(), JvmBackendConfig::new("demo.Example"))
                .unwrap();
        let output_root = unique_temp_dir("ir-to-jvm-write");
        fs::create_dir_all(&output_root).unwrap();
        let target = write_class_file(&artifact, &output_root).unwrap();
        let canonical_root = fs::canonicalize(&output_root).unwrap();
        assert_eq!(target, canonical_root.join("demo/Example.class"));
        assert_eq!(fs::read(&target).unwrap(), artifact.class_bytes);
        let _ = fs::remove_dir_all(output_root);
    }

    #[test]
    fn rejects_invalid_class_name() {
        let err = lower_ir_to_jvm_class_file(&simple_program(), JvmBackendConfig::new(".Example"))
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("class_name must be a legal Java binary name"));
    }

    #[test]
    fn rejects_large_static_data() {
        let mut program = simple_program();
        program.add_data(IrDataDecl {
            label: "huge".to_string(),
            size: MAX_STATIC_DATA_BYTES + 1,
            init: 1,
        });
        let err =
            lower_ir_to_jvm_class_file(&program, JvmBackendConfig::new("TooMuchData")).unwrap_err();
        assert!(err.to_string().contains("Total static data exceeds"));
    }

    #[test]
    fn brainfuck_program_lowers_to_parseable_class() {
        use brainfuck::parser::parse_brainfuck;
        use brainfuck_ir_compiler::{compile, release_config};

        let ast = parse_brainfuck("+.").unwrap();
        let result = compile(&ast, "test.bf", release_config()).unwrap();
        let artifact =
            lower_ir_to_jvm_class_file(&result.program, JvmBackendConfig::new("BrainfuckProgram"))
                .unwrap();
        let parsed = parse_class_file(&artifact.class_bytes).unwrap();
        assert_eq!(parsed.this_class_name, "BrainfuckProgram");
        assert!(parsed.find_method("_start", Some("()I")).is_some());
    }

    #[test]
    fn nib_program_lowers_to_parseable_class() {
        use coding_adventures_nib_parser::parse_nib;
        use nib_ir_compiler::{compile_nib, release_config};
        use nib_type_checker::check;

        let ast = parse_nib("fn main() { let x: u4 = 7; }").unwrap();
        let typed = check(ast);
        assert!(typed.ok);
        let result = compile_nib(typed.typed_ast, release_config());
        let artifact =
            lower_ir_to_jvm_class_file(&result.program, JvmBackendConfig::new("NibProgram"))
                .unwrap();
        let parsed = parse_class_file(&artifact.class_bytes).unwrap();
        assert!(parsed.find_method("_start", Some("()I")).is_some());
        assert!(parsed
            .find_method("__ca_memLoadByte", Some("(I)I"))
            .is_some());
    }

    #[test]
    fn generated_brainfuck_class_runs_on_graalvm_java_when_available() {
        let graalvm_home = match std::env::var("GRAALVM_HOME") {
            Ok(value) => PathBuf::from(value),
            Err(_) => return,
        };

        use brainfuck::parser::parse_brainfuck;
        use brainfuck_ir_compiler::{compile, release_config};

        let ast = parse_brainfuck(&("+".repeat(65) + ".")).unwrap();
        let result = compile(&ast, "output_a.bf", release_config()).unwrap();
        let artifact =
            lower_ir_to_jvm_class_file(&result.program, JvmBackendConfig::new("BrainfuckA"))
                .unwrap();

        let output_root = unique_temp_dir("brainfuck-a");
        fs::create_dir_all(&output_root).unwrap();
        write_class_file(&artifact, &output_root).unwrap();

        let java_bin = graalvm_home.join("bin/java");
        let output = Command::new(java_bin)
            .arg("-cp")
            .arg(&output_root)
            .arg("BrainfuckA")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"A");
        let _ = fs::remove_dir_all(output_root);
    }

    fn simple_program() -> IrProgram {
        let mut program = IrProgram::new("_start");
        program.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("_start".to_string())],
            -1,
        ));
        program.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(0)],
            0,
        ));
        program.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        program
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nonce}"))
    }
}

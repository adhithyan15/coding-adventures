//! # jvm-class-file
//!
//! This crate is the Rust "under the floorboards" layer for the repository's
//! JVM work. It does two jobs:
//!
//! 1. parse a deliberately small, boring subset of the JVM class-file format
//! 2. build a minimal one-method class file for tests and bootstrap tooling
//!
//! The parser is intentionally conservative. When bytes ask for something we do
//! not understand, we return a format error instead of trying to guess. That is
//! especially important for attacker-controlled lengths and nested attributes.

use std::fmt;

const CONSTANT_UTF8: u8 = 1;
const CONSTANT_INTEGER: u8 = 3;
const CONSTANT_LONG: u8 = 5;
const CONSTANT_DOUBLE: u8 = 6;
const CONSTANT_CLASS: u8 = 7;
const CONSTANT_STRING: u8 = 8;
const CONSTANT_FIELDREF: u8 = 9;
const CONSTANT_METHODREF: u8 = 10;
const CONSTANT_NAME_AND_TYPE: u8 = 12;

pub const ACC_PUBLIC: u16 = 0x0001;
pub const ACC_STATIC: u16 = 0x0008;
pub const ACC_SUPER: u16 = 0x0020;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassFileFormatError {
    message: String,
}

impl ClassFileFormatError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClassFileFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ClassFileFormatError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JvmClassVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JvmConstantPoolEntry {
    Utf8(String),
    Integer(i32),
    Long(i64),
    Double(f64),
    Class {
        name_index: u16,
    },
    String {
        string_index: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    Fieldref {
        class_index: u16,
        name_and_type_index: u16,
    },
    Methodref {
        class_index: u16,
        name_and_type_index: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmFieldReference {
    pub class_name: String,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmMethodReference {
    pub class_name: String,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmAttributeInfo {
    pub name: String,
    pub info: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmCodeAttribute {
    pub name: String,
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub nested_attributes: Vec<JvmAttributeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JvmMethodAttribute {
    Code(JvmCodeAttribute),
    Raw(JvmAttributeInfo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmMethodInfo {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
    pub attributes: Vec<JvmMethodAttribute>,
}

impl JvmMethodInfo {
    pub fn code_attribute(&self) -> Option<&JvmCodeAttribute> {
        self.attributes
            .iter()
            .find_map(|attribute| match attribute {
                JvmMethodAttribute::Code(code) => Some(code),
                JvmMethodAttribute::Raw(_) => None,
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JvmClassFile {
    pub version: JvmClassVersion,
    pub access_flags: u16,
    pub this_class_name: String,
    pub super_class_name: String,
    pub constant_pool: Vec<Option<JvmConstantPoolEntry>>,
    pub methods: Vec<JvmMethodInfo>,
}

impl JvmClassFile {
    pub fn get_utf8(&self, index: u16) -> Result<&str, ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::Utf8(value) => Ok(value.as_str()),
            _ => Err(class_file_error(format!(
                "Constant pool entry {index} is not a UTF-8 string"
            ))),
        }
    }

    pub fn resolve_class_name(&self, index: u16) -> Result<String, ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::Class { name_index } => {
                Ok(self.get_utf8(*name_index)?.to_string())
            }
            _ => Err(class_file_error(format!(
                "Constant pool entry {index} is not a Class entry"
            ))),
        }
    }

    pub fn resolve_name_and_type(
        &self,
        index: u16,
    ) -> Result<(String, String), ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::NameAndType {
                name_index,
                descriptor_index,
            } => Ok((
                self.get_utf8(*name_index)?.to_string(),
                self.get_utf8(*descriptor_index)?.to_string(),
            )),
            _ => Err(class_file_error(format!(
                "Constant pool entry {index} is not a NameAndType entry"
            ))),
        }
    }

    pub fn resolve_constant(&self, index: u16) -> Result<ResolvedConstant, ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::Utf8(value) => Ok(ResolvedConstant::Utf8(value.clone())),
            JvmConstantPoolEntry::Integer(value) => Ok(ResolvedConstant::Integer(*value)),
            JvmConstantPoolEntry::Long(value) => Ok(ResolvedConstant::Long(*value)),
            JvmConstantPoolEntry::Double(value) => Ok(ResolvedConstant::Double(*value)),
            JvmConstantPoolEntry::String { string_index } => Ok(ResolvedConstant::String(
                self.get_utf8(*string_index)?.to_string(),
            )),
            other => Err(class_file_error(format!(
                "Constant pool entry {index} is not a loadable constant: {other:?}"
            ))),
        }
    }

    pub fn resolve_fieldref(&self, index: u16) -> Result<JvmFieldReference, ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::Fieldref {
                class_index,
                name_and_type_index,
            } => {
                let (name, descriptor) = self.resolve_name_and_type(*name_and_type_index)?;
                Ok(JvmFieldReference {
                    class_name: self.resolve_class_name(*class_index)?,
                    name,
                    descriptor,
                })
            }
            _ => Err(class_file_error(format!(
                "Constant pool entry {index} is not a Fieldref entry"
            ))),
        }
    }

    pub fn resolve_methodref(
        &self,
        index: u16,
    ) -> Result<JvmMethodReference, ClassFileFormatError> {
        match self.entry(index)? {
            JvmConstantPoolEntry::Methodref {
                class_index,
                name_and_type_index,
            } => {
                let (name, descriptor) = self.resolve_name_and_type(*name_and_type_index)?;
                Ok(JvmMethodReference {
                    class_name: self.resolve_class_name(*class_index)?,
                    name,
                    descriptor,
                })
            }
            _ => Err(class_file_error(format!(
                "Constant pool entry {index} is not a Methodref entry"
            ))),
        }
    }

    pub fn find_method(&self, name: &str, descriptor: Option<&str>) -> Option<&JvmMethodInfo> {
        self.methods.iter().find(|method| {
            method.name == name
                && descriptor
                    .map(|expected| expected == method.descriptor)
                    .unwrap_or(true)
        })
    }

    fn entry(&self, index: u16) -> Result<&JvmConstantPoolEntry, ClassFileFormatError> {
        if index == 0 || usize::from(index) >= self.constant_pool.len() {
            return Err(class_file_error(format!(
                "Constant pool index {index} is out of range"
            )));
        }
        self.constant_pool[usize::from(index)]
            .as_ref()
            .ok_or_else(|| {
                class_file_error(format!(
                    "Constant pool index {index} points at a reserved wide slot"
                ))
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedConstant {
    Utf8(String),
    Integer(i32),
    Long(i64),
    Double(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMinimalClassFileParams {
    pub class_name: String,
    pub method_name: String,
    pub descriptor: String,
    pub code: Vec<u8>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub constants: Vec<MinimalClassConstant>,
    pub major_version: u16,
    pub minor_version: u16,
    pub class_access_flags: u16,
    pub method_access_flags: u16,
    pub super_class_name: String,
}

impl Default for BuildMinimalClassFileParams {
    fn default() -> Self {
        Self {
            class_name: String::new(),
            method_name: String::new(),
            descriptor: String::new(),
            code: Vec::new(),
            max_stack: 0,
            max_locals: 0,
            constants: Vec::new(),
            major_version: 61,
            minor_version: 0,
            class_access_flags: ACC_PUBLIC | ACC_SUPER,
            method_access_flags: ACC_PUBLIC | ACC_STATIC,
            super_class_name: "java/lang/Object".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MinimalClassConstant {
    Integer(i32),
    String(String),
}

pub fn build_minimal_class_file(
    params: BuildMinimalClassFileParams,
) -> Result<Vec<u8>, ClassFileFormatError> {
    if params.class_name.is_empty() {
        return Err(class_file_error("class name must not be empty"));
    }
    if params.method_name.is_empty() {
        return Err(class_file_error("method name must not be empty"));
    }
    if params.descriptor.is_empty() {
        return Err(class_file_error("descriptor must not be empty"));
    }

    let mut pool = ConstantPoolBuilder::default();
    let this_class_index = pool.class_ref(&params.class_name)?;
    let super_class_index = pool.class_ref(if params.super_class_name.is_empty() {
        "java/lang/Object"
    } else {
        &params.super_class_name
    })?;
    let method_name_index = pool.utf8(&params.method_name)?;
    let descriptor_index = pool.utf8(&params.descriptor)?;
    let code_name_index = pool.utf8("Code")?;

    for constant in &params.constants {
        match constant {
            MinimalClassConstant::Integer(value) => {
                pool.integer(*value)?;
            }
            MinimalClassConstant::String(value) => {
                pool.string(value)?;
            }
        }
    }

    let mut code_attribute_body = Vec::new();
    append_u2(&mut code_attribute_body, params.max_stack);
    append_u2(&mut code_attribute_body, params.max_locals);
    append_u4(
        &mut code_attribute_body,
        u32::try_from(params.code.len())
            .map_err(|_| class_file_error("method code exceeds 4 GiB"))?,
    );
    code_attribute_body.extend_from_slice(&params.code);
    append_u2(&mut code_attribute_body, 0);
    append_u2(&mut code_attribute_body, 0);

    let mut code_attribute = Vec::new();
    append_u2(&mut code_attribute, code_name_index);
    append_u4(
        &mut code_attribute,
        u32::try_from(code_attribute_body.len())
            .map_err(|_| class_file_error("Code attribute exceeds 4 GiB"))?,
    );
    code_attribute.extend_from_slice(&code_attribute_body);

    let mut method_info = Vec::new();
    append_u2(&mut method_info, params.method_access_flags);
    append_u2(&mut method_info, method_name_index);
    append_u2(&mut method_info, descriptor_index);
    append_u2(&mut method_info, 1);
    method_info.extend_from_slice(&code_attribute);

    let mut class_bytes = Vec::new();
    append_u4(&mut class_bytes, 0xCAFEBABE);
    append_u2(&mut class_bytes, params.minor_version);
    append_u2(&mut class_bytes, params.major_version);
    append_u2(
        &mut class_bytes,
        u16::try_from(pool.count())
            .map_err(|_| class_file_error("constant pool exceeds u16 count"))?,
    );
    class_bytes.extend_from_slice(&pool.encode());
    append_u2(&mut class_bytes, params.class_access_flags);
    append_u2(&mut class_bytes, this_class_index);
    append_u2(&mut class_bytes, super_class_index);
    append_u2(&mut class_bytes, 0);
    append_u2(&mut class_bytes, 0);
    append_u2(&mut class_bytes, 1);
    class_bytes.extend_from_slice(&method_info);
    append_u2(&mut class_bytes, 0);
    Ok(class_bytes)
}

pub fn parse_class_file(data: &[u8]) -> Result<JvmClassFile, ClassFileFormatError> {
    let mut reader = ClassReader::new(data);
    let magic = reader.u4()?;
    if magic != 0xCAFEBABE {
        return Err(class_file_error(format!(
            "Invalid class-file magic: expected 0xCAFEBABE, got 0x{magic:08X}"
        )));
    }

    let version = JvmClassVersion {
        minor: reader.u2()?,
        major: reader.u2()?,
    };

    let constant_pool_count = usize::from(reader.u2()?);
    let mut constant_pool = vec![None; constant_pool_count];
    let mut index = 1usize;
    while index < constant_pool_count {
        let tag = reader.u1()?;
        let entry = match tag {
            CONSTANT_UTF8 => {
                let length = usize::from(reader.u2()?);
                let bytes = reader.read(length)?;
                let value = std::str::from_utf8(bytes)
                    .map_err(|err| {
                        class_file_error(format!("Invalid modified UTF-8 payload: {err}"))
                    })?
                    .to_string();
                JvmConstantPoolEntry::Utf8(value)
            }
            CONSTANT_INTEGER => JvmConstantPoolEntry::Integer(reader.i4()?),
            CONSTANT_LONG => {
                let value = reader.i8()?;
                constant_pool[index] = Some(JvmConstantPoolEntry::Long(value));
                index += 2;
                continue;
            }
            CONSTANT_DOUBLE => {
                let value = reader.f8()?;
                constant_pool[index] = Some(JvmConstantPoolEntry::Double(value));
                index += 2;
                continue;
            }
            CONSTANT_CLASS => JvmConstantPoolEntry::Class {
                name_index: reader.u2()?,
            },
            CONSTANT_STRING => JvmConstantPoolEntry::String {
                string_index: reader.u2()?,
            },
            CONSTANT_FIELDREF => JvmConstantPoolEntry::Fieldref {
                class_index: reader.u2()?,
                name_and_type_index: reader.u2()?,
            },
            CONSTANT_METHODREF => JvmConstantPoolEntry::Methodref {
                class_index: reader.u2()?,
                name_and_type_index: reader.u2()?,
            },
            CONSTANT_NAME_AND_TYPE => JvmConstantPoolEntry::NameAndType {
                name_index: reader.u2()?,
                descriptor_index: reader.u2()?,
            },
            other => {
                return Err(class_file_error(format!(
                    "Unsupported constant-pool tag: {other}"
                )))
            }
        };
        constant_pool[index] = Some(entry);
        index += 1;
    }

    let access_flags = reader.u2()?;
    let this_class_index = reader.u2()?;
    let super_class_index = reader.u2()?;
    let interfaces_count = usize::from(reader.u2()?);
    for _ in 0..interfaces_count {
        reader.u2()?;
    }

    let fields_count = usize::from(reader.u2()?);
    for _ in 0..fields_count {
        skip_member(&mut reader)?;
    }

    let methods_count = usize::from(reader.u2()?);
    let mut methods = Vec::with_capacity(methods_count);
    for _ in 0..methods_count {
        methods.push(parse_method(&mut reader, &constant_pool)?);
    }

    let class_attributes_count = usize::from(reader.u2()?);
    for _ in 0..class_attributes_count {
        parse_attribute(&mut reader, &constant_pool, false)?;
    }

    if reader.remaining() != 0 {
        return Err(class_file_error(format!(
            "Trailing bytes after class-file parse: {}",
            reader.remaining()
        )));
    }

    let partial = JvmClassFile {
        version,
        access_flags,
        this_class_name: String::new(),
        super_class_name: String::new(),
        constant_pool,
        methods,
    };
    let this_class_name = partial.resolve_class_name(this_class_index)?;
    let super_class_name = partial.resolve_class_name(super_class_index)?;

    Ok(JvmClassFile {
        this_class_name,
        super_class_name,
        ..partial
    })
}

fn parse_method(
    reader: &mut ClassReader<'_>,
    constant_pool: &[Option<JvmConstantPoolEntry>],
) -> Result<JvmMethodInfo, ClassFileFormatError> {
    let access_flags = reader.u2()?;
    let name = get_utf8(constant_pool, reader.u2()?)?.to_string();
    let descriptor = get_utf8(constant_pool, reader.u2()?)?.to_string();
    let attributes_count = usize::from(reader.u2()?);
    let mut attributes = Vec::with_capacity(attributes_count);
    for _ in 0..attributes_count {
        attributes.push(parse_attribute(reader, constant_pool, true)?);
    }
    Ok(JvmMethodInfo {
        access_flags,
        name,
        descriptor,
        attributes,
    })
}

fn parse_attribute(
    reader: &mut ClassReader<'_>,
    constant_pool: &[Option<JvmConstantPoolEntry>],
    allow_code: bool,
) -> Result<JvmMethodAttribute, ClassFileFormatError> {
    let name = get_utf8(constant_pool, reader.u2()?)?.to_string();
    let attribute_length_u32 = reader.u4()?;
    let attribute_length = usize::try_from(attribute_length_u32)
        .map_err(|_| class_file_error("attribute length does not fit in usize"))?;

    if name == "Code" && allow_code {
        let bytes = reader.read(attribute_length)?;
        let mut nested_reader = ClassReader::new(bytes);
        let max_stack = nested_reader.u2()?;
        let max_locals = nested_reader.u2()?;
        let code_length_u32 = nested_reader.u4()?;
        let code_length = usize::try_from(code_length_u32)
            .map_err(|_| class_file_error("code length does not fit in usize"))?;
        let code = nested_reader.read(code_length)?.to_vec();
        let exception_table_count = usize::from(nested_reader.u2()?);
        for _ in 0..exception_table_count {
            nested_reader.read(8)?;
        }
        let nested_count = usize::from(nested_reader.u2()?);
        let mut nested_attributes = Vec::with_capacity(nested_count);
        for _ in 0..nested_count {
            match parse_attribute(&mut nested_reader, constant_pool, false)? {
                JvmMethodAttribute::Code(_) => {
                    return Err(class_file_error("nested Code attributes are not supported"))
                }
                JvmMethodAttribute::Raw(raw) => nested_attributes.push(raw),
            }
        }
        if nested_reader.remaining() != 0 {
            return Err(class_file_error("trailing bytes inside Code attribute"));
        }
        return Ok(JvmMethodAttribute::Code(JvmCodeAttribute {
            name,
            max_stack,
            max_locals,
            code,
            nested_attributes,
        }));
    }

    Ok(JvmMethodAttribute::Raw(JvmAttributeInfo {
        name,
        info: reader.read(attribute_length)?.to_vec(),
    }))
}

fn skip_member(reader: &mut ClassReader<'_>) -> Result<(), ClassFileFormatError> {
    reader.u2()?;
    reader.u2()?;
    reader.u2()?;
    let attributes_count = usize::from(reader.u2()?);
    for _ in 0..attributes_count {
        let _name_index = reader.u2()?;
        let attribute_length_u32 = reader.u4()?;
        let attribute_length = usize::try_from(attribute_length_u32)
            .map_err(|_| class_file_error("attribute length does not fit in usize"))?;
        reader.read(attribute_length)?;
    }
    Ok(())
}

fn get_utf8(
    constant_pool: &[Option<JvmConstantPoolEntry>],
    index: u16,
) -> Result<&str, ClassFileFormatError> {
    let entry = constant_pool
        .get(usize::from(index))
        .and_then(Option::as_ref)
        .ok_or_else(|| class_file_error(format!("Constant pool entry {index} is out of range")))?;
    match entry {
        JvmConstantPoolEntry::Utf8(value) => Ok(value.as_str()),
        _ => Err(class_file_error(format!(
            "Constant pool entry {index} is not a UTF-8 string"
        ))),
    }
}

fn class_file_error(message: impl Into<String>) -> ClassFileFormatError {
    ClassFileFormatError::new(message)
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

#[derive(Debug, Default)]
struct ConstantPoolBuilder {
    entries: Vec<Vec<u8>>,
    keys: std::collections::HashMap<String, u16>,
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

    fn add(&mut self, key: String, payload: Vec<u8>) -> Result<u16, ClassFileFormatError> {
        if let Some(index) = self.keys.get(&key) {
            return Ok(*index);
        }
        self.entries.push(payload);
        let index = u16::try_from(self.entries.len())
            .map_err(|_| class_file_error("constant pool exceeds u16 count"))?;
        self.keys.insert(key, index);
        Ok(index)
    }

    fn utf8(&mut self, value: &str) -> Result<u16, ClassFileFormatError> {
        let encoded = value.as_bytes();
        let length = u16::try_from(encoded.len()).map_err(|_| {
            class_file_error(format!("UTF-8 constant {value:?} exceeds 65535 bytes"))
        })?;
        let mut payload = vec![CONSTANT_UTF8];
        append_u2(&mut payload, length);
        payload.extend_from_slice(encoded);
        self.add(format!("Utf8:{value}"), payload)
    }

    fn integer(&mut self, value: i32) -> Result<u16, ClassFileFormatError> {
        let mut payload = vec![CONSTANT_INTEGER];
        append_i4(&mut payload, value);
        self.add(format!("Integer:{value}"), payload)
    }

    fn class_ref(&mut self, value: &str) -> Result<u16, ClassFileFormatError> {
        let name_index = self.utf8(value)?;
        let mut payload = vec![CONSTANT_CLASS];
        append_u2(&mut payload, name_index);
        self.add(format!("Class:{value}"), payload)
    }

    fn string(&mut self, value: &str) -> Result<u16, ClassFileFormatError> {
        let string_index = self.utf8(value)?;
        let mut payload = vec![CONSTANT_STRING];
        append_u2(&mut payload, string_index);
        self.add(format!("String:{value}"), payload)
    }
}

struct ClassReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> ClassReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    fn read(&mut self, length: usize) -> Result<&'a [u8], ClassFileFormatError> {
        if length > self.remaining() {
            return Err(class_file_error(format!(
                "Unexpected end of class file: need {length} bytes, have {}",
                self.remaining()
            )));
        }
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| class_file_error("offset overflow while reading class file"))?;
        let bytes = &self.data[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn u1(&mut self) -> Result<u8, ClassFileFormatError> {
        Ok(self.read(1)?[0])
    }

    fn u2(&mut self) -> Result<u16, ClassFileFormatError> {
        let bytes = self.read(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u4(&mut self) -> Result<u32, ClassFileFormatError> {
        let bytes = self.read(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i4(&mut self) -> Result<i32, ClassFileFormatError> {
        let bytes = self.read(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i8(&mut self) -> Result<i64, ClassFileFormatError> {
        let bytes = self.read(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn f8(&mut self) -> Result<f64, ClassFileFormatError> {
        Ok(f64::from_bits(self.i8()? as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_parses_minimal_class_file() {
        let bytes = build_minimal_class_file(BuildMinimalClassFileParams {
            class_name: "demo/Example".to_string(),
            method_name: "main".to_string(),
            descriptor: "([Ljava/lang/String;)V".to_string(),
            code: vec![0xB1],
            max_stack: 0,
            max_locals: 1,
            constants: vec![
                MinimalClassConstant::Integer(7),
                MinimalClassConstant::String("hello".to_string()),
            ],
            ..Default::default()
        })
        .unwrap();

        let parsed = parse_class_file(&bytes).unwrap();
        assert_eq!(parsed.this_class_name, "demo/Example");
        assert_eq!(parsed.super_class_name, "java/lang/Object");
        assert_eq!(parsed.version.major, 61);
        let method = parsed
            .find_method("main", Some("([Ljava/lang/String;)V"))
            .unwrap();
        assert_eq!(method.code_attribute().unwrap().code, vec![0xB1]);
        let constants = parsed
            .constant_pool
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| match entry {
                Some(JvmConstantPoolEntry::Integer(_))
                | Some(JvmConstantPoolEntry::String { .. })
                | Some(JvmConstantPoolEntry::Utf8(_)) => {
                    Some((index as u16, entry.as_ref().unwrap()))
                }
                _ => None,
            })
            .map(|(index, _)| parsed.resolve_constant(index).unwrap())
            .collect::<Vec<_>>();
        assert!(constants.contains(&ResolvedConstant::Integer(7)));
        assert!(constants.contains(&ResolvedConstant::String("hello".to_string())));
    }

    #[test]
    fn rejects_invalid_magic() {
        let err = parse_class_file(&[0, 1, 2, 3]).unwrap_err();
        assert!(err.to_string().contains("Invalid class-file magic"));
    }

    #[test]
    fn resolves_fieldrefs_and_methodrefs() {
        let bytes = synthetic_class_with_member_refs();
        let parsed = parse_class_file(&bytes).unwrap();
        let fieldref_index = parsed
            .constant_pool
            .iter()
            .enumerate()
            .find_map(|(index, entry)| {
                matches!(entry, Some(JvmConstantPoolEntry::Fieldref { .. })).then_some(index as u16)
            })
            .unwrap();
        let methodref_index = parsed
            .constant_pool
            .iter()
            .enumerate()
            .find_map(|(index, entry)| {
                matches!(entry, Some(JvmConstantPoolEntry::Methodref { .. }))
                    .then_some(index as u16)
            })
            .unwrap();
        assert_eq!(
            parsed.resolve_fieldref(fieldref_index).unwrap(),
            JvmFieldReference {
                class_name: "demo/Refs".to_string(),
                name: "VALUE".to_string(),
                descriptor: "I".to_string(),
            }
        );
        assert_eq!(
            parsed.resolve_methodref(methodref_index).unwrap(),
            JvmMethodReference {
                class_name: "demo/Refs".to_string(),
                name: "helper".to_string(),
                descriptor: "()I".to_string(),
            }
        );
    }

    #[test]
    fn rejects_nested_code_attribute_recursion() {
        let bytes = class_with_nested_code_attribute();
        let parsed = parse_class_file(&bytes).unwrap();
        let code = parsed
            .find_method("main", Some("()V"))
            .unwrap()
            .code_attribute()
            .unwrap();
        assert_eq!(code.nested_attributes.len(), 1);
        assert_eq!(code.nested_attributes[0].name, "Code");
    }

    fn synthetic_class_with_member_refs() -> Vec<u8> {
        let mut pool = ConstantPoolBuilder::default();
        let this_class_index = pool.class_ref("demo/Refs").unwrap();
        let super_class_index = pool.class_ref("java/lang/Object").unwrap();
        let method_name_index = pool.utf8("main").unwrap();
        let method_descriptor_index = pool.utf8("()V").unwrap();
        let code_name_index = pool.utf8("Code").unwrap();
        let _field_name = pool.utf8("VALUE").unwrap();
        let _field_desc = pool.utf8("I").unwrap();
        let field_nat = {
            let mut payload = vec![CONSTANT_NAME_AND_TYPE];
            append_u2(&mut payload, pool.utf8("VALUE").unwrap());
            append_u2(&mut payload, pool.utf8("I").unwrap());
            pool.add("Nat:VALUE:I".to_string(), payload).unwrap()
        };
        let field_ref = {
            let mut payload = vec![CONSTANT_FIELDREF];
            append_u2(&mut payload, this_class_index);
            append_u2(&mut payload, field_nat);
            pool.add("Field:demo/Refs:VALUE:I".to_string(), payload).unwrap()
        };
        let method_nat = {
            let mut payload = vec![CONSTANT_NAME_AND_TYPE];
            append_u2(&mut payload, pool.utf8("helper").unwrap());
            append_u2(&mut payload, pool.utf8("()I").unwrap());
            pool.add("Nat:helper:()I".to_string(), payload).unwrap()
        };
        let method_ref = {
            let mut payload = vec![CONSTANT_METHODREF];
            append_u2(&mut payload, this_class_index);
            append_u2(&mut payload, method_nat);
            pool.add("Method:demo/Refs:helper:()I".to_string(), payload).unwrap()
        };

        let mut code_attribute_body = Vec::new();
        append_u2(&mut code_attribute_body, 0);
        append_u2(&mut code_attribute_body, 1);
        append_u4(&mut code_attribute_body, 1);
        code_attribute_body.push(0xB1);
        append_u2(&mut code_attribute_body, 0);
        append_u2(&mut code_attribute_body, 0);

        let mut code_attribute = Vec::new();
        append_u2(&mut code_attribute, code_name_index);
        append_u4(
            &mut code_attribute,
            u32::try_from(code_attribute_body.len()).unwrap(),
        );
        code_attribute.extend_from_slice(&code_attribute_body);

        let mut method_info = Vec::new();
        append_u2(&mut method_info, ACC_PUBLIC | ACC_STATIC);
        append_u2(&mut method_info, method_name_index);
        append_u2(&mut method_info, method_descriptor_index);
        append_u2(&mut method_info, 1);
        method_info.extend_from_slice(&code_attribute);

        let mut bytes = Vec::new();
        append_u4(&mut bytes, 0xCAFEBABE);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 61);
        append_u2(&mut bytes, u16::try_from(pool.count()).unwrap());
        bytes.extend_from_slice(&pool.encode());
        append_u2(&mut bytes, ACC_PUBLIC | ACC_SUPER);
        append_u2(&mut bytes, this_class_index);
        append_u2(&mut bytes, super_class_index);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 1);
        bytes.extend_from_slice(&method_info);
        append_u2(&mut bytes, 0);
        let _ = (field_ref, method_ref);
        bytes
    }

    fn class_with_nested_code_attribute() -> Vec<u8> {
        let mut pool = ConstantPoolBuilder::default();
        let this_class_index = pool.class_ref("demo/Nested").unwrap();
        let super_class_index = pool.class_ref("java/lang/Object").unwrap();
        let method_name_index = pool.utf8("main").unwrap();
        let method_descriptor_index = pool.utf8("()V").unwrap();
        let code_name_index = pool.utf8("Code").unwrap();

        let mut nested_attribute_body = Vec::new();
        append_u2(&mut nested_attribute_body, 0);
        append_u2(&mut nested_attribute_body, 0);
        append_u4(&mut nested_attribute_body, 1);
        nested_attribute_body.push(0xB1);
        append_u2(&mut nested_attribute_body, 0);
        append_u2(&mut nested_attribute_body, 0);

        let mut outer_body = Vec::new();
        append_u2(&mut outer_body, 0);
        append_u2(&mut outer_body, 1);
        append_u4(&mut outer_body, 1);
        outer_body.push(0xB1);
        append_u2(&mut outer_body, 0);
        append_u2(&mut outer_body, 1);
        append_u2(&mut outer_body, code_name_index);
        append_u4(
            &mut outer_body,
            u32::try_from(nested_attribute_body.len()).unwrap(),
        );
        outer_body.extend_from_slice(&nested_attribute_body);

        let mut code_attribute = Vec::new();
        append_u2(&mut code_attribute, code_name_index);
        append_u4(
            &mut code_attribute,
            u32::try_from(outer_body.len()).unwrap(),
        );
        code_attribute.extend_from_slice(&outer_body);

        let mut method_info = Vec::new();
        append_u2(&mut method_info, ACC_PUBLIC | ACC_STATIC);
        append_u2(&mut method_info, method_name_index);
        append_u2(&mut method_info, method_descriptor_index);
        append_u2(&mut method_info, 1);
        method_info.extend_from_slice(&code_attribute);

        let mut bytes = Vec::new();
        append_u4(&mut bytes, 0xCAFEBABE);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 61);
        append_u2(&mut bytes, u16::try_from(pool.count()).unwrap());
        bytes.extend_from_slice(&pool.encode());
        append_u2(&mut bytes, ACC_PUBLIC | ACC_SUPER);
        append_u2(&mut bytes, this_class_index);
        append_u2(&mut bytes, super_class_index);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 0);
        append_u2(&mut bytes, 1);
        bytes.extend_from_slice(&method_info);
        append_u2(&mut bytes, 0);
        bytes
    }
}

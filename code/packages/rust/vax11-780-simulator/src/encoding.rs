//! Small byte-oriented helpers for assembling contract tests and examples.

/// Privileged HALT opcode, used as the kernel-mode simulator sentinel.
pub const HALT: u8 = 0x00;
/// No-operation opcode.
pub const NOP: u8 = 0x01;

/// Concatenate encoded instructions.
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flatten().copied().collect()
}

/// Encode an opcode followed by already encoded operands.
pub fn instruction(opcode: u8, operands: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![opcode];
    bytes.extend(operands.iter().flatten().copied());
    bytes
}

/// Encode MOVL.
pub fn movl(source: Vec<u8>, destination: Vec<u8>) -> Vec<u8> {
    instruction(0xd0, &[source, destination])
}

/// Encode a six-bit short literal.
pub fn literal(value: u8) -> Vec<u8> {
    assert!(value < 64, "VAX short literals are six bits");
    vec![value]
}

/// Encode register mode.
pub fn register(index: u8) -> Vec<u8> {
    assert!(index < 16);
    vec![0x50 | index]
}

/// Encode register-deferred mode.
pub fn register_deferred(index: u8) -> Vec<u8> {
    assert!(index < 16);
    vec![0x60 | index]
}

/// Encode autodecrement mode.
pub fn autodecrement(index: u8) -> Vec<u8> {
    assert!(index < 16);
    vec![0x70 | index]
}

/// Encode autoincrement mode.
pub fn autoincrement(index: u8) -> Vec<u8> {
    assert!(index < 16);
    vec![0x80 | index]
}

/// Encode autoincrement-deferred mode.
pub fn autoincrement_deferred(index: u8) -> Vec<u8> {
    assert!(index < 16);
    vec![0x90 | index]
}

/// Encode a byte-displacement operand.
pub fn byte_displacement(index: u8, displacement: i8) -> Vec<u8> {
    assert!(index < 16);
    vec![0xa0 | index, displacement as u8]
}

/// Encode a word-displacement operand.
pub fn word_displacement(index: u8, displacement: i16) -> Vec<u8> {
    assert!(index < 16);
    let mut bytes = vec![0xc0 | index];
    bytes.extend(displacement.to_le_bytes());
    bytes
}

/// Encode a long-displacement operand.
pub fn long_displacement(index: u8, displacement: i32) -> Vec<u8> {
    assert!(index < 16);
    let mut bytes = vec![0xe0 | index];
    bytes.extend(displacement.to_le_bytes());
    bytes
}

/// Encode indexed mode around a base operand.
pub fn indexed(index: u8, base: Vec<u8>) -> Vec<u8> {
    assert!(index < 15, "PC cannot be an index register");
    let mut bytes = vec![0x40 | index];
    bytes.extend(base);
    bytes
}

/// Encode a PC-immediate longword.
pub fn immediate_long(value: u32) -> Vec<u8> {
    let mut bytes = vec![0x8f];
    bytes.extend(value.to_le_bytes());
    bytes
}

/// Encode PC autoincrement-deferred absolute mode.
pub fn absolute(address: u32) -> Vec<u8> {
    let mut bytes = vec![0x9f];
    bytes.extend(address.to_le_bytes());
    bytes
}

/// Encode an eight-bit PC-relative operand.
pub fn pc_relative_byte(displacement: i8) -> Vec<u8> {
    vec![0xaf, displacement as u8]
}

/// Encode a byte-displacement branch or subroutine branch.
pub fn branch_byte(opcode: u8, displacement: i8) -> Vec<u8> {
    vec![opcode, displacement as u8]
}

/// Encode a word-displacement branch or subroutine branch.
pub fn branch_word(opcode: u8, displacement: i16) -> Vec<u8> {
    let mut bytes = vec![opcode];
    bytes.extend(displacement.to_le_bytes());
    bytes
}

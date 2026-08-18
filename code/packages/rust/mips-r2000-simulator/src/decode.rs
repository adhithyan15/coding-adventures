//! Instruction decoder for all three MIPS R2000 instruction formats
//! (R/I/J).

use crate::opcodes::*;
use std::collections::HashMap;

/// Decoded instruction with mnemonic and extracted fields.
///
/// `fields["imm"]` is always the **sign-extended** 16-bit immediate (as
/// `i32`) for I-type instructions.  Instructions that need the raw
/// zero-extended bits (`ANDI`/`ORI`/`XORI`/`LUI`) recover them in
/// `execute.rs` via `(imm as u32) & 0xFFFF` — masking a sign-extended value
/// back to 16 bits always recovers the original bit pattern.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub mnemonic: String,
    pub fields: HashMap<String, i32>,
    pub raw: u32,
}

fn sign_extend16(v: i32) -> i32 {
    (v << 16) >> 16
}

pub fn decode(raw: u32, _pc: i32) -> DecodeResult {
    let op = (raw >> 26) & 0x3F;
    let rs = (raw >> 21) & 0x1F;
    let rt = (raw >> 16) & 0x1F;
    let rd = (raw >> 11) & 0x1F;
    let shamt = (raw >> 6) & 0x1F;
    let funct = raw & 0x3F;
    let imm16 = raw & 0xFFFF;
    let simm = sign_extend16(imm16 as i32);
    let target = raw & 0x03FF_FFFF;

    // SYSCALL is our universal HALT sentinel — op=0, funct=0x0C,
    // irrespective of the other fields (matches the Python original).
    if op == OP_RTYPE && funct == FUNCT_SYSCALL {
        return DecodeResult {
            mnemonic: "syscall".into(),
            fields: HashMap::new(),
            raw,
        };
    }

    match op {
        OP_RTYPE => decode_r_type(raw, rs, rt, rd, shamt, funct),
        OP_REGIMM => decode_regimm(rs, rt, simm, raw),
        OP_J => DecodeResult {
            mnemonic: "j".into(),
            fields: HashMap::from([("target".into(), target as i32)]),
            raw,
        },
        OP_JAL => DecodeResult {
            mnemonic: "jal".into(),
            fields: HashMap::from([("target".into(), target as i32)]),
            raw,
        },
        OP_BEQ => branch_fields("beq", rs, rt, simm, raw),
        OP_BNE => branch_fields("bne", rs, rt, simm, raw),
        OP_BLEZ => branch_fields("blez", rs, 0, simm, raw),
        OP_BGTZ => branch_fields("bgtz", rs, 0, simm, raw),
        OP_ADDI => imm_fields("addi", rs, rt, simm, raw),
        OP_ADDIU => imm_fields("addiu", rs, rt, simm, raw),
        OP_SLTI => imm_fields("slti", rs, rt, simm, raw),
        OP_SLTIU => imm_fields("sltiu", rs, rt, simm, raw),
        OP_ANDI => imm_fields("andi", rs, rt, simm, raw),
        OP_ORI => imm_fields("ori", rs, rt, simm, raw),
        OP_XORI => imm_fields("xori", rs, rt, simm, raw),
        OP_LUI => DecodeResult {
            mnemonic: "lui".into(),
            fields: HashMap::from([("rt".into(), rt as i32), ("imm".into(), simm)]),
            raw,
        },
        OP_LB => imm_fields("lb", rs, rt, simm, raw),
        OP_LH => imm_fields("lh", rs, rt, simm, raw),
        OP_LW => imm_fields("lw", rs, rt, simm, raw),
        OP_LBU => imm_fields("lbu", rs, rt, simm, raw),
        OP_LHU => imm_fields("lhu", rs, rt, simm, raw),
        OP_SB => imm_fields("sb", rs, rt, simm, raw),
        OP_SH => imm_fields("sh", rs, rt, simm, raw),
        OP_SW => imm_fields("sw", rs, rt, simm, raw),
        _ => DecodeResult {
            mnemonic: format!("UNKNOWN(op=0x{op:02x})"),
            fields: HashMap::from([("op".into(), op as i32)]),
            raw,
        },
    }
}

fn imm_fields(mnemonic: &str, rs: u32, rt: u32, simm: i32, raw: u32) -> DecodeResult {
    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rs".into(), rs as i32),
            ("rt".into(), rt as i32),
            ("imm".into(), simm),
        ]),
        raw,
    }
}

fn branch_fields(mnemonic: &str, rs: u32, rt: u32, simm: i32, raw: u32) -> DecodeResult {
    imm_fields(mnemonic, rs, rt, simm, raw)
}

fn decode_r_type(raw: u32, rs: u32, rt: u32, rd: u32, shamt: u32, funct: u32) -> DecodeResult {
    let mnemonic = match funct {
        FUNCT_SLL => "sll",
        FUNCT_SRL => "srl",
        FUNCT_SRA => "sra",
        FUNCT_SLLV => "sllv",
        FUNCT_SRLV => "srlv",
        FUNCT_SRAV => "srav",
        FUNCT_JR => "jr",
        FUNCT_JALR => "jalr",
        FUNCT_BREAK => "break",
        FUNCT_MFHI => "mfhi",
        FUNCT_MTHI => "mthi",
        FUNCT_MFLO => "mflo",
        FUNCT_MTLO => "mtlo",
        FUNCT_MULT => "mult",
        FUNCT_MULTU => "multu",
        FUNCT_DIV => "div",
        FUNCT_DIVU => "divu",
        FUNCT_ADD => "add",
        FUNCT_ADDU => "addu",
        FUNCT_SUB => "sub",
        FUNCT_SUBU => "subu",
        FUNCT_AND => "and",
        FUNCT_OR => "or",
        FUNCT_XOR => "xor",
        FUNCT_NOR => "nor",
        FUNCT_SLT => "slt",
        FUNCT_SLTU => "sltu",
        _ => "r_unknown",
    };
    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rs".into(), rs as i32),
            ("rt".into(), rt as i32),
            ("rd".into(), rd as i32),
            ("shamt".into(), shamt as i32),
            ("funct".into(), funct as i32),
        ]),
        raw,
    }
}

fn decode_regimm(rs: u32, rt: u32, simm: i32, raw: u32) -> DecodeResult {
    let mnemonic = match rt {
        REGIMM_BLTZ => "bltz",
        REGIMM_BGEZ => "bgez",
        REGIMM_BLTZAL => "bltzal",
        REGIMM_BGEZAL => "bgezal",
        _ => "regimm_unknown",
    };
    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([("rs".into(), rs as i32), ("imm".into(), simm)]),
        raw,
    }
}

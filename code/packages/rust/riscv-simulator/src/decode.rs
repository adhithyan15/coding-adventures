//! Instruction decoder for all RV32I formats.

use std::collections::HashMap;
use crate::opcodes::*;

/// Decoded instruction with mnemonic and extracted fields.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub mnemonic: String,
    pub fields: HashMap<String, i32>,
    pub raw: u32,
}

pub fn decode(raw: u32, _pc: i32) -> DecodeResult {
    let opcode = raw & 0x7F;
    match opcode {
        OPCODE_OP_IMM => decode_op_imm(raw),
        OPCODE_OP => decode_r_type(raw),
        OPCODE_LOAD => decode_load(raw),
        OPCODE_STORE => decode_s_type(raw),
        OPCODE_BRANCH => decode_b_type(raw),
        OPCODE_JAL => decode_j_type(raw),
        OPCODE_JALR => decode_jalr(raw),
        OPCODE_LUI => decode_u_type(raw, "lui"),
        OPCODE_AUIPC => decode_u_type(raw, "auipc"),
        OPCODE_SYSTEM => decode_system(raw),
        _ => DecodeResult {
            mnemonic: format!("UNKNOWN(0x{:02x})", opcode),
            fields: HashMap::from([("opcode".into(), opcode as i32)]),
            raw,
        },
    }
}

fn sign_extend(val: i32, bits: u32) -> i32 {
    let shift = 32 - bits;
    (val << shift) >> shift
}

fn decode_op_imm(raw: u32) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let funct3 = ((raw >> 12) & 0x7) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let mut imm = ((raw >> 20) & 0xFFF) as i32;
    imm = sign_extend(imm, 12);

    let mnemonic = match funct3 as u32 {
        FUNCT3_ADDI => "addi",
        FUNCT3_SLTI => "slti",
        FUNCT3_SLTIU => "sltiu",
        FUNCT3_XORI => "xori",
        FUNCT3_ORI => "ori",
        FUNCT3_ANDI => "andi",
        FUNCT3_SLLI => { imm &= 0x1F; "slli" }
        FUNCT3_SRLI => {
            let funct7 = (raw >> 25) & 0x7F;
            imm &= 0x1F;
            if funct7 == FUNCT7_ALT { "srai" } else { "srli" }
        }
        _ => "opimm_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rd".into(), rd), ("rs1".into(), rs1),
            ("imm".into(), imm), ("funct3".into(), funct3),
        ]),
        raw,
    }
}

fn decode_r_type(raw: u32) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let funct3 = ((raw >> 12) & 0x7) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let rs2 = ((raw >> 20) & 0x1F) as i32;
    let funct7 = ((raw >> 25) & 0x7F) as i32;

    let mnemonic = match (funct3 as u32, funct7 as u32) {
        (FUNCT3_ADD, FUNCT7_NORMAL) => "add",
        (FUNCT3_ADD, FUNCT7_ALT) => "sub",
        (FUNCT3_SLL, _) => "sll",
        (FUNCT3_SLT, _) => "slt",
        (FUNCT3_SLTU, _) => "sltu",
        (FUNCT3_XOR, _) => "xor",
        (FUNCT3_SRL, FUNCT7_NORMAL) => "srl",
        (FUNCT3_SRL, FUNCT7_ALT) => "sra",
        (FUNCT3_OR, _) => "or",
        (FUNCT3_AND, _) => "and",
        _ => "r_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rd".into(), rd), ("rs1".into(), rs1), ("rs2".into(), rs2),
            ("funct3".into(), funct3), ("funct7".into(), funct7),
        ]),
        raw,
    }
}

fn decode_load(raw: u32) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let funct3 = ((raw >> 12) & 0x7) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let imm = sign_extend(((raw >> 20) & 0xFFF) as i32, 12);

    let mnemonic = match funct3 as u32 {
        FUNCT3_LB => "lb", FUNCT3_LH => "lh", FUNCT3_LW => "lw",
        FUNCT3_LBU => "lbu", FUNCT3_LHU => "lhu",
        _ => "load_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rd".into(), rd), ("rs1".into(), rs1),
            ("imm".into(), imm), ("funct3".into(), funct3),
        ]),
        raw,
    }
}

fn decode_s_type(raw: u32) -> DecodeResult {
    let funct3 = ((raw >> 12) & 0x7) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let rs2 = ((raw >> 20) & 0x1F) as i32;
    let imm_low = ((raw >> 7) & 0x1F) as i32;
    let imm_high = ((raw >> 25) & 0x7F) as i32;
    let imm = sign_extend((imm_high << 5) | imm_low, 12);

    let mnemonic = match funct3 as u32 {
        FUNCT3_SB => "sb", FUNCT3_SH => "sh", FUNCT3_SW => "sw",
        _ => "store_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rs1".into(), rs1), ("rs2".into(), rs2),
            ("imm".into(), imm), ("funct3".into(), funct3),
        ]),
        raw,
    }
}

fn decode_b_type(raw: u32) -> DecodeResult {
    let funct3 = ((raw >> 12) & 0x7) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let rs2 = ((raw >> 20) & 0x1F) as i32;
    let imm12 = ((raw >> 31) & 0x1) as i32;
    let imm11 = ((raw >> 7) & 0x1) as i32;
    let imm10_5 = ((raw >> 25) & 0x3F) as i32;
    let imm4_1 = ((raw >> 8) & 0xF) as i32;
    let imm = sign_extend(
        (imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1),
        13,
    );

    let mnemonic = match funct3 as u32 {
        FUNCT3_BEQ => "beq", FUNCT3_BNE => "bne",
        FUNCT3_BLT => "blt", FUNCT3_BGE => "bge",
        FUNCT3_BLTU => "bltu", FUNCT3_BGEU => "bgeu",
        _ => "branch_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rs1".into(), rs1), ("rs2".into(), rs2),
            ("imm".into(), imm), ("funct3".into(), funct3),
        ]),
        raw,
    }
}

fn decode_j_type(raw: u32) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let imm20 = ((raw >> 31) & 0x1) as i32;
    let imm10_1 = ((raw >> 21) & 0x3FF) as i32;
    let imm11 = ((raw >> 20) & 0x1) as i32;
    let imm19_12 = ((raw >> 12) & 0xFF) as i32;
    let imm = sign_extend(
        (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1),
        21,
    );

    DecodeResult {
        mnemonic: "jal".into(),
        fields: HashMap::from([("rd".into(), rd), ("imm".into(), imm)]),
        raw,
    }
}

fn decode_jalr(raw: u32) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let imm = sign_extend(((raw >> 20) & 0xFFF) as i32, 12);

    DecodeResult {
        mnemonic: "jalr".into(),
        fields: HashMap::from([
            ("rd".into(), rd), ("rs1".into(), rs1), ("imm".into(), imm),
        ]),
        raw,
    }
}

fn decode_u_type(raw: u32, mnemonic: &str) -> DecodeResult {
    let rd = ((raw >> 7) & 0x1F) as i32;
    let imm = sign_extend((raw >> 12) as i32, 20);

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([("rd".into(), rd), ("imm".into(), imm)]),
        raw,
    }
}

fn decode_system(raw: u32) -> DecodeResult {
    let funct3 = (raw >> 12) & 0x7;

    if funct3 == FUNCT3_PRIV {
        let funct7 = (raw >> 25) & 0x7F;
        if funct7 == FUNCT7_MRET {
            return DecodeResult {
                mnemonic: "mret".into(),
                fields: HashMap::from([("funct7".into(), funct7 as i32)]),
                raw,
            };
        }
        return DecodeResult {
            mnemonic: "ecall".into(),
            fields: HashMap::from([("funct7".into(), funct7 as i32)]),
            raw,
        };
    }

    let rd = ((raw >> 7) & 0x1F) as i32;
    let rs1 = ((raw >> 15) & 0x1F) as i32;
    let csr = ((raw >> 20) & 0xFFF) as i32;

    let mnemonic = match funct3 {
        FUNCT3_CSRRW => "csrrw",
        FUNCT3_CSRRS => "csrrs",
        FUNCT3_CSRRC => "csrrc",
        _ => "system_unknown",
    };

    DecodeResult {
        mnemonic: mnemonic.into(),
        fields: HashMap::from([
            ("rd".into(), rd), ("rs1".into(), rs1),
            ("csr".into(), csr), ("funct3".into(), funct3 as i32),
        ]),
        raw,
    }
}

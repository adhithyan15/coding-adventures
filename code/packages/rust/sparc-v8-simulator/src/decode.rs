//! Instruction decoder for all four SPARC V8 instruction shapes
//! (Format 1 `CALL`, Format 2 `SETHI`/`Bicc`/`NOP`, Format 3r/3i `ALU`,
//! Format 3r/3i memory).

use std::collections::HashMap;

use crate::opcodes::*;

/// Decoded instruction with mnemonic and extracted fields.
///
/// `fields["i"]` is `1` when the instruction carries a sign-extended
/// 13-bit immediate operand (`fields["simm13"]`) and `0` when it carries
/// a register operand (`fields["rs2"]`) — mirrors the `i` bit (bit 13)
/// SPARC V8 Format 3 instructions use to select between the two shapes.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub mnemonic: String,
    pub fields: HashMap<String, i32>,
    pub raw: u32,
}

fn sext13(v: u32) -> i32 {
    let v = v & 0x1FFF;
    if v >= 0x1000 {
        (v as i32) - 0x2000
    } else {
        v as i32
    }
}

fn sext22(v: u32) -> i32 {
    let v = v & 0x3F_FFFF;
    if v >= 0x20_0000 {
        (v as i32) - 0x40_0000
    } else {
        v as i32
    }
}

fn sext30(v: u32) -> i32 {
    let v = v & 0x3FFF_FFFF;
    if v >= 0x2000_0000 {
        (v as i32) - 0x4000_0000
    } else {
        v as i32
    }
}

/// Decode one 32-bit SPARC V8 instruction word.
///
/// `ta 0` (== [`HALT_WORD`]) is intercepted before the general Format-3
/// dispatch and always decodes to mnemonic `"ta"` with no fields — this
/// mirrors the Python original's whole-word HALT check ahead of its
/// field-by-field `op`/`op3` dispatch.
pub fn decode(iw: u32) -> DecodeResult {
    if iw == HALT_WORD {
        return DecodeResult {
            mnemonic: "ta".into(),
            fields: HashMap::new(),
            raw: iw,
        };
    }

    let op = (iw >> 30) & 0x3;
    match op {
        OP_CALL => {
            let disp30 = sext30(iw & 0x3FFF_FFFF);
            DecodeResult {
                mnemonic: "call".into(),
                fields: HashMap::from([("disp30".into(), disp30)]),
                raw: iw,
            }
        }
        OP_FMT2 => decode_fmt2(iw),
        OP_ALU => decode_alu(iw),
        OP_MEM => decode_mem(iw),
        _ => unreachable!("op is masked to 2 bits"),
    }
}

fn decode_fmt2(iw: u32) -> DecodeResult {
    if iw == NOP_WORD {
        return DecodeResult {
            mnemonic: "nop".into(),
            fields: HashMap::new(),
            raw: iw,
        };
    }

    let rd = (iw >> 25) & 0x1F;
    let op2 = (iw >> 22) & 0x7;
    let imm22 = iw & 0x3F_FFFF;

    match op2 {
        OP2_SETHI => DecodeResult {
            mnemonic: "sethi".into(),
            fields: HashMap::from([("rd".into(), rd as i32), ("imm22".into(), imm22 as i32)]),
            raw: iw,
        },
        OP2_BICC => {
            let cond = (iw >> 25) & 0xF;
            let disp22 = sext22(imm22);
            DecodeResult {
                mnemonic: bicc_name(cond).to_string(),
                fields: HashMap::from([("cond".into(), cond as i32), ("disp22".into(), disp22)]),
                raw: iw,
            }
        }
        _ => DecodeResult {
            mnemonic: format!("UNKNOWN_FMT2(op2=0x{op2:X})"),
            fields: HashMap::from([("op2".into(), op2 as i32)]),
            raw: iw,
        },
    }
}

fn bicc_name(cond: u32) -> &'static str {
    match cond & 0xF {
        COND_BA => "ba",
        COND_BN => "bn",
        COND_BNE => "bne",
        COND_BE => "be",
        COND_BG => "bg",
        COND_BLE => "ble",
        COND_BGE => "bge",
        COND_BL => "bl",
        COND_BGU => "bgu",
        COND_BLEU => "bleu",
        COND_BCC => "bcc",
        COND_BCS => "bcs",
        COND_BPOS => "bpos",
        COND_BNEG => "bneg",
        COND_BVC => "bvc",
        COND_BVS => "bvs",
        _ => unreachable!("cond is masked to 4 bits"),
    }
}

/// Shared Format-3 field extraction (`rd`, `op3`, `rs1`, and either
/// `rs2` or `simm13` depending on the `i` bit) used by both the ALU
/// (`op == OP_ALU`) and memory (`op == OP_MEM`) dispatchers.
fn decode_format3_fields(iw: u32) -> (u32, u32, u32, HashMap<String, i32>) {
    let rd = (iw >> 25) & 0x1F;
    let op3 = (iw >> 19) & 0x3F;
    let rs1 = (iw >> 14) & 0x1F;
    let i = (iw >> 13) & 1;

    let mut fields = HashMap::from([
        ("rd".into(), rd as i32),
        ("rs1".into(), rs1 as i32),
        ("i".into(), i as i32),
    ]);
    if i == 1 {
        fields.insert("simm13".into(), sext13(iw & 0x1FFF));
    } else {
        fields.insert("rs2".into(), (iw & 0x1F) as i32);
    }
    (rd, op3, rs1, fields)
}

fn decode_alu(iw: u32) -> DecodeResult {
    let (rd, op3, _rs1, mut fields) = decode_format3_fields(iw);

    // Ticc: the `rd` field doubles as the 4-bit trap condition.
    if op3 == OP3_TICC {
        fields.insert("cond".into(), (rd & 0xF) as i32);
        return DecodeResult {
            mnemonic: "ticc".into(),
            fields,
            raw: iw,
        };
    }

    DecodeResult {
        mnemonic: alu_mnemonic(op3).to_string(),
        fields,
        raw: iw,
    }
}

fn alu_mnemonic(op3: u32) -> &'static str {
    match op3 {
        OP3_ADD => "add",
        OP3_ADDCC => "addcc",
        OP3_ADDX => "addx",
        OP3_ADDXCC => "addxcc",
        OP3_SUB => "sub",
        OP3_SUBCC => "subcc",
        OP3_SUBX => "subx",
        OP3_SUBXCC => "subxcc",
        OP3_AND => "and",
        OP3_ANDCC => "andcc",
        OP3_ANDN => "andn",
        OP3_ANDNCC => "andncc",
        OP3_OR => "or",
        OP3_ORCC => "orcc",
        OP3_ORN => "orn",
        OP3_ORNCC => "orncc",
        OP3_XOR => "xor",
        OP3_XORCC => "xorcc",
        OP3_XNOR => "xnor",
        OP3_XNORCC => "xnorcc",
        OP3_UMUL => "umul",
        OP3_UMULCC => "umulcc",
        OP3_SMUL => "smul",
        OP3_SMULCC => "smulcc",
        OP3_UDIV => "udiv",
        OP3_UDIVCC => "udivcc",
        OP3_SDIV => "sdiv",
        OP3_SDIVCC => "sdivcc",
        OP3_MULSCC => "mulscc",
        OP3_SLL => "sll",
        OP3_SRL => "srl",
        OP3_SRA => "sra",
        OP3_RDY => "rdy",
        OP3_WRY => "wry",
        OP3_JMPL => "jmpl",
        OP3_SAVE => "save",
        OP3_RESTORE => "restore",
        _ => "alu_unknown",
    }
}

fn decode_mem(iw: u32) -> DecodeResult {
    let (_rd, op3, _rs1, fields) = decode_format3_fields(iw);
    DecodeResult {
        mnemonic: mem_mnemonic(op3).to_string(),
        fields,
        raw: iw,
    }
}

fn mem_mnemonic(op3: u32) -> &'static str {
    match op3 {
        OP3_LD => "ld",
        OP3_LDUB => "ldub",
        OP3_LDUH => "lduh",
        OP3_LDSB => "ldsb",
        OP3_LDSH => "ldsh",
        OP3_ST => "st",
        OP3_STB => "stb",
        OP3_STH => "sth",
        _ => "mem_unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::*;

    #[test]
    fn decode_halt() {
        assert_eq!(decode(HALT_WORD).mnemonic, "ta");
    }

    #[test]
    fn decode_nop() {
        assert_eq!(decode(NOP_WORD).mnemonic, "nop");
    }

    #[test]
    fn decode_sethi() {
        let d = decode(encode_sethi(1, 0x12345));
        assert_eq!(d.mnemonic, "sethi");
        assert_eq!(d.fields["rd"], 1);
        assert_eq!(d.fields["imm22"], 0x12345);
    }

    #[test]
    fn decode_add_imm() {
        let d = decode(encode_add_imm(8, 0, 42));
        assert_eq!(d.mnemonic, "add");
        assert_eq!(d.fields["rd"], 8);
        assert_eq!(d.fields["rs1"], 0);
        assert_eq!(d.fields["i"], 1);
        assert_eq!(d.fields["simm13"], 42);
    }

    #[test]
    fn decode_add_negative_imm() {
        let d = decode(encode_add_imm(8, 0, -1));
        assert_eq!(d.fields["simm13"], -1);
    }

    #[test]
    fn decode_add_reg() {
        let d = decode(encode_add(1, 2, 3));
        assert_eq!(d.mnemonic, "add");
        assert_eq!(d.fields["i"], 0);
        assert_eq!(d.fields["rs2"], 3);
    }

    #[test]
    fn decode_bicc() {
        let d = decode(encode_ba(8));
        assert_eq!(d.mnemonic, "ba");
        assert_eq!(d.fields["disp22"], 8);
    }

    #[test]
    fn decode_call() {
        let d = decode(encode_call(100));
        assert_eq!(d.mnemonic, "call");
        assert_eq!(d.fields["disp30"], 100);
    }

    #[test]
    fn decode_ld_st() {
        assert_eq!(decode(encode_ld(1, 2, 4)).mnemonic, "ld");
        assert_eq!(decode(encode_st(1, 2, 4)).mnemonic, "st");
    }

    #[test]
    fn decode_ticc_non_halt() {
        let d = decode(encode_ticc(COND_BNE, 0, 1));
        assert_eq!(d.mnemonic, "ticc");
        assert_eq!(d.fields["cond"], COND_BNE as i32);
    }

    #[test]
    fn decode_round_trip_mnemonics() {
        let cases: Vec<(&str, u32)> = vec![
            ("add", encode_add(1, 2, 3)),
            ("addcc", encode_addcc(1, 2, 3)),
            ("sub", encode_sub(1, 2, 3)),
            ("subcc", encode_subcc(1, 2, 3)),
            ("and", encode_and(1, 2, 3)),
            ("or", encode_or(1, 2, 3)),
            ("xor", encode_xor(1, 2, 3)),
            ("andn", encode_andn(1, 2, 3)),
            ("orn", encode_orn(1, 2, 3)),
            ("xnor", encode_xnor(1, 2, 3)),
            ("sll", encode_sll(1, 2, 4)),
            ("srl", encode_srl(1, 2, 4)),
            ("sra", encode_sra(1, 2, 4)),
            ("umul", encode_umul(1, 2, 3)),
            ("smul", encode_smul(1, 2, 3)),
            ("udiv", encode_udiv(1, 2, 3)),
            ("sdiv", encode_sdiv(1, 2, 3)),
            ("rdy", encode_rdy(1)),
            ("wry", encode_wry(1, 2)),
            ("jmpl", encode_jmpl(1, 2, 0)),
            ("save", encode_save(1, 2, 0)),
            ("restore", encode_restore(1, 2, 0)),
            ("ld", encode_ld(1, 2, 4)),
            ("ldub", encode_ldub(1, 2, 4)),
            ("lduh", encode_lduh(1, 2, 4)),
            ("ldsb", encode_ldsb(1, 2, 4)),
            ("ldsh", encode_ldsh(1, 2, 4)),
            ("st", encode_st(1, 2, 4)),
            ("stb", encode_stb(1, 2, 4)),
            ("sth", encode_sth(1, 2, 4)),
        ];
        for (name, encoded) in &cases {
            let result = decode(*encoded);
            assert_eq!(result.mnemonic, *name, "decode(0x{encoded:08x}) failed");
        }
    }
}

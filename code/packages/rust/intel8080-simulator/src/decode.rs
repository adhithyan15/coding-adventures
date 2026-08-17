//! Instruction decoder for the Intel 8080 ISA.
//!
//! Unlike MIPS R2000 (always 32-bit words) the 8080 is variable-length: 1,
//! 2, or 3 bytes depending on the opcode.  `decode` takes the already-
//! fetched opcode byte plus a `fetch` closure the caller uses to pull any
//! remaining operand bytes from memory (advancing PC as it goes) — this
//! keeps `decode.rs` free of any `Memory`/`Registers` dependency, mirroring
//! how `mips_r2000_simulator::decode` only extracts bit fields and leaves
//! all state mutation to `execute.rs`.
//!
//! The dispatch structure (group-00/01/10/11, then the specific bit-field
//! checks within each group) is a direct transliteration of
//! `intel8080_simulator.simulator.Intel8080Simulator._execute_one` /
//! `_exec_group00` / `_exec_group11` from the Python original — the check
//! **order** matters (e.g. `RET`/`JMP`/`CALL`'s fixed bytes must be tested
//! after the general conditional-return/pop/jump/call/push patterns, since
//! their bit patterns partially overlap), so this module keeps the same
//! sequence of `if`/`else` branches rather than re-deriving it.

use crate::opcodes::*;
use std::collections::HashMap;

/// Decoded instruction: mnemonic + extracted operand fields.
///
/// `fields` keys used across mnemonics: `"dst"`, `"src"`, `"reg"`, `"pair"`,
/// `"imm"`, `"addr"`, `"op"` (ALU op code), `"cond"`, `"n"` (RST vector),
/// `"port"`.  Not every mnemonic populates every key — see the dispatch
/// arms below for which fields a given mnemonic sets.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub mnemonic: String,
    pub fields: HashMap<String, i32>,
    /// Raw bytes of this instruction (opcode + any operand bytes),
    /// preserved for debugging/disassembly.
    pub raw: Vec<u8>,
}

fn result(mnemonic: &str, fields: HashMap<String, i32>, raw: Vec<u8>) -> DecodeResult {
    DecodeResult {
        mnemonic: mnemonic.to_string(),
        fields,
        raw,
    }
}

fn fetch_word(raw: &mut Vec<u8>, fetch: &mut dyn FnMut() -> u8) -> u16 {
    let lo = fetch();
    let hi = fetch();
    raw.push(lo);
    raw.push(hi);
    ((hi as u16) << 8) | lo as u16
}

fn fetch_one(raw: &mut Vec<u8>, fetch: &mut dyn FnMut() -> u8) -> u8 {
    let b = fetch();
    raw.push(b);
    b
}

/// Decode one instruction.  `opcode` is the byte already fetched at the
/// current PC; `fetch` pulls any further operand bytes (and must advance
/// the caller's PC as a side effect — see `simulator::Intel8080Simulator::step`).
pub fn decode(opcode: u8, fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    let mut raw = vec![opcode];
    let bits_76 = (opcode >> 6) & 0x03; // group
    let bits_53 = (opcode >> 3) & 0x07; // dst / sub-op
    let bits_20 = opcode & 0x07; // src / sub-op

    // ── GROUP 01: MOV r1, r2 (0x40-0x7F); 0x76 is HLT, not MOV M,M ──
    if bits_76 == 0b01 {
        if opcode == HLT {
            return result("hlt", HashMap::new(), raw);
        }
        let fields = HashMap::from([
            ("dst".to_string(), bits_53 as i32),
            ("src".to_string(), bits_20 as i32),
        ]);
        return result("mov", fields, raw);
    }

    // ── GROUP 10: ALU register-source (0x80-0xBF) ──
    if bits_76 == 0b10 {
        let fields = HashMap::from([
            ("op".to_string(), bits_53 as i32),
            ("src".to_string(), bits_20 as i32),
        ]);
        return result("alu_reg", fields, raw);
    }

    // ── GROUP 00: data movement / immediate / 16-bit ops (0x00-0x3F) ──
    if bits_76 == 0b00 {
        return decode_group00(opcode, bits_53, bits_20, &mut raw, fetch);
    }

    // ── GROUP 11: branches, stack, I/O, control (0xC0-0xFF) ──
    decode_group11(opcode, bits_53, bits_20, &mut raw, fetch)
}

fn decode_group00(
    opcode: u8,
    dst: u8,
    src: u8,
    raw: &mut Vec<u8>,
    fetch: &mut dyn FnMut() -> u8,
) -> DecodeResult {
    if opcode == NOP {
        return result("nop", HashMap::new(), raw.clone());
    }

    // LXI rp, d16 — 00pp0001
    if src == 0b001 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let word = fetch_word(raw, fetch);
        let fields = HashMap::from([
            ("pair".to_string(), pair as i32),
            ("imm".to_string(), word as i32),
        ]);
        return result("lxi", fields, raw.clone());
    }

    // INX rp — 00pp0011
    if src == 0b011 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("inx", fields, raw.clone());
    }

    // DCX rp — 00pp1011
    if src == 0b011 && (dst & 1) == 1 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("dcx", fields, raw.clone());
    }

    // DAD rp — 00pp1001
    if src == 0b001 && (dst & 1) == 1 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("dad", fields, raw.clone());
    }

    // MVI r, d8 — 00rrr110
    if src == 0b110 {
        let imm = fetch_one(raw, fetch);
        let fields = HashMap::from([
            ("dst".to_string(), dst as i32),
            ("imm".to_string(), imm as i32),
        ]);
        return result("mvi", fields, raw.clone());
    }

    // INR r — 00rrr100
    if src == 0b100 {
        let fields = HashMap::from([("dst".to_string(), dst as i32)]);
        return result("inr", fields, raw.clone());
    }

    // DCR r — 00rrr101
    if src == 0b101 {
        let fields = HashMap::from([("dst".to_string(), dst as i32)]);
        return result("dcr", fields, raw.clone());
    }

    match opcode {
        STAX_B => result("stax", HashMap::from([("pair".to_string(), PAIR_B as i32)]), raw.clone()),
        STAX_D => result("stax", HashMap::from([("pair".to_string(), PAIR_D as i32)]), raw.clone()),
        LDAX_B => result("ldax", HashMap::from([("pair".to_string(), PAIR_B as i32)]), raw.clone()),
        LDAX_D => result("ldax", HashMap::from([("pair".to_string(), PAIR_D as i32)]), raw.clone()),
        SHLD => {
            let addr = fetch_word(raw, fetch);
            result("shld", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        LHLD => {
            let addr = fetch_word(raw, fetch);
            result("lhld", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        STA => {
            let addr = fetch_word(raw, fetch);
            result("sta", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        LDA => {
            let addr = fetch_word(raw, fetch);
            result("lda", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        RLC => result("rlc", HashMap::new(), raw.clone()),
        RRC => result("rrc", HashMap::new(), raw.clone()),
        RAL => result("ral", HashMap::new(), raw.clone()),
        RAR => result("rar", HashMap::new(), raw.clone()),
        DAA => result("daa", HashMap::new(), raw.clone()),
        CMA => result("cma", HashMap::new(), raw.clone()),
        STC => result("stc", HashMap::new(), raw.clone()),
        CMC => result("cmc", HashMap::new(), raw.clone()),
        _ => result(
            "undefined",
            HashMap::from([("opcode".to_string(), opcode as i32)]),
            raw.clone(),
        ),
    }
}

#[allow(clippy::too_many_lines)]
fn decode_group11(
    opcode: u8,
    dst: u8,
    src: u8,
    raw: &mut Vec<u8>,
    fetch: &mut dyn FnMut() -> u8,
) -> DecodeResult {
    // Conditional return — 11CCC000
    if src == 0b000 {
        let fields = HashMap::from([("cond".to_string(), dst as i32)]);
        return result("rcond", fields, raw.clone());
    }

    // POP rp — 11pp0001
    if src == 0b001 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("pop", fields, raw.clone());
    }

    // Conditional jump — 11CCC010
    if src == 0b010 {
        let addr = fetch_word(raw, fetch);
        let fields = HashMap::from([
            ("cond".to_string(), dst as i32),
            ("addr".to_string(), addr as i32),
        ]);
        return result("jcond", fields, raw.clone());
    }

    // Conditional call — 11CCC100
    if src == 0b100 {
        let addr = fetch_word(raw, fetch);
        let fields = HashMap::from([
            ("cond".to_string(), dst as i32),
            ("addr".to_string(), addr as i32),
        ]);
        return result("ccond", fields, raw.clone());
    }

    // PUSH rp — 11pp0101
    if src == 0b101 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("push", fields, raw.clone());
    }

    if opcode == RET {
        return result("ret", HashMap::new(), raw.clone());
    }
    if opcode == JMP {
        let addr = fetch_word(raw, fetch);
        return result("jmp", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone());
    }
    if opcode == CALL {
        let addr = fetch_word(raw, fetch);
        return result("call", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone());
    }

    // ALU immediate — 11ooo110
    if src == 0b110 {
        let imm = fetch_one(raw, fetch);
        let fields = HashMap::from([
            ("op".to_string(), dst as i32),
            ("imm".to_string(), imm as i32),
        ]);
        return result("alu_imm", fields, raw.clone());
    }

    // RST n — 11nnn111
    if src == 0b111 {
        let fields = HashMap::from([("n".to_string(), dst as i32)]);
        return result("rst", fields, raw.clone());
    }

    match opcode {
        XTHL => result("xthl", HashMap::new(), raw.clone()),
        SPHL => result("sphl", HashMap::new(), raw.clone()),
        XCHG => result("xchg", HashMap::new(), raw.clone()),
        PCHL => result("pchl", HashMap::new(), raw.clone()),
        IN => {
            let port = fetch_one(raw, fetch);
            result("in", HashMap::from([("port".to_string(), port as i32)]), raw.clone())
        }
        OUT => {
            let port = fetch_one(raw, fetch);
            result("out", HashMap::from([("port".to_string(), port as i32)]), raw.clone())
        }
        EI => result("ei", HashMap::new(), raw.clone()),
        DI => result("di", HashMap::new(), raw.clone()),
        _ => result(
            "undefined",
            HashMap::from([("opcode".to_string(), opcode as i32)]),
            raw.clone(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_bytes(bytes: &[u8]) -> DecodeResult {
        let mut idx = 1;
        decode(bytes[0], &mut || {
            let b = bytes[idx];
            idx += 1;
            b
        })
    }

    #[test]
    fn decode_hlt() {
        assert_eq!(decode_bytes(&[0x76]).mnemonic, "hlt");
    }

    #[test]
    fn decode_mvi_a() {
        let d = decode_bytes(&[0x3E, 0x2A]);
        assert_eq!(d.mnemonic, "mvi");
        assert_eq!(d.fields["dst"], REG_A as i32);
        assert_eq!(d.fields["imm"], 42);
    }

    #[test]
    fn decode_mov_a_b() {
        let d = decode_bytes(&[0x78]);
        assert_eq!(d.mnemonic, "mov");
        assert_eq!(d.fields["dst"], REG_A as i32);
        assert_eq!(d.fields["src"], REG_B as i32);
    }

    #[test]
    fn decode_add_b() {
        let d = decode_bytes(&[0x80]);
        assert_eq!(d.mnemonic, "alu_reg");
        assert_eq!(d.fields["op"], ALU_ADD as i32);
        assert_eq!(d.fields["src"], REG_B as i32);
    }

    #[test]
    fn decode_jmp() {
        let d = decode_bytes(&[0xC3, 0x34, 0x12]);
        assert_eq!(d.mnemonic, "jmp");
        assert_eq!(d.fields["addr"], 0x1234);
    }

    #[test]
    fn decode_ret_vs_pop_vs_rcond_no_overlap() {
        assert_eq!(decode_bytes(&[0xC9]).mnemonic, "ret");
        assert_eq!(decode_bytes(&[0xC1]).mnemonic, "pop");
        assert_eq!(decode_bytes(&[0xC0]).mnemonic, "rcond");
    }

    #[test]
    fn decode_lxi_h() {
        let d = decode_bytes(&[0x21, 0x00, 0x01]);
        assert_eq!(d.mnemonic, "lxi");
        assert_eq!(d.fields["pair"], PAIR_H as i32);
        assert_eq!(d.fields["imm"], 0x0100);
    }

    #[test]
    fn decode_rst_7() {
        let d = decode_bytes(&[0xFF]);
        assert_eq!(d.mnemonic, "rst");
        assert_eq!(d.fields["n"], 7);
    }
}

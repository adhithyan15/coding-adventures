//! Instruction decoder for the Zilog Z80 ISA.
//!
//! Like the 8080, the Z80 is variable-length: 1 to 4 bytes depending on
//! the opcode (the extra byte over 8080's max of 3 comes from `DD`/`FD`-
//! prefixed instructions that also carry a 16-bit immediate, e.g.
//! `LD IX,nn` = 4 bytes).  `decode` takes the already-fetched opcode byte
//! plus a `fetch` closure the caller uses to pull any remaining operand
//! bytes from memory (advancing PC as it goes) — same shape as
//! `intel8080_simulator::decode::decode`.
//!
//! # Dispatch structure
//!
//! ```text
//! decode(first_byte, fetch)
//!   ├─ first_byte == 0xCB → decode_cb    (bit manipulation / extended rotate-shift)
//!   ├─ first_byte == 0xED → decode_ed    (NOT PORTED — see module docs below)
//!   ├─ first_byte == 0xDD → decode_ddfd(use_ix=true)   (IX basics)
//!   ├─ first_byte == 0xFD → decode_ddfd(use_ix=false)  (IY basics)
//!   └─ otherwise          → decode_main  (base 8080-compatible set + the
//!                                          handful of Z80-only unprefixed
//!                                          opcodes: EX AF,AF' / EXX / DJNZ / JR*)
//! ```
//!
//! `decode_main`'s bit-group dispatch (group-00/01/10/11) is a direct
//! transliteration of `intel8080_simulator::decode`, since the Z80 shares
//! the 8080's bit layout for every non-prefixed 8080-legacy opcode — see
//! `code/specs/z80-encoder.md` for the full byte-identity table.  The
//! seven Z80-only unprefixed opcodes (`0x08`, `0x10`, `0x18`, `0x20`,
//! `0x28`, `0x30`, `0x38`, `0xD9`) are all bytes that fall through 8080's
//! decode to "undefined" — matching the historical fact that Zilog chose
//! precisely the 8080's undefined/reserved opcode slots for these new
//! instructions, so a stock 8080 program never collides with them.
//!
//! # `ED`-prefix: deliberately NOT ported in this v0.1.0
//!
//! The `ED`-prefix opcode space covers 16-bit `ADC`/`SBC HL,rp`, the
//! block-transfer/compare/I-O instruction families (`LDIR`/`LDDR`/
//! `CPIR`/`CPDR`/`INIR`/`INDR`/`OTIR`/`OTDR`, plus their non-repeating
//! `LDI`/`LDD`/`CPI`/`CPD`/`INI`/`IND`/`OUTI`/`OUTD` siblings), `LD A,I`/
//! `LD A,R`/`LD I,A`/`LD R,A`, `NEG`, `RETN`/`RETI`, and interrupt-mode
//! selection (`IM 0`/`IM 1`/`IM 2`) — see
//! `code/packages/python/z80-simulator/src/z80_simulator/simulator.py`'s
//! `_exec_ed` (~190 lines) for the full reference implementation.  None
//! of it is reachable from the minimal-viable `z80-backend` (`const_*` /
//! `ret_*` only), and porting the block-op family in particular pulls in
//! a genuinely different control-flow shape (auto-repeating instructions
//! that re-execute themselves until BC/the compare condition is
//! exhausted) that doesn't fit this crate's per-opcode `execute` model
//! without real design work.  `decode_ed` here fetches (and discards) the
//! second opcode byte — mirroring real Z80 fetch timing — and returns
//! `"undefined"`, which `execute.rs` treats as a fail-closed halt, same
//! as any genuinely undefined byte.  A future increment can port `ED`
//! following the same pattern this module already establishes for `CB`
//! and `DD`/`FD`.

use crate::opcodes::*;
use std::collections::HashMap;

/// Decoded instruction: mnemonic + extracted operand fields.
///
/// `fields` keys used across mnemonics: `"dst"`, `"src"`, `"reg"`,
/// `"pair"`, `"imm"`, `"addr"`, `"op"` (ALU op code), `"cond"`, `"n"` (RST
/// vector), `"port"`, `"bit"` (CB bit-index), `"e"` (JR/DJNZ signed
/// displacement).  Not every mnemonic populates every key.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    pub mnemonic: String,
    pub fields: HashMap<String, i32>,
    /// Raw bytes of this instruction (opcode + any prefix/operand bytes),
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

fn fetch_signed(raw: &mut Vec<u8>, fetch: &mut dyn FnMut() -> u8) -> i32 {
    fetch_one(raw, fetch) as i8 as i32
}

/// Decode one instruction.  `first_byte` is the byte already fetched at
/// the current PC; `fetch` pulls any further bytes (and must advance the
/// caller's PC as a side effect — see `simulator::Z80Simulator::step`).
pub fn decode(first_byte: u8, fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    match first_byte {
        CB_PREFIX => decode_cb(fetch),
        ED_PREFIX => decode_ed(fetch),
        DD_PREFIX => decode_ddfd(true, fetch),
        FD_PREFIX => decode_ddfd(false, fetch),
        _ => decode_main(first_byte, fetch),
    }
}

// ===========================================================================
// Unprefixed opcodes — base 8080-compatible set + the seven Z80-only
// unprefixed opcodes
// ===========================================================================

fn decode_main(opcode: u8, fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    let mut raw = vec![opcode];
    let bits_76 = (opcode >> 6) & 0x03; // group
    let bits_53 = (opcode >> 3) & 0x07; // dst / sub-op
    let bits_20 = opcode & 0x07; // src / sub-op

    // ── Z80-only unprefixed opcodes — all UNDEFINED on a stock 8080, so
    // checking them first can never shadow a real 8080-legacy opcode. ──
    match opcode {
        EX_AF_AF => return result("ex_af_af", HashMap::new(), raw),
        EXX => return result("exx", HashMap::new(), raw),
        DJNZ => {
            let e = fetch_signed(&mut raw, fetch);
            return result("djnz", HashMap::from([("e".to_string(), e)]), raw);
        }
        JR => {
            let e = fetch_signed(&mut raw, fetch);
            return result("jr", HashMap::from([("e".to_string(), e)]), raw);
        }
        JR_NZ | JR_Z | JR_NC | JR_C => {
            let e = fetch_signed(&mut raw, fetch);
            let cond = match opcode {
                JR_NZ => COND_NZ,
                JR_Z => COND_Z,
                JR_NC => COND_NC,
                _ => COND_C,
            };
            let fields = HashMap::from([
                ("cond".to_string(), cond as i32),
                ("e".to_string(), e),
            ]);
            return result("jr_cond", fields, raw);
        }
        _ => {}
    }

    // ── GROUP 01: LD r,r' (0x40-0x7F); 0x76 is HALT, not LD (HL),(HL) ──
    if bits_76 == 0b01 {
        if opcode == HALT {
            return result("halt", HashMap::new(), raw);
        }
        let fields = HashMap::from([
            ("dst".to_string(), bits_53 as i32),
            ("src".to_string(), bits_20 as i32),
        ]);
        return result("ld_r_r", fields, raw);
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

    // LD rp,nn — 00pp0001
    if src == 0b001 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let word = fetch_word(raw, fetch);
        let fields = HashMap::from([
            ("pair".to_string(), pair as i32),
            ("imm".to_string(), word as i32),
        ]);
        return result("ld_rp_nn", fields, raw.clone());
    }

    // INC rp — 00pp0011
    if src == 0b011 && (dst & 1) == 0 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("inc_rp", fields, raw.clone());
    }

    // DEC rp — 00pp1011
    if src == 0b011 && (dst & 1) == 1 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("dec_rp", fields, raw.clone());
    }

    // ADD HL,rp — 00pp1001
    if src == 0b001 && (dst & 1) == 1 {
        let pair = dst >> 1;
        let fields = HashMap::from([("pair".to_string(), pair as i32)]);
        return result("add_hl_rp", fields, raw.clone());
    }

    // LD r,n — 00rrr110
    if src == 0b110 {
        let imm = fetch_one(raw, fetch);
        let fields = HashMap::from([
            ("dst".to_string(), dst as i32),
            ("imm".to_string(), imm as i32),
        ]);
        return result("ld_r_n", fields, raw.clone());
    }

    // INC r — 00rrr100
    if src == 0b100 {
        let fields = HashMap::from([("dst".to_string(), dst as i32)]);
        return result("inc_r", fields, raw.clone());
    }

    // DEC r — 00rrr101
    if src == 0b101 {
        let fields = HashMap::from([("dst".to_string(), dst as i32)]);
        return result("dec_r", fields, raw.clone());
    }

    match opcode {
        LD_BC_A => result("ld_rp_a", HashMap::from([("pair".to_string(), PAIR_BC as i32)]), raw.clone()),
        LD_DE_A => result("ld_rp_a", HashMap::from([("pair".to_string(), PAIR_DE as i32)]), raw.clone()),
        LD_A_BC => result("ld_a_rp", HashMap::from([("pair".to_string(), PAIR_BC as i32)]), raw.clone()),
        LD_A_DE => result("ld_a_rp", HashMap::from([("pair".to_string(), PAIR_DE as i32)]), raw.clone()),
        LD_NN_HL => {
            let addr = fetch_word(raw, fetch);
            result("ld_nn_hl", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        LD_HL_NN => {
            let addr = fetch_word(raw, fetch);
            result("ld_hl_nn", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        LD_NN_A => {
            let addr = fetch_word(raw, fetch);
            result("ld_nn_a", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        LD_A_NN => {
            let addr = fetch_word(raw, fetch);
            result("ld_a_nn", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone())
        }
        RLCA => result("rlca", HashMap::new(), raw.clone()),
        RRCA => result("rrca", HashMap::new(), raw.clone()),
        RLA => result("rla", HashMap::new(), raw.clone()),
        RRA => result("rra", HashMap::new(), raw.clone()),
        DAA => result("daa", HashMap::new(), raw.clone()),
        CPL => result("cpl", HashMap::new(), raw.clone()),
        SCF => result("scf", HashMap::new(), raw.clone()),
        CCF => result("ccf", HashMap::new(), raw.clone()),
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
        return result("ret_cond", fields, raw.clone());
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
        return result("jp_cond", fields, raw.clone());
    }

    // Conditional call — 11CCC100
    if src == 0b100 {
        let addr = fetch_word(raw, fetch);
        let fields = HashMap::from([
            ("cond".to_string(), dst as i32),
            ("addr".to_string(), addr as i32),
        ]);
        return result("call_cond", fields, raw.clone());
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
    if opcode == JP {
        let addr = fetch_word(raw, fetch);
        return result("jp", HashMap::from([("addr".to_string(), addr as i32)]), raw.clone());
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
        EX_SP_HL => result("ex_sp_hl", HashMap::new(), raw.clone()),
        LD_SP_HL => result("ld_sp_hl", HashMap::new(), raw.clone()),
        EX_DE_HL => result("ex_de_hl", HashMap::new(), raw.clone()),
        JP_HL => result("jp_hl", HashMap::new(), raw.clone()),
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

// ===========================================================================
// CB-prefix: bit manipulation and extended rotate/shift
// ===========================================================================

fn decode_cb(fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    let mut raw = vec![CB_PREFIX];
    let op = fetch_one(&mut raw, fetch);
    let r_code = op & 0x07;
    let rot_op = (op >> 3) & 0x07;
    let bit = (op >> 3) & 0x07;

    if op < 0x40 {
        let fields = HashMap::from([
            ("op".to_string(), rot_op as i32),
            ("reg".to_string(), r_code as i32),
        ]);
        return result("cb_rot", fields, raw);
    }

    let fields = HashMap::from([
        ("bit".to_string(), bit as i32),
        ("reg".to_string(), r_code as i32),
    ]);
    if op < 0x80 {
        result("bit", fields, raw)
    } else if op < 0xC0 {
        result("res", fields, raw)
    } else {
        result("set", fields, raw)
    }
}

// ===========================================================================
// ED-prefix — NOT PORTED (see module docs above)
// ===========================================================================

fn decode_ed(fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    let mut raw = vec![ED_PREFIX];
    let op = fetch_one(&mut raw, fetch);
    result(
        "undefined",
        HashMap::from([("opcode".to_string(), op as i32)]),
        raw,
    )
}

// ===========================================================================
// DD/FD-prefix — IX/IY basics (v0.1.0 scope: LD rp,nn and INC rp only)
// ===========================================================================

fn decode_ddfd(use_ix: bool, fetch: &mut dyn FnMut() -> u8) -> DecodeResult {
    let mut raw = vec![if use_ix { DD_PREFIX } else { FD_PREFIX }];
    let op = fetch_one(&mut raw, fetch);

    // LD IX,nn / LD IY,nn — 0x21 (same second byte as LXI H on the base
    // 8080-compatible set; the DD/FD prefix redirects HL → IX/IY).
    if op == 0x21 {
        let imm = fetch_word(&mut raw, fetch);
        let mnemonic = if use_ix { "ld_ix_nn" } else { "ld_iy_nn" };
        return result(mnemonic, HashMap::from([("imm".to_string(), imm as i32)]), raw);
    }

    // INC IX / INC IY — 0x23.
    if op == 0x23 {
        let mnemonic = if use_ix { "inc_ix" } else { "inc_iy" };
        return result(mnemonic, HashMap::new(), raw);
    }

    result(
        "undefined",
        HashMap::from([("opcode".to_string(), op as i32)]),
        raw,
    )
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
    fn decode_halt() {
        assert_eq!(decode_bytes(&[0x76]).mnemonic, "halt");
    }

    #[test]
    fn decode_ld_a_n() {
        let d = decode_bytes(&[0x3E, 0x2A]);
        assert_eq!(d.mnemonic, "ld_r_n");
        assert_eq!(d.fields["dst"], REG_A as i32);
        assert_eq!(d.fields["imm"], 42);
    }

    #[test]
    fn decode_ld_a_b() {
        let d = decode_bytes(&[0x78]);
        assert_eq!(d.mnemonic, "ld_r_r");
        assert_eq!(d.fields["dst"], REG_A as i32);
        assert_eq!(d.fields["src"], REG_B as i32);
    }

    #[test]
    fn decode_add_a_b() {
        let d = decode_bytes(&[0x80]);
        assert_eq!(d.mnemonic, "alu_reg");
        assert_eq!(d.fields["op"], ALU_ADD as i32);
        assert_eq!(d.fields["src"], REG_B as i32);
    }

    #[test]
    fn decode_jp() {
        let d = decode_bytes(&[0xC3, 0x34, 0x12]);
        assert_eq!(d.mnemonic, "jp");
        assert_eq!(d.fields["addr"], 0x1234);
    }

    #[test]
    fn decode_ret_vs_pop_vs_retcond_no_overlap() {
        assert_eq!(decode_bytes(&[0xC9]).mnemonic, "ret");
        assert_eq!(decode_bytes(&[0xC1]).mnemonic, "pop");
        assert_eq!(decode_bytes(&[0xC0]).mnemonic, "ret_cond");
    }

    #[test]
    fn decode_ex_af_af_and_exx() {
        assert_eq!(decode_bytes(&[0x08]).mnemonic, "ex_af_af");
        assert_eq!(decode_bytes(&[0xD9]).mnemonic, "exx");
    }

    #[test]
    fn decode_djnz_and_jr_family() {
        let d = decode_bytes(&[0x10, 0xFE]);
        assert_eq!(d.mnemonic, "djnz");
        assert_eq!(d.fields["e"], -2);

        let d = decode_bytes(&[0x18, 0x05]);
        assert_eq!(d.mnemonic, "jr");
        assert_eq!(d.fields["e"], 5);

        let d = decode_bytes(&[0x20, 0x03]);
        assert_eq!(d.mnemonic, "jr_cond");
        assert_eq!(d.fields["cond"], COND_NZ as i32);
        assert_eq!(d.fields["e"], 3);
    }

    #[test]
    fn decode_cb_rlc_and_bit() {
        let d = decode_bytes(&[0xCB, 0x00]);
        assert_eq!(d.mnemonic, "cb_rot");
        assert_eq!(d.fields["op"], ROT_RLC_CONST);
        assert_eq!(d.fields["reg"], REG_B as i32);

        let d = decode_bytes(&[0xCB, 0x7F]);
        assert_eq!(d.mnemonic, "bit");
        assert_eq!(d.fields["bit"], 7);
        assert_eq!(d.fields["reg"], REG_A as i32);
    }

    // Local alias so the test above doesn't need to depend on
    // `crate::encoding::ROT_RLC` (kept decode.rs's dependency surface to
    // just `opcodes`).
    const ROT_RLC_CONST: i32 = 0;

    #[test]
    fn decode_ddfd_ix_iy_basics() {
        let d = decode_bytes(&[0xDD, 0x21, 0x34, 0x12]);
        assert_eq!(d.mnemonic, "ld_ix_nn");
        assert_eq!(d.fields["imm"], 0x1234);

        let d = decode_bytes(&[0xFD, 0x23]);
        assert_eq!(d.mnemonic, "inc_iy");
    }

    #[test]
    fn decode_ed_not_ported_is_undefined() {
        let d = decode_bytes(&[0xED, 0x57]); // LD A,I on real hardware
        assert_eq!(d.mnemonic, "undefined");
    }

    #[test]
    fn decode_lxi_h_style() {
        let d = decode_bytes(&[0x21, 0x00, 0x01]);
        assert_eq!(d.mnemonic, "ld_rp_nn");
        assert_eq!(d.fields["pair"], PAIR_HL as i32);
        assert_eq!(d.fields["imm"], 0x0100);
    }

    #[test]
    fn decode_rst_7() {
        let d = decode_bytes(&[0xFF]);
        assert_eq!(d.mnemonic, "rst");
        assert_eq!(d.fields["n"], 7);
    }
}

//! Instruction executor — one function per opword "line" (the top 4
//! bits), direct transcription of the Python original's
//! `_exec_line0`..`_exec_lineE` dispatch methods.
//!
//! # Scope: which lines/instructions are ported
//!
//! | Line | Python group | This port |
//! |------|--------------|-----------|
//! | 0 | ORI/ANDI/SUBI/ADDI/EORI/CMPI, BTST/BCHG/BCLR/BSET | ❌ deferred entirely |
//! | 1/2/3 | MOVE.B/W/L, MOVEA | ✅ [`exec_move`] |
//! | 4 | NOP/RTS/RTR/STOP/TRAP/LINK/UNLK/SWAP/EXT/CLR/NEG/NOT/TST/LEA/JSR/JMP | ✅ [`exec_line4`] (NEGX, PEA, `MOVE SR/CCR` deferred) |
//! | 5 | ADDQ/SUBQ/Scc/DBcc | ✅ [`exec_line5`] |
//! | 6 | BRA/BSR/Bcc | ✅ [`exec_line6`] |
//! | 7 | MOVEQ | ✅ [`exec_moveq`] |
//! | 8 | OR/DIVU/DIVS | ✅ OR only, [`exec_line8`] (DIVU/DIVS deferred) |
//! | 9 | SUB/SUBA/SUBX | ✅ SUB/SUBA, [`exec_line9`] (SUBX deferred) |
//! | A | line-A trap (unimplemented on real silicon too) | ❌ (matches real hardware: reserved) |
//! | B | CMP/CMPA/EOR | ✅ [`exec_line_b`] |
//! | C | AND/MULU/MULS/EXG | ✅ AND/EXG, [`exec_line_c`] (MULU/MULS deferred) |
//! | D | ADD/ADDA/ADDX | ✅ ADD/ADDA, [`exec_line_d`] (ADDX deferred) |
//! | E | shift/rotate | ✅ register form only, [`exec_line_e`] (memory-operand form deferred) |
//! | F | line-F trap (co-processor, unimplemented on real silicon too) | ❌ (reserved) |
//!
//! Every deferred instruction/line returns `Err` (never silently
//! mis-executes), so a program that hits one halts the simulator with a
//! descriptive message rather than corrupting state — the same
//! fail-closed policy `mos6502-simulator` uses for illegal opcodes.
//!
//! # Why free functions over `&mut M68kSimulator`, not a decomposed
//! parameter list
//!
//! Same reasoning as `mos6502-simulator::execute`: 68000 instructions are
//! deeply cross-cutting (stack ops touch `A[7]` *and* memory *and* `pc`;
//! almost every ALU op touches `sr`), so decomposing into a MIPS-style
//! parameter list would just mean re-deriving `&mut M68kSimulator`'s
//! field set at every call site.

use crate::decode::{decode_ea, ea_address, ea_read, ea_write, fetch_word, fetch_word_signed};
use crate::flags::{compute_n, compute_nzvc_add, compute_nzvc_neg, compute_nzvc_sub, compute_v_sub};
use crate::opcodes::{cc_taken, mask_for, msb_for, sext16, sext8, sz_arith, sz_move, ADDR_MASK, CC_NAMES};
use crate::simulator::M68kSimulator;

// ===========================================================================
// Small shared helpers (register write, stack push/pop, CCR helpers)
// ===========================================================================

/// Write `sz` bytes into data register `n`, preserving the untouched
/// upper bytes (byte/word ops on a 68000 data register never clobber the
/// rest of the register).
pub fn set_dn(sim: &mut M68kSimulator, n: u8, val: u32, sz: u8) {
    let mask = mask_for(sz);
    let idx = n as usize;
    sim.d[idx] = (sim.d[idx] & !mask) | (val & mask);
}

fn push_long(sim: &mut M68kSimulator, val: u32) -> Result<(), String> {
    sim.a[7] = sim.a[7].wrapping_sub(4) & ADDR_MASK;
    crate::decode::mem_write(&mut sim.mem, sim.a[7], 4, val)
}

fn pop_long(sim: &mut M68kSimulator) -> Result<u32, String> {
    let v = crate::decode::mem_read(&sim.mem, sim.a[7], 4)?;
    sim.a[7] = sim.a[7].wrapping_add(4) & ADDR_MASK;
    Ok(v)
}

fn pop_word(sim: &mut M68kSimulator) -> Result<u16, String> {
    let v = crate::decode::mem_read(&sim.mem, sim.a[7], 2)?;
    sim.a[7] = sim.a[7].wrapping_add(2) & ADDR_MASK;
    Ok(v as u16)
}

/// Current `(N, Z, V, C)` CCR bits, for `Bcc`/`DBcc`/`Scc`.
fn ccr_bits(sim: &M68kSimulator) -> (bool, bool, bool, bool) {
    let sr = sim.sr;
    (sr & 8 != 0, sr & 4 != 0, sr & 2 != 0, sr & 1 != 0)
}

/// Update the CCR bits in `sr`.  `x = None` leaves X unchanged (matches
/// the Python original's `_set_ccr(..., x=None)` default).
fn set_ccr(sim: &mut M68kSimulator, n: bool, z: bool, v: bool, c: bool, x: Option<bool>) {
    let mut sr = sim.sr & 0xFFE0;
    sr |= u16::from(n) << 3;
    sr |= u16::from(z) << 2;
    sr |= u16::from(v) << 1;
    sr |= u16::from(c);
    sr = match x {
        Some(xv) => (sr & !0x10) | (u16::from(xv) << 4),
        None => sr | (sim.sr & 0x10),
    };
    sim.sr = sr;
}

fn set_ccr_logic(sim: &mut M68kSimulator, result: u32, sz: u8) {
    let (n, z) = crate::flags::compute_nz_logic(result, sz);
    set_ccr(sim, n, z, false, false, None);
}

fn set_ccr_nzvc_add(sim: &mut M68kSimulator, a: u32, b: u32, raw: i64, sz: u8) {
    let (n, z, v, c, x) = compute_nzvc_add(a, b, raw, sz);
    set_ccr(sim, n, z, v, c, Some(x));
}

fn set_ccr_nzvc_sub(sim: &mut M68kSimulator, a: u32, b: u32, raw: i64, sz: u8) {
    let (n, z, v, c, x) = compute_nzvc_sub(a, b, raw, sz);
    set_ccr(sim, n, z, v, c, Some(x));
}

fn set_ccr_cmp(sim: &mut M68kSimulator, a: u32, b: u32, raw: i64, sz: u8) {
    let result = (raw & i64::from(mask_for(sz))) as u32;
    let n = compute_n(result, sz);
    let z = crate::flags::compute_z(result, sz);
    let v = compute_v_sub(a, b, result, sz);
    let c = crate::flags::compute_c_sub(a, b);
    set_ccr(sim, n, z, v, c, None); // X unchanged for CMP
}

// ===========================================================================
// Top-level dispatcher
// ===========================================================================

/// Fetch and execute one instruction.  Returns the mnemonic on success,
/// or `Err` describing why decode/execute failed (illegal opcode,
/// deferred addressing mode, misaligned access, deferred instruction
/// family).  Callers (`M68kSimulator::step`) treat `Err` as a fail-closed
/// halt.
pub fn decode_and_execute(sim: &mut M68kSimulator) -> Result<String, String> {
    let op = fetch_word(sim);
    let hi = (op >> 12) & 0xF;
    match hi {
        0x1..=0x3 => exec_move(sim, op),
        0x4 => exec_line4(sim, op),
        0x5 => exec_line5(sim, op),
        0x6 => exec_line6(sim, op),
        0x7 => exec_moveq(sim, op),
        0x8 => exec_line8(sim, op),
        0x9 => exec_line9(sim, op),
        0xB => exec_line_b(sim, op),
        0xC => exec_line_c(sim, op),
        0xD => exec_line_d(sim, op),
        0xE => exec_line_e(sim, op),
        0x0 => Err(format!(
            "line-0 (immediate/bit-ops group: ORI/ANDI/SUBI/ADDI/EORI/CMPI, \
             BTST/BCHG/BCLR/BSET) is not supported by m68k-simulator v0.1.0: {op:#06x}"
        )),
        _ => Err(format!(
            "line-{hi:X} is reserved/unimplemented on real 68000 silicon too: {op:#06x}"
        )),
    }
}

// ===========================================================================
// Lines 1/2/3 -- MOVE / MOVEA
// ===========================================================================

fn exec_move(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let sz_code = ((op >> 12) & 3) as u8;
    let sz = sz_move(sz_code).ok_or_else(|| format!("MOVE bad size code in {op:#06x}"))?;
    let dst_reg = ((op >> 9) & 7) as u8;
    let dst_mode = ((op >> 6) & 7) as u8;
    let src_mode = ((op >> 3) & 7) as u8;
    let src_reg = (op & 7) as u8;

    let src_ea = decode_ea(src_mode, src_reg)?;
    let val = ea_read(sim, src_ea, sz)?;

    if dst_mode == 1 {
        // MOVEA -- move to address register, no flags, word source
        // sign-extends to 32 bits.
        let v = if sz == 2 { sext16(val as u16) as u32 } else { val };
        sim.a[dst_reg as usize] = v;
        return Ok("MOVEA".to_string());
    }

    let dst_ea = decode_ea(dst_mode, dst_reg)?;
    ea_write(sim, dst_ea, sz, val)?;
    let result = val & mask_for(sz);
    set_ccr_logic(sim, result, sz);
    Ok("MOVE".to_string())
}

// ===========================================================================
// Line 4 -- miscellaneous
// ===========================================================================

#[allow(clippy::too_many_lines)]
fn exec_line4(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    if op == 0x4E71 {
        return Ok("NOP".to_string());
    }
    if op == 0x4E70 {
        return Ok("RESET".to_string());
    }
    if op == 0x4E75 {
        let ret = pop_long(sim)? & ADDR_MASK;
        sim.pc = ret;
        return Ok("RTS".to_string());
    }
    if op == 0x4E77 {
        let ccr_word = pop_word(sim)?;
        sim.sr = (sim.sr & 0xFF00) | (ccr_word & 0x1F);
        let ret = pop_long(sim)? & ADDR_MASK;
        sim.pc = ret;
        return Ok("RTR".to_string());
    }
    if op == 0x4E72 {
        // STOP #imm -- load imm into SR, halt.  Not this simulator's
        // chosen HALT convention (see lib.rs) but real 68000 semantics
        // are still worth honouring for any program that happens to use
        // it directly.
        let imm = fetch_word(sim);
        sim.sr = imm;
        sim.halted = true;
        return Ok(format!("STOP #{imm:#06x}"));
    }
    if (0x4E40..=0x4E4F).contains(&op) {
        let n = op & 0xF;
        if n == 15 {
            sim.halted = true;
        } else {
            sim.d[7] = u32::from(n);
        }
        return Ok(format!("TRAP #{n}"));
    }
    if (0x4E50..=0x4E57).contains(&op) {
        let n = (op & 7) as usize;
        let disp = fetch_word_signed(sim);
        push_long(sim, sim.a[n])?;
        sim.a[n] = sim.a[7];
        sim.a[7] = ((i64::from(sim.a[7]) + i64::from(disp)) as u32) & ADDR_MASK;
        return Ok(format!("LINK A{n},#{disp}"));
    }
    if (0x4E58..=0x4E5F).contains(&op) {
        let n = (op & 7) as usize;
        sim.a[7] = sim.a[n];
        sim.a[n] = pop_long(sim)?;
        return Ok(format!("UNLK A{n}"));
    }
    if (0x4840..=0x4847).contains(&op) {
        let n = (op & 7) as usize;
        let val = sim.d[n];
        let swapped = (val >> 16) | ((val & 0xFFFF) << 16);
        sim.d[n] = swapped;
        set_ccr_logic(sim, swapped, 4);
        return Ok(format!("SWAP D{n}"));
    }
    if (0x4880..=0x4887).contains(&op) {
        let n = (op & 7) as usize;
        let b = sext8((sim.d[n] & 0xFF) as u8) as u32;
        let w = b & 0xFFFF;
        set_dn(sim, n as u8, w, 2);
        set_ccr_logic(sim, w, 2);
        return Ok(format!("EXT.W D{n}"));
    }
    if (0x48C0..=0x48C7).contains(&op) {
        let n = (op & 7) as usize;
        let lw = sext16((sim.d[n] & 0xFFFF) as u16) as u32;
        sim.d[n] = lw;
        set_ccr_logic(sim, lw, 4);
        return Ok(format!("EXT.L D{n}"));
    }

    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if (op & 0xFF00) == 0x4200 && sz_code <= 2 {
        // CLR.sz <ea>
        let sz = sz_arith(sz_code).ok_or_else(|| format!("CLR bad size {op:#06x}"))?;
        let ea = decode_ea(mode, reg)?;
        ea_write(sim, ea, sz, 0)?;
        set_ccr(sim, false, true, false, false, None);
        return Ok("CLR".to_string());
    }
    if (op & 0xFF00) == 0x4400 && sz_code <= 2 {
        // NEG.sz <ea>
        let sz = sz_arith(sz_code).ok_or_else(|| format!("NEG bad size {op:#06x}"))?;
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, sz)?;
        let raw = 0u32.wrapping_sub(src) & mask_for(sz);
        ea_write(sim, ea, sz, raw)?;
        let (n, z, v, c, x) = compute_nzvc_neg(src, raw, sz);
        set_ccr(sim, n, z, v, c, Some(x));
        return Ok("NEG".to_string());
    }
    if (op & 0xFF00) == 0x4600 && sz_code <= 2 {
        // NOT.sz <ea>
        let sz = sz_arith(sz_code).ok_or_else(|| format!("NOT bad size {op:#06x}"))?;
        let ea = decode_ea(mode, reg)?;
        let val = ea_read(sim, ea, sz)?;
        let result = (!val) & mask_for(sz);
        ea_write(sim, ea, sz, result)?;
        set_ccr_logic(sim, result, sz);
        return Ok("NOT".to_string());
    }
    if (op & 0xFF00) == 0x4A00 && sz_code <= 2 {
        // TST.sz <ea>
        let sz = sz_arith(sz_code).ok_or_else(|| format!("TST bad size {op:#06x}"))?;
        let ea = decode_ea(mode, reg)?;
        let val = ea_read(sim, ea, sz)? & mask_for(sz);
        set_ccr_logic(sim, val, sz);
        return Ok("TST".to_string());
    }
    if (op & 0x01C0) == 0x01C0 && (op & 0xF000) == 0x4000 {
        // LEA <ea>, An
        let an = ((op >> 9) & 7) as usize;
        if mode >= 2 && !(mode == 7 && reg == 4) {
            let ea = decode_ea(mode, reg)?;
            let addr = ea_address(sim, ea, 4)?;
            sim.a[an] = addr;
            return Ok(format!("LEA ,A{an}"));
        }
    }
    if (op & 0xFFC0) == 0x4E80 {
        // JSR <ea>
        let ea = decode_ea(mode, reg)?;
        let target = ea_address(sim, ea, 4)?;
        push_long(sim, sim.pc)?;
        sim.pc = target & ADDR_MASK;
        return Ok("JSR".to_string());
    }
    if (op & 0xFFC0) == 0x4EC0 {
        // JMP <ea>
        let ea = decode_ea(mode, reg)?;
        let target = ea_address(sim, ea, 4)?;
        sim.pc = target & ADDR_MASK;
        return Ok("JMP".to_string());
    }

    Err(format!(
        "unimplemented/deferred line-4 opcode {op:#06x} (NEGX, PEA, MOVE SR/CCR \
         are not ported by m68k-simulator v0.1.0)"
    ))
}

// ===========================================================================
// Line 5 -- ADDQ, SUBQ, Scc, DBcc
// ===========================================================================

fn exec_line5(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;
    let data = u32::from((op >> 9) & 7);
    let imm = if data == 0 { 8 } else { data };

    if sz_code == 3 && mode == 1 {
        // DBcc Dn, #disp
        let cc = ((op >> 8) & 0xF) as u8;
        let pc_before_ext = sim.pc;
        let disp = fetch_word_signed(sim);
        let target = ((i64::from(pc_before_ext) + i64::from(disp)) as u32) & ADDR_MASK;
        let (n, z, v, c) = ccr_bits(sim);
        if !cc_taken(cc, n, z, v, c) {
            let count = sext16((sim.d[reg as usize] & 0xFFFF) as u16) - 1;
            set_dn(sim, reg, (count as u32) & 0xFFFF, 2);
            if count != -1 {
                sim.pc = target;
            }
        }
        return Ok(format!("DB{}", CC_NAMES[cc as usize]));
    }
    if sz_code == 3 {
        // Scc <ea>
        let cc = ((op >> 8) & 0xF) as u8;
        let (n, z, v, c) = ccr_bits(sim);
        let val = if cc_taken(cc, n, z, v, c) { 0xFF } else { 0x00 };
        let ea = decode_ea(mode, reg)?;
        ea_write(sim, ea, 1, val)?;
        return Ok(format!("S{}", CC_NAMES[cc as usize]));
    }

    let sz = sz_arith(sz_code).ok_or_else(|| format!("ADDQ/SUBQ bad size {op:#06x}"))?;
    let sub = (op >> 8) & 1 != 0;

    if !sub {
        if mode == 1 {
            let n = reg as usize;
            sim.a[n] = sim.a[n].wrapping_add(imm);
            return Ok("ADDQ".to_string());
        }
        let ea = decode_ea(mode, reg)?;
        let a = ea_read(sim, ea, sz)?;
        let raw = i64::from(a) + i64::from(imm);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        ea_write(sim, ea, sz, result)?;
        set_ccr_nzvc_add(sim, a, imm, raw, sz);
        Ok("ADDQ".to_string())
    } else {
        if mode == 1 {
            let n = reg as usize;
            sim.a[n] = sim.a[n].wrapping_sub(imm);
            return Ok("SUBQ".to_string());
        }
        let ea = decode_ea(mode, reg)?;
        let a = ea_read(sim, ea, sz)?;
        let raw = i64::from(a) - i64::from(imm);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        ea_write(sim, ea, sz, result)?;
        set_ccr_nzvc_sub(sim, a, imm, raw, sz);
        Ok("SUBQ".to_string())
    }
}

// ===========================================================================
// Line 6 -- BRA, BSR, Bcc
// ===========================================================================

fn exec_line6(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let cc = ((op >> 8) & 0xF) as u8;
    let disp8 = (op & 0xFF) as u8;
    let pc_base = sim.pc;
    let disp = if disp8 == 0 {
        fetch_word_signed(sim)
    } else {
        sext8(disp8)
    };
    let target = ((i64::from(pc_base) + i64::from(disp)) as u32) & ADDR_MASK;

    if cc == 0 {
        sim.pc = target;
        return Ok("BRA".to_string());
    }
    if cc == 1 {
        push_long(sim, sim.pc)?;
        sim.pc = target;
        return Ok("BSR".to_string());
    }
    let (n, z, v, c) = ccr_bits(sim);
    if cc_taken(cc, n, z, v, c) {
        sim.pc = target;
    }
    Ok(format!("B{}", CC_NAMES[cc as usize]))
}

// ===========================================================================
// Line 7 -- MOVEQ
// ===========================================================================

fn exec_moveq(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    if op & 0x0100 != 0 {
        return Err(format!("not MOVEQ (bit 8 set): {op:#06x}"));
    }
    let dn = ((op >> 9) & 7) as usize;
    let imm = sext8((op & 0xFF) as u8) as u32;
    sim.d[dn] = imm;
    set_ccr_logic(sim, imm, 4);
    Ok("MOVEQ".to_string())
}

// ===========================================================================
// Line 8 -- OR (DIVU/DIVS deferred)
// ===========================================================================

fn exec_line8(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let dn = ((op >> 9) & 7) as usize;
    let dir_bit = (op >> 8) & 1 != 0;
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if sz_code == 3 {
        return Err(format!(
            "DIVU/DIVS are not supported by m68k-simulator v0.1.0: {op:#06x}"
        ));
    }
    let sz = sz_arith(sz_code).ok_or_else(|| format!("OR bad size {op:#06x}"))?;

    if !dir_bit {
        // OR <ea>, Dn
        let ea = decode_ea(mode, reg)?;
        let b = ea_read(sim, ea, sz)?;
        let a = sim.d[dn] & mask_for(sz);
        let result = (a | b) & mask_for(sz);
        set_dn(sim, dn as u8, result, sz);
        set_ccr_logic(sim, result, sz);
        Ok("OR".to_string())
    } else {
        // OR Dn, <ea>
        let a = sim.d[dn] & mask_for(sz);
        let ea = decode_ea(mode, reg)?;
        let addr = ea_address(sim, ea, sz)?;
        let val = crate::decode::mem_read(&sim.mem, addr, sz)?;
        let result = (val | a) & mask_for(sz);
        crate::decode::mem_write(&mut sim.mem, addr, sz, result)?;
        set_ccr_logic(sim, result, sz);
        Ok("OR".to_string())
    }
}

// ===========================================================================
// Line 9 -- SUB, SUBA (SUBX deferred)
// ===========================================================================

fn exec_line9(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let dn = ((op >> 9) & 7) as usize;
    let dir_bit = (op >> 8) & 1 != 0;
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if sz_code == 3 && !dir_bit {
        // SUBA.W -- sign-extends 16-bit source to 32 bits.
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 2)?;
        let src = sext16(src as u16) as u32;
        sim.a[dn] = sim.a[dn].wrapping_sub(src);
        return Ok("SUBA.W".to_string());
    }
    if sz_code == 3 && dir_bit {
        // SUBA.L
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 4)?;
        sim.a[dn] = sim.a[dn].wrapping_sub(src);
        return Ok("SUBA.L".to_string());
    }

    let sz = sz_arith(sz_code).ok_or_else(|| format!("SUB bad size {op:#06x}"))?;
    if dir_bit && mode == 0 {
        return Err(format!(
            "SUBX is not supported by m68k-simulator v0.1.0: {op:#06x}"
        ));
    }

    if !dir_bit {
        // SUB <ea>, Dn
        let ea = decode_ea(mode, reg)?;
        let b = ea_read(sim, ea, sz)?;
        let a = sim.d[dn] & mask_for(sz);
        let raw = i64::from(a) - i64::from(b);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        set_dn(sim, dn as u8, result, sz);
        set_ccr_nzvc_sub(sim, a, b, raw, sz);
        Ok("SUB".to_string())
    } else {
        // SUB Dn, <ea>
        let a = sim.d[dn] & mask_for(sz);
        let ea = decode_ea(mode, reg)?;
        let addr = ea_address(sim, ea, sz)?;
        let val = crate::decode::mem_read(&sim.mem, addr, sz)?;
        let raw = i64::from(val) - i64::from(a);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        crate::decode::mem_write(&mut sim.mem, addr, sz, result)?;
        set_ccr_nzvc_sub(sim, val, a, raw, sz);
        Ok("SUB".to_string())
    }
}

// ===========================================================================
// Line B -- CMP, CMPA, EOR
// ===========================================================================

fn exec_line_b(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let dn = ((op >> 9) & 7) as usize;
    let dir_bit = (op >> 8) & 1 != 0;
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if sz_code == 3 && !dir_bit {
        // CMPA.W
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 2)?;
        let src = sext16(src as u16) as u32;
        let a = sim.a[dn];
        let raw = i64::from(a) - i64::from(src);
        set_ccr_cmp(sim, a, src, raw, 4);
        return Ok("CMPA.W".to_string());
    }
    if sz_code == 3 && dir_bit {
        // CMPA.L
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 4)?;
        let a = sim.a[dn];
        let raw = i64::from(a) - i64::from(src);
        set_ccr_cmp(sim, a, src, raw, 4);
        return Ok("CMPA.L".to_string());
    }

    let sz = sz_arith(sz_code).ok_or_else(|| format!("CMP/EOR bad size {op:#06x}"))?;

    if !dir_bit {
        // CMP <ea>, Dn
        let ea = decode_ea(mode, reg)?;
        let b = ea_read(sim, ea, sz)?;
        let a = sim.d[dn] & mask_for(sz);
        let raw = i64::from(a) - i64::from(b);
        set_ccr_cmp(sim, a, b, raw, sz);
        Ok("CMP".to_string())
    } else {
        // EOR Dn, <ea>
        let a = sim.d[dn] & mask_for(sz);
        let ea = decode_ea(mode, reg)?;
        let result = if mode == 0 {
            let val = sim.d[reg as usize] & mask_for(sz);
            let result = (val ^ a) & mask_for(sz);
            set_dn(sim, reg, result, sz);
            result
        } else {
            let addr = ea_address(sim, ea, sz)?;
            let val = crate::decode::mem_read(&sim.mem, addr, sz)?;
            let result = (val ^ a) & mask_for(sz);
            crate::decode::mem_write(&mut sim.mem, addr, sz, result)?;
            result
        };
        set_ccr_logic(sim, result, sz);
        Ok("EOR".to_string())
    }
}

// ===========================================================================
// Line C -- AND, EXG (MULU/MULS deferred)
// ===========================================================================

fn exec_line_c(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let dn = ((op >> 9) & 7) as usize;
    let dir_bit = (op >> 8) & 1 != 0;
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if op & 0xF1F8 == 0xC140 {
        sim.d.swap(dn, reg as usize);
        return Ok(format!("EXG D{dn},D{reg}"));
    }
    if op & 0xF1F8 == 0xC148 {
        sim.a.swap(dn, reg as usize);
        return Ok(format!("EXG A{dn},A{reg}"));
    }
    if op & 0xF1F8 == 0xC188 {
        std::mem::swap(&mut sim.d[dn], &mut sim.a[reg as usize]);
        return Ok(format!("EXG D{dn},A{reg}"));
    }

    if sz_code == 3 {
        return Err(format!(
            "MULU/MULS are not supported by m68k-simulator v0.1.0: {op:#06x}"
        ));
    }
    let sz = sz_arith(sz_code).ok_or_else(|| format!("AND bad size {op:#06x}"))?;

    if !dir_bit {
        let ea = decode_ea(mode, reg)?;
        let b = ea_read(sim, ea, sz)?;
        let a = sim.d[dn] & mask_for(sz);
        let result = (a & b) & mask_for(sz);
        set_dn(sim, dn as u8, result, sz);
        set_ccr_logic(sim, result, sz);
        Ok("AND".to_string())
    } else {
        let a = sim.d[dn] & mask_for(sz);
        let ea = decode_ea(mode, reg)?;
        let addr = ea_address(sim, ea, sz)?;
        let val = crate::decode::mem_read(&sim.mem, addr, sz)?;
        let result = (val & a) & mask_for(sz);
        crate::decode::mem_write(&mut sim.mem, addr, sz, result)?;
        set_ccr_logic(sim, result, sz);
        Ok("AND".to_string())
    }
}

// ===========================================================================
// Line D -- ADD, ADDA (ADDX deferred)
// ===========================================================================

fn exec_line_d(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let dn = ((op >> 9) & 7) as usize;
    let dir_bit = (op >> 8) & 1 != 0;
    let sz_code = ((op >> 6) & 3) as u8;
    let mode = ((op >> 3) & 7) as u8;
    let reg = (op & 7) as u8;

    if sz_code == 3 && !dir_bit {
        // ADDA.W
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 2)?;
        let src = sext16(src as u16) as u32;
        sim.a[dn] = sim.a[dn].wrapping_add(src);
        return Ok("ADDA.W".to_string());
    }
    if sz_code == 3 && dir_bit {
        // ADDA.L
        let ea = decode_ea(mode, reg)?;
        let src = ea_read(sim, ea, 4)?;
        sim.a[dn] = sim.a[dn].wrapping_add(src);
        return Ok("ADDA.L".to_string());
    }

    let sz = sz_arith(sz_code).ok_or_else(|| format!("ADD bad size {op:#06x}"))?;
    if dir_bit && mode == 0 {
        return Err(format!(
            "ADDX is not supported by m68k-simulator v0.1.0: {op:#06x}"
        ));
    }

    if !dir_bit {
        // ADD <ea>, Dn
        let ea = decode_ea(mode, reg)?;
        let b = ea_read(sim, ea, sz)?;
        let a = sim.d[dn] & mask_for(sz);
        let raw = i64::from(a) + i64::from(b);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        set_dn(sim, dn as u8, result, sz);
        set_ccr_nzvc_add(sim, a, b, raw, sz);
        Ok("ADD".to_string())
    } else {
        // ADD Dn, <ea>
        let a = sim.d[dn] & mask_for(sz);
        let ea = decode_ea(mode, reg)?;
        let addr = ea_address(sim, ea, sz)?;
        let val = crate::decode::mem_read(&sim.mem, addr, sz)?;
        let raw = i64::from(val) + i64::from(a);
        let result = (raw & i64::from(mask_for(sz))) as u32;
        crate::decode::mem_write(&mut sim.mem, addr, sz, result)?;
        set_ccr_nzvc_add(sim, val, a, raw, sz);
        Ok("ADD".to_string())
    }
}

// ===========================================================================
// Line E -- shift/rotate (register form only; memory-operand form deferred)
// ===========================================================================

fn exec_line_e(sim: &mut M68kSimulator, op: u16) -> Result<String, String> {
    let sz_code = ((op >> 6) & 3) as u8;
    if sz_code == 3 {
        return Err(format!(
            "memory-operand shift/rotate is not supported by m68k-simulator \
             v0.1.0: {op:#06x}"
        ));
    }
    let sz = sz_arith(sz_code).ok_or_else(|| format!("shift bad size {op:#06x}"))?;
    let direction = ((op >> 8) & 1) as u8; // 1=left, 0=right
    let reg_count = (op >> 5) & 1 != 0;
    let shift_type = ((op >> 3) & 3) as u8; // 0=AS,1=LS,2=ROX,3=RO
    let dn = (op & 7) as usize;
    let cnt_field = ((op >> 9) & 7) as usize;

    let count: u32 = if reg_count {
        sim.d[cnt_field] % 64
    } else if cnt_field == 0 {
        8
    } else {
        cnt_field as u32
    };

    let val = sim.d[dn] & mask_for(sz);
    let msb = msb_for(sz);
    let mask = mask_for(sz);
    let bits = u32::from(sz) * 8;
    let x_in = sim.sr & 0x10 != 0;

    let (new_val, n, z, v, c, x) =
        shift_value(val, count, direction, shift_type, bits, mask, msb, x_in, sim.sr);
    set_dn(sim, dn as u8, new_val, sz);
    set_ccr(sim, n, z, v, c, Some(x));

    let names = ["AS", "LS", "ROX", "RO"];
    let d_char = if direction == 1 { "L" } else { "R" };
    Ok(format!("{}{d_char}", names[shift_type as usize]))
}

/// Core shift/rotate logic, direct transcription of the Python
/// original's `_shift_value`.  `bits` is `sz * 8` (8/16/32); `count` may
/// exceed `bits` for the register-count form (already reduced `% 64` by
/// the caller) -- circular rotates reduce it again `% bits` here, and a
/// reduced count of exactly 0 (a "full rotation" back to the start) is
/// treated as the identity to avoid a same-width shift, which is
/// undefined behaviour in Rust (unlike Python's arbitrary-precision
/// integers).
#[allow(clippy::too_many_arguments)]
fn shift_value(
    val: u32,
    count: u32,
    direction: u8,
    shift_type: u8,
    bits: u32,
    mask: u32,
    msb: u32,
    x_in: bool,
    sr_for_rotate_x: u16,
) -> (u32, bool, bool, bool, bool, bool) {
    let mut result = val;
    let mut last_out = false;
    let mut v_flag = false;

    match shift_type {
        0 => {
            // Arithmetic shift
            if direction == 1 {
                // ASL
                let orig_msb = val & msb != 0;
                for _ in 0..count {
                    last_out = result & msb != 0;
                    result = (result << 1) & mask;
                    if (result & msb != 0) != orig_msb {
                        v_flag = true;
                    }
                }
            } else {
                // ASR -- replicate the current sign bit in
                let sign_bit = result & msb;
                for _ in 0..count {
                    last_out = result & 1 != 0;
                    result = ((result >> 1) | sign_bit) & mask;
                }
            }
        }
        1 => {
            // Logical shift
            if direction == 1 {
                for _ in 0..count {
                    last_out = result & msb != 0;
                    result = (result << 1) & mask;
                }
            } else {
                for _ in 0..count {
                    last_out = result & 1 != 0;
                    result = (result >> 1) & mask;
                }
            }
        }
        2 => {
            // Rotate through X
            let mut x = x_in;
            if direction == 1 {
                for _ in 0..count {
                    last_out = result & msb != 0;
                    result = ((result << 1) | u32::from(x)) & mask;
                    x = last_out;
                }
            } else {
                for _ in 0..count {
                    last_out = result & 1 != 0;
                    result = ((result >> 1) | (u32::from(x) << (bits - 1))) & mask;
                    x = last_out;
                }
            }
        }
        _ => {
            // Circular rotate
            if count == 0 {
                last_out = if direction == 1 { false } else { result & 1 != 0 };
            } else if direction == 1 {
                let n = count % bits;
                if n != 0 {
                    result = ((result << n) | (result >> (bits - n))) & mask;
                }
                last_out = result & 1 != 0; // C = new LSB
            } else {
                let n = count % bits;
                if n != 0 {
                    result = ((result >> n) | (result << (bits - n))) & mask;
                }
                last_out = result & msb != 0; // C = new MSB
            }
        }
    }

    result &= mask;
    let n_f = result & msb != 0;
    let z_f = result == 0;
    let mut c_f = last_out;
    // X is unchanged for ROL/ROR (rotate-without-extend); X = C for
    // every other shift/rotate family.
    let x_f = if shift_type == 3 {
        sr_for_rotate_x & 0x10 != 0
    } else {
        c_f
    };
    if shift_type == 3 && count == 0 {
        c_f = false; // no rotation occurred -> C cleared
    }

    (result, n_f, z_f, v_flag, c_f, x_f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::M68kSimulator;

    fn sim_with(bytes: &[u8]) -> M68kSimulator {
        let mut sim = M68kSimulator::new(65536);
        sim.mem.load_bytes(0, bytes);
        sim.pc = 0;
        sim
    }

    #[test]
    fn move_l_imm_to_d0() {
        let mut sim = sim_with(&[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]); // MOVE.L #42,D0
        let mnemonic = decode_and_execute(&mut sim).unwrap();
        assert_eq!(mnemonic, "MOVE");
        assert_eq!(sim.d[0], 42);
    }

    #[test]
    fn moveq_sign_extends() {
        let mut sim = sim_with(&[0x70, 0xFF]); // MOVEQ #-1, D0
        decode_and_execute(&mut sim).unwrap();
        assert_eq!(sim.d[0], 0xFFFF_FFFF);
    }

    #[test]
    fn trap_15_halts() {
        let mut sim = sim_with(&[0x4E, 0x4F]);
        decode_and_execute(&mut sim).unwrap();
        assert!(sim.halted);
    }

    #[test]
    fn trap_other_records_in_d7() {
        let mut sim = sim_with(&[0x4E, 0x41]); // TRAP #1
        decode_and_execute(&mut sim).unwrap();
        assert_eq!(sim.d[7], 1);
        assert!(!sim.halted);
    }

    #[test]
    fn add_l_dn_dn_sets_flags() {
        let mut sim = M68kSimulator::new(65536);
        sim.d[0] = 5;
        sim.d[1] = 3;
        // ADD.L D1,D0 : 1101 000 0 10 000 001 = 0xD081
        sim.mem.load_bytes(0, &[0xD0, 0x81]);
        sim.pc = 0;
        decode_and_execute(&mut sim).unwrap();
        assert_eq!(sim.d[0], 8);
    }

    #[test]
    fn bra_branches_unconditionally() {
        let mut sim = sim_with(&[0x60, 0x02]); // BRA #2
        let pc_before = sim.pc;
        decode_and_execute(&mut sim).unwrap();
        assert_eq!(sim.pc, pc_before + 2 + 2);
    }

    #[test]
    fn line0_is_deferred() {
        let mut sim = sim_with(&[0x00, 0x00, 0x00, 0x01]); // ORI-family opword
        assert!(decode_and_execute(&mut sim).is_err());
    }

    #[test]
    fn shift_asl_sets_carry_and_overflow() {
        // ASL.B #1,D0 with D0=0x40 (bit6 set) -> overflow (sign changes)
        let (result, n, z, v, c, _x) =
            shift_value(0x40, 1, 1, 0, 8, 0xFF, 0x80, false, 0);
        assert_eq!(result, 0x80);
        assert!(n);
        assert!(!z);
        assert!(v);
        assert!(!c);
    }

    #[test]
    fn rotate_full_circle_is_identity() {
        // ROL.L by 32 positions is a no-op (bits % 32 == 0).
        let (result, _n, _z, _v, c, _x) =
            shift_value(0x1234_5678, 32, 1, 3, 32, 0xFFFF_FFFF, 0x8000_0000, false, 0);
        assert_eq!(result, 0x1234_5678);
        assert!(!c, "count==0 after modulo clears carry");
    }
}

//! A decoder for the 64-bit x86 subset the in-repo `x86_64-encoder` emits.
//!
//! This is **not** a full x86 decoder — it covers the instructions our backend
//! generates (and grows as the backend grows). Anything else is a clean
//! [`Trap::DecodeError`] (fail-closed). All operations are 64-bit operand size
//! (REX.W), the only width the backend uses for register values, plus the byte
//! loads/stores the byte-tape/array element headers use.
//!
//! ## Instruction encoding (the parts we need)
//!
//! ```text
//!   [REX] opcode [ModRM [SIB] [disp]] [imm]
//! ```
//! - **REX** (`0x40..=0x4F`): `0100 WRXB`. `W`=64-bit operand, `R`=ModRM.reg
//!   bit 3, `X`=SIB.index bit 3, `B`=ModRM.rm / opcode-reg bit 3.
//! - **ModRM**: `mod(2) reg(3) rm(3)`. `mod==11` ⇒ rm is a register; otherwise a
//!   memory operand (`[base + disp]`, with `rm==101 && mod==00` meaning RIP-rel,
//!   `rm==100` meaning a SIB byte follows).

use crate::state::Reg;
use crate::trap::Trap;

/// A decoded source/destination location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// A register.
    Reg(Reg),
    /// `[base + index*scale + disp]`. `index == None` for no index; `rip` true
    /// for RIP-relative (`disp` is relative to the *next* instruction).
    Mem { base: Option<Reg>, index: Option<Reg>, scale: u8, disp: i64, rip: bool },
}

/// A decoded instruction (the subset we execute — see [`super::execute`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instr {
    Push(Reg),
    Pop(Reg),
    /// `mov dst, src` — 64-bit. Exactly one of dst/src is memory (or both reg).
    Mov { dst: Operand, src: Operand },
    /// `mov reg, imm` (imm sign-extended to 64).
    MovImm { dst: Operand, imm: i64 },
    /// `movzx reg64, byte [mem]`.
    Movzx { dst: Reg, src: Operand },
    /// `mov r/m8, r8` (`0x88`) — store the low byte of `src` into `dst` (the
    /// byte-tape `store_byte`; e.g. Brainfuck cells). A register `dst` keeps its
    /// upper bytes.
    MovByteStore { dst: Operand, src: Reg },
    /// `lea reg, mem`.
    Lea { dst: Reg, src: Operand },
    /// Integer ALU `op dst, src` (dst is reg or mem, src reg/mem/imm).
    Alu { op: AluOp, dst: Operand, src: Operand },
    /// `op dst, imm` (imm sign-extended).
    AluImm { op: AluOp, dst: Operand, imm: i64 },
    /// `shl/shr/sar dst, imm8`.
    Shift { op: ShiftOp, dst: Operand, amount: u8 },
    /// `imul reg, rm` (two-operand).
    Imul { dst: Reg, src: Operand },
    /// `not r/m64` — bitwise complement (group-3 `0xF7 /2`). No flags.
    Not { dst: Operand },
    /// `neg r/m64` — two's-complement negate (group-3 `0xF7 /3`). Sets flags
    /// as `sub 0, dst`.
    Neg { dst: Operand },
    /// `div`/`idiv r/m64` (group-3 `0xF7 /6` unsigned, `/7` signed) — divides
    /// the `rdx:rax` pair by the operand: quotient → `rax`, remainder → `rdx`.
    Div { divisor: Operand, signed: bool },
    /// `cqo` (`0x48 0x99`) — sign-extend `rax` into `rdx:rax` (the standard
    /// `idiv` preamble).
    Cqo,
    /// `jmp rel` (absolute target already resolved from rip+rel).
    Jmp(i64),
    /// `jcc rel` — relative displacement; the condition nibble is `cond`.
    Jcc { cond: u8, rel: i64 },
    /// `call rel` — relative displacement (resolved to a target at execute time).
    Call(i64),
    Ret,
    /// `ud2` — illegal-instruction trap.
    Ud2,
    /// `nop`.
    Nop,
    // ── SSE2 scalar double (LANG-FULL E3 — ALGOL `real`) ──────────────────────
    /// `movsd xmm, m64` — load a double into the low lane of an XMM register.
    MovsdLoad { xmm: u8, src: Operand },
    /// `movsd m64, xmm` — store the low double of an XMM register.
    MovsdStore { dst: Operand, xmm: u8 },
    /// `movsd xmm, xmm` — copy the low double (register form).
    MovsdRr { dst: u8, src: u8 },
    /// `addsd`/`subsd`/`mulsd`/`divsd xmm, xmm/m64`.
    SseArith { op: SseOp, dst: u8, src: XmmRm },
    /// `ucomisd xmm, xmm/m64` — unordered compare, sets CF/ZF/PF (OF/SF/AF=0).
    Ucomisd { a: u8, b: XmmRm },
    /// `setcc r/m8` — set the low byte to 1 if the condition holds, else 0.
    Setcc { cond: u8, dst: Operand },
    // ── LANG-FULL E8: int ⇄ real conversions ──────────────────────────────
    /// `cvtsi2sd xmm, r64` — signed 64-bit integer → double (IIR `int_to_real`).
    Cvtsi2sd { xmm: u8, gpr: Reg },
    /// `cvttsd2si r64, xmm` — double → signed 64-bit integer, truncating toward
    /// zero (IIR `real_to_int_trunc`; also the tail of `real_to_int_floor`).
    Cvttsd2si { gpr: Reg, xmm: u8 },
    /// `roundsd xmm, xmm, imm8` — round a double under `mode` (`mode & 3`:
    /// 0 = nearest-even, 1 = −∞/floor, 2 = +∞/ceil, 3 = toward-zero/trunc).
    /// `mode == 1` composes with `cvttsd2si` for IIR `real_to_int_floor`.
    Roundsd { dst: u8, src: u8, mode: u8 },
}

/// SSE2 scalar-double arithmetic ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SseOp { Add, Sub, Mul, Div }

/// An XMM source: either a register (by number 0..15) or a memory operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum XmmRm { Xmm(u8), Mem(Operand) }

/// Integer ALU ops we decode (all set flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum AluOp { Add, Sub, Cmp, And, Or, Xor, Test }

/// Shift ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ShiftOp { Shl, Shr, Sar }

/// One decoded instruction plus its byte length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub instr: Instr,
    pub len: usize,
}

/// Decode the instruction at `code[off..]`. `off` is the byte offset *within the
/// code region*; RIP-relative displacements are resolved against `off + len`.
pub fn decode(code: &[u8], off: usize) -> Result<Decoded, Trap> {
    let mut p = off;
    let at = |p: usize| -> Result<u8, Trap> {
        code.get(p).copied().ok_or(Trap::DecodeError { offset: p as u64, opcode: 0 })
    };

    // ── Mandatory SSE prefix (F2/F3/66) — comes *before* REX ─────────────────
    // SSE2 scalar ops are `prefix 0F op`: `F2` = scalar-double, `66` = packed/
    // compare-double. We only need to remember which one preceded the `0F`.
    let mut sse_prefix: Option<u8> = None;
    let pb = at(p)?;
    if pb == 0xF2 || pb == 0xF3 || pb == 0x66 {
        sse_prefix = Some(pb);
        p += 1;
    }

    // ── REX prefix ──────────────────────────────────────────────────────────
    let mut rex_w = false;
    let mut rex_r = false;
    let mut rex_x = false;
    let mut rex_b = false;
    let b0 = at(p)?;
    if (0x40..=0x4F).contains(&b0) {
        rex_w = b0 & 0x8 != 0;
        rex_r = b0 & 0x4 != 0;
        rex_x = b0 & 0x2 != 0;
        rex_b = b0 & 0x1 != 0;
        p += 1;
    }
    let _ = rex_w; // every op we handle is 64-bit; tracked for future widths
    let _ = rex_x;

    let op = at(p)?;
    p += 1;

    // push/pop r64 (REX.B extends the register).
    if (0x50..=0x57).contains(&op) {
        let r = Reg::from_index((op - 0x50) | ((rex_b as u8) << 3));
        return Ok(Decoded { instr: Instr::Push(r), len: p - off });
    }
    if (0x58..=0x5F).contains(&op) {
        let r = Reg::from_index((op - 0x58) | ((rex_b as u8) << 3));
        return Ok(Decoded { instr: Instr::Pop(r), len: p - off });
    }

    // movabs r64, imm64  (0xB8+rd with REX.W) — loads a full 64-bit immediate,
    // e.g. an `f64` constant's bit pattern before it is `movsd`'d into an XMM.
    if rex_w && (0xB8..=0xBF).contains(&op) {
        let r = Reg::from_index((op - 0xB8) | ((rex_b as u8) << 3));
        let b = code.get(p..p + 8).ok_or(Trap::DecodeError { offset: p as u64, opcode: op })?;
        let imm = i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
        p += 8;
        return Ok(Decoded { instr: Instr::MovImm { dst: Operand::Reg(r), imm }, len: p - off });
    }

    match op {
        0xC3 => return Ok(Decoded { instr: Instr::Ret, len: p - off }),
        0x90 => return Ok(Decoded { instr: Instr::Nop, len: p - off }),
        // cqo — sign-extend rax into rdx:rax (REX.W 0x99). No operands.
        0x99 => return Ok(Decoded { instr: Instr::Cqo, len: p - off }),
        // jmp rel8 / rel32
        0xEB => { let r = at(p)? as i8 as i64; p += 1; return Ok(Decoded { instr: Instr::Jmp(rel_target(p, r)), len: p - off }); }
        0xE9 => { let r = read_i32(code, p)? as i64; p += 4; return Ok(Decoded { instr: Instr::Jmp(rel_target(p, r)), len: p - off }); }
        0xE8 => { let r = read_i32(code, p)? as i64; p += 4; return Ok(Decoded { instr: Instr::Call(rel_target(p, r)), len: p - off }); }
        // jcc rel8 (0x70..0x7F)
        0x70..=0x7F => { let r = at(p)? as i8 as i64; p += 1; return Ok(Decoded { instr: Instr::Jcc { cond: op - 0x70, rel: rel_target(p, r) }, len: p - off }); }
        _ => {}
    }

    // Two-byte 0F opcodes.
    if op == 0x0F {
        let op2 = at(p)?; p += 1;

        // SSE2 scalar double (a mandatory F2/66 prefix preceded the 0F).
        if let Some(pfx) = sse_prefix {
            // Three-byte SSE4.1 form `66 0F 3A <op3> /r ib` — here only ROUNDSD
            // (LANG-FULL E8 `real_to_int_floor`). `op2 == 0x3A` is the opcode-map
            // escape; the real opcode is the next byte, and an `imm8` trails the
            // ModRM (the rounding mode).
            if pfx == 0x66 && op2 == 0x3A {
                let op3 = at(p)?; p += 1;
                let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
                let imm8 = code.get(np).copied()
                    .ok_or(Trap::DecodeError { offset: np as u64, opcode: op3 })?;
                let end = np + 1;
                let src = match m.rm {
                    Operand::Reg(r) => r as u8,
                    _ => return Err(Trap::DecodeError { offset: p as u64, opcode: op3 }),
                };
                let instr = match op3 {
                    0x0B => Instr::Roundsd { dst: m.reg, src, mode: imm8 },
                    other => return Err(Trap::DecodeError { offset: (p - 1) as u64, opcode: other }),
                };
                return Ok(Decoded { instr, len: end - off });
            }
            let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
            let xmm = m.reg; // ModRM.reg is the XMM register number
            let rm_xmm = match m.rm {
                Operand::Reg(r) => XmmRm::Xmm(r as u8),
                mem => XmmRm::Mem(mem),
            };
            let instr = match (pfx, op2) {
                // F2 0F 2A — cvtsi2sd xmm, r/m64 (the rm is a GPR, not an XMM).
                (0xF2, 0x2A) => match m.rm {
                    Operand::Reg(gpr) => Instr::Cvtsi2sd { xmm, gpr },
                    _ => return Err(Trap::DecodeError { offset: p as u64, opcode: op2 }),
                },
                // F2 0F 2C — cvttsd2si r64, xmm/m64 (ModRM.reg is the GPR dest).
                (0xF2, 0x2C) => match rm_xmm {
                    XmmRm::Xmm(s) => Instr::Cvttsd2si { gpr: reg_of(m.reg), xmm: s },
                    XmmRm::Mem(_) => return Err(Trap::DecodeError { offset: p as u64, opcode: op2 }),
                },
                // F2 0F 10 — movsd xmm, xmm/m64 (load)
                (0xF2, 0x10) => match rm_xmm {
                    XmmRm::Xmm(s) => Instr::MovsdRr { dst: xmm, src: s },
                    XmmRm::Mem(mem) => Instr::MovsdLoad { xmm, src: mem },
                },
                // F2 0F 11 — movsd xmm/m64, xmm (store)
                (0xF2, 0x11) => match rm_xmm {
                    XmmRm::Xmm(s) => Instr::MovsdRr { dst: s, src: xmm },
                    XmmRm::Mem(mem) => Instr::MovsdStore { dst: mem, xmm },
                },
                (0xF2, 0x58) => Instr::SseArith { op: SseOp::Add, dst: xmm, src: rm_xmm },
                (0xF2, 0x5C) => Instr::SseArith { op: SseOp::Sub, dst: xmm, src: rm_xmm },
                (0xF2, 0x59) => Instr::SseArith { op: SseOp::Mul, dst: xmm, src: rm_xmm },
                (0xF2, 0x5E) => Instr::SseArith { op: SseOp::Div, dst: xmm, src: rm_xmm },
                // 66 0F 2E — ucomisd xmm, xmm/m64 (also 0F 2F comisd; same here)
                (0x66, 0x2E) | (0x66, 0x2F) => Instr::Ucomisd { a: xmm, b: rm_xmm },
                _ => return Err(Trap::DecodeError { offset: (p - 1) as u64, opcode: op2 }),
            };
            return Ok(Decoded { instr, len: np - off });
        }

        match op2 {
            0x0B => return Ok(Decoded { instr: Instr::Ud2, len: p - off }),
            0xAF => { // imul r64, r/m64
                let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; p = np;
                let dst = reg_of(m.reg);
                return Ok(Decoded { instr: Instr::Imul { dst, src: m.rm }, len: p - off });
            }
            0xB6 => { // movzx r64, r/m8
                let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; p = np;
                return Ok(Decoded { instr: Instr::Movzx { dst: reg_of(m.reg), src: m.rm }, len: p - off });
            }
            0x80..=0x8F => { // jcc rel32
                let r = read_i32(code, p)? as i64; p += 4;
                return Ok(Decoded { instr: Instr::Jcc { cond: op2 - 0x80, rel: rel_target(p, r) }, len: p - off });
            }
            0x90..=0x9F => { // setcc r/m8
                let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; p = np;
                return Ok(Decoded { instr: Instr::Setcc { cond: op2 - 0x90, dst: m.rm }, len: p - off });
            }
            other => return Err(Trap::DecodeError { offset: (p - 1) as u64, opcode: other }),
        }
    }

    // ModRM-based one-byte opcodes.
    match op {
        // mov r/m8, r8 — store the low byte of the reg-field register into r/m.
        0x88 => { let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; Ok(Decoded { instr: Instr::MovByteStore { dst: m.rm, src: reg_of(m.reg) }, len: np - off }) }
        0x89 => { let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; Ok(Decoded { instr: Instr::Mov { dst: m.rm, src: Operand::Reg(reg_of(m.reg)) }, len: np - off }) }
        0x8B => { let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; Ok(Decoded { instr: Instr::Mov { dst: Operand::Reg(reg_of(m.reg)), src: m.rm }, len: np - off }) }
        0x8D => { let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?; Ok(Decoded { instr: Instr::Lea { dst: reg_of(m.reg), src: m.rm }, len: np - off }) }
        0x01 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Add, true),
        0x03 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Add, false),
        0x29 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Sub, true),
        0x2B => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Sub, false),
        0x39 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Cmp, true),
        0x3B => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Cmp, false),
        0x21 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::And, true),
        0x09 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Or, true),
        0x31 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Xor, true),
        0x85 => alu_rr(code, p, rex_r, rex_x, rex_b, off, AluOp::Test, true),
        // group1: op r/m64, imm32 (0x81) or imm8 (0x83); /reg selects the op.
        0x81 | 0x83 => {
            let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
            let mut p2 = np;
            let imm = if op == 0x81 { let v = read_i32(code, p2)? as i64; p2 += 4; v }
                      else { let v = code.get(p2).copied().ok_or(Trap::DecodeError { offset: p2 as u64, opcode: 0 })? as i8 as i64; p2 += 1; v };
            let aop = match m.reg & 0x7 {
                0 => AluOp::Add, 1 => AluOp::Or, 4 => AluOp::And, 5 => AluOp::Sub, 7 => AluOp::Cmp, 6 => AluOp::Xor,
                other => return Err(Trap::DecodeError { offset: off as u64, opcode: 0x80 | other }),
            };
            Ok(Decoded { instr: Instr::AluImm { op: aop, dst: m.rm, imm }, len: p2 - off })
        }
        // mov r/m64, imm32  (0xC7 /0)
        0xC7 => {
            let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
            let imm = read_i32(code, np)? as i64;
            Ok(Decoded { instr: Instr::MovImm { dst: m.rm, imm }, len: np + 4 - off })
        }
        // shift r/m64, imm8 (0xC1 /4=shl /5=shr /7=sar)
        0xC1 => {
            let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
            let amt = code.get(np).copied().ok_or(Trap::DecodeError { offset: np as u64, opcode: 0 })?;
            let sop = match m.reg & 0x7 { 4 => ShiftOp::Shl, 5 => ShiftOp::Shr, 7 => ShiftOp::Sar,
                other => return Err(Trap::DecodeError { offset: off as u64, opcode: 0xC0 | other }) };
            Ok(Decoded { instr: Instr::Shift { op: sop, dst: m.rm, amount: amt }, len: np + 1 - off })
        }
        // group-3 r/m64 — /reg selects: 2=not, 3=neg, 6=div, 7=idiv.
        // (/0,/1=test imm, /4=mul, /5=imul aren't emitted by the backend.)
        0xF7 => {
            let (m, np) = modrm(code, p, rex_r, rex_x, rex_b)?;
            let instr = match m.reg & 0x7 {
                2 => Instr::Not { dst: m.rm },
                3 => Instr::Neg { dst: m.rm },
                6 => Instr::Div { divisor: m.rm, signed: false },
                7 => Instr::Div { divisor: m.rm, signed: true },
                // /0,/1 (test imm) /4 (mul) /5 (imul) — not emitted by the backend.
                _ => return Err(Trap::DecodeError { offset: off as u64, opcode: 0xF7 }),
            };
            Ok(Decoded { instr, len: np - off })
        }
        other => Err(Trap::DecodeError { offset: (p - 1) as u64, opcode: other }),
    }
}

#[allow(clippy::too_many_arguments)]
fn alu_rr(code: &[u8], p: usize, r: bool, x: bool, b: bool, off: usize, op: AluOp, rm_is_dst: bool) -> Result<Decoded, Trap> {
    let (m, np) = modrm(code, p, r, x, b)?;
    let (dst, src) = if rm_is_dst { (m.rm, Operand::Reg(reg_of(m.reg))) }
                     else { (Operand::Reg(reg_of(m.reg)), m.rm) };
    Ok(Decoded { instr: Instr::Alu { op, dst, src }, len: np - off })
}

/// Resolve a relative branch to an absolute *code-region* offset. `cursor` is
/// the byte offset just past the instruction (= rip-relative base), so the
/// target is `cursor + rel`.
fn rel_target(cursor: usize, rel: i64) -> i64 { cursor as i64 + rel }

fn reg_of(i: u8) -> Reg { Reg::from_index(i) }

struct ModRm { reg: u8, rm: Operand }

/// Decode a ModRM byte (and any SIB/displacement). Returns the parsed operand
/// info and the new cursor position.
fn modrm(code: &[u8], p: usize, rex_r: bool, rex_x: bool, rex_b: bool) -> Result<(ModRm, usize), Trap> {
    let byte = code.get(p).copied().ok_or(Trap::DecodeError { offset: p as u64, opcode: 0 })?;
    let mut cur = p + 1;
    let md = byte >> 6;
    let reg = (byte >> 3 & 0x7) | ((rex_r as u8) << 3);
    let rm_field = byte & 0x7;

    if md == 0b11 {
        let r = Reg::from_index(rm_field | ((rex_b as u8) << 3));
        return Ok((ModRm { reg, rm: Operand::Reg(r) }, cur));
    }

    // Memory operand.
    let mut base: Option<Reg> = None;
    let mut index: Option<Reg> = None;
    let mut scale = 1u8;
    let mut rip = false;

    if rm_field == 0b100 {
        // SIB byte.
        let sib = code.get(cur).copied().ok_or(Trap::DecodeError { offset: cur as u64, opcode: 0 })?;
        cur += 1;
        scale = 1 << (sib >> 6);
        let idx = (sib >> 3 & 0x7) | ((rex_x as u8) << 3);
        if idx != 0b100 { index = Some(Reg::from_index(idx)); } // index==100 (no REX.X) → none
        let bas = (sib & 0x7) | ((rex_b as u8) << 3);
        // base==101 with mod==00 means disp32 with no base; otherwise a register.
        if (sib & 0x7) == 0b101 && md == 0b00 { base = None; } else { base = Some(Reg::from_index(bas)); }
    } else if rm_field == 0b101 && md == 0b00 {
        rip = true; // RIP-relative
    } else {
        base = Some(Reg::from_index(rm_field | ((rex_b as u8) << 3)));
    }

    let disp: i64 = match md {
        0b00 => if rip || (rm_field == 0b100 && base.is_none()) { let d = read_i32(code, cur)? as i64; cur += 4; d } else { 0 },
        0b01 => { let d = code.get(cur).copied().ok_or(Trap::DecodeError { offset: cur as u64, opcode: 0 })? as i8 as i64; cur += 1; d }
        0b10 => { let d = read_i32(code, cur)? as i64; cur += 4; d }
        _ => 0,
    };

    Ok((ModRm { reg, rm: Operand::Mem { base, index, scale, disp, rip } }, cur))
}

fn read_i32(code: &[u8], p: usize) -> Result<i32, Trap> {
    let b = code.get(p..p + 4).ok_or(Trap::DecodeError { offset: p as u64, opcode: 0 })?;
    Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact prologue/const/ret bytes the x86_64-backend emits for
    // `const_u64 v=42; ret_u64 v` (captured from compile_function_with_relocs).
    const MIN_FN: &[u8] = &[
        0x55,                                     // push rbp
        0x48, 0x89, 0xE5,                         // mov rbp, rsp
        0x48, 0x81, 0xEC, 0x10, 0x00, 0x00, 0x00, // sub rsp, 0x10
        0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00, // mov rax, 42
        0x48, 0x89, 0x85, 0xF8, 0xFF, 0xFF, 0xFF, // mov [rbp-8], rax
        0x48, 0x8B, 0x85, 0xF8, 0xFF, 0xFF, 0xFF, // mov rax, [rbp-8]
        0x48, 0x89, 0xEC,                         // mov rsp, rbp
        0x5D,                                     // pop rbp
        0xC3,                                     // ret
    ];

    fn decode_all(code: &[u8]) -> Vec<Instr> {
        let mut out = Vec::new();
        let mut off = 0;
        while off < code.len() {
            let d = decode(code, off).unwrap_or_else(|e| panic!("decode at {off}: {e}"));
            out.push(d.instr.clone());
            off += d.len;
        }
        out
    }

    #[test]
    fn decodes_the_backend_prologue_const_ret() {
        let instrs = decode_all(MIN_FN);
        assert_eq!(instrs[0], Instr::Push(Reg::Rbp));
        assert_eq!(instrs[1], Instr::Mov { dst: Operand::Reg(Reg::Rbp), src: Operand::Reg(Reg::Rsp) });
        assert_eq!(instrs[2], Instr::AluImm { op: AluOp::Sub, dst: Operand::Reg(Reg::Rsp), imm: 0x10 });
        assert_eq!(instrs[3], Instr::MovImm { dst: Operand::Reg(Reg::Rax), imm: 42 });
        // mov [rbp-8], rax
        assert_eq!(instrs[4], Instr::Mov {
            dst: Operand::Mem { base: Some(Reg::Rbp), index: None, scale: 1, disp: -8, rip: false },
            src: Operand::Reg(Reg::Rax) });
        assert_eq!(instrs.last(), Some(&Instr::Ret));
    }

    #[test]
    fn group3_and_cqo_decode() {
        // The exact bytes the x86_64-encoder emits (REX.W 0x48 prefix, mod=11
        // register-direct, reg field selects the group-3 op).
        // not rax  = 48 F7 D0 (/2, rm=rax)
        assert_eq!(decode(&[0x48, 0xF7, 0xD0], 0).unwrap().instr,
                   Instr::Not { dst: Operand::Reg(Reg::Rax) });
        // neg rax  = 48 F7 D8 (/3)
        assert_eq!(decode(&[0x48, 0xF7, 0xD8], 0).unwrap().instr,
                   Instr::Neg { dst: Operand::Reg(Reg::Rax) });
        // div rcx  = 48 F7 F1 (/6, rm=rcx, unsigned)
        assert_eq!(decode(&[0x48, 0xF7, 0xF1], 0).unwrap().instr,
                   Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: false });
        // idiv rcx = 48 F7 F9 (/7, signed)
        assert_eq!(decode(&[0x48, 0xF7, 0xF9], 0).unwrap().instr,
                   Instr::Div { divisor: Operand::Reg(Reg::Rcx), signed: true });
        // cqo      = 48 99
        assert_eq!(decode(&[0x48, 0x99], 0).unwrap().instr, Instr::Cqo);
    }

    #[test]
    fn byte_store_decodes() {
        // The encoder emits `REX(0x40) 88 ModRM(mod=00, reg=src, rm=base)`.
        // 0x40 0x88 0x08 = mov byte [rax], cl  (reg=cl=1, rm=rax=0, mod=00).
        assert_eq!(decode(&[0x40, 0x88, 0x08], 0).unwrap().instr,
                   Instr::MovByteStore {
                       dst: Operand::Mem { base: Some(Reg::Rax), index: None, scale: 1, disp: 0, rip: false },
                       src: Reg::Rcx,
                   });
    }

    #[test]
    fn ud2_and_jcc_decode() {
        assert_eq!(decode(&[0x0F, 0x0B], 0).unwrap().instr, Instr::Ud2);
        // jb rel8 +5  (0x72 0x05)
        let d = decode(&[0x72, 0x05], 0).unwrap();
        assert_eq!(d.instr, Instr::Jcc { cond: 0x2, rel: 7 }); // 2 (len) + 5
    }

    #[test]
    fn sse2_and_movabs_decode() {
        // movabs rax, 0x4004000000000000  (2.5)  — 48 B8 <imm64>
        let d = decode(&[0x48, 0xB8, 0,0,0,0,0,0,0x04,0x40], 0).unwrap();
        assert_eq!(d.instr, Instr::MovImm { dst: Operand::Reg(Reg::Rax), imm: 0x4004_0000_0000_0000u64 as i64 });
        assert_eq!(d.len, 10);
        // movsd xmm0, [rbp-8]  — F2 0F 10 85 F8 FF FF FF
        let d = decode(&[0xF2, 0x0F, 0x10, 0x85, 0xF8, 0xFF, 0xFF, 0xFF], 0).unwrap();
        assert_eq!(d.instr, Instr::MovsdLoad { xmm: 0, src: Operand::Mem { base: Some(Reg::Rbp), index: None, scale: 1, disp: -8, rip: false } });
        // mulsd xmm0, xmm1  — F2 0F 59 C1
        assert_eq!(decode(&[0xF2, 0x0F, 0x59, 0xC1], 0).unwrap().instr,
                   Instr::SseArith { op: SseOp::Mul, dst: 0, src: XmmRm::Xmm(1) });
        // ucomisd xmm0, xmm1  — 66 0F 2E C1
        assert_eq!(decode(&[0x66, 0x0F, 0x2E, 0xC1], 0).unwrap().instr,
                   Instr::Ucomisd { a: 0, b: XmmRm::Xmm(1) });
        // sete al  — 40 0F 94 C0   (cond 4 = E)
        assert_eq!(decode(&[0x40, 0x0F, 0x94, 0xC0], 0).unwrap().instr,
                   Instr::Setcc { cond: 0x4, dst: Operand::Reg(Reg::Rax) });
    }

    /// LANG-FULL E8: the three conversion opcodes decode — including the
    /// three-byte `66 0F 3A 0B` form with a trailing `imm8`.
    #[test]
    fn e8_conversions_decode() {
        // cvtsi2sd xmm0, rax  — F2 48 0F 2A C0  (rm is a GPR, not an XMM)
        let d = decode(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0], 0).unwrap();
        assert_eq!(d.instr, Instr::Cvtsi2sd { xmm: 0, gpr: Reg::Rax });
        assert_eq!(d.len, 5);
        // cvttsd2si rax, xmm0 — F2 48 0F 2C C0  (ModRM.reg is the GPR dest)
        let d = decode(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0], 0).unwrap();
        assert_eq!(d.instr, Instr::Cvttsd2si { gpr: Reg::Rax, xmm: 0 });
        assert_eq!(d.len, 5);
        // roundsd xmm0, xmm0, 1 — 66 0F 3A 0B C0 01  (3-byte opcode + imm8)
        let d = decode(&[0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x01], 0).unwrap();
        assert_eq!(d.instr, Instr::Roundsd { dst: 0, src: 0, mode: 1 });
        assert_eq!(d.len, 6);
    }
}

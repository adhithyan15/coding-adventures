//! LANG83 PR6a — shortest round-trip float output as in-module WASM functions.
//!
//! `call_builtin "write_f64_py", x` / `"write_f64_js", x` lower to
//!
//! ```text
//! local.get $x        ;; f64
//! i32.const 0 | 1     ;; layout: 0 = Python repr, 1 = JavaScript toString
//! call $__iir_write_f64
//! ```
//!
//! and this file builds `$__iir_write_f64` and the functions it calls. They
//! live **inside the module**, like `$__str_eq` and `$__ensure_capacity`: the
//! host only has to provide `env.putchar`, which every output program already
//! imports. Formatting a float is pure computation, so it is not the host's
//! job. The module gets no `__write_f64` import, and no shared helper library
//! is involved.
//!
//! # The algorithm
//!
//! The digits are the shortest decimal that reads back as exactly `x`, closest
//! on a length tie and even on an exact tie. This is Burger–Dybvig free-format
//! generation over exact big integers, the same core that `twig_runtime.c`
//! (`twig_f64_digits`, the readable C reference) and `iir-to-llvm`'s
//! `lang83_f64.rs` carry. The functions below follow the C one for one:
//!
//! | # | function | signature | job |
//! |---|----------|-----------|-----|
//! | 0 | `bn_set` | `(a: i32, v: i64)` | `a = v` |
//! | 1 | `bn_mul_small` | `(a: i32, m: i64)` | `a *= m` (`m < 2^32`) |
//! | 2 | `bn_mul_pow2` | `(a: i32, n: i64)` | `a *= 2^n`, 31 bits per pass |
//! | 3 | `bn_mul_pow10` | `(a: i32, n: i64)` | `a *= 10^n`, 10^9 per pass |
//! | 4 | `bn_add` | `(d: i32, a: i32, b: i32)` | `d = a + b` |
//! | 5 | `bn_sub` | `(a: i32, b: i32)` | `a -= b` |
//! | 6 | `bn_cmp` | `(a: i32, b: i32) -> i32` | `-1 / 0 / 1` |
//! | 7 | `f64_digits` | `(mag: i64) -> i32` | the digits, `k` stored at `K` |
//! | 8 | `put_digits` | `(from: i32, to: i32)` | `putchar('0' + D[i])` |
//! | 9 | `put_repeat` | `(c: i32, n: i64)` | `putchar(c)` `n` times |
//! | 10 | `write_f64` | `(x: f64, js: i32)` | specials, sign, layout |
//!
//! # Memory
//!
//! A big integer is 40 base-2^32 limbs, each in an 8-byte word (so `limb * m +
//! carry` cannot overflow an i64), least significant first: 320 bytes. The five
//! big integers, the 32-byte digit buffer and the 8-byte `k` slot live in
//! [`F64_SCRATCH_BYTES`] bytes reserved at the end of the module's static data
//! segment, before the bump allocator's start. No string literal, array or
//! runtime string can be placed there, and the region costs nothing at run time.
//!
//! ```text
//! base+0     r      base+320  s      base+640  m+     base+960  m-
//! base+1280  t      base+1600 digits (32 bytes)        base+1632 k (i64)
//! ```
//!
//! Every loop is counted. Each limb loop runs exactly 40 times. The digit loop
//! stops after 31 digits even though no double needs more than 17. No address
//! is computed from the input value except digit indices, which are bounded by
//! that count, and the buffer is 32 bytes.

use wasm_types::{FuncType, FunctionBody, ValueType};

/// Bytes of linear memory the float writer reserves (five 320-byte big
/// integers, a 32-byte digit buffer, an 8-byte `k`, padded to 64).
pub(crate) const F64_SCRATCH_BYTES: usize = 1664;

/// How many in-module functions [`build_f64_helpers`] appends.
#[cfg(test)]
pub(crate) const F64_HELPER_COUNT: u32 = 11;

/// Index of `$__iir_write_f64` among the helpers (it is the last one).
pub(crate) const WRITE_F64_OFFSET: u32 = 10;

// Offsets of the scratch areas from the region's base.
const R: i32 = 0;
const S: i32 = 320;
const MP: i32 = 640;
const MM: i32 = 960;
const T: i32 = 1280;
const DIGITS: i32 = 1600;
const K: i32 = 1632;

/// Number of 32-bit limbs per big integer.
const LIMBS: i32 = 40;

// The WASM opcodes this file emits (WebAssembly core spec, §5.4).
const BLOCK: u8 = 0x02;
const LOOP: u8 = 0x03;
const IF: u8 = 0x04;
const ELSE: u8 = 0x05;
const END: u8 = 0x0B;
const BR: u8 = 0x0C;
const BR_IF: u8 = 0x0D;
const RETURN: u8 = 0x0F;
const CALL: u8 = 0x10;
const SELECT: u8 = 0x1B;
const LOCAL_GET: u8 = 0x20;
const LOCAL_SET: u8 = 0x21;
const I64_LOAD: u8 = 0x29;
const I32_LOAD8_U: u8 = 0x2D;
const I64_STORE: u8 = 0x37;
const I32_STORE8: u8 = 0x3A;
const I32_CONST: u8 = 0x41;
const I64_CONST: u8 = 0x42;
const I32_EQZ: u8 = 0x45;
const I32_LT_S: u8 = 0x48;
const I32_GT_S: u8 = 0x4A;
const I32_LE_S: u8 = 0x4C;
const I32_GE_S: u8 = 0x4E;
const I64_EQZ: u8 = 0x50;
const I64_EQ: u8 = 0x51;
const I64_NE: u8 = 0x52;
const I64_LT_S: u8 = 0x53;
const I64_LT_U: u8 = 0x54;
const I64_GT_S: u8 = 0x55;
const I64_GT_U: u8 = 0x56;
const I64_LE_S: u8 = 0x57;
const I64_GE_S: u8 = 0x59;
const I32_ADD: u8 = 0x6A;
const I32_SUB: u8 = 0x6B;
const I32_AND: u8 = 0x71;
const I32_OR: u8 = 0x72;
const I32_SHL: u8 = 0x74;
const I64_ADD: u8 = 0x7C;
const I64_SUB: u8 = 0x7D;
const I64_MUL: u8 = 0x7E;
const I64_DIV_U: u8 = 0x80;
const I64_REM_U: u8 = 0x82;
const I64_AND: u8 = 0x83;
const I64_OR: u8 = 0x84;
const I64_SHL: u8 = 0x86;
const I64_SHR_S: u8 = 0x87;
const I64_SHR_U: u8 = 0x88;
const I32_WRAP_I64: u8 = 0xA7;
const I64_EXTEND_I32_S: u8 = 0xAC;
const I64_EXTEND_I32_U: u8 = 0xAD;
const I64_REINTERPRET_F64: u8 = 0xBD;
/// Block type "no result".
const VOID: u8 = 0x40;

const LOW32: i64 = 0xFFFF_FFFF;
const EXP_ALL_ONES: i64 = 0x7FF0_0000_0000_0000;

/// A tiny structured-assembly buffer: each method appends one instruction.
/// Reading a function body below is reading the WASM text format, one call
/// per instruction.
struct Asm {
    code: Vec<u8>,
}

impl Asm {
    fn new() -> Self {
        Asm { code: Vec::new() }
    }
    fn op(&mut self, byte: u8) -> &mut Self {
        self.code.push(byte);
        self
    }
    fn uleb(&mut self, mut v: u64) {
        loop {
            let byte = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                self.code.push(byte);
                return;
            }
            self.code.push(byte | 0x80);
        }
    }
    fn sleb(&mut self, mut v: i64) {
        loop {
            let byte = (v & 0x7F) as u8;
            v >>= 7;
            let done = (v == 0 && byte & 0x40 == 0) || (v == -1 && byte & 0x40 != 0);
            if done {
                self.code.push(byte);
                return;
            }
            self.code.push(byte | 0x80);
        }
    }
    fn i32c(&mut self, v: i32) -> &mut Self {
        self.code.push(I32_CONST);
        self.sleb(i64::from(v));
        self
    }
    fn i64c(&mut self, v: i64) -> &mut Self {
        self.code.push(I64_CONST);
        self.sleb(v);
        self
    }
    fn get(&mut self, local: u32) -> &mut Self {
        self.code.push(LOCAL_GET);
        self.uleb(u64::from(local));
        self
    }
    fn set(&mut self, local: u32) -> &mut Self {
        self.code.push(LOCAL_SET);
        self.uleb(u64::from(local));
        self
    }
    fn call(&mut self, func: u32) -> &mut Self {
        self.code.push(CALL);
        self.uleb(u64::from(func));
        self
    }
    fn br(&mut self, depth: u32) -> &mut Self {
        self.code.push(BR);
        self.uleb(u64::from(depth));
        self
    }
    fn br_if(&mut self, depth: u32) -> &mut Self {
        self.code.push(BR_IF);
        self.uleb(u64::from(depth));
        self
    }
    /// `block` / `loop` / `if` with no result.
    fn open(&mut self, kind: u8) -> &mut Self {
        self.code.push(kind);
        self.code.push(VOID);
        self
    }
    /// A memory access with natural alignment and a static offset.
    fn mem(&mut self, opcode: u8, align_log2: u32, offset: i32) -> &mut Self {
        self.code.push(opcode);
        self.uleb(u64::from(align_log2));
        self.uleb(offset as u64);
        self
    }
    fn load64(&mut self, offset: i32) -> &mut Self {
        self.mem(I64_LOAD, 3, offset)
    }
    fn store64(&mut self, offset: i32) -> &mut Self {
        self.mem(I64_STORE, 3, offset)
    }
    fn finish(mut self, locals: Vec<ValueType>) -> FunctionBody {
        self.code.push(END);
        FunctionBody { locals, code: self.code }
    }
}

/// Where the helpers are in the function index space, and what they call.
#[derive(Clone, Copy)]
struct Ctx {
    /// Function index of helper #0 (`bn_set`).
    base: u32,
    /// Function index of the `env.putchar(i32)` import.
    putchar: u32,
    /// Linear-memory address of the scratch region.
    scratch: i32,
}

impl Ctx {
    fn f(&self, n: u32) -> u32 {
        self.base + n
    }
    fn addr(&self, off: i32) -> i32 {
        self.scratch + off
    }
}

const BN_SET: u32 = 0;
const BN_MUL_SMALL: u32 = 1;
const BN_MUL_POW2: u32 = 2;
const BN_MUL_POW10: u32 = 3;
const BN_ADD: u32 = 4;
const BN_SUB: u32 = 5;
const BN_CMP: u32 = 6;
const F64_DIGITS: u32 = 7;
const PUT_DIGITS: u32 = 8;
const PUT_REPEAT: u32 = 9;

/// `limb(i)` address: `a + (i << 3)` with `a` and `i` in i32 locals.
fn limb_addr(asm: &mut Asm, a: u32, i: u32) {
    asm.get(a).get(i).i32c(3).op(I32_SHL).op(I32_ADD);
}

/// `i += 1; br_if 0 (i < 40)` — the tail of every limb loop.
fn next_limb(asm: &mut Asm, i: u32) {
    asm.get(i).i32c(1).op(I32_ADD).set(i);
    asm.get(i).i32c(LIMBS).op(I32_LT_S).br_if(0);
}

/// `bn_set(a, v)`: zero the 40 limbs, then the low and high halves of `v`.
fn bn_set() -> FunctionBody {
    const A: u32 = 0;
    const V: u32 = 1;
    const I: u32 = 2;
    let mut asm = Asm::new();
    asm.open(LOOP);
    limb_addr(&mut asm, A, I);
    asm.i64c(0).store64(0);
    next_limb(&mut asm, I);
    asm.op(END);
    asm.get(A).get(V).i64c(LOW32).op(I64_AND).store64(0);
    asm.get(A).get(V).i64c(32).op(I64_SHR_U).store64(8);
    asm.finish(vec![ValueType::I32])
}

/// `bn_mul_small(a, m)`: `t = limb * m + carry; limb = t & 0xFFFFFFFF;
/// carry = t >> 32` for every limb.
fn bn_mul_small() -> FunctionBody {
    const A: u32 = 0;
    const M: u32 = 1;
    const I: u32 = 2;
    const CARRY: u32 = 3;
    const P: u32 = 4;
    const TMP: u32 = 5;
    let mut asm = Asm::new();
    asm.open(LOOP);
    limb_addr(&mut asm, A, I);
    asm.set(P);
    asm.get(P).load64(0).get(M).op(I64_MUL).get(CARRY).op(I64_ADD).set(TMP);
    asm.get(P).get(TMP).i64c(LOW32).op(I64_AND).store64(0);
    asm.get(TMP).i64c(32).op(I64_SHR_U).set(CARRY);
    next_limb(&mut asm, I);
    asm.op(END);
    asm.finish(vec![ValueType::I32, ValueType::I64, ValueType::I32, ValueType::I64])
}

/// `bn_mul_pow2(a, n)` / `bn_mul_pow10(a, n)`: multiply in chunks of at most
/// `chunk` (31 bits / nine decimal digits) so each step's multiplier stays
/// below 2^32.
fn bn_mul_pow(ctx: Ctx, decimal: bool) -> FunctionBody {
    const A: u32 = 0;
    const N: u32 = 1;
    const C: u32 = 2;
    const P: u32 = 3;
    const J: u32 = 4;
    let chunk = if decimal { 9 } else { 31 };
    let mut asm = Asm::new();
    asm.open(BLOCK).open(LOOP);
    asm.get(N).i64c(0).op(I64_LE_S).br_if(1); // done when n <= 0
    // c = n < chunk ? n : chunk
    asm.get(N).i64c(chunk).get(N).i64c(chunk).op(I64_LT_S).op(SELECT).set(C);
    if decimal {
        // p = 10^c, by c multiplications.
        asm.i64c(1).set(P).i64c(0).set(J);
        asm.open(LOOP);
        asm.get(P).i64c(10).op(I64_MUL).set(P);
        asm.get(J).i64c(1).op(I64_ADD).set(J);
        asm.get(J).get(C).op(I64_LT_S).br_if(0);
        asm.op(END);
        asm.get(A).get(P).call(ctx.f(BN_MUL_SMALL));
    } else {
        asm.get(A).i64c(1).get(C).op(I64_SHL).call(ctx.f(BN_MUL_SMALL));
    }
    asm.get(N).get(C).op(I64_SUB).set(N);
    asm.br(0);
    asm.op(END).op(END);
    asm.finish(vec![ValueType::I64, ValueType::I64, ValueType::I64])
}

/// `bn_add(d, a, b)`: limb sums with carry. Reads index `i` of `a` and `b`
/// before writing index `i` of `d`, so `d` may alias either.
fn bn_add() -> FunctionBody {
    const D: u32 = 0;
    const A: u32 = 1;
    const B: u32 = 2;
    const I: u32 = 3;
    const CARRY: u32 = 4;
    const TMP: u32 = 5;
    let mut asm = Asm::new();
    asm.open(LOOP);
    limb_addr(&mut asm, A, I);
    asm.load64(0);
    limb_addr(&mut asm, B, I);
    asm.load64(0).op(I64_ADD).get(CARRY).op(I64_ADD).set(TMP);
    limb_addr(&mut asm, D, I);
    asm.get(TMP).i64c(LOW32).op(I64_AND).store64(0);
    asm.get(TMP).i64c(32).op(I64_SHR_U).set(CARRY);
    next_limb(&mut asm, I);
    asm.op(END);
    asm.finish(vec![ValueType::I32, ValueType::I64, ValueType::I64])
}

/// `bn_sub(a, b)` for `a >= b`: `t = a_i + 2^32 - b_i - borrow` is never
/// negative, and its bit 32 is set exactly when the limb did not borrow.
fn bn_sub() -> FunctionBody {
    const A: u32 = 0;
    const B: u32 = 1;
    const I: u32 = 2;
    const BORROW: u32 = 3;
    const TMP: u32 = 4;
    let mut asm = Asm::new();
    asm.open(LOOP);
    limb_addr(&mut asm, A, I);
    asm.load64(0).i64c(1 << 32).op(I64_ADD);
    limb_addr(&mut asm, B, I);
    asm.load64(0).op(I64_SUB).get(BORROW).op(I64_SUB).set(TMP);
    limb_addr(&mut asm, A, I);
    asm.get(TMP).i64c(LOW32).op(I64_AND).store64(0);
    asm.i64c(1).get(TMP).i64c(32).op(I64_SHR_U).op(I64_SUB).set(BORROW);
    next_limb(&mut asm, I);
    asm.op(END);
    asm.finish(vec![ValueType::I32, ValueType::I64, ValueType::I64])
}

/// `bn_cmp(a, b)`: from the top limb down, the first difference decides.
fn bn_cmp() -> FunctionBody {
    const A: u32 = 0;
    const B: u32 = 1;
    const I: u32 = 2;
    const X: u32 = 3;
    const Y: u32 = 4;
    let mut asm = Asm::new();
    asm.i32c(LIMBS - 1).set(I);
    asm.open(LOOP);
    limb_addr(&mut asm, A, I);
    asm.load64(0).set(X);
    limb_addr(&mut asm, B, I);
    asm.load64(0).set(Y);
    asm.get(X).get(Y).op(I64_NE).open(IF);
    asm.i32c(-1).i32c(1).get(X).get(Y).op(I64_LT_U).op(SELECT).op(RETURN);
    asm.op(END);
    asm.get(I).i32c(1).op(I32_SUB).set(I);
    asm.get(I).i32c(0).op(I32_GE_S).br_if(0);
    asm.op(END);
    asm.i32c(0);
    asm.finish(vec![ValueType::I32, ValueType::I64, ValueType::I64])
}

/// `bn_mul_small(addr, 10)` for each of `addrs`.
fn times_ten(asm: &mut Asm, ctx: Ctx, addrs: &[i32]) {
    for &a in addrs {
        asm.i32c(ctx.addr(a)).i64c(10).call(ctx.f(BN_MUL_SMALL));
    }
}

/// `bn_cmp(x, y)` with both operands scratch offsets; leaves the i32 result.
fn cmp(asm: &mut Asm, ctx: Ctx, x: i32, y: i32) {
    asm.i32c(ctx.addr(x)).i32c(ctx.addr(y)).call(ctx.f(BN_CMP));
}

/// `bn_add(d, a, b)` with scratch offsets.
fn add(asm: &mut Asm, ctx: Ctx, d: i32, a: i32, b: i32) {
    asm.i32c(ctx.addr(d)).i32c(ctx.addr(a)).i32c(ctx.addr(b)).call(ctx.f(BN_ADD));
}

/// `f64_digits(mag) -> n`: the Burger–Dybvig core (see `twig_f64_digits` in
/// `twig_runtime.c` for the commented C). Stores the digit values at
/// `DIGITS`, `k` at `K`, returns the digit count.
fn f64_digits(ctx: Ctx) -> FunctionBody {
    const MAG: u32 = 0;
    const F: u32 = 1;
    const E: u32 = 2;
    const EVEN: u32 = 3; // i32: 1 when f is even (interval edges included)
    const CLOSER: u32 = 4;
    const LE: u32 = 5;
    const SE: u32 = 6;
    const KK: u32 = 7;
    const N: u32 = 8; // i32
    const D: u32 = 9; // i32
    const LOWV: u32 = 10; // i32
    const HIGHV: u32 = 11; // i32
    const C: u32 = 12; // i32
    const BEXP: u32 = 13;
    const MANT: u32 = 14;
    let mut asm = Asm::new();
    // x = f * 2^e; a subnormal (biased exponent 0) has no hidden bit.
    asm.get(MAG).i64c((1 << 52) - 1).op(I64_AND).set(MANT);
    asm.get(MAG).i64c(52).op(I64_SHR_U).set(BEXP);
    asm.get(MANT).get(MANT).i64c(1 << 52).op(I64_OR).get(BEXP).op(I64_EQZ).op(SELECT).set(F);
    asm.i64c(-1074).get(BEXP).i64c(1075).op(I64_SUB).get(BEXP).op(I64_EQZ).op(SELECT).set(E);
    asm.get(F).i64c(1).op(I64_AND).op(I64_EQZ).set(EVEN);
    // closer = mant == 0 && bexp > 1: the gap below x is half the gap above.
    asm.get(MANT).op(I64_EQZ).get(BEXP).i64c(1).op(I64_GT_U).op(I32_AND).op(I64_EXTEND_I32_U).set(CLOSER);
    // le = max(e, 0); se = max(-e, 0)
    asm.get(E).i64c(0).get(E).i64c(0).op(I64_GT_S).op(SELECT).set(LE);
    asm.i64c(0).get(E).op(I64_SUB).i64c(0).get(E).i64c(0).op(I64_LT_S).op(SELECT).set(SE);
    // r = f * 2^(le+1+c); s = 2^(se+1+c); m+ = 2^(le+c); m- = 2^le
    asm.i32c(ctx.addr(R)).get(F).call(ctx.f(BN_SET));
    asm.i32c(ctx.addr(R)).get(LE).i64c(1).op(I64_ADD).get(CLOSER).op(I64_ADD).call(ctx.f(BN_MUL_POW2));
    asm.i32c(ctx.addr(S)).i64c(1).call(ctx.f(BN_SET));
    asm.i32c(ctx.addr(S)).get(SE).i64c(1).op(I64_ADD).get(CLOSER).op(I64_ADD).call(ctx.f(BN_MUL_POW2));
    asm.i32c(ctx.addr(MP)).i64c(1).call(ctx.f(BN_SET));
    asm.i32c(ctx.addr(MP)).get(LE).get(CLOSER).op(I64_ADD).call(ctx.f(BN_MUL_POW2));
    asm.i32c(ctx.addr(MM)).i64c(1).call(ctx.f(BN_SET));
    asm.i32c(ctx.addr(MM)).get(LE).call(ctx.f(BN_MUL_POW2));
    // k ~ floor((e + 52) * 1233 / 4096); i64.shr_s floors.
    asm.get(E).i64c(52).op(I64_ADD).i64c(1233).op(I64_MUL).i64c(12).op(I64_SHR_S).set(KK);
    asm.get(KK).i64c(0).op(I64_GE_S).open(IF);
    asm.i32c(ctx.addr(S)).get(KK).call(ctx.f(BN_MUL_POW10));
    asm.op(ELSE);
    for a in [R, MP, MM] {
        asm.i32c(ctx.addr(a)).i64c(0).get(KK).op(I64_SUB).call(ctx.f(BN_MUL_POW10));
    }
    asm.op(END);
    // Fix k up: while r + m+ >= s (> when odd), i.e. cmp + even > 0.
    asm.open(BLOCK).open(LOOP);
    add(&mut asm, ctx, T, R, MP);
    cmp(&mut asm, ctx, T, S);
    asm.get(EVEN).op(I32_ADD).i32c(0).op(I32_LE_S).br_if(1);
    times_ten(&mut asm, ctx, &[S]);
    asm.get(KK).i64c(1).op(I64_ADD).set(KK);
    asm.br(0);
    asm.op(END).op(END);
    // Fix k down: while 10(r + m+) < s (<= when odd), i.e. cmp < even.
    asm.open(BLOCK).open(LOOP);
    add(&mut asm, ctx, T, R, MP);
    times_ten(&mut asm, ctx, &[T]);
    cmp(&mut asm, ctx, T, S);
    asm.get(EVEN).op(I32_GE_S).br_if(1);
    times_ten(&mut asm, ctx, &[R, MP, MM]);
    asm.get(KK).i64c(1).op(I64_SUB).set(KK);
    asm.br(0);
    asm.op(END).op(END);
    asm.i32c(ctx.addr(K)).get(KK).store64(0);
    // Generate digits.
    asm.i32c(0).set(N);
    asm.open(BLOCK).open(LOOP); // $done / $digit
    times_ten(&mut asm, ctx, &[R, MP, MM]);
    // d = r div s by repeated subtraction (at most 9), r = r mod s.
    asm.i32c(0).set(D);
    asm.open(BLOCK).open(LOOP);
    cmp(&mut asm, ctx, R, S);
    asm.i32c(0).op(I32_LT_S).br_if(1);
    asm.i32c(ctx.addr(R)).i32c(ctx.addr(S)).call(ctx.f(BN_SUB));
    asm.get(D).i32c(1).op(I32_ADD).set(D);
    asm.br(0);
    asm.op(END).op(END);
    // low = r <= m- (< when odd); high = r + m+ >= s (> when odd).
    cmp(&mut asm, ctx, R, MM);
    asm.get(EVEN).op(I32_LT_S).set(LOWV);
    add(&mut asm, ctx, T, R, MP);
    cmp(&mut asm, ctx, T, S);
    asm.get(EVEN).op(I32_ADD).i32c(0).op(I32_GT_S).set(HIGHV);
    // Neither: store d and continue (while fewer than 31 digits).
    asm.get(LOWV).get(HIGHV).op(I32_OR).op(I32_EQZ).open(IF);
    asm.i32c(ctx.addr(DIGITS)).get(N).op(I32_ADD).get(D).mem(I32_STORE8, 0, 0);
    asm.get(N).i32c(1).op(I32_ADD).set(N);
    asm.get(N).i32c(31).op(I32_LT_S).br_if(1); // → $digit
    asm.br(2); // → after $done
    asm.op(END);
    // Stop here. Both sides possible: the closer, an exact tie to even.
    asm.get(LOWV).get(HIGHV).op(I32_AND).open(IF);
    add(&mut asm, ctx, T, R, R);
    cmp(&mut asm, ctx, T, S);
    asm.set(C);
    asm.get(D);
    asm.get(C).i32c(0).op(I32_GT_S);
    asm.get(C).op(I32_EQZ).get(D).i32c(1).op(I32_AND).op(I32_AND);
    asm.op(I32_OR).op(I32_ADD).set(D);
    asm.op(ELSE);
    // One side: d + 1 when only the upper rounding fits (never 10).
    asm.get(D).get(HIGHV).op(I32_ADD).set(D);
    asm.op(END);
    asm.i32c(ctx.addr(DIGITS)).get(N).op(I32_ADD).get(D).mem(I32_STORE8, 0, 0);
    asm.get(N).i32c(1).op(I32_ADD).set(N);
    asm.op(END).op(END);
    asm.get(N);
    let i64s = |n| std::iter::repeat_n(ValueType::I64, n);
    let mut locals: Vec<ValueType> = Vec::new();
    locals.extend(i64s(2)); // F, E
    locals.push(ValueType::I32); // EVEN
    locals.extend(i64s(4)); // CLOSER, LE, SE, KK
    locals.extend(std::iter::repeat_n(ValueType::I32, 5)); // N, D, LOWV, HIGHV, C
    locals.extend(i64s(2)); // BEXP, MANT
    asm.finish(locals)
}

/// `put_digits(from, to)`: `putchar('0' + DIGITS[i])` for `i` in `[from, to)`.
fn put_digits(ctx: Ctx) -> FunctionBody {
    const FROM: u32 = 0;
    const TO: u32 = 1;
    let mut asm = Asm::new();
    asm.open(BLOCK).open(LOOP);
    asm.get(FROM).get(TO).op(I32_GE_S).br_if(1);
    asm.i32c(ctx.addr(DIGITS)).get(FROM).op(I32_ADD).mem(I32_LOAD8_U, 0, 0);
    asm.i32c(48).op(I32_ADD).call(ctx.putchar);
    asm.get(FROM).i32c(1).op(I32_ADD).set(FROM);
    asm.br(0);
    asm.op(END).op(END);
    asm.finish(vec![])
}

/// `put_repeat(c, n)`: `putchar(c)` `n` times (nothing when `n <= 0`).
fn put_repeat(ctx: Ctx) -> FunctionBody {
    const CH: u32 = 0;
    const N: u32 = 1;
    let mut asm = Asm::new();
    asm.open(BLOCK).open(LOOP);
    asm.get(N).i64c(0).op(I64_LE_S).br_if(1);
    asm.get(CH).call(ctx.putchar);
    asm.get(N).i64c(1).op(I64_SUB).set(N);
    asm.br(0);
    asm.op(END).op(END);
    asm.finish(vec![])
}

/// `putchar` each byte of `text`.
fn put_text(asm: &mut Asm, ctx: Ctx, text: &[u8]) {
    for &b in text {
        asm.i32c(i32::from(b)).call(ctx.putchar);
    }
}

/// `if js { put js_text } else { put py_text }; return`.
fn put_by_layout(asm: &mut Asm, ctx: Ctx, js: u32, js_text: &[u8], py_text: &[u8]) {
    asm.get(js).open(IF);
    put_text(asm, ctx, js_text);
    asm.op(ELSE);
    put_text(asm, ctx, py_text);
    asm.op(END).op(RETURN);
}

/// `write_f64(x, js)`: NaN, sign, infinities, zeros, then the digits laid out
/// (fixed when -4 <= e < 16 for Python / -6 <= e < 21 for JavaScript, else
/// `d[.ddd]e±X`, Python padding the exponent to two digits).
fn write_f64(ctx: Ctx) -> FunctionBody {
    const X: u32 = 0;
    const JS: u32 = 1;
    const BITS: u32 = 2;
    const MAG: u32 = 3;
    const N: u32 = 4; // i32
    const E: u32 = 5;
    const A: u32 = 6;
    const E1: u32 = 7;
    const M: u32 = 8;
    let mut asm = Asm::new();
    asm.get(X).op(I64_REINTERPRET_F64).set(BITS);
    asm.get(BITS).i64c(i64::MAX).op(I64_AND).set(MAG);
    // NaN (any sign): exponent all ones, mantissa non-zero.
    asm.get(MAG).i64c(EXP_ALL_ONES).op(I64_GT_U).open(IF);
    put_by_layout(&mut asm, ctx, JS, b"NaN", b"nan");
    asm.op(END);
    // A minus sign for every negative value except JavaScript's -0.
    asm.get(BITS).i64c(0).op(I64_LT_S);
    asm.get(JS).get(MAG).op(I64_EQZ).op(I32_AND).op(I32_EQZ);
    asm.op(I32_AND).open(IF);
    put_text(&mut asm, ctx, b"-");
    asm.op(END);
    asm.get(MAG).i64c(EXP_ALL_ONES).op(I64_EQ).open(IF);
    put_by_layout(&mut asm, ctx, JS, b"Infinity", b"inf");
    asm.op(END);
    asm.get(MAG).op(I64_EQZ).open(IF);
    put_by_layout(&mut asm, ctx, JS, b"0", b"0.0");
    asm.op(END);
    // The digits, and e = k - 1 (the exponent of the first digit).
    asm.get(MAG).call(ctx.f(F64_DIGITS)).set(N);
    asm.i32c(ctx.addr(K)).load64(0).i64c(1).op(I64_SUB).set(E);
    // fixed = e >= (js ? -6 : -4) && e < (js ? 21 : 16)
    asm.get(E).i64c(-6).i64c(-4).get(JS).op(SELECT).op(I64_GE_S);
    asm.get(E).i64c(21).i64c(16).get(JS).op(SELECT).op(I64_LT_S);
    asm.op(I32_AND).open(IF);
    {
        asm.get(E).i64c(0).op(I64_LT_S).open(IF);
        // 0.000ddd
        put_text(&mut asm, ctx, b"0.");
        asm.i32c(48).i64c(-1).get(E).op(I64_SUB).call(ctx.f(PUT_REPEAT));
        asm.i32c(0).get(N).call(ctx.f(PUT_DIGITS));
        asm.op(ELSE);
        // e+1 integer places: digits while they last, then zeros.
        asm.get(E).i64c(1).op(I64_ADD).set(E1);
        asm.get(N).op(I64_EXTEND_I32_S).get(E1).get(N).op(I64_EXTEND_I32_S).get(E1).op(I64_LT_S).op(SELECT).set(M);
        asm.i32c(0).get(M).op(I32_WRAP_I64).call(ctx.f(PUT_DIGITS));
        asm.i32c(48).get(E1).get(M).op(I64_SUB).call(ctx.f(PUT_REPEAT));
        asm.get(N).op(I64_EXTEND_I32_S).get(E1).op(I64_GT_S).open(IF);
        put_text(&mut asm, ctx, b".");
        asm.get(E1).op(I32_WRAP_I64).get(N).call(ctx.f(PUT_DIGITS));
        asm.op(ELSE);
        // Python keeps ".0" on an integral float; JavaScript does not.
        asm.get(JS).op(I32_EQZ).open(IF);
        put_text(&mut asm, ctx, b".0");
        asm.op(END);
        asm.op(END);
        asm.op(END);
    }
    asm.op(ELSE);
    {
        // d[.ddd]e±X
        asm.i32c(0).i32c(1).call(ctx.f(PUT_DIGITS));
        asm.get(N).i32c(1).op(I32_GT_S).open(IF);
        put_text(&mut asm, ctx, b".");
        asm.i32c(1).get(N).call(ctx.f(PUT_DIGITS));
        asm.op(END);
        put_text(&mut asm, ctx, b"e");
        asm.i32c(i32::from(b'-')).i32c(i32::from(b'+')).get(E).i64c(0).op(I64_LT_S).op(SELECT).call(ctx.putchar);
        asm.i64c(0).get(E).op(I64_SUB).get(E).get(E).i64c(0).op(I64_LT_S).op(SELECT).set(A);
        // Python pads the exponent to two digits (1e-05); JavaScript does not.
        asm.get(JS).op(I32_EQZ).get(A).i64c(10).op(I64_LT_S).op(I32_AND).open(IF);
        put_text(&mut asm, ctx, b"0");
        asm.op(END);
        asm.get(A).i64c(100).op(I64_GE_S).open(IF);
        asm.get(A).i64c(100).op(I64_DIV_U).op(I32_WRAP_I64).i32c(48).op(I32_ADD).call(ctx.putchar);
        asm.op(END);
        asm.get(A).i64c(10).op(I64_GE_S).open(IF);
        asm.get(A).i64c(10).op(I64_DIV_U).i64c(10).op(I64_REM_U).op(I32_WRAP_I64).i32c(48).op(I32_ADD).call(ctx.putchar);
        asm.op(END);
        asm.get(A).i64c(10).op(I64_REM_U).op(I32_WRAP_I64).i32c(48).op(I32_ADD).call(ctx.putchar);
    }
    asm.op(END);
    let mut locals = vec![ValueType::I64, ValueType::I64, ValueType::I32];
    locals.extend(std::iter::repeat_n(ValueType::I64, 4)); // E, A, E1, M
    asm.finish(locals)
}

/// The eleven helpers in index order, each with its type, for a module whose
/// helper #0 gets function index `base`, whose `env.putchar` import is
/// `putchar`, and whose scratch region starts at linear address `scratch`.
pub(crate) fn build_f64_helpers(base: u32, putchar: u32, scratch: u32) -> Vec<(FuncType, FunctionBody)> {
    use ValueType::{F64, I32, I64};
    let ctx = Ctx { base, putchar, scratch: scratch as i32 };
    let ty = |params: Vec<ValueType>, results: Vec<ValueType>| FuncType { params, results };
    vec![
        (ty(vec![I32, I64], vec![]), bn_set()),
        (ty(vec![I32, I64], vec![]), bn_mul_small()),
        (ty(vec![I32, I64], vec![]), bn_mul_pow(ctx, false)),
        (ty(vec![I32, I64], vec![]), bn_mul_pow(ctx, true)),
        (ty(vec![I32, I32, I32], vec![]), bn_add()),
        (ty(vec![I32, I32], vec![]), bn_sub()),
        (ty(vec![I32, I32], vec![I32]), bn_cmp()),
        (ty(vec![I64], vec![I32]), f64_digits(ctx)),
        (ty(vec![I32, I32], vec![]), put_digits(ctx)),
        (ty(vec![I32, I64], vec![]), put_repeat(ctx)),
        (ty(vec![F64, I32], vec![]), write_f64(ctx)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_count_and_write_index_agree_with_the_builder() {
        let helpers = build_f64_helpers(100, 3, 4096);
        assert_eq!(helpers.len() as u32, F64_HELPER_COUNT);
        let (ty, _) = &helpers[WRITE_F64_OFFSET as usize];
        assert_eq!(ty.params, [ValueType::F64, ValueType::I32]);
        assert!(ty.results.is_empty());
    }

    #[test]
    fn scratch_layout_fits_the_reservation() {
        // Five big integers, the digit buffer and k, inside the reservation.
        assert_eq!(T + LIMBS * 8, DIGITS);
        assert!(DIGITS + 32 <= K && (K + 8) as usize <= F64_SCRATCH_BYTES);
        assert_eq!(K % 8, 0, "k is an aligned i64");
    }

    #[test]
    fn leb128_encodings() {
        let mut a = Asm::new();
        a.i64c(-1074).i32c(1600).i64c(1 << 52);
        // i64.const -1074 = 0x42 0xCE 0x77; i32.const 1600 = 0x41 0xC0 0x0C.
        assert_eq!(&a.code[..6], &[0x42, 0xCE, 0x77, 0x41, 0xC0, 0x0C]);
    }
}

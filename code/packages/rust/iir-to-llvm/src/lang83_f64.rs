//! LANG83 PR6a — shortest round-trip float output, as module-local LLVM IR.
//!
//! `call_builtin "write_f64_py", x` and `call_builtin "write_f64_js", x` lower
//! to `call void @__iir_write_f64(double x, i32 0|1)`. This file holds the IR
//! that call reaches. Like PR1b's `@__iir_write_i64`, it is emitted **into the
//! module** as `linkonce_odr` definitions (one copy survives however many
//! modules a link combines), so a `.ll` that prints a float needs nothing but
//! libc's `putchar` — no harness runtime, no shared helper library.
//!
//! # What it computes
//!
//! The digits are the shortest decimal that reads back as exactly `x` (closest
//! on a length tie, even on an exact tie). The algorithm is Burger–Dybvig's
//! free-format digit generation over exact big integers, the same core the
//! native runtime (`twig_runtime.c`, `twig_f64_digits`) and the WASM backend
//! carry. The C version is the readable reference; this IR follows it
//! function for function:
//!
//! | IR function | C reference | Job |
//! |-------------|-------------|-----|
//! | `@__iir_bn_set(a, v)` | `twig_bn_set` | `a = v` |
//! | `@__iir_bn_mul_small(a, m)` | `twig_bn_mul_small` | `a *= m` (`m < 2^32`) |
//! | `@__iir_bn_mul_pow2(a, n)` | `twig_bn_mul_pow2` | `a *= 2^n`, 31 bits per pass |
//! | `@__iir_bn_mul_pow10(a, n)` | `twig_bn_mul_pow10` | `a *= 10^n`, 10^9 per pass |
//! | `@__iir_bn_add(d, a, b)` | `twig_bn_add` | `d = a + b` (`d` may alias) |
//! | `@__iir_bn_sub(a, b)` | `twig_bn_sub` | `a -= b`, `a >= b` |
//! | `@__iir_bn_cmp(a, b)` | `twig_bn_cmp` | `-1 / 0 / 1` |
//! | `@__iir_f64_digits(mag, out, kout)` | `twig_f64_digits` | digits 0..9 into `out`, count returned, `k` stored |
//! | `@__iir_write_f64(x, js)` | `twig_write_f64` | specials, sign, layout, `putchar` |
//!
//! A big integer is `[40 x i64]` on the stack: 40 base-2^32 limbs, least
//! significant first, each in a 64-bit word so `limb * m + carry` cannot
//! overflow. 1280 bits covers the largest value any double needs (< 2^1084).
//!
//! Every loop below is a counted loop over a `phi` induction variable; there
//! is no recursion and no memory outside the five stack big integers and the
//! 32-byte digit buffer, whose writes are bounded by `n < 31`.

/// The module-local definitions, emitted once when a module uses either
/// float writer. Needs `declare i32 @putchar(i32)`, which the caller emits
/// (the module may already declare it for the `putchar` builtin).
pub(crate) const LANG83_F64_RUNTIME_LL: &str = r#"
; ── LANG83 PR6a: shortest round-trip float output (see lang83_f64.rs) ──────

; a = v: zero all 40 limbs, then the low and high 32 bits of v.
define linkonce_odr void @__iir_bn_set(ptr %a, i64 %v) {
entry:
  br label %zero
zero:
  %i = phi i64 [ 0, %entry ], [ %i1, %zero ]
  %p = getelementptr i64, ptr %a, i64 %i
  store i64 0, ptr %p
  %i1 = add i64 %i, 1
  %done = icmp eq i64 %i1, 40
  br i1 %done, label %fill, label %zero
fill:
  %lo = and i64 %v, 4294967295
  store i64 %lo, ptr %a
  %hi = lshr i64 %v, 32
  %p1 = getelementptr i64, ptr %a, i64 1
  store i64 %hi, ptr %p1
  ret void
}

; a *= m, m < 2^32: limb * m + carry < 2^64, low half stays, high half carries.
define linkonce_odr void @__iir_bn_mul_small(ptr %a, i64 %m) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i1, %loop ]
  %carry = phi i64 [ 0, %entry ], [ %c1, %loop ]
  %p = getelementptr i64, ptr %a, i64 %i
  %l = load i64, ptr %p
  %prod = mul i64 %l, %m
  %t = add i64 %prod, %carry
  %lo = and i64 %t, 4294967295
  store i64 %lo, ptr %p
  %c1 = lshr i64 %t, 32
  %i1 = add i64 %i, 1
  %done = icmp eq i64 %i1, 40
  br i1 %done, label %exit, label %loop
exit:
  ret void
}

; a *= 2^n, at most 31 bits per pass.
define linkonce_odr void @__iir_bn_mul_pow2(ptr %a, i64 %n) {
entry:
  br label %head
head:
  %k = phi i64 [ %n, %entry ], [ %k1, %body ]
  %more = icmp sgt i64 %k, 0
  br i1 %more, label %body, label %exit
body:
  %small = icmp slt i64 %k, 31
  %c = select i1 %small, i64 %k, i64 31
  %m = shl i64 1, %c
  call void @__iir_bn_mul_small(ptr %a, i64 %m)
  %k1 = sub i64 %k, %c
  br label %head
exit:
  ret void
}

; a *= 10^n, at most nine decimal digits (10^9 < 2^32) per pass.
define linkonce_odr void @__iir_bn_mul_pow10(ptr %a, i64 %n) {
entry:
  br label %head
head:
  %k = phi i64 [ %n, %entry ], [ %k1, %apply ]
  %more = icmp sgt i64 %k, 0
  br i1 %more, label %chunk, label %exit
chunk:
  %small = icmp slt i64 %k, 9
  %c = select i1 %small, i64 %k, i64 9
  br label %pow
pow:
  %j = phi i64 [ 0, %chunk ], [ %j1, %pow ]
  %p = phi i64 [ 1, %chunk ], [ %p1, %pow ]
  %p1 = mul i64 %p, 10
  %j1 = add i64 %j, 1
  %pdone = icmp eq i64 %j1, %c
  br i1 %pdone, label %apply, label %pow
apply:
  call void @__iir_bn_mul_small(ptr %a, i64 %p1)
  %k1 = sub i64 %k, %c
  br label %head
exit:
  ret void
}

; d = a + b, limb by limb with carry. Reads index i before writing it, so
; d may alias a or b (`t = r + r`).
define linkonce_odr void @__iir_bn_add(ptr %d, ptr %a, ptr %b) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i1, %loop ]
  %carry = phi i64 [ 0, %entry ], [ %c1, %loop ]
  %pa = getelementptr i64, ptr %a, i64 %i
  %pb = getelementptr i64, ptr %b, i64 %i
  %pd = getelementptr i64, ptr %d, i64 %i
  %x = load i64, ptr %pa
  %y = load i64, ptr %pb
  %s0 = add i64 %x, %y
  %t = add i64 %s0, %carry
  %lo = and i64 %t, 4294967295
  store i64 %lo, ptr %pd
  %c1 = lshr i64 %t, 32
  %i1 = add i64 %i, 1
  %done = icmp eq i64 %i1, 40
  br i1 %done, label %exit, label %loop
exit:
  ret void
}

; a -= b (a >= b). t = a_i + 2^32 - b_i - borrow is never negative; bit 32 of
; t is set exactly when this limb did not need to borrow.
define linkonce_odr void @__iir_bn_sub(ptr %a, ptr %b) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i1, %loop ]
  %borrow = phi i64 [ 0, %entry ], [ %b1, %loop ]
  %pa = getelementptr i64, ptr %a, i64 %i
  %pb = getelementptr i64, ptr %b, i64 %i
  %x = load i64, ptr %pa
  %y = load i64, ptr %pb
  %t0 = add i64 %x, 4294967296
  %t1 = sub i64 %t0, %y
  %t = sub i64 %t1, %borrow
  %lo = and i64 %t, 4294967295
  store i64 %lo, ptr %pa
  %hi = lshr i64 %t, 32
  %b1 = sub i64 1, %hi
  %i1 = add i64 %i, 1
  %done = icmp eq i64 %i1, 40
  br i1 %done, label %exit, label %loop
exit:
  ret void
}

; -1, 0, 1 as a <, ==, > b — from the most significant limb down.
define linkonce_odr i32 @__iir_bn_cmp(ptr %a, ptr %b) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 39, %entry ], [ %i1, %next ]
  %pa = getelementptr i64, ptr %a, i64 %i
  %pb = getelementptr i64, ptr %b, i64 %i
  %x = load i64, ptr %pa
  %y = load i64, ptr %pb
  %ne = icmp ne i64 %x, %y
  br i1 %ne, label %differ, label %next
next:
  %i1 = sub i64 %i, 1
  %end = icmp slt i64 %i1, 0
  br i1 %end, label %equal, label %loop
differ:
  %lt = icmp ult i64 %x, %y
  %r = select i1 %lt, i32 -1, i32 1
  ret i32 %r
equal:
  ret i32 0
}

; The shortest digits of the finite positive double with bits %mag: digit
; values 0..9 into %out, their count returned, k (x = 0.D * 10^k) to %kout.
define linkonce_odr i32 @__iir_f64_digits(i64 %mag, ptr %out, ptr %kout) {
entry:
  %r = alloca [40 x i64]
  %s = alloca [40 x i64]
  %mp = alloca [40 x i64]
  %mm = alloca [40 x i64]
  %t = alloca [40 x i64]
  ; x = f * 2^e; subnormals (biased exponent 0) have no hidden bit, e = -1074.
  %mant = and i64 %mag, 4503599627370495
  %bexp = lshr i64 %mag, 52
  %isub = icmp eq i64 %bexp, 0
  %fn = or i64 %mant, 4503599627370496
  %f = select i1 %isub, i64 %mant, i64 %fn
  %en = sub i64 %bexp, 1075
  %e = select i1 %isub, i64 -1074, i64 %en
  ; even: the interval's edges read back as x (round-half-even), so each
  ; test widens by one: "c > -even" / "c < even".
  %fodd = and i64 %f, 1
  %even64 = xor i64 %fodd, 1
  %even = trunc i64 %even64 to i32
  %neven = sub i32 0, %even
  ; closer: at a power of two the gap below x is half the gap above.
  %mz = icmp eq i64 %mant, 0
  %bgt1 = icmp ugt i64 %bexp, 1
  %cl = and i1 %mz, %bgt1
  %closer = zext i1 %cl to i64
  %epos = icmp sgt i64 %e, 0
  %le = select i1 %epos, i64 %e, i64 0
  %eneg = icmp slt i64 %e, 0
  %negE = sub i64 0, %e
  %se = select i1 %eneg, i64 %negE, i64 0
  ; r = f*2^(le+1+c), s = 2^(se+1+c), m+ = 2^(le+c), m- = 2^le
  call void @__iir_bn_set(ptr %r, i64 %f)
  %rx1 = add i64 %le, 1
  %rx = add i64 %rx1, %closer
  call void @__iir_bn_mul_pow2(ptr %r, i64 %rx)
  call void @__iir_bn_set(ptr %s, i64 1)
  %sx1 = add i64 %se, 1
  %sx = add i64 %sx1, %closer
  call void @__iir_bn_mul_pow2(ptr %s, i64 %sx)
  call void @__iir_bn_set(ptr %mp, i64 1)
  %px = add i64 %le, %closer
  call void @__iir_bn_mul_pow2(ptr %mp, i64 %px)
  call void @__iir_bn_set(ptr %mm, i64 1)
  call void @__iir_bn_mul_pow2(ptr %mm, i64 %le)
  ; k ~ floor((e + 52) * log10 2), 1233/4096 ~ log10 2 (ashr floors).
  %kn0 = add i64 %e, 52
  %kn = mul i64 %kn0, 1233
  %k0 = ashr i64 %kn, 12
  %kpos = icmp sge i64 %k0, 0
  br i1 %kpos, label %scale_s, label %scale_r
scale_s:
  call void @__iir_bn_mul_pow10(ptr %s, i64 %k0)
  br label %fix_up
scale_r:
  %nk = sub i64 0, %k0
  call void @__iir_bn_mul_pow10(ptr %r, i64 %nk)
  call void @__iir_bn_mul_pow10(ptr %mp, i64 %nk)
  call void @__iir_bn_mul_pow10(ptr %mm, i64 %nk)
  br label %fix_up
fix_up:
  ; while r + m+ >= s (> when odd): s *= 10, k += 1
  %k1 = phi i64 [ %k0, %scale_s ], [ %k0, %scale_r ], [ %k1n, %up ]
  call void @__iir_bn_add(ptr %t, ptr %r, ptr %mp)
  %cu = call i32 @__iir_bn_cmp(ptr %t, ptr %s)
  %big = icmp sgt i32 %cu, %neven
  br i1 %big, label %up, label %fix_down
up:
  call void @__iir_bn_mul_small(ptr %s, i64 10)
  %k1n = add i64 %k1, 1
  br label %fix_up
fix_down:
  ; while 10 (r + m+) < s (<= when odd): r, m+, m- *= 10, k -= 1
  %k2 = phi i64 [ %k1, %fix_up ], [ %k2n, %down ]
  call void @__iir_bn_add(ptr %t, ptr %r, ptr %mp)
  call void @__iir_bn_mul_small(ptr %t, i64 10)
  %cd = call i32 @__iir_bn_cmp(ptr %t, ptr %s)
  %tiny = icmp slt i32 %cd, %even
  br i1 %tiny, label %down, label %gen
down:
  call void @__iir_bn_mul_small(ptr %r, i64 10)
  call void @__iir_bn_mul_small(ptr %mp, i64 10)
  call void @__iir_bn_mul_small(ptr %mm, i64 10)
  %k2n = sub i64 %k2, 1
  br label %fix_down
gen:
  store i64 %k2, ptr %kout
  br label %next_digit
next_digit:
  %n = phi i64 [ 0, %gen ], [ %n1, %more ]
  call void @__iir_bn_mul_small(ptr %r, i64 10)
  call void @__iir_bn_mul_small(ptr %mp, i64 10)
  call void @__iir_bn_mul_small(ptr %mm, i64 10)
  br label %divide
divide:
  ; d = r div s by repeated subtraction (d <= 9), leaving r = r mod s.
  %d = phi i64 [ 0, %next_digit ], [ %d1, %subtract ]
  %cq = call i32 @__iir_bn_cmp(ptr %r, ptr %s)
  %fits = icmp sge i32 %cq, 0
  br i1 %fits, label %subtract, label %decide
subtract:
  call void @__iir_bn_sub(ptr %r, ptr %s)
  %d1 = add i64 %d, 1
  br label %divide
decide:
  ; low: r <= m- (< when odd) — stopping with d stays inside the interval.
  ; high: r + m+ >= s (> when odd) — stopping with d+1 does.
  %c1 = call i32 @__iir_bn_cmp(ptr %r, ptr %mm)
  %low = icmp slt i32 %c1, %even
  call void @__iir_bn_add(ptr %t, ptr %r, ptr %mp)
  %c2 = call i32 @__iir_bn_cmp(ptr %t, ptr %s)
  %high = icmp sgt i32 %c2, %neven
  %stop = or i1 %low, %high
  %n1 = add i64 %n, 1
  %slot = getelementptr i8, ptr %out, i64 %n
  br i1 %stop, label %last, label %more
more:
  %d8 = trunc i64 %d to i8
  store i8 %d8, ptr %slot
  %room = icmp slt i64 %n1, 31
  br i1 %room, label %next_digit, label %finish
last:
  %both = and i1 %low, %high
  br i1 %both, label %closest, label %one_side
one_side:
  ; high only: d + 1 (never 10); low only: d.
  %h64 = zext i1 %high to i64
  %ds = add i64 %d, %h64
  br label %store_last
closest:
  ; both d and d+1 read back as x: the closer wins, an exact tie (2r == s)
  ; goes to the even digit.
  call void @__iir_bn_add(ptr %t, ptr %r, ptr %r)
  %c3 = call i32 @__iir_bn_cmp(ptr %t, ptr %s)
  %gt = icmp sgt i32 %c3, 0
  %eq = icmp eq i32 %c3, 0
  %dodd = trunc i64 %d to i1
  %tie_up = and i1 %eq, %dodd
  %up1 = or i1 %gt, %tie_up
  %u64 = zext i1 %up1 to i64
  %dc = add i64 %d, %u64
  br label %store_last
store_last:
  %dl = phi i64 [ %ds, %one_side ], [ %dc, %closest ]
  %dl8 = trunc i64 %dl to i8
  store i8 %dl8, ptr %slot
  br label %finish
finish:
  %cnt = phi i64 [ %n1, %more ], [ %n1, %store_last ]
  %cnt32 = trunc i64 %cnt to i32
  ret i32 %cnt32
}

; putchar('0' + d[i]) for i in [from, to).
define linkonce_odr void @__iir_put_digits(ptr %d, i64 %from, i64 %to) {
entry:
  br label %head
head:
  %i = phi i64 [ %from, %entry ], [ %i1, %body ]
  %more = icmp slt i64 %i, %to
  br i1 %more, label %body, label %exit
body:
  %p = getelementptr i8, ptr %d, i64 %i
  %v = load i8, ptr %p
  %v32 = zext i8 %v to i32
  %ch = add i32 %v32, 48
  %w = call i32 @putchar(i32 %ch)
  %i1 = add i64 %i, 1
  br label %head
exit:
  ret void
}

; putchar(c) n times (n <= 0: nothing).
define linkonce_odr void @__iir_put_repeat(i32 %c, i64 %n) {
entry:
  br label %head
head:
  %i = phi i64 [ 0, %entry ], [ %i1, %body ]
  %more = icmp slt i64 %i, %n
  br i1 %more, label %body, label %exit
body:
  %w = call i32 @putchar(i32 %c)
  %i1 = add i64 %i, 1
  br label %head
exit:
  ret void
}

; Every byte of the n-byte constant at %p.
define linkonce_odr void @__iir_put_text(ptr %p, i64 %n) {
entry:
  br label %head
head:
  %i = phi i64 [ 0, %entry ], [ %i1, %body ]
  %more = icmp slt i64 %i, %n
  br i1 %more, label %body, label %exit
body:
  %q = getelementptr i8, ptr %p, i64 %i
  %v = load i8, ptr %q
  %v32 = zext i8 %v to i32
  %w = call i32 @putchar(i32 %v32)
  %i1 = add i64 %i, 1
  br label %head
exit:
  ret void
}

@__iir_f64_nan_py = linkonce_odr unnamed_addr constant [3 x i8] c"nan"
@__iir_f64_nan_js = linkonce_odr unnamed_addr constant [3 x i8] c"NaN"
@__iir_f64_inf_py = linkonce_odr unnamed_addr constant [3 x i8] c"inf"
@__iir_f64_inf_js = linkonce_odr unnamed_addr constant [8 x i8] c"Infinity"
@__iir_f64_zero_py = linkonce_odr unnamed_addr constant [3 x i8] c"0.0"

; Print x as Python repr (%js = 0) or JavaScript toString (%js != 0), no
; newline. e = k - 1 is the exponent of the first digit; fixed notation when
; -4 <= e < 16 (Python) / -6 <= e < 21 (JavaScript), else d[.ddd]e+-X.
define linkonce_odr void @__iir_write_f64(double %x, i32 %js) {
entry:
  %digits = alloca [32 x i8]
  %kslot = alloca i64
  %bits = bitcast double %x to i64
  %mag = and i64 %bits, 9223372036854775807
  %isjs = icmp ne i32 %js, 0
  %isnan = icmp ugt i64 %mag, 9218868437227405312
  br i1 %isnan, label %nan, label %signed
nan:
  %nanp = select i1 %isjs, ptr @__iir_f64_nan_js, ptr @__iir_f64_nan_py
  call void @__iir_put_text(ptr %nanp, i64 3)
  ret void
signed:
  ; A minus sign for every negative value except JavaScript's -0 ("0").
  %neg = icmp slt i64 %bits, 0
  %iszero = icmp eq i64 %mag, 0
  %jszero = and i1 %isjs, %iszero
  %keep = xor i1 %jszero, true
  %minus = and i1 %neg, %keep
  br i1 %minus, label %sign, label %special
sign:
  %ws = call i32 @putchar(i32 45)
  br label %special
special:
  %isinf = icmp eq i64 %mag, 9218868437227405312
  br i1 %isinf, label %inf, label %finite
inf:
  br i1 %isjs, label %inf_js, label %inf_py
inf_js:
  call void @__iir_put_text(ptr @__iir_f64_inf_js, i64 8)
  ret void
inf_py:
  call void @__iir_put_text(ptr @__iir_f64_inf_py, i64 3)
  ret void
finite:
  br i1 %iszero, label %zero, label %nonzero
zero:
  br i1 %isjs, label %zero_js, label %zero_py
zero_js:
  %wz = call i32 @putchar(i32 48)
  ret void
zero_py:
  call void @__iir_put_text(ptr @__iir_f64_zero_py, i64 3)
  ret void
nonzero:
  %n32 = call i32 @__iir_f64_digits(i64 %mag, ptr %digits, ptr %kslot)
  %n = sext i32 %n32 to i64
  %k = load i64, ptr %kslot
  %e = sub i64 %k, 1
  %lo = select i1 %isjs, i64 -6, i64 -4
  %hi = select i1 %isjs, i64 21, i64 16
  %ge_lo = icmp sge i64 %e, %lo
  %lt_hi = icmp slt i64 %e, %hi
  %fixed = and i1 %ge_lo, %lt_hi
  br i1 %fixed, label %fixed_notation, label %sci
fixed_notation:
  %eneg = icmp slt i64 %e, 0
  br i1 %eneg, label %below_one, label %at_least_one
below_one:
  ; 0.000ddd: "0." then -e-1 zeros, then the digits.
  %w0 = call i32 @putchar(i32 48)
  %w1 = call i32 @putchar(i32 46)
  %lead = sub i64 -1, %e
  call void @__iir_put_repeat(i32 48, i64 %lead)
  call void @__iir_put_digits(ptr %digits, i64 0, i64 %n)
  ret void
at_least_one:
  ; e+1 integer places: digits while they last, then zeros.
  %e1 = add i64 %e, 1
  %fewer = icmp slt i64 %n, %e1
  %int_digits = select i1 %fewer, i64 %n, i64 %e1
  call void @__iir_put_digits(ptr %digits, i64 0, i64 %int_digits)
  %pad = sub i64 %e1, %int_digits
  call void @__iir_put_repeat(i32 48, i64 %pad)
  %has_frac = icmp sgt i64 %n, %e1
  br i1 %has_frac, label %fraction, label %integral
fraction:
  %wd = call i32 @putchar(i32 46)
  call void @__iir_put_digits(ptr %digits, i64 %e1, i64 %n)
  ret void
integral:
  ; Python keeps ".0" on an integral float; JavaScript does not.
  br i1 %isjs, label %done, label %point_zero
point_zero:
  %wp = call i32 @putchar(i32 46)
  %wq = call i32 @putchar(i32 48)
  br label %done
done:
  ret void
sci:
  call void @__iir_put_digits(ptr %digits, i64 0, i64 1)
  %multi = icmp sgt i64 %n, 1
  br i1 %multi, label %sci_frac, label %sci_exp
sci_frac:
  %wf = call i32 @putchar(i32 46)
  call void @__iir_put_digits(ptr %digits, i64 1, i64 %n)
  br label %sci_exp
sci_exp:
  %we = call i32 @putchar(i32 101)
  %eneg2 = icmp slt i64 %e, 0
  %sgn = select i1 %eneg2, i32 45, i32 43
  %wsg = call i32 @putchar(i32 %sgn)
  %negated = sub i64 0, %e
  %a = select i1 %eneg2, i64 %negated, i64 %e
  ; Python pads the exponent to two digits (1e-05); JavaScript does not.
  %one_digit = icmp slt i64 %a, 10
  %ispy = xor i1 %isjs, true
  %pad0 = and i1 %ispy, %one_digit
  br i1 %pad0, label %exp_pad, label %exp_hundreds
exp_pad:
  %wpad = call i32 @putchar(i32 48)
  br label %exp_hundreds
exp_hundreds:
  %has100 = icmp sge i64 %a, 100
  br i1 %has100, label %put100, label %exp_tens
put100:
  %h = udiv i64 %a, 100
  %h32 = trunc i64 %h to i32
  %hc = add i32 %h32, 48
  %wh = call i32 @putchar(i32 %hc)
  br label %exp_tens
exp_tens:
  %has10 = icmp sge i64 %a, 10
  br i1 %has10, label %put10, label %exp_ones
put10:
  %tq = udiv i64 %a, 10
  %tr = urem i64 %tq, 10
  %t32 = trunc i64 %tr to i32
  %tc = add i32 %t32, 48
  %wt = call i32 @putchar(i32 %tc)
  br label %exp_ones
exp_ones:
  %o = urem i64 %a, 10
  %o32 = trunc i64 %o to i32
  %oc = add i32 %o32, 48
  %wo = call i32 @putchar(i32 %oc)
  ret void
}
"#;

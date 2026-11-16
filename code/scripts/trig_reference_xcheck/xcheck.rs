// TRIG-REF-1 (PHY00a 3.3-3.9) in Rust: the second host cross-check of the
// conformance-vector generator (PHY00a 6.4 step 2).  Same stdin/stdout protocol
// as xcheck.c.  Rust never contracts a*b+c (no `mul_add`), and nothing here
// calls a libm function.  It shares no code with xcheck.c or the Python model.
use std::io::{self, BufRead, Write};

fn f(b: u64) -> f64 {
    f64::from_bits(b)
}

const INV_PIO2: u64 = 0x3FE45F306DC9C883;
const PIO2: [u64; 5] = [
    0x3FF921FB40000000,
    0x3E74442D00000000,
    0x3CF8469880000000,
    0x3B78CC5160000000,
    0x39F01B839A25204A,
];
const SC: [u64; 9] = [
    0xBFC5555555555555,
    0x3F81111111111111,
    0xBF2A01A01A01A01A,
    0x3EC71DE3A556C734,
    0xBE5AE64567F544E4,
    0x3DE6124613A86D09,
    0xBD6AE7F3E733B81F,
    0x3CE952C77030AD4A,
    0xBC62F49B46814157,
];
const CC: [u64; 8] = [
    0x3FA5555555555555,
    0xBF56C16C16C16C17,
    0x3EFA01A01A01A01A,
    0xBE927E4FB7789F5C,
    0x3E21EED8EFF8D898,
    0xBDA93974A8C07C9D,
    0x3D2AE7F3E733B81F,
    0xBCA6827863B97D97,
];
const MAGIC: u64 = 0x4338000000000000;
const LIMIT: u64 = 0x41C0000000000000;
const TINY: u64 = 0x3E40000000000000;
const RAD: u64 = 0x3F91DF46A2529D39;
const DEG: u64 = 0x404CA5DC1A63C1F8;
const PI: u64 = 0x400921FB54442D18;
const HALF_PI: u64 = 0x3FF921FB54442D18;
const POLE: u64 = 0x3CD203AF9EE75616; // 1e-15
const BIG: u64 = 0x7FE1CCF385EBC8A0; // 1e308
const TERM: u64 = 0x3C670EF54646D497; // 1e-17
const SQ_TINY: u64 = 0x01A56E1FC2F8F359; // 1e-300

fn sin_k(r: f64) -> f64 {
    let z = r * r;
    let mut p = f(SC[8]);
    for n in (0..8).rev() {
        p *= z;
        p += f(SC[n]);
    }
    let mut w = r * z;
    w *= p;
    r + w
}

fn cos_k(r: f64) -> f64 {
    let z = r * r;
    let mut q = f(CC[7]);
    for n in (0..7).rev() {
        q *= z;
        q += f(CC[n]);
    }
    let mut u = 0.5 * z;
    u = 1.0 - u;
    let mut v = z * z;
    v *= q;
    u + v
}

fn reduce(x: f64) -> (f64, f64) {
    let t = x * f(INV_PIO2);
    let k = (t + f(MAGIC)) - f(MAGIC);
    let mut r = x;
    for p in PIO2 {
        let m = k * f(p);
        r -= m;
    }
    (k, r)
}

// The negated comparison is the point: a NaN fails `<=`, so one test covers NaN,
// both infinities and every finite value beyond the domain (PHY00a 3.6).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn sin(x: f64) -> f64 {
    if !(x.abs() <= f(LIMIT)) {
        return f64::NAN;
    }
    if x.abs() < f(TINY) {
        return x;
    }
    let (k, r) = reduce(x);
    match (k as i64) & 3 {
        0 => sin_k(r),
        1 => cos_k(r),
        2 => -sin_k(r),
        _ => -cos_k(r),
    }
}

// The negated comparison is the point: a NaN fails `<=`, so one test covers NaN,
// both infinities and every finite value beyond the domain (PHY00a 3.6).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn cos(x: f64) -> f64 {
    if !(x.abs() <= f(LIMIT)) {
        return f64::NAN;
    }
    if x.abs() < f(TINY) {
        return 1.0;
    }
    let (k, r) = reduce(x);
    match (k as i64) & 3 {
        0 => cos_k(r),
        1 => -sin_k(r),
        2 => -cos_k(r),
        _ => sin_k(r),
    }
}

fn tan(x: f64) -> f64 {
    let s = sin(x);
    let c = cos(x);
    if c.abs() < f(POLE) {
        return if s > 0.0 { f(BIG) } else { -f(BIG) };
    }
    s / c
}

fn sqrt(x: f64) -> Option<f64> {
    if x < 0.0 {
        return None;
    }
    if x == 0.0 || x == f64::INFINITY {
        return Some(x);
    }
    let (mut scaled, mut scale) = (x, 1.0);
    while scaled < 0.25 {
        scaled *= 4.0;
        scale *= 0.5;
    }
    while scaled >= 4.0 {
        scaled *= 0.25;
        scale *= 2.0;
    }
    let mut guess = if scaled >= 1.0 { scaled } else { 1.0 };
    for _ in 0..60 {
        let next = (guess + scaled / guess) / 2.0;
        if (next - guess).abs() < f(POLE) * guess + f(SQ_TINY) {
            return Some(next * scale);
        }
        guess = next;
    }
    Some(guess * scale)
}

fn atan_core(x: f64) -> f64 {
    let reduced = x / (1.0 + sqrt(1.0 + x * x).unwrap());
    let t = reduced;
    let tsq = t * t;
    let mut term = t;
    let mut result = t;
    for n in 1..=30 {
        term *= -tsq;
        term *= (2 * n - 1) as f64;
        term /= (2 * n + 1) as f64;
        result += term;
        if term.abs() < f(TERM) {
            break;
        }
    }
    2.0 * result
}

fn atan(x: f64) -> f64 {
    if x.abs() <= f(TINY) {
        return x;
    }
    if x > 1.0 {
        return f(HALF_PI) - atan_core(1.0 / x);
    }
    if x < -1.0 {
        return -f(HALF_PI) - atan_core(1.0 / x);
    }
    atan_core(x)
}

fn atan2(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x > 0.0 {
        atan(y / x)
    } else if x < 0.0 && y >= 0.0 {
        atan(y / x) + f(PI)
    } else if x < 0.0 && y < 0.0 {
        atan(y / x) - f(PI)
    } else if x == 0.0 && y > 0.0 {
        f(HALF_PI)
    } else if x == 0.0 && y < 0.0 {
        -f(HALF_PI)
    } else {
        0.0
    }
}

fn emit(out: &mut impl Write, r: f64) {
    if r.is_nan() {
        writeln!(out, "7ff8000000000000").unwrap();
    } else {
        writeln!(out, "{:016x}", r.to_bits()).unwrap();
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut it = line.split_whitespace();
        let (op, arg) = match (it.next(), it.next()) {
            (Some(a), Some(b)) => (a, b),
            _ => continue,
        };
        if op == "atan2" {
            let (ys, xs) = arg.split_once(',').unwrap();
            let y = f(u64::from_str_radix(ys, 16).unwrap());
            let x = f(u64::from_str_radix(xs, 16).unwrap());
            emit(&mut out, atan2(y, x));
            continue;
        }
        let x = f(u64::from_str_radix(arg, 16).unwrap());
        match op {
            "sin" => emit(&mut out, sin(x)),
            "cos" => emit(&mut out, cos(x)),
            "tan" => emit(&mut out, tan(x)),
            "radians" => emit(&mut out, x * f(RAD)),
            "degrees" => emit(&mut out, x * f(DEG)),
            "atan" => emit(&mut out, atan(x)),
            "sqrt" => match sqrt(x) {
                Some(r) => emit(&mut out, r),
                None => writeln!(out, "error").unwrap(),
            },
            _ => panic!("bad op {}", op),
        }
    }
}

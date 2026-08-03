//! # Three-way conformance — the payoff of the C → SIR → Ruby initiative.
//!
//! For a corpus of small C programs in the milestone-1 integer subset
//! (functions, typed `+`/`-`/`*` arithmetic, casts, declarations, `printf`,
//! `return` — no control flow, comparisons, or division yet), we run each
//! program **three ways** and assert the stdout is *byte-identical*:
//!
//! 1. **Reference C** — the original source compiled with `clang -fwrapv` and
//!    run.  `-fwrapv` makes signed overflow two's-complement wrap, which is the
//!    behaviour SIR models, so this is the oracle.
//! 2. **Emitted Ruby** — `c_to_semantic_ir::compile_source` → `semantic_ir_to_ruby`
//!    → run with the real `ruby`.  Ruby is arbitrary-precision, so the *only*
//!    thing that can make it wrap like C is the `Convert` nodes the frontend
//!    inserted per C's integer promotions.
//! 3. **Emitted C** — the same SIR → `semantic_ir_to_c` → compiled and run.
//!
//! When all three agree on `(uint8_t)(200 + 100) == 44` and
//! `(int32_t)(2e9 + 2e9) == -294967296`, a C program and its Ruby translation
//! genuinely produce the same observable results, wraparound included.
//!
//! The corpus prints unsigned results that can exceed `INT_MAX` with `%u` and
//! signed / small results with `%d`, because the lowering renders `printf`
//! by printing the stored (already-wrapped) integer value regardless of the
//! conversion specifier — so the reference `printf` must be handed a specifier
//! that prints that same value.
//!
//! Any leg whose toolchain is absent (`clang`/`ruby`/a C compiler) is skipped
//! gracefully; if the reference compiler is missing the whole test no-ops.

use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

static SEQ: AtomicUsize = AtomicUsize::new(0);

/// A per-(process, call) unique temp stem so parallel cases never collide.
fn uniq(ext: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "c3way_{}_{}{ext}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Write `bytes` to a *fresh* file, failing (rather than truncating) if the
/// path already exists.  `create_new` maps to `O_EXCL`/`CREATE_NEW`, so a
/// symlink pre-planted at our predictable temp path by a local attacker can't
/// redirect the write onto a victim file (CWE-59 / TOCTOU).
fn write_fresh(path: &std::path::Path, bytes: &[u8]) {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("fresh temp file")
        .write_all(bytes)
        .expect("write temp file");
}

/// A C compiler that honours `-fwrapv` (clang or gcc — *not* MSVC `cl`, whose
/// flag syntax differs).  `SIR_CC` points at clang in this repo's setup.
fn reference_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for c in ["clang", "gcc"] {
        if probe(c) {
            return Some(c.to_string());
        }
    }
    None
}

/// Any C compiler for the emitted (self-contained, well-defined) C — it does
/// its own masking, so it needs no `-fwrapv`.
fn emitted_cc() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for c in ["cc", "clang", "gcc"] {
        if probe(c) {
            return Some(c.to_string());
        }
    }
    None
}

fn probe(cmd: &str) -> bool {
    std::process::Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ruby_present() -> bool {
    std::process::Command::new("ruby")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn norm(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .trim_end()
        .to_string()
}

/// Compile `src` with the reference compiler + `-fwrapv`, run it, return stdout.
fn run_reference(cc: &str, src: &str) -> String {
    let cpath = uniq(".c");
    let exe = uniq(std::env::consts::EXE_SUFFIX);
    write_fresh(&cpath, src.as_bytes());
    let build = std::process::Command::new(cc)
        .args(["-std=c99", "-fwrapv", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("reference compiler runs");
    assert!(
        build.status.success(),
        "reference C failed to compile:\n{}\n--- source ---\n{src}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = std::process::Command::new(&exe)
        .output()
        .expect("reference exe runs");
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&exe);
    norm(&run.stdout)
}

/// Lower `src` to SIR, emit Ruby, run it with `ruby`, return stdout.
fn run_emitted_ruby(src: &str) -> String {
    let m = c_to_semantic_ir::compile_source(src, "conf").expect("C lowering");
    let ruby = semantic_ir_to_ruby::compile(&m).expect("ruby emit").source;
    let path = uniq(".rb");
    write_fresh(&path, ruby.as_bytes());
    let out = std::process::Command::new("ruby")
        .arg(&path)
        .output()
        .expect("ruby runs");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "emitted ruby failed:\n{}\n--- ruby ---\n{ruby}",
        String::from_utf8_lossy(&out.stderr)
    );
    norm(&out.stdout)
}

/// Lower `src` to SIR, emit C, compile + run it, return stdout.
fn run_emitted_c(cc: &str, src: &str) -> String {
    let m = c_to_semantic_ir::compile_source(src, "conf").expect("C lowering");
    let csrc = semantic_ir_to_c::compile(&m).expect("c emit").source;
    let cpath = uniq(".c");
    let exe = uniq(std::env::consts::EXE_SUFFIX);
    write_fresh(&cpath, csrc.as_bytes());
    let build = std::process::Command::new(cc)
        .args(["-std=c99", "-o"])
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("emitted C compiler runs");
    assert!(
        build.status.success(),
        "emitted C failed to compile:\n{}\n--- emitted C ---\n{csrc}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = std::process::Command::new(&exe)
        .output()
        .expect("emitted exe runs");
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&exe);
    norm(&run.stdout)
}

/// One corpus program plus a human label for failure messages.
struct Case {
    label: &'static str,
    src: &'static str,
}

/// The milestone-1 corpus.  Each program is a complete, compilable C source
/// (the `#include`s are stripped by the frontend but needed by the reference
/// compile).  Focus: unsigned overflow at every width, signed overflow via
/// cast/multiply, narrowing casts, promotion order, and multi-function calls.
fn corpus() -> Vec<Case> {
    vec![
        Case {
            label: "uint8 literal overflow → 44",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t c = 200 + 100; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint8 promotion order (a+b via locals) → 44",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t a = 200; uint8_t b = 100; uint8_t c = a + b; \
                   printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint8 narrowing cast (uint8_t)500 → 244",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t c = (uint8_t)500; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint8 multiply wrap 16*16 → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t c = 16 * 16; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint8 unsigned underflow 5-10 → 251",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t c = 5 - 10; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint16 add overflow 60000+10000 → 4464",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint16_t c = 60000 + 10000; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "uint16 multiply overflow 300*300 → 24464",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint16_t c = 300 * 300; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "int32 add overflow via cast → -294967296",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int32_t y = (int32_t)(2000000000 + 2000000000); \
                   printf(\"%d\\n\", y); return 0; }",
        },
        Case {
            label: "int32 multiply overflow 100000*100000 → 1410065408",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int32_t y = 100000 * 100000; printf(\"%d\\n\", y); return 0; }",
        },
        Case {
            label: "uint32 wrap of -1 (printed %u) → 4294967295",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint32_t z = -1; printf(\"%u\\n\", z); return 0; }",
        },
        Case {
            label: "operator precedence 2 + 3 * 4 → 14",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%d\\n\", 2 + 3 * 4); return 0; }",
        },
        Case {
            label: "multi-function call add(2,3) → 5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int add(int a, int b) { return a + b; }\n\
                   int main(void) { printf(\"%d\\n\", add(2, 3)); return 0; }",
        },
        // ── milestone 2: control flow & comparisons ──────────────────────────
        Case {
            label: "for-loop accumulator sum(1..100) → 5050",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint32_t sum = 0; \
                   for (int i = 1; i <= 100; i = i + 1) { sum = sum + i; } \
                   printf(\"%u\\n\", sum); return 0; }",
        },
        Case {
            label: "while countdown sum(5..1) → 15",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int32_t i = 5; int32_t acc = 0; \
                   while (i > 0) { acc = acc + i; i = i - 1; } \
                   printf(\"%d\\n\", acc); return 0; }",
        },
        Case {
            label: "if/else min(7,3) → 3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int min(int a, int b) { int r = 0; \
                   if (a < b) { r = a; } else { r = b; } return r; }\n\
                   int main(void) { printf(\"%d\\n\", min(7, 3)); return 0; }",
        },
        Case {
            label: "factorial loop 10! → 3628800",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint32_t f = 1; \
                   for (int i = 1; i <= 10; i = i + 1) { f = f * i; } \
                   printf(\"%u\\n\", f); return 0; }",
        },
        Case {
            // The headline of milestone 2: fixed-width wraparound survives loop
            // mutation — `x = x + 100` on a uint8_t, ten times, wraps every step.
            label: "uint8 wraparound accumulated in a loop → 232",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t x = 0; \
                   for (int i = 0; i < 10; i = i + 1) { x = x + 100; } \
                   printf(\"%d\\n\", x); return 0; }",
        },
        Case {
            label: "comparison as a value (5 > 3) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a = 5; int b = 3; int c = a > b; \
                   printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "equality branch classify(0) → 100",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int classify(int x) { int r = 0; \
                   if (x == 0) { r = 100; } else { r = 200; } return r; }\n\
                   int main(void) { printf(\"%d\\n\", classify(0)); return 0; }",
        },
        // ── milestone 3: early return ────────────────────────────────────────
        Case {
            // The headline: idiomatic recursion with a guard clause — impossible
            // before early-return lifting.
            label: "recursive fib(20) with a guard clause → 6765",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int fib(int n) { if (n < 2) { return n; } \
                   return fib(n - 1) + fib(n - 2); }\n\
                   int main(void) { printf(\"%d\\n\", fib(20)); return 0; }",
        },
        Case {
            label: "chained guard clauses sign(-5) → -1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int sign(int x) { if (x > 0) { return 1; } \
                   if (x < 0) { return -1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", sign(-5)); return 0; }",
        },
        Case {
            label: "unbraced guard clause (no block) sign2(0) → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int sign2(int x) { if (x > 0) return 1; if (x < 0) return -1; return 0; }\n\
                   int main(void) { printf(\"%d\\n\", sign2(0)); return 0; }",
        },
        Case {
            label: "if/else where both branches return, larger(7,3) → 7",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int larger(int a, int b) { if (a > b) { return a; } else { return b; } }\n\
                   int main(void) { printf(\"%d\\n\", larger(7, 3)); return 0; }",
        },
        Case {
            label: "statements before an early return, f(6) → 12",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int f(int x) { int y = x * 2; if (y > 10) { return y; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", f(6)); return 0; }",
        },
        Case {
            label: "nested if inside an else, deep(-5) → 300",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int deep(int x) { if (x == 0) { return 100; } \
                   else { if (x > 0) { return 200; } return 300; } }\n\
                   int main(void) { printf(\"%d\\n\", deep(-5)); return 0; }",
        },
        Case {
            // Early return still carries the width semantics through.
            label: "early return of a wrapped uint8 → 44",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int wrap8(int go) { uint8_t c = 200 + 100; if (go > 0) { return c; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", wrap8(1)); return 0; }",
        },
        Case {
            label: "early return combined with a loop result → 5050",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   uint32_t total(int go) { uint32_t s = 0; \
                   for (int i = 1; i <= 100; i = i + 1) { s = s + i; } \
                   if (go > 0) { return s; } return 0; }\n\
                   int main(void) { printf(\"%u\\n\", total(1)); return 0; }",
        },
        // ── milestone 4: logical operators (&& || !) ─────────────────────────
        Case {
            label: "&& range check in-range in_range(5) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int in_range(int x) { if (x >= 0 && x < 10) { return 1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", in_range(5)); return 0; }",
        },
        Case {
            label: "&& range check out-of-range in_range(15) → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int in_range(int x) { if (x >= 0 && x < 10) { return 1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", in_range(15)); return 0; }",
        },
        Case {
            label: "|| out-of-band far(-3) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int far(int x) { if (x < 0 || x > 100) { return 1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", far(-3)); return 0; }",
        },
        Case {
            label: "&& as a value both(3,0) → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int both(int a, int b) { return a && b; }\n\
                   int main(void) { printf(\"%d\\n\", both(3, 0)); return 0; }",
        },
        Case {
            label: "! as a value negate(0) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int negate(int x) { return !x; }\n\
                   int main(void) { printf(\"%d\\n\", negate(0)); return 0; }",
        },
        Case {
            label: "! in a condition not_big(3) → 100",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int not_big(int x) { if (!(x > 5)) { return 100; } return 200; }\n\
                   int main(void) { printf(\"%d\\n\", not_big(3)); return 0; }",
        },
        Case {
            label: "chained && excludes 50 band(50) → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int band(int x) { if (x > 0 && x < 100 && x != 50) { return 1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", band(50)); return 0; }",
        },
        Case {
            label: "mixed || / && / ! precedence classify(4) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int classify(int x, int y) { if ((x > 0 || y > 0) && !(x == y)) { return 1; } return 0; }\n\
                   int main(void) { printf(\"%d\\n\", classify(4, 4)); return 0; }",
        },
        // ── milestone 5: bitwise & | ^ ~ and shifts << >> ────────────────────
        Case {
            label: "bitwise and 0xF0 & 0x3C → 48",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%d\\n\", 0xF0 & 0x3C); return 0; }",
        },
        Case {
            label: "bitwise or | xor combine flags(7) → 5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int flags(int x) { return (x & 1) | (x & 4); }\n\
                   int main(void) { printf(\"%d\\n\", flags(7)); return 0; }",
        },
        Case {
            label: "xor 0xFF ^ 0x0F → 240",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%d\\n\", 0xFF ^ 0x0F); return 0; }",
        },
        Case {
            label: "~uint8 promotes to int, ~0 → -1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t x = 0; printf(\"%d\\n\", ~x); return 0; }",
        },
        Case {
            label: "(uint8_t)~0 narrows to 255",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t x = ~0; printf(\"%d\\n\", x); return 0; }",
        },
        Case {
            label: "left shift 1 << 10 → 1024",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%d\\n\", 1 << 10); return 0; }",
        },
        Case {
            label: "left shift into a uint8 wraps 1<<9 → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t x = 1 << 9; printf(\"%d\\n\", x); return 0; }",
        },
        Case {
            label: "unsigned right shift is logical 200u >> 1 → 100",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint32_t x = 200; printf(\"%u\\n\", x >> 1); return 0; }",
        },
        Case {
            label: "signed right shift is arithmetic (int8)-128 >> 1 → -64",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int8_t x = -128; printf(\"%d\\n\", x >> 1); return 0; }",
        },
        Case {
            label: "mask a value x & 0xFF → 52",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int lo(int x) { return x & 0xFF; }\n\
                   int main(void) { printf(\"%d\\n\", lo(0x1234)); return 0; }",
        },
        Case {
            // The signedness trap: a uint64 with bit 63 set is stored as a
            // negative int64, so `>>` must be *logical* — high-word extract.
            label: "uint64 logical right shift of a high-bit value → 2147483648",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint64_t x = 1; uint64_t y = x << 63; \
                   printf(\"%llu\\n\", (unsigned long long)(y >> 32)); return 0; }",
        },
        // ── milestone 6: division & modulo (truncate toward zero) ────────────
        // All four sign combinations — the whole point is that `/` truncates
        // (not floors) and `%` takes the sign of the dividend.
        Case {
            label: "div/mod + + : 7/2 → 3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int d(int a, int b) { return a / b; }\n\
                   int main(void) { printf(\"%d\\n\", d(7, 2)); return 0; }",
        },
        Case {
            label: "div - + truncates toward zero: -7/2 → -3 (not -4)",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int d(int a, int b) { return a / b; }\n\
                   int main(void) { printf(\"%d\\n\", d(-7, 2)); return 0; }",
        },
        Case {
            label: "div + - : 7/-2 → -3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int d(int a, int b) { return a / b; }\n\
                   int main(void) { printf(\"%d\\n\", d(7, -2)); return 0; }",
        },
        Case {
            label: "div - - : -7/-2 → 3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int d(int a, int b) { return a / b; }\n\
                   int main(void) { printf(\"%d\\n\", d(-7, -2)); return 0; }",
        },
        Case {
            label: "mod takes the dividend's sign: -7%2 → -1 (not 1)",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int m(int a, int b) { return a % b; }\n\
                   int main(void) { printf(\"%d\\n\", m(-7, 2)); return 0; }",
        },
        Case {
            label: "mod + - : 7%-2 → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int m(int a, int b) { return a % b; }\n\
                   int main(void) { printf(\"%d\\n\", m(7, -2)); return 0; }",
        },
        // (`INT_MIN / -1` is deliberately NOT a conformance case: it is signed-
        // overflow UB, and real x86 hardware *traps* (SIGFPE) on it even under
        // `-fwrapv`, so the reference program crashes rather than producing a
        // value.  The backends still guard it — see the emitted-C `_sir_itdiv`
        // INT64_MIN/-1 guard and the `int_min_div_is_guarded` unit test — so our
        // own output is defined, we just don't claim to match a trapping oracle.)
        Case {
            label: "unsigned division 100u / 7u → 14",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint32_t a = 100; uint32_t b = 7; \
                   printf(\"%u\\n\", a / b); return 0; }",
        },
        Case {
            // A uint64 with bit 63 set is a negative int64 in the backends'
            // storage, so a *signed* division would be wrong — unsigned `/` must
            // route to the uint64 path.
            label: "uint64 division of a high-bit value (2^63)/2 → 4611686018427387904",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint64_t x = 1; uint64_t hi = x << 63; \
                   printf(\"%llu\\n\", (unsigned long long)(hi / 2)); return 0; }",
        },
        Case {
            label: "uint64 modulo of a high-bit value (2^63 + 5) % 2 → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint64_t x = 1; uint64_t hi = (x << 63) + 5; \
                   printf(\"%llu\\n\", (unsigned long long)(hi % 2)); return 0; }",
        },
        Case {
            label: "gcd via a % b (Euclid) gcd(48,36) → 12",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int gcd(int a, int b) { while (b != 0) { int t = a % b; a = b; b = t; } return a; }\n\
                   int main(void) { printf(\"%d\\n\", gcd(48, 36)); return 0; }",
        },
        // ── milestone 7: per-block scoping (shadowing, re-used names) ────────
        // These were all *rejected* before; each is valid C whose inner binding
        // shadows an outer one without clobbering it.
        Case {
            label: "nested-block shadow doesn't touch the outer v → 1001",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int f(int x) { int v = 1; { uint8_t v = 250; v = v + 6; } return v + 1000; }\n\
                   int main(void) { printf(\"%d\\n\", f(0)); return 0; }",
        },
        Case {
            label: "shadow in a lifted else-branch, outer v survives → 1001",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int f(int x) { int v = 1; if (x > 0) { return 5; } \
                   else { uint8_t v = 250; } return v + 1000; }\n\
                   int main(void) { printf(\"%d\\n\", f(-1)); return 0; }",
        },
        // (A self-referential shadow like `int v = v + 5;` is deliberately not a
        // case: C scopes the inner `v` from its declarator — before the
        // initializer — so its RHS reads the *uninitialized* inner `v`, which is
        // UB.  We only conform on well-defined shadowing.)
        Case {
            label: "two sequential for-loops re-use i → 13",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int f(void) { int s = 0; \
                   for (int i = 0; i < 3; i = i + 1) { s = s + i; } \
                   for (int i = 0; i < 5; i = i + 1) { s = s + i; } return s; }\n\
                   int main(void) { printf(\"%d\\n\", f()); return 0; }",
        },
        Case {
            label: "shadowing a parameter in a block → 5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int f(int v) { { int v = 5; return v; } }\n\
                   int main(void) { printf(\"%d\\n\", f(99)); return 0; }",
        },
        // ── milestone 9: floating point ──────────────────────────────────────
        // Every result is cast to `(int)` *inside the C source* before `printf`,
        // so the reference `printf("%d", …)`, the emitted C, and the emitted
        // Ruby all format the same integer — the float display convention (which
        // diverges: C `%f` == "3.140000" vs Ruby "3.14") never enters the
        // comparison.  What is tested is the *floating-point arithmetic*: mixed
        // int/double promotion, true division, and float→int truncation.
        Case {
            label: "double literal + int promotion (int)(1 + 2.5) → 3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { double r = 1 + 2.5; printf(\"%d\\n\", (int)r); return 0; }",
        },
        Case {
            label: "true division (int)((7.0 / 2.0) * 10) → 35",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { double q = 7.0 / 2.0; printf(\"%d\\n\", (int)(q * 10.0)); return 0; }",
        },
        Case {
            label: "integer / is still truncating, not floated (7 / 2) → 3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int q = 7 / 2; printf(\"%d\\n\", q); return 0; }",
        },
        Case {
            label: "float→int cast truncates toward zero (int)(-3.9) → -3",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { double x = -3.9; printf(\"%d\\n\", (int)x); return 0; }",
        },
        Case {
            label: "double parameter and return, scaled area (int)area(3.0) → 12",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   double area(double r) { return r * r + r; }\n\
                   int main(void) { printf(\"%d\\n\", (int)area(3.0)); return 0; }",
        },
        Case {
            label: "double comparison as an int value (2.5 > 2.4) → 1",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int c = 2.5 > 2.4; printf(\"%d\\n\", c); return 0; }",
        },
        Case {
            label: "accumulate doubles in a loop, (int)sum(0.5 x 10) → 5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { double s = 0.0; \
                   for (int i = 0; i < 10; i = i + 1) { s = s + 0.5; } \
                   printf(\"%d\\n\", (int)s); return 0; }",
        },
        Case {
            label: "int widened to double mid-expression (int)(x / 2.0 * 4) → 10",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int x = 5; double r = x / 2.0 * 4.0; \
                   printf(\"%d\\n\", (int)r); return 0; }",
        },
        // ── milestone 11: fixed-size arrays ──────────────────────────────────
        Case {
            label: "array literal indexed read a[2] → 30",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[3] = {10, 20, 30}; printf(\"%d\\n\", a[2]); return 0; }",
        },
        Case {
            label: "array sized from initializer, a[0] → 5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[] = {5, 6, 7}; printf(\"%d\\n\", a[0]); return 0; }",
        },
        Case {
            label: "indexed write then read a[1] = 99 → 99",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[3] = {1, 2, 3}; a[1] = 99; printf(\"%d\\n\", a[1]); return 0; }",
        },
        Case {
            label: "sum an array in a loop {1..5} → 15",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[5] = {1, 2, 3, 4, 5}; int s = 0; \
                   for (int i = 0; i < 5; i = i + 1) { s = s + a[i]; } \
                   printf(\"%d\\n\", s); return 0; }",
        },
        Case {
            label: "fill an array in a loop then read a[4] → 8",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[5]; \
                   for (int i = 0; i < 5; i = i + 1) { a[i] = i * 2; } \
                   printf(\"%d\\n\", a[4]); return 0; }",
        },
        Case {
            label: "partial initializer zero-fills the rest, a[2] → 0",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[3] = {7}; printf(\"%d\\n\", a[0] + a[2]); return 0; }",
        },
        Case {
            label: "uint8 array element wraps on store, a[0] → 44",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { uint8_t a[2] = {0, 0}; a[0] = 200 + 100; \
                   printf(\"%d\\n\", a[0]); return 0; }",
        },
        Case {
            label: "computed index a[i+1] → 20",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { int a[3] = {10, 20, 30}; int i = 0; \
                   printf(\"%d\\n\", a[i + 1]); return 0; }",
        },
        // ── milestone 10: faithful printf (float DISPLAY, no (int) cast) ──────
        // These print doubles *directly* with `%f`/`%.Nf` and assert the emitted
        // text is byte-identical to reference C's printf — the payoff of the
        // faithful format lowering.  Restricted to FIXED notation (`%f`): the
        // exponent forms `%e`/`%g` format their exponent with a platform-varying
        // digit count (Windows libc `e+004` vs Ruby/UCRT `e+04`), so those are
        // supported by the lowering but deliberately kept out of the byte-
        // identical corpus.  Values avoid exact-half precision boundaries so C
        // and Ruby rounding never disagree.
        Case {
            label: "printf %f default precision 3.14 → 3.140000",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { double x = 3.14; printf(\"%f\\n\", x); return 0; }",
        },
        Case {
            label: "printf %.2f rounds 3.14159 → 3.14",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%.2f\\n\", 3.14159); return 0; }",
        },
        Case {
            label: "printf %.4f rounds up 3.14159 → 3.1416 with literal text",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"pi is about %.4f today\\n\", 3.14159); return 0; }",
        },
        Case {
            label: "printf %.1f of a computed double 7.0/2.0 → 3.5",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%.1f\\n\", 7.0 / 2.0); return 0; }",
        },
        Case {
            label: "printf negative double %.2f -1.5 → -1.50",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"%.2f\\n\", -1.5); return 0; }",
        },
        Case {
            label: "printf mixed %d and %.2f in one call → n=3 avg=2.50",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"n=%d avg=%.2f\\n\", 3, 2.5); return 0; }",
        },
        Case {
            label: "printf %.2f of a double param/return area(3.0) → 12.00",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   double area(double r) { return r * r + r; }\n\
                   int main(void) { printf(\"%.2f\\n\", area(3.0)); return 0; }",
        },
        Case {
            label: "printf literal-only format (no conversions) passes through",
            src: "#include <stdio.h>\n#include <stdint.h>\n\
                   int main(void) { printf(\"no args, 100%% literal\\n\"); return 0; }",
        },
    ]
}

#[test]
fn three_way_byte_identical_over_corpus() {
    let Some(ref_cc) = reference_cc() else {
        eprintln!("skipping: no reference C compiler (clang/gcc) for -fwrapv oracle");
        return;
    };
    let have_ruby = ruby_present();
    let emit_cc = emitted_cc();
    if !have_ruby {
        eprintln!("note: ruby absent — the emitted-Ruby leg is skipped");
    }
    if emit_cc.is_none() {
        eprintln!("note: no C compiler for the emitted-C leg — that leg is skipped");
    }

    for case in corpus() {
        let reference = run_reference(&ref_cc, case.src);

        if have_ruby {
            let ruby = run_emitted_ruby(case.src);
            assert_eq!(
                ruby, reference,
                "[{}] emitted Ruby stdout != reference C stdout",
                case.label
            );
        }

        if let Some(cc) = &emit_cc {
            let emitted = run_emitted_c(cc, case.src);
            assert_eq!(
                emitted, reference,
                "[{}] emitted C stdout != reference C stdout",
                case.label
            );
        }
    }
}

/// A focused sanity check on the two headline numbers, independent of the loop
/// above, so a corpus edit can never silently drop them.
#[test]
fn headline_wraparound_numbers() {
    let Some(ref_cc) = reference_cc() else {
        return;
    };
    let u8_overflow = "#include <stdio.h>\n#include <stdint.h>\n\
        int main(void) { uint8_t c = 200 + 100; printf(\"%d\\n\", c); return 0; }";
    let i32_overflow = "#include <stdio.h>\n#include <stdint.h>\n\
        int main(void) { int32_t y = (int32_t)(2000000000 + 2000000000); \
        printf(\"%d\\n\", y); return 0; }";

    assert_eq!(run_reference(&ref_cc, u8_overflow), "44");
    assert_eq!(run_reference(&ref_cc, i32_overflow), "-294967296");

    if ruby_present() {
        assert_eq!(run_emitted_ruby(u8_overflow), "44");
        assert_eq!(run_emitted_ruby(i32_overflow), "-294967296");
    }
    if let Some(cc) = emitted_cc() {
        assert_eq!(run_emitted_c(&cc, u8_overflow), "44");
        assert_eq!(run_emitted_c(&cc, i32_overflow), "-294967296");
    }
}

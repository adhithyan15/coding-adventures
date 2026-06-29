//! Differential **soundness** conformance harness for closurec's SIMPLE
//! optimizer.
//!
//! ## Why this exists
//!
//! The `tests/diff/*` fixtures lock the optimizer's *byte output* in place, but
//! a byte fixture can't tell you whether that output is *semantically correct* —
//! a plausible-looking literal can still be the wrong value. The classic trap is
//! negative zero: `Math.min(0, -0)` is `-0`, but a fold that emits the literal
//! `0` looks fine in a byte diff while silently flipping the sign bit. Two such
//! `-0` miscompiles were found by hand; this harness exists so the *next* one is
//! caught automatically.
//!
//! ## How it works (self-contained, no runtime JS engine in CI)
//!
//! For each entry we know the **true runtime value** of the source expression as
//! a canonical, `Object.is`-faithful string (the `golden` field). Those goldens
//! were generated once, offline, by Node/V8 (see `tests/conformance/README.md`)
//! — CI does **not** run Node. At test time we:
//!
//!   1. run closurec at `--compilation_level SIMPLE` on the source expression,
//!   2. parse the optimized output with a tiny *literal* evaluator below, and
//!   3. if the output folded all the way to a literal, assert its canonical
//!      value equals `golden` — i.e. the optimization preserved the value.
//!
//! Numbers reuse closurec's own V8-faithful `format_js_number` (the emitter
//! already prints the canonical decimal), so the canonical form is just the raw
//! token — no float reparsing, no formatting mismatch. If the output did **not**
//! fold to a pure literal (a declined or partial fold), there is nothing to
//! value-check: declining is always sound, and the byte fixtures cover it, so we
//! count it as `skipped` and move on (loudly — see the end-of-test summary, so a
//! silently-growing skip set can't hide a coverage hole).
//!
//! `KNOWN_DIVERGENCES` records inputs that closurec *currently* miscompiles, with
//! the value it wrongly produces. The test asserts the divergence is still
//! present; the day the underlying bug is fixed, that assertion fails and tells
//! us to promote the entry into `CORPUS`. (Today: the pre-existing unary-minus
//! `-0` → `0` flattening, tracked separately.)

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_closurec");

/// `(source expression, canonical value of its true runtime result)`.
/// Goldens generated offline by Node/V8 via `tests/conformance/gen_goldens.mjs`.
const CORPUS: &[(&str, &str)] = &[
    // ---- plain numeric literals ----
    ("3", "n:3"),
    ("1.5", "n:1.5"),
    ("-1", "n:-1"),
    // ---- string methods ----
    (r#""abcd".slice(1,3)"#, "s:bc"),
    (r#""ab".repeat(3)"#, "s:ababab"),
    (r#""HELLO".toLowerCase()"#, "s:hello"),
    (r#""abc".length"#, "n:3"),
    (r#""abcabc".indexOf("c")"#, "n:2"),
    // ---- string → array ----
    (r#""a,b,c".split(",")"#, "[s:a,s:b,s:c]"),
    // ---- static built-ins → primitive ----
    ("String.fromCharCode(65,66)", "s:AB"),
    ("Number.isInteger(5)", "b:true"),
    ("Array.isArray([])", "b:true"),
    ("isNaN(\"x\")", "b:true"),
    ("isFinite(3)", "b:true"),
    ("Boolean(0)", "b:false"),
    ("String(42)", "s:42"),
    ("Number(\"7\")", "n:7"),
    // ---- static built-ins → array / object ----
    ("Array.of(1,2,3)", "[n:1,n:2,n:3]"),
    ("Object.keys({})", "[]"),
    ("Object.entries({a:1,b:2})", "[[s:a,n:1],[s:b,n:2]]"),
    (r#"Object.fromEntries([["a",1],["b",2]])"#, "{k:a=n:1,k:b=n:2}"),
    // ---- folds that are not yet in `main` (cooking PRs) decline here and are
    //      reported as `skipped`; once their PR merges they fold and this
    //      harness begins value-checking them automatically ----
    ("Math.max(1,2,3)", "n:3"),
    ("Math.min(5,2,8)", "n:2"),
];

/// `(source, true value, value closurec WRONGLY produces today)`.
/// The test asserts the bug is still present; when fixed, promote to `CORPUS`.
const KNOWN_DIVERGENCES: &[(&str, &str, &str)] = &[
    // Pre-existing unary-minus fold flattens the `-0` literal to `0` (`var x=-0`
    // emits `var x=0`). Tracked as its own fix task, independent of any fold.
    ("-0", "n:-0", "n:0"),
];

/// Optimize one source *expression* at SIMPLE and return the emitted output with
/// the trailing `;`/newline stripped.
fn optimize(src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("closurec_conf_{}", sanitize(src)));
    std::fs::create_dir_all(&dir).expect("mk tmp dir");
    let path = dir.join("a.js");
    std::fs::write(&path, format!("{src};\n")).expect("write src");
    let out = Command::new(BINARY)
        .args([
            "--compilation_level",
            "SIMPLE",
            "--js",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("run closurec");
    assert!(
        out.status.success(),
        "closurec failed on {src:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string()
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(40)
        .collect()
}

// ------------------------------------------------------------------------
// A tiny literal evaluator: parses the subset of JS that a fully-folded SIMPLE
// output can be — number / string / boolean / null literals and arrays/objects
// of those — and produces the same canonical string the Node generator emits.
// Returns `None` the instant it meets anything non-literal (an identifier that
// isn't true/false/null, a call, etc.) — that means the fold declined, so there
// is no literal value to check.
// ------------------------------------------------------------------------

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn value(&mut self) -> Option<String> {
        self.ws();
        match self.peek()? {
            b'(' => {
                // statement-position object literals are wrapped: `({...})`
                self.i += 1;
                let v = self.value()?;
                self.ws();
                if self.peek()? != b')' {
                    return None;
                }
                self.i += 1;
                Some(v)
            }
            b'"' | b'\'' => self.string(),
            b'[' => self.array(),
            b'{' => self.object(),
            b'-' | b'0'..=b'9' | b'.' => self.number(),
            _ => self.keyword(),
        }
    }

    fn number(&mut self) -> Option<String> {
        let start = self.i;
        // closurec emits well-formed decimal/exponent tokens; read greedily.
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        if self.i == start {
            return None;
        }
        // The token is already V8-canonical (format_js_number); use it verbatim.
        Some(format!("n:{}", std::str::from_utf8(&self.s[start..self.i]).ok()?))
    }

    fn string(&mut self) -> Option<String> {
        let quote = self.s[self.i];
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = self.peek()?;
            self.i += 1;
            match c {
                b'\\' => {
                    let e = self.peek()?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'0' => out.push('\0'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        b'\'' => out.push('\''),
                        b'/' => out.push('/'),
                        b'u' => {
                            // \uXXXX (closurec does not emit \u{...} surrogate form)
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            let cp = u32::from_str_radix(hex, 16).ok()?;
                            self.i += 4;
                            out.push(char::from_u32(cp)?);
                        }
                        _ => return None,
                    }
                }
                _ if c == quote => break,
                _ => {
                    // copy a (possibly multi-byte UTF-8) char starting at c
                    let len = utf8_len(c);
                    out.push_str(std::str::from_utf8(&self.s[self.i - 1..self.i - 1 + len]).ok()?);
                    self.i += len - 1;
                }
            }
        }
        Some(format!("s:{out}"))
    }

    fn array(&mut self) -> Option<String> {
        self.i += 1; // [
        let mut parts = Vec::new();
        loop {
            self.ws();
            match self.peek()? {
                b']' => {
                    self.i += 1;
                    break;
                }
                b',' => {
                    // array hole, e.g. `[,1]`
                    self.i += 1;
                    parts.push("hole".to_string());
                    continue;
                }
                _ => {}
            }
            parts.push(self.value()?);
            self.ws();
            match self.peek()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    break;
                }
                _ => return None,
            }
        }
        Some(format!("[{}]", parts.join(",")))
    }

    fn object(&mut self) -> Option<String> {
        self.i += 1; // {
        let mut parts = Vec::new();
        loop {
            self.ws();
            if self.peek()? == b'}' {
                self.i += 1;
                break;
            }
            let key = self.key()?;
            self.ws();
            if self.peek()? != b':' {
                return None;
            }
            self.i += 1;
            let val = self.value()?;
            parts.push(format!("k:{key}={val}"));
            self.ws();
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    break;
                }
                _ => return None,
            }
        }
        Some(format!("{{{}}}", parts.join(",")))
    }

    fn key(&mut self) -> Option<String> {
        self.ws();
        match self.peek()? {
            b'"' | b'\'' => self.string().map(|s| s.trim_start_matches("s:").to_string()),
            b'0'..=b'9' | b'-' | b'.' => self.number().map(|s| s.trim_start_matches("n:").to_string()),
            _ => {
                // bare identifier key
                let start = self.i;
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'$') {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                if self.i == start {
                    return None;
                }
                Some(std::str::from_utf8(&self.s[start..self.i]).ok()?.to_string())
            }
        }
    }

    fn keyword(&mut self) -> Option<String> {
        for (kw, canon) in [("true", "b:true"), ("false", "b:false"), ("null", "null")] {
            if self.s[self.i..].starts_with(kw.as_bytes()) {
                self.i += kw.len();
                return Some(canon.to_string());
            }
        }
        None // an identifier / call / anything else → not a literal
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Canonicalize an optimized output *iff* it is a single pure literal; `None`
/// means the fold declined (no literal value to check).
fn canonicalize(optimized: &str) -> Option<String> {
    let mut p = P {
        s: optimized.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i == p.s.len() {
        Some(v)
    } else {
        None // trailing junk → not a single clean literal
    }
}

#[test]
fn conformance_optimized_values_match_oracle() {
    let mut checked = 0usize;
    let mut skipped: Vec<&str> = Vec::new();

    for (src, golden) in CORPUS {
        let opt = optimize(src);
        match canonicalize(&opt) {
            Some(canon) => {
                assert_eq!(
                    &canon, golden,
                    "VALUE MISMATCH (miscompile) for {src:?}\n  optimized: {opt}\n  closurec value: {canon}\n  true value:     {golden}",
                );
                checked += 1;
            }
            None => skipped.push(src),
        }
    }

    // Loud, non-silent accounting of coverage (per the no-silent-caps rule).
    eprintln!(
        "[conformance] value-checked {checked}/{} corpus entries; {} declined (no literal value to check): {:?}",
        CORPUS.len(),
        skipped.len(),
        skipped,
    );
    assert!(checked > 0, "expected at least some entries to fold to literals");
}

#[test]
fn conformance_known_divergences_still_diverge() {
    for (src, golden, wrong) in KNOWN_DIVERGENCES {
        let opt = optimize(src);
        let canon = canonicalize(&opt);
        assert_ne!(golden, wrong, "a KNOWN_DIVERGENCE must actually differ from the truth");
        assert_eq!(
            canon.as_deref(),
            Some(*wrong),
            "KNOWN_DIVERGENCE {src:?} no longer reproduces (optimized: {opt:?}). \
             If the underlying bug was fixed, promote this entry into CORPUS.",
        );
    }
}

// ---- self-tests for the literal evaluator (so a parser bug can't mask a
//      real miscompile by silently returning None) ----

#[test]
fn canonicalizer_self_test() {
    assert_eq!(canonicalize("3").as_deref(), Some("n:3"));
    assert_eq!(canonicalize("-1").as_deref(), Some("n:-1"));
    assert_eq!(canonicalize("1.5").as_deref(), Some("n:1.5"));
    assert_eq!(canonicalize("0").as_deref(), Some("n:0"));
    assert_eq!(canonicalize(r#""bc""#).as_deref(), Some("s:bc"));
    assert_eq!(canonicalize("true").as_deref(), Some("b:true"));
    assert_eq!(canonicalize("false").as_deref(), Some("b:false"));
    assert_eq!(canonicalize("null").as_deref(), Some("null"));
    assert_eq!(canonicalize("[1,2,3]").as_deref(), Some("[n:1,n:2,n:3]"));
    assert_eq!(canonicalize("[]").as_deref(), Some("[]"));
    assert_eq!(
        canonicalize(r#"[["a",1],["b",2]]"#).as_deref(),
        Some("[[s:a,n:1],[s:b,n:2]]")
    );
    assert_eq!(
        canonicalize("({a:1,b:2})").as_deref(),
        Some("{k:a=n:1,k:b=n:2}")
    );
    assert_eq!(canonicalize(r#"{"1":"x"}"#).as_deref(), Some("{k:1=s:x}"));
    // non-literals decline:
    assert_eq!(canonicalize("Math.max(1,2,3)"), None);
    assert_eq!(canonicalize("o.fromEntries([])"), None);
    assert_eq!(canonicalize("x"), None);
}

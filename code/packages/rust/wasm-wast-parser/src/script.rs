//! # Script-directive parsing — a whole `.wast` file (modules + directives).
//!
//! The official WebAssembly spec testsuite ships as `.wast` files: a
//! sequence of top-level forms mixing real `(module ...)` definitions with
//! **test directives** — `register`, `invoke`, `assert_return`,
//! `assert_trap`, `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
//! `assert_unlinkable`. This module recognizes that outer shape; it does
//! not itself decide pass/fail — that's `wasm-conformance`'s job, this
//! crate just hands back a typed [`Directive`] per top-level form for the
//! harness to execute and grade.
//!
//! ## A deliberate asymmetry: eager vs. lazy module building
//!
//! A `(module ...)` directive is built **eagerly**, whichever of its three
//! source forms it uses (plain text, `quote` text, or `binary` bytes) —
//! encoded to a real [`WasmModule`] right here, propagating any real syntax
//! error up through this function's own `Result`, since `assert_return`/
//! `assert_trap` need an already-valid module to invoke against.
//!
//! `assert_invalid`/`assert_malformed`'s module, by contrast, is kept as a
//! **raw, unparsed [`ModuleSource`]** — because for these two directive
//! kinds, failing to parse or encode is exactly the thing the harness is
//! *testing for*. Eagerly building it here would turn every legitimate
//! `assert_malformed` fixture into a hard error that aborts the whole
//! script. The harness calls [`crate::module::parse_module_expr`] (or the
//! `quote`/`binary` equivalents) itself, at the point it actually wants to
//! observe whether that call succeeds or fails.

use crate::module::parse_module_expr;
use crate::numeric::{parse_f32_bits, parse_f64_bits, parse_i32, parse_i64};
use crate::sexpr::{expect_get, parse_source, SExpr};
use crate::WastParseError;
use wasm_types::WasmModule;

/// A test action: invoke an exported function, or read an exported global.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Invoke { module: Option<String>, name: String, args: Vec<ConstValue> },
    Get { module: Option<String>, name: String },
}

/// A concrete constant value — either a literal argument to `invoke`, or an
/// exact expected result in `assert_return`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstValue {
    I32(i32),
    I64(i64),
    /// Stored as raw bits, not `f32`/`f64` — `PartialEq` on floats is the
    /// wrong notion of equality for conformance grading (NaN != NaN, but
    /// two NaNs with the same bit pattern ARE the same test outcome); the
    /// harness compares bits directly, never `==` on the float value.
    F32Bits(u32),
    F64Bits(u64),
    /// A reference value (WASM17): `None` for an exact `(ref.null func)` /
    /// `(ref.null extern)` literal, `Some(n)` for a `(ref.extern n)`
    /// literal (the official testsuite's own script-syntax convenience for
    /// constructing an externref test value from a plain integer — not a
    /// real WASM instruction, see `code/specs/
    /// W08-wasm-funcref-externref.md`). The bare, type-less `(ref.null)`
    /// and `(ref.func)` wildcard forms are NOT representable here — they
    /// only appear as `assert_return` expectations, never as an exact
    /// argument or result, so they live in [`Expected`] instead.
    Ref(Option<u32>),
    /// A `v128.const` literal's raw 16 bytes (SIMD PR1b-3), already packed
    /// by [`crate::module::parse_v128_const`] regardless of which of the 6
    /// text shapes (`i8x16`/.../`f64x2`) wrote them — grading compares
    /// these bytes exactly, there's no shape tag to preserve since the
    /// runtime value itself (`wasm_execution::V128Bytes`) has none either.
    V128([u8; 16]),
}

/// One expected result slot in `assert_return` — either an exact value, or
/// (float-only) a NaN *class*: any quiet or signaling NaN bit pattern
/// satisfies `NanArithmetic`; only the canonical quiet-NaN bit pattern (or
/// its negation) satisfies `NanCanonical`. See the WebAssembly spec's own
/// NaN propagation rules for why exact NaN payloads aren't always
/// deterministic across conforming implementations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Expected {
    Value(ConstValue),
    NanCanonicalF32,
    NanArithmeticF32,
    NanCanonicalF64,
    NanArithmeticF64,
    /// Bare `(ref.null)` with no heap-type keyword (WASM17) — matches the
    /// null reference of ANY reference type. Used by the real testsuite
    /// when the static result type is ambiguous at the script level (e.g.
    /// `select.wast`'s `join-funcnull` / `global.wast`'s table-initialized
    /// `get-elem`). Distinct from `Value(ConstValue::Ref(None))`, which is
    /// an EXACT `(ref.null func)`/`(ref.null extern)` literal.
    RefNullAny,
    /// Bare `(ref.func)` with no function-index argument (WASM17) —
    /// matches ANY non-null `funcref`, since the real testsuite can't
    /// predict which specific function pointer a table lookup returns,
    /// only that it isn't null.
    RefFuncAny,
}

/// The three ways a `.wast` script can embed a module for `assert_invalid`/
/// `assert_malformed`/`assert_unlinkable` — captured RAW, not built, since
/// these directives test whether building it fails. `Text` carries the
/// original `(module ...)` [`SExpr`] for `parse_module_expr` to re-attempt;
/// `Binary`/`Quote` carry the concatenated raw bytes/text a caller can feed
/// to `wasm-module-parser`/this crate's own text path respectively.
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleSource {
    Text(SExpr),
    Binary(Vec<u8>),
    Quote(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Module(WasmModule),
    Register { name: String, module_name: Option<String> },
    Action(Action),
    AssertReturn { action: Action, expected: Vec<Expected> },
    AssertTrap { action: Action, message: String },
    AssertExhaustion { action: Action, message: String },
    AssertInvalid { module: ModuleSource, message: String },
    AssertMalformed { module: ModuleSource, message: String },
    AssertUnlinkable { module: ModuleSource, message: String },
}

pub fn parse_script(src: &str) -> Result<Vec<Directive>, WastParseError> {
    let exprs = parse_source(src)?;
    exprs.iter().map(parse_directive).collect()
}

fn parse_directive(e: &SExpr) -> Result<Directive, WastParseError> {
    let items = e.as_list().ok_or(WastParseError::UnexpectedToken {
        pos: e.pos(),
        found: "atom".into(),
        expected: "a top-level script form",
    })?;
    let head = items.first().and_then(|i| i.as_atom()).ok_or(WastParseError::UnexpectedToken {
        pos: e.pos(),
        found: "".into(),
        expected: "a directive keyword",
    })?;
    match head {
        "module" => Ok(Directive::Module(build_module_directive(e)?)),
        "register" => {
            let name = expect_str(expect_get(items, 1)?)?;
            let module_name = items.get(2).and_then(|m| m.as_atom()).map(|s| s.to_string());
            Ok(Directive::Register { name, module_name })
        }
        "invoke" | "get" => Ok(Directive::Action(parse_action(e)?)),
        "assert_return" => {
            let action = parse_action(expect_get(items, 1)?)?;
            let expected = items.get(2..).unwrap_or(&[]).iter().map(parse_expected).collect::<Result<_, _>>()?;
            Ok(Directive::AssertReturn { action, expected })
        }
        "assert_trap" => {
            let (action_or_module, message) = parse_assert_with_message(items)?;
            match action_or_module {
                ActionOrModule::Action(a) => Ok(Directive::AssertTrap { action: a, message }),
                ActionOrModule::Module(m) => Ok(Directive::AssertUnlinkable { module: m, message }),
            }
        }
        "assert_exhaustion" => {
            let action = parse_action(expect_get(items, 1)?)?;
            let message = expect_str(expect_get(items, 2)?)?;
            Ok(Directive::AssertExhaustion { action, message })
        }
        "assert_invalid" => {
            let module = parse_module_source(expect_get(items, 1)?)?;
            let message = expect_str(expect_get(items, 2)?)?;
            Ok(Directive::AssertInvalid { module, message })
        }
        "assert_malformed" => {
            let module = parse_module_source(expect_get(items, 1)?)?;
            let message = expect_str(expect_get(items, 2)?)?;
            Ok(Directive::AssertMalformed { module, message })
        }
        "assert_unlinkable" => {
            let module = parse_module_source(expect_get(items, 1)?)?;
            let message = expect_str(expect_get(items, 2)?)?;
            Ok(Directive::AssertUnlinkable { module, message })
        }
        other => Err(WastParseError::UnexpectedToken { pos: e.pos(), found: other.to_string(), expected: "a known directive" }),
    }
}

enum ActionOrModule {
    Action(Action),
    Module(ModuleSource),
}

/// `assert_trap` takes either `(invoke/get ...)` (a runtime trap) OR a
/// `(module ...)` form (a *link-time* trap, which the spec's own test
/// corpus files as `assert_trap` even though every other unlinkable case
/// uses `assert_unlinkable` — both shapes carry the identical `(thing,
/// message)` structure, so this one helper serves `assert_trap` and
/// `assert_unlinkable` alike.
fn parse_assert_with_message(items: &[SExpr]) -> Result<(ActionOrModule, String), WastParseError> {
    let thing = expect_get(items, 1)?;
    let message = expect_str(expect_get(items, 2)?)?;
    if thing.is_keyword_list("invoke") || thing.is_keyword_list("get") {
        Ok((ActionOrModule::Action(parse_action(thing)?), message))
    } else {
        Ok((ActionOrModule::Module(parse_module_source(thing)?), message))
    }
}

fn parse_action(e: &SExpr) -> Result<Action, WastParseError> {
    let items = e.as_list().ok_or(WastParseError::UnexpectedToken { pos: e.pos(), found: "".into(), expected: "an action" })?;
    match items.first().and_then(|i| i.as_atom()) {
        Some("invoke") => {
            // `(invoke $module? "name" arg-expr*)` -- the optional module
            // reference, if present, is a bare `$name` atom right after
            // `invoke`, distinguishing it from the always-quoted export name.
            let mut i = 1;
            let module = if matches!(items.get(i), Some(SExpr::Atom(s, _)) if s.starts_with('$')) {
                let m = items[i].as_atom().unwrap().to_string();
                i += 1;
                Some(m)
            } else {
                None
            };
            let name = expect_str(expect_get(items, i)?)?;
            i += 1;
            let args = items.get(i..).unwrap_or(&[]).iter().map(parse_const_value).collect::<Result<_, _>>()?;
            Ok(Action::Invoke { module, name, args })
        }
        Some("get") => {
            let mut i = 1;
            let module = if matches!(items.get(i), Some(SExpr::Atom(s, _)) if s.starts_with('$')) {
                let m = items[i].as_atom().unwrap().to_string();
                i += 1;
                Some(m)
            } else {
                None
            };
            let name = expect_str(expect_get(items, i)?)?;
            Ok(Action::Get { module, name })
        }
        other => Err(WastParseError::UnexpectedToken {
            pos: e.pos(),
            found: other.unwrap_or("").to_string(),
            expected: "'invoke' or 'get'",
        }),
    }
}

/// Parse a `(i32.const 1)`-shaped const expression to a [`ConstValue`] —
/// the argument-list shape `assert_return`/`invoke` both use for concrete
/// values (as opposed to [`parse_expected`], which additionally accepts
/// the NaN-class result forms only `assert_return` allows).
fn parse_const_value(e: &SExpr) -> Result<ConstValue, WastParseError> {
    let items = e.as_list().ok_or(WastParseError::UnexpectedToken { pos: e.pos(), found: "".into(), expected: "a const expression" })?;
    let head = expect_get(items, 0)?;
    let (kind, pos) = (head.as_atom().unwrap_or(""), head.pos());
    let lit = items.get(1).and_then(|a| a.as_atom()).ok_or(WastParseError::UnexpectedEof)?;
    match kind {
        "i32.const" => Ok(ConstValue::I32(parse_i32(lit, pos)?)),
        "i64.const" => Ok(ConstValue::I64(parse_i64(lit, pos)?)),
        "f32.const" => Ok(ConstValue::F32Bits(parse_f32_bits(lit, pos)?)),
        "f64.const" => Ok(ConstValue::F64Bits(parse_f64_bits(lit, pos)?)),
        // `(v128.const <shape> <lane0> ... <laneN-1>)` (SIMD PR1b-3) --
        // reuses `wasm-wast-parser`'s own instruction-syntax literal
        // parser directly (`items[1..]` is exactly the `<shape> <lanes...>`
        // operand list `parse_v128_const` expects), so all 6 shapes are
        // supported here for free, same as in real instruction bodies.
        "v128.const" => {
            let (bytes, _consumed) = crate::module::parse_v128_const(&items[1..], pos)?;
            Ok(ConstValue::V128(bytes))
        }
        // `(ref.null func)` / `(ref.null extern)` (WASM17) -- an EXACT null
        // literal. The bare, heap-type-less `(ref.null)` wildcard never
        // reaches this function; `parse_expected` intercepts it first (see
        // that function's own match) since it's only valid as an
        // `assert_return` expectation, never a concrete argument or result.
        "ref.null" => match lit {
            "func" | "extern" => Ok(ConstValue::Ref(None)),
            other => Err(WastParseError::UnexpectedToken { pos, found: other.to_string(), expected: "func or extern" }),
        },
        // `(ref.extern N)` (WASM17) -- the testsuite's own script-syntax
        // convenience for an externref test value; not a real instruction.
        "ref.extern" => {
            let n = lit.parse::<u32>().map_err(|_| WastParseError::UnexpectedToken { pos, found: lit.to_string(), expected: "an integer" })?;
            Ok(ConstValue::Ref(Some(n)))
        }
        other => Err(WastParseError::UnexpectedToken { pos, found: other.to_string(), expected: "a *.const expression" }),
    }
}

fn parse_expected(e: &SExpr) -> Result<Expected, WastParseError> {
    let items = e.as_list().ok_or(WastParseError::UnexpectedToken { pos: e.pos(), found: "".into(), expected: "an expected result" })?;
    let kind = expect_get(items, 0)?.as_atom().unwrap_or("");
    let lit = items.get(1).and_then(|a| a.as_atom());
    match (kind, lit) {
        ("f32.const", Some("nan:canonical")) => Ok(Expected::NanCanonicalF32),
        ("f32.const", Some("nan:arithmetic")) => Ok(Expected::NanArithmeticF32),
        ("f64.const", Some("nan:canonical")) => Ok(Expected::NanCanonicalF64),
        ("f64.const", Some("nan:arithmetic")) => Ok(Expected::NanArithmeticF64),
        // Bare `(ref.null)` / `(ref.func)` (WASM17) -- wildcard expectations
        // only meaningful as an `assert_return` result, see [`Expected`]'s
        // own doc comments. `lit` is `None` here precisely because these
        // forms carry no second element at all.
        ("ref.null", None) => Ok(Expected::RefNullAny),
        ("ref.func", None) => Ok(Expected::RefFuncAny),
        _ => Ok(Expected::Value(parse_const_value(e)?)),
    }
}

/// Build a real [`WasmModule`] for an eagerly-built `(module ...)`
/// **directive** -- as opposed to `assert_invalid`/`assert_malformed`'s
/// module, which stays a raw [`ModuleSource`] (see this file's module doc
/// comment). Routes through whichever of the three source kinds the
/// directive actually used: plain text via [`parse_module_expr`], `quote`
/// text via this crate's own [`crate::module::parse_module`] (which accepts
/// both the explicit `(module ...)` form and the WAT "abbreviated module"
/// bare-field-list form -- the official testsuite's `comments.wast` and
/// `block.wast` use the latter for their `module quote` directives), and
/// `binary` bytes via `wasm-module-parser`.
fn build_module_directive(e: &SExpr) -> Result<WasmModule, WastParseError> {
    match parse_module_source(e)? {
        ModuleSource::Text(expr) => parse_module_expr(&expr),
        ModuleSource::Quote(bytes) => {
            let text = std::str::from_utf8(&bytes).map_err(|_| WastParseError::InvalidUtf8 { pos: e.pos() })?;
            crate::module::parse_module(text)
        }
        ModuleSource::Binary(bytes) => wasm_module_parser::WasmModuleParser::parse(&bytes)
            .map_err(|err| WastParseError::EmbeddedBinaryModuleError { pos: e.pos(), message: err.to_string() }),
    }
}

fn parse_module_source(e: &SExpr) -> Result<ModuleSource, WastParseError> {
    let items = e.as_list().ok_or(WastParseError::UnexpectedToken { pos: e.pos(), found: "".into(), expected: "a module form" })?;
    // Skip `module`, an optional `$name`, then look for a `binary`/`quote`
    // tag -- its absence means this is a plain text module.
    let mut i = 1;
    if matches!(items.get(i), Some(SExpr::Atom(s, _)) if s.starts_with('$')) {
        i += 1;
    }
    match items.get(i).and_then(|a| a.as_atom()) {
        Some("binary") => {
            let bytes = concat_strings(&items[i + 1..])?;
            Ok(ModuleSource::Binary(bytes))
        }
        Some("quote") => {
            let bytes = concat_strings(&items[i + 1..])?;
            Ok(ModuleSource::Quote(bytes))
        }
        _ => Ok(ModuleSource::Text(e.clone())),
    }
}

fn concat_strings(items: &[SExpr]) -> Result<Vec<u8>, WastParseError> {
    let mut out = Vec::new();
    for it in items {
        match it {
            SExpr::Str(b, _) => out.extend_from_slice(b),
            other => return Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "".into(), expected: "a string literal" }),
        }
    }
    Ok(out)
}

fn expect_str(e: &SExpr) -> Result<String, WastParseError> {
    match e {
        SExpr::Str(b, _) => Ok(String::from_utf8_lossy(b).to_string()),
        other => Err(WastParseError::UnexpectedToken { pos: other.pos(), found: "".into(), expected: "a string literal" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_module_directive() {
        let dirs = parse_script("(module (func (result i32) (i32.const 42)))").unwrap();
        assert_eq!(dirs.len(), 1);
        assert!(matches!(dirs[0], Directive::Module(_)));
    }

    #[test]
    fn parses_assert_return_with_exact_values() {
        let dirs = parse_script(r#"(assert_return (invoke "f" (i32.const 1) (i64.const 2)) (i32.const 3))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { action: Action::Invoke { name, args, module }, expected } => {
                assert_eq!(name, "f");
                assert_eq!(module, &None);
                assert_eq!(args, &vec![ConstValue::I32(1), ConstValue::I64(2)]);
                assert_eq!(expected, &vec![Expected::Value(ConstValue::I32(3))]);
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn parses_assert_return_nan_classes() {
        let dirs = parse_script(r#"(assert_return (invoke "f") (f32.const nan:canonical) (f64.const nan:arithmetic))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(expected, &vec![Expected::NanCanonicalF32, Expected::NanArithmeticF64]);
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn parses_assert_trap_with_message() {
        let dirs = parse_script(r#"(assert_trap (invoke "div0" (i32.const 1) (i32.const 0)) "integer divide by zero")"#).unwrap();
        match &dirs[0] {
            Directive::AssertTrap { action: Action::Invoke { name, .. }, message } => {
                assert_eq!(name, "div0");
                assert_eq!(message, "integer divide by zero");
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn parses_register_directive() {
        let dirs = parse_script(r#"(register "test-module" $M)"#).unwrap();
        assert_eq!(dirs[0], Directive::Register { name: "test-module".to_string(), module_name: Some("$M".to_string()) });
    }

    #[test]
    fn parses_get_action() {
        let dirs = parse_script(r#"(assert_return (get "g") (i32.const 42))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { action: Action::Get { name, .. }, .. } => assert_eq!(name, "g"),
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn assert_invalid_captures_raw_module_without_building_it() {
        // A module that WOULD fail type-checking (an i32 result declared,
        // an i64 actually produced) if we tried to encode it eagerly --
        // proves this directive kind doesn't call parse_module_expr here.
        let dirs = parse_script(
            r#"(assert_invalid (module (func (result i32) (i64.const 1))) "type mismatch")"#,
        )
        .unwrap();
        match &dirs[0] {
            Directive::AssertInvalid { module: ModuleSource::Text(_), message } => {
                assert_eq!(message, "type mismatch");
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn assert_malformed_binary_variant_concatenates_string_bytes() {
        let dirs = parse_script(r#"(assert_malformed (module binary "\00\61\73\6d" "\01\00\00\00") "bad")"#).unwrap();
        match &dirs[0] {
            Directive::AssertMalformed { module: ModuleSource::Binary(bytes), .. } => {
                assert_eq!(bytes, &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn module_quote_directive_builds_a_real_module_from_the_abbreviated_form() {
        // Matches the official testsuite's comments.wast/block.wast shape:
        // the quoted strings concatenate to bare fields with no enclosing
        // `(module ...)` -- this directive kind (unlike assert_malformed's
        // ModuleSource::Quote) must actually BUILD a usable module, since
        // `assert_return` invokes an export from it.
        let dirs = parse_script(r#"(module quote "(func (export \"f\") (result i32) (i32.const 42))")"#).unwrap();
        match &dirs[0] {
            Directive::Module(m) => {
                assert_eq!(m.functions.len(), 1);
                assert_eq!(m.exports[0].name, "f");
            }
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn module_binary_directive_builds_a_real_module_from_decoded_bytes() {
        // The WASM magic + version bytes alone decode to a valid, empty
        // module -- proves this directive kind actually routes `binary`
        // bytes through wasm-module-parser instead of silently discarding
        // them (the way the old code did, since `parse_module_expr`
        // doesn't recognize a bare "binary" atom as a field).
        let dirs = parse_script(r#"(module binary "\00\61\73\6d\01\00\00\00")"#).unwrap();
        match &dirs[0] {
            Directive::Module(m) => assert_eq!(m, &wasm_types::WasmModule::default()),
            other => panic!("unexpected directive: {other:?}"),
        }
    }

    #[test]
    fn multi_directive_script_parses_in_order() {
        let dirs = parse_script(
            r#"(module $m (func (export "f") (result i32) (i32.const 42)))
               (assert_return (invoke $m "f") (i32.const 42))"#,
        )
        .unwrap();
        assert_eq!(dirs.len(), 2);
        assert!(matches!(dirs[0], Directive::Module(_)));
        assert!(matches!(&dirs[1], Directive::AssertReturn { action: Action::Invoke { module: Some(_), .. }, .. }));
    }

    // ── Security-review regressions: a directive missing a required
    // trailing field must produce a clean Err, never index-panic. Every
    // directive kind that indexes a positional field is covered. ─────────

    #[test]
    fn register_missing_name_errors_cleanly_not_panics() {
        assert!(matches!(parse_script("(register)"), Err(WastParseError::UnexpectedEof)));
    }

    #[test]
    fn assert_return_missing_action_errors_cleanly_not_panics() {
        assert!(matches!(parse_script("(assert_return)"), Err(WastParseError::UnexpectedEof)));
    }

    #[test]
    fn assert_exhaustion_missing_message_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_exhaustion (invoke "f"))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn assert_trap_missing_message_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_trap (invoke "f" (i32.const 1)))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn assert_malformed_missing_message_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_malformed (module binary "\00"))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn invoke_with_empty_list_argument_errors_cleanly_not_panics() {
        // `()` as an arg where a const expression is expected -- empty
        // list, so `items[0]` in the old code would index-panic.
        let err = parse_script(r#"(assert_return (invoke "f" ()) (i32.const 1))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedEof));
    }

    #[test]
    fn action_with_no_kind_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_return () (i32.const 1))"#).unwrap_err();
        assert!(matches!(
            err,
            WastParseError::UnexpectedToken { .. } | WastParseError::UnexpectedEof
        ));
    }

    // ── WASM17: ref.null / ref.func / ref.extern script literals ────────────

    #[test]
    fn v128_const_literal_as_assert_return_expected_value() {
        // The real corpus's own shape (e.g. simd_splat.wast): `(assert_return
        // (invoke "i8x16.splat" (i32.const 5)) (v128.const i8x16 5 5 ... 5))`.
        // Reuses `wasm-wast-parser`'s own instruction-syntax literal parser
        // (SIMD PR1b-3) -- all 6 shapes work here for free, same as in a
        // real instruction body.
        let dirs = parse_script(
            r#"(assert_return (invoke "f" (i32.const 5)) (v128.const i32x4 5 5 5 5))"#,
        )
        .unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                let mut want = [0u8; 16];
                for lane in 0..4 {
                    want[lane * 4..lane * 4 + 4].copy_from_slice(&5i32.to_le_bytes());
                }
                assert_eq!(*expected, vec![Expected::Value(ConstValue::V128(want))]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn ref_extern_literal_as_invoke_arg_and_exact_expected_value() {
        // The real corpus's own shape: `(assert_return (invoke "f"
        // (ref.extern 1) (ref.extern 2)) (ref.extern 1))`.
        let dirs = parse_script(
            r#"(assert_return (invoke "f" (ref.extern 1) (ref.extern 2)) (ref.extern 1))"#,
        )
        .unwrap();
        match &dirs[0] {
            Directive::AssertReturn { action: Action::Invoke { args, .. }, expected } => {
                assert_eq!(*args, vec![ConstValue::Ref(Some(1)), ConstValue::Ref(Some(2))]);
                assert_eq!(*expected, vec![Expected::Value(ConstValue::Ref(Some(1)))]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn ref_null_func_and_extern_literals_are_exact_null() {
        let dirs = parse_script(
            r#"(assert_return (invoke "f" (ref.null func)) (ref.null extern))"#,
        )
        .unwrap();
        match &dirs[0] {
            Directive::AssertReturn { action: Action::Invoke { args, .. }, expected } => {
                assert_eq!(*args, vec![ConstValue::Ref(None)]);
                assert_eq!(*expected, vec![Expected::Value(ConstValue::Ref(None))]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn bare_ref_null_and_ref_func_are_wildcard_expectations() {
        // Bare forms (no heap-type keyword / no funcidx) only ever appear
        // as `assert_return` expectations, matching "any null" / "any
        // non-null funcref" respectively -- see select.wast's
        // `join-funcnull` and global.wast's table-initialized `get-elem`.
        let dirs = parse_script(r#"(assert_return (invoke "f") (ref.null) (ref.func))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(*expected, vec![Expected::RefNullAny, Expected::RefFuncAny]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn ref_extern_non_integer_literal_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_return (invoke "f") (ref.extern nope))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
    }
}

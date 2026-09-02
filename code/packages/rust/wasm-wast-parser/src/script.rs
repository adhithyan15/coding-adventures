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

/// One `f32` LANE of a `v128.const f32x4` literal used as an
/// `assert_return` EXPECTED value (SIMD widen PR28) — either an exact bit
/// pattern, or a NaN *class*, same `nan:canonical`/`nan:arithmetic`
/// vocabulary as [`Expected::NanCanonicalF32`]/[`Expected::NanArithmeticF32`]
/// already support for a whole scalar `f32` result. Needed because a
/// `v128.const`'s existing [`ConstValue::V128`] representation is exact
/// BYTES only — it has no way to say "this lane must be SOME NaN, exact
/// payload unconstrained" the way the scalar `Expected` variants can for a
/// single float. First needed by `simd_conversions.wast`'s
/// `f64x2.promote_low_f32x4`/`f32x4.demote_f64x2_zero` NaN-payload
/// directives: promoting/demoting a NaN can canonicalize its payload (see
/// `wasm-execution`'s `SimdOpKind::DemoteF64x2Zero`/`PromoteLowF32x4` doc
/// comments), so the upstream corpus itself expects a NaN *class* per
/// lane, not an exact payload.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum F32LaneExpected {
    Exact(u32),
    NanCanonical,
    NanArithmetic,
}

/// The `f64`-lane counterpart of [`F32LaneExpected`], used by
/// `v128.const f64x2` expected values (SIMD widen PR28) -- same three
/// cases, `f64` width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum F64LaneExpected {
    Exact(u64),
    NanCanonical,
    NanArithmetic,
}

/// One expected result slot in `assert_return` — either an exact value, or
/// (float-only) a NaN *class*: any quiet or signaling NaN bit pattern
/// satisfies `NanArithmetic`; only the canonical quiet-NaN bit pattern (or
/// its negation) satisfies `NanCanonical`. See the WebAssembly spec's own
/// NaN propagation rules for why exact NaN payloads aren't always
/// deterministic across conforming implementations.
///
/// No longer `Copy` as of [`Self::Either`] (relaxed SIMD epic PR1) — a
/// `Box<Expected>` can't be. Every existing call site already took
/// `&Expected`/moved a freshly-constructed value, so this cost nothing;
/// confirmed by this crate's own test suite passing unchanged.
#[derive(Debug, Clone, PartialEq)]
pub enum Expected {
    Value(ConstValue),
    NanCanonicalF32,
    NanArithmeticF32,
    NanCanonicalF64,
    NanArithmeticF64,
    /// `(v128.const f32x4 lane0 lane1 lane2 lane3)` used as an
    /// `assert_return` expected value where AT LEAST ONE lane is a NaN
    /// class (`nan:canonical`/`nan:arithmetic`) rather than an exact
    /// literal (SIMD widen PR28) -- see [`F32LaneExpected`]. A
    /// `v128.const f32x4` with NO NaN-class lanes never reaches this
    /// variant; it still parses to the plain, pre-existing
    /// `Expected::Value(ConstValue::V128(_))` byte-exact path, unchanged.
    V128F32x4([F32LaneExpected; 4]),
    /// Same as [`Self::V128F32x4`], but for the 2-lane `f64x2` shape --
    /// see [`F64LaneExpected`].
    V128F64x2([F64LaneExpected; 2]),
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
    /// Bare `(ref.i31)` with no argument (W20) — matches ANY `i31ref`,
    /// same "can't predict the exact value, just the shape" wildcard as
    /// `RefFuncAny` above. Used by the real `i31.wast`'s own `(assert_return
    /// (invoke "new" (i32.const 1)) (ref.i31))`.
    RefI31Any,
    /// Bare `(ref.array)` (GC proposal, real corpus vendoring pass --
    /// `array.wast`/`array_new_data.wast`/`array_new_elem.wast`'s own
    /// `(assert_return (invoke "new") (ref.array))`) -- same wildcard shape
    /// as `RefFuncAny`/`RefI31Any` above: the test can't predict the exact
    /// array object identity `array.new*` allocates, only that it's SOME
    /// non-null array reference. Graded the same conservative way
    /// `RefFuncAny` already is (`WasmValue::Ref(Some(_))`) -- this crate's
    /// value representation carries no per-kind runtime type tag to
    /// distinguish "some struct ref" from "some array ref" anyway (see
    /// `wasm-conformance::value_matches_expected`'s own doc comment on
    /// `RefFuncAny`), so accepting any non-null ref handle here is exact
    /// for this crate's purposes, not an approximation.
    RefArrayAny,
    /// Bare `(ref.struct)` (GC proposal, real corpus vendoring pass --
    /// `struct.wast`'s own `(assert_return (invoke "new") (ref.struct))`)
    /// -- same wildcard shape and same grading as `RefArrayAny` above.
    RefStructAny,
    /// Bare `(ref.eq)` (GC proposal, real corpus vendoring pass --
    /// `array.wast`'s own `(assert_return (invoke "new") (ref.eq))`) --
    /// `eqref` is the GC type hierarchy's common supertype of `i31ref`,
    /// every struct ref, and every array ref (but NOT `funcref`/
    /// `externref`), so this wildcard is graded slightly more broadly than
    /// `RefArrayAny`/`RefStructAny`/`RefFuncAny`: it accepts either a
    /// non-null ref handle OR an `i31ref` (this crate's own
    /// `WasmValue::I32`, see `RefI31Any`'s own doc comment) -- everything
    /// `eqref` can hold, this crate's value representation can express.
    RefEqAny,
    /// `(either A B)` (relaxed SIMD epic PR1 — see `code/specs/
    /// W19-wasm-relaxed-simd-first-slice.md`) — the actual result must
    /// match `A` **or** `B`, not necessarily either specific one. The
    /// upstream corpus uses this to grade relaxed-simd ops, which the
    /// spec deliberately leaves implementation-defined for certain input
    /// patterns (e.g. `i8x16.relaxed_swizzle`'s out-of-range-index
    /// behavior can be "clamp to zero" OR "wrap modulo the lane count",
    /// both conforming). `A`/`B` are themselves full `Expected` values
    /// (boxed, recursively — the WAST grammar itself allows nesting),
    /// not just `ConstValue`s — this is a NEW top-level assert_return
    /// combinator, not a new `ConstValue`/lane shape like `V128F32x4`/
    /// `V128F64x2` above. Grading lives in `wasm-conformance`'s
    /// `value_matches_expected`, which just tries `A` then `B` — see that
    /// function's own `Either` arm. Discovered by reading the real
    /// upstream `i8x16_relaxed_swizzle.wast`/`i16x8_relaxed_q15mulr_s.
    /// wast` corpus content directly, not assumed from the opcode list —
    /// every relaxed-simd `.wast` file at this repo's pinned
    /// `WebAssembly/testsuite` commit uses `either` at least once, so
    /// this is a genuine prerequisite for vendoring ANY relaxed-simd
    /// fixture, not an optional nicety.
    Either(Box<Expected>, Box<Expected>),
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
    /// `Err(String)` (W14) when this module's own instruction stream fails
    /// to build -- e.g. an opcode this repo doesn't implement yet. Captured
    /// here rather than propagated as a whole-`parse_script` `Err`, so one
    /// unbuildable module doesn't prevent every OTHER directive in the same
    /// `.wast` file (independently parseable and gradeable) from being
    /// walked at all. Only a module's own *build* failure is captured this
    /// way -- a genuine tokenizer/S-expression *syntax* error still fails
    /// `parse_script` as a whole (see that function's own doc comment).
    ///
    /// `id`: the module's own `$name` -- `(module $Mf ...)` -- if given
    /// (real WASM: task #93/linking.wast). Previously discarded entirely
    /// during parsing ("doesn't affect encoding"), which meant a script's
    /// executor had no way to resolve a LATER `(invoke $Mf "f" ...)` or
    /// `(register "M" $Mf)` back to this specific module -- both actions
    /// already carry a `$name` reference (`Action::Invoke.module`,
    /// `Directive::Register.module_name`), they just had nothing to
    /// resolve it against.
    /// `Box`ed (clippy `large_enum_variant`): `WasmModule` is a large flat
    /// struct (many `Vec` fields), and every OTHER `Directive` variant is
    /// much smaller -- without boxing, a `Vec<Directive>` for a whole
    /// script (hundreds of directives in a real corpus file) would pad
    /// EVERY entry to this one variant's size.
    Module { id: Option<String>, result: Box<Result<WasmModule, String>> },
    /// `(module definition $M <fields...>)` (real corpus vendoring pass,
    /// `instance.wast`'s "Instantiation is generative" tests): a module
    /// DECLARED under `$M` but deliberately NOT instantiated here -- unlike
    /// a plain `(module $id ...)` (built AND instantiated eagerly by
    /// [`Directive::Module`]), a "definition" only becomes a live instance
    /// when a LATER `(module instance ...)` directive names it, and can be
    /// instantiated more than once (each instantiation getting its own
    /// independent global/table/memory state) -- exactly what a single
    /// eagerly-built [`WasmModule`] + [`Directive::Module`]'s "build once,
    /// instantiate once" shape can't express. Parsed the same "capture the
    /// build error, don't abort the whole script" way as `Directive::
    /// Module` -- see that variant's own doc comment. `id` is `None` for
    /// the rarer anonymous `(module definition <fields...>)` form (no
    /// `$name` at all -- the real corpus's own `memory.wast`/`table.wast`
    /// use this to validate a boundary-case module -- e.g. a memory at
    /// exactly the max page count -- WITHOUT actually instantiating/
    /// allocating it, since nothing ever needs to reference it again by
    /// name).
    ModuleDefinition { id: Option<String>, result: Box<Result<WasmModule, String>> },
    /// `(module instance $I $M)` (same real-corpus pass as `ModuleDefinition`
    /// above): instantiate the module declared by an earlier `(module
    /// definition $M ...)`, registering the FRESH instance under `$I` (or
    /// anonymously, for the rare unnamed `(module instance $M)` form --
    /// `id: None`). `definition_id` is `$M` itself, resolved against
    /// whatever instance/definition registry the executor keeps.
    ModuleInstance { id: Option<String>, definition_id: String },
    Register { name: String, module_name: Option<String> },
    Action(Action),
    AssertReturn { action: Action, expected: Vec<Expected> },
    AssertTrap { action: Action, message: String },
    /// `(assert_exception (invoke ...))` (W21 -- the exceptions proposal):
    /// the action must produce an UNCAUGHT WASM exception (`throw`,
    /// propagated without a matching `try_table` catch clause) -- a
    /// genuinely different outcome from `AssertTrap`'s ordinary trap (the
    /// real spec's own `try_table` never catches a trap, only an
    /// exception; see `wasm-execution`'s `TrapError::is_exception`). No
    /// message string, unlike `assert_trap`/`assert_exhaustion` -- the
    /// real corpus's own shape (`throw.wast`'s `(assert_exception (invoke
    /// "throw-if" (i32.const 10)))`) never carries one.
    AssertException { action: Action },
    AssertExhaustion { action: Action, message: String },
    AssertInvalid { module: ModuleSource, message: String },
    AssertMalformed { module: ModuleSource, message: String },
    AssertUnlinkable { module: ModuleSource, message: String },
}

/// Parses `src` into a full `Vec<Directive>`. Only fails as a whole for a
/// genuine tokenizer/S-expression *syntax* error (`parse_source`'s own
/// `?`) -- a script with a directive boundary that can't be identified at
/// all. A well-formed `(module ...)` directive whose own instruction
/// stream fails to BUILD (W14 -- e.g. names an opcode this repo doesn't
/// implement yet) does NOT abort this function; it's captured as
/// `Directive::Module(Err(_))` and every other directive in the file is
/// still parsed and returned normally. See `Directive::Module`'s own doc
/// comment for the full rationale.
pub fn parse_script(src: &str) -> Result<Vec<Directive>, WastParseError> {
    let exprs = parse_source(src)?;
    // "Inline module" shorthand: a whole `.wast` SCRIPT (not just a `module
    // quote` body, which `module.rs::parse_module` already handles via its
    // own "abbreviated module" support -- see that function's doc comment)
    // consisting entirely of bare module FIELDS, with no enclosing
    // `(module ...)` and no directives at all. The real corpus's own
    // `inline-module.wast` is exactly this: `(func) (memory 0) (func
    // (export "f"))` at the top level, nothing else. Recognized by
    // checking every top-level item's head keyword is a module-field
    // keyword (never a script-directive keyword like `module`/`register`/
    // `assert_*`/`invoke`/`get`) -- if EVERY item qualifies, synthesize one
    // `(module <fields...>)` wrapping all of them and parse that as a
    // single `Directive::Module`, exactly as if the file had written the
    // wrapper explicitly. A script that's already directive-shaped (every
    // currently-vendored file) is completely unaffected: `all()` on an
    // empty iterator is vacuously true, so this only fires when the script
    // has at least one item and none of them look like a real directive.
    if !exprs.is_empty() && exprs.iter().all(is_bare_module_field) {
        let synthetic_module = SExpr::List(
            std::iter::once(SExpr::Atom("module".to_string(), 0)).chain(exprs).collect(),
            0,
        );
        return Ok(vec![Directive::Module {
            id: None,
            result: Box::new(build_module_directive(&synthetic_module).map_err(|e| e.to_string())),
        }]);
    }
    exprs.iter().map(parse_directive).collect()
}

/// Does this top-level item look like a `.wat` module FIELD (`type`,
/// `import`, `func`, `table`, `memory`, `global`, `export`, `start`,
/// `elem`, `data`, `tag`) rather than a script-directive keyword? Used
/// only to detect the "inline module" shorthand above -- real directive
/// keywords (`module`, `register`, `invoke`, `get`, every `assert_*`)
/// never overlap with this set, so there's no ambiguity either way.
fn is_bare_module_field(e: &SExpr) -> bool {
    matches!(
        e.as_list().and_then(|items| items.first()).and_then(|i| i.as_atom()),
        Some(
            "type" | "import" | "func" | "table" | "memory" | "global" | "export" | "start"
                | "elem" | "data" | "tag"
        )
    )
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
        // `(module definition $M ...)` / `(module instance $I $M)` (real
        // corpus vendoring pass, `instance.wast`) are recognized BEFORE the
        // ordinary `(module ...)`/`(module $name ...)` fallback below --
        // without this, `items[1]` being the bare atom `definition`/
        // `instance` (never `$`-prefixed) would fall through to
        // `extract_module_id` (returns `None`, since neither word starts
        // with `$`) and `build_module_directive`, which would then try to
        // parse `definition`/`instance` and the following `$name`(s) as
        // ordinary MODULE FIELDS. Every module-field loop in `module.rs`
        // only ever matches `SExpr::List` keyword forms (`f.is_keyword_
        // list("global")` etc.) and silently ignores any other item, so a
        // bare atom is never an error -- it's quietly skipped. That would
        // make `(module definition $M (global ...) ...)` build as an
        // ANONYMOUS module containing $M's own fields (losing the `$M`
        // identity entirely), and `(module instance $I1 $M)` (three bare
        // atoms, zero field lists) build as a trivially EMPTY anonymous
        // module -- both silently "succeed" while doing nothing like what
        // the script actually asked for, and any later `(register "I1"
        // $I1)` referencing the never-registered `$I1` name would then hit
        // a genuine, hard-to-diagnose `Fail` instead of an honest
        // `NotYetSupported`. Recognizing the two forms explicitly here
        // avoids that silent-garbage trap.
        "module" if items.get(1).and_then(|i| i.as_atom()) == Some("definition") => {
            // The `$name` is OPTIONAL here -- unlike `module instance`
            // below, where a nameless instance could never be
            // instantiated-from later so there'd be no point writing one,
            // an anonymous `(module definition <fields...>)` is exactly
            // how the real corpus's own `memory.wast`/`table.wast` spell
            // "validate this boundary-case module (e.g. a memory at
            // exactly the max page count) but don't actually instantiate/
            // allocate it" -- see `Directive::ModuleDefinition`'s own doc
            // comment.
            let (name, fields_start) = match items.get(2) {
                Some(SExpr::Atom(s, _)) if s.starts_with('$') => (Some(s.clone()), 3),
                _ => (None, 2),
            };
            // Re-synthesize as an ordinary (optionally named) `(module
            // [$name] <fields...>)` list and reuse the normal
            // module-building path unchanged -- a "definition" is a real
            // module in every way except WHEN it gets instantiated (see
            // `ModuleInstance` above), so its own fields (globals/tables/
            // funcs/etc.) parse exactly like any other module's.
            let synthetic = SExpr::List(
                std::iter::once(SExpr::Atom("module".to_string(), e.pos()))
                    .chain(name.clone().map(|n| SExpr::Atom(n, e.pos())))
                    .chain(items[fields_start..].iter().cloned())
                    .collect(),
                e.pos(),
            );
            Ok(Directive::ModuleDefinition {
                id: name,
                result: Box::new(build_module_directive(&synthetic).map_err(|e| e.to_string())),
            })
        }
        "module" if items.get(1).and_then(|i| i.as_atom()) == Some("instance") => {
            let names: Vec<&str> = items[2..]
                .iter()
                .map(|i| {
                    i.as_atom().filter(|s| s.starts_with('$')).ok_or(WastParseError::UnexpectedToken {
                        pos: e.pos(),
                        found: "".into(),
                        expected: "a $name after 'module instance'",
                    })
                })
                .collect::<Result<_, _>>()?;
            let (id, definition_id) = match names.as_slice() {
                // `(module instance $I $M)` -- named instance $I of
                // definition $M (the only shape the real corpus's own
                // `instance.wast` uses).
                [instance_name, definition_name] => (Some(instance_name.to_string()), definition_name.to_string()),
                // `(module instance $M)` -- an anonymous instance (no `$I`
                // to register it under later); not used by any currently
                // vendored fixture, supported here for completeness since
                // it's the same grammar rule with one optional name.
                [definition_name] => (None, definition_name.to_string()),
                _ => {
                    return Err(WastParseError::UnexpectedToken {
                        pos: e.pos(),
                        found: "".into(),
                        expected: "'(module instance $definition)' or '(module instance $instance $definition)'",
                    })
                }
            };
            Ok(Directive::ModuleInstance { id, definition_id })
        }
        "module" => Ok(Directive::Module {
            id: extract_module_id(e),
            result: Box::new(build_module_directive(e).map_err(|e| e.to_string())),
        }),
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
        // W21 (exceptions proposal): `(assert_exception (invoke ...))` --
        // no message string, unlike every OTHER `assert_*` directive with
        // an action (see `Directive::AssertException`'s own doc comment).
        "assert_exception" => {
            let action = parse_action(expect_get(items, 1)?)?;
            Ok(Directive::AssertException { action })
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
            // `any` (real corpus vendoring pass -- `ref_null.wast`'s own
            // `(ref.null any)`): same collapsed `Ref(None)` representation
            // as `func`/`extern` -- see `wasm-wast-parser::module::
            // parse_ref_null_heap_type`'s matching "any" arm for why this
            // is a real, wired heap type (`ValueType::Anyref`'s `0x6E`
            // encoding), not a fallback guess.
            // `exn` (real corpus vendoring pass -- `ref_null.wast`'s own
            // `(ref.null exn)`): same collapsed `Ref(None)` representation,
            // see `wasm-wast-parser::module::parse_ref_null_heap_type`'s
            // matching "exn" arm. `none`/`nofunc`/`noextern`/`noexn` (same
            // file's own bottom-type cases) are every bit as collapsed --
            // see that function's own doc comment on why an internally-
            // consistent, non-spec-canonical byte is exact here.
            "func" | "extern" | "any" | "exn" | "none" | "nofunc" | "noextern" | "noexn" => Ok(ConstValue::Ref(None)),
            other => Err(WastParseError::UnexpectedToken {
                pos,
                found: other.to_string(),
                expected: "func, extern, any, exn, none, nofunc, noextern, or noexn",
            }),
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

/// Does this `v128.const` lane token's text denote a NaN *class*
/// (`nan:canonical`/`nan:arithmetic`), not an exact literal? A leading
/// `-` sign is stripped first -- WAST allows `-nan:canonical` (the class
/// check itself is sign-agnostic either way, see
/// `wasm-conformance::value_matches_expected`'s existing scalar
/// `NanCanonicalF32`/etc. arms), so `-nan:canonical`/`-nan:arithmetic`
/// count as NaN-class tokens too, same as the unsigned spelling.
fn lane_is_nan_class(text: &str) -> bool {
    let stripped = text.strip_prefix('-').unwrap_or(text);
    stripped == "nan:canonical" || stripped == "nan:arithmetic"
}

/// Parse one `f32x4` lane token to a [`F32LaneExpected`] -- an exact
/// literal (reusing the same `parse_f32_bits` every other f32 literal in
/// this crate goes through) unless it's a NaN-class token (SIMD widen
/// PR28), which [`lane_is_nan_class`] already confirmed the caller only
/// reaches for.
fn parse_f32_lane_expected(text: &str, pos: usize) -> Result<F32LaneExpected, WastParseError> {
    match text.strip_prefix('-').unwrap_or(text) {
        "nan:canonical" => Ok(F32LaneExpected::NanCanonical),
        "nan:arithmetic" => Ok(F32LaneExpected::NanArithmetic),
        _ => Ok(F32LaneExpected::Exact(parse_f32_bits(text, pos)?)),
    }
}

/// The `f64x2` counterpart of [`parse_f32_lane_expected`].
fn parse_f64_lane_expected(text: &str, pos: usize) -> Result<F64LaneExpected, WastParseError> {
    match text.strip_prefix('-').unwrap_or(text) {
        "nan:canonical" => Ok(F64LaneExpected::NanCanonical),
        "nan:arithmetic" => Ok(F64LaneExpected::NanArithmetic),
        _ => Ok(F64LaneExpected::Exact(parse_f64_bits(text, pos)?)),
    }
}

/// Parse a `v128.const f32x4`/`f64x2` expected value once
/// [`parse_expected`]'s own match guard has already confirmed at least one
/// lane is a NaN-class token -- `shape` is exactly `"f32x4"` or
/// `"f64x2"`, guaranteed by that same guard.
fn parse_v128_expected_lanes(shape: &str, lanes: &[SExpr], pos: usize) -> Result<Expected, WastParseError> {
    match shape {
        "f32x4" => {
            if lanes.len() < 4 {
                return Err(WastParseError::UnexpectedEof);
            }
            let mut out = [F32LaneExpected::Exact(0); 4];
            for (slot, lane) in out.iter_mut().zip(&lanes[..4]) {
                let text = lane.as_atom().ok_or(WastParseError::UnexpectedToken { pos, found: "".into(), expected: "a numeric literal" })?;
                *slot = parse_f32_lane_expected(text, lane.pos())?;
            }
            Ok(Expected::V128F32x4(out))
        }
        "f64x2" => {
            if lanes.len() < 2 {
                return Err(WastParseError::UnexpectedEof);
            }
            let mut out = [F64LaneExpected::Exact(0); 2];
            for (slot, lane) in out.iter_mut().zip(&lanes[..2]) {
                let text = lane.as_atom().ok_or(WastParseError::UnexpectedToken { pos, found: "".into(), expected: "a numeric literal" })?;
                *slot = parse_f64_lane_expected(text, lane.pos())?;
            }
            Ok(Expected::V128F64x2(out))
        }
        _ => unreachable!("shape already validated as f32x4/f64x2 by parse_expected's own match guard"),
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
        // `(v128.const f32x4/f64x2 lane0 ...)` where at least one lane is a
        // NaN-class token (SIMD widen PR28) -- see [`Expected::V128F32x4`]/
        // [`Expected::V128F64x2`]'s own doc comments for why this needs a
        // dedicated per-lane representation instead of the plain
        // byte-exact `ConstValue::V128` path every other v128.const
        // (including f32x4/f64x2 ones with ONLY exact lanes) still uses.
        ("v128.const", Some(shape @ ("f32x4" | "f64x2")))
            if items.get(2..).unwrap_or(&[]).iter().any(|l| l.as_atom().is_some_and(lane_is_nan_class)) =>
        {
            parse_v128_expected_lanes(shape, items.get(2..).unwrap_or(&[]), e.pos())
        }
        // Bare `(ref.null)` / `(ref.func)` (WASM17) -- wildcard expectations
        // only meaningful as an `assert_return` result, see [`Expected`]'s
        // own doc comments. `lit` is `None` here precisely because these
        // forms carry no second element at all.
        ("ref.null", None) => Ok(Expected::RefNullAny),
        ("ref.func", None) => Ok(Expected::RefFuncAny),
        // Bare `(ref.i31)` (W20) -- same wildcard shape as `ref.func`
        // above, see [`Expected::RefI31Any`]'s own doc comment.
        ("ref.i31", None) => Ok(Expected::RefI31Any),
        // Bare `(ref.array)`/`(ref.struct)`/`(ref.eq)` (GC proposal) -- same
        // wildcard shape as `ref.func`/`ref.i31` above, see each variant's
        // own doc comment ([`Expected::RefArrayAny`]/[`Expected::
        // RefStructAny`]/[`Expected::RefEqAny`]).
        ("ref.array", None) => Ok(Expected::RefArrayAny),
        ("ref.struct", None) => Ok(Expected::RefStructAny),
        ("ref.eq", None) => Ok(Expected::RefEqAny),
        // `(either A B)` (relaxed SIMD epic PR1 -- see [`Expected::Either`]'s
        // own doc comment). `lit` is always `None` here in practice --
        // both children are LISTS (e.g. `(v128.const i8x16 ...)`), never a
        // bare atom, so `as_atom()` on `items[1]` never matches -- but the
        // guard doesn't need to check `lit` itself, `kind` alone
        // disambiguates. Recurses through the same `parse_expected` this
        // match arm lives in, so an `either` arm can in principle be any
        // other `Expected` shape (a NaN class, a nested `either`, etc.),
        // not just a plain `v128.const`/`ConstValue`.
        // `(either A B ...)` -- relaxed SIMD epic PR3 generalizes this arm
        // from exactly 2 children to N (>= 2 required): the real
        // `relaxed_min_max.wast` corpus (see `code/specs/
        // W19-wasm-relaxed-simd-first-slice.md`) is the first
        // relaxed-simd file whose `either` groups carry FOUR
        // alternatives, not the two `i8x16_relaxed_swizzle.wast`/
        // `i16x8_relaxed_q15mulr_s.wast` each used -- the original
        // `items[1]`/`items[2]`-only version would have silently DROPPED
        // alternatives 3 and 4 rather than erroring, a real correctness
        // bug (a test whose actual result matches only the 3rd/4th
        // alternative would wrongly fail to grade as passing).
        // `Expected::Either` itself stays binary (its own doc comment
        // already anticipated "a nested `either`") -- N children fold
        // into a right-leaning chain of nested `Either`s here, so
        // `value_matches_expected`'s existing `||`-based grading in
        // `wasm-conformance` needs no changes at all to support this.
        ("either", _) => {
            let mut alternatives = items[1..].iter().map(parse_expected);
            let first = alternatives.next().ok_or(WastParseError::UnexpectedEof)??;
            alternatives.try_fold(first, |acc, next| Ok(Expected::Either(Box::new(acc), Box::new(next?))))
        }
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

/// A `(module $name ...)` directive's own `$name`, if given -- the same
/// leading-`$`-atom-right-after-the-keyword position `parse_module_source`
/// below skips over, extracted here (task #93/linking.wast) so
/// `Directive::Module` can carry it for the executor to resolve later
/// `(invoke $name ...)`/`(register "..." $name)` references against.
fn extract_module_id(e: &SExpr) -> Option<String> {
    let items = e.as_list()?;
    match items.get(1) {
        Some(SExpr::Atom(s, _)) if s.starts_with('$') => Some(s.clone()),
        _ => None,
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
        SExpr::Str(b, pos) => String::from_utf8(b.clone()).map_err(|_| WastParseError::InvalidUtf8 { pos: *pos }),
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
        assert!(matches!(dirs[0], Directive::Module { id: None, .. }));
    }

    /// "Inline module" shorthand -- the real corpus's own `inline-
    /// module.wast` is a whole `.wast` FILE with no `(module ...)`
    /// wrapper at all, just bare fields at the top level.
    #[test]
    fn a_whole_script_of_bare_module_fields_is_one_implicit_module() {
        let dirs = parse_script(r#"(func) (memory 0) (func (export "f"))"#).unwrap();
        assert_eq!(dirs.len(), 1);
        match &dirs[0] {
            Directive::Module { id: None, result } => {
                let module = result.as_ref().as_ref().expect("inline module must build");
                assert_eq!(module.functions.len(), 2);
                assert_eq!(module.memories.len(), 1);
                assert_eq!(module.exports.len(), 1);
                assert_eq!(module.exports[0].name, "f");
            }
            other => panic!("expected an implicit Directive::Module, got {other:?}"),
        }
    }

    /// A script that's already directive-shaped (the overwhelming common
    /// case, every currently-vendored file) must be completely unaffected
    /// by the inline-module fallback -- it only fires when EVERY top-level
    /// item looks like a bare field, never when even one real directive is
    /// present.
    #[test]
    fn a_script_with_a_real_module_directive_is_not_treated_as_inline_fields() {
        let dirs = parse_script(r#"(module (func $f (export "f") (result i32) (i32.const 1))) (assert_return (invoke "f") (i32.const 1))"#).unwrap();
        assert_eq!(dirs.len(), 2);
        assert!(matches!(dirs[0], Directive::Module { .. }));
        assert!(matches!(dirs[1], Directive::AssertReturn { .. }));
    }

    #[test]
    fn an_empty_script_parses_to_zero_directives() {
        assert_eq!(parse_script("").unwrap(), vec![]);
        assert_eq!(parse_script("   ;; just a comment\n").unwrap(), vec![]);
    }

    // ══════════════════════════════════════════════════════════════════════
    // Annotations -- `(@id ...)` custom out-of-band tooling syntax, must be
    // silently ignored wherever an ordinary form is allowed. See
    // `sexpr::strip_annotations`'s own doc comment for the design; these
    // tests exercise it end-to-end through the real script/module parsing
    // entry points, mirroring shapes from the real corpus's own
    // `annotations.wast`.
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn a_bare_annotation_at_top_level_is_silently_skipped() {
        let dirs = parse_script(r#"(@a) (module (func))"#).unwrap();
        assert_eq!(dirs.len(), 1);
        assert!(matches!(dirs[0], Directive::Module { .. }));
    }

    #[test]
    fn annotations_interspersed_through_a_whole_module_do_not_break_parsing() {
        // Mirrors annotations.wast's own densely-annotated module shape --
        // an annotation between the `module` keyword and the name, between
        // every field, and inside `export`/`func`/`param`/`result`/
        // `block`/instruction positions.
        let src = r#"
            ((@a) module (@a) $m (@a)
              ((@a) import (@a) "spectest" (@a) "global_i32" (@a)
                ((@a) global (@a) $g (@a) i32 (@a)) (@a)
              ) (@a)
              ((@a) func (@a) $f (@a)
                ((@a) export (@a) "f" (@a)) (@a)
                ((@a) param (@a) i32 (@a)) (@a)
                ((@a) result (@a) i32 (@a)) (@a)
                ((@a) i32.add (@a)
                  ((@a) local.get (@a) 0 (@a)) (@a)
                  ((@a) i32.const (@a) 1 (@a))
                )
              ) (@a)
            )
        "#;
        let dirs = parse_script(src).unwrap();
        assert_eq!(dirs.len(), 1);
        match &dirs[0] {
            Directive::Module { result, .. } => {
                let module = result.as_ref().as_ref().expect("densely-annotated module must still build");
                assert_eq!(module.imports.len(), 1);
                assert_eq!(module.functions.len(), 1);
                assert_eq!(module.exports.len(), 1);
                assert_eq!(module.exports[0].name, "f");
            }
            other => panic!("expected Directive::Module, got {other:?}"),
        }
    }

    #[test]
    fn annotation_with_an_adjacent_quoted_id_is_accepted() {
        // `(@"a")` -- the id comes from an immediately-adjacent string, no
        // intervening whitespace -- one of annotations.wast's own valid
        // (non-`assert_malformed`) shapes.
        let dirs = parse_script(r#"(module (@"a") (func))"#).unwrap();
        assert_eq!(dirs.len(), 1);
        assert!(matches!(dirs[0], Directive::Module { .. }));
    }

    #[test]
    fn bare_at_sign_with_no_id_is_malformed() {
        assert!(crate::module::parse_module("(@)").is_err());
    }

    #[test]
    fn at_sign_followed_by_whitespace_then_an_id_is_malformed() {
        // The id must be IMMEDIATELY adjacent to `@` -- whitespace in
        // between means there's no id at all, per annotations.wast's own
        // `(@ x)` case.
        assert!(crate::module::parse_module("(@ x)").is_err());
    }

    #[test]
    fn empty_quoted_annotation_id_is_malformed() {
        assert!(crate::module::parse_module(r#"(@"")"#).is_err());
    }

    #[test]
    fn nested_annotation_forms_are_dropped_as_a_unit() {
        // The annotation's own internal grammar is irrelevant once a list
        // is recognized as an annotation -- everything inside, however
        // strange, is simply discarded whole.
        let dirs = parse_script(r#"(module (@a x-y $yz "aa" -2 0.3 0x3 (bla) () ("aa" a)) (func))"#).unwrap();
        assert_eq!(dirs.len(), 1);
        assert!(matches!(dirs[0], Directive::Module { .. }));
    }

    /// Task #93 (linking.wast): a module's own `(module $Mf ...)` name must
    /// be captured, not discarded -- it's how a LATER `(invoke $Mf "f" ...)`
    /// or `(register "M" $Mf)` resolves back to this specific module.
    #[test]
    fn module_directive_captures_its_own_name() {
        let dirs = parse_script("(module $Mf (func (result i32) (i32.const 42)))").unwrap();
        assert_eq!(dirs.len(), 1);
        assert!(matches!(&dirs[0], Directive::Module { id: Some(id), .. } if id == "$Mf"));
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
    fn parses_assert_exception_no_message_real_corpus_shape() {
        // W21 -- `throw.wast`'s own real shape: no message string, unlike
        // `assert_trap`/`assert_exhaustion`.
        let dirs = parse_script(r#"(assert_exception (invoke "throw-if" (i32.const 10)))"#).unwrap();
        match &dirs[0] {
            Directive::AssertException { action: Action::Invoke { name, .. } } => {
                assert_eq!(name, "throw-if");
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
            Directive::Module { result, .. } => match result.as_ref() {
                Ok(m) => {
                    assert_eq!(m.functions.len(), 1);
                    assert_eq!(m.exports[0].name, "f");
                }
                Err(e) => panic!("unexpected build failure: {e}"),
            },
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
            Directive::Module { result, .. } => {
                // Not quite `WasmModule::default()`: a binary module with
                // no data count section (§12) genuinely has none, so
                // `missing_data_count_section` is `true` here -- see that
                // field's own doc comment (W-addendum 2026-09-01 pass).
                // Harmless in this module specifically since it also has
                // no `memory.init`/`data.drop` to gate.
                let expected = wasm_types::WasmModule { missing_data_count_section: true, ..Default::default() };
                assert_eq!(result.as_ref(), &Ok(expected))
            }
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
        assert!(matches!(dirs[0], Directive::Module { .. }));
        assert!(matches!(&dirs[1], Directive::AssertReturn { action: Action::Invoke { module: Some(_), .. }, .. }));
    }

    #[test]
    fn a_module_using_an_unbuildable_instruction_does_not_abort_parsing_the_rest_of_the_script() {
        // W14: a module's own instruction stream failing to BUILD (an
        // unrecognized opcode name) must NOT abort `parse_script` for the
        // whole file -- the module directly before it, and everything
        // after the broken one, must still parse and come back as real
        // `Directive`s. Real motivating case: `simd_const.wast`'s sole
        // `i64x2.add` usage (an opcode this repo doesn't implement)
        // previously aborted grading its other ~445 directives.
        let dirs = parse_script(
            r#"(module $good1 (func (export "f") (result i32) (i32.const 1)))
               (module $bad (func (export "g") (result i32) (this.is.not.a.real.opcode)))
               (module $good2 (func (export "h") (result i32) (i32.const 2)))
               (assert_return (invoke $good1 "f") (i32.const 1))
               (assert_return (invoke $good2 "h") (i32.const 2))"#,
        )
        .unwrap();
        assert_eq!(dirs.len(), 5);
        assert!(matches!(&dirs[0], Directive::Module { result, .. } if result.is_ok()), "{:?}", dirs[0]);
        match &dirs[1] {
            Directive::Module { result, .. } => match result.as_ref() {
                Err(msg) => assert!(msg.contains("this.is.not.a.real.opcode"), "{msg}"),
                Ok(_) => panic!("expected a captured build error, got a built module"),
            },
            other => panic!("expected a captured build error, got {other:?}"),
        }
        assert!(matches!(&dirs[2], Directive::Module { result, .. } if result.is_ok()), "{:?}", dirs[2]);
        assert!(matches!(dirs[3], Directive::AssertReturn { .. }));
        assert!(matches!(dirs[4], Directive::AssertReturn { .. }));
    }

    #[test]
    fn a_genuine_syntax_error_still_aborts_the_whole_script() {
        // Unchanged, deliberate contrast with the test above: an
        // UNBALANCED-PAREN-level syntax error (not a semantic build
        // failure inside an otherwise well-formed module) still can't be
        // partially graded -- directive boundaries themselves aren't
        // reliably identifiable, so `parse_script` still returns a real
        // `Err` for the whole file, exactly as before this change.
        parse_script(r#"(module (func (result i32) i32.const 1)"#).unwrap_err();
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
    fn v128_const_f32x4_with_nan_class_lanes_as_assert_return_expected_value() {
        // SIMD widen PR28's own `simd_conversions.wast` shape (e.g.
        // `f32x4.demote_f64x2_zero`'s NaN-payload directives): a
        // `v128.const f32x4` expected value where SOME lanes are exact
        // literals and OTHERS are NaN-class tokens -- something the
        // plain byte-exact `ConstValue::V128` path can't represent at
        // all (see `Expected::V128F32x4`'s own doc comment). Mixes
        // `nan:canonical` and `nan:arithmetic` in the same literal to
        // prove both tokens parse, alongside two ordinary exact-zero
        // lanes to prove the exact/NaN-class lanes coexist correctly.
        let dirs = parse_script(r#"(assert_return (invoke "f") (v128.const f32x4 nan:canonical nan:arithmetic 0 0))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(
                    *expected,
                    vec![Expected::V128F32x4([
                        F32LaneExpected::NanCanonical,
                        F32LaneExpected::NanArithmetic,
                        F32LaneExpected::Exact(0),
                        F32LaneExpected::Exact(0),
                    ])]
                );
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn v128_const_f64x2_with_nan_class_lanes_as_assert_return_expected_value() {
        // The `f64x2` counterpart, matching `f64x2.promote_low_f32x4`'s
        // own NaN-payload directives in `simd_conversions.wast` -- both
        // lanes are NaN-class here (no exact lane mixed in), since that's
        // the real corpus's own shape for this particular op.
        let dirs = parse_script(r#"(assert_return (invoke "f") (v128.const f64x2 nan:canonical nan:arithmetic))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(*expected, vec![Expected::V128F64x2([F64LaneExpected::NanCanonical, F64LaneExpected::NanArithmetic])]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn v128_const_f32x4_with_no_nan_class_lanes_still_uses_the_plain_byte_exact_path() {
        // A `v128.const f32x4` expected value with ONLY exact lanes must
        // NOT be routed through the new `Expected::V128F32x4` machinery --
        // confirms `parse_expected`'s match guard genuinely gates on "at
        // least one NaN-class lane", not "shape is f32x4/f64x2", so every
        // pre-existing exact-value v128 test (e.g.
        // `v128_const_literal_as_assert_return_expected_value` above)
        // keeps working unchanged.
        let dirs = parse_script(r#"(assert_return (invoke "f") (v128.const f32x4 1.5 2.5 3.5 4.5))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert!(matches!(expected[0], Expected::Value(ConstValue::V128(_))), "expected the plain byte-exact V128 path, got {:?}", expected[0]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    // ── Relaxed SIMD epic PR1: `(either A B)` -- see code/specs/
    // W19-wasm-relaxed-simd-first-slice.md ───────────────────────────────

    #[test]
    fn either_of_two_v128_const_values_parses_as_expected_either() {
        // The real upstream shape (`i8x16_relaxed_swizzle.wast`):
        // `(assert_return (invoke ...) (either (v128.const i8x16 ...)
        // (v128.const i8x16 ...)))`.
        let dirs = parse_script(
            r#"(assert_return (invoke "f") (either (v128.const i32x4 0 0 0 0) (v128.const i32x4 1 1 1 1)))"#,
        )
        .unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                let zeros = [0u8; 16];
                let mut ones = [0u8; 16];
                for lane in 0..4 {
                    ones[lane * 4..lane * 4 + 4].copy_from_slice(&1i32.to_le_bytes());
                }
                assert_eq!(
                    expected[0],
                    Expected::Either(
                        Box::new(Expected::Value(ConstValue::V128(zeros))),
                        Box::new(Expected::Value(ConstValue::V128(ones))),
                    )
                );
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn either_recurses_through_parse_expected_for_non_v128_children() {
        // `either`'s children are full `Expected` values, not just
        // `ConstValue` -- confirms the recursion by nesting a NaN-class
        // expectation (a shape that ISN'T a plain `ConstValue`) as one arm.
        let dirs = parse_script(r#"(assert_return (invoke "f") (either (f32.const nan:canonical) (f32.const 0)))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(
                    expected[0],
                    Expected::Either(Box::new(Expected::NanCanonicalF32), Box::new(Expected::Value(ConstValue::F32Bits(0))))
                );
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn either_with_four_alternatives_folds_into_nested_binary_either() {
        // Relaxed SIMD epic PR3: the real `relaxed_min_max.wast` corpus
        // is the first relaxed-simd file whose `either` groups carry FOUR
        // alternatives, not the two `i8x16_relaxed_swizzle.wast`/
        // `i16x8_relaxed_q15mulr_s.wast` each used. Confirms
        // `parse_expected`'s generalized `either` arm doesn't silently
        // drop alternatives 3 and 4 the way the original items[1]/
        // items[2]-only version would have -- it folds all N children
        // into a right-leaning chain of nested `Expected::Either`s.
        let dirs = parse_script(
            r#"(assert_return (invoke "f")
                   (either (v128.const i32x4 0 0 0 0)
                           (v128.const i32x4 1 1 1 1)
                           (v128.const i32x4 2 2 2 2)
                           (v128.const i32x4 3 3 3 3)))"#,
        )
        .unwrap();
        let v128_of = |n: i32| {
            let mut bytes = [0u8; 16];
            for lane in 0..4 {
                bytes[lane * 4..lane * 4 + 4].copy_from_slice(&n.to_le_bytes());
            }
            Expected::Value(ConstValue::V128(bytes))
        };
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(
                    expected[0],
                    Expected::Either(
                        Box::new(Expected::Either(Box::new(Expected::Either(Box::new(v128_of(0)), Box::new(v128_of(1)))), Box::new(v128_of(2)))),
                        Box::new(v128_of(3)),
                    )
                );
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn either_with_four_alternatives_on_scalar_i32_expected_values() {
        // Same generalized `either` arm, exercised on plain scalar
        // `i32.const` alternatives (not `v128.const`) inside a real
        // `(module ...)` + `assert_return` script, to confirm the N-ary
        // fold isn't somehow special-cased to `v128` shapes -- grading
        // itself (the recursive `||` in `wasm-conformance::
        // value_matches_expected`) is that crate's own test coverage,
        // not this crate's; this test only confirms the parse tree here.
        let dirs = parse_script(
            r#"(module (func (export "f") (param i32) (result i32) (local.get 0)))
               (assert_return (invoke "f" (i32.const 3)) (either (i32.const 0) (i32.const 1) (i32.const 2) (i32.const 3)))"#,
        )
        .unwrap();
        match &dirs[1] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(
                    expected[0],
                    Expected::Either(
                        Box::new(Expected::Either(
                            Box::new(Expected::Either(Box::new(Expected::Value(ConstValue::I32(0))), Box::new(Expected::Value(ConstValue::I32(1))))),
                            Box::new(Expected::Value(ConstValue::I32(2))),
                        )),
                        Box::new(Expected::Value(ConstValue::I32(3))),
                    )
                );
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
    fn bare_ref_i31_is_a_wildcard_expectation() {
        // W20 -- `i31.wast`'s own `(assert_return (invoke "new" (i32.const
        // 1)) (ref.i31))` real shape.
        let dirs = parse_script(r#"(assert_return (invoke "f") (ref.i31))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(*expected, vec![Expected::RefI31Any]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn bare_ref_array_struct_eq_are_wildcard_expectations() {
        // GC proposal -- `array.wast`'s own `(assert_return (invoke "new")
        // (ref.array))`/`(ref.eq)`, `struct.wast`'s own `(ref.struct)`.
        let dirs = parse_script(r#"(assert_return (invoke "f") (ref.array) (ref.struct) (ref.eq))"#).unwrap();
        match &dirs[0] {
            Directive::AssertReturn { expected, .. } => {
                assert_eq!(*expected, vec![Expected::RefArrayAny, Expected::RefStructAny, Expected::RefEqAny]);
            }
            other => panic!("expected AssertReturn, got {other:?}"),
        }
    }

    #[test]
    fn module_definition_with_a_name_builds_but_is_not_a_plain_module() {
        let dirs = parse_script(r#"(module definition $M (memory 1))"#).unwrap();
        match &dirs[0] {
            Directive::ModuleDefinition { id, result } => {
                assert_eq!(id.as_deref(), Some("$M"));
                assert!(result.is_ok(), "expected a buildable module, got {result:?}");
            }
            other => panic!("expected ModuleDefinition, got {other:?}"),
        }
    }

    #[test]
    fn module_definition_without_a_name_is_anonymous() {
        // `memory.wast`/`table.wast`'s own real shape: a boundary-case
        // module validated but deliberately never instantiated.
        let dirs = parse_script(r#"(module definition (memory 65536))"#).unwrap();
        match &dirs[0] {
            Directive::ModuleDefinition { id, result } => {
                assert_eq!(*id, None);
                assert!(result.is_ok(), "expected a buildable module, got {result:?}");
            }
            other => panic!("expected ModuleDefinition, got {other:?}"),
        }
    }

    #[test]
    fn module_instance_with_both_names_resolves_instance_and_definition_ids() {
        let dirs = parse_script(r#"(module instance $I1 $M)"#).unwrap();
        match &dirs[0] {
            Directive::ModuleInstance { id, definition_id } => {
                assert_eq!(id.as_deref(), Some("$I1"));
                assert_eq!(definition_id, "$M");
            }
            other => panic!("expected ModuleInstance, got {other:?}"),
        }
    }

    #[test]
    fn module_instance_with_only_a_definition_name_is_anonymous() {
        let dirs = parse_script(r#"(module instance $M)"#).unwrap();
        match &dirs[0] {
            Directive::ModuleInstance { id, definition_id } => {
                assert_eq!(*id, None);
                assert_eq!(definition_id, "$M");
            }
            other => panic!("expected ModuleInstance, got {other:?}"),
        }
    }

    #[test]
    fn module_definition_and_instance_do_not_abort_the_rest_of_the_script() {
        // Regression test for the exact bug this feature fixes: before
        // `module definition`/`module instance` were recognized as their
        // own directive shapes, `build_module_directive` silently treated
        // the bare `definition`/`instance`/`$name` atoms as harmless
        // unrecognized module fields, quietly building a WRONG (anonymous
        // or empty) module instead of erroring -- see the real
        // `instance.wast` fixture this defends against.
        let dirs = parse_script(
            r#"
            (module definition $M (memory (export "mem") 1))
            (module instance $I $M)
            (register "I" $I)
            "#,
        )
        .unwrap();
        assert_eq!(dirs.len(), 3);
        assert!(matches!(dirs[0], Directive::ModuleDefinition { .. }));
        assert!(matches!(dirs[1], Directive::ModuleInstance { .. }));
        assert!(matches!(
            dirs[2],
            Directive::Register { ref name, ref module_name }
                if name == "I" && module_name.as_deref() == Some("$I")
        ));
    }

    #[test]
    fn ref_extern_non_integer_literal_errors_cleanly_not_panics() {
        let err = parse_script(r#"(assert_return (invoke "f") (ref.extern nope))"#).unwrap_err();
        assert!(matches!(err, WastParseError::UnexpectedToken { .. }));
    }

    /// `expect_str` (used for `register`'s name, `invoke`/`get` action
    /// names, and assert-message strings) had the same
    /// `String::from_utf8_lossy` defect as the module-name-string call
    /// sites in `module.rs` -- same fix (`String::from_utf8`), same
    /// `WastParseError::InvalidUtf8` error, different call site. `\80` is
    /// a bare UTF-8 continuation byte with no lead byte, never valid alone.
    #[test]
    fn register_name_rejects_invalid_utf8_instead_of_replacing_it() {
        let err = parse_script(r#"(register "\80" $M)"#).unwrap_err();
        assert!(matches!(err, WastParseError::InvalidUtf8 { .. }), "expected InvalidUtf8, got {err:?}");
    }
}

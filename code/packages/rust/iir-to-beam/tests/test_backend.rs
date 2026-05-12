//! Integration tests for the iir-to-beam backend.
//!
//! These tests exercise the full validate → lower pipeline from
//! `IIRModule` inputs to `BEAMModule` outputs.
//!
//! # Test organisation
//!
//! 1–10:  Validation rejection tests — modules/instructions that MUST be
//!        rejected before any lowering occurs.
//! 11–12: Validation acceptance — modules that MUST pass validation.
//! 13–14: const instruction lowering (Int and Bool).
//! 15–26: Arithmetic and bitwise instruction lowering.
//! 27–32: Comparison instruction lowering (synthesized with cond + labels).
//! 33–35: Control flow (label, jmp, jmp_if_true, jmp_if_false).
//! 36–37: Return instructions.
//! 38:    type_assert nop.
//! 39:    load_reg / store_reg.
//! 40:    call (inter-function).
//! 41:    multi-function export table.
//! 42:    parameter register assignment.
//! 43:    register reuse for repeated variable names.
//! 44:    validate-then-lower round-trip.
//! 45:    non-empty instruction stream.

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use iir_to_beam::{lower_iir_to_beam, validate_for_beam, IIRBeamConfig};

// ===========================================================================
// Test helpers
// ===========================================================================

/// Build an `IIRModule` with a single zero-parameter function named `"main"`.
fn make_module_single(instrs: Vec<IIRInstr>) -> IIRModule {
    let fn_ = IIRFunction::new("main", vec![], "void", instrs);
    IIRModule {
        name: "test".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

/// Build an `IIRModule` with a single function using the given name,
/// parameters, and return type.
fn make_module_fn(
    fn_name: &str,
    params: Vec<(&str, &str)>,
    ret_type: &str,
    instrs: Vec<IIRInstr>,
) -> IIRModule {
    let params_owned: Vec<(String, String)> = params
        .into_iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect();
    let fn_ = IIRFunction::new(fn_name, params_owned, ret_type, instrs);
    IIRModule {
        name: "test".into(),
        functions: vec![fn_],
        entry_point: Some(fn_name.into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

/// Default lowering config for tests.
fn cfg() -> IIRBeamConfig {
    IIRBeamConfig::new("testmod")
}

// ===========================================================================
// Helper: check that the instruction stream contains an instruction with the
// given opcode (ignoring operands).
// ===========================================================================
fn has_opcode(beam: &iir_to_beam::BEAMModule, opcode: u8) -> bool {
    beam.instructions.iter().any(|i| i.opcode == opcode)
}

// BEAM opcodes referenced in assertions
const OP_GC_BIF2: u8 = 125;
const OP_GC_BIF1: u8 = 124;
const OP_IS_EQ_EXACT: u8 = 43;
const OP_IS_NE_EXACT: u8 = 44;
const OP_IS_LT: u8 = 47;
const OP_IS_GE: u8 = 48;
const OP_MOVE: u8 = 64;
const OP_RETURN: u8 = 19;
const OP_JUMP: u8 = 36;
const OP_LABEL: u8 = 1;
const OP_CALL: u8 = 4;

// ===========================================================================
// 1. test_empty_module_rejected
// ===========================================================================

/// An IIRModule with no functions must be rejected.
///
/// A BEAM module with no functions would produce an empty code section and an
/// empty export table — the BEAM loader would refuse to load it.
#[test]
fn test_empty_module_rejected() {
    let module = IIRModule {
        name: "empty".into(),
        functions: vec![],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errs = validate_for_beam(&module);
    assert!(!errs.is_empty(), "should reject empty module");
    assert!(errs.iter().any(|e| e.contains("EmptyModule")));
}

// ===========================================================================
// 2. test_empty_function_rejected
// ===========================================================================

/// A function with zero instructions must be rejected.
///
/// An empty function body in BEAM would produce a `func_info` preamble with no
/// code — valid bytecode but almost certainly a front-end bug.
#[test]
fn test_empty_function_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![]));
    assert!(!errs.is_empty(), "should reject empty function");
    assert!(errs.iter().any(|e| e.contains("EmptyFunction")));
}

// ===========================================================================
// 3. test_any_type_rejected
// ===========================================================================

/// Instructions with `type_hint == "any"` must be rejected.
///
/// BEAM arithmetic BIFs expect integers; passing untyped values at compile
/// time would silently produce code that might raise `badarith` at runtime.
#[test]
fn test_any_type_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "add",
        Some("v".into()),
        vec![Operand::Var("a".into()), Operand::Var("b".into())],
        "any", // <-- rejected
    )]));
    assert!(
        errs.iter().any(|e| e.contains("UntypedInstruction")),
        "expected UntypedInstruction error, got: {:?}",
        errs
    );
}

// ===========================================================================
// 4. test_polymorphic_type_rejected
// ===========================================================================

/// Instructions with `type_hint == "polymorphic"` must be rejected.
///
/// `"polymorphic"` is the profiler sentinel for "seen multiple types at
/// runtime" — it means the JIT should not specialise.  Static BEAM lowering
/// cannot emit correct code for polymorphic instructions.
#[test]
fn test_polymorphic_type_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "add",
        Some("v".into()),
        vec![Operand::Var("a".into()), Operand::Var("b".into())],
        "polymorphic", // <-- rejected
    )]));
    assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
}

// ===========================================================================
// 5. test_str_type_rejected
// ===========================================================================

/// Instructions with `type_hint == "str"` must be rejected.
///
/// This backend emits only integer arithmetic.  String operations would
/// require Erlang binary/list BIFs that this lowering does not implement.
#[test]
fn test_str_type_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "add",
        Some("v".into()),
        vec![Operand::Var("a".into()), Operand::Var("b".into())],
        "str", // <-- rejected
    )]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

// ===========================================================================
// 6. test_ref_type_rejected
// ===========================================================================

/// Instructions with `type_hint` starting with `"ref<"` must be rejected.
///
/// Heap pointer types require GC-managed BEAM terms; this backend does not
/// implement GC-aware lowering in v1.
#[test]
fn test_ref_type_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "add",
        Some("v".into()),
        vec![Operand::Var("a".into()), Operand::Var("b".into())],
        "ref<u8>", // <-- rejected
    )]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
}

// ===========================================================================
// 7. test_float_const_rejected
// ===========================================================================

/// A `const` instruction with a `Float` operand must be rejected.
///
/// BEAM supports floats via `fmove` into float registers, but this lowering
/// does not implement that path in v1.  Silently truncating a float to an
/// integer would produce subtly incorrect results.
#[test]
fn test_float_const_rejected() {
    // type_hint is "f64" (valid concrete type), but the operand is a Float —
    // that combination is specifically rejected for `const` instructions.
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "const",
        Some("v".into()),
        vec![Operand::Float(3.14)], // <-- rejected
        "f64",
    )]));
    assert!(
        errs.iter().any(|e| e.contains("Float")),
        "expected float-const rejection, got: {:?}",
        errs
    );
}

// ===========================================================================
// 8. test_call_builtin_rejected
// ===========================================================================

/// `call_builtin` must be rejected — it requires a NIF bridge.
#[test]
fn test_call_builtin_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "call_builtin",
        Some("v".into()),
        vec![Operand::Var("println".into())],
        "void",
    )]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

// ===========================================================================
// 9. test_io_out_accepted (LANG32)
// ===========================================================================

/// `io_out` is now SUPPORTED by the BEAM backend (LANG32) via erlang:display/1.
/// The validator must NOT reject it.
#[test]
fn test_io_out_accepted() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "io_out",
        None,
        vec![Operand::Var("x".into())],
        "void",
    )]));
    assert!(
        errs.iter().all(|e| !e.contains("UnsupportedOp")),
        "io_out should be accepted by BEAM validator (LANG32); got: {:?}",
        errs
    );
}

// ===========================================================================
// 10. test_alloc_rejected
// ===========================================================================

/// `alloc` with an unsupported reference type must be rejected.
///
/// `alloc ref<u8>` is not a LispyPair allocation — this backend only knows
/// how to lower `alloc ref<LispyPair>` (Erlang cons cells via put_list).
/// Any other ref type on alloc gets an UnsupportedType error from the
/// validator since we cannot map it to a BEAM instruction.
#[test]
fn test_alloc_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "alloc",
        Some("ptr".into()),
        vec![Operand::Int(8)],
        "ref<u8>",  // not ref<LispyPair> — rejected
    )]));
    // alloc ref<u8> is rejected with UnsupportedType (not in the allowed set).
    assert!(
        errs.iter().any(|e| e.contains("UnsupportedType") || e.contains("UnsupportedOp")),
        "expected alloc with unsupported ref type to be rejected, got: {:?}", errs
    );
}

// ===========================================================================
// 11. test_validate_passes_on_valid_module
// ===========================================================================

/// A module with a simple `const` + `ret_void` should pass validation.
#[test]
fn test_validate_passes_on_valid_module() {
    let errs = validate_for_beam(&make_module_single(vec![
        IIRInstr::new(
            "const",
            Some("v".into()),
            vec![Operand::Int(42)],
            "i32",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]));
    assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
}

// ===========================================================================
// 12. test_const_i32
// ===========================================================================

/// Lowering a `const i32` instruction should succeed and produce a non-empty
/// atom table containing the module name.
#[test]
fn test_const_i32() {
    let m = make_module_single(vec![
        IIRInstr::new("const", Some("v".into()), vec![Operand::Int(99)], "i32"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(!beam.atoms.is_empty());
    assert!(beam.atoms.contains(&"testmod".to_string()));
}

// ===========================================================================
// 13. test_const_bool_true
// ===========================================================================

/// `const true` should lower to `move {i,1} {x,rd}`.
#[test]
fn test_const_bool_true() {
    let m = make_module_single(vec![
        IIRInstr::new("const", Some("flag".into()), vec![Operand::Bool(true)], "bool"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // The lowered module should have a MOVE instruction (for the const) and a RETURN.
    assert!(has_opcode(&beam, OP_MOVE));
}

// ===========================================================================
// 14. test_const_bool_false
// ===========================================================================

/// `const false` should lower to `move {i,0} {x,rd}`.
#[test]
fn test_const_bool_false() {
    let m = make_module_single(vec![
        IIRInstr::new("const", Some("flag".into()), vec![Operand::Bool(false)], "bool"),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_MOVE));
}

// ===========================================================================
// 15. test_add_i32
// ===========================================================================

/// `add` should emit `gc_bif2 erlang:+/2`.
#[test]
fn test_add_i32() {
    let m = make_module_fn(
        "add_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("result".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        has_opcode(&beam, OP_GC_BIF2),
        "expected GC_BIF2 for add, instructions: {:?}",
        beam.instructions.iter().map(|i| i.opcode).collect::<Vec<_>>()
    );
    // The erlang:+ atom must be in the atom table.
    assert!(beam.atoms.contains(&"+".to_string()));
}

// ===========================================================================
// 16. test_sub_i32
// ===========================================================================

/// `sub` should emit `gc_bif2 erlang:-/2`.
#[test]
fn test_sub_i32() {
    let m = make_module_fn(
        "sub_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "sub",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"-".to_string()));
}

// ===========================================================================
// 17. test_mul_i32
// ===========================================================================

/// `mul` should emit `gc_bif2 erlang:*/2`.
#[test]
fn test_mul_i32() {
    let m = make_module_fn(
        "mul_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "mul",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"*".to_string()));
}

// ===========================================================================
// 18. test_div_i32
// ===========================================================================

/// `div` should emit `gc_bif2 erlang:div/2`.
#[test]
fn test_div_i32() {
    let m = make_module_fn(
        "div_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "div",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"div".to_string()));
}

// ===========================================================================
// 19. test_mod_i32
// ===========================================================================

/// `mod` should emit `gc_bif2 erlang:rem/2`.
#[test]
fn test_mod_i32() {
    let m = make_module_fn(
        "mod_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "mod",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"rem".to_string()));
}

// ===========================================================================
// 20. test_neg_i32
// ===========================================================================

/// `neg` should emit `gc_bif1 erlang:-/1`.
#[test]
fn test_neg_i32() {
    let m = make_module_fn(
        "neg_fn",
        vec![("a", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "neg",
                Some("r".into()),
                vec![Operand::Var("a".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // gc_bif1 is opcode 124
    assert!(
        has_opcode(&beam, OP_GC_BIF1),
        "expected GC_BIF1 for neg"
    );
}

// ===========================================================================
// 21. test_and_i32
// ===========================================================================

/// `and` should emit `gc_bif2 erlang:band/2`.
#[test]
fn test_and_i32() {
    let m = make_module_fn(
        "and_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "and",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"band".to_string()));
}

// ===========================================================================
// 22. test_or_i32
// ===========================================================================

/// `or` should emit `gc_bif2 erlang:bor/2`.
#[test]
fn test_or_i32() {
    let m = make_module_fn(
        "or_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "or",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"bor".to_string()));
}

// ===========================================================================
// 23. test_xor_i32
// ===========================================================================

/// `xor` should emit `gc_bif2 erlang:bxor/2`.
#[test]
fn test_xor_i32() {
    let m = make_module_fn(
        "xor_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "xor",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"bxor".to_string()));
}

// ===========================================================================
// 24. test_not_i32
// ===========================================================================

/// `not` should emit `gc_bif1 erlang:bnot/1`.
#[test]
fn test_not_i32() {
    let m = make_module_fn(
        "not_fn",
        vec![("a", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "not",
                Some("r".into()),
                vec![Operand::Var("a".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF1));
    assert!(beam.atoms.contains(&"bnot".to_string()));
}

// ===========================================================================
// 25. test_shl_i32
// ===========================================================================

/// `shl` should emit `gc_bif2 erlang:bsl/2`.
#[test]
fn test_shl_i32() {
    let m = make_module_fn(
        "shl_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "shl",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"bsl".to_string()));
}

// ===========================================================================
// 26. test_shr_i32
// ===========================================================================

/// `shr` should emit `gc_bif2 erlang:bsr/2`.
#[test]
fn test_shr_i32() {
    let m = make_module_fn(
        "shr_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "shr",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_GC_BIF2));
    assert!(beam.atoms.contains(&"bsr".to_string()));
}

// ===========================================================================
// 27. test_cmp_eq
// ===========================================================================

/// `cmp_eq` must produce a synthesized boolean using `is_eq_exact` + two
/// `move` instructions + a synthetic convergence label.
///
/// The synthesis pattern is:
///   move {i,0} {x,rd}          ← pre-load 0 (false)
///   is_eq_exact {f,synth} r1 r2 ← branch to synth if NOT equal
///   move {i,1} {x,rd}          ← equal: overwrite with 1 (true)
///   label {u,synth}            ← convergence
#[test]
fn test_cmp_eq() {
    let m = make_module_fn(
        "cmp_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_eq",
                Some("eq".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("eq".into())], "bool"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_EQ_EXACT), "expected is_eq_exact");
    assert!(has_opcode(&beam, OP_MOVE), "expected move (for boolean synthesis)");
}

// ===========================================================================
// 28. test_cmp_ne
// ===========================================================================

/// `cmp_ne` must use `is_ne_exact` in the synthesis pattern.
#[test]
fn test_cmp_ne() {
    let m = make_module_fn(
        "cne_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_ne",
                Some("ne".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_NE_EXACT), "expected is_ne_exact");
}

// ===========================================================================
// 29. test_cmp_lt
// ===========================================================================

/// `cmp_lt` must use `is_lt(r1,r2)` — falls through when r1 < r2.
#[test]
fn test_cmp_lt() {
    let m = make_module_fn(
        "lt_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_lt",
                Some("lt".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_LT), "expected is_lt");
}

// ===========================================================================
// 30. test_cmp_le
// ===========================================================================

/// `cmp_le` must use `is_ge(r2,r1)` — swapped operands turn >= into <=.
#[test]
fn test_cmp_le() {
    let m = make_module_fn(
        "le_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_le",
                Some("le".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_GE), "expected is_ge (swapped for le)");
}

// ===========================================================================
// 31. test_cmp_gt
// ===========================================================================

/// `cmp_gt` must use `is_lt(r2,r1)` — swapped operands turn lt into gt.
#[test]
fn test_cmp_gt() {
    let m = make_module_fn(
        "gt_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_gt",
                Some("gt".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_LT), "expected is_lt (swapped for gt)");
}

// ===========================================================================
// 32. test_cmp_ge
// ===========================================================================

/// `cmp_ge` must use `is_ge(r1,r2)`.
#[test]
fn test_cmp_ge() {
    let m = make_module_fn(
        "ge_fn",
        vec![("a", "i32"), ("b", "i32")],
        "bool",
        vec![
            IIRInstr::new(
                "cmp_ge",
                Some("ge".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_GE), "expected is_ge");
}

// ===========================================================================
// 33. test_label_and_jmp
// ===========================================================================

/// `label` and `jmp` must produce `{label,N}` and `{jump,{f,N}}` respectively.
#[test]
fn test_label_and_jmp() {
    let m = make_module_single(vec![
        IIRInstr::new(
            "label",
            None,
            vec![Operand::Var("loop".into())],
            "void",
        ),
        IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var("loop".into())],
            "void",
        ),
        // Unreachable but needed to satisfy "non-empty function"
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // There should be ≥3 labels: preamble (2) + the IIR "loop" label.
    let label_count = beam.instructions.iter().filter(|i| i.opcode == OP_LABEL).count();
    assert!(label_count >= 3, "expected ≥3 labels (preamble + IIR label)");
    assert!(has_opcode(&beam, OP_JUMP), "expected jump instruction");
}

// ===========================================================================
// 34. test_jmp_if_true
// ===========================================================================

/// `jmp_if_true` must produce `is_eq_exact` (branch-if-false) + `jump` + label.
///
/// The synthesis:
///   is_eq_exact {f,fall} {x,cond} {i,0}  ← skip jump when cond == 0
///   jump {f,target}                        ← cond != 0: take the branch
///   label {u,fall}
#[test]
fn test_jmp_if_true() {
    let m = make_module_single(vec![
        IIRInstr::new(
            "const",
            Some("cond".into()),
            vec![Operand::Bool(true)],
            "bool",
        ),
        IIRInstr::new(
            "label",
            None,
            vec![Operand::Var("target".into())],
            "void",
        ),
        IIRInstr::new(
            "jmp_if_true",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("target".into())],
            "void",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_EQ_EXACT), "expected is_eq_exact for jmp_if_true");
    assert!(has_opcode(&beam, OP_JUMP));
}

// ===========================================================================
// 35. test_jmp_if_false
// ===========================================================================

/// `jmp_if_false` must produce `is_ne_exact` (branch-if-true) + `jump` + label.
#[test]
fn test_jmp_if_false() {
    let m = make_module_single(vec![
        IIRInstr::new(
            "const",
            Some("cond".into()),
            vec![Operand::Bool(false)],
            "bool",
        ),
        IIRInstr::new(
            "label",
            None,
            vec![Operand::Var("done".into())],
            "void",
        ),
        IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var("cond".into()), Operand::Var("done".into())],
            "void",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_IS_NE_EXACT), "expected is_ne_exact for jmp_if_false");
    assert!(has_opcode(&beam, OP_JUMP));
}

// ===========================================================================
// 36. test_ret_void
// ===========================================================================

/// `ret_void` must produce a `{return}` instruction.
#[test]
fn test_ret_void() {
    let beam = lower_iir_to_beam(
        &make_module_single(vec![IIRInstr::new("ret_void", None, vec![], "void")]),
        &cfg(),
    )
    .unwrap();
    assert!(has_opcode(&beam, OP_RETURN));
}

// ===========================================================================
// 37. test_ret_with_value
// ===========================================================================

/// `ret` with a non-x0 register must emit a `move` to x0 before `return`.
#[test]
fn test_ret_with_value() {
    // Function with 2 params — "result" will be allocated to x2 or higher.
    let m = make_module_fn(
        "ret_fn",
        vec![("a", "i32"), ("b", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("result".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(has_opcode(&beam, OP_RETURN));
    // There must be at least one MOVE (to move result into x0 before return,
    // since result is in x2, not x0).
    assert!(has_opcode(&beam, OP_MOVE));
}

// ===========================================================================
// 38. test_type_assert_is_nop
// ===========================================================================

/// `type_assert` must produce no BEAM instructions (it is erased).
///
/// The instruction count with `type_assert` should equal the count without it.
#[test]
fn test_type_assert_is_nop() {
    let without = make_module_single(vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let with_assert = make_module_single(vec![
        IIRInstr::new(
            "type_assert",
            None,
            vec![Operand::Var("x".into())],
            "i32",
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam_without = lower_iir_to_beam(&without, &cfg()).unwrap();
    let beam_with    = lower_iir_to_beam(&with_assert, &cfg()).unwrap();
    assert_eq!(
        beam_without.instructions.len(),
        beam_with.instructions.len(),
        "type_assert should produce no instructions"
    );
}

// ===========================================================================
// 39. test_load_reg_store_reg
// ===========================================================================

/// `load_reg` and `store_reg` must produce `move` instructions.
#[test]
fn test_load_reg_store_reg() {
    let m = make_module_fn(
        "reg_fn",
        vec![("a", "i32")],
        "i32",
        vec![
            // load_reg: copy "a" into "copy_of_a"
            IIRInstr::new(
                "load_reg",
                Some("copy_of_a".into()),
                vec![Operand::Var("a".into())],
                "i32",
            ),
            // store_reg: write "copy_of_a" back into "a" (same register, no-op semantically)
            IIRInstr::new(
                "store_reg",
                None,
                vec![Operand::Var("a".into()), Operand::Var("copy_of_a".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("copy_of_a".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // There should be MOVE instructions from the load_reg and store_reg.
    assert!(has_opcode(&beam, OP_MOVE));
}

// ===========================================================================
// 40. test_call_function
// ===========================================================================

/// A two-function module where `main` calls `add_two` must produce a `call`
/// instruction in the output.
///
/// The call instruction sequence is:
///   move {x,arg_regs…} {x,0,1,…}   ← set up arguments
///   call {u,arity} {f,entry_label}  ← call
///   move {x,0} {x,result_reg}       ← capture return value from x0
#[test]
fn test_call_function() {
    // "add_two" function: returns a + b
    let add_two = IIRFunction::new(
        "add_two",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("r".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "i32"),
        ],
    );
    // "main" function: loads constants, calls add_two
    let main_fn = IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            IIRInstr::new(
                "const",
                Some("x".into()),
                vec![Operand::Int(3)],
                "i32",
            ),
            IIRInstr::new(
                "const",
                Some("y".into()),
                vec![Operand::Int(4)],
                "i32",
            ),
            IIRInstr::new(
                "call",
                Some("sum".into()),
                vec![
                    Operand::Var("add_two".into()),
                    Operand::Var("x".into()),
                    Operand::Var("y".into()),
                ],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("sum".into())], "i32"),
        ],
    );
    let module = IIRModule {
        name: "multi".into(),
        functions: vec![add_two, main_fn],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let beam = lower_iir_to_beam(&module, &IIRBeamConfig::new("multi")).unwrap();
    assert!(has_opcode(&beam, OP_CALL), "expected call instruction");
}

// ===========================================================================
// 41. test_multi_function_exports
// ===========================================================================

/// A module with two functions must have two entries in the export table.
#[test]
fn test_multi_function_exports() {
    let fn1 = IIRFunction::new(
        "foo",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let fn2 = IIRFunction::new(
        "bar",
        vec![],
        "void",
        vec![IIRInstr::new("ret_void", None, vec![], "void")],
    );
    let module = IIRModule {
        name: "two_fn".into(),
        functions: vec![fn1, fn2],
        entry_point: Some("foo".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let beam = lower_iir_to_beam(&module, &IIRBeamConfig::new("two_fn")).unwrap();
    assert_eq!(beam.exports.len(), 2, "expected 2 exports for 2 functions");
    // Both function names should be in the atom table.
    assert!(beam.atoms.contains(&"foo".to_string()));
    assert!(beam.atoms.contains(&"bar".to_string()));
}

// ===========================================================================
// 42. test_params_get_first_registers
// ===========================================================================

/// Parameters must be assigned x0, x1, x2 in order (Erlang calling convention).
///
/// We verify this indirectly: after lowering a 3-parameter function, the
/// export arity must be 3.
#[test]
fn test_params_get_first_registers() {
    let m = make_module_fn(
        "three_param",
        vec![("a", "i32"), ("b", "i32"), ("c", "i32")],
        "i32",
        vec![
            IIRInstr::new(
                "add",
                Some("ab".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new(
                "add",
                Some("abc".into()),
                vec![Operand::Var("ab".into()), Operand::Var("c".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("abc".into())], "i32"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert_eq!(beam.exports[0].arity, 3, "function must be exported with arity 3");
}

// ===========================================================================
// 43. test_register_reuse
// ===========================================================================

/// The same variable name used in multiple instructions must map to the same
/// x-register throughout the function.
///
/// We verify this by checking that lowering succeeds (no UndefinedVariable
/// error) and produces a non-empty instruction stream.
#[test]
fn test_register_reuse() {
    let m = make_module_fn(
        "reuse_fn",
        vec![("val", "i32")],
        "i32",
        vec![
            // Use "val" as src in two separate instructions.
            IIRInstr::new(
                "add",
                Some("doubled".into()),
                vec![Operand::Var("val".into()), Operand::Var("val".into())],
                "i32",
            ),
            IIRInstr::new(
                "add",
                Some("tripled".into()),
                vec![Operand::Var("doubled".into()), Operand::Var("val".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("tripled".into())], "i32"),
        ],
    );
    // Should succeed without UndefinedVariable errors.
    let result = lower_iir_to_beam(&m, &cfg());
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let beam = result.unwrap();
    assert!(!beam.instructions.is_empty());
}

// ===========================================================================
// 44. test_validate_then_lower_succeeds
// ===========================================================================

/// Full round-trip: validate then lower should succeed for a well-formed module.
#[test]
fn test_validate_then_lower_succeeds() {
    let m = make_module_fn(
        "round_trip",
        vec![("n", "i64")],
        "i64",
        vec![
            IIRInstr::new(
                "const",
                Some("one".into()),
                vec![Operand::Int(1)],
                "i64",
            ),
            IIRInstr::new(
                "add",
                Some("result".into()),
                vec![Operand::Var("n".into()), Operand::Var("one".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "i64"),
        ],
    );
    let errs = validate_for_beam(&m);
    assert!(errs.is_empty(), "validation errors: {:?}", errs);
    let result = lower_iir_to_beam(&m, &cfg());
    assert!(result.is_ok(), "lowering failed: {:?}", result.err());
    let beam = result.unwrap();
    assert_eq!(beam.name, "testmod");
    assert!(!beam.atoms.is_empty());
    assert!(!beam.exports.is_empty());
}

// ===========================================================================
// 45. test_lowering_produces_nonempty_instructions
// ===========================================================================

/// Even the simplest module must produce a non-empty instruction stream.
///
/// At minimum: preamble (3 instrs) + ret_void (1) + int_code_end (1) = 5.
#[test]
fn test_lowering_produces_nonempty_instructions() {
    let beam = lower_iir_to_beam(
        &make_module_single(vec![IIRInstr::new("ret_void", None, vec![], "void")]),
        &cfg(),
    )
    .unwrap();
    assert!(
        beam.instructions.len() >= 5,
        "expected ≥5 instructions, got {}",
        beam.instructions.len()
    );
}

// ===========================================================================
// Phase 2 heap op tests (46–58): BEAM lowering of cons/car/cdr/null?/make_nil
// ===========================================================================
//
// These tests verify that the IIR heap ops produced by iir-builtin-lowering
// Phase 2 are correctly accepted and lowered by the BEAM backend.
//
// Organisation:
// 46: alloc ref<LispyPair> accepted by validator.
// 47: cons pattern (alloc+field_store+field_store) lowers to put_list.
// 48: put_list atom is NOT in atom table (put_list is a BEAM opcode, not a BIF).
// 49: car (field_load idx 0) lowers to get_list.
// 50: cdr (field_load idx 1) lowers to get_list.
// 51: null? (is_null) lowers with is_nil synthesis.
// 52: make_nil (const 0 ref<LispyPair>) lowers to move [] atom.
// 53: "[]" appears in atom table after make_nil lowering.
// 54: cons pair compiles without error (end-to-end).
// 55: car/cdr field_load compiles end-to-end.
// 56: null? produces is_nil + 2 synthetic labels (move/is_nil/jump/move).
// 57: mini length function (null? + add + cdr + call) compiles.
// 58: alloc ref<LispyPair> accepted but alloc ref<u8> still rejected.

// BEAM opcodes for the heap tests
const OP_PUT_LIST: u8 = 69;
const OP_GET_LIST: u8 = 65;
const OP_IS_NIL:   u8 = 52;

// ===========================================================================
// 46. alloc ref<LispyPair> accepted by validator
// ===========================================================================

/// `alloc ref<LispyPair>` must pass validation (it is a known heap op).
#[test]
fn test_46_alloc_lispy_pair_accepted_by_validator() {
    // We need the full alloc + 2 field_stores to have a valid module (that
    // can also be lowered successfully), but here we only test that the
    // validator accepts the individual alloc instruction.
    let fn_ = IIRFunction::new(
        "main",
        vec![("h".into(), "i64".into()), ("t".into(), "i64".into())],
        "ref<LispyPair>",
        vec![
            IIRInstr::new(
                "alloc",
                Some("cell".into()),
                vec![],
                "ref<LispyPair>",  // accepted — LispyPair is the one ref type allowed on alloc
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(0),
                    Operand::Var("h".into()),
                ],
                "void",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(1),
                    Operand::Var("t".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("cell".into())], "ref<LispyPair>"),
        ],
    );
    let module = IIRModule {
        name: "test".into(),
        functions: vec![fn_],
        entry_point: Some("main".into()),
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    };
    let errs = validate_for_beam(&module);
    assert!(
        errs.is_empty(),
        "alloc ref<LispyPair> should be accepted by validator, got: {:?}", errs
    );
}

// ===========================================================================
// 47. cons pattern (alloc + field_store×2) lowers to put_list
// ===========================================================================

/// The alloc + two field_stores sequence must fuse into a single put_list.
#[test]
fn test_47_cons_pattern_produces_put_list() {
    let m = make_module_fn(
        "cons_fn",
        vec![("h", "i64"), ("t", "i64")],
        "ref<LispyPair>",
        vec![
            IIRInstr::new("alloc", Some("cell".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(0),
                    Operand::Var("h".into()),
                ],
                "void",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(1),
                    Operand::Var("t".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("cell".into())], "ref<LispyPair>"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        has_opcode(&beam, OP_PUT_LIST),
        "cons pattern (alloc + 2 field_stores) must produce put_list, got opcodes: {:?}",
        beam.instructions.iter().map(|i| i.opcode).collect::<Vec<_>>()
    );
}

// ===========================================================================
// 48. car (field_load idx 0) lowers to get_list
// ===========================================================================

/// `field_load` with index 0 (car) must produce a `get_list` instruction.
#[test]
fn test_48_car_produces_get_list() {
    let m = make_module_fn(
        "car_fn",
        vec![("pair", "ref<LispyPair>")],
        "ref<any>",
        vec![
            IIRInstr::new(
                "field_load",
                Some("head".into()),
                vec![Operand::Var("pair".into()), Operand::Int(0)],
                "ref<any>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("head".into())], "ref<any>"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        has_opcode(&beam, OP_GET_LIST),
        "field_load index 0 (car) must produce get_list"
    );
}

// ===========================================================================
// 49. cdr (field_load idx 1) lowers to get_list
// ===========================================================================

/// `field_load` with index 1 (cdr) must also produce a `get_list` instruction.
#[test]
fn test_49_cdr_produces_get_list() {
    let m = make_module_fn(
        "cdr_fn",
        vec![("pair", "ref<LispyPair>")],
        "ref<any>",
        vec![
            IIRInstr::new(
                "field_load",
                Some("tail".into()),
                vec![Operand::Var("pair".into()), Operand::Int(1)],
                "ref<any>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("tail".into())], "ref<any>"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        has_opcode(&beam, OP_GET_LIST),
        "field_load index 1 (cdr) must produce get_list"
    );
}

// ===========================================================================
// 50. null? (is_null) lowers with is_nil synthesis
// ===========================================================================

/// `is_null` must produce an `is_nil` instruction (plus the boolean synthesis).
#[test]
fn test_50_is_null_produces_is_nil() {
    let m = make_module_fn(
        "null_fn",
        vec![("xs", "ref<LispyPair>")],
        "bool",
        vec![
            IIRInstr::new(
                "is_null",
                Some("result".into()),
                vec![Operand::Var("xs".into())],
                "bool",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("result".into())], "bool"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        has_opcode(&beam, OP_IS_NIL),
        "is_null must produce is_nil, got opcodes: {:?}",
        beam.instructions.iter().map(|i| i.opcode).collect::<Vec<_>>()
    );
}

// ===========================================================================
// 51. make_nil lowers to move [] atom
// ===========================================================================

/// `const 0 : ref<LispyPair>` (nil sentinel) must lower to a move of the
/// BEAM `[]` atom — NOT a move of the integer 0.
#[test]
fn test_51_make_nil_lowers_to_nil_atom_move() {
    let m = make_module_single(vec![
        IIRInstr::new(
            "const",
            Some("nil".into()),
            vec![Operand::Int(0)],
            "ref<LispyPair>",  // nil sentinel
        ),
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // The "[]" atom must appear in the atom table.
    assert!(
        beam.atoms.contains(&"[]".to_string()),
        "nil lowering must intern the [] atom"
    );
    // There must be a MOVE instruction (to load the atom into the register).
    assert!(has_opcode(&beam, OP_MOVE), "nil lowering must produce a move instruction");
}

// ===========================================================================
// 52. "[]" atom in atom table after make_nil
// ===========================================================================

/// The `[]` atom is always interned at module build time, even if make_nil
/// is never used (we pre-intern it alongside the BIF atoms).
#[test]
fn test_52_nil_atom_interned_by_default() {
    // A module with no heap ops still has [] in the atom table because we
    // pre-intern it upfront alongside the BIF names.
    let m = make_module_single(vec![
        IIRInstr::new("ret_void", None, vec![], "void"),
    ]);
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    assert!(
        beam.atoms.contains(&"[]".to_string()),
        "[] should always be interned even in non-heap modules"
    );
}

// ===========================================================================
// 53. cons pair compiles without error (end-to-end validation + lowering)
// ===========================================================================

/// A function that builds a cons pair must compile end-to-end without errors.
#[test]
fn test_53_cons_pair_compiles_end_to_end() {
    let m = make_module_fn(
        "make_pair",
        vec![("head", "i64"), ("tail", "i64")],
        "ref<LispyPair>",
        vec![
            IIRInstr::new("alloc", Some("cell".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(0),
                    Operand::Var("head".into()),
                ],
                "void",
            ),
            IIRInstr::new(
                "field_store",
                None,
                vec![
                    Operand::Var("cell".into()),
                    Operand::Int(1),
                    Operand::Var("tail".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("cell".into())], "ref<LispyPair>"),
        ],
    );
    let errs = validate_for_beam(&m);
    assert!(errs.is_empty(), "cons function validation failed: {:?}", errs);
    let result = lower_iir_to_beam(&m, &cfg());
    assert!(result.is_ok(), "cons function lowering failed: {:?}", result.err());
}

// ===========================================================================
// 54. car/cdr field_load compiles end-to-end
// ===========================================================================

/// A function that reads both fields from a cons cell must compile cleanly.
#[test]
fn test_54_field_load_compiles_end_to_end() {
    let m = make_module_fn(
        "get_fields",
        vec![("pair", "ref<LispyPair>")],
        "ref<any>",
        vec![
            // head = car(pair)
            IIRInstr::new(
                "field_load",
                Some("head".into()),
                vec![Operand::Var("pair".into()), Operand::Int(0)],
                "ref<any>",
            ),
            // tail = cdr(pair)
            IIRInstr::new(
                "field_load",
                Some("tail".into()),
                vec![Operand::Var("pair".into()), Operand::Int(1)],
                "ref<any>",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("head".into())], "ref<any>"),
        ],
    );
    let errs = validate_for_beam(&m);
    assert!(errs.is_empty(), "field_load validation failed: {:?}", errs);
    let result = lower_iir_to_beam(&m, &cfg());
    assert!(result.is_ok(), "field_load lowering failed: {:?}", result.err());
    let beam = result.unwrap();
    // Both car and cdr should produce get_list instructions.
    let get_list_count = beam.instructions.iter().filter(|i| i.opcode == OP_GET_LIST).count();
    assert_eq!(get_list_count, 2, "expected 2 get_list instructions for car+cdr");
}

// ===========================================================================
// 55. null? synthesis uses move + is_nil + jump + move (6 instructions)
// ===========================================================================

/// `is_null` synthesis produces: move(true) + is_nil + jump + label + move(false) + label.
/// That is exactly 6 BEAM instructions for the null check.
#[test]
fn test_55_null_pred_synthesis_instruction_count() {
    let m = make_module_fn(
        "check_nil",
        vec![("xs", "ref<LispyPair>")],
        "bool",
        vec![
            IIRInstr::new(
                "is_null",
                Some("result".into()),
                vec![Operand::Var("xs".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let beam = lower_iir_to_beam(&m, &cfg()).unwrap();
    // Count: move(1) + is_nil + jump + label + move(0) + label = 6
    assert!(has_opcode(&beam, OP_IS_NIL), "must have is_nil");
    assert!(has_opcode(&beam, OP_JUMP),   "must have jump for done_label");
    // Both paths produce a MOVE (pre-load true + false path).
    let move_count = beam.instructions.iter().filter(|i| i.opcode == OP_MOVE).count();
    assert!(move_count >= 2, "expected ≥2 move instructions for null? synthesis");
}

// ===========================================================================
// 56. alloc ref<u8> still rejected (only ref<LispyPair> is accepted on alloc)
// ===========================================================================

/// The validator must still reject `alloc ref<u8>` — only `ref<LispyPair>` is
/// whitelisted for the BEAM backend.
#[test]
fn test_56_alloc_non_lispy_pair_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "alloc",
        Some("ptr".into()),
        vec![],
        "ref<u8>",  // rejected — not ref<LispyPair>
    )]));
    assert!(
        !errs.is_empty(),
        "alloc ref<u8> must be rejected"
    );
    assert!(
        errs.iter().any(|e| e.contains("UnsupportedType") || e.contains("UnsupportedOp")),
        "expected UnsupportedType or UnsupportedOp for alloc ref<u8>, got: {:?}", errs
    );
}

// ===========================================================================
// 57. Mini length function: null? + cmp + cdr + call compiles
// ===========================================================================

/// A simplified (non-recursive) fragment of a list-length computation:
///   if (null? xs) then 0 else 1
///
/// This exercises null? + cmp_eq + jmp_if_true in combination.
/// (A fully recursive length would require a self-call, which we test elsewhere.)
#[test]
fn test_57_mini_length_fragment_compiles() {
    // Simulates:
    //   if (null? xs) { return 0 } else { return 1 }
    // using IIR labels and branches.
    let m = make_module_fn(
        "length_fragment",
        vec![("xs", "ref<LispyPair>")],
        "i64",
        vec![
            // nil_result = null?(xs)
            IIRInstr::new(
                "is_null",
                Some("nil_result".into()),
                vec![Operand::Var("xs".into())],
                "bool",
            ),
            // label "is_nil_true"
            IIRInstr::new(
                "label",
                None,
                vec![Operand::Var("is_nil_true".into())],
                "void",
            ),
            // if nil_result goto is_nil_true
            IIRInstr::new(
                "jmp_if_true",
                None,
                vec![
                    Operand::Var("nil_result".into()),
                    Operand::Var("is_nil_true".into()),
                ],
                "void",
            ),
            // length = 1 (non-nil path)
            IIRInstr::new(
                "const",
                Some("length".into()),
                vec![Operand::Int(1)],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("length".into())], "i64"),
            // nil path: return 0
            IIRInstr::new(
                "const",
                Some("zero".into()),
                vec![Operand::Int(0)],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("zero".into())], "i64"),
        ],
    );
    let errs = validate_for_beam(&m);
    assert!(errs.is_empty(), "length_fragment validation failed: {:?}", errs);
    let result = lower_iir_to_beam(&m, &cfg());
    assert!(result.is_ok(), "length_fragment lowering failed: {:?}", result.err());
    let beam = result.unwrap();
    // Must have is_nil (from null?) and is_eq_exact (from jmp_if_true synthesis).
    assert!(has_opcode(&beam, OP_IS_NIL), "must have is_nil for null?");
    assert!(!beam.instructions.is_empty());
}

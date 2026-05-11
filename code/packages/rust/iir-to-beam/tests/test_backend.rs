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
// 9. test_io_out_rejected
// ===========================================================================

/// `io_out` must be rejected — raw I/O has no BEAM instruction equivalent.
#[test]
fn test_io_out_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "io_out",
        None,
        vec![Operand::Var("x".into())],
        "void",
    )]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
}

// ===========================================================================
// 10. test_alloc_rejected
// ===========================================================================

/// `alloc` must be rejected — GC heap allocation requires NIF support.
#[test]
fn test_alloc_rejected() {
    let errs = validate_for_beam(&make_module_single(vec![IIRInstr::new(
        "alloc",
        Some("ptr".into()),
        vec![Operand::Int(8)],
        "ref<u8>",
    )]));
    assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
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

use intel8086_backend::{compile, BackendError, Intel8086Backend};
use intel8086_simulator::Intel8086Simulator;
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};

fn ctx<'a>(name: &'a str, params: &'a [(String, String)], ret_ty: &'a str) -> FunctionContext<'a> {
    FunctionContext {
        name,
        params,
        return_type: ret_ty,
    }
}

fn ci(op: &str, dest: Option<&str>, srcs: Vec<CIROperand>, ty: &str) -> CIRInstr {
    CIRInstr::new(op, dest, srcs, ty)
}

fn word02_binary(op: &str, width: &str, left: i64, right: i64) -> Vec<CIRInstr> {
    vec![
        ci(
            &format!("const_{width}"),
            Some("left"),
            vec![CIROperand::Int(left)],
            width,
        ),
        ci(
            &format!("const_{width}"),
            Some("right"),
            vec![CIROperand::Int(right)],
            width,
        ),
        ci(
            op,
            Some("result"),
            vec![
                CIROperand::Var("left".into()),
                CIROperand::Var("right".into()),
            ],
            width,
        ),
        ci(
            &format!("ret_{width}"),
            None,
            vec![CIROperand::Var("result".into())],
            width,
        ),
    ]
}

#[test]
fn word02b_complement_executes_and_preserves_other_live_value() {
    for (width, value, expected) in [("u8", 0, 0xff), ("u16", 0x1234, 0xedcb)] {
        let cir = vec![
            ci(&format!("const_{width}"), Some("x"), vec![CIROperand::Int(value)], width),
            ci(&format!("not_{width}"), Some("y"), vec![CIROperand::Var("x".into())], width),
            ci(&format!("ret_{width}"), None, vec![CIROperand::Var("y".into())], width),
        ];
        let bytes = compile(&ctx("not", &[], width), &cir).unwrap();
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&bytes);
        assert!(sim.run_loaded_with_limit(100).halted);
        assert_eq!(sim.ax, expected);
    }
    let cir = vec![
        ci("const_u16", Some("left"), vec![CIROperand::Int(0x1234)], "u16"),
        ci("const_u16", Some("right"), vec![CIROperand::Int(2)], "u16"),
        ci("not_u16", Some("inverted"), vec![CIROperand::Var("right".into())], "u16"),
        ci("add_u16", Some("result"), vec![CIROperand::Var("left".into()), CIROperand::Var("inverted".into())], "u16"),
        ci("ret_u16", None, vec![CIROperand::Var("result".into())], "u16"),
    ];
    let bytes = compile(&ctx("preserve", &[], "u16"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(100).halted);
    assert_eq!(sim.ax, 0x1231);
}

#[test]
fn word02_executes_two_live_byte_and_word_operations() {
    for (op, width, left, right, expected) in [
        ("add_u8", "u8", 255, 2, 1),
        ("sub_u8", "u8", 0, 1, 255),
        ("and_u8", "u8", 0x55, 0x0f, 5),
        ("or_u8", "u8", 0x50, 0x0f, 0x5f),
        ("xor_u8", "u8", 0x55, 0x0f, 0x5a),
        ("add_u16", "u16", 65535, 2, 1),
        ("sub_u16", "u16", 0, 1, 65535),
        ("and_u16", "u16", 0x1234, 0x00ff, 0x0034),
        ("or_u16", "u16", 0x1234, 0x00ff, 0x12ff),
        ("xor_u16", "u16", 0x1234, 0x00ff, 0x12cb),
    ] {
        let cir = word02_binary(op, width, left, right);
        let bytes = compile(&ctx("word02", &[], width), &cir).unwrap();
        let mut sim = Intel8086Simulator::new(65536);
        sim.load_program(&bytes);
        assert!(sim.run_loaded_with_limit(100).halted, "{op}");
        assert_eq!(sim.ax, expected, "{op}");
    }
}

#[test]
fn word02_preserves_second_live_value_across_first_result() {
    let mut cir = word02_binary("add_u16", "u16", 5, 2);
    cir[3] = ci(
        "sub_u16",
        Some("difference"),
        vec![
            CIROperand::Var("result".into()),
            CIROperand::Var("right".into()),
        ],
        "u16",
    );
    cir.push(ci(
        "ret_u16",
        None,
        vec![CIROperand::Var("difference".into())],
        "u16",
    ));
    let bytes = compile(&ctx("reuse", &[], "u16"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(100).halted);
    assert_eq!(sim.ax, 5);
}

#[test]
fn word02_rejects_three_live_values_without_spilling() {
    let mut cir = word02_binary("add_u16", "u16", 5, 2);
    cir[3] = ci(
        "add_u16",
        Some("next"),
        vec![
            CIROperand::Var("left".into()),
            CIROperand::Var("result".into()),
        ],
        "u16",
    );
    cir.push(ci(
        "add_u16",
        Some("final"),
        vec![
            CIROperand::Var("next".into()),
            CIROperand::Var("right".into()),
        ],
        "u16",
    ));
    cir.push(ci(
        "ret_u16",
        None,
        vec![CIROperand::Var("final".into())],
        "u16",
    ));
    assert!(matches!(
        compile(&ctx("three", &[], "u16"), &cir),
        Err(BackendError::UnsupportedOp(_))
    ));
}

#[test]
fn word02_reversed_subtraction_preserves_primary_and_returns_secondary() {
    let mut cir = word02_binary("sub_u16", "u16", 5, 2);
    cir[2].srcs.reverse(); // 2 - 5 wraps, with the result in BX.
    cir[3] = ci(
        "add_u16",
        Some("restored"),
        vec![
            CIROperand::Var("result".into()),
            CIROperand::Var("left".into()),
        ],
        "u16",
    );
    cir.push(ci(
        "ret_u16",
        None,
        vec![CIROperand::Var("restored".into())],
        "u16",
    ));
    let bytes = compile(&ctx("reverse", &[], "u16"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(100).halted);
    assert_eq!(sim.ax, 2);
}

#[test]
fn word02_rejects_simultaneously_live_mixed_widths() {
    let cir = vec![
        ci("const_u8", Some("byte"), vec![CIROperand::Int(2)], "u8"),
        ci("const_u16", Some("word"), vec![CIROperand::Int(3)], "u16"),
        ci(
            "add_u8",
            Some("byte2"),
            vec![
                CIROperand::Var("byte".into()),
                CIROperand::Var("byte".into()),
            ],
            "u8",
        ),
        ci(
            "add_u16",
            Some("word2"),
            vec![
                CIROperand::Var("word".into()),
                CIROperand::Var("word".into()),
            ],
            "u16",
        ),
        ci(
            "ret_u16",
            None,
            vec![CIROperand::Var("word2".into())],
            "u16",
        ),
    ];
    assert!(
        matches!(compile(&ctx("mixed", &[], "u16"), &cir), Err(BackendError::UnsupportedOp(message)) if message.contains("mixed-width"))
    );
}

#[test]
fn word02_returns_secondary_byte_and_preserves_primary_byte() {
    let mut cir = word02_binary("sub_u8", "u8", 5, 2);
    cir[2].srcs.reverse();
    cir[3] = ci(
        "add_u8",
        Some("restored"),
        vec![
            CIROperand::Var("result".into()),
            CIROperand::Var("left".into()),
        ],
        "u8",
    );
    cir.push(ci(
        "ret_u8",
        None,
        vec![CIROperand::Var("restored".into())],
        "u8",
    ));
    let bytes = compile(&ctx("reverse_byte", &[], "u8"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(100).halted);
    assert_eq!(sim.ax, 2);

    let mut cir = word02_binary("add_u8", "u8", 5, 2);
    cir[3].srcs[0] = CIROperand::Var("right".into());
    let bytes = compile(&ctx("return_second", &[], "u8"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(100).halted);
    assert_eq!(sim.ax, 2);
}

#[test]
fn word02_reports_type_arity_and_missing_binding_errors() {
    let cases = [
        (
            vec![ci("const_u8", Some("x"), vec![CIROperand::Int(1)], "u16")],
            "requires u8 type",
        ),
        (
            vec![ci(
                "add_u16",
                Some("x"),
                vec![CIROperand::Var("a".into())],
                "u16",
            )],
            "two variables",
        ),
        (
            vec![ci(
                "ret_u16",
                None,
                vec![CIROperand::Var("missing".into())],
                "u16",
            )],
            "undefined variable",
        ),
        (
            vec![ci(
                "const_bool",
                Some("x"),
                vec![CIROperand::Int(1)],
                "bool",
            )],
            "must be Bool",
        ),
    ];
    for (cir, expected) in cases {
        let error = compile(&ctx("invalid", &[], "u16"), &cir).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
    let cir = vec![
        ci("const_u16", Some("a"), vec![CIROperand::Int(1)], "u16"),
        ci("const_u16", Some("b"), vec![CIROperand::Int(2)], "u16"),
        ci(
            "add_u8",
            Some("c"),
            vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
            "u8",
        ),
        ci("ret_u8", None, vec![CIROperand::Var("c".into())], "u8"),
    ];
    assert!(compile(&ctx("width", &[], "u8"), &cir)
        .unwrap_err()
        .to_string()
        .contains("source width mismatch"));
    assert_eq!(Intel8086Backend::new().name(), "intel8086");
}

#[test]
fn word02_rejects_malformed_boolean_and_instruction_shapes() {
    let cases = [
        (
            vec![
                ci("const_u8", Some("x"), vec![CIROperand::Int(255)], "u8"),
                ci("ret_bool", None, vec![CIROperand::Var("x".into())], "bool"),
            ],
            "source type mismatch",
        ),
        (
            vec![ci(
                "const_bool",
                Some("x"),
                vec![CIROperand::Bool(true)],
                "u8",
            )],
            "requires bool type",
        ),
        (
            vec![ci(
                "const_u8",
                Some("x"),
                vec![CIROperand::Int(1), CIROperand::Int(2)],
                "u8",
            )],
            "one source",
        ),
        (
            vec![ci("ret_void", None, vec![CIROperand::Int(1)], "void")],
            "no dest or sources",
        ),
        (
            vec![
                ci(
                    "const_bool",
                    Some("x"),
                    vec![CIROperand::Bool(true)],
                    "bool",
                ),
                ci(
                    "ret_bool",
                    Some("y"),
                    vec![CIROperand::Var("x".into())],
                    "bool",
                ),
            ],
            "no dest",
        ),
        (
            vec![
                ci(
                    "const_bool",
                    Some("a"),
                    vec![CIROperand::Bool(true)],
                    "bool",
                ),
                ci("const_u8", Some("b"), vec![CIROperand::Int(2)], "u8"),
                ci(
                    "add_u8",
                    Some("c"),
                    vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                    "u8",
                ),
                ci("ret_u8", None, vec![CIROperand::Var("c".into())], "u8"),
            ],
            "source type mismatch",
        ),
        (
            vec![
                ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
                ci("const_u16", Some("b"), vec![CIROperand::Int(2)], "u16"),
                ci(
                    "add_u16",
                    Some("c"),
                    vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
                    "u16",
                ),
                ci("ret_u16", None, vec![CIROperand::Var("c".into())], "u16"),
            ],
            "source type mismatch",
        ),
    ];
    for (cir, expected) in cases {
        let error = compile(&ctx("malformed", &[], "bool"), &cir).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn empty_cir_emits_hlt() {
    let bytes = compile(&ctx("empty", &[], "void"), &[]).expect("lowering");
    assert_eq!(bytes, vec![0xF4]);
}

#[test]
fn backend_name_is_intel8086() {
    assert_eq!(Intel8086Backend.name(), "intel8086");
}

#[test]
#[should_panic(expected = "intel8086 backend is emit-only")]
fn backend_run_panics_per_spec() {
    Intel8086Backend.run(&[], &[]);
}

/// `const_i64 v=42; ret_i64 v` -> `MOV AX,42; HLT` = `[0xB8, 0x2A, 0x00, 0xF4]`.
///
/// This is the EXACT byte sequence `lang-aot --emit=intel8086` produces
/// for the trivial IIR program `const 42; ret`.
#[test]
fn canonical_const_42_then_ret() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0x2A, 0x00, 0xF4]);
}

/// Byte-for-byte parity is necessary but not sufficient -- genuinely
/// execute the emitted bytes in the new simulator (accounting for
/// segmented CS:IP addressing) and check AX, not just assert a
/// hand-derived byte array.
#[test]
fn canonical_const_42_then_ret_actually_executes_to_ax_equals_42() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("lowering");

    // Load and run at a non-zero CS to prove this genuinely goes through
    // segmented CS:IP physical addressing, not a flat-memory shortcut --
    // CS=0x0010 -> physical base 0x00100 (see
    // intel8086_simulator::simulator::phys_addr).
    let mut sim = Intel8086Simulator::new(1 << 20);
    let cs = 0x0010u16;
    sim.cs = cs;
    let origin = intel8086_simulator::simulator::phys_addr(cs, 0);
    sim.load_program_at(&bytes, origin);

    // Two instructions: MOV AX,42 (materialises 42 into AX) then HLT,
    // which intel8086-simulator's execute() intercepts to set
    // halted = true and stop the fetch-decode-execute loop.
    let result = sim.run_loaded_with_limit(10);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.ax, 42);
    assert!(result.halted);
    assert!(sim.halted);
}

#[test]
fn const_zero_then_ret_emits_four_bytes() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(0)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("zero", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0x00, 0x00, 0xF4], "MOV AX,0; HLT");
}

#[test]
fn const_max_16bit_immediate_works() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(65535)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let bytes = compile(&ctx("max", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes[0..3], [0xB8, 0xFF, 0xFF], "MOV AX,0xFFFF");
}

#[test]
fn const_negative_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(-1)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("neg", &[], "void"), &cir)
        .expect_err("-1 is below the unsigned 16-bit MOV immediate range");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(-1)));
}

#[test]
fn const_over_65535_out_of_range_errors() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(65536)], "i64"),
        ci("ret_void", None, vec![], "void"),
    ];
    let err = compile(&ctx("big", &[], "void"), &cir)
        .expect_err("65536 overflows the unsigned 16-bit MOV immediate");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(65536)));
}

#[test]
fn ret_void_alone_emits_just_hlt() {
    let cir = vec![ci("ret_void", None, vec![], "void")];
    let bytes = compile(&ctx("noop", &[], "void"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xF4]);
}

#[test]
fn const_bool_true() {
    let cir = vec![
        ci(
            "const_bool",
            Some("b"),
            vec![CIROperand::Bool(true)],
            "bool",
        ),
        ci("ret_bool", None, vec![CIROperand::Var("b".into())], "bool"),
    ];
    let bytes = compile(&ctx("btrue", &[], "bool"), &cir).expect("lowering");
    assert_eq!(bytes[0..3], [0xB8, 0x01, 0x00], "MOV AX,1");
}

#[test]
fn word_u16_result_executes_in_ax() {
    let cir = vec![
        ci(
            "const_u16",
            Some("word"),
            vec![CIROperand::Int(0x1234)],
            "u16",
        ),
        ci("ret_u16", None, vec![CIROperand::Var("word".into())], "u16"),
    ];
    let bytes = compile(&ctx("word_result", &[], "u16"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0x34, 0x12, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(sim.ax, 0x1234);
}

#[test]
fn byte_result_is_zero_extended_in_ax() {
    let cir = vec![
        ci("const_u8", Some("byte"), vec![CIROperand::Int(0xFF)], "u8"),
        ci("ret_u8", None, vec![CIROperand::Var("byte".into())], "u8"),
    ];
    let bytes = compile(&ctx("byte_result", &[], "u8"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0xFF, 0x00, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(sim.ax, 0x00FF, "AL=0xFF and AH=0");
}

#[test]
fn const_u8_rejects_word_sized_literal() {
    let cir = vec![ci(
        "const_u8",
        Some("byte"),
        vec![CIROperand::Int(0x100)],
        "u8",
    )];
    let err = compile(&ctx("wide_byte", &[], "u8"), &cir).expect_err("u8 overflow");
    assert!(matches!(err, BackendError::ImmediateOutOfRange(0x100)));
}

#[test]
fn typed_return_must_match_current_result_width() {
    let cir = vec![
        ci(
            "const_u16",
            Some("word"),
            vec![CIROperand::Int(0x1234)],
            "u16",
        ),
        ci("ret_u8", None, vec![CIROperand::Var("word".into())], "u8"),
    ];
    let err = compile(&ctx("wrong_width", &[], "u8"), &cir).expect_err("width mismatch");
    assert!(matches!(err, BackendError::UnsupportedOp(message) if message.contains("16-bit")));
}

#[test]
fn unsupported_op_returns_err() {
    let cir = vec![ci(
        "add_i64",
        Some("c"),
        vec![CIROperand::Var("a".into()), CIROperand::Var("b".into())],
        "i64",
    )];
    let err = compile(&ctx("addtest", &[], "i64"), &cir).expect_err("add not yet supported");
    assert!(matches!(err, BackendError::UnsupportedOp(s) if s == "add_i64"));
}

#[test]
fn multi_const_can_return_first_value() {
    // WORD02 tracks both values, including a return of the first.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("a".into())], "i64"),
    ];
    let bytes = compile(&ctx("two_const_ret_first", &[], "i64"), &cir).unwrap();
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    assert!(sim.run_loaded_with_limit(10).halted);
    assert_eq!(sim.ax, 1);
}

#[test]
fn backend_trait_compile_matches_free_function() {
    let cir = vec![
        ci("const_i64", Some("v"), vec![CIROperand::Int(42)], "i64"),
        ci("ret_i64", None, vec![CIROperand::Var("v".into())], "i64"),
    ];
    let via_trait = Intel8086Backend.compile(&cir).expect("trait compile");
    let via_free_fn = compile(&ctx("fortytwo", &[], "i64"), &cir).expect("free-fn compile");
    assert_eq!(via_trait, via_free_fn);
}

// ===========================================================================
// The terminated:bool regression test
// ===========================================================================
//
// A real bug class was fixed in four prior lanes of this campaign (Intel
// 8051, Intel 8080, MOS 6502, Zilog Z80): the defensive "is the program
// already terminated?" check was written as a trailing-byte-value
// comparison (or an `is_empty()` check), which a legitimate const_*
// immediate's encoded bytes can fool. This test proves
// intel8086-backend does NOT have that bug.

#[test]
fn const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator() {
    // imm = 0xF400 -- `MOV AX,0xF400` encodes as [0xB8, 0x00, 0xF4]. The
    // immediate's high byte (0xF4) numerically collides with HLT's own
    // opcode byte (intel8086_encoder::HALT_BYTE == 0xF4). A naive "is
    // the program already terminated?" check that compares the trailing
    // byte to HALT_BYTE would wrongly conclude this program already
    // halted and skip appending a real HLT -- leaving a 3-byte program
    // with NO genuine halt instruction, even though this CIR has no
    // ret_*/ret_void at all. The correct fix (this backend's
    // `terminated: bool` local) is set true ONLY by a genuine
    // ret_*/ret_void arm, so a program with no ret must always get a
    // real HLT appended, regardless of what byte value precedes it.
    let cir = vec![ci(
        "const_i64",
        Some("v"),
        vec![CIROperand::Int(0xF400)],
        "i64",
    )];
    let bytes = compile(&ctx("collide", &[], "i64"), &cir).expect("lowering");
    assert_eq!(
        bytes,
        vec![0xB8, 0x00, 0xF4, 0xF4],
        "MOV AX,0xF400; HLT -- a real terminator must be appended despite the \
         byte collision between the immediate's high byte and HALT_BYTE"
    );

    // Execute it for good measure: AX must hold 0xF400 and the simulator
    // must have genuinely halted (not run off into garbage memory).
    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted, "the emitted program must contain a real HLT");
    assert_eq!(sim.ax, 0xF400);
    assert_eq!(result.steps, 2, "MOV AX,0xF400, then the appended HLT");
}

#[test]
fn const_whose_low_byte_collides_with_halt_opcode_is_unaffected() {
    // Sanity check the other half of the byte pair: imm = 0x00F4 encodes
    // as [0xB8, 0xF4, 0x00] -- the LOW byte collides with HALT_BYTE, but
    // the trailing byte is 0x00, so even a naive trailing-byte-only
    // check would (by luck) still append HLT here. Included so the test
    // suite documents both halves of the immediate independently.
    let cir = vec![ci(
        "const_i64",
        Some("v"),
        vec![CIROperand::Int(0x00F4)],
        "i64",
    )];
    let bytes = compile(&ctx("collide_lo", &[], "i64"), &cir).expect("lowering");
    assert_eq!(bytes, vec![0xB8, 0xF4, 0x00, 0xF4]);

    let mut sim = Intel8086Simulator::new(65536);
    sim.load_program(&bytes);
    let result = sim.run_loaded_with_limit(10);
    assert!(result.halted);
    assert_eq!(sim.ax, 0x00F4);
}

#[test]
fn multiple_consts_in_a_row_with_no_ret_still_gets_exactly_one_terminator() {
    // Proves the `terminated` flag correctly resets on each subsequent
    // const_* -- if it didn't reset, a stale `true` from some
    // intermediate state could suppress the final HLT.
    let cir = vec![
        ci("const_i64", Some("a"), vec![CIROperand::Int(1)], "i64"),
        ci("const_i64", Some("b"), vec![CIROperand::Int(2)], "i64"),
        ci("const_i64", Some("c"), vec![CIROperand::Int(3)], "i64"),
    ];
    let bytes = compile(&ctx("three_consts", &[], "i64"), &cir).expect("lowering");
    // 3 x MOV AX,#imm16 (3 bytes each) + 1 HLT = 10 bytes.
    assert_eq!(bytes.len(), 10);
    assert_eq!(bytes.last(), Some(&0xF4));
    // Confirm there is exactly one HLT byte at the very end preceded by
    // the last MOV's non-HLT-opcode bytes (0xB8 is the MOV opcode, not
    // 0xF4), i.e. this isn't an accidental double-halt.
    assert_eq!(&bytes[6..10], &[0xB8, 0x03, 0x00, 0xF4]);
}

//! Hostile-input security regression and fuzz tests for `avm-bytecode-lifter` (SWF04).
//!
//! Covers:
//! - CWE-1321: Prototype pollution guards in `AvmEnvironment`
//! - CWE-190: `i64::MIN` negation (`flash_negate`) without debug overflow panic
//! - Malformed/truncated AVM1 `DoAction` streams & invalid branch targets
//! - Malformed/crafted AVM2 ABC streams (valid magic `0x10 0x00 0x2E 0x00` + hostile counts)
//! - Seeded 1,200-case AVM1 + AVM2 fuzz campaign asserting zero panics

#![forbid(unsafe_code)]

use avm_bytecode_lifter::{
    lift_avm1, lift_avm2, run_avm2, AbcParser, Avm1Parser, AvmEnvironment, AvmRunner, LifterError,
    MAX_BYTECODE_SIZE,
};
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use vm_core::value::Value;

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }
}

#[test]
fn hostile_cwe1321_environment_prototype_guards() {
    let env = AvmEnvironment::env_new();
    let obj_id = env.env_new_object("CustomObj");

    for prop in [
        "__proto__",
        "prototype",
        "constructor",
        "__PROTO__",
        "Prototype",
        "CONSTRUCTOR",
    ] {
        assert!(env.env_is_prototype_pollution("Object", prop));
        let err = env
            .env_set_member(obj_id, prop, Value::Int(1))
            .expect_err("forbidden prototype property write must fail");
        assert!(matches!(err, LifterError::PrototypePollutionBlocked(_)));

        let err_var = env
            .env_set_variable(&format!("Object.{}", prop), Value::Int(1))
            .expect_err("forbidden variable path must fail");
        assert!(matches!(err_var, LifterError::PrototypePollutionBlocked(_)));
    }

    // Frozen built-in variable names cannot be overwritten
    for frozen_name in ["Object", "Object.prototype", "Array.prototype", "MovieClip"] {
        let err_frozen = env
            .env_set_variable(frozen_name, Value::Int(99))
            .expect_err("frozen native prototype variable must reject writes");
        assert!(matches!(
            err_frozen,
            LifterError::PrototypePollutionBlocked(_)
        ));
    }
}

#[test]
fn hostile_flash_negate_i64_min_no_overflow_panic() {
    let mut module = IIRModule::new("negate_min", "flash_avm2");
    let instrs = vec![
        IIRInstr::new(
            "call_builtin",
            Some("res".to_string()),
            vec![
                Operand::Var("flash_negate".to_string()),
                Operand::Int(i64::MIN),
            ],
            "any",
        ),
        IIRInstr::new("ret", None, vec![Operand::Var("res".to_string())], "any"),
    ];
    let main_fn = IIRFunction::new("main", vec![], "any", instrs);
    module.add_or_replace(main_fn);
    module.entry_point = Some("main".to_string());

    let mut runner = AvmRunner::runner_new();
    let out = runner
        .runner_execute(&module)
        .expect("flash_negate(i64::MIN) must wrap cleanly without panic");
    assert_eq!(out, Value::Int(i64::MIN));
}

#[test]
fn hostile_avm1_and_avm2_crafted_malformed_inputs() {
    // Oversized bytecode rejected cleanly
    assert!(Avm1Parser::a1p_new(&vec![0u8; MAX_BYTECODE_SIZE + 1]).is_err());
    assert!(AbcParser::a2p_new(&vec![0u8; MAX_BYTECODE_SIZE + 1]).is_err());

    // AVM1 truncated long action (0x96 Push with length 0xFFFF on empty tail)
    assert!(lift_avm1(&[0x96, 0xFF, 0xFF], "trunc_avm1").is_err());
    // AVM1 jump to out-of-bounds target (0x99 Jump, len=2, offset=0x7FFF)
    let bad_jump = [0x99, 0x02, 0x00, 0xFF, 0x7F, 0x00];
    assert!(lift_avm1(&bad_jump, "bad_jump").is_err());

    // AVM2 ABC header (minor=16, major=46) with huge u30 integer count (0xFF 0xFF 0xFF 0xFF 0x0F)
    let crafted_abc_cpool = [0x10, 0x00, 0x2E, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
    assert!(lift_avm2(&crafted_abc_cpool, "huge_cpool").is_err());

    // AVM2 ABC with huge method count
    let crafted_abc_methods = [
        0x10, 0x00, 0x2E, 0x00, // version 46.16
        0, 0, 0, 0, 0, 0, 0, 0, // empty constant pools
        0xFF, 0xFF, 0xFF, 0xFF, 0x07, // huge method count
    ];
    assert!(lift_avm2(&crafted_abc_methods, "huge_methods").is_err());

    // AVM2 ABC bad magic
    assert!(run_avm2(&[0x00, 0x00, 0x00, 0x00]).is_err());
}

#[test]
fn hostile_avm1_and_avm2_fuzz_1200_cases_no_panics() {
    let mut rng = SplitMix64(0x5F04_DEAD_BEEF_CAFE);

    // 600 AVM1 malformed / truncated / random bytecode cases
    for _ in 0..600 {
        let len = rng.next_usize(128);
        let bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        if let Ok(module) = lift_avm1(&bytes, "fuzz_avm1") {
            let mut runner = AvmRunner::runner_new();
            let _ = runner.runner_execute(&module);
        }
    }

    // 600 AVM2 crafted ABC cases (with valid 46.16 magic header so parser dives deep)
    for i in 0..600 {
        let tail_len = rng.next_usize(128);
        let mut bytes = Vec::with_capacity(4 + tail_len);
        if i % 5 == 0 {
            // Completely arbitrary bytes
            for _ in 0..tail_len {
                bytes.push(rng.next_u64() as u8);
            }
        } else {
            // Valid ABC header (minor=16, major=46) + hostile body
            bytes.extend_from_slice(&[0x10, 0x00, 0x2E, 0x00]);
            for _ in 0..tail_len {
                bytes.push(rng.next_u64() as u8);
            }
        }
        if let Ok(module) = lift_avm2(&bytes, "fuzz_avm2") {
            let mut runner = AvmRunner::runner_new();
            let _ = runner.runner_execute(&module);
        }
    }
}

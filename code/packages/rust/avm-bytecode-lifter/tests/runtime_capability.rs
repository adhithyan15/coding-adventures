//! # VER08b runtime capability conformance for `avm-bytecode-lifter`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use std::path::PathBuf;

use avm_bytecode_lifter::{
    lift_avm1, lift_avm2, run_avm1, validate_constant_count, validate_instruction_count,
    validate_stack_depth, AbcParser, Avm1Parser, AvmRunner,
};
use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        // 1. Validation functions
        checks.push(check(
            "validate_stack_depth",
            validate_stack_depth(10).is_ok(),
        ));
        checks.push(check(
            "validate_instruction_count",
            validate_instruction_count(100).is_ok(),
        ));
        checks.push(check(
            "validate_constant_count",
            validate_constant_count(200).is_ok(),
        ));

        // 2. AVM1 Parser & Lifter
        let avm1_bc = vec![0x96, 5, 0, 7, 10, 0, 0, 0, 0x26]; // push 10 + trace
        let mut p = Avm1Parser::a1p_new(&avm1_bc).expect("valid parser");
        let acts = p.a1p_parse_all();
        checks.push(check("a1p_parse_all", acts.is_ok()));

        let mod1 = lift_avm1(&avm1_bc, "avm1_cap");
        checks.push(check("lift_avm1", mod1.is_ok()));

        if let Ok(m) = mod1 {
            let mut runner = AvmRunner::runner_new();
            let exec = runner.runner_execute(&m);
            checks.push(check("runner_execute", exec.is_ok()));
            checks.push(check("runner_output", runner.runner_output() == "10\n"));
        }

        let run_res = run_avm1(&avm1_bc);
        checks.push(check("run_avm1", run_res == Ok("10\n".to_string())));

        // 3. AVM2 ABC Parser & Lifter
        let mut abc = Vec::new();
        abc.extend_from_slice(&16u16.to_le_bytes());
        abc.extend_from_slice(&46u16.to_le_bytes());
        abc.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0]); // cpool
        abc.push(1); // method_count
        abc.extend_from_slice(&[0, 0, 0, 0]); // method 0
        abc.push(0); // metadata
        abc.push(0); // classes
        abc.push(1); // scripts
        abc.extend_from_slice(&[0, 0]); // script 0
        abc.push(1); // bodies
        abc.extend_from_slice(&[0, 8, 4, 0, 0]); // body 0 header
        let code = [0x24, 42, 0x48]; // pushbyte 42, returnvalue
        abc.push(code.len() as u8);
        abc.extend_from_slice(&code);
        abc.extend_from_slice(&[0, 0]); // exceptions, traits

        let mut a2_parser = AbcParser::a2p_new(&abc).expect("valid abc parser");
        let parsed_abc = a2_parser.a2p_parse();
        checks.push(check("a2p_parse", parsed_abc.is_ok()));

        let mod2 = lift_avm2(&abc, "avm2_cap");
        checks.push(check("lift_avm2", mod2.is_ok()));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/avm-bytecode-lifter",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}

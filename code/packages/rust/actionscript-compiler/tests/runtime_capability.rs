//! # VER08b runtime capability conformance for `actionscript-compiler`.
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

use actionscript_compiler::{
    compile_actionscript, run_actionscript, validate_dialect, validate_instruction_count,
    validate_token_count, AsDialect, AsLexer, AsParser, AsVmRunner,
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
        checks.push(check("validate_dialect", validate_dialect(1).is_ok()));
        checks.push(check(
            "validate_token_count",
            validate_token_count(100).is_ok(),
        ));
        checks.push(check(
            "validate_instruction_count",
            validate_instruction_count(200).is_ok(),
        ));

        // 2. Lexer
        let mut lexer = AsLexer::lexer_new("var x = 10;", AsDialect::As1);
        let tokens_res = lexer.lexer_tokenize();
        checks.push(check(
            "lexer_tokenize",
            tokens_res.as_ref().is_ok_and(|toks| toks.len() >= 4),
        ));

        // 3. Parser
        if let Ok(toks) = tokens_res {
            let mut parser = AsParser::parser_new(toks, AsDialect::As1);
            let prog_res = parser.parser_parse_program();
            checks.push(check("parser_parse_program", prog_res.is_ok()));
        }

        // 4. Compiler and VM Runner
        let mod_res = compile_actionscript("var y = 20; trace(y);", AsDialect::As1, "cap_test");
        checks.push(check("compile_actionscript", mod_res.is_ok()));

        if let Ok(module) = mod_res {
            let mut runner = AsVmRunner::runner_new();
            let exec_res = runner.runner_execute(&module);
            checks.push(check("runner_execute", exec_res.is_ok()));
            checks.push(check("runner_output", runner.runner_output() == "20\n"));
        }

        // 5. run_actionscript high-level API
        let run_res = run_actionscript("var z = 30; trace(z);", AsDialect::As1);
        checks.push(check("run_actionscript", run_res == Ok("30\n".to_string())));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/actionscript-compiler",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}

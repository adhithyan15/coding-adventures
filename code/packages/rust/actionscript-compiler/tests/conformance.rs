//! Comprehensive conformance tests for `actionscript-compiler` (SWF03).
//!
//! Tests:
//! - AS1, AS2, AS3 lexing and dialect keyword partitioning
//! - AS1 timeline verbs (stop, play, gotoAndPlay, gotoAndStop, tellTarget, call, eval, onClipEvent, trace)
//! - AS1 paths (_root, _parent, _global, this)
//! - AS2 classes, interfaces, type annotations, visibility, inheritance
//! - AS3 packages, namespaces, const declarations, typed vectors Vector.<T>, try/catch/finally, is/as
//! - End-to-end compilation to InterpreterIR and execution on vm-core
//! - Security and bounds enforcement (MAX_AST_DEPTH, MAX_TOKENS, MAX_INSTRUCTIONS)
//! - Warrant validation functions (validate_dialect, validate_token_count, validate_instruction_count)

use actionscript_compiler::{
    compile_actionscript, run_actionscript, validate_dialect, validate_instruction_count,
    validate_token_count, AsCompilerError, AsDialect, AsLexer, AsVmRunner, TokenKind,
    MAX_AST_DEPTH, MAX_INSTRUCTIONS, MAX_TOKENS,
};

#[test]
fn test_warrant_validation_functions() {
    assert_eq!(validate_dialect(1), Ok(true));
    assert_eq!(validate_dialect(2), Ok(true));
    assert_eq!(validate_dialect(3), Ok(true));
    assert_eq!(
        validate_dialect(0),
        Err(AsCompilerError::UnsupportedDialect)
    );
    assert_eq!(
        validate_dialect(4),
        Err(AsCompilerError::UnsupportedDialect)
    );

    assert_eq!(validate_token_count(0), Ok(true));
    assert_eq!(validate_token_count(MAX_TOKENS), Ok(true));
    assert_eq!(
        validate_token_count(MAX_TOKENS + 1),
        Err(AsCompilerError::TokenLimitExceeded)
    );

    assert_eq!(validate_instruction_count(0), Ok(true));
    assert_eq!(validate_instruction_count(MAX_INSTRUCTIONS), Ok(true));
    assert_eq!(
        validate_instruction_count(MAX_INSTRUCTIONS + 1),
        Err(AsCompilerError::InstructionLimitExceeded)
    );
}

#[test]
fn test_as1_lexer_keywords_and_verbs() {
    let src = "stop(); play(); gotoAndPlay(5); gotoAndStop(10); tellTarget(_root) { stop(); } trace(_global);";
    let mut lexer = AsLexer::lexer_new(src, AsDialect::As1);
    let tokens = lexer.lexer_tokenize().expect("tokenize as1");

    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Stop));
    assert!(kinds.contains(&TokenKind::Play));
    assert!(kinds.contains(&TokenKind::GotoAndPlay));
    assert!(kinds.contains(&TokenKind::GotoAndStop));
    assert!(kinds.contains(&TokenKind::TellTarget));
    assert!(kinds.contains(&TokenKind::Root));
    assert!(kinds.contains(&TokenKind::Global));
    assert!(kinds.contains(&TokenKind::Trace));
}

#[test]
fn test_as2_lexer_classes_and_types() {
    let src = "class Animal extends Object implements IAlive { public var name: String; private static var count: Number = 0; }";
    let mut lexer = AsLexer::lexer_new(src, AsDialect::As2);
    let tokens = lexer.lexer_tokenize().expect("tokenize as2");

    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Class));
    assert!(kinds.contains(&TokenKind::Extends));
    assert!(kinds.contains(&TokenKind::Implements));
    assert!(kinds.contains(&TokenKind::Public));
    assert!(kinds.contains(&TokenKind::Private));
    assert!(kinds.contains(&TokenKind::Static));
}

#[test]
fn test_as3_lexer_packages_and_vectors() {
    let src = "package com.example { import flash.display.*; const MAX: int = 100; var v: Vector.<int> = new Vector.<int>(); try { throw 42; } catch(e: Error) {} finally {} }";
    let mut lexer = AsLexer::lexer_new(src, AsDialect::As3);
    let tokens = lexer.lexer_tokenize().expect("tokenize as3");

    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Package));
    assert!(kinds.contains(&TokenKind::Import));
    assert!(kinds.contains(&TokenKind::Const));
    assert!(kinds.contains(&TokenKind::Vector));
    assert!(kinds.contains(&TokenKind::DotLt));
    assert!(kinds.contains(&TokenKind::Try));
    assert!(kinds.contains(&TokenKind::Catch));
    assert!(kinds.contains(&TokenKind::Finally));
    assert!(kinds.contains(&TokenKind::Throw));
}

#[test]
fn test_as1_execution_arithmetic_and_trace() {
    let src = r#"
        var a = 10;
        var b = 25;
        var sum = a + b;
        trace(sum);
        var diff = b - a;
        trace(diff);
        var prod = a * 3;
        trace(prod);
    "#;
    let out = run_actionscript(src, AsDialect::As1).expect("run as1 arithmetic");
    assert_eq!(out, "35\n15\n30\n");
}

#[test]
fn test_as1_execution_control_flow_while_and_for() {
    let src = r#"
        var count = 0;
        var i = 0;
        while (i < 5) {
            count = count + i;
            i = i + 1;
        }
        trace(count);

        var total = 0;
        for (var j = 1; j <= 4; j++) {
            total = total + j;
        }
        trace(total);
    "#;
    let out = run_actionscript(src, AsDialect::As1).expect("run as1 loops");
    assert_eq!(out, "10\n10\n");
}

#[test]
fn test_as1_execution_functions_and_recursion() {
    let src = r#"
        function factorial(n) {
            if (n <= 1) {
                return 1;
            }
            return n * factorial(n - 1);
        }

        trace(factorial(5));
        trace(factorial(6));
    "#;
    let out = run_actionscript(src, AsDialect::As1).expect("run as1 recursion");
    assert_eq!(out, "120\n720\n");
}

#[test]
fn test_as1_timeline_verbs_logged() {
    let src = r#"
        stop();
        play();
        gotoAndPlay(3);
        gotoAndStop(7);
        tellTarget(_root) {
            stop();
        }
        call(12);
        trace("timeline done");
    "#;
    let module = compile_actionscript(src, AsDialect::As1, "timeline_mod")
        .expect("compile as1 timeline verbs");
    let mut runner = AsVmRunner::runner_new();
    let _ = runner.runner_execute(&module).expect("execute timeline");

    let log = runner.runner_action_log();
    assert_eq!(
        log,
        vec![
            "stop",
            "play",
            "gotoAndPlay(3)",
            "gotoAndStop(7)",
            "tellTarget(_root)",
            "stop",
            "endTellTarget",
            "call(12)",
        ]
    );

    assert_eq!(runner.runner_output(), "timeline done\n");
}

#[test]
fn test_as1_clip_event_registration() {
    let src = r#"
        onClipEvent(load) {
            trace("clip loaded");
        }
    "#;
    let module = compile_actionscript(src, AsDialect::As1, "clip_mod").expect("compile clip event");
    let mut runner = AsVmRunner::runner_new();
    let _ = runner.runner_execute(&module).expect("execute clip event");

    let log = runner.runner_action_log();
    assert_eq!(log, vec!["onClipEvent(load, __onClipEvent_load)"]);
}

#[test]
fn test_as2_class_instantiation_and_methods() {
    let src = r#"
        class Point {
            public var x: Number = 0;
            public var y: Number = 0;

            function Point(init_x: Number, init_y: Number) {
                this.x = init_x;
                this.y = init_y;
            }

            public function getSum(): Number {
                return this.x + this.y;
            }
        }

        var pt = new Point(15, 35);
        trace(pt.getSum());
    "#;
    let out = run_actionscript(src, AsDialect::As2).expect("run as2 class");
    assert_eq!(out, "50\n");
}

#[test]
fn test_as3_package_const_and_vectors() {
    let src = r#"
        package test.pkg {
            const MULTIPLIER: int = 4;
            var vec = new Vector.<int>(10, 20, 30);
            var x = 42;
            var is_num = x is Number;
            trace(is_num);
            var res = vec[1] * MULTIPLIER;
            trace(res);
        }
    "#;
    let out = run_actionscript(src, AsDialect::As3).expect("run as3 package");
    assert_eq!(out, "true\n80\n");
}

#[test]
fn test_as3_try_catch_finally_and_is_as() {
    let src = r#"
        package {
            var msg = "hello world";
            trace(msg is String);
            trace(msg as String);

            var num = 123;
            trace(num as String);

            try {
                trace("in try");
                throw "custom error";
            } catch (e: Error) {
                trace("in catch");
            } finally {
                trace("in finally");
            }
        }
    "#;
    let module = compile_actionscript(src, AsDialect::As3, "as3_try_mod").expect("compile as3 try");
    let mut runner = AsVmRunner::runner_new();
    let _ = runner.runner_execute(&module).expect("run as3 try");

    assert_eq!(
        runner.runner_output(),
        "true\nhello world\nnull\nin try\nin catch\nin finally\n"
    );
    assert_eq!(runner.runner_action_log(), vec!["throw(custom error)"]);
}

#[test]
fn test_recursion_depth_limit_protection() {
    // Generate deeply nested parentheses to exceed MAX_AST_DEPTH
    let mut deeply_nested = String::new();
    for _ in 0..(MAX_AST_DEPTH + 10) {
        deeply_nested.push('(');
    }
    deeply_nested.push_str("42");
    for _ in 0..(MAX_AST_DEPTH + 10) {
        deeply_nested.push(')');
    }
    deeply_nested.push(';');

    let res = compile_actionscript(&deeply_nested, AsDialect::As1, "depth_mod");
    assert!(
        matches!(res, Err(AsCompilerError::RecursionDepthExceeded)),
        "expected RecursionDepthExceeded, got: {:?}",
        res
    );
}

#[test]
fn test_token_limit_protection() {
    // Source containing more than MAX_TOKENS
    let mut big_source = String::with_capacity(MAX_TOKENS * 3);
    for _ in 0..(MAX_TOKENS + 100) {
        big_source.push_str("1 ");
    }

    let mut lexer = AsLexer::lexer_new(&big_source, AsDialect::As1);
    let res = lexer.lexer_tokenize();
    assert!(
        matches!(res, Err(AsCompilerError::TokenLimitExceeded)),
        "expected TokenLimitExceeded, got: {:?}",
        res
    );
}

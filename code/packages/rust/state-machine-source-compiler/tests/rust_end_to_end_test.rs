use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use state_machine::{
    FixtureDefinition, GuardDefinition, InputDefinition, MachineKind, MatcherDefinition,
    PDATransition, PushdownAutomaton, RegisterDefinition, StateDefinition, StateMachineDefinition,
    TokenDefinition, TransitionDefinition, DFA, END_INPUT, EPSILON, NFA,
};
use state_machine_source_compiler::to_rust_source;

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn generated_rust_modules_compile_and_execute_dfa_nfa_and_pda() -> io::Result<()> {
    if !tool_runs("cargo", &["--version"])? || !tool_runs("rustc", &["--version"])? {
        eprintln!("skipping generated Rust e2e test because cargo or rustc is not runnable");
        return Ok(());
    }
    if !fresh_rust_binary_runs()? {
        eprintln!(
            "skipping generated Rust e2e test because freshly built Rust binaries do not run"
        );
        return Ok(());
    }

    let root = temp_project_dir("state-machine-generated-rust-e2e")?;
    let result = (|| {
        write_generated_crate(&root)?;
        let status = run_with_timeout(
            Command::new("cargo")
                .arg("test")
                .arg("--quiet")
                .current_dir(&root),
            Duration::from_secs(120),
        )?;
        assert!(
            status.success,
            "generated Rust crate tests failed\nstdout:\n{}\nstderr:\n{}",
            status.stdout, status.stderr
        );
        Ok(())
    })();
    let _ = fs::remove_dir_all(&root);
    result
}

fn write_generated_crate(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("tests"))?;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let state_machine_path = manifest_dir
        .parent()
        .expect("compiler crate should live beside state-machine")
        .join("state-machine");

    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"generated-state-machine-e2e\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nstate-machine = {{ path = {} }}\n",
            toml_string(&state_machine_path)
        ),
    )?;
    fs::write(root.join("src/lib.rs"), generated_library_source(root)?)?;
    fs::write(root.join("tests/generated_behavior.rs"), behavior_tests())?;
    Ok(())
}

fn generated_library_source(root: &Path) -> io::Result<String> {
    let mut source = String::new();
    source.push_str("pub mod turnstile;\n");
    source.push_str("pub mod contains_ab;\n");
    source.push_str("pub mod balanced_parens;\n");
    source.push_str("pub mod html_skeleton;\n");
    let src_dir = root.join("src");

    let modules = [
        ("turnstile", turnstile_source()?),
        ("contains_ab", contains_ab_source()?),
        ("balanced_parens", balanced_parens_source()?),
        ("html_skeleton", html_skeleton_source()?),
    ];
    for (module, module_source) in modules {
        source.push_str("pub use ");
        source.push_str(module);
        source.push_str("::*;\n");
        fs::write(src_dir.join(format!("{module}.rs")), module_source)?;
    }
    Ok(source)
}

fn turnstile_source() -> io::Result<String> {
    let dfa = DFA::new(
        set(&["locked", "unlocked"]),
        set(&["coin", "push"]),
        HashMap::from([
            (
                ("locked".to_string(), "coin".to_string()),
                "unlocked".to_string(),
            ),
            (
                ("locked".to_string(), "push".to_string()),
                "locked".to_string(),
            ),
            (
                ("unlocked".to_string(), "coin".to_string()),
                "unlocked".to_string(),
            ),
            (
                ("unlocked".to_string(), "push".to_string()),
                "locked".to_string(),
            ),
        ]),
        "locked".to_string(),
        set(&["unlocked"]),
    )
    .map_err(io::Error::other)?;
    to_rust_source(&dfa.to_definition("turnstile")).map_err(io::Error::other)
}

fn contains_ab_source() -> io::Result<String> {
    let nfa = NFA::new(
        set(&["q0", "q1", "q2"]),
        set(&["a", "b"]),
        HashMap::from([
            (("q0".to_string(), "a".to_string()), set(&["q0", "q1"])),
            (("q0".to_string(), "b".to_string()), set(&["q0"])),
            (("q1".to_string(), "b".to_string()), set(&["q2"])),
            (("q2".to_string(), EPSILON.to_string()), set(&["q2"])),
        ]),
        "q0".to_string(),
        set(&["q2"]),
    )
    .map_err(io::Error::other)?;
    to_rust_source(&nfa.to_definition("contains-ab")).map_err(io::Error::other)
}

fn balanced_parens_source() -> io::Result<String> {
    let pda = PushdownAutomaton::new(
        set(&["scan", "accept"]),
        set(&["(", ")"]),
        set(&["$", "("]),
        vec![
            PDATransition {
                source: "scan".to_string(),
                event: Some("(".to_string()),
                stack_read: "$".to_string(),
                target: "scan".to_string(),
                stack_push: vec!["$".to_string(), "(".to_string()],
            },
            PDATransition {
                source: "scan".to_string(),
                event: Some("(".to_string()),
                stack_read: "(".to_string(),
                target: "scan".to_string(),
                stack_push: vec!["(".to_string(), "(".to_string()],
            },
            PDATransition {
                source: "scan".to_string(),
                event: Some(")".to_string()),
                stack_read: "(".to_string(),
                target: "scan".to_string(),
                stack_push: Vec::new(),
            },
            PDATransition {
                source: "scan".to_string(),
                event: None,
                stack_read: "$".to_string(),
                target: "accept".to_string(),
                stack_push: vec!["$".to_string()],
            },
        ],
        "scan".to_string(),
        "$".to_string(),
        set(&["accept"]),
    )
    .map_err(io::Error::other)?;
    to_rust_source(&pda.to_definition("balanced-parens")).map_err(io::Error::other)
}

fn html_skeleton_source() -> io::Result<String> {
    let mut definition = StateMachineDefinition::new("html-skeleton", MachineKind::Transducer);
    definition.version = Some("0.1.0".to_string());
    definition.profile = Some("lexer/v1".to_string());
    definition.runtime_min = Some("state-machine-tokenizer/0.1".to_string());
    definition.initial = Some("data".to_string());
    definition.done = Some("done".to_string());
    definition.alphabet = vec!["<".to_string(), "x".to_string()];
    definition.includes = vec!["html-common".to_string()];
    definition.tokens = vec![TokenDefinition {
        name: "Text".to_string(),
        fields: vec!["data".to_string()],
    }];
    definition.tokens.push(TokenDefinition {
        name: "EOF".to_string(),
        fields: Vec::new(),
    });
    definition.inputs = vec![InputDefinition {
        id: "text".to_string(),
        matcher: MatcherDefinition::Anything,
    }];
    definition.registers = vec![RegisterDefinition {
        id: "text_buffer".to_string(),
        type_name: "string".to_string(),
    }];
    definition.guards = vec![GuardDefinition {
        id: "can_emit".to_string(),
    }];
    definition.fixtures = vec![FixtureDefinition {
        name: "plain-text".to_string(),
        input: "x".to_string(),
        tokens: vec!["Text(data=x)".to_string(), "EOF".to_string()],
    }];
    definition.states = vec![
        StateDefinition {
            initial: true,
            ..StateDefinition::new("data")
        },
        StateDefinition {
            final_state: true,
            ..StateDefinition::new("done")
        },
    ];
    let mut text = TransitionDefinition::new("data", None, vec!["data".to_string()]);
    text.matcher = Some(MatcherDefinition::Anything);
    text.actions = vec!["append_text(current)".to_string()];
    let mut eof = TransitionDefinition::new(
        "data",
        Some(END_INPUT.to_string()),
        vec!["done".to_string()],
    );
    eof.guard = Some("can_emit".to_string());
    eof.actions = vec!["flush_text".to_string(), "emit(EOF)".to_string()];
    eof.consume = false;
    definition.transitions = vec![text, eof];
    to_rust_source(&definition).map_err(io::Error::other)
}

fn behavior_tests() -> &'static str {
    r#"use state_machine::EffectfulInput;

use generated_state_machine_e2e::{
    balanced_parens_pda, contains_ab_nfa, html_skeleton_definition, html_skeleton_transducer,
    turnstile_dfa,
};

#[test]
fn generated_dfa_accepts_turnstile_sequences() {
    let dfa = turnstile_dfa().expect("generated DFA should import");
    assert!(dfa.accepts(&["coin"]));
    assert!(dfa.accepts(&["coin", "coin"]));
    assert!(!dfa.accepts(&["push"]));
    assert!(!dfa.accepts(&["coin", "push"]));
}

#[test]
fn generated_nfa_accepts_contains_ab_language() {
    let nfa = contains_ab_nfa().expect("generated NFA should import");
    assert!(nfa.accepts(&["a", "b"]));
    assert!(nfa.accepts(&["b", "a", "b"]));
    assert!(!nfa.accepts(&["a", "a"]));
    assert!(!nfa.accepts(&["b", "b"]));
}

#[test]
fn generated_pda_accepts_balanced_parentheses() {
    let pda = balanced_parens_pda().expect("generated PDA should import");
    assert!(pda.accepts(&["(", ")"]));
    assert!(pda.accepts(&["(", "(", ")", ")"]));
    assert!(!pda.accepts(&["("]));
    assert!(!pda.accepts(&[")"]));
}

#[test]
fn generated_transducer_emits_effects() {
    let mut transducer = html_skeleton_transducer().expect("generated transducer should import");
    let text = transducer.process(EffectfulInput::event("x")).unwrap();
    assert_eq!(text.effects, vec!["append_text(current)".to_string()]);
    assert!(text.consume);
    let eof = transducer.process(EffectfulInput::end()).unwrap();
    assert_eq!(eof.effects, vec!["flush_text".to_string(), "emit(EOF)".to_string()]);
    assert!(!eof.consume);
    assert_eq!(transducer.current_state(), "done");
    assert!(transducer.is_final());
}

#[test]
fn generated_transducer_definition_preserves_lexer_profile_metadata() {
    let definition = html_skeleton_definition();
    assert_eq!(definition.profile.as_deref(), Some("lexer/v1"));
    assert_eq!(definition.runtime_min.as_deref(), Some("state-machine-tokenizer/0.1"));
    assert_eq!(definition.done.as_deref(), Some("done"));
    assert_eq!(definition.tokens[0].name, "Text");
    assert_eq!(definition.inputs[0].id, "text");
    assert_eq!(definition.registers[0].id, "text_buffer");
    assert_eq!(definition.guards[0].id, "can_emit");
    assert_eq!(definition.fixtures[0].name, "plain-text");
}
"#
}

fn fresh_rust_binary_runs() -> io::Result<bool> {
    let root = temp_project_dir("state-machine-rust-probe")?;
    let source = root.join("main.rs");
    let binary = root.join(if cfg!(windows) { "probe.exe" } else { "probe" });
    let result = (|| {
        fs::write(&source, "fn main() { println!(\"probe-ok\"); }\n")?;
        let compile = run_with_timeout(
            Command::new("rustc").arg(&source).arg("-o").arg(&binary),
            Duration::from_secs(30),
        )?;
        if !compile.success {
            return Ok(false);
        }
        let run = run_with_timeout(&mut Command::new(&binary), Duration::from_secs(10))?;
        Ok(run.success && run.stdout.contains("probe-ok"))
    })();
    let _ = fs::remove_dir_all(&root);
    result
}

fn temp_project_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn tool_runs(program: &str, args: &[&str]) -> io::Result<bool> {
    let status = run_with_timeout(
        Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
        Duration::from_secs(10),
    )?;
    Ok(status.success)
}

fn run_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<CommandResult> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            let output = child.wait_with_output()?;
            return Ok(CommandResult {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            return Ok(CommandResult {
                success: false,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: format!(
                    "{}\nprocess timed out after {} seconds",
                    String::from_utf8_lossy(&output.stderr),
                    timeout.as_secs()
                ),
            });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

struct CommandResult {
    success: bool,
    stdout: String,
    stderr: String,
}

fn toml_string(path: &Path) -> String {
    let mut out = String::from("\"");
    for ch in path.to_string_lossy().chars() {
        match ch {
            '\\' => out.push('/'),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

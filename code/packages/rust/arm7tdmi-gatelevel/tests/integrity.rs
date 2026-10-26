use std::fs;
use std::path::PathBuf;
use std::process::Command;

const ENGINE: &str = include_str!("../src/engine.rs");
const LIB: &str = include_str!("../src/lib.rs");
const STATE: &str = include_str!("../src/state.rs");

const CONDITION: &str = r#"fn condition_passed(cpsr: u32, condition: u8) -> bool {
    gates::condition_holds(cpsr, condition) == 1
}"#;
const ADD: &str = r#"fn add_with_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));
    (result, carry == 1, overflow == 1)
}"#;
const SHIFT: &str = r#"fn shifted(value: u32, kind: u32, amount: u32, immediate: bool, old_carry: bool) -> (u32, bool) {
    gates::arm_shift(value, kind, amount, immediate, old_carry)
}"#;
const CLOCK_BIT: &str = r#"pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}"#;
const MEMORY_WRITE: &str = r#"let old = self.q[address];
        let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
        let mut state: [FlipFlopState; 8] =
            std::array::from_fn(|bit| stable_state((old >> bit) & 1));
        register(&input, 0, &mut state);
        self.q[address] = register(&input, 1, &mut state)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));"#;

fn one(source: &str, needle: &str) -> Result<(), String> {
    if source.matches(needle).count() == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected one exact authority chain, found {}",
            source.matches(needle).count()
        ))
    }
}

fn validate(engine: &str, lib: &str, state: &str) -> Result<(), String> {
    syn::parse_file(engine).map_err(|error| error.to_string())?;
    syn::parse_file(lib).map_err(|error| error.to_string())?;
    syn::parse_file(state).map_err(|error| error.to_string())?;
    for (name, contract) in [("condition", CONDITION), ("add", ADD), ("shift", SHIFT)] {
        one(engine, contract).map_err(|error| format!("{name}: {error}"))?;
    }
    for (name, contract) in [("clock_bit", CLOCK_BIT), ("memory_write", MEMORY_WRITE)] {
        one(state, contract).map_err(|error| format!("{name}: {error}"))?;
    }
    one(state, "register(&input, 0, &mut state);\n    q.copy_from_slice(&register(&input, 1, &mut state));").map_err(|e| format!("clock_bits: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let trace = candidate.step()?;\n        self.commit_transition(&trace.state_after)?;").map_err(|e| format!("step: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let result = candidate.run(max_steps)?;\n        self.commit_transition(&result.final_state)?;").map_err(|e| format!("run: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let result = candidate.execute(program, origin, instruction_set, max_steps)?;\n        self.commit_transition(&result.final_state)?;").map_err(|e| format!("execute: {e}"))?;
    one(
        lib,
        "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n    }",
    )
    .map_err(|e| format!("complete commit: {e}"))?;
    one(lib, "clock_bits(\n                &mut self.registers[0],\n                u64::from(next.registers.common_r0_r7[0]),\n            );\n            self.clock_complete_state(&before);\n            return Err(Arm7TdmiError::CommitFailed);").map_err(|e| format!("rollback: {e}"))?;
    if lib.contains("arm7tdmi_simulator") {
        return Err("production source references the functional oracle".into());
    }
    Ok(())
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "hostile mutation anchor");
    source.replacen(old, new, 1)
}

fn compile_variant(engine: &str, lib: &str, state: &str) {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nonce = format!("{}-{:?}", std::process::id(), std::thread::current().id())
        .replace(['(', ')', ' '], "");
    let root = std::env::temp_dir().join(format!("arm7tdmi-gate-hostile-{nonce}"));
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    let packages = package.parent().unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname='arm7tdmi-gate-hostile'\nversion='0.0.0'\nedition='2021'\n[lib]\npath='src/lib.rs'\n[dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\n",
            packages.join("arithmetic").display(),
            packages.join("logic-gates").display()
        ),
    )
    .unwrap();
    fs::write(src.join("engine.rs"), engine).unwrap();
    fs::write(src.join("lib.rs"), lib).unwrap();
    fs::write(src.join("state.rs"), state).unwrap();
    fs::write(src.join("gates.rs"), include_str!("../src/gates.rs")).unwrap();
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            package.parent().unwrap().join("target/hostile-check"),
        )
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "hostile fixture must compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reviewed_source_to_sink_authority_chains_are_exact() {
    validate(ENGINE, LIB, STATE).unwrap();
}

#[test]
fn compile_valid_hostile_condition_and_helper_bypasses_are_rejected() {
    let host = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "condition == 14 || (cpsr & (1 << 30) != 0 && condition == 0)",
    );
    let mutated = replace_once(ENGINE, CONDITION, &host);
    compile_variant(&mutated, LIB, STATE);
    assert!(validate(&mutated, LIB, STATE).is_err());

    let indirect = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "host_condition(cpsr, condition)",
    );
    let indirect = format!("{indirect}\nfn host_condition(cpsr: u32, condition: u8) -> bool {{ condition == 14 || (condition == 0 && cpsr & (1 << 30) != 0) }}");
    let mutated = replace_once(ENGINE, CONDITION, &indirect);
    compile_variant(&mutated, LIB, STATE);
    assert!(validate(&mutated, LIB, STATE).is_err());
}

#[test]
fn compile_valid_hostile_direct_q_and_post_commit_bypasses_are_rejected() {
    let direct = replace_once(CLOCK_BIT, "register(&input, 0, &mut state);\n    *q = register(&input, 1, &mut state)[0];", "register(&input, 0, &mut state);\n    let _anchor = register(&input, 1, &mut state)[0];\n    *q = u8::from(value);");
    let mutated_state = replace_once(STATE, CLOCK_BIT, &direct);
    compile_variant(ENGINE, LIB, &mutated_state);
    assert!(validate(ENGINE, LIB, &mutated_state).is_err());

    let direct_memory = replace_once(MEMORY_WRITE, "self.q[address] = register(&input, 1, &mut state)\n            .iter()\n            .enumerate()\n            .fold(0, |byte, (bit, q)| byte | (q << bit));", "let _anchor = register(&input, 1, &mut state);\n        self.q[address] = value;");
    let mutated_state = replace_once(STATE, MEMORY_WRITE, &direct_memory);
    compile_variant(ENGINE, LIB, &mutated_state);
    assert!(validate(ENGINE, LIB, &mutated_state).is_err());

    let mutated_lib = replace_once(LIB, "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n    }", "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n        self.halted = 0;\n    }");
    compile_variant(ENGINE, &mutated_lib, STATE);
    assert!(validate(ENGINE, &mutated_lib, STATE).is_err());
}

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn validate(root: &Path) -> Result<(), String> {
    let source_dir = root.join("src");
    let mut combined = String::new();
    for entry in fs::read_dir(&source_dir).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            syn::parse_file(&source).map_err(|error| format!("{}: {error}", path.display()))?;
            combined.push_str(&source);
        }
    }
    let manifest =
        fs::read_to_string(root.join("Cargo.toml")).map_err(|error| error.to_string())?;
    let production_manifest = manifest
        .split("[dev-dependencies]")
        .next()
        .unwrap_or(&manifest);
    for required in [
        "register(&input, 0, &mut state)",
        "register(&input, 1, &mut state)",
        "clock_bit(&mut self.q, candidate.q)",
        "self.memory.restore(&candidate.memory)",
        "crate::gates::add8(left, right, carry)",
        "state.d = bit_or(state.d, operand)",
        "increment16(pc)",
        "let (high, low) = opcode_lines(opcode);",
        "and_gate(high[6], low[8]) == 1",
        "let (high_lines, low_lines) = opcode_lines(opcode);",
        "let decoded = decode_from_lines(opcode, &high_lines, &low_lines)?;",
        "if high_lines[0] == 1",
        "short_control_condition(low, state.q, state.d, state.df, state.ef)",
        "long_branch_condition(low, state.q, state.d, state.df)",
        "long_skip_condition(low, state.q, state.d, state.df, state.ie)",
        "select16(\n        increment16(address)",
        "gates::interrupt_control(self.first_instruction_pending, self.ie)",
        "gates::pack_selectors(self.x.read() as u8, self.p.read() as u8)",
        "if let Err(error) = self.allocation_checkpoint()",
    ] {
        if !combined.contains(required) {
            return Err(format!("required structural path missing: {required}"));
        }
    }
    for forbidden in [
        "wrapping_add(",
        "state.d |= operand",
        "self.q = u8::from(candidate.q)",
        "_detached_gate",
        "decode(opcode)?",
        "decode4(opcode >> 4)",
        "match high",
        "matches!(low",
        "condition(take)",
    ] {
        if combined.contains(forbidden) {
            return Err(format!("host-language bypass found: {forbidden}"));
        }
    }
    if production_manifest.contains("cdp1802-simulator") {
        return Err("functional oracle is a production dependency".into());
    }
    Ok(())
}

fn copy_sources(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        fs::copy(entry.path(), destination).unwrap();
    }
}

fn compile_fixture(root: &Path) {
    let output = Command::new("cargo")
        .args(["check", "--lib", "--offline"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", root.join("shared-target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hostile fixture must type-check before rejection:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn manifest(crate_dir: &Path, include_oracle: bool) -> String {
    let rust = crate_dir.parent().unwrap();
    let oracle = if include_oracle {
        format!(
            "functional-oracle = {{ package = \"cdp1802-simulator\", path = {:?} }}\n",
            rust.join("cdp1802-simulator")
        )
    } else {
        String::new()
    };
    format!(
        "[package]\nname = \"hostile-cdp1802-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\narithmetic = {{ path = {:?} }}\nlogic-gates = {{ path = {:?} }}\n{oracle}",
        rust.join("arithmetic"),
        rust.join("logic-gates")
    )
}

#[test]
fn production_structure_passes_and_compiler_valid_bypasses_fail() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    validate(&crate_dir).unwrap();
    let mutations: [(&str, &str, &str, bool); 11] = [
        (
            "engine.rs",
            "crate::gates::add8(left, right, carry)",
            "(left.wrapping_add(right).wrapping_add(u8::from(carry)), false)",
            false,
        ),
        (
            "engine.rs",
            "let (high_lines, low_lines) = opcode_lines(opcode);",
            "let high_lines = crate::gates::decode4(opcode >> 4);\n    let low_lines = crate::gates::decode4(opcode & 0x0f);",
            false,
        ),
        (
            "decode.rs",
            "if and_gate(high[6], low[8]) == 1 {",
            "if opcode == 0x68 {",
            false,
        ),
        (
            "engine.rs",
            "let take_wire = short_control_condition(low, state.q, state.d, state.df, state.ef);",
            "let take_wire = u8::from(state.q || !state.q);",
            false,
        ),
        (
            "engine.rs",
            "let take_wire = long_branch_condition(low, state.q, state.d, state.df);",
            "let take_wire = u8::from(state.q || !state.q);",
            false,
        ),
        (
            "engine.rs",
            "let take_wire = long_skip_condition(low, state.q, state.d, state.df, state.ie);",
            "let take_wire = u8::from(state.ie || !state.ie);",
            false,
        ),
        (
            "lib.rs",
            "clock_bit(&mut self.q, candidate.q)",
            "self.q = u8::from(candidate.q)",
            false,
        ),
        (
            "lib.rs",
            "mod decode;",
            "mod decode;\nuse functional_oracle::Cdp1802Simulator as HostOracle;\nconst _HOST_ORACLE_ALIAS: Option<HostOracle> = None;",
            true,
        ),
        (
            "engine.rs",
            "let (high_lines, low_lines) = opcode_lines(opcode);",
            "let _detached_gate = opcode_lines(opcode);\n    let high_lines = crate::gates::decode4(opcode >> 4);\n    let low_lines = crate::gates::decode4(opcode & 0x0f);",
            false,
        ),
        (
            "lib.rs",
            "let (inhibited, eligible) =\n            gates::interrupt_control(self.first_instruction_pending, self.ie);",
            "let inhibited = u8::from(self.first_instruction_pending != 0);\n        let eligible = u8::from(self.first_instruction_pending == 0 && self.ie != 0);",
            false,
        ),
        (
            "engine.rs",
            "if let Err(error) = self.allocation_checkpoint() {\n                self.state = rollback;\n                return Err(error);\n            }",
            "if false {\n                self.state = rollback;\n                return Err(Cdp1802Error::AllocationFailed);\n            }",
            false,
        ),
    ];

    for (index, (file, needle, replacement, include_oracle)) in mutations.into_iter().enumerate() {
        let temporary = tempfile::tempdir().unwrap();
        copy_sources(&crate_dir.join("src"), &temporary.path().join("src"));
        fs::write(
            temporary.path().join("Cargo.toml"),
            manifest(&crate_dir, include_oracle),
        )
        .unwrap();
        let path = temporary.path().join("src").join(file);
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            source.contains(needle),
            "mutation {index} did not find its target"
        );
        fs::write(&path, source.replacen(needle, replacement, 1)).unwrap();
        compile_fixture(temporary.path());
        assert!(
            validate(temporary.path()).is_err(),
            "compiler-valid hostile mutation {index} escaped validation"
        );
    }
}

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
        "decode4(opcode >> 4)",
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
fn production_structure_passes_and_five_compiler_valid_bypasses_fail() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    validate(&crate_dir).unwrap();
    let mutations: [(&str, &str, &str, bool); 5] = [
        (
            "engine.rs",
            "crate::gates::add8(left, right, carry)",
            "(left.wrapping_add(right).wrapping_add(u8::from(carry)), false)",
            false,
        ),
        (
            "engine.rs",
            "let high = decode4(opcode >> 4)\n        .into_iter()\n        .position(|wire| wire == 1)\n        .expect(\"four-bit decoder is one-hot\") as u8;",
            "let high = opcode >> 4;",
            false,
        ),
        (
            "engine.rs",
            "let opcode = state.memory[usize::from(pc)];",
            "let _detached_gate = increment16(pc);\n    let opcode = state.memory[usize::from(pc)];",
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

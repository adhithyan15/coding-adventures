//! Structural and compiler-backed evidence for the HALT authority boundary.

use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{ImplItem, Item};

fn compact(tokens: &str) -> String {
    tokens
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn methods(source: &str, owner: &str) -> Result<Vec<(String, String)>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut found = Vec::new();
    for item in file.items {
        let Item::Impl(item_impl) = item else {
            continue;
        };
        if compact(&item_impl.self_ty.to_token_stream().to_string()) != owner {
            continue;
        }
        for item in item_impl.items {
            if let ImplItem::Fn(method) = item {
                found.push((
                    method.sig.ident.to_string(),
                    compact(&method.block.to_token_stream().to_string()),
                ));
            }
        }
    }
    Ok(found)
}

fn one_method(source: &str, owner: &str, name: &str) -> Result<String, String> {
    let matching: Vec<_> = methods(source, owner)?
        .into_iter()
        .filter(|(method, _)| method == name)
        .collect();
    if matching.len() != 1 {
        return Err(format!(
            "expected one {owner}::{name}, found {}",
            matching.len()
        ));
    }
    Ok(matching[0].1.clone())
}

fn require(body: &str, fragment: &str, context: &str) -> Result<(), String> {
    let fragment = compact(fragment);
    if body.contains(&fragment) {
        Ok(())
    } else {
        Err(format!("{context} no longer contains `{fragment}`"))
    }
}

fn validate_halt_authority(cpu: &str, state: &str) -> Result<(), String> {
    let required = [
        "new",
        "reset",
        "load",
        "run",
        "step",
        "snapshot",
        "restore",
        "load_wires_from_state",
        "clock_wires_into_state",
        "halted",
        "clock_halted",
        "sync_halted_observer",
        "physical_storage_inventory",
        "execute",
        "interrupt",
        "nmi",
    ];
    let cpu_methods = methods(cpu, "GateLevelCpu")?;
    for name in required {
        let count = cpu_methods
            .iter()
            .filter(|(method, _)| method == name)
            .count();
        if count != 1 {
            return Err(format!("expected one GateLevelCpu::{name}, found {count}"));
        }
    }

    require(
        &one_method(cpu, "GateLevelCpu", "reset")?,
        "self.clock_halted(false)",
        "reset",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "load")?,
        "self.clock_halted(false)",
        "load",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "run")?,
        "while!self.halted()",
        "run",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "run")?,
        "self.restore(&before)?",
        "run",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "step")?,
        "if self.halted()",
        "step",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "snapshot")?,
        "halted:self.halted()",
        "snapshot",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "restore")?,
        "self.clock_halted(state.halted)",
        "restore",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "load_wires_from_state")?,
        "self.sync_halted_observer()",
        "wire load",
    )?;
    require(
        &one_method(cpu, "GateLevelCpu", "clock_wires_into_state")?,
        "self.sync_halted_observer()",
        "wire commit",
    )?;

    let halted = one_method(cpu, "GateLevelCpu", "halted")?;
    if halted != compact("{ self.halt_state.read() != 0 }") {
        return Err("halted() is no longer the exact HALT-Q observer".into());
    }
    let clock = one_method(cpu, "GateLevelCpu", "clock_halted")?;
    if clock != compact("{ self.halt_state.write(u16::from(value)); self.sync_halted_observer(); }")
    {
        return Err("clock_halted() no longer exclusively clocks Q then synchronizes".into());
    }
    let sync = one_method(cpu, "GateLevelCpu", "sync_halted_observer")?;
    if sync != compact("{ self.halted = self.halted(); }") {
        return Err("the host observer is no longer an exact Q mirror".into());
    }

    for (name, body) in &cpu_methods {
        if name != "clock_halted" && body.contains(&compact("self.halt_state.write")) {
            return Err(format!("{name} writes HALT storage outside clock_halted"));
        }
        if name != "sync_halted_observer" {
            let without_calls = body.replace(&compact("self.halted()"), "");
            if without_calls.contains(&compact("self.halted")) {
                return Err(format!("{name} reads or writes the host HALT observer"));
            }
        }
    }

    let execute = one_method(cpu, "GateLevelCpu", "execute")?;
    if execute.matches(&compact("self.clock_halted(")).count() != 1
        || execute.matches(&compact("self.clock_halted(true)")).count() != 1
    {
        return Err("BRK must clock HALT Q exactly once".into());
    }
    for interrupt in ["interrupt", "nmi"] {
        let body = one_method(cpu, "GateLevelCpu", interrupt)?;
        require(&body, "self.load_wires_from_state()", interrupt)?;
        require(&body, "self.clock_wires_into_state()", interrupt)?;
    }
    let inventory = one_method(cpu, "GateLevelCpu", "physical_storage_inventory")?;
    for fragment in [
        "self.memory.dff_count()",
        "self.rf.dff_count()",
        "self.halt_state.dff_count()",
        "self.input_ports.iter().map(StateRegister::dff_count).sum()",
        "self.output_ports.iter().map(StateRegister::dff_count).sum()",
    ] {
        require(&inventory, fragment, "physical storage inventory")?;
    }

    let write = one_method(state, "StateRegister", "write")?;
    if write.matches(&compact("register(&bits")).count() != 2
        || write.contains(&compact("slave_q="))
        || write.contains(&compact("master_q="))
    {
        return Err("StateRegister::write no longer uses both DFF clock phases".into());
    }
    Ok(())
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "fixture anchor `{old}`");
    source.replacen(old, new, 1)
}

fn fixture_root(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mos6502-halt-authority-{}-{case}",
        std::process::id()
    ))
}

fn compile_full_fixture(case: &str, cpu: &str, state: &str) {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust_packages = package.parent().expect("Rust package directory");
    let root = fixture_root(case);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create fixture source directory");
    for entry in fs::read_dir(package.join("src")).expect("read production sources") {
        let entry = entry.expect("source entry");
        fs::copy(entry.path(), root.join("src").join(entry.file_name())).expect("copy source");
    }
    fs::write(root.join("src/cpu.rs"), cpu).expect("write hostile CPU source");
    fs::write(root.join("src/state.rs"), state).expect("write hostile state source");
    let manifest = format!(
        "[package]\nname = \"mos6502-halt-fixture-{case}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\nlogic-gates = {{ path = {:?} }}\narithmetic = {{ path = {:?} }}\nmos6502-simulator = {{ path = {:?} }}\n",
        rust_packages.join("logic-gates"),
        rust_packages.join("arithmetic"),
        rust_packages.join("mos6502-simulator"),
    );
    fs::write(root.join("Cargo.toml"), manifest).expect("write fixture manifest");
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(std::env::temp_dir().join("mos6502-halt-authority-target"))
        .output()
        .expect("run cargo check for hostile fixture");
    assert!(
        output.status.success(),
        "hostile fixture {case} must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).expect("remove fixture source");
}

#[test]
fn production_halt_transition_chain_matches_the_reviewed_contract() {
    validate_halt_authority(
        include_str!("../src/cpu.rs"),
        include_str!("../src/state.rs"),
    )
    .expect("HALT DFF authority contract");
}

#[test]
fn compiler_checked_hostile_bypasses_are_rejected() {
    let cpu = include_str!("../src/cpu.rs");
    let state = include_str!("../src/state.rs");
    let cases = [
        (
            "observer",
            replace_once(cpu, "if self.halted() {", "if self.halted {"),
            state.to_string(),
        ),
        (
            "helper",
            replace_once(
                cpu,
                "pub fn halted(&self) -> bool {\n        self.halt_state.read() != 0\n    }",
                "pub fn halted(&self) -> bool { self.host_halted() }\n\n    fn host_halted(&self) -> bool { self.halted }",
            ),
            state.to_string(),
        ),
        (
            "dead-anchor",
            replace_once(
                cpu,
                "self.halt_state.write(u16::from(value));\n        self.sync_halted_observer();",
                "if false { self.halt_state.write(u16::from(value)); }\n        self.halted = value;",
            ),
            state.to_string(),
        ),
        (
            "post-commit",
            replace_once(
                cpu,
                "self.clock_halted(true);\n                \"BRK",
                "self.clock_halted(true);\n                self.clock_halted(false);\n                \"BRK",
            ),
            state.to_string(),
        ),
        (
            "direct-q",
            cpu.to_string(),
            replace_once(
                state,
                "register(&bits, 0, &mut self.state);\n        register(&bits, 1, &mut self.state);",
                "for (bit, state) in self.state.iter_mut().enumerate() {\n            let q = bits[bit];\n            state.master_q = q;\n            state.master_q_bar = q ^ 1;\n            state.slave_q = q;\n            state.slave_q_bar = q ^ 1;\n        }",
            ),
        ),
    ];

    for (case, hostile_cpu, hostile_state) in cases {
        compile_full_fixture(case, &hostile_cpu, &hostile_state);
        assert!(
            validate_halt_authority(&hostile_cpu, &hostile_state).is_err(),
            "authority validator accepted hostile fixture {case}"
        );
    }
}

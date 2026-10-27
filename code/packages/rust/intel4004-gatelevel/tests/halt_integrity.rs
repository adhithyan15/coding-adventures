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

fn validate_halt_authority(cpu: &str, registers: &str) -> Result<(), String> {
    let required = [
        "new",
        "halted",
        "physical_storage_inventory",
        "clock_halted",
        "sync_halted_observer",
        "snapshot",
        "load_program",
        "restore",
        "step",
        "run",
        "reset",
        "execute",
    ];
    let cpu_methods = methods(cpu, "Intel4004GateLevel")?;
    for name in required {
        let count = cpu_methods
            .iter()
            .filter(|(method, _)| method == name)
            .count();
        if count != 1 {
            return Err(format!(
                "expected one Intel4004GateLevel::{name}, found {count}"
            ));
        }
    }

    let halted = one_method(cpu, "Intel4004GateLevel", "halted")?;
    if halted != compact("{ self.is_halted.read() != 0 }") {
        return Err("halted() is no longer the exact HALT-Q observer".into());
    }
    let clock = one_method(cpu, "Intel4004GateLevel", "clock_halted")?;
    if clock != compact("{ self.is_halted.write(u16::from(value)); self.sync_halted_observer(); }")
    {
        return Err("clock_halted() no longer exclusively clocks Q then synchronizes".into());
    }
    let sync = one_method(cpu, "Intel4004GateLevel", "sync_halted_observer")?;
    if sync != compact("{ self.halted_observer = self.halted(); }") {
        return Err("the host observer is no longer an exact Q mirror".into());
    }

    for (name, body) in &cpu_methods {
        if !matches!(
            name.as_str(),
            "halted" | "clock_halted" | "physical_storage_inventory"
        ) && body.contains(&compact("self.is_halted."))
        {
            return Err(format!(
                "{name} accesses HALT storage outside the reviewed read/write/inventory methods"
            ));
        }
        if name != "sync_halted_observer" && body.contains(&compact("self.halted_observer")) {
            return Err(format!("{name} reads or writes the host HALT observer"));
        }
    }

    require(
        &one_method(cpu, "Intel4004GateLevel", "snapshot")?,
        "halted: self.halted()",
        "snapshot",
    )?;
    require(
        &one_method(cpu, "Intel4004GateLevel", "load_program")?,
        "self.clock_halted(false)",
        "load_program",
    )?;
    require(
        &one_method(cpu, "Intel4004GateLevel", "restore")?,
        "self.clock_halted(state.halted)",
        "restore",
    )?;
    let step = one_method(cpu, "Intel4004GateLevel", "step")?;
    require(&step, "self.sync_halted_observer()", "step")?;
    require(&step, "if self.halted()", "step")?;
    let run = one_method(cpu, "Intel4004GateLevel", "run")?;
    require(&run, "if self.halted()", "run")?;
    require(&run, "self.restore(&before)?", "run")?;
    require(
        &one_method(cpu, "Intel4004GateLevel", "reset")?,
        "self.clock_halted(false)",
        "reset",
    )?;
    let execute = one_method(cpu, "Intel4004GateLevel", "execute")?;
    if execute.matches(&compact("self.clock_halted(")).count() != 1
        || execute.matches(&compact("self.clock_halted(true)")).count() != 1
    {
        return Err("HLT must clock HALT Q exactly once".into());
    }

    let inventory = one_method(cpu, "Intel4004GateLevel", "physical_storage_inventory")?;
    for fragment in [
        "self.regs.dff_count()",
        "self.acc.dff_count()",
        "self.carry_flag.dff_count()",
        "self.pc.dff_count()",
        "self.stack.dff_count()",
        "self.ram.main_dff_count()",
        "self.ram.status_dff_count()",
        "self.ram.output_dff_count()",
        "self.is_halted.dff_count()",
    ] {
        require(&inventory, fragment, "physical storage inventory")?;
    }

    let write = one_method(registers, "StateRegister", "write")?;
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
        "intel4004-halt-authority-{}-{case}",
        std::process::id()
    ))
}

fn compile_full_fixture(case: &str, cpu: &str, registers: &str) {
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
    fs::write(root.join("src/registers.rs"), registers).expect("write hostile register source");
    let manifest = format!(
        "[package]\nname = \"intel4004-halt-fixture-{case}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nlogic-gates = {{ path = {:?} }}\narithmetic = {{ path = {:?} }}\nintel4004-simulator = {{ path = {:?} }}\n",
        rust_packages.join("logic-gates"),
        rust_packages.join("arithmetic"),
        rust_packages.join("intel4004-simulator"),
    );
    fs::write(root.join("Cargo.toml"), manifest).expect("write fixture manifest");
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--quiet", "--offline", "--lib", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(root.join("target"))
        .output()
        .expect("run cargo check for hostile fixture");
    assert!(
        output.status.success(),
        "hostile fixture {case} must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).expect("remove fixture crate");
}

#[test]
fn production_halt_transition_chain_matches_the_reviewed_contract() {
    validate_halt_authority(
        include_str!("../src/cpu.rs"),
        include_str!("../src/registers.rs"),
    )
    .expect("HALT DFF authority contract");
}

#[test]
fn compiler_checked_hostile_bypasses_are_rejected() {
    let cpu = include_str!("../src/cpu.rs");
    let registers = include_str!("../src/registers.rs");
    let cases = [
        (
            "observer",
            replace_once(
                cpu,
                "self.sync_halted_observer();\n        if self.halted() {",
                "self.sync_halted_observer();\n        if self.halted_observer {",
            ),
            registers.to_string(),
        ),
        (
            "helper",
            replace_once(
                cpu,
                "pub fn halted(&self) -> bool {\n        self.is_halted.read() != 0\n    }",
                "pub fn halted(&self) -> bool { self.host_halted() }\n\n    fn host_halted(&self) -> bool { self.halted_observer }",
            ),
            registers.to_string(),
        ),
        (
            "dead-anchor",
            replace_once(
                cpu,
                "self.is_halted.write(u16::from(value));\n        self.sync_halted_observer();",
                "if false { self.is_halted.write(u16::from(value)); }\n        self.halted_observer = value;",
            ),
            registers.to_string(),
        ),
        (
            "post-commit",
            replace_once(
                cpu,
                "self.clock_halted(true);\n            self.pc.increment();",
                "self.clock_halted(true);\n            self.clock_halted(false);\n            self.pc.increment();",
            ),
            registers.to_string(),
        ),
        (
            "direct-q",
            cpu.to_string(),
            replace_once(
                registers,
                "pub(crate) fn write(&mut self, value: u16) {\n        let bits = int_to_bits(value, self.width);\n        register(&bits, 0, &mut self.state);\n        register(&bits, 1, &mut self.state);\n    }",
                "pub(crate) fn write(&mut self, value: u16) {\n        let bits = int_to_bits(value, self.width);\n        for (bit, state) in self.state.iter_mut().enumerate() {\n            let q = bits[bit];\n            state.master_q = q;\n            state.master_q_bar = q ^ 1;\n            state.slave_q = q;\n            state.slave_q_bar = q ^ 1;\n        }\n    }",
            ),
        ),
    ];

    for (case, hostile_cpu, hostile_registers) in cases {
        compile_full_fixture(case, &hostile_cpu, &hostile_registers);
        assert!(
            validate_halt_authority(&hostile_cpu, &hostile_registers).is_err(),
            "authority validator accepted hostile fixture {case}"
        );
    }
}

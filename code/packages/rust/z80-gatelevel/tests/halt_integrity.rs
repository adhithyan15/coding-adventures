//! Structural and compiler-backed evidence for the HALT authority boundary.

use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{ImplItem, Item};

const CPU_METHOD_HASHES: &[(&str, u64)] = &[
    ("new", 0xd875_3e9a_e785_f11b),
    ("reset", 0x36d3_6c9c_b28c_1533),
    ("load", 0xf51a_9da8_a4a0_bb54),
    ("run", 0x8b1b_edcf_7d74_f4f0),
    ("run_loaded_with_limit", 0x9a06_83d6_1af8_ed16),
    ("get_state", 0xe21b_e4b9_5380_8847),
    ("snapshot", 0xd0da_33d6_093e_42b5),
    ("restore", 0xd9f6_70e6_77a2_b8d2),
    ("interrupt", 0xfe79_d3d2_e0df_a4cb),
    ("nmi", 0xeba2_6f34_f5cc_17a9),
    ("load_wires_from_state", 0x7a03_68c0_bc5d_438f),
    ("clock_wires_into_state", 0x225b_6043_cf57_ddf7),
    ("halted", 0x410a_1b97_fa97_6cfb),
    ("clock_halted", 0x5daf_c883_f868_5183),
    ("sync_halted_observer", 0x9191_6506_4609_84aa),
    ("physical_storage_inventory", 0x9817_b722_0fe6_80c3),
    ("step", 0x0971_2a29_1c05_a7d6),
    ("exec_main", 0x2f99_f638_3eae_54b8),
];

const STATE_REGISTER_WRITE_HASH: u64 = 0x2653_5b7f_4ffe_8965;
const STATE_REGISTER_COUNT_HASH: u64 = 0xdb8d_1ce8_468a_b9ca;
const MEMORY_COUNT_HASH: u64 = 0xabba_a36e_9ada_13d8;
const REGISTER_FILE_COUNT_HASH: u64 = 0x93d8_b505_6fea_6f8c;
const INVENTORY_TOTAL_HASH: u64 = 0x6670_2de5_a7e3_ed39;

fn compact(tokens: &str) -> String {
    tokens
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn canonical_hash(node: &impl ToTokens) -> u64 {
    node.to_token_stream()
        .to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn methods(source: &str, owner: &str) -> Result<Vec<(String, String, u64)>, String> {
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
                    canonical_hash(&method),
                ));
            }
        }
    }
    Ok(found)
}

fn one_method(source: &str, owner: &str, name: &str) -> Result<(String, u64), String> {
    let matching: Vec<_> = methods(source, owner)?
        .into_iter()
        .filter(|(method, _, _)| method == name)
        .collect();
    if matching.len() != 1 {
        return Err(format!(
            "expected one {owner}::{name}, found {}",
            matching.len()
        ));
    }
    Ok((matching[0].1.clone(), matching[0].2))
}

fn validate_halt_authority(cpu: &str, state: &str, registers: &str) -> Result<(), String> {
    let mut mismatches = Vec::new();
    for &(name, expected) in CPU_METHOD_HASHES {
        let (_, actual) = one_method(cpu, "GateLevelCpuZ80", name)?;
        if actual != expected {
            mismatches.push(format!(
                "GateLevelCpuZ80::{name}: expected {expected:#018x}, got {actual:#018x}"
            ));
        }
    }
    for (source, owner, name, expected) in [
        (state, "StateRegister", "write", STATE_REGISTER_WRITE_HASH),
        (
            state,
            "StateRegister",
            "dff_count",
            STATE_REGISTER_COUNT_HASH,
        ),
        (state, "DffMemory", "dff_count", MEMORY_COUNT_HASH),
        (
            registers,
            "RegisterFile",
            "dff_count",
            REGISTER_FILE_COUNT_HASH,
        ),
        (
            cpu,
            "PhysicalStorageInventory",
            "total",
            INVENTORY_TOTAL_HASH,
        ),
    ] {
        let (_, actual) = one_method(source, owner, name)?;
        if actual != expected {
            mismatches.push(format!(
                "{owner}::{name}: expected {expected:#018x}, got {actual:#018x}"
            ));
        }
    }

    let cpu_methods = methods(cpu, "GateLevelCpuZ80")?;
    for (name, body, _) in cpu_methods {
        if name != "clock_halted" && body.contains(&compact("self.halt_state.write")) {
            mismatches.push(format!(
                "GateLevelCpuZ80::{name} writes HALT storage outside clock_halted"
            ));
        }
        if !matches!(name.as_str(), "new" | "sync_halted_observer") {
            let without_q_observations = body.replace(&compact("self.halted()"), "");
            if without_q_observations.contains(&compact("self.halted")) {
                mismatches.push(format!(
                    "GateLevelCpuZ80::{name} reads or writes the host HALT observer"
                ));
            }
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches.join("\n"))
    }
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "fixture anchor `{old}`");
    source.replacen(old, new, 1)
}

fn fixture_root(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!("z80-halt-authority-{}-{case}", std::process::id()))
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
        "[package]\nname = \"z80-halt-fixture-{case}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\nlogic-gates = {{ path = {:?} }}\narithmetic = {{ path = {:?} }}\nz80-simulator = {{ path = {:?} }}\n",
        rust_packages.join("logic-gates"),
        rust_packages.join("arithmetic"),
        rust_packages.join("z80-simulator"),
    );
    fs::write(root.join("Cargo.toml"), manifest).expect("write fixture manifest");
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(std::env::temp_dir().join("z80-halt-authority-target"))
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
        include_str!("../src/registers.rs"),
    )
    .expect("HALT DFF authority contract");
}

#[test]
fn compiler_checked_hostile_bypasses_are_rejected() {
    let cpu = include_str!("../src/cpu.rs");
    let state = include_str!("../src/state.rs");
    let registers = include_str!("../src/registers.rs");
    let cases = [
        (
            "observer",
            replace_once(
                cpu,
                "while !self.halted() && traces.len() < max_steps {",
                "while !self.halted && traces.len() < max_steps {",
            ),
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
                "self.clock_halted(true);\n            return (\"HALT\"",
                "self.clock_halted(true);\n            self.clock_halted(false);\n            return (\"HALT\"",
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
            validate_halt_authority(&hostile_cpu, &hostile_state, registers).is_err(),
            "authority validator accepted hostile fixture {case}"
        );
    }
}

use quote::ToTokens;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const LIB_LOCK: u64 = 0x1f6b_b5a1_17cf_64ec;
const ENGINE_LOCK: u64 = 0x8314_ccdc_a068_c535;
const GATES_LOCK: u64 = 0x6b10_8fb1_b7a0_9416;
const ALLOCATION_LOCK: u64 = 0xae2a_d6b9_ccb8_7638;
const SYSTEM360_LIB_LOCK: u64 = 0x9b0e_87fa_4661_de30;
const SYSTEM360_EXECUTOR_LOCK: u64 = 0x100b_3b6c_09f4_e64f;
const SYSTEM360_GATES_LOCK: u64 = 0x40ac_678d_18ec_7f7e;
const SYSTEM360_HFP_LOCK: u64 = 0xd47e_243c_d4f6_e080;
const SYSTEM360_STATE_LOCK: u64 = 0x3818_2c9d_9d0b_a82f;

fn parsed_lock(source: &str) -> Result<u64, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let normalized = syntax.into_token_stream().to_string();
    Ok(normalized
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        }))
}

fn validate_sources(root: &Path, inherited: &Path) -> Result<(), String> {
    for (name, expected) in [
        ("lib.rs", LIB_LOCK),
        ("engine.rs", ENGINE_LOCK),
        ("gates.rs", GATES_LOCK),
        ("allocation.rs", ALLOCATION_LOCK),
    ] {
        let source = fs::read_to_string(root.join(name)).map_err(|error| error.to_string())?;
        let actual = parsed_lock(&source)?;
        if actual != expected {
            return Err(format!(
                "parsed-token lock changed for {name}: {actual:#018x}"
            ));
        }
    }
    for (name, expected) in [
        ("lib.rs", SYSTEM360_LIB_LOCK),
        ("executor.rs", SYSTEM360_EXECUTOR_LOCK),
        ("gates.rs", SYSTEM360_GATES_LOCK),
        ("hfp.rs", SYSTEM360_HFP_LOCK),
        ("state.rs", SYSTEM360_STATE_LOCK),
    ] {
        let source = fs::read_to_string(inherited.join(name)).map_err(|error| error.to_string())?;
        let actual = parsed_lock(&source)?;
        if actual != expected {
            return Err(format!(
                "parsed-token lock changed for inherited {name}: {actual:#018x}"
            ));
        }
    }
    validate_lib_shape(&fs::read_to_string(root.join("lib.rs")).unwrap())
}

fn validate_lib_shape(source: &str) -> Result<(), String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let structures: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "System370GateLevel" => Some(item),
            _ => None,
        })
        .collect();
    if structures.len() != 1 {
        return Err("expected one System370GateLevel".to_owned());
    }
    let fields: Vec<_> = structures[0]
        .fields
        .iter()
        .map(|field| field.ident.as_ref().map(ToString::to_string))
        .collect();
    if fields != [Some("core".to_owned())] {
        return Err("System/370 must own exactly one embedded core field".to_owned());
    }
    for (name, public) in [("allocation", false), ("engine", false), ("gates", true)] {
        let modules: Vec<_> = syntax
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(module) if module.ident == name => Some(module),
                _ => None,
            })
            .collect();
        if modules.len() != 1 || modules[0].content.is_some() || modules[0].semi.is_none() {
            return Err(format!("{name} must be one external module"));
        }
        let is_public = matches!(modules[0].vis, syn::Visibility::Public(_));
        if is_public != public {
            return Err(format!("{name} visibility changed"));
        }
    }
    Ok(())
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn inherited_source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("ibm-system360-gatelevel/src")
}

#[test]
fn parsed_token_locks_cover_the_complete_live_route() {
    validate_sources(&source_root(), &inherited_source_root()).unwrap();
}

#[test]
fn comments_and_formatting_do_not_change_the_parsed_lock() {
    let source = fs::read_to_string(source_root().join("gates.rs")).unwrap();
    let decorated = format!("// harmless formatting witness\n\n{source}\n/* trailing comment */\n");
    assert_eq!(
        parsed_lock(&source).unwrap(),
        parsed_lock(&decorated).unwrap()
    );
    let lookalike =
        format!("{source}\nconst LOOKALIKE: &str = \"decode_extension core commit\";\n");
    assert_ne!(
        parsed_lock(&source).unwrap(),
        parsed_lock(&lookalike).unwrap()
    );
}

static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

struct TempCrate(PathBuf);

impl TempCrate {
    fn new() -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("system370-hostile-{}-{id}", std::process::id()));
        fs::create_dir_all(path.join("src")).unwrap();
        for name in ["lib.rs", "engine.rs", "gates.rs", "allocation.rs"] {
            fs::copy(source_root().join(name), path.join("src").join(name)).unwrap();
        }
        fs::create_dir_all(path.join("system360/src")).unwrap();
        for name in ["lib.rs", "executor.rs", "gates.rs", "hfp.rs", "state.rs"] {
            fs::copy(
                inherited_source_root().join(name),
                path.join("system360/src").join(name),
            )
            .unwrap();
        }
        let rust = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("package parent");
        let manifest = format!(
            "[package]\nname='system370-hostile'\nversion='0.0.0'\nedition='2021'\n\n[dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\nibm-system360-gatelevel={{path='{}'}}\nibm-system370-simulator={{path='{}'}}\n",
            rust.join("arithmetic").display(),
            rust.join("logic-gates").display(),
            path.join("system360").display(),
            rust.join("ibm-system370-simulator").display(),
        );
        fs::write(path.join("Cargo.toml"), manifest).unwrap();
        let inherited_manifest = format!(
            "[package]\nname='ibm-system360-gatelevel'\nversion='0.1.1'\nedition='2021'\n\n[dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\nibm-system360-simulator={{path='{}'}}\n",
            rust.join("arithmetic").display(),
            rust.join("logic-gates").display(),
            rust.join("ibm-system360-simulator").display(),
        );
        fs::write(path.join("system360/Cargo.toml"), inherited_manifest).unwrap();
        Self(path)
    }

    fn mutate(&self, file: &str, addition: &str) {
        let path = self.0.join(file);
        let mut source = fs::read_to_string(&path).unwrap();
        source.push_str(addition);
        fs::write(path, source).unwrap();
    }

    fn replace(&self, file: &str, needle: &str, replacement: &str) {
        let path = self.0.join(file);
        let source = fs::read_to_string(&path).unwrap();
        assert_eq!(
            source.matches(needle).count(),
            1,
            "unique hostile anchor {needle}"
        );
        fs::write(path, source.replacen(needle, replacement, 1)).unwrap();
    }

    fn check_compiles(&self) {
        let status = Command::new("cargo")
            .args(["check", "--quiet", "--offline", "--manifest-path"])
            .arg(self.0.join("Cargo.toml"))
            .env(
                "CARGO_TARGET_DIR",
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../target"),
            )
            .status()
            .unwrap();
        assert!(
            status.success(),
            "hostile crate must compile before rejection"
        );
    }
}

impl Drop for TempCrate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_complete_compiler_valid_hostile_crate_is_rejected() {
    let mutations = [
        ("src/lib.rs", "\nstruct HostStateMirror(System370State);\n"),
        ("src/lib.rs", "\nfn direct_q_write(cpu: &mut System370GateLevel) { cpu.core.clock_gpr_q(0, 0); cpu.core.clock_memory_q(0, 0); }\n"),
        ("src/lib.rs", "\nfn direct_q_read(cpu: &System370GateLevel) -> (u32, u8) { (cpu.core.gpr_q(0), cpu.core.memory_q(0)) }\n"),
        ("src/lib.rs", "\nfn post_commit_overwrite(cpu: &mut System370GateLevel) { let _ = cpu.set_program_counter(0); }\n"),
        ("src/lib.rs", "\nfn post_trace_overwrite(trace: &mut StepTrace) { trace.pc_after = 0; }\n"),
        ("src/lib.rs", "\nfn host_authored_final_state(cpu: &System370GateLevel) -> System370State { cpu.get_state().unwrap() }\n"),
        ("src/lib.rs", "\nuse engine as routed_engine;\n"),
        ("src/lib.rs", "\nmacro_rules! discarded_route { () => { mod fake_engine; } }\n"),
        ("src/lib.rs", "\nfn oracle_authored_transition() -> System370State { System370Simulator::new().get_state() }\n"),
        ("src/engine.rs", "\nfn raw_host_dispatch(opcode: u8) -> bool { matches!(opcode, 0x0e | 0x0f | 0xbd..=0xbf) }\n"),
        ("src/engine.rs", "\nfn native_architecture(a: u32, b: u32) -> bool { a.wrapping_add(1) < b && a != 0 }\n"),
        ("src/engine.rs", "\nfn helper_indirection(opcode: u8) -> Option<Extension> { let helper = classify_extension; helper(opcode) }\n"),
        ("src/engine.rs", "\n#[cfg(any())] fn inactive_decoder(opcode: u8) -> Option<Extension> { classify_extension(opcode) }\n"),
        ("src/engine.rs", "\nfn duplicate_decoder(opcode: u8) -> Option<Extension> { classify_extension(opcode) }\n"),
        ("src/engine.rs", "\nfn post_datapath_overwrite(settled: &mut SettledExtension) { settled.pc = 0; }\n"),
        ("src/gates.rs", "\nfn host_ordering(left: u32, right: u32) -> bool { left < right }\n"),
        ("src/gates.rs", "\nfn shadow_gate_network(opcode: u8) -> [u8; 5] { decode_extension(opcode) }\n"),
        ("src/gates.rs", "\nconst EXPECTATION_SHADOW: [u8; 5] = [1, 0, 0, 0, 0];\n"),
        ("src/allocation.rs", "\nfn bypass_allocation_preflight() -> bool { true }\n"),
        ("system360/src/lib.rs", "\nstruct PackedHostMirror(System360State);\n"),
        ("system360/src/executor.rs", "\nfn duplicate_inherited_executor(before: &System360State, trace: &StepTrace) -> Result<System360State, System360Error> { let authority = DecodeAuthority::new(trace.raw[0])?; execute(before, before.clone(), trace, &authority) }\n"),
    ];
    for (file, mutation) in mutations {
        let hostile = TempCrate::new();
        hostile.mutate(file, mutation);
        hostile.check_compiles();
        assert!(
            validate_sources(&hostile.0.join("src"), &hostile.0.join("system360/src")).is_err(),
            "mutation in {file}"
        );
    }

    let active = [
        (
            "src/gates.rs",
            "from_bits(&ripple_carry_adder_with_carry(&bits_u24(a), &bits_u24(b), 0).sum)",
            "a.wrapping_add(b) & 0x00ff_ffff",
        ),
        (
            "system360/src/lib.rs",
            "halted: self.halted != 0,",
            "halted: false,",
        ),
        (
            "system360/src/lib.rs",
            "let gate_next = executor::execute(&before, candidate, &trace, &authority)?;",
            "let gate_next = oracle_next.clone();",
        ),
        (
            "system360/src/lib.rs",
            "clock_bit(&mut self.halted, next.halted);",
            "clock_bit(&mut self.halted, false);",
        ),
    ];
    for (file, needle, replacement) in active {
        let hostile = TempCrate::new();
        hostile.replace(file, needle, replacement);
        hostile.check_compiles();
        assert!(
            validate_sources(&hostile.0.join("src"), &hostile.0.join("system360/src")).is_err(),
            "active mutation in {file}"
        );
    }

    for replacement in [
        "pub mod engine;",
        "#[path = \"engine.rs\"] mod engine;",
        "mod engine { include!(\"engine.rs\"); }",
        "macro_rules! declare_engine { () => { mod engine; } } declare_engine!();",
    ] {
        let hostile = TempCrate::new();
        hostile.replace("src/lib.rs", "mod engine;", replacement);
        if replacement.contains("include!") {
            hostile.replace(
                "src/engine.rs",
                "//! Independent settled-wire engine for the five System/370 extension orders.",
                "/// Independent settled-wire engine for the five System/370 extension orders.",
            );
        }
        hostile.check_compiles();
        assert!(
            validate_sources(&hostile.0.join("src"), &hostile.0.join("system360/src")).is_err(),
            "module route {replacement}"
        );
    }
}

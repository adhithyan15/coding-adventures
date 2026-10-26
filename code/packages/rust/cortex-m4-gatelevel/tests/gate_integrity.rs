use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{ImplItem, Item, ItemStruct};

const LIB: &str = include_str!("../src/lib.rs");
const GATES: &str = include_str!("../src/gates.rs");
const FLOATING: &str = include_str!("../src/floating.rs");
const LIB_LOCK: u64 = 0x029d_190e_4ced_6d01;
const GATES_LOCK: u64 = 0x2247_5d85_4726_a83f;
const FLOATING_LOCK: u64 = 0x399c_d0ef_65f1_face;

fn fnv1a64(source: &str) -> u64 {
    source
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn parse(source: &str) -> syn::File {
    syn::parse_file(source).expect("the complete hostile fixture must remain valid Rust syntax")
}

fn copy_sources(destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
    }
}

fn fixture_manifest(include_oracle: bool) -> String {
    let rust = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned();
    let oracle = if include_oracle {
        format!(
            "functional-oracle = {{ package = \"cortex-m4-simulator\", path = {:?} }}\n",
            rust.join("cortex-m4-simulator")
        )
    } else {
        String::new()
    };
    format!(
        "[package]\nname = \"hostile-cortex-m4-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\narithmetic = {{ path = {:?} }}\nlogic-gates = {{ path = {:?} }}\ncortex-m3-gatelevel = {{ path = {:?} }}\n{oracle}",
        rust.join("arithmetic"),
        rust.join("logic-gates"),
        rust.join("cortex-m3-gatelevel"),
    )
}

fn compile_complete_fixture(
    root: &Path,
    target: &Path,
    lib: &str,
    gates: &str,
    floating: &str,
    include_oracle: bool,
) {
    let source = root.join("src");
    copy_sources(&source);
    fs::write(source.join("lib.rs"), lib).unwrap();
    fs::write(source.join("gates.rs"), gates).unwrap();
    fs::write(source.join("floating.rs"), floating).unwrap();
    fs::write(root.join("Cargo.toml"), fixture_manifest(include_oracle)).unwrap();
    let output = Command::new("cargo")
        .args(["check", "--lib", "--offline"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hostile full-crate fixture must compile before rejection:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn count_function(source: &str, name: &str) -> usize {
    source.matches(&format!("fn {name}(")).count()
}

fn validate_production(lib: &str, gates: &str, floating: &str) -> Result<(), String> {
    let lib_syntax = syn::parse_file(lib).map_err(|error| error.to_string())?;
    syn::parse_file(gates).map_err(|error| error.to_string())?;
    syn::parse_file(floating).map_err(|error| error.to_string())?;

    if fnv1a64(lib) != LIB_LOCK
        || fnv1a64(gates) != GATES_LOCK
        || fnv1a64(floating) != FLOATING_LOCK
    {
        return Err(format!(
            "source lock changed: {:016x} {:016x} {:016x}",
            fnv1a64(lib),
            fnv1a64(gates),
            fnv1a64(floating)
        ));
    }
    let all = format!("{lib}\n{gates}\n{floating}");
    for forbidden in [
        "cortex_m4_simulator",
        "f32::",
        ": f32",
        "-> f32",
        "functional_oracle",
        "post_commit",
    ] {
        if all.contains(forbidden) {
            return Err(format!("forbidden production authority: {forbidden}"));
        }
    }
    for name in [
        "classify_extension",
        "execute_extension",
        "execute_saturating",
        "execute_parallel",
        "execute_dsp_multiply",
        "execute_fp_binary",
        "execute_fp_unary",
        "execute_fp_compare",
        "execute_fp_status",
        "commit",
        "clock_extension",
    ] {
        if count_function(lib, name) != 1 {
            return Err(format!("missing or duplicate authority function {name}"));
        }
    }
    for name in ["binary", "square_root", "compare", "pack"] {
        if count_function(floating, name) != 1 {
            return Err(format!("missing or duplicate floating authority {name}"));
        }
    }

    let machine = lib_syntax
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(ItemStruct { ident, fields, .. }) if ident == "CortexM4GateLevel" => {
                Some(fields)
            }
            _ => None,
        })
        .ok_or_else(|| "missing CortexM4GateLevel".to_owned())?;
    let core_fields = machine
        .iter()
        .filter(|field| {
            quote::quote!(#field)
                .to_string()
                .contains("CortexM3GateLevel")
        })
        .count();
    if core_fields != 1 {
        return Err("the machine must own exactly one inherited executor".to_owned());
    }
    let steps: Vec<_> = lib_syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(block) => Some(block),
            _ => None,
        })
        .flat_map(|block| &block.items)
        .filter_map(|item| match item {
            ImplItem::Fn(method) if method.sig.ident == "step" => Some(method),
            _ => None,
        })
        .collect();
    if steps.len() != 1 {
        return Err("expected one public step authority".to_owned());
    }
    let step_tokens = quote::quote!(#(#steps)*).to_string();
    if step_tokens.matches("inherited . step").count() != 1 {
        return Err("step must route inherited instructions once".to_owned());
    }
    Ok(())
}

#[test]
fn approved_production_sources_are_exact_and_independent() {
    validate_production(LIB, GATES, FLOATING).unwrap();
}

#[test]
fn compile_valid_hostile_full_crate_mutations_are_rejected() {
    let duplicate_executor = LIB.replace(
        "fn classify_extension(",
        "fn alternate_inherited_executor() -> CortexM3GateLevel { CortexM3GateLevel::new() }\nfn classify_extension(",
    );
    let mirror = LIB
        .replace(
            "fpscr: [u8; 32],",
            "fpscr: [u8; 32],\n    post_commit_fpscr: u32,",
        )
        .replace(
            "fpscr: [0; 32],",
            "fpscr: [0; 32],\n            post_commit_fpscr: 0,",
        );
    let hostile = [
        (
            LIB.replace(
                "use cortex_m3_gatelevel::{",
                "use functional_oracle::CortexM4Simulator as HostOracle;\nconst _HOST_ORACLE_ALIAS: Option<HostOracle> = None;\nuse cortex_m3_gatelevel::{",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "oracle routing",
            true,
        ),
        (
            duplicate_executor,
            GATES.to_owned(),
            FLOATING.to_owned(),
            "duplicate inherited executor",
            false,
        ),
        (
            LIB.replace(
                "fn classify_extension(",
                "fn raw_host_dispatch(raw: u16) -> bool { raw == 0xee00 }\nfn classify_extension(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "raw host decode",
            false,
        ),
        (
            LIB.replace(
                "fn execute_saturating(",
                "fn host_dsp_result_flag_predicate_write_enable(left: u32, right: u32) -> (u32, u8, bool, u8) { let result = left.wrapping_add(right); (result, u8::from(result < left), result == 0, u8::from(left != right)) }\nfn execute_saturating(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "host DSP authority",
            false,
        ),
        (
            LIB.to_owned(),
            GATES.to_owned(),
            FLOATING.replace(
                "pub fn binary(",
                "fn host_float(value: u32) -> f32 { f32::from_bits(value) }\npub fn binary(",
            ),
            "host floating authority",
            false,
        ),
        (
            mirror,
            GATES.to_owned(),
            FLOATING.to_owned(),
            "hidden post-commit mirror",
            false,
        ),
        (
            LIB.replace(
                "fn execute_extension(",
                "fn execute_extension_shadow() {}\nfn execute_extension(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "dead shadow helper",
            false,
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    let target = temporary.path().join("target");
    for (index, (lib, gates, floating, label, include_oracle)) in hostile.into_iter().enumerate() {
        parse(&lib);
        parse(&gates);
        parse(&floating);
        let root = temporary.path().join(format!("fixture-{index}"));
        fs::create_dir_all(&root).unwrap();
        compile_complete_fixture(&root, &target, &lib, &gates, &floating, include_oracle);
        assert!(
            validate_production(&lib, &gates, &floating).is_err(),
            "hostile {label} mutation was accepted"
        );
    }
}

use syn::{ImplItem, Item, ItemStruct};

const LIB: &str = include_str!("../src/lib.rs");
const GATES: &str = include_str!("../src/gates.rs");
const FLOATING: &str = include_str!("../src/floating.rs");
const LIB_LOCK: u64 = 0xb3e2_6f39_89a4_98f7;
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
    let hostile = [
        (
            LIB.replace(
                "use cortex_m3_gatelevel::{",
                "use cortex_m4_simulator as functional_oracle;\nuse cortex_m3_gatelevel::{",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "oracle routing",
        ),
        (
            LIB.replace(
                "core: CortexM3GateLevel,",
                "core: CortexM3GateLevel,\n    duplicate_core: CortexM3GateLevel,",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "duplicate inherited executor",
        ),
        (
            LIB.replace(
                "fn classify_extension(",
                "fn raw_host_dispatch(raw: u16) -> bool { raw == 0xee00 }\nfn classify_extension(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "raw host decode",
        ),
        (
            LIB.replace(
                "fn execute_saturating(",
                "fn host_dsp_result(left: u32, right: u32) -> u32 { left.wrapping_add(right) }\nfn execute_saturating(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "host DSP authority",
        ),
        (
            LIB.to_owned(),
            GATES.to_owned(),
            FLOATING.replace(
                "pub fn binary(",
                "fn host_float(value: u32) -> f32 { f32::from_bits(value) }\npub fn binary(",
            ),
            "host floating authority",
        ),
        (
            LIB.replace(
                "fpscr: [u8; 32],",
                "fpscr: [u8; 32],\n    post_commit_fpscr: u32,",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "hidden post-commit mirror",
        ),
        (
            LIB.replace(
                "fn execute_extension(",
                "fn execute_extension_shadow() {}\nfn execute_extension(",
            ),
            GATES.to_owned(),
            FLOATING.to_owned(),
            "dead shadow helper",
        ),
    ];

    for (lib, gates, floating, label) in hostile {
        parse(&lib);
        parse(&gates);
        parse(&floating);
        assert!(
            validate_production(&lib, &gates, &floating).is_err(),
            "hostile {label} mutation was accepted"
        );
    }
}

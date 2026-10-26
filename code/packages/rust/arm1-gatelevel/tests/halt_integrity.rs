use quote::ToTokens;
use syn::{ImplItem, Item};

const METHOD_HASHES: &[(&str, u64)] = &[
    ("reset", 0x4844_e86b_9a11_be2c),
    ("halted", 0xe303_7904_64e6_2c54),
    ("physical_storage_inventory", 0x0db9_b48c_057f_5278),
    ("clock_halted", 0x2b67_0210_a26a_e083),
    ("synchronize_halted_observer", 0xacdc_9bff_edfa_c3d4),
    ("get_state", 0xac14_9e35_f627_1ca9),
    ("restore", 0xedaa_bd7b_75ca_1859),
    ("load_at_checked", 0x8670_3e95_f689_6cf5),
    ("raise_irq", 0xe25d_d1ad_75a7_90c5),
    ("raise_fiq", 0xbb1b_d6b1_aad1_835e),
    ("step", 0x2b6d_b91d_0539_55d3),
    ("run", 0xd76b_46a2_c1cd_e4c5),
    ("step_checked", 0x0e26_5ee8_47fe_2cd9),
    ("run_loaded_checked", 0xf419_cac7_6a31_2d00),
    ("run_checked", 0x9602_0f0a_44b8_c7af),
    ("execute_swi", 0x090d_6721_1abf_414f),
];

const CLOCK_BIT_HASH: u64 = 0x998a_1834_aa6f_841b;

fn canonical_hash(node: &impl ToTokens) -> u64 {
    node.to_token_stream()
        .to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn method_hash(source: &str, name: &str) -> Result<u64, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let methods: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(item_impl) => Some(item_impl),
            _ => None,
        })
        .flat_map(|item_impl| item_impl.items.iter())
        .filter_map(|item| match item {
            ImplItem::Fn(method) if method.sig.ident == name => Some(method),
            _ => None,
        })
        .collect();
    if methods.len() != 1 {
        return Err(format!(
            "expected exactly one method named {name}, found {}",
            methods.len()
        ));
    }
    Ok(canonical_hash(methods[0]))
}

fn function_hash(source: &str, name: &str) -> Result<u64, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let functions: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .collect();
    if functions.len() != 1 {
        return Err(format!(
            "expected exactly one function named {name}, found {}",
            functions.len()
        ));
    }
    Ok(canonical_hash(functions[0]))
}

fn validate_halt_authority(source: &str, state_source: &str) -> Result<(), String> {
    let mut mismatches = Vec::new();
    for &(name, expected) in METHOD_HASHES {
        let actual = method_hash(source, name)?;
        if actual != expected {
            mismatches.push(format!(
                "HALT authority method {name} changed: expected {expected:#018x}, got {actual:#018x}"
            ));
        }
    }
    let actual = function_hash(state_source, "clock_bit")?;
    if actual != CLOCK_BIT_HASH {
        mismatches.push(format!(
            "clock_bit changed: expected {CLOCK_BIT_HASH:#018x}, got {actual:#018x}"
        ));
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches.join("\n"))
    }
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(
        source.matches(old).count(),
        1,
        "mutation anchor must be exact"
    );
    source.replacen(old, new, 1)
}

#[test]
fn production_halt_transition_chain_matches_the_reviewed_contract() {
    validate_halt_authority(
        include_str!("../src/lib.rs"),
        include_str!("../src/state.rs"),
    )
    .expect("HALT DFF authority contract");
}

#[test]
fn hostile_observer_helper_and_post_clock_bypasses_are_rejected() {
    let source = include_str!("../src/lib.rs");
    let state = include_str!("../src/state.rs");

    let observer = replace_once(
        source,
        "let flags_before = self.flags();\n\n        if self.halted() {",
        "let flags_before = self.flags();\n\n        if self.halted {",
    );
    syn::parse_file(&observer).expect("direct-observer fixture is valid Rust syntax");
    assert!(validate_halt_authority(&observer, state).is_err());

    let helper = replace_once(
        source,
        "pub fn halted(&self) -> bool {\n        self.halt_q[0] != 0\n    }",
        "pub fn halted(&self) -> bool { self.host_halted() }\n\n    fn host_halted(&self) -> bool { self.halted }",
    );
    syn::parse_file(&helper).expect("helper-indirection fixture is valid Rust syntax");
    assert!(validate_halt_authority(&helper, state).is_err());

    let post_clock = replace_once(
        source,
        "self.clock_halted(true);\n            return;",
        "self.clock_halted(true);\n            self.halt_q[0] = 0;\n            return;",
    );
    syn::parse_file(&post_clock).expect("post-clock fixture is valid Rust syntax");
    assert!(validate_halt_authority(&post_clock, state).is_err());
}

#[test]
fn hostile_direct_q_and_dead_anchor_clock_bypasses_are_rejected() {
    let source = include_str!("../src/lib.rs");
    let state = include_str!("../src/state.rs");

    let direct_q = replace_once(
        source,
        "clock_bit(&mut self.halt_q[0], value);",
        "self.halt_q[0] = u8::from(value);",
    );
    syn::parse_file(&direct_q).expect("direct-Q fixture is valid Rust syntax");
    assert!(validate_halt_authority(&direct_q, state).is_err());

    let dead_anchor = replace_once(
        source,
        "clock_bit(&mut self.halt_q[0], value);",
        "if false { clock_bit(&mut self.halt_q[0], value); }\n        self.halt_q[0] = u8::from(value);",
    );
    syn::parse_file(&dead_anchor).expect("dead-anchor fixture is valid Rust syntax");
    assert!(validate_halt_authority(&dead_anchor, state).is_err());

    let state_bypass = replace_once(
        state,
        "*q = register(&input, 1, &mut state)[0];",
        "let _clocked = register(&input, 1, &mut state)[0];\n    *q = u8::from(value);",
    );
    syn::parse_file(&state_bypass).expect("clock-bit bypass fixture is valid Rust syntax");
    assert!(validate_halt_authority(source, &state_bypass).is_err());
}

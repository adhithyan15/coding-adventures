use quote::ToTokens;
use syn::{Item, ItemFn, ItemMod};

const RELATION_FROM_SUBTRACT: &str = r#"
fn relation_from_subtract(zero: u8, less: u8) -> RelationWires {
    let unequal = not_gate(zero);
    RelationWires {
        lt: and_gate(unequal, less),
        gt: and_gate(unequal, not_gate(less)),
        eq: zero,
    }
}
"#;

const WRITE_CR_FIELD: &str = r#"
pub fn write_cr_field(old_cr: u32, field_selector: [u8; 3], relation: RelationWires, so: u8,) -> u32 {
    let selected = decode_lines::<3, 8>(&field_selector);
    let old = u32_to_bits(old_cr);
    let nibble = relation_bits(relation, so);
    let updated: [u8; 32] = std::array::from_fn(|bit| {
        let architectural_field = 7 - bit / 4;
        mux2(old[bit], nibble[bit % 4], selected[architectural_field])
    });
    bits_to_u32(&updated)
}
"#;

const WRITE_CR0: &str = r#"
pub fn write_cr0(old_cr: u32, value: u32, so: u8, write_enable: u8) -> u32 {
    let candidate = write_cr_field(old_cr, [0, 0, 0], compare_signed(value, 0), so);
    let old = u32_to_bits(old_cr);
    let candidate = u32_to_bits(candidate);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        mux2(old[bit], candidate[bit], write_enable)
    }))
}
"#;

const WRITE_CA: &str = r#"
pub fn write_ca(old_xer: u32, new_ca: u8, write_enable: u8) -> u32 {
    let mut updated = u32_to_bits(old_xer);
    updated[29] = mux2(updated[29], new_ca, write_enable);
    bits_to_u32(&updated)
}
"#;

const MTCRF: &str = r#"
pub fn mtcrf(old_cr: u32, source: u32, field_mask: [u8; 8]) -> u32 {
    let old = u32_to_bits(old_cr);
    let source = u32_to_bits(source);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        let architectural_field = 7 - bit / 4;
        mux2(old[bit], source[bit], field_mask[7 - architectural_field])
    }))
}
"#;

const SHIFT_CA: &str = r#"
pub fn arithmetic_right_shift_ca(source: u32, count: [u8; 6]) -> u8 {
    let source = u32_to_bits(source);
    let sign = source[31];
    let mut discarded_by_count = [0_u8; 64];
    let mut discarded = 0;
    for (bit, source_wire) in source.into_iter().enumerate() {
        discarded = or_gate(discarded, source_wire);
        discarded_by_count[bit + 1] = and_gate(sign, discarded);
    }
    discarded_by_count[33..].fill(and_gate(sign, discarded));
    let selected = decode_lines::<6, 64>(&count);
    (0..64).fold(0, |ca, line| {
        or_gate(ca, and_gate(selected[line], discarded_by_count[line]))
    })
}
"#;

const SET_CA: &str = r#"
fn set_ca(state: &mut PowerPcState, value: u8, write_enable: u8) {
    state.xer = status::write_ca(state.xer, value, write_enable);
}
"#;

const SET_CR_FIELD: &str = r#"
fn set_cr_field(state: &mut PowerPcState, field: [u8; 3], relation: status::RelationWires) {
    state.cr = status::write_cr_field(state.cr, field, relation, status::so(state.xer));
}
"#;

const UPDATE_CR0: &str = r#"
fn update_cr0(state: &mut PowerPcState, value: u32, write_enable: u8) {
    state.cr = status::write_cr0(state.cr, value, status::so(state.xer), write_enable);
}
"#;

const EQUAL_LINE: &str = r#"
fn equal_line(value: u32, width: usize, expected: u32) -> u8 {
    (0..width).fold(1, |selected, bit| {
        let actual = ((value >> bit) & 1) as u8;
        let wanted = ((expected >> bit) & 1) as u8;
        and_gate(selected, xnor_gate(actual, wanted))
    })
}
"#;

fn canonical(node: &impl ToTokens) -> String {
    node.to_token_stream().to_string()
}

fn unique_function<'a>(file: &'a syn::File, name: &str) -> Result<&'a ItemFn, String> {
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
            "expected one top-level {name} function, found {}",
            functions.len()
        ));
    }
    if functions[0]
        .attrs
        .iter()
        .any(|attribute| !attribute.path().is_ident("doc"))
    {
        return Err(format!("{name} has an unapproved attribute"));
    }
    Ok(functions[0])
}

fn exact_function(source: &str, name: &str, expected: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let actual = unique_function(&file, name)?;
    let expected: ItemFn = syn::parse_str(expected).map_err(|error| error.to_string())?;
    let mut actual = actual.clone();
    actual.attrs.clear();
    if canonical(&actual) != canonical(&expected) {
        return Err(format!(
            "{name} no longer matches the approved gate chain\nactual: {}\nexpected: {}",
            canonical(&actual),
            canonical(&expected)
        ));
    }
    Ok(())
}

fn exact_status_module(source: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let modules: Vec<_> = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if module.ident == "status" => Some(module),
            _ => None,
        })
        .collect();
    if modules.len() != 1 {
        return Err(format!(
            "expected one top-level status module, found {}",
            modules.len()
        ));
    }
    let expected: ItemMod = syn::parse_str("pub mod status;").unwrap();
    if canonical(modules[0]) != canonical(&expected) {
        return Err("status must be the public, external, attribute-free binding".to_owned());
    }
    Ok(())
}

fn validate_status(source: &str) -> Result<(), String> {
    for (name, expected) in [
        ("relation_from_subtract", RELATION_FROM_SUBTRACT),
        ("write_cr_field", WRITE_CR_FIELD),
        ("write_cr0", WRITE_CR0),
        ("write_ca", WRITE_CA),
        ("mtcrf", MTCRF),
        ("arithmetic_right_shift_ca", SHIFT_CA),
    ] {
        exact_function(source, name, expected)?;
    }
    Ok(())
}

fn validate_consumers(source: &str) -> Result<(), String> {
    for (name, expected) in [
        ("set_ca", SET_CA),
        ("set_cr_field", SET_CR_FIELD),
        ("update_cr0", UPDATE_CR0),
    ] {
        exact_function(source, name, expected)?;
    }
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let execute = unique_function(&syntax, "execute_x31")?;
    let tokens = canonical(execute);
    for required in [
        "decoder :: decode (raw)",
        "status :: compare_signed",
        "status :: compare_unsigned",
        "status :: mtcrf",
        "status :: arithmetic_right_shift_ca",
        "status :: ca (state . xer)",
        "decoded . oe_wire",
        "decoded . rc_wire",
    ] {
        if !tokens.contains(required) {
            return Err(format!(
                "execute_x31 lost required chain fragment {required}"
            ));
        }
    }
    Ok(())
}

fn validate_decoder(source: &str) -> Result<(), String> {
    exact_function(source, "equal_line", EQUAL_LINE)?;
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let decode = unique_function(&syntax, "decode")?;
    let tokens = canonical(decode);
    for required in [
        "rc_wire : (raw & 1) as u8",
        "oe_wire : ((raw >> 10) & 1) as u8",
        "cr_field_wires : std :: array :: from_fn",
        "mtcrf_mask_wires : std :: array :: from_fn",
        "subfic : opcode (8)",
        "compare_immediate : or_gate",
        "compare_register : or_gate",
        "recordable_x",
        "mtcrf : xo10_line (144)",
        "addc : xo9_line (10)",
        "adde : xo9_line (138)",
        "sraw : xo10_line (792)",
        "srawi : xo10_line (824)",
    ] {
        if !tokens.contains(required) {
            return Err(format!("decoder lost status-wire fragment {required}"));
        }
    }
    Ok(())
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(
        source.matches(old).count(),
        1,
        "fixture mutation must be exact"
    );
    source.replacen(old, new, 1)
}

#[test]
fn production_status_module_and_complete_commit_chain_match_reviewed_ast() {
    exact_status_module(include_str!("../src/lib.rs")).unwrap();
    validate_decoder(include_str!("../src/decoder.rs")).unwrap();
    validate_status(include_str!("../src/status.rs")).unwrap();
    validate_consumers(include_str!("../src/cpu.rs")).unwrap();
}

#[test]
fn hostile_ast_rejects_host_decoded_or_forged_status_lines() {
    let decoder = include_str!("../src/decoder.rs");
    let host_equality = replace_once(
        decoder,
        "and_gate(selected, xnor_gate(actual, wanted))",
        "and_gate(selected, u8::from(actual == wanted))",
    );
    let forged_addc = replace_once(decoder, "addc: xo9_line(10),", "addc: 1,");
    for hostile in [host_equality, forged_addc] {
        syn::parse_file(&hostile).expect("decoder fixture is compile-valid Rust syntax");
        assert!(validate_decoder(&hostile).is_err());
    }
}

#[test]
fn hostile_ast_rejects_host_decisions_direct_updates_and_dead_anchors() {
    let status = include_str!("../src/status.rs");
    let host_relation = replace_once(
        status,
        "let unequal = not_gate(zero);",
        "let unequal = u8::from(zero == 0);",
    );
    let direct_ca = replace_once(
        status,
        "let mut updated = u32_to_bits(old_xer);",
        "let direct = (old_xer & !(1 << 29)) | (u32::from(new_ca) << 29); let mut updated = u32_to_bits(direct);",
    );
    let shadow = replace_once(
        status,
        "let mut updated = u32_to_bits(old_xer);",
        "fn mux2(_: u8, data: u8, _: u8) -> u8 { data } let mut updated = u32_to_bits(old_xer);",
    );
    for hostile in [host_relation, direct_ca, shadow] {
        syn::parse_file(&hostile).expect("hostile fixture is compile-valid Rust syntax");
        assert!(validate_status(&hostile).is_err());
    }

    let cpu = include_str!("../src/cpu.rs");
    let dead_anchor = replace_once(
        cpu,
        "state.xer = status::write_ca(state.xer, value, write_enable);",
        "let _anchor = status::write_ca(state.xer, value, write_enable); state.xer = (state.xer & !(1 << 29)) | (u32::from(value) << 29);",
    );
    let post_overwrite = replace_once(
        cpu,
        "state.cr = status::write_cr0(state.cr, value, status::so(state.xer), write_enable);",
        "state.cr = status::write_cr0(state.cr, value, status::so(state.xer), write_enable); state.cr = value;",
    );
    let oracle = replace_once(
        cpu,
        "state.cr = status::write_cr_field(state.cr, field, relation, status::so(state.xer));",
        "let _ = (field, relation); state.cr = PowerPc601Simulator::new().get_state().cr;",
    );
    for hostile in [dead_anchor, post_overwrite, oracle] {
        syn::parse_file(&hostile).expect("hostile consumer is compile-valid Rust syntax");
        assert!(validate_consumers(&hostile).is_err());
    }
}

#[test]
fn hostile_ast_rejects_alternate_duplicate_cfg_path_inline_and_macro_routing() {
    let root = include_str!("../src/lib.rs");
    let fixtures = [
        replace_once(
            root,
            "pub mod status;",
            "#[path = \"status.rs\"] pub mod status;",
        ),
        replace_once(root, "pub mod status;", "pub mod status {}"),
        replace_once(
            root,
            "pub mod status;",
            "#[cfg(any())] pub mod status; pub mod status;",
        ),
        replace_once(
            root,
            "pub mod status;",
            "macro_rules! route { () => { pub mod status; } } route!();",
        ),
    ];
    for fixture in fixtures {
        syn::parse_file(&fixture).expect("routing fixture is compile-valid Rust syntax");
        assert!(exact_status_module(&fixture).is_err());
    }
}

#[test]
fn ast_parser_ignores_comment_string_and_character_literal_confusers() {
    let mut status = include_str!("../src/status.rs").to_owned();
    status.push_str(
        r#"
const _SCANNER_STRING: &str = "} state.xer = old & !mask; if left < right {";
const _SCANNER_CHARACTER: char = '}';
// mod status { fn write_ca() { state.xer >>= 29; } }
/* "unterminated-looking delimiters {{{" and direct |= mask text */
"#,
    );
    syn::parse_file(&status).expect("confuser fixture remains valid Rust syntax");
    validate_status(&status).expect("AST validation is unaffected by lexical confusers");
}

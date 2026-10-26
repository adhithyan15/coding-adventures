use quote::ToTokens;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use syn::visit::{self, Visit};
use syn::{Expr, ExprAssign, ExprCall, Item, ItemFn, ItemMod};

const EXECUTE_RAW_AST_HASH: u64 = 0xf3be_0974_d1ac_4a66;
const EXECUTE_X31_AST_HASH: u64 = 0x8f48_602a_2f06_2968;

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

const SIGNED_RELATION: &str = r#"
pub fn signed_relation(zero: u8, negative: u8, overflow: u8) -> RelationWires {
    relation_from_subtract(zero, xor_gate(negative, overflow))
}
"#;

const UNSIGNED_RELATION: &str = r#"
pub fn unsigned_relation(zero: u8, carry: u8) -> RelationWires {
    relation_from_subtract(zero, not_gate(carry))
}
"#;

const COMPARE_SIGNED: &str = r#"
pub fn compare_signed(left: u32, right: u32) -> RelationWires {
    let difference = alu::sub(left, right);
    signed_relation(difference.zero, difference.negative, difference.overflow)
}
"#;

const COMPARE_UNSIGNED: &str = r#"
pub fn compare_unsigned(left: u32, right: u32) -> RelationWires {
    let difference = alu::sub(left, right);
    unsigned_relation(difference.zero, difference.carry)
}
"#;

const SELECT_RELATION: &str = r#"
pub fn select_relation(signed: RelationWires, unsigned: RelationWires, signed_enable: u8, unsigned_enable: u8,) -> RelationWires {
    RelationWires {
        lt: or_gate(and_gate(signed.lt, signed_enable), and_gate(unsigned.lt, unsigned_enable),),
        gt: or_gate(and_gate(signed.gt, signed_enable), and_gate(unsigned.gt, unsigned_enable),),
        eq: or_gate(and_gate(signed.eq, signed_enable), and_gate(unsigned.eq, unsigned_enable),),
    }
}
"#;

const DECODE_LINES: &str = r#"
fn decode_lines<const INPUTS: usize, const OUTPUTS: usize>(inputs: &[u8; INPUTS]) -> [u8; OUTPUTS] {
    std::array::from_fn(|line| {
        inputs.iter().enumerate().fold(1, |selected, (bit, input)| {
            let expected = ((line >> bit) & 1) as u8;
            and_gate(selected, xnor_gate(*input, expected))
        })
    })
}
"#;

const RELATION_BITS: &str = r#"
fn relation_bits(relation: RelationWires, so: u8) -> [u8; 4] {
    [so, relation.eq, relation.gt, relation.lt]
}
"#;

const WRITE_CR_FIELD: &str = r#"
pub fn write_cr_field(old_cr: u32, field_selector: [u8; 3], relation: RelationWires, so: u8, write_enable: u8,) -> u32 {
    let selected = decode_lines::<3, 8>(&field_selector);
    let old = u32_to_bits(old_cr);
    let nibble = relation_bits(relation, so);
    let updated: [u8; 32] = std::array::from_fn(|bit| {
        let architectural_field = 7 - bit / 4;
        let field_write = and_gate(selected[architectural_field], write_enable);
        mux2(old[bit], nibble[bit % 4], field_write)
    });
    bits_to_u32(&updated)
}
"#;

const WRITE_CR0: &str = r#"
pub fn write_cr0(old_cr: u32, value: u32, so: u8, write_enable: u8) -> u32 {
    let candidate = write_cr_field(old_cr, [0, 0, 0], compare_signed(value, 0), so, 1);
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
pub fn mtcrf(old_cr: u32, source: u32, field_mask: [u8; 8], write_enable: u8) -> u32 {
    let old = u32_to_bits(old_cr);
    let source = u32_to_bits(source);
    bits_to_u32(&std::array::from_fn::<_, 32, _>(|bit| {
        let architectural_field = 7 - bit / 4;
        let field_write = and_gate(field_mask[7 - architectural_field], write_enable);
        mux2(old[bit], source[bit], field_write)
    }))
}
"#;

const SO: &str = r#"
pub fn so(xer: u32) -> u8 {
    u32_to_bits(xer)[31]
}
"#;

const CA: &str = r#"
pub fn ca(xer: u32) -> u8 {
    u32_to_bits(xer)[29]
}
"#;

const COUNT_WIRES: &str = r#"
pub fn count_wires(value: u32) -> [u8; 6] {
    let bits = u32_to_bits(value);
    std::array::from_fn(|bit| bits[bit])
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
fn set_cr_field(state: &mut PowerPcState, field: [u8; 3], relation: status::RelationWires, write_enable: u8,) {
    state.cr = status::write_cr_field(state.cr, field, relation, status::so(state.xer), write_enable,);
}
"#;

const UPDATE_CR0: &str = r#"
fn update_cr0(state: &mut PowerPcState, value: u32, write_enable: u8) {
    state.cr = status::write_cr0(state.cr, value, status::so(state.xer), write_enable);
}
"#;

const EXECUTE_COMPARE_IMMEDIATE: &str = r#"
fn execute_compare_immediate(state: &mut PowerPcState, decoded: &decoder::DecodedInstruction, opcode: u32, immediate: u16, simm: u32, next: u32,) -> &'static str {
    let relation = status::select_relation(
        status::compare_signed(state.gpr[decoded.ra], simm),
        status::compare_unsigned(state.gpr[decoded.ra], u32::from(immediate)),
        decoded.status.compare_immediate_signed,
        decoded.status.compare_immediate_unsigned,
    );
    set_cr_field(state, decoded.cr_field_wires, relation, decoded.status.compare_immediate,);
    state.cia = next;
    match opcode {
        10 => "CMPLI",
        11 => "CMPI",
        _ => unreachable!(),
    }
}
"#;

const EXECUTE_COMPARE_REGISTER: &str = r#"
fn execute_compare_register(state: &mut PowerPcState, decoded: &decoder::DecodedInstruction, xo10: u32, ra: usize, rb: usize,) -> &'static str {
    let relation = status::select_relation(
        status::compare_signed(state.gpr[ra], state.gpr[rb]),
        status::compare_unsigned(state.gpr[ra], state.gpr[rb]),
        decoded.status.compare_register_signed,
        decoded.status.compare_register_unsigned,
    );
    set_cr_field(state, decoded.cr_field_wires, relation, decoded.status.compare_register,);
    match xo10 {
        0 => "CMP",
        32 => "CMPL",
        _ => unreachable!(),
    }
}
"#;

const EXECUTE_MTCRF: &str = r#"
fn execute_mtcrf(state: &mut PowerPcState, decoded: &decoder::DecodedInstruction, source_register: usize,) -> &'static str {
    state.cr = status::mtcrf(state.cr, state.gpr[source_register], decoded.mtcrf_mask_wires, decoded.status.mtcrf,);
    "MTCRF"
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

const DECODE: &str = r#"
pub fn decode(raw: u32) -> DecodedInstruction {
    let xo10 = ((raw >> 1) & 0x3ff) as u16;
    let opcode_value = raw >> 26;
    let opcode = |expected| equal_line(opcode_value, 6, expected);
    let primary31 = opcode(31);
    let xo10_line = |expected| and_gate(primary31, equal_line(u32::from(xo10), 10, expected));
    let xo9_line = |expected| and_gate(primary31, equal_line(u32::from(xo10 & 0x1ff), 9, expected));
    let compare_immediate_unsigned = opcode(10);
    let compare_immediate_signed = opcode(11);
    let compare_register_signed = xo10_line(0);
    let compare_register_unsigned = xo10_line(32);
    let recordable_x = [24, 26, 28, 124, 316, 444, 476, 536, 792, 824]
        .into_iter()
        .fold(0, |line, xo| or_gate(line, xo10_line(xo)));
    DecodedInstruction {
        raw,
        opcode: opcode_value as u8,
        rd: ((raw >> 21) & 0x1f) as usize,
        ra: ((raw >> 16) & 0x1f) as usize,
        rb: ((raw >> 11) & 0x1f) as usize,
        xo10,
        xo9: xo10 & 0x1ff,
        immediate: raw as u16,
        record: raw & 1 != 0,
        rc_wire: (raw & 1) as u8,
        oe_wire: ((raw >> 10) & 1) as u8,
        cr_field_wires: std::array::from_fn(|bit| ((raw >> (23 + bit)) & 1) as u8),
        mtcrf_mask_wires: std::array::from_fn(|bit| ((raw >> (12 + bit)) & 1) as u8),
        status: StatusOpcodeWires {
            subfic: opcode(8),
            andi: opcode(28),
            andis: opcode(29),
            compare_immediate: or_gate(compare_immediate_unsigned, compare_immediate_signed),
            compare_immediate_signed,
            compare_immediate_unsigned,
            compare_register: or_gate(compare_register_signed, compare_register_unsigned),
            compare_register_signed,
            compare_register_unsigned,
            recordable_x,
            mtcrf: xo10_line(144),
            addc: xo9_line(10),
            adde: xo9_line(138),
            sraw: xo10_line(792),
            srawi: xo10_line(824),
        },
        absolute: raw & 2 != 0,
        link: raw & 1 != 0,
    }
}
"#;

fn canonical(node: &impl ToTokens) -> String {
    node.to_token_stream().to_string()
}

fn canonical_hash(node: &impl ToTokens) -> u64 {
    canonical(node)
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
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

fn exact_function_hash(source: &str, name: &str, expected: u64) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let actual = unique_function(&file, name)?;
    let found = canonical_hash(actual);
    if found != expected {
        return Err(format!(
            "{name} full dispatcher AST changed: expected {expected:#018x}, found {found:#018x}"
        ));
    }
    Ok(())
}

#[derive(Default)]
struct NestedHelperShadows {
    forbidden: Vec<String>,
    found: Vec<String>,
}

impl<'ast> Visit<'ast> for NestedHelperShadows {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        let name = function.sig.ident.to_string();
        if self.forbidden.contains(&name) {
            self.found.push(name);
        }
        visit::visit_item_fn(self, function);
    }
}

fn reject_nested_helper_shadows(
    source: &str,
    function: &str,
    forbidden: &[&str],
) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let function = unique_function(&file, function)?;
    let mut shadows = NestedHelperShadows {
        forbidden: forbidden.iter().map(|name| (*name).to_owned()).collect(),
        found: Vec::new(),
    };
    shadows.visit_block(&function.block);
    if shadows.found.is_empty() {
        Ok(())
    } else {
        Err(format!("nested helper shadows found: {:?}", shadows.found))
    }
}

fn called_name(call: &ExprCall) -> Option<String> {
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    path.path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn writes_state(left: &Expr) -> bool {
    let target = canonical(left).replace(' ', "");
    target == "state"
        || target == "*state"
        || target.starts_with("state.")
        || target.starts_with("state[")
        || target.starts_with("(*state).")
}

struct PostHelperWrites {
    helpers: Vec<String>,
    helper_seen: bool,
    write_seen: bool,
}

impl<'ast> Visit<'ast> for PostHelperWrites {
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        visit::visit_expr_call(self, call);
        if called_name(call).is_some_and(|name| self.helpers.contains(&name)) {
            self.helper_seen = true;
        }
    }

    fn visit_expr_assign(&mut self, assignment: &'ast ExprAssign) {
        if self.helper_seen && writes_state(&assignment.left) {
            self.write_seen = true;
        }
        visit::visit_expr_assign(self, assignment);
    }
}

struct DispatcherArmSafety {
    helpers: Vec<String>,
    post_helper_write: bool,
}

impl<'ast> Visit<'ast> for DispatcherArmSafety {
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let mut writes = PostHelperWrites {
            helpers: self.helpers.clone(),
            helper_seen: false,
            write_seen: false,
        };
        writes.visit_expr(&arm.body);
        self.post_helper_write |= writes.write_seen;
        visit::visit_arm(self, arm);
    }
}

fn reject_post_helper_state_writes(
    source: &str,
    function: &str,
    helpers: &[&str],
) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let function_name = function;
    let function = unique_function(&file, function_name)?;
    let mut safety = DispatcherArmSafety {
        helpers: helpers.iter().map(|name| (*name).to_owned()).collect(),
        post_helper_write: false,
    };
    safety.visit_block(&function.block);
    if safety.post_helper_write {
        Err(format!(
            "{function_name} writes whole state or a state field after an approved helper"
        ))
    } else {
        Ok(())
    }
}

fn assert_rustc_accepts(source: &str) {
    static FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);
    let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    let output_path = std::env::temp_dir().join(format!(
        "powerpc601-status-hostile-{}-{id}.rmeta",
        std::process::id()
    ));
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut child = Command::new(rustc)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-o",
        ])
        .arg(&output_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("rustc must start for compile-checked hostile fixtures");
    child
        .stdin
        .take()
        .expect("rustc stdin")
        .write_all(source.as_bytes())
        .expect("write hostile fixture to rustc");
    let output = child.wait_with_output().expect("rustc must finish");
    let _ = std::fs::remove_file(&output_path);
    assert!(
        output.status.success(),
        "hostile fixture must compile, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
        ("signed_relation", SIGNED_RELATION),
        ("unsigned_relation", UNSIGNED_RELATION),
        ("compare_signed", COMPARE_SIGNED),
        ("compare_unsigned", COMPARE_UNSIGNED),
        ("select_relation", SELECT_RELATION),
        ("decode_lines", DECODE_LINES),
        ("relation_bits", RELATION_BITS),
        ("write_cr_field", WRITE_CR_FIELD),
        ("write_cr0", WRITE_CR0),
        ("write_ca", WRITE_CA),
        ("so", SO),
        ("ca", CA),
        ("mtcrf", MTCRF),
        ("count_wires", COUNT_WIRES),
        ("arithmetic_right_shift_ca", SHIFT_CA),
    ] {
        exact_function(source, name, expected)?;
    }
    Ok(())
}

fn validate_consumers(source: &str) -> Result<(), String> {
    for (name, expected) in [
        ("execute_compare_immediate", EXECUTE_COMPARE_IMMEDIATE),
        ("execute_compare_register", EXECUTE_COMPARE_REGISTER),
        ("execute_mtcrf", EXECUTE_MTCRF),
        ("set_ca", SET_CA),
        ("set_cr_field", SET_CR_FIELD),
        ("update_cr0", UPDATE_CR0),
    ] {
        exact_function(source, name, expected)?;
    }
    exact_function_hash(source, "execute_raw", EXECUTE_RAW_AST_HASH)?;
    exact_function_hash(source, "execute_x31", EXECUTE_X31_AST_HASH)?;
    reject_nested_helper_shadows(source, "execute_raw", &["execute_compare_immediate"])?;
    reject_nested_helper_shadows(
        source,
        "execute_x31",
        &["execute_compare_register", "execute_mtcrf"],
    )?;
    reject_post_helper_state_writes(source, "execute_raw", &["execute_compare_immediate"])?;
    reject_post_helper_state_writes(
        source,
        "execute_x31",
        &["execute_compare_register", "execute_mtcrf"],
    )?;
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let execute_raw = canonical(unique_function(&syntax, "execute_raw")?);
    let execute_x31 = canonical(unique_function(&syntax, "execute_x31")?);
    for (tokens, required) in [
        (
            &execute_raw,
            "10 | 11 => Ok (execute_compare_immediate (state , & decoded , opcode , immediate , simm , next ,))",
        ),
        (
            &execute_x31,
            "0 | 32 => execute_compare_register (state , & decoded , xo10 , ra , rb)",
        ),
        (
            &execute_x31,
            "144 => execute_mtcrf (state , & decoded , rs)",
        ),
    ] {
        if !tokens.contains(required) {
            return Err(format!("dispatcher lost exact status call {required}"));
        }
    }
    for forbidden in [
        "state . cr =",
        "status :: compare_signed",
        "status :: compare_unsigned",
        "status :: select_relation",
        "status :: mtcrf",
    ] {
        if execute_raw.contains(forbidden) || execute_x31.contains(forbidden) {
            return Err(format!("dispatcher contains bypass fragment {forbidden}"));
        }
    }
    Ok(())
}

fn validate_decoder(source: &str) -> Result<(), String> {
    exact_function(source, "equal_line", EQUAL_LINE)?;
    exact_function(source, "decode", DECODE)
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
    let dead_decoder_anchor = replace_once(
        decoder,
        "let compare_immediate_unsigned = opcode(10);",
        "let compare_immediate_unsigned = if false { opcode(10) } else { u8::from(opcode_value == 10) };",
    );
    for hostile in [host_equality, forged_addc, dead_decoder_anchor] {
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
        "state.cr = status::write_cr_field(\n        state.cr,\n        field,\n        relation,\n        status::so(state.xer),\n        write_enable,\n    );",
        "let _ = (field, relation); state.cr = PowerPc601Simulator::new().get_state().cr;",
    );
    for hostile in [dead_anchor, post_overwrite, oracle] {
        syn::parse_file(&hostile).expect("hostile consumer is compile-valid Rust syntax");
        assert!(validate_consumers(&hostile).is_err());
    }
}

#[test]
fn hostile_ast_rejects_helper_replacement_and_caller_post_helper_overwrite() {
    let status = include_str!("../src/status.rs");
    let helper_replacement = replace_once(
        status,
        "let difference = alu::sub(left, right);\n    signed_relation(difference.zero, difference.negative, difference.overflow)",
        "let _dead_gate_anchor = alu::sub(left, right); match (left as i32).cmp(&(right as i32)) { std::cmp::Ordering::Less => RelationWires { lt: 1, gt: 0, eq: 0 }, std::cmp::Ordering::Greater => RelationWires { lt: 0, gt: 1, eq: 0 }, std::cmp::Ordering::Equal => RelationWires { lt: 0, gt: 0, eq: 1 } }",
    );
    syn::parse_file(&helper_replacement)
        .expect("helper-replacement fixture is compile-valid Rust syntax");
    assert!(validate_status(&helper_replacement).is_err());

    let cpu = include_str!("../src/cpu.rs");
    let caller_post_overwrite = replace_once(
        cpu,
        "0 | 32 => execute_compare_register(state, &decoded, xo10, ra, rb),",
        "0 | 32 => { let old_cr = state.cr; let mnemonic = execute_compare_register(state, &decoded, xo10, ra, rb); state.cr = old_cr; mnemonic },",
    );
    syn::parse_file(&caller_post_overwrite)
        .expect("caller-overwrite fixture is compile-valid Rust syntax");
    assert!(validate_consumers(&caller_post_overwrite).is_err());
}

#[test]
fn compile_checked_hostile_dispatchers_reject_nested_shadows_and_whole_state_overwrites() {
    let nested_shadow = r#"
#[derive(Clone, Copy)]
struct PowerPcState { cr: u32 }
fn execute_compare_register(_: &mut PowerPcState) -> &'static str { "CMP" }
fn execute_x31(state: &mut PowerPcState) -> &'static str {
    match 0_u32 {
        0 | 32 => {
            fn execute_compare_register(state: &mut PowerPcState) -> &'static str {
                state.cr = 0;
                "FORGED"
            }
            execute_compare_register(state)
        }
        _ => "OTHER",
    }
}
"#;
    assert_rustc_accepts(nested_shadow);
    assert!(reject_nested_helper_shadows(
        nested_shadow,
        "execute_x31",
        &["execute_compare_register"]
    )
    .is_err());

    let whole_state_overwrite = r#"
#[derive(Clone, Copy)]
struct PowerPcState { cr: u32 }
fn execute_compare_register(state: &mut PowerPcState) -> &'static str {
    state.cr = 1;
    "CMP"
}
fn execute_x31(state: &mut PowerPcState) -> &'static str {
    match 0_u32 {
        0 | 32 => {
            let before = *state;
            let mnemonic = execute_compare_register(state);
            *state = before;
            mnemonic
        }
        _ => "OTHER",
    }
}
"#;
    assert_rustc_accepts(whole_state_overwrite);
    assert!(reject_post_helper_state_writes(
        whole_state_overwrite,
        "execute_x31",
        &["execute_compare_register"]
    )
    .is_err());
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

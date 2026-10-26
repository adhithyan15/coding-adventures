use std::fs;
use std::path::PathBuf;
use std::process::Command;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::visit::Visit;

const ENGINE: &str = include_str!("../src/engine.rs");
const GATES: &str = include_str!("../src/gates.rs");
const LIB: &str = include_str!("../src/lib.rs");
const STATE: &str = include_str!("../src/state.rs");

// These fingerprints are over parsed token trees, not source bytes. Harmless
// formatting and comments disappear, while identifiers, punctuation,
// delimiters, literals, and complete control/data flow stay exact.
const ENGINE_TOKENS: u64 = 0xf1df_9761_2b39_65f6;
const GATES_TOKENS: u64 = 0xaeab_896f_d817_7402;
const LIB_TOKENS: u64 = 0x20c4_cc8d_1401_7460;
const STATE_TOKENS: u64 = 0x5433_6bcd_7c59_f1f7;

const THUMB_FUNCTION_TOKENS: &[(&str, u64)] = &[
    ("bx", 0xbec5_297d_40dd_5018),
    ("execute_thumb", 0x7a5f_8ac6_a898_32cc),
    ("execute_thumb_shift_add", 0xb4fd_e8e3_7259_a32f),
    ("execute_thumb_immediate", 0x8f0e_3f6c_2ada_8530),
    (
        "execute_thumb_data_or_register_transfer",
        0x392f_5ea2_de23_7a5c,
    ),
    ("execute_thumb_alu", 0xd5db_cf80_9cd1_2c3e),
    ("execute_thumb_high", 0x60c5_3c82_39b0_8aa5),
    ("execute_thumb_register_transfer", 0xc3ee_9be3_79c1_5840),
    ("execute_thumb_immediate_transfer", 0x9662_4fd9_5bb3_89d5),
    ("execute_thumb_sp_transfer", 0xb640_3b98_8f1a_5df7),
    ("execute_thumb_misc", 0xe65c_50fc_556a_2a00),
    ("execute_thumb_push_pop", 0xbc79_0716_f32e_2613),
    (
        "execute_thumb_multiple_or_conditional",
        0x5e8b_ac46_4593_1345,
    ),
    ("execute_thumb_branch_or_link", 0x1fa1_7dbe_e7b4_006c),
];

const ENGINE_FUNCTION_TOKENS: &[(&str, u64)] = &[
    ("condition_passed", 0x11b4_fdee_fae6_82fe),
    ("add_with_carry", 0x8369_a176_321a_3cb1),
    ("shifted", 0x08ca_0bcd_9536_80e3),
    ("execute_arm", 0x95d6_c1fd_21d8_d1e7),
    ("execute_data_processing", 0xaf8c_c292_6ea9_5275),
    ("set_nzcv", 0x6698_44d5_6cee_bece),
    ("read_register", 0xeabd_f22a_b9fc_4dd3),
    ("write_register", 0xe336_537c_57de_fe24),
    ("spsr", 0xaf8d_9584_0f4f_bd7f),
    ("set_spsr", 0x4656_d03d_b4fd_0d5e),
    ("read_mem", 0x3deb_b143_6cff_df20),
    ("write_mem", 0x9fce_d1bf_6473_efa1),
];

const GATE_FUNCTION_TOKENS: &[(&str, u64)] = &[
    ("masked_eq_32", 0xa595_ac2b_b09d_e845),
    ("add_32", 0x3f51_7e16_b87a_ae35),
    ("status_with_nz", 0x05cf_60ef_dca0_176e),
    ("arm_shift", 0xd45c_2e15_6f65_4573),
];

const STATE_FUNCTION_TOKENS: &[(&str, u64)] = &[
    ("clock_bit", 0x5878_89b0_80a4_78b9),
    ("clock_bits", 0xcad6_c6fc_2a73_a44b),
    ("write", 0x5318_fb0e_97ec_cef9),
    ("snapshot", 0x6f24_14ca_45e4_a312),
];

const LIB_FUNCTION_TOKENS: &[(&str, u64)] = &[
    ("reset", 0xf90f_81c3_e413_81df),
    ("restore", 0x67f6_d9c7_18b2_8311),
    ("load", 0x882d_02e0_c0c1_ecda),
    ("get_state", 0xd1e0_9435_4585_5fb7),
    ("engine", 0x7d42_545d_a4e9_3953),
    ("write_register", 0xf737_cd22_7566_4e88),
    ("write_memory", 0xc593_ce39_d01a_8be7),
    ("step", 0xd48f_4dfa_a87c_3db3),
    ("run", 0xdf45_641b_d065_7de8),
    ("execute", 0x8704_f455_0120_1b5e),
    ("clock_complete_state", 0x1a99_690c_f181_04fb),
    ("commit_transition", 0xb705_15c6_8347_d5a2),
];

const CONDITION: &str = r#"fn condition_passed(cpsr: u32, condition: u8) -> bool {
    gates::condition_holds(cpsr, condition) == 1
}"#;
const ADD: &str = r#"fn add_with_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));
    (result, carry == 1, overflow == 1)
}"#;
const CLOCK_BIT: &str = r#"pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}"#;
const MEMORY_WRITE: &str = r#"let old = self.q[address];
        let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
        let mut state: [FlipFlopState; 8] =
            std::array::from_fn(|bit| stable_state((old >> bit) & 1));
        register(&input, 0, &mut state);
        self.q[address] = register(&input, 1, &mut state)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));"#;

// These anchors span the reviewed source-to-sink boundary.  They deliberately
// lock the decoded family wires, operation wires, mux/control consumers,
// architectural read routes, and DFF/public commit routes separately: keeping
// a dead gate expression beside a host-authoritative result cannot satisfy the
// whole set.
const ARM_FAMILY_ROUTE: &str = r#"let family = gates::field(raw, 25, 3);
        if gates::selected(family, 0, 3) | gates::selected(family, 1, 3) == 1 {
            execute_data_processing(state, raw, pc)?
        } else if gates::selected(family, 2, 3) | gates::selected(family, 3, 3) == 1 {
            execute_single_transfer(state, raw, pc)?
        } else if gates::selected(family, 4, 3) == 1 {
            execute_block_transfer(state, raw, pc)?
        } else if gates::selected(family, 5, 3) == 1 {
            execute_branch(state, raw, pc)?"#;

fn delimiter_name(delimiter: Delimiter) -> &'static str {
    match delimiter {
        Delimiter::Parenthesis => "paren",
        Delimiter::Brace => "brace",
        Delimiter::Bracket => "bracket",
        Delimiter::None => "none",
    }
}

fn flatten_tree(tree: TokenTree, tokens: &mut Vec<String>) {
    match tree {
        TokenTree::Group(group) => {
            let delimiter = delimiter_name(group.delimiter());
            tokens.push(format!("open:{delimiter}"));
            flatten_stream(group.stream(), tokens);
            tokens.push(format!("close:{delimiter}"));
        }
        TokenTree::Ident(ident) => tokens.push(format!("ident:{ident}")),
        TokenTree::Punct(punct) => {
            tokens.push(format!("punct:{}:{:?}", punct.as_char(), punct.spacing()))
        }
        TokenTree::Literal(literal) => tokens.push(format!("literal:{literal}")),
    }
}

fn flatten_stream(stream: TokenStream, tokens: &mut Vec<String>) {
    for tree in stream {
        flatten_tree(tree, tokens);
    }
}

fn parsed_tokens(source: &str) -> Result<Vec<String>, String> {
    let stream = source
        .parse::<TokenStream>()
        .map_err(|error| error.to_string())?;
    let mut tokens = Vec::new();
    flatten_stream(stream, &mut tokens);
    Ok(tokens)
}

fn fingerprint(tokens: &[String]) -> u64 {
    tokens
        .iter()
        .flat_map(|token| token.bytes().chain(std::iter::once(0xff)))
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn token_fingerprint(source: &str) -> Result<u64, String> {
    parsed_tokens(source).map(|tokens| fingerprint(&tokens))
}

fn find_function_fingerprint(stream: TokenStream, name: &str) -> Option<u64> {
    let trees: Vec<_> = stream.into_iter().collect();
    for index in 0..trees.len().saturating_sub(1) {
        let TokenTree::Ident(keyword) = &trees[index] else {
            continue;
        };
        let TokenTree::Ident(candidate) = &trees[index + 1] else {
            continue;
        };
        if keyword != "fn" || candidate != name {
            continue;
        }
        let mut function = Vec::new();
        for tree in trees[index..].iter().cloned() {
            let body =
                matches!(&tree, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace);
            flatten_tree(tree, &mut function);
            if body {
                return Some(fingerprint(&function));
            }
        }
    }
    for tree in trees {
        if let TokenTree::Group(group) = tree {
            if let Some(found) = find_function_fingerprint(group.stream(), name) {
                return Some(found);
            }
        }
    }
    None
}

fn function_fingerprint(source: &str, name: &str) -> Result<u64, String> {
    let stream = source
        .parse::<TokenStream>()
        .map_err(|error| error.to_string())?;
    find_function_fingerprint(stream, name).ok_or_else(|| format!("missing parsed function {name}"))
}

fn token_sequence_count(source: &str, needle: &str) -> Result<usize, String> {
    let source = parsed_tokens(source)?;
    let needle = parsed_tokens(needle)?;
    if needle.is_empty() {
        return Err("empty token lock".into());
    }
    Ok(source
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count())
}

fn exact_token_count(source: &str, needle: &str, expected: usize) -> Result<(), String> {
    let actual = token_sequence_count(source, needle)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected} occurrences, found {actual}"))
    }
}

fn validate_function_locks(label: &str, source: &str, locks: &[(&str, u64)]) -> Result<(), String> {
    let mut mismatches = Vec::new();
    for (name, expected) in locks {
        let actual = function_fingerprint(source, name)?;
        if actual != *expected {
            mismatches.push(format!(
                "{name}: expected {expected:#018x}, found {actual:#018x}"
            ));
        }
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{label} exact parsed-body lock changed: {}",
            mismatches.join(", ")
        ))
    }
}

struct SelectedCallCounter<'a> {
    argument: &'a str,
    count: usize,
}

impl<'ast> Visit<'ast> for SelectedCallCounter<'_> {
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        let is_selected = matches!(call.func.as_ref(), syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "selected"));
        let has_argument = matches!(call.args.first(), Some(syn::Expr::Path(path)) if path.path.is_ident(self.argument));
        if is_selected && has_argument {
            self.count += 1;
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn selected_call_count(source: &str, argument: &str) -> Result<usize, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut counter = SelectedCallCounter { argument, count: 0 };
    counter.visit_file(&syntax);
    Ok(counter.count)
}

#[derive(Default)]
struct ForbiddenPathVisitor {
    paths: Vec<String>,
}

impl<'ast> Visit<'ast> for ForbiddenPathVisitor {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(first) = path.segments.first() {
            if first.ident == "arm7tdmi_simulator" || first.ident == "oracle" {
                self.paths.push(
                    path.segments
                        .iter()
                        .map(|segment| segment.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::"),
                );
            }
        }
        syn::visit::visit_path(self, path);
    }
}

fn forbidden_paths(source: &str) -> Result<Vec<String>, String> {
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut visitor = ForbiddenPathVisitor::default();
    visitor.visit_file(&syntax);
    Ok(visitor.paths)
}

fn validate(engine: &str, gates: &str, lib: &str, state: &str) -> Result<(), String> {
    syn::parse_file(engine).map_err(|error| error.to_string())?;
    syn::parse_file(gates).map_err(|error| error.to_string())?;
    syn::parse_file(lib).map_err(|error| error.to_string())?;
    syn::parse_file(state).map_err(|error| error.to_string())?;
    validate_function_locks("engine authority", engine, ENGINE_FUNCTION_TOKENS)?;
    validate_function_locks("complete Thumb", engine, THUMB_FUNCTION_TOKENS)?;
    for (argument, expected, name) in [
        ("opcode", 31, "complete operation-wire route"),
        ("family", 14, "complete family-wire route"),
        ("operation", 14, "complete Thumb suboperation-wire route"),
    ] {
        let actual = selected_call_count(engine, argument)?;
        if actual != expected {
            return Err(format!("{name}: expected {expected} calls, found {actual}"));
        }
    }
    exact_token_count(engine, "Arm7TdmiError::PrivilegedInstruction", 0)
        .map_err(|error| format!("unreachable privileged fault: {error}"))?;
    exact_token_count(engine, "Arm7TdmiError::MisalignedFetch", 1)
        .map_err(|error| format!("private fetch-only alignment fault: {error}"))?;
    validate_function_locks("gate datapath", gates, GATE_FUNCTION_TOKENS)?;
    validate_function_locks("stable-Q storage", state, STATE_FUNCTION_TOKENS)?;
    validate_function_locks("public lifecycle", lib, LIB_FUNCTION_TOKENS)?;
    for source in [engine, gates, lib, state] {
        let forbidden = forbidden_paths(source)?;
        if !forbidden.is_empty() {
            return Err(format!(
                "production source references forbidden oracle paths: {}",
                forbidden.join(", ")
            ));
        }
    }
    for (name, source, expected) in [
        ("engine", engine, ENGINE_TOKENS),
        ("gates", gates, GATES_TOKENS),
        ("lib", lib, LIB_TOKENS),
        ("state", state, STATE_TOKENS),
    ] {
        let actual = token_fingerprint(source)?;
        if actual != expected {
            return Err(format!(
                "{name} exact parsed-token lock changed: expected {expected:#018x}, found {actual:#018x}"
            ));
        }
    }
    Ok(())
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "hostile mutation anchor");
    source.replacen(old, new, 1)
}

fn compile_variant(engine: &str, gates: &str, lib: &str, state: &str) {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nonce = format!("{}-{:?}", std::process::id(), std::thread::current().id())
        .replace(['(', ')', ' '], "");
    let root = std::env::temp_dir().join(format!("arm7tdmi-gate-hostile-{nonce}"));
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    let packages = package.parent().unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname='arm7tdmi-gate-hostile'\nversion='0.0.0'\nedition='2021'\n[lib]\npath='src/lib.rs'\n[dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\n",
            packages.join("arithmetic").display(),
            packages.join("logic-gates").display()
        ),
    )
    .unwrap();
    fs::write(src.join("engine.rs"), engine).unwrap();
    fs::write(src.join("lib.rs"), lib).unwrap();
    fs::write(src.join("state.rs"), state).unwrap();
    fs::write(src.join("gates.rs"), gates).unwrap();
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            package.parent().unwrap().join("target/hostile-check"),
        )
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "hostile fixture must compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_rejected_for(
    engine: &str,
    gates: &str,
    lib: &str,
    state: &str,
    expected_markers: &[&str],
) {
    let error = validate(engine, gates, lib, state).expect_err("hostile source must be rejected");
    for marker in expected_markers {
        assert!(
            error.contains(marker),
            "hostile source must be rejected through {marker:?}, got: {error}"
        );
    }
}

fn compile_and_reject_engine(engine: &str, expected_markers: &[&str]) {
    compile_variant(engine, GATES, LIB, STATE);
    assert_rejected_for(engine, GATES, LIB, STATE, expected_markers);
}

#[test]
fn reviewed_source_to_sink_authority_chains_are_exact() {
    validate(ENGINE, GATES, LIB, STATE).unwrap();
}

#[test]
fn parsed_locks_ignore_layout_and_lookalikes_but_reject_real_semantic_changes() {
    let formatted = replace_once(
        ENGINE,
        "use core::fmt;",
        "/* gates::selected(opcode, 0, 4); oracle::execute(); \"string\" 'x' */\nuse  core::fmt ;",
    );
    validate(&formatted, GATES, LIB, STATE)
        .expect("comments and harmless layout cannot change parsed authority");

    let host = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "condition == 14 || (cpsr & (1 << 30) != 0 && condition == 0)",
    );
    let hostile = replace_once(ENGINE, CONDITION, &host);
    let hostile = format!(
        "{hostile}\nconst CONDITION_ROUTE_LOOKALIKE: &str = {CONDITION:?};\nconst CHAR_LOOKALIKES: [char; 4] = ['g', ':', '(', ')'];"
    );
    compile_variant(&hostile, GATES, LIB, STATE);
    let error = validate(&hostile, GATES, LIB, STATE).unwrap_err();
    assert!(
        error.contains("engine authority exact parsed-body lock changed")
            && error.contains("condition_passed"),
        "string/char lookalikes must not satisfy the semantic lock: {error}"
    );
}

#[test]
fn compile_valid_hostile_condition_and_helper_bypasses_are_rejected() {
    let host = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "condition == 14 || (cpsr & (1 << 30) != 0 && condition == 0)",
    );
    let mutated = replace_once(ENGINE, CONDITION, &host);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "condition_passed",
        ],
    );

    let indirect = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "host_condition(cpsr, condition)",
    );
    let indirect = format!("{indirect}\nfn host_condition(cpsr: u32, condition: u8) -> bool {{ condition == 14 || (condition == 0 && cpsr & (1 << 30) != 0) }}");
    let mutated = replace_once(ENGINE, CONDITION, &indirect);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "condition_passed",
        ],
    );
}

#[test]
fn compile_valid_hostile_arithmetic_decode_and_oracle_bypasses_are_rejected() {
    let host_add = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let carry = u32::from(carry);\n    (left.wrapping_add(right).wrapping_add(carry), false, false)",
    );
    let mutated = replace_once(ENGINE, ADD, &host_add);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "add_with_carry",
        ],
    );

    let host_decode = replace_once(
        ARM_FAMILY_ROUTE,
        "if gates::selected(family, 0, 3) | gates::selected(family, 1, 3) == 1",
        "if family == 0 || family == 1",
    );
    let mutated = replace_once(ENGINE, ARM_FAMILY_ROUTE, &host_decode);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "execute_arm",
        ],
    );

    let oracle_condition = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "oracle::condition_holds(cpsr, condition)",
    );
    let oracle_condition = format!(
        "{oracle_condition}\nmod oracle {{ pub(super) fn condition_holds(cpsr: u32, condition: u8) -> bool {{ condition == 14 || (condition == 0 && cpsr & (1 << 30) != 0) }} }}"
    );
    let mutated = replace_once(ENGINE, CONDITION, &oracle_condition);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "condition_passed",
        ],
    );
}

#[test]
fn compile_valid_hostile_complete_thumb_dispatch_surface_is_rejected() {
    // Family plus immediate/ALU/high/register-transfer operation selection.
    let family = replace_once(
        ENGINE,
        "let mnemonic = if gates::selected(family, 0, 3) == 1 {",
        "let mnemonic = if family == 0 {",
    );
    let immediate = replace_once(
        &family,
        "let immediate = (raw & 0xff) as u32;\n    let left = read_register(state, rd)?;\n    if gates::selected(operation, 0, 2) == 1 {",
        "let immediate = (raw & 0xff) as u32;\n    let left = read_register(state, rd)?;\n    if operation == 0 {",
    );
    let alu = replace_once(
        &immediate,
        "let carry = state.cpsr & CPSR_C != 0;\n    if gates::selected(opcode, 0x0, 4) == 1 {",
        "let carry = state.cpsr & CPSR_C != 0;\n    if opcode == 0 {",
    );
    let high = replace_once(
        &alu,
        "if gates::selected(operation, 0, 2) == 1 {\n        let result = gates::add_32(left, right, 0).0;",
        "if operation == 0 {\n        let result = gates::add_32(left, right, 0).0;",
    );
    let register_transfer = replace_once(
        &high,
        "if gates::selected(operation, 0, 3) == 1 {",
        "if operation == 0 {",
    );
    compile_and_reject_engine(
        &register_transfer,
        &[
            "complete Thumb exact parsed-body lock changed",
            "execute_thumb",
            "execute_thumb_immediate",
            "execute_thumb_alu",
            "execute_thumb_high",
            "execute_thumb_register_transfer",
        ],
    );

    // Immediate/SP/misc fixed-bit and transfer-direction selectors.
    let immediate_transfer = replace_once(
        ENGINE,
        "let load = gates::selected(gates::field(word, 11, 1), 1, 1);\n    let halfword_line = u8::from(halfword);",
        "let load = u8::from(raw & (1 << 11) != 0);\n    let halfword_line = u8::from(halfword);",
    );
    let sp_transfer = replace_once(
        &immediate_transfer,
        "let load = gates::selected(gates::field(u32::from(raw), 11, 1), 1, 1);",
        "let load = u8::from(raw & (1 << 11) != 0);",
    );
    let misc = replace_once(
        &sp_transfer,
        "let use_sp = gates::selected(gates::field(u32::from(raw), 11, 1), 1, 1);",
        "let use_sp = u8::from(raw & (1 << 11) != 0);",
    );
    compile_and_reject_engine(
        &misc,
        &[
            "complete Thumb exact parsed-body lock changed",
            "execute_thumb_immediate_transfer",
            "execute_thumb_sp_transfer",
            "execute_thumb_misc",
        ],
    );

    // Stack/multiple register write enables and their direction bits.
    let stack = replace_once(
        ENGINE,
        "let pop = gates::selected(gates::field(word, 11, 1), 1, 1);",
        "let pop = u8::from(raw & (1 << 11) != 0);",
    );
    let multiple = replace_once(
        &stack,
        "if gates::masked_eq_16(raw, 0xf000, 0xc000) == 1 {\n        let word = u32::from(raw);\n        let load = gates::selected(gates::field(word, 11, 1), 1, 1);",
        "if gates::masked_eq_16(raw, 0xf000, 0xc000) == 1 {\n        let word = u32::from(raw);\n        let load = u8::from(raw & (1 << 11) != 0);",
    );
    let list_write_enable = replace_once(
        &multiple,
        "if pop == 1 {\n        for register in 0..8 {\n            let selected_register = gates::selected(gates::field(list, register, 1), 1, 1);",
        "if pop == 1 {\n        for register in 0..8 {\n            let selected_register = u8::from(list & (1 << register) != 0);",
    );
    compile_and_reject_engine(
        &list_write_enable,
        &[
            "complete Thumb exact parsed-body lock changed",
            "execute_thumb_push_pop",
            "execute_thumb_multiple_or_conditional",
        ],
    );

    // High-register PC write enable, conditional fixed bits, and trace route.
    let high_write = replace_once(
        ENGINE,
        "let result = gates::add_32(left, right, 0).0;\n        if destination_is_pc == 1 {",
        "let result = gates::add_32(left, right, 0).0;\n        if rd == 15 {",
    );
    let bx_route = replace_once(
        &high_write,
        "let unpredictable_target = gates::selected(low_bits, 2, 2);\n    let thumb_selected = gates::selected(gates::field(target, 0, 1), 1, 1);",
        "let unpredictable_target = u8::from(target & 3 == 2);\n    let thumb_selected = u8::from(target & 1 != 0);",
    );
    let conditional = replace_once(
        &bx_route,
        "if software_interrupt == 1 {",
        "if condition == 0xf {",
    );
    let mnemonic = replace_once(
        &conditional,
        "Ok((mnemonic, executed == 1))",
        "Ok((mnemonic, mnemonic != \"CONDITION_FAILED\"))",
    );
    compile_and_reject_engine(
        &mnemonic,
        &[
            "complete Thumb exact parsed-body lock changed",
            "bx",
            "execute_thumb",
            "execute_thumb_high",
            "execute_thumb_multiple_or_conditional",
        ],
    );
}

#[test]
fn compile_valid_hostile_dead_anchor_and_post_gate_overwrite_are_rejected() {
    let dead_anchor = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let _gate_anchor = gates::add_32(left, right, u8::from(carry));\n    (left.wrapping_add(right).wrapping_add(u32::from(carry)), false, false)",
    );
    let mutated = replace_once(ENGINE, ADD, &dead_anchor);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "add_with_carry",
        ],
    );

    let post_gate = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let (mut result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    result = left.wrapping_add(right).wrapping_add(u32::from(carry));\n    (result, carry == 1, overflow == 1)",
    );
    let mutated = replace_once(ENGINE, ADD, &post_gate);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert_rejected_for(
        &mutated,
        GATES,
        LIB,
        STATE,
        &[
            "engine authority exact parsed-body lock changed",
            "add_with_carry",
        ],
    );
}

#[test]
fn compile_valid_hostile_direct_q_and_post_commit_bypasses_are_rejected() {
    let direct = replace_once(CLOCK_BIT, "register(&input, 0, &mut state);\n    *q = register(&input, 1, &mut state)[0];", "register(&input, 0, &mut state);\n    let _anchor = register(&input, 1, &mut state)[0];\n    *q = u8::from(value);");
    let mutated_state = replace_once(STATE, CLOCK_BIT, &direct);
    compile_variant(ENGINE, GATES, LIB, &mutated_state);
    assert_rejected_for(
        ENGINE,
        GATES,
        LIB,
        &mutated_state,
        &[
            "stable-Q storage exact parsed-body lock changed",
            "clock_bit",
        ],
    );

    let direct_memory = replace_once(MEMORY_WRITE, "self.q[address] = register(&input, 1, &mut state)\n            .iter()\n            .enumerate()\n            .fold(0, |byte, (bit, q)| byte | (q << bit));", "let _anchor = register(&input, 1, &mut state);\n        self.q[address] = value;");
    let mutated_state = replace_once(STATE, MEMORY_WRITE, &direct_memory);
    compile_variant(ENGINE, GATES, LIB, &mutated_state);
    assert_rejected_for(
        ENGINE,
        GATES,
        LIB,
        &mutated_state,
        &["stable-Q storage exact parsed-body lock changed", "write"],
    );

    let mutated_lib = replace_once(LIB, "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n    }", "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n        self.halted = 0;\n    }");
    compile_variant(ENGINE, GATES, &mutated_lib, STATE);
    assert_rejected_for(
        ENGINE,
        GATES,
        &mutated_lib,
        STATE,
        &[
            "public lifecycle exact parsed-body lock changed",
            "clock_complete_state",
        ],
    );
}

use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, ExprAssign, ExprBinary, ExprCall, ExprIf, ExprMatch, ExprMethodCall, File, Item,
    ItemFn, Lit, PatIdent,
};

const LIB: &str = include_str!("../src/lib.rs");
const ENGINE: &str = include_str!("../src/engine.rs");
const GATES: &str = include_str!("../src/gates.rs");
const STATE: &str = include_str!("../src/state.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");
const THIS_TEST: &str = include_str!("gate_integrity.rs");
const COMMIT_TRANSITION_LOCK: u64 = 0x2ee8_e471_8637_5eea;
const SELECT_BIT_32_LOCK: u64 = 0x758a_723d_49f3_095e;
const RESTORE_REGISTERS_AND_CONTROL_LOCK: u64 = 0xfd9a_7454_2eb8_48be;
const RESTORE_VALIDATED_LOCK: u64 = 0xe4ae_655f_7787_c9d2;
const CLOCK_BITS_LOCK: u64 = 0x6436_6416_c7c9_28fe;
const CLOCK_BIT_LOCK: u64 = 0x5cd8_5142_e647_7ac1;
const MEMORY_WRITE_LOCK: u64 = 0x9104_bd6e_d0c2_6446;

// These are deliberately compiled as part of this integration test. They
// model the precise bypass that a required-call-only audit used to miss: the
// DFF network is evaluated, then a direct assignment replaces its stable Q.
#[allow(dead_code)]
mod compile_valid_hostile_fixtures {
    use logic_gates::sequential::{register, FlipFlopState};

    fn stable_state(q: u8) -> FlipFlopState {
        FlipFlopState {
            master_q: q,
            master_q_bar: q ^ 1,
            slave_q: q,
            slave_q_bar: q ^ 1,
        }
    }

    pub fn hostile_clock_bits_bypass<const WIDTH: usize>(q: &mut [u8; WIDTH], value: u64) {
        let input: [u8; WIDTH] = std::array::from_fn(|bit| ((value >> bit) & 1) as u8);
        let mut state: [FlipFlopState; WIDTH] = std::array::from_fn(|bit| stable_state(q[bit]));
        register(&input, 0, &mut state);
        q.copy_from_slice(&register(&input, 1, &mut state));
        q.copy_from_slice(&input);
    }

    pub fn hostile_clock_bit_bypass(q: &mut u8, value: bool) {
        let input = [u8::from(value)];
        let mut state = [stable_state(*q)];
        register(&input, 0, &mut state);
        *q = register(&input, 1, &mut state)[0];
        *q = input[0];
    }

    pub struct HostileMemory {
        q: Vec<u8>,
    }

    impl HostileMemory {
        pub fn hostile_memory_write_bypass(&mut self, address: usize, value: u8) {
            let old = self.q[address];
            let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
            let mut state: [FlipFlopState; 8] =
                std::array::from_fn(|bit| stable_state((old >> bit) & 1));
            register(&input, 0, &mut state);
            self.q[address] = register(&input, 1, &mut state)
                .iter()
                .enumerate()
                .fold(0, |byte, (bit, q)| byte | (q << bit));
            self.q[address] = value;
        }
    }
}

#[derive(Default)]
struct BodyFacts {
    calls: Vec<String>,
    binary: BTreeSet<&'static str>,
    paths: BTreeSet<String>,
    local_names: BTreeSet<String>,
    dead_control: bool,
    match_count: usize,
}

impl<'ast> Visit<'ast> for BodyFacts {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let name = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            self.calls.push(name);
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        self.calls.push(node.method.to_string());
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        let name = match node.op {
            BinOp::Add(_) => "add",
            BinOp::Sub(_) => "sub",
            BinOp::Mul(_) => "mul",
            BinOp::Shl(_) => "shl",
            BinOp::Shr(_) => "shr",
            BinOp::And(_) => "and",
            BinOp::Or(_) => "or",
            _ => "other",
        };
        self.binary.insert(name);
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast ExprIf) {
        if matches!(&*node.cond, Expr::Lit(literal) if matches!(literal.lit, Lit::Bool(ref value) if !value.value))
        {
            self.dead_control = true;
        }
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        self.match_count += 1;
        visit::visit_expr_match(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.local_names.insert(node.sig.ident.to_string());
        visit::visit_item_fn(self, node);
    }

    fn visit_pat_ident(&mut self, node: &'ast PatIdent) {
        self.local_names.insert(node.ident.to_string());
        visit::visit_pat_ident(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.paths.insert(
            node.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_path(self, node);
    }
}

fn parse(source: &str) -> Result<File, String> {
    syn::parse_file(source).map_err(|error| error.to_string())
}

#[derive(Default)]
struct FunctionBodies<'a> {
    wanted: &'a str,
    matches: Vec<syn::Block>,
}

impl<'ast> Visit<'ast> for FunctionBodies<'_> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if node.sig.ident == self.wanted {
            self.matches.push((*node.block).clone());
        }
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if node.sig.ident == self.wanted {
            self.matches.push(node.block.clone());
        }
        visit::visit_impl_item_fn(self, node);
    }
}

fn function_body(source: &str, function: &str) -> Result<syn::Block, String> {
    let file = parse(source)?;
    let mut bodies = FunctionBodies {
        wanted: function,
        ..FunctionBodies::default()
    };
    bodies.visit_file(&file);
    match bodies.matches.len() {
        1 => Ok(bodies.matches.remove(0)),
        count => Err(format!("expected one {function} body, found {count}")),
    }
}

fn audit_function(
    source: &str,
    function: &str,
    required_calls: &[&str],
    forbidden_binary: &[&str],
) -> Result<(), String> {
    let body = function_body(source, function)?;
    let mut facts = BodyFacts::default();
    facts.visit_block(&body);
    if facts.dead_control {
        return Err(format!("{function} contains dead control flow"));
    }
    for required in required_calls {
        let terminal = required.rsplit("::").next().unwrap();
        if facts.local_names.contains(terminal) {
            return Err(format!("{function} shadows required gate {terminal}"));
        }
        if !facts
            .calls
            .iter()
            .any(|call| call == required || call.ends_with(&format!("::{required}")))
        {
            return Err(format!("{function} does not call {required}"));
        }
    }
    for forbidden in forbidden_binary {
        if facts.binary.contains(forbidden) {
            return Err(format!("{function} contains host {forbidden}"));
        }
    }
    Ok(())
}

fn assert_no_host_match(source: &str, function: &str) -> Result<(), String> {
    let body = function_body(source, function)?;
    let mut facts = BodyFacts::default();
    facts.visit_block(&body);
    if facts.match_count == 0 {
        Ok(())
    } else {
        Err(format!(
            "{function} contains {} host match expression(s)",
            facts.match_count
        ))
    }
}

fn function_fingerprint(source: &str, function: &str) -> Result<u64, String> {
    let body = function_body(source, function)?;
    let normalized = body.to_token_stream().to_string();
    Ok(normalized
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        }))
}

fn assert_body_lock(source: &str, function: &str, expected: u64) -> Result<(), String> {
    let actual = function_fingerprint(source, function)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{function} body changed: expected {expected:#018x}, found {actual:#018x}"
        ))
    }
}

fn replace_once(source: &str, anchor: &str, replacement: &str) -> String {
    assert_eq!(
        source.matches(anchor).count(),
        1,
        "hostile fixture anchor must identify exactly one production fragment"
    );
    let mutated = source.replacen(anchor, replacement, 1);
    parse(&mutated).expect("anchored hostile fixture must remain valid Rust syntax");
    mutated
}

#[derive(Default)]
struct StateWrites(bool);

impl<'ast> Visit<'ast> for StateWrites {
    fn visit_expr_assign(&mut self, node: &'ast ExprAssign) {
        let left = node.left.to_token_stream().to_string().replace(' ', "");
        if left == "state" || left.starts_with("state.") || left.starts_with("state[") {
            self.0 = true;
        }
        visit::visit_expr_assign(self, node);
    }
}

fn assert_no_state_write_after_call(
    source: &str,
    function: &str,
    terminal_call: &str,
) -> Result<(), String> {
    let body = function_body(source, function)?;
    let mut dispatched = false;
    for statement in body.stmts {
        let mut facts = BodyFacts::default();
        facts.visit_stmt(&statement);
        if dispatched {
            let mut writes = StateWrites::default();
            writes.visit_stmt(&statement);
            if writes.0 {
                return Err(format!("{function} mutates state after {terminal_call}"));
            }
        }
        dispatched |= facts
            .calls
            .iter()
            .any(|call| call == terminal_call || call.ends_with(&format!("::{terminal_call}")));
    }
    if !dispatched {
        return Err(format!("{function} never calls {terminal_call}"));
    }
    Ok(())
}

fn assert_external_modules(source: &str) -> Result<(), String> {
    let file = parse(source)?;
    let expected = BTreeMap::from([
        ("engine".to_owned(), false),
        ("gates".to_owned(), true),
        ("state".to_owned(), false),
    ]);
    let mut actual = BTreeMap::new();
    for item in file.items {
        if let Item::Mod(module) = item {
            if module.ident == "allocation_regressions"
                && module.content.is_some()
                && module.attrs.iter().any(|attribute| {
                    attribute.path().is_ident("cfg")
                        && attribute.meta.to_token_stream().to_string() == "cfg (test)"
                })
            {
                continue;
            }
            if module.content.is_some() || !module.attrs.is_empty() {
                return Err(format!(
                    "{} must be an un-attributed external module",
                    module.ident
                ));
            }
            actual.insert(
                module.ident.to_string(),
                matches!(module.vis, syn::Visibility::Public(_)),
            );
        }
    }
    if actual != expected {
        return Err(format!("unexpected module routing: {actual:?}"));
    }
    Ok(())
}

fn assert_storage_inventory(source: &str) -> Result<(), String> {
    let file = parse(source)?;
    let Item::Struct(storage) = file
        .items
        .iter()
        .find(|item| matches!(item, Item::Struct(item) if item.ident == "CortexM23GateLevel"))
        .ok_or("missing storage owner")?
    else {
        unreachable!()
    };
    let actual = storage
        .fields
        .iter()
        .map(|field| {
            (
                field.ident.as_ref().expect("named field").to_string(),
                field.ty.to_token_stream().to_string().replace(' ', ""),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = BTreeMap::from([
        ("memory".into(), "DffMemory".into()),
        ("registers".into(), "[[u8;32];13]".into()),
        ("msp_non_secure".into(), "[u8;32]".into()),
        ("psp_non_secure".into(), "[u8;32]".into()),
        ("lr".into(), "[u8;32]".into()),
        ("pc".into(), "[u8;32]".into()),
        ("apsr".into(), "[u8;4]".into()),
        ("ipsr".into(), "[u8;6]".into()),
        ("primask_non_secure".into(), "u8".into()),
        ("control_non_secure".into(), "[u8;2]".into()),
        ("handler_mode".into(), "u8".into()),
        ("halted".into(), "u8".into()),
        ("sleeping".into(), "u8".into()),
        ("event_registered".into(), "u8".into()),
        ("loaded_origin".into(), "[u8;32]".into()),
        ("loaded_len".into(), "[u8;17]".into()),
    ]);
    if actual != expected {
        return Err(format!("storage inventory changed: {actual:?}"));
    }
    let independently_summed = 65_536 * 8 + 13 * 32 + 4 * 32 + 4 + 6 + 1 + 2 + 1 + 3 + 32 + 17;
    if independently_summed != cortex_m23_gatelevel::FLIP_FLOP_COUNT {
        return Err("concrete storage widths do not match the published count".into());
    }
    Ok(())
}

fn assert_memory_has_only_stable_q(source: &str) -> Result<(), String> {
    let file = parse(source)?;
    let Item::Struct(storage) = file
        .items
        .iter()
        .find(|item| matches!(item, Item::Struct(item) if item.ident == "DffMemory"))
        .ok_or("missing DffMemory")?
    else {
        unreachable!()
    };
    let fields = storage
        .fields
        .iter()
        .map(|field| {
            (
                field.ident.as_ref().expect("named field").to_string(),
                field.ty.to_token_stream().to_string().replace(' ', ""),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if fields != BTreeMap::from([("q".to_owned(), "Vec<u8>".to_owned())]) {
        return Err(format!(
            "memory contains a non-Q authority or mirror: {fields:?}"
        ));
    }
    Ok(())
}

#[test]
fn production_routing_storage_and_gate_authority_are_structural() {
    assert_external_modules(LIB).unwrap();
    assert_storage_inventory(LIB).unwrap();
    assert_memory_has_only_stable_q(STATE).unwrap();
    let production_dependencies = MANIFEST
        .split("[dependencies]")
        .nth(1)
        .expect("dependency section")
        .split("[dev-dependencies]")
        .next()
        .expect("production dependencies");
    assert!(!production_dependencies.contains("simulator"));

    for source in [LIB, ENGINE, GATES, STATE] {
        let file = parse(source).unwrap();
        let mut facts = BodyFacts::default();
        facts.visit_file(&file);
        assert!(facts.paths.iter().all(|path| {
            !path.contains("cortex_m0_simulator")
                && !path.contains("cortex_m23_simulator")
                && !path.contains("cortex_m0_gatelevel")
        }));
    }

    audit_function(
        GATES,
        "add_32",
        &["ripple_carry_adder_with_carry", "xor_gate"],
        &["add"],
    )
    .unwrap();
    audit_function(GATES, "sub_32", &["add_32", "not_gate"], &["sub"]).unwrap();
    audit_function(
        GATES,
        "select_bit_32",
        &["field", "is_zero_32", "selected", "and_gate", "or_gate"],
        &["and", "or"],
    )
    .unwrap();
    audit_function(
        GATES,
        "multiply_32",
        &["select_32", "shift_left_32", "add_32"],
        &["mul"],
    )
    .unwrap();
    audit_function(
        GATES,
        "shift_immediate_network",
        &[
            "selected",
            "select_bit_32",
            "select_one_hot_32",
            "select_one_hot_bit",
        ],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        GATES,
        "shift_register_network",
        &[
            "selected",
            "less_than_unsigned_32",
            "equal_32",
            "select_bit_32",
            "select_one_hot_32",
            "select_one_hot_bit",
        ],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    assert_no_host_match(GATES, "shift_immediate_network").unwrap();
    assert_no_host_match(GATES, "shift_register_network").unwrap();
    audit_function(
        ENGINE,
        "classify_at",
        &["line16", "and_gate", "or_gate"],
        &[],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "condition_holds",
        &[
            "and_gate",
            "or_gate",
            "xor_gate",
            "not_gate",
            "selected",
            "select_one_hot_bit",
        ],
        &["and", "or"],
    )
    .unwrap();
    assert_no_host_match(ENGINE, "condition_holds").unwrap();
    audit_function(
        ENGINE,
        "shift_immediate",
        &["shift_immediate_network"],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "shift_register",
        &["shift_register_network"],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "add_or_subtract",
        &["add_32", "sub_32", "select_32", "select_one_hot_bit"],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "execute_shift_add_sub",
        &["line16", "any_line", "shift_immediate", "add_or_subtract"],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    for function in [
        "shift_immediate",
        "shift_register",
        "add_or_subtract",
        "execute_shift_add_sub",
    ] {
        assert_no_host_match(ENGINE, function).unwrap();
    }
    audit_function(
        ENGINE,
        "execute_decoded",
        &[
            "execute_shift_add_sub",
            "execute_immediate",
            "execute_data_processing",
            "execute_special",
            "execute_literal_load",
            "execute_register_memory",
            "execute_immediate_memory",
            "execute_stack_memory",
            "execute_address_generation",
            "execute_miscellaneous",
            "execute_multiple_memory",
            "execute_conditional",
            "execute_unconditional",
            "execute_bl",
            "execute_barrier",
            "execute_special_register",
        ],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "execute_barrier",
        &["line16"],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "read_special",
        &[
            "special_lines",
            "bit_or_32",
            "select_one_hot_32",
            "any_line",
        ],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "execute_special_register",
        &[
            "special_lines",
            "read_special",
            "less_or_equal_unsigned_32",
            "any_line",
            "select_32",
        ],
        &["add", "sub", "mul", "shl", "shr", "and", "or"],
    )
    .unwrap();
    for function in [
        "execute_barrier",
        "read_special",
        "execute_special_register",
    ] {
        assert_no_host_match(ENGINE, function).unwrap();
    }
    audit_function(
        ENGINE,
        "checked_range",
        &["aligned_32", "less_or_equal_unsigned_32"],
        &[],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "fetch16",
        &["aligned_32", "sub_32", "less_or_equal_unsigned_32"],
        &[],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "execute_conditional",
        &["condition_holds", "add_32"],
        &["add", "sub"],
    )
    .unwrap();
    audit_function(
        ENGINE,
        "step_state",
        &["execute_decoded", "add_32"],
        &["add", "sub", "mul", "shl", "shr"],
    )
    .unwrap();
    assert_no_state_write_after_call(ENGINE, "step_state", "execute_decoded").unwrap();
    audit_function(
        LIB,
        "commit_transition",
        &[
            "read",
            "byte_changed",
            "write",
            "restore_registers_and_control",
        ],
        &["and", "or"],
    )
    .unwrap();
    audit_function(STATE, "clock_bits", &["register"], &[]).unwrap();
    audit_function(STATE, "clock_bit", &["register"], &[]).unwrap();
    audit_function(STATE, "write", &["register"], &[]).unwrap();
}

#[test]
fn critical_decode_to_commit_bodies_are_exactly_locked() {
    let locks = [
        (ENGINE, "execute_decoded", 0xeb7c_dd02_569f_e628_u64),
        (ENGINE, "shift_immediate", 0x2d43_114a_5ad1_47f3),
        (ENGINE, "shift_register", 0x8c18_0772_fe45_bbb7),
        (ENGINE, "add_or_subtract", 0x8672_d184_e60d_64d9),
        (ENGINE, "execute_shift_add_sub", 0xc750_8f25_cb63_1262),
        (ENGINE, "condition_holds", 0x03e2_586d_93bd_6379),
        (ENGINE, "execute_barrier", 0xa8cf_2b3c_a5a6_d271),
        (ENGINE, "special_lines", 0xe00f_d341_22e6_cce0),
        (ENGINE, "read_special", 0xe8d0_16c8_6dbc_85f7),
        (ENGINE, "execute_special_register", 0x4ce9_bae6_c912_1fe7),
        (ENGINE, "step_state", 0x4a27_89b6_2615_be22),
        (GATES, "shift_immediate_network", 0xbb7f_b93c_6f3e_98f0),
        (GATES, "shift_register_network", 0x14d7_a3c8_0d88_8e6e),
        (GATES, "select_bit_32", SELECT_BIT_32_LOCK),
        (LIB, "commit_transition", COMMIT_TRANSITION_LOCK),
        (
            LIB,
            "restore_registers_and_control",
            RESTORE_REGISTERS_AND_CONTROL_LOCK,
        ),
        (LIB, "restore_validated", RESTORE_VALIDATED_LOCK),
        (LIB, "reset", 0xf862_a5b2_bd94_bfa9),
        (LIB, "load", 0xc14c_37a4_aa3a_8bb1),
        (LIB, "write_register", 0xb15a_8456_dc34_a702),
        (LIB, "write_memory", 0x3079_1d0f_217d_cd1b),
        (LIB, "signal_event", 0x7e6a_e298_6e56_a9c6),
        (LIB, "step", 0x3aa2_7897_5b43_f0d1),
        (LIB, "run", 0x77bd_e445_639d_00b9),
        (LIB, "execute", 0x59fd_f7d2_1460_60a3),
        (STATE, "clock_bits", CLOCK_BITS_LOCK),
        (STATE, "clock_bit", CLOCK_BIT_LOCK),
        (STATE, "restore_snapshot", 0x10da_57ec_9213_8692),
        (STATE, "clear", 0x4d1e_959e_8ba0_313f),
        (STATE, "write", MEMORY_WRITE_LOCK),
    ];
    let actual = locks
        .into_iter()
        .map(|(source, function, expected)| {
            (
                function,
                expected,
                function_fingerprint(source, function).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        actual.iter().all(|(_, expected, found)| expected == found),
        "review and update exact body locks: {actual:#x?}"
    );
}

#[test]
fn one_hot_bit_selection_deasserts_every_line_outside_the_word() {
    assert_eq!(cortex_m23_gatelevel::gates::select_bit_32(1 << 31, 31), 1);
    assert_eq!(cortex_m23_gatelevel::gates::select_bit_32(u32::MAX, 32), 0);
    assert_eq!(cortex_m23_gatelevel::gates::select_bit_32(u32::MAX, 33), 0);
    assert_eq!(
        cortex_m23_gatelevel::gates::select_bit_32(u32::MAX, usize::MAX as u32),
        0
    );
}

#[test]
fn structural_guard_rejects_hostile_backends_and_confusers() {
    let host_multiply = "fn multiply_32(left:u32,right:u32)->u32{left*right}";
    assert!(audit_function(host_multiply, "multiply_32", &["add_32"], &["mul"]).is_err());

    let dead_anchor =
        "fn multiply_32(left:u32,right:u32)->u32{if false{let _=add_32(left,right,0);}left*right}";
    assert!(audit_function(dead_anchor, "multiply_32", &["add_32"], &["mul"]).is_err());

    let post_gate_overwrite =
        "fn step_state(left:u32,right:u32)->u32{let _=add_32(left,right,0);left+right}";
    assert!(audit_function(post_gate_overwrite, "step_state", &["add_32"], &["add"]).is_err());

    // These fixtures are real, type-checked Rust functions above. The former
    // required-call audit accepts them because each still invokes `register`;
    // the exact stable-Q body locks must reject their direct overwrite.
    for (function, expected) in [
        ("hostile_clock_bits_bypass", CLOCK_BITS_LOCK),
        ("hostile_clock_bit_bypass", CLOCK_BIT_LOCK),
        ("hostile_memory_write_bypass", MEMORY_WRITE_LOCK),
    ] {
        assert!(audit_function(THIS_TEST, function, &["register"], &[]).is_ok());
        assert!(assert_body_lock(THIS_TEST, function, expected).is_err());
    }

    let oracle_substitution =
        "fn step_state()->u32{cortex_m23_simulator::CortexM23Simulator::new();0}";
    assert!(audit_function(oracle_substitution, "step_state", &["execute_decoded"], &[]).is_err());

    let clock_bits_bypass = replace_once(
        STATE,
        "    q.copy_from_slice(&register(&input, 1, &mut state));",
        "    q.copy_from_slice(&register(&input, 1, &mut state));\n    q.copy_from_slice(&input);",
    );
    assert!(assert_body_lock(&clock_bits_bypass, "clock_bits", CLOCK_BITS_LOCK).is_err());

    let clock_bit_bypass = replace_once(
        STATE,
        "    *q = register(&input, 1, &mut state)[0];",
        "    *q = register(&input, 1, &mut state)[0];\n    *q = input[0];",
    );
    assert!(assert_body_lock(&clock_bit_bypass, "clock_bit", CLOCK_BIT_LOCK).is_err());

    let memory_q_bypass = replace_once(
        STATE,
        "            .fold(0, |byte, (bit, q)| byte | (q << bit));",
        "            .fold(0, |byte, (bit, q)| byte | (q << bit));\n        self.q[address] = value;",
    );
    assert!(assert_body_lock(&memory_q_bypass, "write", MEMORY_WRITE_LOCK).is_err());

    let architectural_dff_bypass = replace_once(
        LIB,
        "        self.restore_registers_and_control(next);",
        "        self.restore_registers_and_control(next);\n        self.pc = [0; 32];",
    );
    assert!(assert_body_lock(
        &architectural_dff_bypass,
        "commit_transition",
        COMMIT_TRANSITION_LOCK
    )
    .is_err());

    let register_commit_bypass = replace_once(
        LIB,
        "        clock_bits(&mut self.loaded_len, next.loaded_len as u64);",
        "        clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n        self.loaded_len = [0; 17];",
    );
    assert!(assert_body_lock(
        &register_commit_bypass,
        "restore_registers_and_control",
        RESTORE_REGISTERS_AND_CONTROL_LOCK
    )
    .is_err());

    let rollback_dff_bypass = replace_once(
        LIB,
        "        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);",
        "        clock_bits(&mut self.loaded_len, snapshot.loaded_len as u64);\n        self.loaded_len = [0; 17];",
    );
    assert!(assert_body_lock(
        &rollback_dff_bypass,
        "restore_validated",
        RESTORE_VALIDATED_LOCK
    )
    .is_err());

    assert!(assert_external_modules("mod engine{} pub mod gates{} mod state{}").is_err());
    assert!(
        assert_external_modules("#[path=\"oracle.rs\"] mod engine; pub mod gates; mod state;")
            .is_err()
    );
    assert!(assert_external_modules(
        "macro_rules! m{()=>{mod engine;}} m!(); pub mod gates; mod state;"
    )
    .is_err());
    assert!(
        assert_external_modules("const _: &str=\"mod engine;\"; pub mod gates; mod state;")
            .is_err()
    );
    assert!(assert_external_modules("const _: char='m'; pub mod gates; mod state;").is_err());

    let shadowed = "fn step_state()->u32{fn execute_decoded()->u32{0} let add_32=||0;execute_decoded()+add_32()}";
    assert!(audit_function(shadowed, "step_state", &["execute_decoded", "add_32"], &[]).is_err());

    let dead_dispatch = "fn step_state()->u32{if false{execute_decoded();add_32();}0}";
    assert!(audit_function(
        dead_dispatch,
        "step_state",
        &["execute_decoded", "add_32"],
        &[]
    )
    .is_err());

    let post_dispatch = "fn step_state(mut state:State){execute_decoded(&mut state);state.pc=0;}";
    assert!(
        assert_no_state_write_after_call(post_dispatch, "step_state", "execute_decoded").is_err()
    );

    let direct = "fn shift_immediate()->u32{shift_immediate_network()}";
    let indirect = "fn shift_immediate()->u32{fn helper()->u32{host_shift()} helper()}";
    assert_ne!(
        function_fingerprint(direct, "shift_immediate").unwrap(),
        function_fingerprint(indirect, "shift_immediate").unwrap()
    );

    let committed = "fn commit_transition(){read();write();restore_registers_and_control();}";
    let overwritten =
        "fn commit_transition(){read();write();restore_registers_and_control();state.pc=0;}";
    assert_ne!(
        function_fingerprint(committed, "commit_transition").unwrap(),
        function_fingerprint(overwritten, "commit_transition").unwrap()
    );
}

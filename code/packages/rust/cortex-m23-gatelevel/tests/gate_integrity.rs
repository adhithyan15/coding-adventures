use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::{
    BinOp, Expr, ExprAssign, ExprBinary, ExprCall, ExprIf, ExprMethodCall, File, ImplItem, Item,
    ItemFn, Lit, PatIdent,
};

const LIB: &str = include_str!("../src/lib.rs");
const ENGINE: &str = include_str!("../src/engine.rs");
const GATES: &str = include_str!("../src/gates.rs");
const STATE: &str = include_str!("../src/state.rs");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[derive(Default)]
struct BodyFacts {
    calls: Vec<String>,
    binary: BTreeSet<&'static str>,
    paths: BTreeSet<String>,
    local_names: BTreeSet<String>,
    dead_control: bool,
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

fn function_body(source: &str, function: &str) -> Result<syn::Block, String> {
    let file = parse(source)?;
    let mut matches = Vec::new();
    for item in file.items {
        match item {
            Item::Fn(item) if item.sig.ident == function => matches.push(*item.block),
            Item::Impl(item) => {
                for member in item.items {
                    if let ImplItem::Fn(method) = member {
                        if method.sig.ident == function {
                            matches.push(method.block);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
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
        "multiply_32",
        &["select_32", "shift_left_32", "add_32"],
        &["mul"],
    )
    .unwrap();
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
        &["and_gate", "or_gate", "xor_gate", "not_gate"],
        &["and", "or"],
    )
    .unwrap();
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
        &["read", "write", "restore_registers_and_control"],
        &[],
    )
    .unwrap();
    audit_function(STATE, "clock_bits", &["register"], &[]).unwrap();
    audit_function(STATE, "clock_bit", &["register"], &[]).unwrap();
    audit_function(STATE, "write", &["register"], &[]).unwrap();
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

    let oracle_substitution =
        "fn step_state()->u32{cortex_m23_simulator::CortexM23Simulator::new();0}";
    assert!(audit_function(oracle_substitution, "step_state", &["execute_decoded"], &[]).is_err());

    let direct_q = "fn write(&mut self,address:usize,value:u8){self.q[address]=value}";
    assert!(audit_function(direct_q, "write", &["register"], &[]).is_err());

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
}

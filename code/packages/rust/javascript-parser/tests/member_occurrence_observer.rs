//! CLOC34: prove actual typed prefix ownership, not merely token presence.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use coding_adventures_javascript_ast::{
    expression::Expression, statement::TaggedStatement, EqIgnoringCv, Program, ProgramItem,
    Statement,
};
use coding_adventures_javascript_parser::{
    bridge::{grammar_to_program, grammar_to_program_with_array_observer,
        grammar_to_program_with_composite_observer, ArrayOccurrenceObserver,
        CompositeOccurrenceObserver, ObservedBridgeError},
    parse_javascript_typed, parse_javascript_typed_with_cv, DEFAULT_ES_VERSION,
};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn tokens<'a>(children: &'a [ASTNodeOrToken], out: &mut Vec<&'a Token>) {
    for child in children {
        match child {
            ASTNodeOrToken::Node(node) => tokens(&node.children, out),
            ASTNodeOrToken::Token(token) => out.push(token),
        }
    }
}

#[derive(Debug)]
struct Record {
    kind: &'static str,
    owner: usize,
    end: usize,
    computed: bool,
    optional: bool,
    text: String,
    id: String,
    parents: Vec<String>,
}

struct Recording<'a> { log: &'a mut CVLog, records: Vec<Record> }
impl Recording<'_> {
    fn record(&mut self, node: &GrammarASTNode, end: usize, kind: &'static str,
        computed: bool, optional: bool) -> Result<Option<String>, String> {
        let mut physical = Vec::new();
        tokens(&node.children[..=end], &mut physical);
        let parents: Vec<_> = physical.iter().map(|t| t.cv.clone().expect("real lexer CV")).collect();
        let refs: Vec<_> = parents.iter().map(String::as_str).collect();
        let id = self.log.try_merge(&refs, None)?;
        self.records.push(Record { kind, owner: node as *const GrammarASTNode as usize,
            end, computed, optional, text: physical.iter().map(|t| t.value.as_str()).collect(),
            id: id.clone(), parents });
        Ok(Some(id))
    }
}
impl ArrayOccurrenceObserver for Recording<'_> {
    fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, String> {
        self.record(node, node.children.len() - 1, "array", false, false)
    }
}
impl CompositeOccurrenceObserver for Recording<'_> {
    fn member_constructed(&mut self, node: &GrammarASTNode, end: usize, computed: bool,
        optional: bool) -> Result<Option<String>, String> {
        let ASTNodeOrToken::Token(last) = &node.children[end] else { panic!("direct endpoint"); };
        if computed { assert_eq!(last.value, "]"); }
        self.record(node, end, "member", computed, optional)
    }
}

// This test-only walk follows typed construction order. Leaf identities and
// non-observed wrapper CVs are checked independently of CV-erased equality.
fn visit(expr: &Expression, nodes: &mut Vec<(&'static str, Option<String>, bool, bool)>,
    leaves: &mut Vec<(String, Option<String>)>) {
    match expr {
        Expression::MemberExpression(m) => {
            visit(&m.object, nodes, leaves); visit(&m.property, nodes, leaves);
            nodes.push(("member", m.cv.clone(), m.computed, false));
        }
        Expression::OptionalMemberExpression(m) => {
            visit(&m.object, nodes, leaves); visit(&m.property, nodes, leaves);
            nodes.push(("member", m.cv.clone(), m.computed, true));
        }
        Expression::ArrayExpression(a) => {
            for value in a.elements.iter().flatten() { visit(value, nodes, leaves); }
            nodes.push(("array", a.cv.clone(), false, false));
        }
        Expression::ChainExpression(c) => { assert!(c.cv.is_none()); visit(&c.expression, nodes, leaves); }
        Expression::CallExpression(c) => {
            assert!(c.cv.is_none()); visit(&c.callee, nodes, leaves);
            for arg in &c.arguments { visit(arg, nodes, leaves); }
        }
        Expression::OptionalCallExpression(c) => {
            assert!(c.cv.is_none()); visit(&c.callee, nodes, leaves);
            for arg in &c.arguments { visit(arg, nodes, leaves); }
        }
        Expression::NewExpression(n) => {
            assert!(n.cv.is_none()); visit(&n.callee, nodes, leaves);
            for arg in &n.arguments { visit(arg, nodes, leaves); }
        }
        Expression::Identifier(i) => leaves.push((i.name.clone(), i.cv.clone())),
        Expression::Super(s) => leaves.push(("super".into(), s.cv.clone())),
        other => panic!("fixture visitor missing {other:?}"),
    }
}
fn program_nodes(program: &Program) -> (Vec<(&'static str, Option<String>, bool, bool)>, Vec<(String, Option<String>)>) {
    let mut nodes = Vec::new(); let mut leaves = Vec::new();
    for item in &program.body {
        let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(s))) = item
            else { panic!("fixture expression"); };
        visit(&s.expression, &mut nodes, &mut leaves);
    }
    (nodes, leaves)
}

#[test]
fn exact_member_prefixes_own_distinct_ids_and_original_ordered_parents() {
    let source = "[a[b.c],a?.[d.e],f().x[y],new F().x,(a).b.c,a?.b.c,a?.().x];a.b;a.b;";
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let node = parse_javascript_typed_with_cv(source, "prefix.js", DEFAULT_ES_VERSION, &mut log).unwrap();
    let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap();
    let mut observer = Recording { log: &mut log, records: Vec::new() };
    let observed = grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap();
    assert!(legacy.eq_ignoring_cv(&observed));
    let (typed, leaves) = program_nodes(&observed);
    assert_eq!(leaves, program_nodes(&legacy).1, "original leaf CVs preserved");
    assert_eq!(observer.records.iter().filter(|r| r.kind == "member").map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["b.c", "a[b.c]", "d.e", "a?.[d.e]", "f().x", "f().x[y]", "newF().x",
            "(a).b", "(a).b.c", "a?.b", "a?.b.c", "a?.().x", "a.b", "a.b"]);
    assert_eq!(typed.len(), observer.records.len());
    for (typed, record) in typed.iter().zip(&observer.records) {
        assert_eq!(typed, &(record.kind, Some(record.id.clone()), record.computed, record.optional));
        assert_eq!(observer.log.get(&record.id).unwrap().parent_ids, record.parents);
        assert!(!record.parents.contains(&record.id), "fresh composite identity");
    }
    let unique: std::collections::BTreeSet<_> = observer.records.iter().map(|r| &r.id).collect();
    assert_eq!(unique.len(), observer.records.len());
    for pair in [["(a).b", "(a).b.c"], ["a?.b", "a?.b.c"], ["f().x", "f().x[y]"]] {
        let first = observer.records.iter().find(|r| r.text == pair[0]).unwrap();
        let second = observer.records.iter().find(|r| r.text == pair[1]).unwrap();
        assert_eq!(first.owner, second.owner, "same borrowed owner, distinct suffixes");
        assert!(first.end < second.end);
        assert!(second.parents.starts_with(&first.parents));
        assert_ne!(first.parents, second.parents, "later suffix excluded from earlier prefix");
    }
    let repeated: Vec<_> = observer.records.iter().filter(|r| r.text == "a.b").collect();
    assert_ne!(repeated[0].owner, repeated[1].owner);
    assert!(repeated[0].parents.iter().all(|id| !repeated[1].parents.contains(id)), "siblings excluded");
    assert_eq!(grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap(), legacy);
}

#[derive(Default)]
struct Noop { arrays: usize, members: usize }
impl ArrayOccurrenceObserver for Noop {
    fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> { self.arrays += 1; Ok(None) }
}
impl CompositeOccurrenceObserver for Noop {
    fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
        self.members += 1; Ok(None)
    }
}
const PARENTS: &[&str] = &[
    "a.b;", "f(a.b);", "var x=a.b;", "let x=a.b;", "function f(x=a.b){return a.b;}",
    "x=()=>a.b;", "class C { x=a.b; m(x=a.b){return a.b;} }", "({[a.b]:a.b});",
    "if(x){a.b;}else{a.b;}", "for(var x=a.b;false;){a.b;}",
    "try{a.b;}catch(e){a.b;}finally{a.b;}", "switch(x){case 1:a.b;break;default:a.b;}",
    "x[a.b];", "new F(a.b);", "x?.[a.b];", "export default a.b;",
    "[a.b,[],a?.b.c];", "class C extends B { m(){super.x;super[y];} }",
];
#[test]
fn no_op_and_array_only_compatibility_under_supported_parents() {
    for source in PARENTS {
        let node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap();
        let mut observer = Noop::default();
        assert_eq!(grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap(), legacy, "{source}");
        assert!(observer.members > 0, "{source}");
        let mut old = Noop::default();
        assert_eq!(grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut old).unwrap(), legacy);
        assert_eq!(old.members, 0);
        assert_eq!(old.arrays, observer.arrays);
    }
}

struct Refuse { on_array: bool, calls: usize }
impl ArrayOccurrenceObserver for Refuse {
    fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
        self.calls += 1; if self.on_array { Err("exact array refusal".into()) } else { Ok(None) }
    }
}
impl CompositeOccurrenceObserver for Refuse {
    fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
        self.calls += 1; Err("exact member refusal".into())
    }
}
#[test]
fn first_member_refusal_stops_both_methods_and_refuses_all_parent_paths() {
    for source in PARENTS {
        let node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        let mut sink = Refuse { on_array: false, calls: 0 };
        let error = grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink).unwrap_err();
        assert!(matches!(error, ObservedBridgeError::Observer(s) if s == "exact member refusal"), "{source}");
        assert_eq!(sink.calls, 1, "no array/member callback after refusal: {source}");
    }
    for (source, expected) in [("[a.b].c;", "exact member refusal"), ("[[]].x;", "exact array refusal")] {
        let node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        let mut sink = Refuse { on_array: true, calls: 0 };
        assert!(matches!(grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink),
            Err(ObservedBridgeError::Observer(s)) if s == expected));
        assert_eq!(sink.calls, 1);
    }
}

#[test]
fn graph_exhaustion_from_member_sink_is_a_refusal() {
    for limits in [GraphLimits { max_nodes: 5, ..Default::default() },
        GraphLimits { max_edges: 0, ..Default::default() }, GraphLimits { max_events: 0, ..Default::default() }] {
        let mut log = CVLog::new_checked_compact(limits).unwrap();
        let node = parse_javascript_typed_with_cv("a.b;", "limit.js", DEFAULT_ES_VERSION, &mut log).unwrap();
        assert_eq!(log.entries().len(), 5);
        let mut sink = Recording { log: &mut log, records: Vec::new() };
        assert!(matches!(grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink),
            Err(ObservedBridgeError::Observer(s)) if s.contains("limit")));
        assert!(sink.records.is_empty());
    }
}

fn break_close(node: &mut GrammarASTNode, remove: bool) -> bool {
    if ["member_expression", "call_expression", "optional_chain_expression"].contains(&node.rule_name.as_str()) {
        if let Some(index) = node.children.iter().position(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "]")) {
            if remove { node.children.remove(index); }
            else if let ASTNodeOrToken::Token(token) = &mut node.children[index] { token.value = ")".into(); }
            return true;
        }
    }
    node.children.iter_mut().any(|c| match c { ASTNodeOrToken::Node(n) => break_close(n, remove), _ => false })
}
#[test]
fn observed_only_bad_computed_endpoints_never_reach_a_sink() {
    for source in ["a[b];", "f()[b];", "a?.[b];"] {
        for remove in [false, true] {
            let mut node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
            assert!(break_close(&mut node, remove));
            let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap();
            let mut old = Noop::default();
            assert_eq!(grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut old).unwrap(), legacy);
            let mut sink = Noop::default();
            assert!(matches!(grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink),
                Err(ObservedBridgeError::Observer(s)) if s == "member occurrence endpoint is not a direct closing bracket"));
            assert_eq!(sink.members, 0);
        }
    }
}

#[test]
fn ordinary_errors_and_member_panics_keep_failure_kinds() {
    for source in ["a.b;", "async function f(){}"] {
        let mut node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        if source == "a.b;" { node.rule_name = "wrong_root".into(); }
        let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap_err().to_string();
        let mut sink = Noop::default();
        let error = grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink).unwrap_err();
        assert!(matches!(error, ObservedBridgeError::Bridge(_)));
        assert_eq!(error.to_string(), legacy);
        assert_eq!(sink.members, 0);
    }
    struct Panic;
    impl ArrayOccurrenceObserver for Panic {
        fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> { Ok(None) }
    }
    impl CompositeOccurrenceObserver for Panic {
        fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> { panic!("injected member panic"); }
    }
    let node = parse_javascript_typed("a.b;", DEFAULT_ES_VERSION).unwrap();
    assert!(matches!(grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut Panic),
        Err(ObservedBridgeError::Worker(s)) if s == "worker panicked"));
}

#[test]
fn nested_composite_run_uses_an_independent_context() {
    struct Nested(usize);
    impl ArrayOccurrenceObserver for Nested {
        fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> { Ok(None) }
    }
    impl CompositeOccurrenceObserver for Nested {
        fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
            let node = parse_javascript_typed("[a.b];", DEFAULT_ES_VERSION).unwrap();
            let mut inner = Noop::default();
            let result = grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut inner).unwrap();
            assert_eq!((inner.arrays, inner.members), (1, 1));
            assert!(program_nodes(&result).0.iter().all(|n| n.1.is_none()));
            self.0 += 1; Ok(Some(format!("caller-member-{}", self.0)))
        }
    }
    let node = parse_javascript_typed("a.b.c;", DEFAULT_ES_VERSION).unwrap();
    let mut sink = Nested(0);
    let result = grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink).unwrap();
    assert_eq!(program_nodes(&result).0.iter().map(|n| n.1.as_deref()).collect::<Vec<_>>(),
        [Some("caller-member-1"), Some("caller-member-2")]);
    assert_eq!(sink.0, 2);
}

#[test]
fn late_member_refusal_disposes_deep_prefix_on_worker() {
    const CHILD: &str = "CLOC34_MEMBER_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "late_member_refusal_disposes_deep_prefix_on_worker", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        assert!(result.status.success(), "{:?}\n{}\n{}", result.status,
            String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        return;
    }
    struct Late(usize);
    impl ArrayOccurrenceObserver for Late {
        fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> { Ok(None) }
    }
    impl CompositeOccurrenceObserver for Late {
        fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
            self.0 += 1;
            if self.0 == 4097 { Err("late member refusal".into()) } else { Ok(None) }
        }
    }
    let node = parse_javascript_typed(&format!("a{}[[]];", ".p".repeat(4096)), DEFAULT_ES_VERSION).unwrap();
    std::thread::Builder::new().stack_size(128 * 1024).spawn(move || {
        let mut sink = Late(0);
        match grammar_to_program_with_composite_observer(&node, DEFAULT_ES_VERSION, &mut sink) {
            Err(ObservedBridgeError::Observer(s)) => assert_eq!(s, "late member refusal"),
            Ok(program) => { std::mem::forget(program); panic!("late refusal ignored"); }
            Err(other) => panic!("wrong error: {other}"),
        }
        assert_eq!(sink.0, 4097);
    }).unwrap().join().unwrap();
}

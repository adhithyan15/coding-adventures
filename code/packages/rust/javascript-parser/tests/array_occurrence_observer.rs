//! CLOC32 verifies ownership on real typed arrays, not a registry containing
//! unrelated IDs. This library hook does not certify CLI source spans.
use coding_adventures_correlation_vector::{CVLog, GraphLimits};
use coding_adventures_javascript_ast::{
    expression::{ArrayExpression, Expression}, statement::TaggedStatement,
    EqIgnoringCv, Program, ProgramItem, Statement,
};
use coding_adventures_javascript_parser::{
    bridge::{grammar_to_program, grammar_to_program_with_array_observer,
        ArrayOccurrenceObserver, ObservedBridgeError},
    parse_javascript_typed, parse_javascript_typed_with_cv, DEFAULT_ES_VERSION,
};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

fn tokens<'a>(node: &'a GrammarASTNode, out: &mut Vec<&'a Token>) {
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(node) => tokens(node, out),
            ASTNodeOrToken::Token(token) => out.push(token),
        }
    }
}

fn array_occurrences(node: &GrammarASTNode, out: &mut Vec<usize>) {
    for child in &node.children {
        if let ASTNodeOrToken::Node(node) = child { array_occurrences(node, out); }
    }
    if node.rule_name == "array_literal" {
        // Test-only pointer equality proves the borrowed occurrence is exact;
        // pointers never become graph/serialized identities.
        out.push(node as *const GrammarASTNode as usize);
    }
}

fn expression(program: &Program) -> &Expression {
    let ProgramItem::Statement(Statement::Tagged(TaggedStatement::ExpressionStatement(s))) = &program.body[0]
        else { panic!("expression statement"); };
    &s.expression
}

fn array(expr: &Expression) -> &ArrayExpression {
    let Expression::ArrayExpression(array) = expr else { panic!("array"); };
    array
}

struct Recording<'a> {
    log: &'a mut CVLog,
    records: Vec<(usize, String, Vec<String>)>,
}

impl ArrayOccurrenceObserver for Recording<'_> {
    fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, String> {
        let mut physical = Vec::new();
        tokens(node, &mut physical);
        assert_eq!(physical.first().unwrap().value, "[");
        assert_eq!(physical.last().unwrap().value, "]");
        let parents: Vec<_> = physical.iter().map(|t| t.cv.clone().expect("real lexer identity")).collect();
        let refs: Vec<_> = parents.iter().map(String::as_str).collect();
        let id = self.log.try_merge(&refs, None)?;
        self.log.contribute(&id, "test-array-observer", "constructed", Default::default())?;
        self.records.push((node as *const GrammarASTNode as usize, id.clone(), parents));
        Ok(Some(id))
    }
}

#[test]
fn typed_arrays_own_distinct_ids_and_exact_postorder_token_parents() {
    let mut log = CVLog::new_checked_compact(GraphLimits::default()).unwrap();
    let node = parse_javascript_typed_with_cv("[[],[1,,2],[]];", "arrays.js", DEFAULT_ES_VERSION, &mut log).unwrap();
    let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap();
    let mut expected_occurrences = Vec::new();
    array_occurrences(&node, &mut expected_occurrences);
    let mut observer = Recording { log: &mut log, records: Vec::new() };
    let observed = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap();
    assert!(legacy.eq_ignoring_cv(&observed), "only CV fields may change");
    assert_eq!(observer.records.iter().map(|r| r.0).collect::<Vec<_>>(), expected_occurrences);
    assert_eq!(observer.records.len(), 4);
    let outer = array(expression(&observed));
    for (element, record) in outer.elements.iter().zip(&observer.records[..3]) {
        assert_eq!(array(element.as_ref().unwrap()).cv.as_deref(), Some(record.1.as_str()));
    }
    assert_eq!(outer.cv.as_deref(), Some(observer.records[3].1.as_str()));
    assert_eq!(array(outer.elements[1].as_ref().unwrap()).elements.len(), 3);
    assert!(array(outer.elements[1].as_ref().unwrap()).elements[1].is_none());
    let mut all_tokens = Vec::new();
    tokens(&node, &mut all_tokens);
    for (_, id, parents) in &observer.records {
        assert_eq!(&observer.log.get(id).unwrap().parent_ids, parents);
        assert!(!all_tokens.iter().any(|t| t.cv.as_deref() == Some(id)), "composite must be fresh");
        for parent in parents { assert!(observer.log.get(parent).unwrap().origin.is_some()); }
    }
    let unique: std::collections::BTreeSet<_> = observer.records.iter().map(|r| &r.1).collect();
    assert_eq!(unique.len(), 4, "identical empty arrays are distinct occurrences");
    // The legacy facade remains unobserved even after an observed run.
    assert_eq!(grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap(), legacy);
}

struct Noop(usize);
impl ArrayOccurrenceObserver for Noop {
    fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
        self.0 += 1;
        Ok(None)
    }
}

struct Refuse(usize);
impl ArrayOccurrenceObserver for Refuse {
    fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
        self.0 += 1;
        Err(format!("exact first observer error {}", self.0))
    }
}

const PARENTS: &[&str] = &[
    "[];", "f([]);", "var x=[];", "let x=[];", "function f(a=[]){return [];}",
    "x=()=>[];", "class C { x=[]; m(a=[]){return [];} }", "({[[]]:[]});",
    "if(x){[];}else{[];}", "for(var x=[];false;){[];}",
    "try{[];}catch(e){[];}finally{[];}", "switch(x){case 1:[];break;default:[];}",
    "x[[]];", "new F([]);", "x?.[[]];", "export default [];",
];

#[test]
fn no_op_observer_preserves_the_entire_program_under_all_supported_parents() {
    for source in PARENTS {
        let node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap_or_else(|e| panic!("{source}: {e}"));
        let mut occurrences = Vec::new();
        array_occurrences(&node, &mut occurrences);
        assert!(!occurrences.is_empty());
        let mut observer = Noop(0);
        let observed = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap();
        assert_eq!(observed, legacy, "{source}");
        assert_eq!(observer.0, occurrences.len(), "{source}");
    }
}

#[test]
fn observer_error_never_becomes_a_partial_program_or_later_observer_call() {
    for source in PARENTS {
        let node = parse_javascript_typed(source, DEFAULT_ES_VERSION).unwrap();
        let mut observer = Refuse(0);
        let error = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap_err();
        assert!(matches!(error, ObservedBridgeError::Observer(ref s) if s == "exact first observer error 1"), "{source}: {error}");
        assert_eq!(observer.0, 1, "no callback after first refusal: {source}");
    }
}

#[test]
fn checked_graph_failure_from_the_observer_refuses_the_bridge() {
    for limits in [
        GraphLimits { max_nodes: 4, ..Default::default() },
        GraphLimits { max_edges: 0, ..Default::default() },
        GraphLimits { max_events: 0, ..Default::default() },
    ] {
        let mut log = CVLog::new_checked_compact(limits).unwrap();
        let node = parse_javascript_typed_with_cv("[];", "array.js", DEFAULT_ES_VERSION, &mut log).unwrap();
        assert_eq!(log.entries().len(), 4, "physical [, ], ; and EOF");
        let mut observer = Recording { log: &mut log, records: Vec::new() };
        let error = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap_err();
        assert!(matches!(error, ObservedBridgeError::Observer(ref s) if s.contains("limit")), "{error}");
        assert!(observer.records.is_empty());
    }
}

#[test]
fn ordinary_bridge_errors_keep_their_original_kind_and_diagnostic() {
    let mut node = parse_javascript_typed("[];", DEFAULT_ES_VERSION).unwrap();
    node.rule_name = "wrong_root".into();
    let legacy = grammar_to_program(&node, DEFAULT_ES_VERSION).unwrap_err().to_string();
    let mut observer = Noop(0);
    let error = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer).unwrap_err();
    assert!(matches!(error, ObservedBridgeError::Bridge(_)));
    assert_eq!(error.to_string(), legacy);
    assert_eq!(observer.0, 0);
}

#[test]
fn nested_observed_run_has_its_own_borrowed_context() {
    struct Nested(Vec<String>);
    impl ArrayOccurrenceObserver for Nested {
        fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
            let inner = parse_javascript_typed("[];", DEFAULT_ES_VERSION).unwrap();
            let mut noop = Noop(0);
            let result = grammar_to_program_with_array_observer(&inner, DEFAULT_ES_VERSION, &mut noop).unwrap();
            assert_eq!(noop.0, 1);
            assert!(array(expression(&result)).cv.is_none());
            let id = format!("caller-owned-{}", self.0.len());
            self.0.push(id.clone());
            Ok(Some(id))
        }
    }
    let node = parse_javascript_typed("[[]];", DEFAULT_ES_VERSION).unwrap();
    let mut nested = Nested(Vec::new());
    let result = grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut nested).unwrap();
    let outer = array(expression(&result));
    assert_eq!(outer.cv.as_deref(), Some("caller-owned-1"));
    assert_eq!(array(outer.elements[0].as_ref().unwrap()).cv.as_deref(), Some("caller-owned-0"));
    assert_eq!(nested.0, ["caller-owned-0", "caller-owned-1"]);
}

#[test]
fn observer_failure_disposes_a_deep_candidate_on_a_small_caller() {
    const CHILD: &str = "CLOC32_ARRAY_BRIDGE_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "observer_failure_disposes_a_deep_candidate_on_a_small_caller", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        assert!(result.status.success(), "{:?}\n{}\n{}", result.status,
            String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        return;
    }
    let source = format!("[a{}];", ".p".repeat(4096));
    let node = parse_javascript_typed(&source, DEFAULT_ES_VERSION).unwrap();
    std::thread::Builder::new().stack_size(128 * 1024).spawn(move || {
        let mut observer = Refuse(0);
        match grammar_to_program_with_array_observer(&node, DEFAULT_ES_VERSION, &mut observer) {
            Err(ObservedBridgeError::Observer(s)) => assert_eq!(s, "exact first observer error 1"),
            Ok(program) => {
                // Scaffold red must report a normal assertion, not abort the
                // test process by recursively dropping unexpected deep success.
                std::mem::forget(program);
                panic!("observer refusal was ignored");
            }
            Err(other) => panic!("wrong failure: {other}"),
        }
    }).unwrap().join().unwrap();
}

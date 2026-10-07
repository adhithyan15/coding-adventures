//! Callback counters are independent of the journal: they run inside real
//! Pass::run calls. Distinct schedule inventory must not impersonate repeated
//! sweeps, failed callback returns, or candidate acceptance.
use coding_adventures_closure_pass_pipeline::{IterationPolicy, Pass, PassContext, PassError, PassOutput, PassPipeline, PassStats, MAX_SWEEPS};
use coding_adventures_correlation_vector::{CVLog, Contribution, GraphLimits};
use coding_adventures_javascript_ast::{Program, SourceType};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

struct Witness {
    name: &'static str, deps: &'static [&'static str], policy: IterationPolicy,
    changes: usize, fail: bool, contributions: usize, calls: Arc<AtomicUsize>,
}
impl Witness {
    fn new(name: &'static str, calls: &Arc<AtomicUsize>) -> Self {
        Self { name, deps: &[], policy: IterationPolicy::OneShot, changes: 0,
            fail: false, contributions: 0, calls: calls.clone() }
    }
}
impl Pass for Witness {
    fn name(&self) -> &'static str { self.name }
    fn depends_on(&self) -> &[&'static str] { self.deps }
    fn iteration_policy(&self) -> IterationPolicy { self.policy }
    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail { return Err(PassError { pass_name: self.name.to_owned(), message: "original callback error".to_owned() }); }
        Ok(PassOutput { program: ctx.program.clone(), changed: call < self.changes,
            contributions: (0..self.contributions).map(|_| Contribution { source: self.name.to_owned(), tag: "returned".to_owned(), meta: HashMap::new() }).collect(),
            diagnostics: vec![], stats: PassStats::default() })
    }
}
fn setup(cap: usize) -> (CVLog, Program) {
    let mut cv = CVLog::new_checked_chronology(GraphLimits { max_events: cap, ..Default::default() }).unwrap();
    let root = cv.try_create(None).unwrap();
    let program = Program::new(root, EsVersion::Es2025, SourceType::Module);
    (cv, program)
}
fn wire(cv: &CVLog) -> Value {
    cv.validate_graph().unwrap();
    let text = cv.to_json_string().unwrap();
    for imported in [CVLog::from_json_string(&text).unwrap(), CVLog::from_checked_json(&text, GraphLimits::default()).unwrap()] {
        assert_eq!(imported.to_json_string().unwrap(), text);
    }
    serde_json::from_str(&text).unwrap()
}
fn ends(wire: &Value) -> Vec<&str> {
    wire["journal"]["events"].as_array().unwrap().iter()
        .filter(|r| r["event"]["kind"] == "context_end")
        .map(|r| r["event"]["outcome"]["kind"].as_str().unwrap()).collect()
}
#[test]
fn nine_then_twelve_actual_callbacks_include_unchanged_and_repeated_one_shot() {
    let calls: Vec<_> = (0..3).map(|_| Arc::new(AtomicUsize::new(0))).collect();
    let mut pipeline = PassPipeline::new();
    let mut fold = Witness::new("fold", &calls[0]); fold.policy = IterationPolicy::FixedPoint; fold.changes = 2;
    let mut unchanged = Witness::new("unchanged", &calls[1]); unchanged.policy = IterationPolicy::FixedPoint;
    let mut observer = Witness::new("observer", &calls[2]); observer.changes = usize::MAX; observer.contributions = 1;
    pipeline.add(Box::new(fold)).add(Box::new(unchanged)).add(Box::new(observer));
    let (mut cv, program) = setup(45);
    let output = pipeline.run(program, &Sidecar::new(), &mut cv).unwrap();
    assert_eq!(calls.iter().map(|c| c.load(Ordering::SeqCst)).sum::<usize>(), 9);
    assert_eq!(output.execution_order, ["fold", "unchanged", "observer"]);
    assert_eq!(cv.pass_order(), ["observer"]);
    let first = wire(&cv);
    let invocations: Vec<_> = first["journal"]["events"].as_array().unwrap().iter()
        .filter(|r| r["event"]["scope"]["kind"] == "pass").collect();
    assert_eq!(invocations.len(), 9);
    for (index, record) in invocations.iter().enumerate() {
        assert_eq!(record["event"]["scope"]["slot"], format!("{:016x}", index % 3));
        assert_eq!(record["event"]["scope"]["sweep"], format!("{:016x}", index / 3));
    }
    assert_eq!(ends(&first).last().copied(), Some("converged"));
    let output = pipeline.run(output.program, &Sidecar::new(), &mut cv).unwrap();
    assert_eq!(calls.iter().map(|c| c.load(Ordering::SeqCst)).sum::<usize>(), 12);
    assert_eq!(output.execution_order, ["fold", "unchanged", "observer"]);
    let second = wire(&cv);
    assert_eq!(second["journal"]["events"].as_array().unwrap().iter().filter(|r| r["event"]["scope"]["kind"] == "pass").count(), 12);
}
#[test]
fn callback_failure_and_returned_contribution_rejection_are_distinct() {
    for callback_failure in [true, false] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut pass = Witness::new("witness", &calls); pass.fail = callback_failure; pass.contributions = 2;
        let mut pipeline = PassPipeline::new(); pipeline.add(Box::new(pass));
        let (mut cv, program) = setup(if callback_failure { 64 } else { 9 });
        let root = program.cv.clone().unwrap();
        let error = pipeline.run(program, &Sidecar::new(), &mut cv).unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let kind = if callback_failure { "callback_failure" } else { "acceptance_failure" };
        assert_eq!(ends(&wire(&cv)), [kind, kind]);
        assert_eq!(cv.get(&root).unwrap().contributions.len(), usize::from(!callback_failure));
        if callback_failure { assert_eq!(error.message, "original callback error"); }
        else { assert!(error.message.contains("CV contribution failed")); }
    }
}
#[test]
fn scheduling_failure_has_no_schedule_or_fabricated_pass_invocation() {
    for duplicate in [true, false] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut first = Witness::new("a", &calls); first.deps = if duplicate { &[] } else { &["b"] };
        let mut second = Witness::new(if duplicate { "a" } else { "b" }, &calls); second.deps = if duplicate { &[] } else { &["a"] };
        let mut pipeline = PassPipeline::new(); pipeline.add(Box::new(first)).add(Box::new(second));
        let (mut cv, program) = setup(64);
        assert!(pipeline.run(program, &Sidecar::new(), &mut cv).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let wire = wire(&cv); assert_eq!(ends(&wire), ["scheduling_failure"]);
        assert_eq!(cv.journal().unwrap().events().len(), 3); // root, pipeline begin, pipeline end
    }
}
#[test]
fn recording_prefix_failures_preserve_terminal_reservations() {
    for cap in [1, 3, 6] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut pipeline = PassPipeline::new(); pipeline.add(Box::new(Witness::new("witness", &calls)));
        let (mut cv, program) = setup(cap);
        assert!(pipeline.run(program, &Sidecar::new(), &mut cv).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let wire = wire(&cv);
        assert_eq!(ends(&wire), if cap == 1 { vec![] } else { vec!["recording_failure"] });
    }
}
#[test]
fn empty_missing_dependency_and_changed_one_shot_converge() {
    for case in 0..3 {
        let calls = Arc::new(AtomicUsize::new(0)); let mut pipeline = PassPipeline::new();
        if case != 0 {
            let mut pass = Witness::new("witness", &calls);
            pass.changes = usize::MAX;
            if case == 1 { pass.deps = &["not-registered"]; }
            pipeline.add(Box::new(pass));
        }
        let (mut cv, program) = setup(64);
        let output = pipeline.run(program, &Sidecar::new(), &mut cv).unwrap();
        assert!(output.diagnostics.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(case != 0));
        assert_eq!(ends(&wire(&cv)).last().copied(), Some("converged"));
    }
}
#[test]
fn cap_records_every_actual_invocation_and_truthful_terminal_outcome() {
    let calls = Arc::new(AtomicUsize::new(0)); let mut pipeline = PassPipeline::new();
    let mut pass = Witness::new("changing", &calls); pass.policy = IterationPolicy::FixedPoint; pass.changes = usize::MAX;
    pipeline.add(Box::new(pass)).add(Box::new(Witness::new("observer", &calls)));
    let (mut cv, program) = setup(1024);
    let output = pipeline.run(program, &Sidecar::new(), &mut cv).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), MAX_SWEEPS * 2);
    assert!(output.diagnostics.iter().any(|d| d.group.0 == "pipeline.fixed-point-cap-reached"));
    let wire = wire(&cv); assert_eq!(ends(&wire).last().copied(), Some("cap"));
    assert_eq!(wire["journal"]["events"].as_array().unwrap().iter().filter(|r| r["event"]["scope"]["kind"] == "pass").count(), 200);
}

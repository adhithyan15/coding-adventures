//! MACSYMA runtime session facade.
//!
//! This crate is the Rust runtime layer above the grammar-driven MACSYMA
//! compiler. It keeps the public API pure and WASM-friendly: callers pass
//! source strings in and receive evaluated IR nodes plus display metadata.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cas_list_operations::{
    append, apply_ as list_apply, first, flatten, join, last, length, map_ as list_map, part,
    range_ as list_range, rest, reverse, sort_, ListOperationError, ListResult,
};
use cas_pretty_printer::{pretty, pretty_2d, MacsymaDialect};
use cas_simplify::{simplify, AssumptionContext};
use cas_solve::{solve_linear_system, try_solve_inequality, try_solve_transcendental, SOLVE};
use cas_substitution::subst;
use cas_trig::{expand_trig, trig_reduce, trig_simplify};
use coding_adventures_macsyma_compiler::{
    compile_macsyma_with_options, CompileError, CompileOptions, DISPLAY as COMPILER_DISPLAY,
    SUPPRESS as COMPILER_SUPPRESS,
};
use symbolic_ir::{
    apply, str_node, sym, IRApply, IRNode, ASSIGN, DEFINE, GREATER, GREATER_EQUAL, IF, LESS,
    LESS_EQUAL, LIST, POW,
};
use symbolic_vm::backend::{handler_fn, Backend, Handler};
use symbolic_vm::handlers::build_handler_table;
use symbolic_vm::VM;

/// Statement wrapper for visible top-level MACSYMA statements.
pub const DISPLAY: &str = COMPILER_DISPLAY;
/// Statement wrapper for suppressed top-level MACSYMA statements.
pub const SUPPRESS: &str = COMPILER_SUPPRESS;
/// Alias for callers that prefer explicit runtime-head naming.
pub const DISPLAY_HEAD: &str = DISPLAY;
/// Alias for callers that prefer explicit runtime-head naming.
pub const SUPPRESS_HEAD: &str = SUPPRESS;
/// Runtime-owned head for clearing bindings and history.
pub const KILL: &str = "Kill";
/// Runtime-owned head for re-evaluation with option flags.
pub const EV: &str = "Ev";
/// Sentinel symbol matched by `Kill(all)`.
pub const ALL: &str = "all";
/// Runtime-owned head for declaring MACSYMA symbol properties.
pub const DECLARE: &str = "Declare";
/// Runtime-owned head for querying properties declared on a symbol.
pub const PROPERTIES: &str = "Properties";
/// Runtime-owned head for querying symbols with declared properties.
pub const PROP_VARS: &str = "PropVars";

const ASSUME: &str = "Assume";
const DONE: &str = "done";
const EXPAND: &str = "Expand";
const FACTOR: &str = "Factor";
const FLOAT_FUNC: &str = "Float";
const FORGET: &str = "Forget";
const IS: &str = "Is";
const RAT_SIMPLIFY: &str = "RatSimplify";
const SIMPLIFY: &str = "Simplify";
const SUBST: &str = "Subst";
const TRIG_EXPAND: &str = "TrigExpand";
const TRIG_REDUCE: &str = "TrigReduce";
const TRIG_SIMPLIFY: &str = "TrigSimplify";
const LENGTH: &str = "Length";
const FIRST: &str = "First";
const REST: &str = "Rest";
const LAST: &str = "Last";
const APPEND: &str = "Append";
const REVERSE: &str = "Reverse";
const RANGE: &str = "Range";
const MAP: &str = "Map";
const APPLY_HEAD: &str = "Apply";
const SORT: &str = "Sort";
const PART: &str = "Part";
const FLATTEN: &str = "Flatten";
const JOIN: &str = "Join";
const SHOWTIME: &str = "showtime";
const TRUE_SYMBOL: &str = "True";
const FALSE_SYMBOL: &str = "False";

/// Return the runtime extension table for MACSYMA surface function names.
///
/// The compiler has a small built-in table for core functions. This runtime
/// table is applied after grammar-driven compilation so the runtime can add
/// CAS/substrate and control heads without parser or lexer changes.
pub fn macsyma_name_table() -> HashMap<String, String> {
    MACSYMA_NAME_TABLE
        .iter()
        .map(|(name, head)| ((*name).to_string(), (*head).to_string()))
        .collect()
}

/// Extend a caller-owned MACSYMA name table in place.
///
/// The operation is idempotent: repeated calls install the same mappings.
pub fn extend_macsyma_name_table(target: &mut HashMap<String, String>) {
    for (name, head) in MACSYMA_NAME_TABLE {
        target.insert(name.to_string(), head.to_string());
    }
}

const MACSYMA_NAME_TABLE: &[(&str, &str)] = &[
    ("subst", SUBST),
    ("simplify", SIMPLIFY),
    ("expand", EXPAND),
    ("factor", FACTOR),
    ("solve", "Solve"),
    ("nsolve", "NSolve"),
    ("linsolve", "Solve"),
    ("taylor", "Taylor"),
    ("limit", "Limit"),
    ("length", LENGTH),
    ("first", FIRST),
    ("rest", REST),
    ("last", LAST),
    ("append", APPEND),
    ("reverse", REVERSE),
    ("makelist", "MakeList"),
    ("map", MAP),
    ("apply", APPLY_HEAD),
    ("sublist", "Select"),
    ("sort", SORT),
    ("part", PART),
    ("flatten", FLATTEN),
    ("join", JOIN),
    ("matrix", "Matrix"),
    ("transpose", "Transpose"),
    ("determinant", "Determinant"),
    ("invert", "Inverse"),
    ("dot", "Dot"),
    ("mattrace", "Trace"),
    ("matrix_size", "Dimensions"),
    ("ident", "IdentityMatrix"),
    ("zeromatrix", "ZeroMatrix"),
    ("rank", "Rank"),
    ("rowreduce", "RowReduce"),
    ("eigenvalues", "Eigenvalues"),
    ("eigenvectors", "Eigenvectors"),
    ("charpoly", "CharPoly"),
    ("nullspace", "NullSpace"),
    ("columnspace", "ColumnSpace"),
    ("rowspace", "RowSpace"),
    ("norm", "Norm"),
    ("lu", "LU"),
    ("mnewton", "MNewton"),
    ("gcd", "Gcd"),
    ("lcm", "Lcm"),
    ("mod", "Mod"),
    ("floor", "Floor"),
    ("ceiling", "Ceiling"),
    ("abs", "Abs"),
    ("sign", "Sign"),
    ("float", FLOAT_FUNC),
    ("lhs", "Lhs"),
    ("rhs", "Rhs"),
    ("at", "At"),
    ("primep", "IsPrime"),
    ("is_prime", "IsPrime"),
    ("next_prime", "NextPrime"),
    ("prev_prime", "PrevPrime"),
    ("ifactor", "FactorInteger"),
    ("divisors", "Divisors"),
    ("totient", "Totient"),
    ("moebius", "MoebiusMu"),
    ("jacobi", "JacobiSymbol"),
    ("chinese", "ChineseRemainder"),
    ("numdigits", "IntegerLength"),
    ("radcan", "Radcan"),
    ("logcontract", "LogContract"),
    ("logexpand", "LogExpand"),
    ("exponentialize", "Exponentialize"),
    ("demoivre", "DeMoivre"),
    ("cbrt", "Cbrt"),
    ("trigsimp", TRIG_SIMPLIFY),
    ("trigexpand", TRIG_EXPAND),
    ("trigreduce", TRIG_REDUCE),
    ("collect", "Collect"),
    ("together", "Together"),
    ("ratsimp", RAT_SIMPLIFY),
    ("partfrac", "Apart"),
    ("%i", "ImaginaryUnit"),
    ("realpart", "Re"),
    ("imagpart", "Im"),
    ("conjugate", "Conjugate"),
    ("cabs", "Abs"),
    ("carg", "Arg"),
    ("rectform", "RectForm"),
    ("polarform", "PolarForm"),
    ("laplace", "Laplace"),
    ("ilt", "ILT"),
    ("delta", "DiracDelta"),
    ("hstep", "UnitStep"),
    ("unit_step", "UnitStep"),
    ("fourier", "Fourier"),
    ("ifourier", "IFourier"),
    ("ode2", "ODE2"),
    ("algfactor", "AlgFactor"),
    ("groebner", "Groebner"),
    ("poly_reduce", "PolyReduce"),
    ("ideal_solve", "IdealSolve"),
    ("kill", KILL),
    ("ev", EV),
    ("block", "Block"),
    ("assume", "Assume"),
    ("forget", "Forget"),
    ("is", "Is"),
    ("declare", DECLARE),
    ("properties", PROPERTIES),
    ("propvars", PROP_VARS),
    ("matchdeclare", "MatchDeclare"),
    ("defrule", "Defrule"),
    ("apply1", "Apply1"),
    ("apply2", "Apply2"),
    ("tellsimp", "TellSimp"),
    ("erf", "Erf"),
    ("erfc", "Erfc"),
    ("erfi", "Erfi"),
    ("si", "Si"),
    ("ci", "Ci"),
    ("shi", "Shi"),
    ("chi", "Chi"),
    ("li2", "Li2"),
    ("gamma", "Gamma"),
    ("beta", "Beta"),
    ("fresnel_s", "FresnelS"),
    ("fresnel_c", "FresnelC"),
    ("lambert_w", "LambertW"),
];

const MACSYMA_HELP_TOPICS: &[(&str, &str)] = &[
    (
        "arithmetic",
        "Arithmetic: use +, -, *, /, and ^. Example: expand((x + 1)^2);",
    ),
    (
        "calculus",
        "Calculus: diff(expr, var), integrate(expr, var), limit(expr, var, point), and taylor(expr, var, point, order).",
    ),
    (
        "diff",
        "diff(expr, var) differentiates expr with respect to var. Example: diff(x^3, x);",
    ),
    (
        "integrate",
        "integrate(expr, var) computes an antiderivative when supported. Example: integrate(x^2, x);",
    ),
    (
        "solve",
        "solve(expr, var) solves equations or supported inequalities. Use linsolve([...], [...]) for linear systems and nsolve(poly, var) for numeric polynomial roots.",
    ),
    (
        "matrix",
        "Matrix tools: matrix([...], ...), transpose, determinant, invert, dot, rank, rowreduce, ident, zeromatrix, and matrix_size.",
    ),
    (
        "lists",
        "List tools: length, first, rest, last, append, reverse, range, map, apply, sublist, sort, part, flatten, join, and makelist.",
    ),
    (
        "assumptions",
        "Assumptions: assume(x > 0), declare(x, positive), is(x > 0), forget(), properties(x), and propvars().",
    ),
    (
        "properties",
        "properties(symbol) lists declared properties. propvars() lists symbols with declared properties.",
    ),
    (
        "display",
        "Display: terminate with ; to show output and $ to suppress it. ev(expr, display2d) renders 2D output.",
    ),
    (
        "history",
        "History: % is the last output; %iN and %oN refer to input and output number N.",
    ),
    (
        "showtime",
        "showtime:true enables per-expression timing; showtime:false disables it.",
    ),
    (
        "repl",
        "REPL commands: :quit exits. Use --file path.mac for batch execution.",
    ),
];

const MACSYMA_HELP_ALIASES: &[(&str, &str)] = &[
    ("d", "diff"),
    ("derivative", "diff"),
    ("integral", "integrate"),
    ("matrices", "matrix"),
    ("list", "lists"),
    ("assume", "assumptions"),
    ("declare", "assumptions"),
    ("propvars", "properties"),
    ("display2d", "display"),
    ("%", "history"),
    ("timing", "showtime"),
    ("quit", "repl"),
];

/// Return the requested topic for a MACSYMA `?` help query.
pub fn parse_macsyma_help_query(source: &str) -> Option<String> {
    let stripped = source.trim();
    if !stripped.starts_with('?') {
        return None;
    }
    let mut topic = stripped.trim_start_matches('?').trim().to_string();
    if topic.ends_with(';') || topic.ends_with('$') {
        topic.truncate(topic.len() - 1);
        topic = topic.trim().to_string();
    }
    Some(topic)
}

/// Return user-facing MACSYMA help text for `topic`.
pub fn macsyma_help_text(topic: Option<&str>) -> String {
    let raw_key = topic.unwrap_or("").trim().to_ascii_lowercase();
    if raw_key.is_empty() {
        return format!(
            "MACSYMA help topics: {}. Use ? topic for details.",
            sorted_help_topics().join(", ")
        );
    }
    let key = MACSYMA_HELP_ALIASES
        .iter()
        .find_map(|(alias, target)| (*alias == raw_key).then_some(*target))
        .unwrap_or(raw_key.as_str());
    if let Some((_, text)) = MACSYMA_HELP_TOPICS.iter().find(|(name, _)| *name == key) {
        return (*text).to_string();
    }
    format!(
        "No MACSYMA help topic named {:?}. Available topics: {}.",
        topic.unwrap_or(""),
        sorted_help_topics().join(", ")
    )
}

fn sorted_help_topics() -> Vec<&'static str> {
    let mut topics = MACSYMA_HELP_TOPICS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    topics.sort_unstable();
    topics
}

/// One evaluated MACSYMA statement.
#[derive(Debug, Clone, PartialEq)]
pub struct EvalResult {
    /// The 1-based input index, matching MACSYMA's `%iN` convention.
    pub input_index: usize,
    /// The 1-based output index, matching MACSYMA's `%oN` convention.
    pub output_index: usize,
    /// The compiled input expression with display/suppress wrapper removed.
    pub input: IRNode,
    /// The evaluated output expression.
    pub output: IRNode,
    /// Presentation text selected by MACSYMA display flags.
    pub output_text: String,
    /// Whether this statement should be shown by a REPL. `;` displays, `$`
    /// suppresses.
    pub display: bool,
    /// Optional wall-clock timing diagnostic emitted when `showtime` is true.
    pub timing_text: Option<String>,
}

/// In-memory `%i`/`%o` history for a single runtime session.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct History {
    inputs: Vec<IRNode>,
    outputs: Vec<IRNode>,
}

impl History {
    pub fn record_input(&mut self, node: IRNode) -> usize {
        self.inputs.push(node);
        self.inputs.len()
    }

    pub fn record_output(&mut self, node: IRNode) -> usize {
        self.outputs.push(node);
        self.outputs.len()
    }

    pub fn get_input(&self, index: usize) -> Option<&IRNode> {
        index.checked_sub(1).and_then(|idx| self.inputs.get(idx))
    }

    pub fn get_output(&self, index: usize) -> Option<&IRNode> {
        index.checked_sub(1).and_then(|idx| self.outputs.get(idx))
    }

    pub fn last_output(&self) -> Option<&IRNode> {
        self.outputs.last()
    }

    pub fn resolve_history_symbol(&self, name: &str) -> Option<&IRNode> {
        if name == "%" {
            return self.last_output();
        }

        if let Some(digits) = name.strip_prefix("%i") {
            let index = parse_history_index(digits)?;
            return self.get_input(index);
        }
        if let Some(digits) = name.strip_prefix("%o") {
            let index = parse_history_index(digits)?;
            return self.get_output(index);
        }
        None
    }

    pub fn next_input_index(&self) -> usize {
        self.inputs.len() + 1
    }

    pub fn inputs(&self) -> &[IRNode] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[IRNode] {
        &self.outputs
    }

    pub fn reset(&mut self) {
        self.inputs.clear();
        self.outputs.clear();
    }
}

fn parse_history_index(digits: &str) -> Option<usize> {
    if digits.is_empty() {
        return None;
    }
    digits.parse::<usize>().ok()
}

#[derive(Debug, Clone)]
struct MacsymaBackendState {
    env: HashMap<String, IRNode>,
    assumptions: AssumptionContext,
    showtime: bool,
}

impl MacsymaBackendState {
    fn new() -> Self {
        Self {
            env: initial_env(),
            assumptions: AssumptionContext::new(),
            showtime: false,
        }
    }

    fn unbind(&mut self, name: &str) {
        if name == SHOWTIME {
            self.showtime = false;
            self.env.insert(SHOWTIME.to_string(), sym(FALSE_SYMBOL));
        } else {
            self.env.remove(name);
        }
    }

    fn reset_environment(&mut self) {
        self.env = initial_env();
        self.showtime = false;
    }

    fn bind(&mut self, name: &str, value: IRNode) {
        if name == SHOWTIME {
            self.showtime = matches_symbol(&value, TRUE_SYMBOL);
        }
        self.env.insert(name.to_string(), value);
    }

    fn showtime(&self) -> bool {
        self.showtime
    }
}

fn initial_env() -> HashMap<String, IRNode> {
    let mut env = HashMap::new();
    env.insert(TRUE_SYMBOL.to_string(), sym(TRUE_SYMBOL));
    env.insert(FALSE_SYMBOL.to_string(), sym(FALSE_SYMBOL));
    env.insert(SHOWTIME.to_string(), sym(FALSE_SYMBOL));
    env.insert("%pi".to_string(), IRNode::Float(std::f64::consts::PI));
    env.insert("%e".to_string(), IRNode::Float(std::f64::consts::E));
    env.insert("%i".to_string(), sym("ImaginaryUnit"));
    env.insert("ImaginaryUnit".to_string(), sym("ImaginaryUnit"));
    env
}

/// MACSYMA-flavored symbolic backend.
///
/// It owns the runtime binding table and runtime/CAS handlers while preserving
/// the symbolic VM fallback behavior: unbound names and unknown heads remain
/// unevaluated IR instead of panicking.
pub struct MacsymaBackend {
    state: Arc<Mutex<MacsymaBackendState>>,
    handlers: HashMap<String, Handler>,
    held: HashSet<String>,
}

impl MacsymaBackend {
    fn new(state: Arc<Mutex<MacsymaBackendState>>) -> Self {
        let mut handlers = build_handler_table(true);
        handlers.insert(DISPLAY_HEAD.to_string(), handler_fn(display_handler));
        handlers.insert(SUPPRESS_HEAD.to_string(), handler_fn(suppress_handler));
        handlers.insert(EV.to_string(), handler_fn(ev_handler));
        handlers.insert(FLOAT_FUNC.to_string(), handler_fn(float_handler));
        handlers.insert(SOLVE.to_string(), handler_fn(solve_handler));
        handlers.insert(SIMPLIFY.to_string(), handler_fn(simplify_handler));
        handlers.insert(SUBST.to_string(), handler_fn(subst_handler));
        handlers.insert(RAT_SIMPLIFY.to_string(), handler_fn(simplify_handler));
        handlers.insert(TRIG_SIMPLIFY.to_string(), handler_fn(trig_simplify_handler));
        handlers.insert(TRIG_EXPAND.to_string(), handler_fn(trig_expand_handler));
        handlers.insert(TRIG_REDUCE.to_string(), handler_fn(trig_reduce_handler));

        let assume_state = state.clone();
        handlers.insert(
            ASSUME.to_string(),
            Arc::new(move |_vm, expr| assume_handler(&assume_state, expr)),
        );
        let forget_state = state.clone();
        handlers.insert(
            FORGET.to_string(),
            Arc::new(move |_vm, expr| forget_handler(&forget_state, expr)),
        );
        let is_state = state.clone();
        handlers.insert(
            IS.to_string(),
            Arc::new(move |_vm, expr| is_handler(&is_state, expr)),
        );
        let declare_state = state.clone();
        handlers.insert(
            DECLARE.to_string(),
            Arc::new(move |_vm, expr| declare_handler(&declare_state, expr)),
        );
        let properties_state = state.clone();
        handlers.insert(
            PROPERTIES.to_string(),
            Arc::new(move |_vm, expr| properties_handler(&properties_state, expr)),
        );
        let propvars_state = state.clone();
        handlers.insert(
            PROP_VARS.to_string(),
            Arc::new(move |_vm, expr| propvars_handler(&propvars_state, expr)),
        );
        handlers.insert(
            LENGTH.to_string(),
            list_handler(Some(1), |args| length(&args[0])),
        );
        handlers.insert(
            FIRST.to_string(),
            list_handler(Some(1), |args| first(&args[0])),
        );
        handlers.insert(
            REST.to_string(),
            list_handler(Some(1), |args| rest(&args[0])),
        );
        handlers.insert(
            LAST.to_string(),
            list_handler(Some(1), |args| last(&args[0])),
        );
        handlers.insert(
            REVERSE.to_string(),
            list_handler(Some(1), |args| reverse(&args[0])),
        );
        handlers.insert(APPEND.to_string(), list_handler(None, append));
        handlers.insert(JOIN.to_string(), list_handler(None, join));
        handlers.insert(RANGE.to_string(), list_handler(None, range_handler));
        handlers.insert(
            MAP.to_string(),
            list_handler(Some(2), |args| list_map(args[0].clone(), &args[1])),
        );
        handlers.insert(
            APPLY_HEAD.to_string(),
            list_handler(Some(2), |args| list_apply(args[0].clone(), &args[1])),
        );
        handlers.insert(
            SORT.to_string(),
            list_handler(Some(1), |args| sort_(&args[0])),
        );
        handlers.insert(
            PART.to_string(),
            list_handler(Some(2), |args| part(&args[0], integer_argument(&args[1])?)),
        );
        handlers.insert(FLATTEN.to_string(), list_handler(None, flatten_handler));

        let kill_state = state.clone();
        handlers.insert(
            KILL.to_string(),
            Arc::new(move |_vm, expr| {
                apply_kill_to_state(&kill_state, &expr.args);
                sym(DONE)
            }),
        );

        let held = [
            ASSIGN, DEFINE, IF, KILL, EV, ASSUME, FORGET, IS, DECLARE, PROPERTIES, PROP_VARS,
            SOLVE, SUBST,
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        Self {
            state,
            handlers,
            held,
        }
    }
}

impl Backend for MacsymaBackend {
    fn lookup(&self, name: &str) -> Option<IRNode> {
        self.state
            .lock()
            .expect("macsyma backend state poisoned")
            .env
            .get(name)
            .cloned()
    }

    fn bind(&mut self, name: &str, value: IRNode) {
        self.state
            .lock()
            .expect("macsyma backend state poisoned")
            .bind(name, value);
    }

    fn on_unresolved(&self, name: &str) -> IRNode {
        sym(name)
    }

    fn on_unknown_head(&self, expr: IRApply) -> IRNode {
        IRNode::Apply(Box::new(expr))
    }

    fn handler_for(&self, name: &str) -> Option<&Handler> {
        self.handlers.get(name)
    }

    fn hold_heads(&self) -> &HashSet<String> {
        &self.held
    }
}

/// Stateful MACSYMA evaluator over the Rust symbolic VM.
pub struct MacsymaSession {
    vm: VM,
    history: History,
    backend_state: Arc<Mutex<MacsymaBackendState>>,
}

impl MacsymaSession {
    pub fn new() -> Self {
        let backend_state = Arc::new(Mutex::new(MacsymaBackendState::new()));
        let backend = MacsymaBackend::new(backend_state.clone());
        Self {
            vm: VM::new(Box::new(backend)),
            history: History::default(),
            backend_state,
        }
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    /// Compile and evaluate every statement in `source`.
    pub fn eval_source(&mut self, source: &str) -> Result<Vec<EvalResult>, CompileError> {
        if let Some(topic) = parse_macsyma_help_query(source) {
            return Ok(vec![self.eval_help_query(&topic)]);
        }
        let statements = compile_macsyma_with_options(
            source,
            CompileOptions {
                wrap_terminators: true,
            },
        )?;
        Ok(self.eval_statements(statements))
    }

    /// Evaluate already-compiled statements. Display wrappers are honored when
    /// present; unwrapped statements default to display=true.
    pub fn eval_statements(&mut self, statements: Vec<IRNode>) -> Vec<EvalResult> {
        statements
            .into_iter()
            .map(|statement| self.eval_statement(statement))
            .collect()
    }

    fn eval_statement(&mut self, statement: IRNode) -> EvalResult {
        let (input, display) = unwrap_display(canonicalize_surface_names(statement));
        let show_timing = self
            .backend_state
            .lock()
            .expect("macsyma backend state poisoned")
            .showtime()
            && !is_showtime_assignment(&input);
        let started_at = Instant::now();
        let input_index = self.history.record_input(input.clone());
        let resolved_input = resolve_session_references(&self.history, input.clone());
        let kill_all = is_kill_all(&input);
        let output = self.vm.eval(resolved_input);
        let elapsed = started_at.elapsed();
        let output_index = self.history.record_output(output.clone());
        let output_text = display_text_for(&input, &output);
        if kill_all {
            self.history.reset();
        }
        EvalResult {
            input_index,
            output_index,
            input,
            output,
            output_text,
            display,
            timing_text: show_timing.then(|| format_timing(elapsed.as_secs_f64())),
        }
    }

    fn eval_help_query(&mut self, topic: &str) -> EvalResult {
        let query = if topic.is_empty() {
            "?".to_string()
        } else {
            format!("? {topic}")
        };
        let text = macsyma_help_text(Some(topic));
        let input = str_node(query);
        let output = str_node(text.clone());
        let input_index = self.history.record_input(input.clone());
        let output_index = self.history.record_output(output.clone());
        EvalResult {
            input_index,
            output_index,
            input,
            output,
            output_text: text,
            display: true,
            timing_text: None,
        }
    }
}

impl Default for MacsymaSession {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_session_references(history: &History, node: IRNode) -> IRNode {
    match node {
        IRNode::Symbol(name) => history
            .resolve_history_symbol(&name)
            .cloned()
            .unwrap_or(IRNode::Symbol(name)),
        IRNode::Apply(apply) => {
            let IRApply { head, args } = *apply;
            if is_head_name(&head, KILL) {
                return IRNode::Apply(Box::new(IRApply { head, args }));
            }
            IRNode::Apply(Box::new(IRApply {
                head: resolve_session_references(history, head),
                args: args
                    .into_iter()
                    .map(|arg| resolve_session_references(history, arg))
                    .collect(),
            }))
        }
        other => other,
    }
}

fn canonicalize_surface_names(node: IRNode) -> IRNode {
    let table = macsyma_name_table();
    canonicalize_surface_names_with(&table, node)
}

fn canonicalize_surface_names_with(table: &HashMap<String, String>, node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(apply) => {
            let IRApply { head, args } = *apply;
            let head = match head {
                IRNode::Symbol(name) => table
                    .get(&name)
                    .map(|canonical| sym(canonical.as_str()))
                    .unwrap_or(IRNode::Symbol(name)),
                other => canonicalize_surface_names_with(table, other),
            };
            IRNode::Apply(Box::new(IRApply {
                head,
                args: args
                    .into_iter()
                    .map(|arg| canonicalize_surface_names_with(table, arg))
                    .collect(),
            }))
        }
        other => other,
    }
}

fn unwrap_display(statement: IRNode) -> (IRNode, bool) {
    if let IRNode::Apply(apply) = statement {
        if apply.args.len() == 1 {
            if let IRNode::Symbol(head) = &apply.head {
                if head == DISPLAY {
                    return (apply.args[0].clone(), true);
                }
                if head == SUPPRESS {
                    return (apply.args[0].clone(), false);
                }
            }
        }
        return (IRNode::Apply(apply), true);
    }
    (statement, true)
}

fn display_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() == 1 {
        expr.args[0].clone()
    } else {
        IRNode::Apply(Box::new(expr))
    }
}

fn suppress_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    display_handler(_vm, expr)
}

fn assume_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    let mut state = state.lock().expect("macsyma backend state poisoned");
    match expr.args.as_slice() {
        [relation] => state.assumptions.assume_relation(relation),
        [symbol, property] => state.assumptions.assume_property(symbol, property),
        _ => {}
    }
    sym(DONE)
}

fn forget_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    let mut state = state.lock().expect("macsyma backend state poisoned");
    if expr.args.is_empty() {
        state.assumptions.forget_all();
    } else {
        state.assumptions.forget_relation(&expr.args[0]);
    }
    sym(DONE)
}

fn is_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let state = state.lock().expect("macsyma backend state poisoned");
    match state.assumptions.is_true_relation(&expr.args[0]) {
        Some(true) => sym("True"),
        Some(false) => sym("False"),
        None => sym("unknown"),
    }
}

fn declare_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if expr.args.len() % 2 != 0 {
        return IRNode::Apply(Box::new(expr));
    }
    let mut state = state.lock().expect("macsyma backend state poisoned");
    for pair in expr.args.chunks_exact(2) {
        state.assumptions.assume_property(&pair[0], &pair[1]);
    }
    sym(DONE)
}

fn properties_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let Some(name) = symbol_name(&expr.args[0]) else {
        return apply(sym(LIST), vec![]);
    };
    let state = state.lock().expect("macsyma backend state poisoned");
    apply(
        sym(LIST),
        state
            .assumptions
            .facts_for(name)
            .into_iter()
            .map(sym)
            .collect(),
    )
}

fn propvars_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if !expr.args.is_empty() {
        return IRNode::Apply(Box::new(expr));
    }
    let state = state.lock().expect("macsyma backend state poisoned");
    apply(
        sym(LIST),
        state
            .assumptions
            .symbols_with_facts()
            .into_iter()
            .map(|name| sym(&name))
            .collect(),
    )
}

fn ev_handler(vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.is_empty() {
        return IRNode::Apply(Box::new(expr));
    }

    let flags: HashSet<String> = expr.args[1..]
        .iter()
        .filter_map(symbol_name)
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let mut result = vm.eval(expr.args[0].clone());

    if flags.contains("numer") || flags.contains("float") {
        result = numer_fold(result);
    }

    if flags.contains("expand") {
        result = vm.eval(apply(sym(EXPAND), vec![result]));
    }
    if flags.contains("factor") {
        result = vm.eval(apply(sym(FACTOR), vec![result]));
    }
    if flags.contains("ratsimp") {
        result = vm.eval(apply(sym(RAT_SIMPLIFY), vec![result]));
    }
    if flags.contains("trigsimp") {
        result = vm.eval(apply(sym(TRIG_SIMPLIFY), vec![result]));
    }
    if flags.contains("trigreduce") {
        result = vm.eval(apply(sym(TRIG_REDUCE), vec![result]));
    }

    result
}

fn float_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    numer_fold(expr.args[0].clone())
}

fn simplify_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    simplify(expr.args[0].clone(), 50)
}

fn trig_simplify_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    trig_simplify(&expr.args[0])
}

fn trig_expand_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    expand_trig(&expr.args[0])
}

fn trig_reduce_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    trig_reduce(&expr.args[0])
}

fn solve_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    let fallback = IRNode::Apply(Box::new(expr.clone()));
    if expr.args.len() != 2 {
        return fallback;
    }

    if matches!(expr.args[1], IRNode::Symbol(_)) && is_inequality(&expr.args[0]) {
        return try_solve_inequality(&expr.args[0], &expr.args[1])
            .map(|solutions| apply(sym(LIST), solutions))
            .unwrap_or(fallback);
    }

    if matches!(expr.args[1], IRNode::Symbol(_)) {
        return try_solve_transcendental(&expr.args[0], &expr.args[1])
            .map(|solutions| apply(sym(LIST), solutions))
            .unwrap_or(fallback);
    }

    let Some(equations) = apply_with_head(&expr.args[0], LIST) else {
        return fallback;
    };
    let Some(variables) = apply_with_head(&expr.args[1], LIST) else {
        return fallback;
    };
    if !variables
        .args
        .iter()
        .all(|variable| matches!(variable, IRNode::Symbol(_)))
    {
        return fallback;
    }

    solve_linear_system(&equations.args, &variables.args)
        .map(|rules| apply(sym(LIST), rules))
        .unwrap_or(fallback)
}

fn subst_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    let fallback = IRNode::Apply(Box::new(expr.clone()));
    if expr.args.len() != 3 {
        return fallback;
    }
    subst(expr.args[0].clone(), &expr.args[1], expr.args[2].clone())
}

fn list_handler<F>(arity: Option<usize>, body: F) -> Handler
where
    F: Fn(&[IRNode]) -> ListResult<IRNode> + Send + Sync + 'static,
{
    Arc::new(move |_vm, expr| {
        let fallback = IRNode::Apply(Box::new(expr.clone()));
        if arity.is_some_and(|expected| expr.args.len() != expected) {
            return fallback;
        }
        body(&expr.args).unwrap_or(fallback)
    })
}

fn range_handler(args: &[IRNode]) -> ListResult<IRNode> {
    if !(1..=3).contains(&args.len()) {
        return Err(ListOperationError("Range takes 1 to 3 arguments".into()));
    }
    let start = integer_argument(&args[0])?;
    let stop = if args.len() >= 2 {
        Some(integer_argument(&args[1])?)
    } else {
        None
    };
    let step = if args.len() >= 3 {
        integer_argument(&args[2])?
    } else {
        1
    };
    list_range(start, stop, step)
}

fn flatten_handler(args: &[IRNode]) -> ListResult<IRNode> {
    if !(1..=2).contains(&args.len()) {
        return Err(ListOperationError("Flatten takes 1 or 2 arguments".into()));
    }
    let depth = if args.len() == 2 {
        integer_argument(&args[1])?
    } else {
        1
    };
    flatten(&args[0], depth)
}

fn integer_argument(node: &IRNode) -> ListResult<i64> {
    match node {
        IRNode::Integer(value) => Ok(*value),
        _ => Err(ListOperationError("expected integer argument".into())),
    }
}

fn is_inequality(node: &IRNode) -> bool {
    match node {
        IRNode::Apply(apply) => {
            is_head_name(&apply.head, LESS)
                || is_head_name(&apply.head, GREATER)
                || is_head_name(&apply.head, LESS_EQUAL)
                || is_head_name(&apply.head, GREATER_EQUAL)
        }
        _ => false,
    }
}

fn numer_fold(node: IRNode) -> IRNode {
    match node {
        IRNode::Integer(n) => IRNode::Float(n as f64),
        IRNode::Rational(n, d) => IRNode::Float(n as f64 / d as f64),
        IRNode::Apply(apply) => {
            let IRApply { head, args } = *apply;
            if is_head_name(&head, POW) && args.len() == 2 {
                return apply_node(head, vec![numer_fold(args[0].clone()), args[1].clone()]);
            }
            apply_node(head, args.into_iter().map(numer_fold).collect())
        }
        other => other,
    }
}

fn display_text_for(input: &IRNode, output: &IRNode) -> String {
    if has_ev_flag(input, "display2d") {
        pretty_2d(output, &MacsymaDialect)
    } else {
        pretty(output, &MacsymaDialect)
    }
}

fn has_ev_flag(input: &IRNode, flag: &str) -> bool {
    match input {
        IRNode::Apply(apply) if is_head_name(&apply.head, EV) => apply
            .args
            .iter()
            .skip(1)
            .filter_map(symbol_name)
            .any(|name| name.eq_ignore_ascii_case(flag)),
        _ => false,
    }
}

fn apply_kill_to_state(state: &Arc<Mutex<MacsymaBackendState>>, args: &[IRNode]) {
    let mut state = state.lock().expect("macsyma backend state poisoned");
    for arg in args {
        if let Some(name) = symbol_name(arg) {
            if name == ALL {
                state.reset_environment();
            } else {
                state.unbind(name);
            }
        }
    }
}

fn is_kill_all(node: &IRNode) -> bool {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, KILL) => apply
            .args
            .iter()
            .any(|arg| symbol_name(arg).is_some_and(|name| name == ALL)),
        _ => false,
    }
}

fn is_showtime_assignment(node: &IRNode) -> bool {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, ASSIGN) && apply.args.len() == 2 => {
            matches_symbol(&apply.args[0], SHOWTIME)
        }
        _ => false,
    }
}

fn format_timing(elapsed_seconds: f64) -> String {
    format!("Evaluation took {elapsed_seconds:.6} seconds.")
}

fn is_head_name(head: &IRNode, expected: &str) -> bool {
    symbol_name(head).is_some_and(|name| name == expected)
}

fn matches_symbol(node: &IRNode, expected: &str) -> bool {
    symbol_name(node).is_some_and(|name| name == expected)
}

fn symbol_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}

fn apply_with_head<'a>(node: &'a IRNode, expected: &str) -> Option<&'a IRApply> {
    match node {
        IRNode::Apply(apply) if is_head_name(&apply.head, expected) => Some(apply),
        _ => None,
    }
}

fn apply_node(head: IRNode, args: Vec<IRNode>) -> IRNode {
    IRNode::Apply(Box::new(IRApply { head, args }))
}

//! MACSYMA runtime session facade.
//!
//! This crate is the Rust runtime layer above the grammar-driven MACSYMA
//! compiler. It keeps the public API pure and WASM-friendly: callers pass
//! source strings in and receive evaluated IR nodes plus display metadata.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use cas_simplify::simplify;
use cas_solve::{solve_linear_system, try_solve_inequality, SOLVE};
use cas_trig::{expand_trig, trig_simplify};
use coding_adventures_macsyma_compiler::{
    compile_macsyma_with_options, CompileError, CompileOptions, DISPLAY as COMPILER_DISPLAY,
    SUPPRESS as COMPILER_SUPPRESS,
};
use symbolic_ir::{
    apply, sym, IRApply, IRNode, ASSIGN, DEFINE, GREATER, GREATER_EQUAL, IF, LESS, LESS_EQUAL,
    LIST, POW,
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

const DONE: &str = "done";
const EXPAND: &str = "Expand";
const FACTOR: &str = "Factor";
const FLOAT_FUNC: &str = "Float";
const RAT_SIMPLIFY: &str = "RatSimplify";
const SIMPLIFY: &str = "Simplify";
const TRIG_EXPAND: &str = "TrigExpand";
const TRIG_SIMPLIFY: &str = "TrigSimplify";

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
    ("subst", "Subst"),
    ("simplify", SIMPLIFY),
    ("expand", EXPAND),
    ("factor", FACTOR),
    ("solve", "Solve"),
    ("nsolve", "NSolve"),
    ("linsolve", "Solve"),
    ("taylor", "Taylor"),
    ("limit", "Limit"),
    ("length", "Length"),
    ("first", "First"),
    ("rest", "Rest"),
    ("last", "Last"),
    ("append", "Append"),
    ("reverse", "Reverse"),
    ("makelist", "MakeList"),
    ("map", "Map"),
    ("apply", "Apply"),
    ("sublist", "Select"),
    ("sort", "Sort"),
    ("part", "Part"),
    ("flatten", "Flatten"),
    ("join", "Join"),
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
    ("trigreduce", "TrigReduce"),
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
    /// Whether this statement should be shown by a REPL. `;` displays, `$`
    /// suppresses.
    pub display: bool,
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
}

impl MacsymaBackendState {
    fn new() -> Self {
        Self { env: initial_env() }
    }

    fn unbind(&mut self, name: &str) {
        self.env.remove(name);
    }

    fn reset_environment(&mut self) {
        self.env = initial_env();
    }
}

fn initial_env() -> HashMap<String, IRNode> {
    let mut env = HashMap::new();
    env.insert("True".to_string(), sym("True"));
    env.insert("False".to_string(), sym("False"));
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
        handlers.insert(RAT_SIMPLIFY.to_string(), handler_fn(simplify_handler));
        handlers.insert(TRIG_SIMPLIFY.to_string(), handler_fn(trig_simplify_handler));
        handlers.insert(TRIG_EXPAND.to_string(), handler_fn(trig_expand_handler));

        let kill_state = state.clone();
        handlers.insert(
            KILL.to_string(),
            Arc::new(move |_vm, expr| {
                apply_kill_to_state(&kill_state, &expr.args);
                sym(DONE)
            }),
        );

        let held = [ASSIGN, DEFINE, IF, KILL, EV, SOLVE]
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
            .env
            .insert(name.to_string(), value);
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
}

impl MacsymaSession {
    pub fn new() -> Self {
        let backend_state = Arc::new(Mutex::new(MacsymaBackendState::new()));
        let backend = MacsymaBackend::new(backend_state.clone());
        Self {
            vm: VM::new(Box::new(backend)),
            history: History::default(),
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
        let input_index = self.history.record_input(input.clone());
        let resolved_input = resolve_session_references(&self.history, input.clone());
        let kill_all = is_kill_all(&input);
        let output = self.vm.eval(resolved_input);
        let output_index = self.history.record_output(output.clone());
        if kill_all {
            self.history.reset();
        }
        EvalResult {
            input_index,
            output_index,
            input,
            output,
            display,
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

fn is_head_name(head: &IRNode, expected: &str) -> bool {
    symbol_name(head).is_some_and(|name| name == expected)
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

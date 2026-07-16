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
use cas_simplify::{expand, radcan, simplify, AssumptionContext};
use cas_solve::{solve_linear_system, try_solve_inequality, try_solve_transcendental, SOLVE};
use cas_substitution::subst;
use cas_trig::{expand_trig, trig_reduce, trig_simplify};
use coding_adventures_macsyma_compiler::{
    compile_macsyma_with_options, CompileError, CompileOptions, DISPLAY as COMPILER_DISPLAY,
    SUPPRESS as COMPILER_SUPPRESS,
};
use symbolic_ir::{
    apply, flt, int, rat, str_node, sym, IRApply, IRNode, ASSIGN, BESSEL_J, BESSEL_Y, CHEBYSHEV_T,
    CHEBYSHEV_U, DEFINE, DIV, EXP, GREATER, GREATER_EQUAL, HERMITE_H, IF, LEGENDRE_P, LEGENDRE_Q,
    LESS, LESS_EQUAL, LIST, LOG, MUL, NEG, POW, SQRT, SUB,
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
/// Runtime-owned head for the `load("name")` package directive.
///
/// Track M2 — flips a per-backend gate so the gated orthopoly evaluator
/// handlers can fire.  The list of allowed package names is the
/// compile-time-constant `LOAD_ALLOWLIST` below; the load handler has
/// exactly one dispatch arm per allowed name and never turns a
/// user-supplied string into a module reference, import path, or
/// callable.
pub const LOAD: &str = "Load";

/// Compile-time-constant allowlist consulted by the `Load` handler.
///
/// Adding a new entry is a deliberate two-line change: append to this
/// slice AND add a matching `match` arm in `make_load_handler` below.
/// The two are kept side-by-side so an audit can verify the second
/// when it sees the first.
const LOAD_ALLOWLIST: &[&str] = &["orthopoly"];

/// Name of the orthopoly package — referenced in handlers as a string
/// constant rather than re-typing the literal, so a `&[&str]` audit
/// surfaces every site that depends on it.
const ORTHOPOLY_NAME: &str = "orthopoly";

/// A MACSYMA-surface error meant to be returned verbatim to the user.
///
/// Distinguished from the runtime's broader `CompileError` so the REPL
/// can format it without leaking a Rust stack trace.  Carries a single
/// owned message string; this mirrors `MacsymaUserError` on Python.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacsymaUserError(pub String);

impl std::fmt::Display for MacsymaUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MacsymaUserError {}

const ASSUME: &str = "Assume";
const DONE: &str = "done";
const EXPAND: &str = "Expand";
const FACTOR: &str = "Factor";
const FLOAT_FUNC: &str = "Float";
const FORGET: &str = "Forget";
const IS: &str = "Is";
const RAT_SIMPLIFY: &str = "RatSimplify";
const RADCAN: &str = "Radcan";
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
    // ---------------------------------------------------------------
    // Track M2 — runtime package loader and orthogonal polynomials.
    // ---------------------------------------------------------------
    //
    // `load("orthopoly")` is the session-level directive that turns
    // on the orthogonal polynomial closed-form evaluators (see the
    // gated handlers registered on `MacsymaBackend`).  Until that
    // call the names below parse to their canonical IR head but the
    // gated handler returns the expression unevaluated, matching the
    // Python runtime's surface contract (Track M1).
    ("load", LOAD),
    ("legendre_p", LEGENDRE_P),
    ("legendre_q", LEGENDRE_Q),
    ("chebyshev_t", CHEBYSHEV_T),
    ("chebyshev_u", CHEBYSHEV_U),
    ("hermite", HERMITE_H),
    ("bessel_j", BESSEL_J),
    ("bessel_y", BESSEL_Y),
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
    /// Track M2 — names that have been loaded via `load("name")`.
    ///
    /// Per-session: each backend owns its own set, so two parallel
    /// sessions stay independent.  The orthopoly evaluator handlers
    /// consult this set on every dispatch; if `"orthopoly"` is missing
    /// they return the expression unevaluated, observationally
    /// identical to the no-handler-registered path before `load`.
    loaded_packages: HashSet<String>,
}

impl MacsymaBackendState {
    fn new() -> Self {
        Self {
            env: initial_env(),
            assumptions: AssumptionContext::new(),
            showtime: false,
            loaded_packages: HashSet::new(),
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
        handlers.insert(EXPAND.to_string(), handler_fn(expand_handler));
        handlers.insert(SUBST.to_string(), handler_fn(subst_handler));
        handlers.insert(RAT_SIMPLIFY.to_string(), handler_fn(simplify_handler));
        let radcan_state = state.clone();
        handlers.insert(
            RADCAN.to_string(),
            Arc::new(move |_vm, expr| radcan_handler(&radcan_state, expr)),
        );
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
        let abs_state = state.clone();
        handlers.insert(
            "Abs".to_string(),
            Arc::new(move |vm, expr| runtime_abs_handler(&abs_state, vm, expr)),
        );
        let sqrt_state = state.clone();
        handlers.insert(
            SQRT.to_string(),
            Arc::new(move |vm, expr| runtime_sqrt_handler(&sqrt_state, vm, expr)),
        );
        let log_state = state.clone();
        handlers.insert(
            LOG.to_string(),
            Arc::new(move |vm, expr| runtime_log_handler(&log_state, vm, expr)),
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

        // Track M2 — `load("name")` directive.
        //
        // Static dispatch via a `match` arm per allowlist entry.  No
        // dynamic library loading, no path resolution, no FFI — the
        // user-supplied name only flows into a comparison against
        // `LOAD_ALLOWLIST` and then through the match.
        let load_state = state.clone();
        handlers.insert(
            LOAD.to_string(),
            Arc::new(move |_vm, expr| run_load_handler(&load_state, expr)),
        );

        // Track M2 — orthopoly evaluator stubs.
        //
        // The handlers are *always* installed, but each one consults
        // `loaded_packages` before doing any work.  Until
        // `load("orthopoly")` fires they return the expression
        // unevaluated, matching the pre-`load` round-trip behaviour
        // the Python runtime gets by leaving the heads unregistered.
        let legendre_p_state = state.clone();
        handlers.insert(
            LEGENDRE_P.to_string(),
            Arc::new(move |vm, expr| {
                run_orthopoly_recurrence(&legendre_p_state, vm, expr, legendre_p_recurrence)
            }),
        );
        let chebyshev_t_state = state.clone();
        handlers.insert(
            CHEBYSHEV_T.to_string(),
            Arc::new(move |vm, expr| {
                run_orthopoly_recurrence(&chebyshev_t_state, vm, expr, chebyshev_t_recurrence)
            }),
        );
        let chebyshev_u_state = state.clone();
        handlers.insert(
            CHEBYSHEV_U.to_string(),
            Arc::new(move |vm, expr| {
                run_orthopoly_recurrence(&chebyshev_u_state, vm, expr, chebyshev_u_recurrence)
            }),
        );
        let hermite_h_state = state.clone();
        handlers.insert(
            HERMITE_H.to_string(),
            Arc::new(move |vm, expr| {
                run_orthopoly_recurrence(&hermite_h_state, vm, expr, hermite_h_recurrence)
            }),
        );
        let legendre_q_state = state.clone();
        handlers.insert(
            LEGENDRE_Q.to_string(),
            Arc::new(move |_vm, expr| run_orthopoly_passthrough(&legendre_q_state, expr)),
        );
        let bessel_j_state = state.clone();
        handlers.insert(
            BESSEL_J.to_string(),
            Arc::new(move |_vm, expr| run_orthopoly_passthrough(&bessel_j_state, expr)),
        );
        let bessel_y_state = state.clone();
        handlers.insert(
            BESSEL_Y.to_string(),
            Arc::new(move |_vm, expr| run_orthopoly_passthrough(&bessel_y_state, expr)),
        );

        let held = [
            ASSIGN, DEFINE, IF, KILL, EV, ASSUME, FORGET, IS, DECLARE, PROPERTIES, PROP_VARS,
            SOLVE, SUBST, RADCAN,
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

/// `Load("name")` directive — flip the per-package gate on the
/// backend's `loaded_packages` set.
///
/// Mirrors `make_load_handler` on the Python runtime:
///
///   1. Validates arity (exactly one string-or-symbol argument).
///   2. Checks the name against the compile-time `LOAD_ALLOWLIST`.
///      Unknown names panic with `MacsymaUserError` carrying a
///      clear message that advertises what *is* available.
///   3. Returns early (idempotent) if the package is already loaded.
///   4. Otherwise flips the package gate via a *static* `match` arm.
///      There is no dynamic library lookup, so a hostile name string
///      can never be turned into an executable code path.
///
/// The return value is the same name the user passed (wrapped as an
/// `IRNode::String`) so `load("orthopoly")` prints cleanly in the
/// REPL — matching Maxima's "return the path" convention.
fn run_load_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        panic!(
            "{}",
            MacsymaUserError(format!(
                "load takes 1 argument, got {}",
                expr.args.len()
            ))
        );
    }
    // Accept the Maxima short form `load(orthopoly)` (bare symbol) as
    // well as `load("orthopoly")` (string).  The symbol's name is
    // pulled out by reference — never looked up in the environment,
    // so a hostile user cannot shadow `orthopoly` with a binding.
    let name: String = match &expr.args[0] {
        IRNode::Str(s) => s.clone(),
        IRNode::Symbol(s) => s.clone(),
        _ => panic!(
            "{}",
            MacsymaUserError("load: argument must be a string or symbol".to_string())
        ),
    };
    if !LOAD_ALLOWLIST.contains(&name.as_str()) {
        let mut allowed: Vec<&&str> = LOAD_ALLOWLIST.iter().collect();
        allowed.sort();
        let allowed_str = allowed
            .into_iter()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "{}",
            MacsymaUserError(format!(
                "load: unknown package '{name}'; available: {allowed_str}"
            ))
        );
    }
    {
        let mut guard = state.lock().expect("macsyma backend state poisoned");
        if guard.loaded_packages.contains(&name) {
            // Idempotent: already loaded, nothing to do.
            return str_node(name);
        }
        // Static dispatch — exactly one arm per allowlist entry.  No
        // dynamic library lookup, so the name string cannot escape
        // the match into a code path it wasn't statically permitted
        // to reach.
        match name.as_str() {
            "orthopoly" => {
                guard.loaded_packages.insert(ORTHOPOLY_NAME.to_string());
            }
            _ => panic!(
                "{}",
                MacsymaUserError(format!(
                    "load: internal error — '{name}' in allowlist but not dispatched"
                ))
            ),
        }
    }
    str_node(name)
}

/// Gated wrapper for an orthopoly closed-form recurrence.
///
/// When `loaded_packages` does not contain `"orthopoly"`, returns the
/// expression unevaluated — observationally identical to no handler
/// being installed.  Once loaded, dispatches into the supplied
/// `recurrence` if `(n, x)` parses out cleanly (non-negative integer
/// `n` plus arbitrary IR `x`); otherwise leaves the expression alone.
fn run_orthopoly_recurrence(
    state: &Arc<Mutex<MacsymaBackendState>>,
    vm: &mut VM,
    expr: IRApply,
    recurrence: fn(i64, &IRNode, &mut VM) -> IRNode,
) -> IRNode {
    if !state
        .lock()
        .expect("macsyma backend state poisoned")
        .loaded_packages
        .contains(ORTHOPOLY_NAME)
    {
        return IRNode::Apply(Box::new(expr));
    }
    if expr.args.len() != 2 {
        return IRNode::Apply(Box::new(expr));
    }
    let n = match expr.args[0] {
        IRNode::Integer(n) if n >= 0 => n,
        _ => return IRNode::Apply(Box::new(expr)),
    };
    let x_node = expr.args[1].clone();
    let result = recurrence(n, &x_node, vm);
    vm.eval(result)
}

/// Gated passthrough handler — used for `LegendreQ`, `BesselJ`,
/// `BesselY`.  After `load("orthopoly")` the runtime "knows" these
/// heads but has no closed-form reduction, so it returns the
/// expression unevaluated so downstream rewrites still see it.
fn run_orthopoly_passthrough(
    state: &Arc<Mutex<MacsymaBackendState>>,
    expr: IRApply,
) -> IRNode {
    let _gated = state
        .lock()
        .expect("macsyma backend state poisoned")
        .loaded_packages
        .contains(ORTHOPOLY_NAME);
    // Whether loaded or not, a passthrough returns the same thing.
    // Keeping the gate check around so the symbol is at least
    // "consulted" mirrors the recurrence handler's structure and
    // makes it obvious that the head wasn't accidentally treated as
    // unknown.
    let _ = _gated;
    IRNode::Apply(Box::new(expr))
}

// ---------------------------------------------------------------------
// Closed-form recurrences — IR-only, identical to Python `orthopoly.py`.
// ---------------------------------------------------------------------
//
// Each helper builds the IR tree symbolically; the VM's automatic
// simplifier (called via `vm.eval` in the gated wrapper above) folds
// the polynomial to canonical form.  Working entirely in IR rather
// than at host arithmetic means a free `x` symbol still yields a
// polynomial.

fn legendre_p_recurrence(n: i64, x: &IRNode, vm: &mut VM) -> IRNode {
    // Bonnet recursion: (k+1) P_{k+1} = (2k+1) x P_k − k P_{k−1}.
    if n == 0 {
        return int(1);
    }
    if n == 1 {
        return x.clone();
    }
    let mut p_prev: IRNode = int(1);
    let mut p_curr: IRNode = x.clone();
    for k in 1..n {
        let two_k_plus_one = int(2 * k + 1);
        let k_node = int(k);
        let k_plus_one = int(k + 1);
        let next = apply(
            sym(DIV),
            vec![
                apply(
                    sym(SUB),
                    vec![
                        apply(
                            sym(MUL),
                            vec![
                                two_k_plus_one,
                                apply(sym(MUL), vec![x.clone(), p_curr.clone()]),
                            ],
                        ),
                        apply(sym(MUL), vec![k_node, p_prev.clone()]),
                    ],
                ),
                k_plus_one,
            ],
        );
        p_prev = p_curr;
        p_curr = vm.eval(next);
    }
    p_curr
}

fn chebyshev_t_recurrence(n: i64, x: &IRNode, vm: &mut VM) -> IRNode {
    // T_{k+1} = 2x T_k − T_{k−1}, seeded T_0 = 1, T_1 = x.
    if n == 0 {
        return int(1);
    }
    if n == 1 {
        return x.clone();
    }
    let mut t_prev: IRNode = int(1);
    let mut t_curr: IRNode = x.clone();
    for _ in 1..n {
        let two_x = apply(sym(MUL), vec![int(2), x.clone()]);
        let next = apply(
            sym(SUB),
            vec![apply(sym(MUL), vec![two_x, t_curr.clone()]), t_prev.clone()],
        );
        t_prev = t_curr;
        t_curr = vm.eval(next);
    }
    t_curr
}

fn chebyshev_u_recurrence(n: i64, x: &IRNode, vm: &mut VM) -> IRNode {
    // U_{k+1} = 2x U_k − U_{k−1}, seeded U_0 = 1, U_1 = 2x.
    if n == 0 {
        return int(1);
    }
    let two_x = vm.eval(apply(sym(MUL), vec![int(2), x.clone()]));
    if n == 1 {
        return two_x;
    }
    let mut u_prev: IRNode = int(1);
    let mut u_curr: IRNode = two_x;
    for _ in 1..n {
        let two_x_factor = apply(sym(MUL), vec![int(2), x.clone()]);
        let next = apply(
            sym(SUB),
            vec![
                apply(sym(MUL), vec![two_x_factor, u_curr.clone()]),
                u_prev.clone(),
            ],
        );
        u_prev = u_curr;
        u_curr = vm.eval(next);
    }
    u_curr
}

fn hermite_h_recurrence(n: i64, x: &IRNode, vm: &mut VM) -> IRNode {
    // Physicists' Hermite: H_{k+1} = 2x H_k − 2k H_{k−1},
    // seeded H_0 = 1, H_1 = 2x.  Matches Maxima's `hermite`.
    if n == 0 {
        return int(1);
    }
    let two_x = vm.eval(apply(sym(MUL), vec![int(2), x.clone()]));
    if n == 1 {
        return two_x;
    }
    let mut h_prev: IRNode = int(1);
    let mut h_curr: IRNode = two_x;
    for k in 1..n {
        let two_x_factor = apply(sym(MUL), vec![int(2), x.clone()]);
        let two_k = int(2 * k);
        let next = apply(
            sym(SUB),
            vec![
                apply(sym(MUL), vec![two_x_factor, h_curr.clone()]),
                apply(sym(MUL), vec![two_k, h_prev.clone()]),
            ],
        );
        h_prev = h_curr;
        h_curr = vm.eval(next);
    }
    h_curr
}

fn runtime_abs_handler(
    state: &Arc<Mutex<MacsymaBackendState>>,
    vm: &mut VM,
    expr: IRApply,
) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let inner = vm.eval(expr.args[0].clone());
    if let Some(numeric) = abs_numeric_node(&inner) {
        return numeric;
    }
    if let IRNode::Symbol(name) = &inner {
        let assumptions = &state.lock().expect("macsyma backend state poisoned").assumptions;
        if assumptions.is_nonneg(name) == Some(true) {
            return inner;
        }
        if assumptions.sign_of(name) == Some(0) {
            return int(0);
        }
        if assumptions.is_negative(name) == Some(true) {
            return apply(sym(NEG), vec![inner]);
        }
    }
    if let IRNode::Apply(apply_node) = &inner {
        if apply_node.head == sym("Abs") {
            return inner;
        }
        if apply_node.head == sym(NEG) && apply_node.args.len() == 1 {
            return vm.eval(apply(sym("Abs"), vec![apply_node.args[0].clone()]));
        }
        if apply_node.head == sym(MUL)
            && apply_node.args.len() == 2
            && apply_node.args[0] == int(-1)
        {
            return vm.eval(apply(sym("Abs"), vec![apply_node.args[1].clone()]));
        }
        if apply_node.head == sym(POW) && apply_node.args.len() == 2 {
            if let IRNode::Integer(exp) = apply_node.args[1] {
                if exp >= 2 && exp % 2 == 0 {
                    return inner;
                }
            }
        }
    }
    apply(expr.head, vec![inner])
}

fn runtime_sqrt_handler(
    state: &Arc<Mutex<MacsymaBackendState>>,
    vm: &mut VM,
    expr: IRApply,
) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let arg = vm.eval(expr.args[0].clone());
    if let Some(numeric) = sqrt_numeric_node(&arg) {
        return numeric;
    }
    if let IRNode::Apply(apply_node) = &arg {
        if apply_node.head == sym(POW) && apply_node.args.len() == 2 {
            let base = &apply_node.args[0];
            if let IRNode::Integer(exp) = apply_node.args[1] {
                if exp > 0 && exp % 2 == 0 {
                    let k = exp / 2;
                    if k == 1 {
                        if let IRNode::Symbol(name) = base {
                            let assumptions =
                                &state.lock().expect("macsyma backend state poisoned").assumptions;
                            if assumptions.is_nonneg(name) == Some(true) {
                                return base.clone();
                            }
                        }
                    }
                    if k % 2 == 0 {
                        return apply(sym(POW), vec![base.clone(), int(k)]);
                    }
                    if k == 1 {
                        return apply(sym("Abs"), vec![base.clone()]);
                    }
                    return apply(
                        sym("Abs"),
                        vec![apply(sym(POW), vec![base.clone(), int(k)])],
                    );
                }
            }
        }
    }
    apply(expr.head, vec![arg])
}

fn runtime_log_handler(
    state: &Arc<Mutex<MacsymaBackendState>>,
    vm: &mut VM,
    expr: IRApply,
) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let arg = vm.eval(expr.args[0].clone());
    if let Some(numeric) = log_numeric_node(&arg) {
        return numeric;
    }
    if let IRNode::Apply(apply_node) = &arg {
        if apply_node.head == sym(EXP) && apply_node.args.len() == 1 {
            return apply_node.args[0].clone();
        }
        if apply_node.head == sym(POW) && apply_node.args.len() == 2 {
            let base = &apply_node.args[0];
            if let IRNode::Symbol(name) = base {
                let assumptions = &state.lock().expect("macsyma backend state poisoned").assumptions;
                if assumptions.is_nonneg(name) == Some(true) {
                    return apply(
                        sym(MUL),
                        vec![apply_node.args[1].clone(), apply(sym(LOG), vec![base.clone()])],
                    );
                }
            }
        }
    }
    apply(expr.head, vec![arg])
}

fn abs_numeric_node(node: &IRNode) -> Option<IRNode> {
    match node {
        IRNode::Integer(n) => Some(int(n.abs())),
        IRNode::Rational(n, d) => Some(rat(n.abs(), *d)),
        IRNode::Float(v) => Some(flt(v.abs())),
        _ => None,
    }
}

fn sqrt_numeric_node(node: &IRNode) -> Option<IRNode> {
    match node {
        IRNode::Integer(n) => {
            if *n < 0 {
                return None;
            }
            if let Some(root) = i64_sqrt_exact(*n) {
                Some(int(root))
            } else {
                Some(flt((*n as f64).sqrt()))
            }
        }
        IRNode::Rational(n, d) => {
            if *n < 0 {
                return None;
            }
            match (i64_sqrt_exact(*n), i64_sqrt_exact(*d)) {
                (Some(n_root), Some(d_root)) => Some(rat(n_root, d_root)),
                _ => Some(flt((*n as f64 / *d as f64).sqrt())),
            }
        }
        IRNode::Float(v) if *v >= 0.0 => Some(flt(v.sqrt())),
        _ => None,
    }
}

fn log_numeric_node(node: &IRNode) -> Option<IRNode> {
    let value = match node {
        IRNode::Integer(n) => *n as f64,
        IRNode::Rational(n, d) => *n as f64 / *d as f64,
        IRNode::Float(v) => *v,
        _ => return None,
    };
    if value == 1.0 {
        return Some(int(0));
    }
    if value <= 0.0 {
        return None;
    }
    Some(flt(value.ln()))
}

fn i64_sqrt_exact(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let root = (n as f64).sqrt().round() as i64;
    if root.saturating_mul(root) == n {
        Some(root)
    } else {
        None
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

    /// Track M2 — snapshot of the set of currently-loaded packages.
    ///
    /// Returns a fresh `HashSet<String>` so callers can inspect the
    /// session state without holding the backend's lock.  The
    /// canonical entry is `"orthopoly"` once `load("orthopoly")` has
    /// run.
    pub fn loaded_packages(&self) -> HashSet<String> {
        self.backend_state
            .lock()
            .expect("macsyma backend state poisoned")
            .loaded_packages
            .clone()
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
    if !expr.args.len().is_multiple_of(2) {
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

/// `expand(expr)` — full polynomial expansion: distribute `Mul` over
/// `Add`/`Sub`, expand bounded non-negative integer `Pow`s, and collect
/// like terms (`expand(x + x)` → `2*x`, repeated factors fold into a
/// power). Was previously wired to the `Expand` head via the `EXPAND`
/// name-table entry, but no handler was ever registered for that head in
/// `symbolic-vm`'s shared table, so `expand((x+1)^2)` silently returned
/// the unevaluated input. Fixed by delegating to `cas_simplify::expand`
/// — see that crate for the full algorithm.
fn expand_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    expand(expr.args[0].clone())
}

fn radcan_handler(state: &Arc<Mutex<MacsymaBackendState>>, expr: IRApply) -> IRNode {
    if expr.args.len() != 1 {
        return IRNode::Apply(Box::new(expr));
    }
    let state = state.lock().expect("macsyma backend state poisoned");
    radcan(expr.args[0].clone(), Some(&state.assumptions))
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

//! The tree-walking evaluator.
//!
//! [`Interpreter`] walks the [`GrammarASTNode`] tree from `j-parser` and
//! computes `array_runtime::Array` values. J has exactly **two** expression
//! nonterminals (MA06 §3, reused from APL's `value_expr`/`function_expr`
//! split almost verbatim): `noun_expr` (arrays/scalars) and `verb_expr` (a
//! primitive glyph, optionally combined with `/`/`\`/`@` into a *derived
//! verb*, or a parenthesised **train** — the one genuinely new production
//! with no APL precedent). This evaluator mirrors that split:
//! [`Interpreter::eval_noun_expr`] walks the noun tree and calls
//! [`Interpreter::apply_monadic`]/[`Interpreter::apply_dyadic`] whenever it
//! meets a `verb_expr`, which in turn dispatch on an internal [`JFn`] — the
//! runtime's own representation of "which verb, and with which
//! adverb/conjunction/train structure (if any) applied". `JFn` generalizes
//! `apl-runtime::eval::AplFn` exactly as MA06 §5 anticipated: the same
//! `Atom`/`NonScalar`/`Reduce`/`Scan` shape, plus `Compose`/`Hook`/`Fork` for
//! trains (APL has no train production at all, so those three variants are
//! new here).
//!
//! The 12 primitive verbs that are ordinary scalar dyadic functions
//! (`+ - * % <. >. = ~: < > <: >:`) share `array_runtime::ops::BinOp` for
//! both their dyadic meaning (`ops::elementwise`) and reduce/scan (see
//! `JFn::Atom`) — exactly like APL's own 12 atoms. `$`/`i.`/`,`/`#`/`^` do
//! not fit that shape at all (their monadic and dyadic meanings are
//! unrelated to each other, or — for `#`/`^` — have no shared kernel at
//! all), so they get bespoke logic in `builtins.rs` instead (see
//! `JFn::NonScalar`). Unlike APL, J has **no** outer-product operator in
//! this cut's scope (MA06 §4 lists only `/` and `\` as adverbs), so there is
//! no `JFn::Outer` counterpart to `AplFn::Outer`.

use crate::builtins;
use crate::value::display;
use array_runtime::{ops, ops::BinOp, Array};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum recursion depth for this evaluator's own tree-walk. `j-parser`'s
/// own `MAX_RULE_DEPTH` (70, see that crate's `lib.rs`) already bounds how
/// deep a CST built from untrusted input can possibly be, so — exactly like
/// `apl-runtime::eval::MAX_DEPTH`'s own rationale — this bound can never
/// actually trip on a tree that came from `try_parse_j`; it exists purely as
/// **defense in depth**.
///
/// This crate adds one genuinely new recursion shape APL's own evaluator
/// never had: `JFn::Compose`/`Hook`/`Fork` recurse back through
/// [`Interpreter::apply_monadic`]/[`Interpreter::apply_dyadic`] themselves
/// (evaluating a train's tooth verbs against the same operand(s)), not just
/// through the noun-expression walk `apl-runtime::eval::apply_monadic`/
/// `apply_dyadic` never needed a guard for (APL's own `AplFn` variants are
/// all leaf dispatches with no recursive calls back into `apply_*`). Both
/// `apply_monadic` and `apply_dyadic` below take their own depth-guard entry
/// for exactly this reason — real defense in depth for a real (if
/// parser-bounded) new recursion shape, not a redundant copy-paste of
/// APL's guard placement.
const MAX_DEPTH: usize = 512;

/// A persistent J session: a variable workspace and the current evaluation
/// depth.
pub struct Interpreter {
    vars: HashMap<String, Array>,
    depth: Rc<Cell<usize>>,
}

/// RAII guard that decrements the depth counter on every exit path
/// (including a `?` early return).
struct DepthGuard(Rc<Cell<usize>>);

impl Drop for DepthGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// One of `j.tokens`' five bespoke (non-`BinOp`) primitive verbs, kept
/// around so error messages can name the actual glyph (`"$"`, not
/// `"DOLLAR"`).
#[derive(Clone, Copy)]
enum NonScalarAtom {
    Dollar,
    Idot,
    Ravel,
    Hash,
    Caret,
}

impl NonScalarAtom {
    fn glyph(self) -> &'static str {
        match self {
            NonScalarAtom::Dollar => "$",
            NonScalarAtom::Idot => "i.",
            NonScalarAtom::Ravel => ",",
            NonScalarAtom::Hash => "#",
            NonScalarAtom::Caret => "^",
        }
    }
}

/// The runtime's own representation of a `verb_expr`: "which verb, and with
/// which adverb/conjunction/train structure (if any) applied" — generalizing
/// `apl-runtime::eval::AplFn` per MA06 §5's own explicit instruction.
enum JFn {
    /// One of the 12 primitive verbs that map onto `array_runtime::ops::BinOp`
    /// (`+ - * % <. >. = ~: < > <: >:`). Exactly one glyph per `BinOp`
    /// variant, so `BinOp` alone is enough to recover which glyph this was
    /// for monadic dispatch — no separate glyph tag needed here, unlike
    /// [`NonScalar`](JFn::NonScalar).
    Atom(BinOp),
    /// `$`/`i.`/`,`/`#`/`^` — bespoke monadic+dyadic logic (`builtins.rs`)
    /// that does not fit "an operator over a scalar dyadic function" at
    /// all, so none of these ever plug into reduce/scan.
    NonScalar(NonScalarAtom),
    /// A `BinOp`-mappable atom with `/` (reduce) applied — inherently a
    /// *monadic* derived verb (`+/A` reduces the one array `A`).
    Reduce(BinOp),
    /// A `BinOp`-mappable atom with `\` (scan) applied — also monadic.
    Scan(BinOp),
    /// `f@g` ("atop" compose, MA06 §4's one in-scope conjunction): monadic
    /// `(f@g) y = f (g y)`; dyadic `x (f@g) y = f (x g y)` — see
    /// [`Interpreter::apply_dyadic`]'s doc comment for why the dyadic
    /// formula is this crate's own considered generalization, not something
    /// MA06 spells out verbatim.
    Compose(Box<JFn>, Box<JFn>),
    /// A 2-tooth train (hook): monadic `(f g) y = y f (g y)`; dyadic
    /// `x (f g) y = x f (g y)` — `g` always applies monadically to `y`
    /// alone, regardless of the surrounding call's own arity (the defining
    /// property of a hook, MA06 §3).
    Hook(Box<JFn>, Box<JFn>),
    /// A 3-tooth train (fork), or the 4+-tooth case peeled down to this base
    /// case (MA06 §3's corrected folding rule — see `fold_train`). `left` is
    /// either an ordinary verb tooth or a captured literal noun (only
    /// meaningful in this leading position).
    Fork(ForkLeft, Box<JFn>, Box<JFn>),
}

/// The first ("left") tooth of a [`JFn::Fork`]: either an ordinary verb, or
/// a literal noun constant (MA06 §3's "leading noun" fork case — a bare
/// noun tooth is only ever meaningful here, never in a hook or anywhere
/// else, see `fold_train`/`tooth_to_verb`).
enum ForkLeft {
    Verb(Box<JFn>),
    Noun(Array),
}

/// What one `train_tooth` evaluates to, before it's known whether the
/// overall train needs it to be a verb or (in a fork's leading position
/// only) accepts it as a literal noun — see [`Interpreter::eval_train_tooth`].
enum ToothValue {
    Verb(JFn),
    Noun(Array),
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            vars: HashMap::new(),
            depth: Rc::new(Cell::new(0)),
        }
    }

    /// Enter one level of recursion, erroring if [`MAX_DEPTH`] is exceeded.
    /// The returned guard decrements the counter when it drops.
    fn enter(&self) -> Result<DepthGuard, String> {
        self.depth.set(self.depth.get() + 1);
        let guard = DepthGuard(Rc::clone(&self.depth));
        if self.depth.get() > MAX_DEPTH {
            return Err("j-runtime: expression nesting too deep".to_string());
        }
        Ok(guard)
    }

    /// Evaluate a whole `program` node, returning the auto-print output for
    /// every statement that is *not* an assignment (MA06 §4: assignment is
    /// silent; a bare `noun_expr` result auto-prints — mirrors
    /// `apl-runtime::eval::Interpreter::run` exactly, since J's `program`/
    /// `line`/`statement` productions are structurally identical to APL's).
    pub fn run(&mut self, program: &GrammarASTNode) -> Result<String, String> {
        let mut out = String::new();
        for line in node_children(program) {
            if line.rule_name != "line" {
                continue;
            }
            // A `line` with just a bare NEWLINE (blank line, or a
            // comment-only line — `NB.` comments are already stripped by
            // the lexer's skip pattern) has no `statement` child at all;
            // skip it.
            let stmt = line.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            });
            let Some(stmt) = stmt else { continue };
            if let Some(text) = self.eval_statement(stmt)? {
                out.push_str(&text);
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// `statement = assignment` (a pure passthrough rule, always exactly one
    /// child) — evaluate the `assignment` and decide whether to print based
    /// on whether it was an *actual* assignment (`NAME ASSIGN_LOCAL|ASSIGN_GLOBAL
    /// assignment`, 3 children) or a plain `noun_expr` passthrough (1 child).
    fn eval_statement(&mut self, stmt: &GrammarASTNode) -> Result<Option<String>, String> {
        let assignment = only_node(stmt)?;
        let is_assignment = assignment.children.len() == 3;
        let value = self.eval_assignment(assignment)?;
        if is_assignment {
            Ok(None)
        } else {
            Ok(Some(display(&value)))
        }
    }

    /// `assignment = NAME ASSIGN_LOCAL assignment | NAME ASSIGN_GLOBAL
    /// assignment | noun_expr`. Both `=.` and `=:` do the identical thing in
    /// this cut (MA06 §4: "not meaningfully different ... since this cut
    /// has no user-defined verbs/tacit definitions yet"), so which token
    /// matched doesn't need to be inspected at all — only whether this node
    /// has 3 children (an actual assignment) or 1 (a passthrough). The
    /// right-hand side of a 3-child node recurses back through `assignment`
    /// itself, so `A=.B=.3` binds `3` to *both* `B` and `A` (right-
    /// associative chained assignment, mirroring
    /// `apl-runtime::eval::eval_assignment` exactly).
    fn eval_assignment(&mut self, node: &GrammarASTNode) -> Result<Array, String> {
        let _guard = self.enter()?;
        if node.children.len() == 3 {
            let name = assignment_target_name(node)?;
            // `only_node` finds the lone `Node` child among
            // `[Token(NAME), Token(ASSIGN_LOCAL|ASSIGN_GLOBAL), Node(assignment)]`
            // — the nested `assignment` to recurse into.
            let inner = only_node(node)?;
            let value = self.eval_assignment(inner)?;
            self.vars.insert(name, value.clone());
            Ok(value)
        } else {
            let noun_expr = only_node(node)?;
            self.eval_noun_expr(noun_expr)
        }
    }

    /// `noun_expr`:
    /// - 1 child `[Node(term)]` — a bare term.
    /// - 2 children `[Node(verb_expr), Node(noun_expr)]` — monadic
    ///   application.
    /// - 3 children `[Node(term), Node(verb_expr), Node(noun_expr)]` —
    ///   dyadic application, right-recursive (`A+B+C` is `A+(B+C)`).
    fn eval_noun_expr(&self, node: &GrammarASTNode) -> Result<Array, String> {
        let _guard = self.enter()?;
        let kids = node_children(node);
        match kids.len() {
            1 => self.eval_term(kids[0]),
            2 => {
                let f = self.parse_verb_expr(kids[0])?;
                let arg = self.eval_noun_expr(kids[1])?;
                self.apply_monadic(&f, &arg)
            }
            3 => {
                let lhs = self.eval_term(kids[0])?;
                let f = self.parse_verb_expr(kids[1])?;
                let rhs = self.eval_noun_expr(kids[2])?;
                self.apply_dyadic(&f, &lhs, &rhs)
            }
            n => Err(format!("j-runtime: malformed noun_expr with {n} children")),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | LPAREN noun_expr RPAREN`.
    fn eval_term(&self, node: &GrammarASTNode) -> Result<Array, String> {
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // "Stranding": one or more juxtaposed NUMBER tokens form a
                // single term — `1 2 3` is one 3-element vector, a lone `5`
                // is a rank-0 scalar (MA06 §4, inherited unchanged from
                // APL). This literal-construction path has no grammar-level
                // depth bound on the *count* of stranded numbers (`term`'s
                // repetition is flat, not recursive), so it is capped here
                // directly, mirroring `apl-runtime::eval::eval_term`'s own
                // identical guard.
                if node.children.len() > builtins::MAX_ARRAY_LENGTH {
                    return Err(format!(
                        "j-runtime: stranded literal of {} numbers exceeds the cap of {} elements",
                        node.children.len(),
                        builtins::MAX_ARRAY_LENGTH
                    ));
                }
                let mut nums = Vec::new();
                for c in &node.children {
                    if let ASTNodeOrToken::Token(tok) = c {
                        nums.push(parse_j_number(&tok.value)?);
                    }
                }
                if nums.len() == 1 {
                    Ok(Array::scalar(nums[0]))
                } else {
                    Ok(Array::from_vec(nums))
                }
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => self
                .vars
                .get(&t.value)
                .cloned()
                .ok_or_else(|| format!("j-runtime: undefined variable '{}'", t.value)),
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)?; // the Node(noun_expr) child
                self.eval_noun_expr(inner)
            }
            _ => Err("j-runtime: malformed term".to_string()),
        }
    }

    /// `verb_expr = simple_verb [ AT verb_expr ] | LPAREN verb_train RPAREN`.
    /// The one production with no APL precedent (a parenthesised train)
    /// needs `&self` (a train's leading-noun tooth may be a `NAME` needing a
    /// variable lookup, see [`Interpreter::eval_train_tooth`]) — unlike
    /// `apl-runtime::eval::parse_function_expr`/`parse_function_atom`, which
    /// are free functions since APL's `function_expr` never evaluates a
    /// noun.
    fn parse_verb_expr(&self, node: &GrammarASTNode) -> Result<JFn, String> {
        let _guard = self.enter()?;
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(simple)] if simple.rule_name == "simple_verb" => {
                parse_simple_verb(simple)
            }
            [ASTNodeOrToken::Node(simple), ASTNodeOrToken::Token(at), ASTNodeOrToken::Node(rest)]
                if simple.rule_name == "simple_verb" && at.effective_type_name() == "AT" =>
            {
                let f = parse_simple_verb(simple)?;
                let g = self.parse_verb_expr(rest)?;
                Ok(JFn::Compose(Box::new(f), Box::new(g)))
            }
            [ASTNodeOrToken::Token(lparen), ASTNodeOrToken::Node(train), ASTNodeOrToken::Token(_rparen)]
                if lparen.effective_type_name() == "LPAREN" =>
            {
                self.parse_verb_train(train)
            }
            _ => Err("j-runtime: malformed verb_expr".to_string()),
        }
    }

    /// `verb_train = train_tooth train_tooth { train_tooth }` — a flat list
    /// of 2+ `train_tooth` children. Evaluate each tooth to a
    /// [`ToothValue`] first, then [`fold_train`] applies MA06 §3's
    /// corrected right-to-left, peel-from-the-left folding rule.
    fn parse_verb_train(&self, node: &GrammarASTNode) -> Result<JFn, String> {
        let teeth = node_children(node)
            .into_iter()
            .map(|tooth| self.eval_train_tooth(tooth))
            .collect::<Result<Vec<_>, _>>()?;
        fold_train(teeth)
    }

    /// `train_tooth = verb_expr | term`. A `term` tooth is a literal noun
    /// constant — no variables should appear there in practice, but if a
    /// bare `NAME` does, it's looked up in the current bindings exactly like
    /// any other noun evaluation, erroring cleanly if unbound.
    fn eval_train_tooth(&self, node: &GrammarASTNode) -> Result<ToothValue, String> {
        let _guard = self.enter()?;
        match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "verb_expr" => {
                Ok(ToothValue::Verb(self.parse_verb_expr(n)?))
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "term" => {
                Ok(ToothValue::Noun(self.eval_term(n)?))
            }
            _ => Err("j-runtime: malformed train_tooth".to_string()),
        }
    }

    /// Apply a monadic (one-argument) verb.
    fn apply_monadic(&self, f: &JFn, y: &Array) -> Result<Array, String> {
        let _guard = self.enter()?;
        match f {
            JFn::Atom(op) => apply_monadic_scalar(*op, y),
            JFn::NonScalar(NonScalarAtom::Dollar) => Ok(builtins::shape(y)),
            JFn::NonScalar(NonScalarAtom::Idot) => builtins::index_generator(y),
            JFn::NonScalar(NonScalarAtom::Ravel) => Ok(builtins::ravel(y)),
            JFn::NonScalar(NonScalarAtom::Hash) => Ok(builtins::tally(y)),
            JFn::NonScalar(NonScalarAtom::Caret) => Ok(builtins::monadic_exp(y)),
            JFn::Reduce(op) => ops::reduce(*op, y),
            JFn::Scan(op) => ops::scan(*op, y),
            // Compose (atop), monadic: `(f@g) y = f (g y)` (MA06 §4).
            JFn::Compose(f, g) => {
                let inner = self.apply_monadic(g, y)?;
                self.apply_monadic(f, &inner)
            }
            // Hook, monadic: `(f g) y = y f (g y)` (MA06 §3).
            JFn::Hook(f, g) => {
                let gy = self.apply_monadic(g, y)?;
                self.apply_dyadic(f, y, &gy)
            }
            // Fork, monadic: verb-left `(f g h) y = (f y) g (h y)`;
            // leading-noun `(n g h) y = n g (h y)` (MA06 §3).
            JFn::Fork(left, g, h) => match left {
                ForkLeft::Verb(f) => {
                    let fy = self.apply_monadic(f, y)?;
                    let hy = self.apply_monadic(h, y)?;
                    self.apply_dyadic(g, &fy, &hy)
                }
                ForkLeft::Noun(n) => {
                    let hy = self.apply_monadic(h, y)?;
                    self.apply_dyadic(g, n, &hy)
                }
            },
        }
    }

    /// Apply a dyadic (two-argument) verb.
    fn apply_dyadic(&self, f: &JFn, x: &Array, y: &Array) -> Result<Array, String> {
        let _guard = self.enter()?;
        match f {
            JFn::Atom(op) => ops::elementwise(*op, x, y),
            JFn::NonScalar(NonScalarAtom::Dollar) => builtins::reshape(x, y),
            JFn::NonScalar(NonScalarAtom::Idot) => builtins::index_of(x, y),
            JFn::NonScalar(NonScalarAtom::Ravel) => builtins::catenate(x, y),
            JFn::NonScalar(NonScalarAtom::Hash) => builtins::replicate(x, y),
            JFn::NonScalar(NonScalarAtom::Caret) => builtins::dyadic_pow(x, y),
            JFn::Reduce(_) => Err(
                "j-runtime: / (reduce) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
            JFn::Scan(_) => Err(
                "j-runtime: \\ (scan) takes exactly one operand, but was applied dyadically"
                    .to_string(),
            ),
            // Compose (atop), dyadic: `x (f@g) y = f (x g y)`. MA06 §4 only
            // spells out the monadic formula; this dyadic reading is this
            // crate's own considered generalization -- real J's standard
            // "atop" semantics work exactly this way (the right verb applies
            // with the surrounding arity, the left verb always applies
            // monadically to the result), matching the task's own explicit
            // design note rather than an unstated assumption.
            JFn::Compose(f, g) => {
                let inner = self.apply_dyadic(g, x, y)?;
                self.apply_monadic(f, &inner)
            }
            // Hook, dyadic: `x (f g) y = x f (g y)` -- `g` is ALWAYS applied
            // monadically to `y` alone, regardless of this call's own
            // dyadic arity (the defining property of a hook, MA06 §3).
            JFn::Hook(f, g) => {
                let gy = self.apply_monadic(g, y)?;
                self.apply_dyadic(f, x, &gy)
            }
            // Fork, dyadic: verb-left `x (f g h) y = (x f y) g (x h y)`.
            // Leading-noun `x (n g h) y = n g (x h y)` -- MA06 §3 only gives
            // the monadic formula for the leading-noun case; this dyadic
            // reading is this crate's own disclosed generalization: the
            // noun `n` stays a literal constant regardless of arity (it is
            // never "applied" at all), while `h` applies with the
            // surrounding call's own arity, exactly mirroring how an
            // ordinary (non-noun-leading) fork's `h` does the same.
            JFn::Fork(left, g, h) => match left {
                ForkLeft::Verb(f) => {
                    let fxy = self.apply_dyadic(f, x, y)?;
                    let hxy = self.apply_dyadic(h, x, y)?;
                    self.apply_dyadic(g, &fxy, &hxy)
                }
                ForkLeft::Noun(n) => {
                    let hxy = self.apply_dyadic(h, x, y)?;
                    self.apply_dyadic(g, n, &hxy)
                }
            },
        }
    }
}

/// Monadic meaning of the six `BinOp`-mappable verbs that have one (MA06
/// §4): `+` conjugate (identity — this cut has no complex numbers, so
/// conjugate is a no-op), `-` negate, `*` sign, `%` reciprocal, `<.` floor,
/// `>.` ceiling. The six comparisons (`= ~: < <: >: >`, mapped onto
/// `Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt`) have **no** monadic meaning in J either —
/// a clean, explicit error rather than silently picking a behavior (mirrors
/// `apl-runtime::eval::apply_monadic_scalar` exactly, just with J's own
/// glyph spellings in the error text).
fn apply_monadic_scalar(op: BinOp, x: &Array) -> Result<Array, String> {
    let f: fn(f64) -> f64 = match op {
        BinOp::Add => |v| v,
        BinOp::Sub => |v| -v,
        BinOp::Mul => j_sign,
        BinOp::Div => |v| 1.0 / v,
        BinOp::Min => f64::floor, // FLOOR atom (<.) -- floor monadically
        BinOp::Max => f64::ceil,  // CEILING atom (>.) -- ceiling monadically
        BinOp::Eq => return Err("j-runtime: no monadic form for =".to_string()),
        BinOp::Ne => return Err("j-runtime: no monadic form for ~:".to_string()),
        BinOp::Lt => return Err("j-runtime: no monadic form for <".to_string()),
        BinOp::Le => return Err("j-runtime: no monadic form for <:".to_string()),
        BinOp::Ge => return Err("j-runtime: no monadic form for >:".to_string()),
        BinOp::Gt => return Err("j-runtime: no monadic form for >".to_string()),
    };
    Ok(
        Array::from_shape(x.data().iter().map(|&v| f(v)).collect(), x.shape().to_vec())
            .expect("monadic map preserves shape/length"),
    )
}

/// J's monadic `*` (sign): `1` for positive, `_1` for negative, `0` for
/// zero. **Not** `f64::signum()` — that returns `1.0` for `0.0`, which is
/// wrong for this three-way sign function (mirrors
/// `apl-runtime::eval::apl_sign`'s exact, already-correct logic).
fn j_sign(x: f64) -> f64 {
    if x.is_nan() {
        f64::NAN
    } else if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Parse one `NUMBER` token's source text into an `f64`. J's own negative-
/// literal convention (MA06 §4's addendum, `j.tokens` SECTION 4) is a
/// leading underscore in the mantissa and/or exponent (`_3`, `1.5E_3`) —
/// never APL's high-minus `¯` (no ASCII spelling) and never a bare `-`
/// (already the `MINUS` verb token) — so every underscore is translated to
/// ASCII `-` before handing the text to Rust's `f64` parser, mirroring
/// `apl-runtime::eval::parse_apl_number`'s identical role for `¯`.
fn parse_j_number(s: &str) -> Result<f64, String> {
    s.replace('_', "-")
        .parse::<f64>()
        .map_err(|_| format!("j-runtime: invalid number literal '{s}'"))
}

/// `simple_verb = verb_primitive [ REDUCE | SCAN ]`. A free function (unlike
/// [`Interpreter::parse_verb_expr`]) since a bare primitive/adverb pair never
/// needs a variable lookup.
fn parse_simple_verb(node: &GrammarASTNode) -> Result<JFn, String> {
    match node.children.as_slice() {
        [ASTNodeOrToken::Node(prim)] => parse_verb_primitive(prim),
        [ASTNodeOrToken::Node(prim), ASTNodeOrToken::Token(adverb)] => {
            let base = parse_verb_primitive(prim)?;
            match adverb.effective_type_name() {
                "REDUCE" => Ok(JFn::Reduce(require_scalar_binop(&base, "reduce")?)),
                "SCAN" => Ok(JFn::Scan(require_scalar_binop(&base, "scan")?)),
                other => Err(format!("j-runtime: unexpected adverb token '{other}'")),
            }
        }
        _ => Err("j-runtime: malformed simple_verb".to_string()),
    }
}

/// `verb_primitive`: always exactly one child, a single token naming the
/// primitive glyph. Maps each of the 12 `BinOp`-compatible verbs onto its
/// `BinOp` variant, and each of the 5 bespoke verbs onto its
/// [`NonScalarAtom`] — **critically**, `FLOOR` (`<.`) maps to `BinOp::Min`
/// and `CEILING` (`>.`) maps to `BinOp::Max`, the same mapping
/// `apl-runtime::eval::parse_function_atom` already uses for APL's `⌊`/`⌈`
/// (MA06 §4: "the mapping is the *opposite* character from what APL's
/// `⌊`/`⌈` might suggest" refers to *which ASCII digraph* spells "floor" —
/// `<.`, not `>.`, because `<` already means "less than" — not to a
/// different `BinOp` target; the underlying `BinOp` values are identical to
/// APL's own FLOOR→Min/CEILING→Max mapping).
fn parse_verb_primitive(node: &GrammarASTNode) -> Result<JFn, String> {
    let tok = match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) => t,
        _ => return Err("j-runtime: malformed verb_primitive".to_string()),
    };
    Ok(match tok.effective_type_name() {
        "PLUS" => JFn::Atom(BinOp::Add),
        "MINUS" => JFn::Atom(BinOp::Sub),
        "STAR" => JFn::Atom(BinOp::Mul),
        "PERCENT" => JFn::Atom(BinOp::Div),
        "FLOOR" => JFn::Atom(BinOp::Min),
        "CEILING" => JFn::Atom(BinOp::Max),
        "EQ" => JFn::Atom(BinOp::Eq),
        "NE" => JFn::Atom(BinOp::Ne),
        "LT" => JFn::Atom(BinOp::Lt),
        "LE" => JFn::Atom(BinOp::Le),
        "GE" => JFn::Atom(BinOp::Ge),
        "GT" => JFn::Atom(BinOp::Gt),
        "DOLLAR" => JFn::NonScalar(NonScalarAtom::Dollar),
        "IDOT" => JFn::NonScalar(NonScalarAtom::Idot),
        "RAVEL" => JFn::NonScalar(NonScalarAtom::Ravel),
        "HASH" => JFn::NonScalar(NonScalarAtom::Hash),
        "CARET" => JFn::NonScalar(NonScalarAtom::Caret),
        other => return Err(format!("j-runtime: unknown verb primitive token '{other}'")),
    })
}

/// Reduce/scan apply only to the 12 verbs that map onto a `BinOp` —
/// `$`/`i.`/`,`/`#`/`^` are not "a scalar dyadic function" at all, so
/// stacking an adverb onto one of them is a clean, explicit scope error
/// (mirrors `apl-runtime::eval::require_scalar_binop` exactly).
fn require_scalar_binop(f: &JFn, context: &str) -> Result<BinOp, String> {
    match f {
        JFn::Atom(op) => Ok(*op),
        JFn::NonScalar(a) => Err(format!(
            "{context}: {} is not a scalar dyadic verb",
            a.glyph()
        )),
        JFn::Reduce(_) | JFn::Scan(_) | JFn::Compose(_, _) | JFn::Hook(_, _) | JFn::Fork(_, _, _) => {
            unreachable!("parse_verb_primitive never produces an adverb/conjunction/train JFn")
        }
    }
}

/// Fold a flat list of `verb_train` teeth into a `JFn`, following MA06 §3's
/// corrected right-to-left, peel-from-the-left recursive rule:
///
/// - **2 teeth** `[f, g]`: `Hook(f, g)` — both teeth must be verbs; a noun
///   in either position of a 2-tooth train is a clean error (a leading noun
///   is only meaningful in the 3-tooth fork's own first slot).
/// - **3 teeth** `[left, g, h]`: `Fork(left, g, h)`, where `left` is
///   `ForkLeft::Noun` if the first tooth evaluated to a noun, else
///   `ForkLeft::Verb`.
/// - **4+ teeth** `[t0, t1, …, tN-1]`: peel the FIRST tooth off (it must be
///   a verb — a noun here has no defined meaning, since it would need to
///   become a `Hook`'s left tooth) and recurse on the rest:
///   `Hook(t0, fold_train([t1, …, tN-1]))`. Repeatedly peeling by exactly
///   one tooth at a time means the recursion always bottoms out at the
///   3-tooth fork base case (never the 2-tooth case, since `N - (N - 3) ==
///   3` for every `N ≥ 4`) — so the *only* tooth that may ever be a noun in
///   an N-tooth train is the one at index `N - 3` (the position that lands
///   in the terminal fork's own leading slot), falling out naturally from
///   this recursive structure with no extra position-tracking needed.
fn fold_train(mut teeth: Vec<ToothValue>) -> Result<JFn, String> {
    match teeth.len() {
        n if n < 2 => Err(format!(
            "j-runtime: a train needs at least 2 teeth, got {n}"
        )),
        2 => {
            let g = tooth_to_verb(teeth.pop().expect("len == 2"), "a 2-tooth hook")?;
            let f = tooth_to_verb(teeth.pop().expect("len == 2"), "a 2-tooth hook")?;
            Ok(JFn::Hook(Box::new(f), Box::new(g)))
        }
        3 => {
            let h = tooth_to_verb(teeth.pop().expect("len == 3"), "a fork's middle/right tooth")?;
            let g = tooth_to_verb(teeth.pop().expect("len == 3"), "a fork's middle/right tooth")?;
            let left = match teeth.pop().expect("len == 3") {
                ToothValue::Noun(n) => ForkLeft::Noun(n),
                ToothValue::Verb(f) => ForkLeft::Verb(Box::new(f)),
            };
            Ok(JFn::Fork(left, Box::new(g), Box::new(h)))
        }
        _ => {
            let rest = teeth.split_off(1);
            let first = tooth_to_verb(
                teeth.pop().expect("split_off(1) leaves exactly one element"),
                "a 4+-tooth train's peeled-off leading tooth",
            )?;
            let g = fold_train(rest)?;
            Ok(JFn::Hook(Box::new(first), Box::new(g)))
        }
    }
}

/// Require a [`ToothValue`] to be a verb, with a clean error naming the
/// context if it's a noun instead (see [`fold_train`]'s doc comment for
/// exactly which positions this can legally reject).
fn tooth_to_verb(t: ToothValue, context: &str) -> Result<JFn, String> {
    match t {
        ToothValue::Verb(f) => Ok(f),
        ToothValue::Noun(_) => Err(format!(
            "j-runtime: a bare noun tooth is only meaningful in a fork's leading position, not in {context}"
        )),
    }
}

/// The `NAME` token of an actual assignment's target — the first child of a
/// 3-child `assignment` node
/// (`[Token(NAME), Token(ASSIGN_LOCAL|ASSIGN_GLOBAL), Node(assignment)]`).
fn assignment_target_name(node: &GrammarASTNode) -> Result<String, String> {
    match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => Ok(t.value.clone()),
        _ => Err("j-runtime: malformed assignment (missing target name)".to_string()),
    }
}

// ── AST helpers ──────────────────────────────────────────────────────────────

fn node_children(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn first_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

fn only_node(node: &GrammarASTNode) -> Result<&GrammarASTNode, String> {
    first_node(node).ok_or_else(|| format!("j-runtime: malformed '{}' node", node.rule_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A direct, white-box unit test of the depth guard mechanism itself
    /// (`enter`/`DepthGuard`). Unlike `apl-runtime` (which has no such test
    /// at all -- `j-parser`'s own `MAX_RULE_DEPTH` bounds any *real* parsed
    /// tree far below `MAX_DEPTH`, so a realistic `feed()` call can never
    /// actually trip this guard, exactly like APL's own equivalent comment
    /// says), this crate adds one anyway: MA06-d's own task brief calls out
    /// "the depth guard" as one of the minimum things to cover, and the only
    /// way to actually exercise it at all is a direct test of `enter()`
    /// itself, since driving it via genuine J source is architecturally
    /// impossible (the parser's own cap fires first). This is a deliberate,
    /// disclosed small addition beyond the literal apl-runtime mirror, not
    /// an oversight in apl-runtime being copied forward.
    #[test]
    fn depth_guard_trips_after_max_depth_and_recovers() {
        let interp = Interpreter::new();
        let mut guards = Vec::new();
        for _ in 0..MAX_DEPTH {
            guards.push(interp.enter().expect("should stay under the cap"));
        }
        assert!(interp.enter().is_err(), "one more enter() must trip the cap");
        drop(guards);
        assert!(
            interp.enter().is_ok(),
            "dropping every guard must let a fresh enter() succeed again"
        );
    }
}

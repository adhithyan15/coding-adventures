//! ESTree-compatible expression nodes (CLOC09 Phase 1).
//!
//! 15 variants:
//! - Leaves: [`Identifier`], [`NumericLiteral`], [`StringLiteral`],
//!   [`BooleanLiteral`], [`NullLiteral`], [`BigIntLiteral`] (Phase 1.x —
//!   added in CLOC12.15), [`UndefinedLiteral`] (Phase 1.x — added in
//!   CLOC12.16).
//! - Operators: [`BinaryExpression`], [`LogicalExpression`],
//!   [`UnaryExpression`], [`AssignmentExpression`],
//!   [`ConditionalExpression`].
//! - Application: [`CallExpression`], [`MemberExpression`].
//! - Composites: [`ArrayExpression`], [`ObjectExpression`].
//! - Callables: [`FunctionExpression`] (Phase 1.x — added in CLOC12.149;
//!   the expression sibling of [`crate::FunctionDeclaration`], e.g.
//!   `var f = function () {}`, IIFEs, function-valued properties);
//!   [`ArrowFunctionExpression`] (Phase 1.x — added in CLOC12.151; the
//!   `=>` form, e.g. `x => x + 1`, `(a, b) => { return a; }`).
//! - Templates: [`TemplateLiteral`] (Phase 1.x — added in CLOC12.154; the
//!   backtick form, e.g. `` `a${x}b` ``, with parallel `quasis` /
//!   `expressions`).
//!
//! Every struct carries `cv: Option<CvId>` first per the CLOC09
//! amendment. Operator enums serialize to ESTree-canonical operator
//! strings (`"=="`, `"==="`, `"&&"`, etc.) via per-variant
//! `#[serde(rename = "...")]`.

use crate::declaration::FunctionParam;
use crate::statement::BlockStatement;
use crate::CvId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// Expression — the tagged union
// ---------------------------------------------------------------------

/// Tagged union of every expression variant. JSON wire format is
/// `{"type": "<Variant>", ...}` per ESTree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Expression {
    Identifier(Identifier),
    NumericLiteral(NumericLiteral),
    StringLiteral(StringLiteral),
    BooleanLiteral(BooleanLiteral),
    NullLiteral(NullLiteral),
    BigIntLiteral(BigIntLiteral),
    UndefinedLiteral(UndefinedLiteral),
    RegExpLiteral(RegExpLiteral),
    BinaryExpression(BinaryExpression),
    LogicalExpression(LogicalExpression),
    UnaryExpression(UnaryExpression),
    AssignmentExpression(AssignmentExpression),
    ConditionalExpression(ConditionalExpression),
    CallExpression(CallExpression),
    MemberExpression(MemberExpression),
    OptionalMemberExpression(OptionalMemberExpression),
    OptionalCallExpression(OptionalCallExpression),
    ChainExpression(ChainExpression),
    ArrayExpression(ArrayExpression),
    ObjectExpression(ObjectExpression),
    FunctionExpression(FunctionExpression),
    ArrowFunctionExpression(ArrowFunctionExpression),
    ClassExpression(ClassExpression),
    TemplateLiteral(TemplateLiteral),
    UpdateExpression(UpdateExpression),
    NewExpression(NewExpression),
    SequenceExpression(SequenceExpression),
    TaggedTemplateExpression(TaggedTemplateExpression),
    SpreadElement(SpreadElement),
    YieldExpression(YieldExpression),
    AwaitExpression(AwaitExpression),
    ThisExpression(ThisExpression),
    Super(Super),
    NewTarget(NewTarget),
    ImportMeta(ImportMeta),
    ImportExpression(ImportExpression),
}

// ---------------------------------------------------------------------
// Leaves
// ---------------------------------------------------------------------

/// A name reference: variable, parameter, function name, property key
/// (when not computed). ESTree `Identifier`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub name: String,
}

/// A numeric literal — `42`, `3.14`, `0xff`. ESTree splits its single
/// `Literal` node into typed variants for us; the underlying numeric
/// value is always an IEEE 754 double.
///
/// `value` is the parsed numeric value; `raw` preserves the original
/// source text (so `0xFF` round-trips as `0xFF` rather than `255`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NumericLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: f64,
    pub raw: String,
}

/// A string literal — `"hello"`, `'world'`. `value` is the unescaped
/// string contents; `raw` preserves the original source representation
/// including quotes and escape sequences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: String,
    pub raw: String,
}

/// A regular-expression literal — `/ab+c/gi`. Modelled as its own leaf so a
/// regex is never confused with an ordinary reference: previously the bridge
/// fell back to an [`Identifier`] whose name happened to be the regex source,
/// which round-tripped only by accident (the text is not a valid identifier,
/// so the rename passes skipped it) and left a latent hazard.
///
/// The two halves are stored split so passes can reason about the flags
/// without re-parsing: `pattern` is the body between the slashes (`ab+c`) and
/// `flags` is the trailing flag set (`gi`, possibly empty). The printer
/// reconstructs the source as `/{pattern}/{flags}` — the slashes and the
/// pattern's own escapes are part of `pattern`'s opaque text, so no escaping
/// is applied. Mirrors ESTree's `RegExpLiteral` (its `regex: {pattern, flags}`
/// companion object), minus the `value` field (a live `RegExp`, which a
/// source-to-source minifier never needs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegExpLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub pattern: String,
    pub flags: String,
}

/// A boolean literal — `true` / `false`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub value: bool,
}

/// The `null` literal. Carries no payload beyond its `cv`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NullLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// The `undefined` literal.  Carries no payload beyond its `cv`.
///
/// **Note: `undefined` is technically an identifier in ECMAScript,
/// not a reserved word — `var undefined = 1;` is legal in
/// non-strict mode and shadows the global.** ESTree historically
/// modelled it as `Identifier { name: "undefined" }`.  We follow
/// the modern *typed* variant approach: a dedicated leaf node so
/// passes can pattern-match on it without first checking the
/// identifier name.  The emitter renders it as `void 0` (a fixed
/// expression that always evaluates to the genuine undefined value
/// regardless of any shadow binding in scope — upstream Closure's
/// safe rendering).
///
/// Added in Phase 1.x (CLOC12.16) to close gap-001.  Closes the
/// final hole in CLOC12.09's typeof-literal fold table — passes
/// can now fold `typeof <UndefinedLiteral>` to `"undefined"`
/// without needing the surrounding context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndefinedLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// A BigInt literal — `123n`, `0n`, `0x1fn`. ESTree models this as a
/// separate node from [`NumericLiteral`] because bigints can exceed
/// the `f64` range — the `value` field is therefore a `String` (per
/// ESTree's JSON-safety convention) holding the decimal expansion of
/// the bigint, while `raw` keeps the original source representation
/// including the trailing `n` suffix.
///
/// Added in Phase 1.x (CLOC12.15) to model the bigint primitive
/// (gap-021). No optimisation rides on this yet — passes treat a
/// `BigIntLiteral` as already-folded the same way `NumericLiteral`
/// is treated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigIntLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// Decimal expansion of the bigint as a string. E.g. `"123"`,
    /// `"0"`, `"31"` (for `0x1fn`). Always non-negative — the `-` in
    /// `-123n` is a [`UnaryExpression`] over a `BigIntLiteral`, never
    /// part of the literal itself.
    pub value: String,
    /// Original source representation including the trailing `n`,
    /// e.g. `"123n"`, `"0x1fn"`. Preserved so that hex-typed bigints
    /// round-trip through the emitter without losing their radix.
    pub raw: String,
}

// ---------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------

/// `a op b` where `op` is non-short-circuiting. Note that ESTree splits
/// short-circuiting operators (`&&`, `||`, `??`) into [`LogicalExpression`]
/// — the semantic distinction matters for evaluation order and for
/// passes that reason about side effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

/// Binary operators per ECMAScript. Serializes as the ESTree-canonical
/// source-text string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    #[serde(rename = "==")] Eq,
    #[serde(rename = "!=")] NotEq,
    #[serde(rename = "===")] StrictEq,
    #[serde(rename = "!==")] StrictNotEq,
    #[serde(rename = "<")] Lt,
    #[serde(rename = "<=")] LtEq,
    #[serde(rename = ">")] Gt,
    #[serde(rename = ">=")] GtEq,
    #[serde(rename = "<<")] LeftShift,
    #[serde(rename = ">>")] RightShift,
    #[serde(rename = ">>>")] UnsignedRightShift,
    #[serde(rename = "+")] Add,
    #[serde(rename = "-")] Sub,
    #[serde(rename = "*")] Mul,
    #[serde(rename = "/")] Div,
    #[serde(rename = "%")] Mod,
    #[serde(rename = "**")] Exp,
    #[serde(rename = "|")] BitOr,
    #[serde(rename = "^")] BitXor,
    #[serde(rename = "&")] BitAnd,
    #[serde(rename = "in")] In,
    #[serde(rename = "instanceof")] InstanceOf,
}

/// `a && b`, `a || b`, `a ?? b`. Split from [`BinaryExpression`] because
/// these short-circuit — `right` is only evaluated when the truthiness
/// of `left` requires it. Optimization passes that reason about side
/// effects need this distinction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: LogicalOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "&&")] And,
    #[serde(rename = "||")] Or,
    #[serde(rename = "??")] NullishCoalescing,
}

/// Prefix unary: `-x`, `+x`, `!x`, `~x`, `typeof x`, `void x`, `delete x`.
/// `prefix: true` in v1. The **read-modify-write** operators `++` / `--` are
/// *not* here — they carry a side effect and a writability requirement the
/// pure unary operators do not, so they live in their own
/// [`UpdateExpression`] node (as in ESTree).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnaryExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: UnaryOperator,
    pub prefix: bool,
    pub argument: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    #[serde(rename = "-")] Negate,
    #[serde(rename = "+")] Plus,
    #[serde(rename = "!")] Not,
    #[serde(rename = "~")] BitNot,
    #[serde(rename = "typeof")] TypeOf,
    #[serde(rename = "void")] Void,
    #[serde(rename = "delete")] Delete,
}

/// Update expression: `++x`, `x++`, `--x`, `x--`. ESTree's `UpdateExpression`.
///
/// Kept distinct from [`UnaryExpression`] because `++` / `--` **mutate** their
/// operand — a read-modify-write — so unlike the pure unary operators they:
///   * have a side effect (a DCE or purity pass must NOT treat `x++` as
///     removable dead code, and a fold pass must NOT reorder past it), and
///   * require a *writable reference* as their argument (an identifier or a
///     member access), never an arbitrary value (`5++` is a syntax error).
///
/// `prefix` distinguishes the two evaluation orders, which differ in the value
/// the expression *yields* (the mutation of the operand is identical):
///
/// ```text
///   ++x   prefix:  increment x, then YIELD THE NEW value   (x was 1 → 2, yields 2)
///   x++   postfix: YIELD THE OLD value, then increment x   (x was 1, yields 1 → x is 2)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: UpdateOperator,
    pub prefix: bool,
    pub argument: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateOperator {
    #[serde(rename = "++")] Increment,
    #[serde(rename = "--")] Decrement,
}

/// `x = y`, `x += y`, etc. The target is an [`AssignmentTarget`]
/// (identifier or member expression in Phase 1; destructuring patterns
/// land in Phase 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub operator: AssignmentOperator,
    pub left: AssignmentTarget,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentOperator {
    #[serde(rename = "=")] Eq,
    #[serde(rename = "+=")] AddEq,
    #[serde(rename = "-=")] SubEq,
    #[serde(rename = "*=")] MulEq,
    #[serde(rename = "/=")] DivEq,
    #[serde(rename = "%=")] ModEq,
    #[serde(rename = "**=")] ExpEq,
    #[serde(rename = "<<=")] LeftShiftEq,
    #[serde(rename = ">>=")] RightShiftEq,
    #[serde(rename = ">>>=")] UnsignedRightShiftEq,
    #[serde(rename = "|=")] BitOrEq,
    #[serde(rename = "^=")] BitXorEq,
    #[serde(rename = "&=")] BitAndEq,
    // Phase 5: LogicalAndEq (&&=), LogicalOrEq (||=),
    // NullishCoalescingEq (??=) once we have ES2021 in scope.
}

/// The left-hand-side of an `AssignmentExpression`. Identifier or member
/// expression in Phase 1; destructuring patterns are Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssignmentTarget {
    Identifier(Identifier),
    MemberExpression(Box<MemberExpression>),
}

/// `test ? consequent : alternate`. The ternary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionalExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub test: Box<Expression>,
    pub consequent: Box<Expression>,
    pub alternate: Box<Expression>,
}

// ---------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------

/// `f(x, y, z)`. `callee` is an expression — typically an identifier or
/// a member expression but in principle any expression that evaluates
/// to a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

/// `new Ctor(a, b)` — construction via the `new` operator. Structurally a
/// [`CallExpression`] with a `new` keyword, but a **distinct** node because
/// its evaluation semantics and its precedence rules differ:
///
/// # Semantics
///
/// `new Ctor(args)` allocates a fresh object, runs `Ctor` with `this` bound to
/// it, and yields the object (unless the constructor returns its own object).
/// A [`CallExpression`] `Ctor(args)` merely calls the function. A pass may not
/// rewrite one into the other.
///
/// # `arguments` may be empty
///
/// `new Ctor` (no parens) and `new Ctor()` (empty parens) are the **same**
/// program — both carry `arguments: vec![]`. The two source spellings collapse
/// to one node; the emitter chooses a canonical spelling.
///
/// # Precedence
///
/// Per the ECMAScript grammar the *argumented* form is a `MemberExpression`
/// (`new MemberExpression Arguments`) while the *bare* form is a lower
/// `NewExpression` (`new NewExpression`), so in source they bind differently:
///
/// ```text
///   new X().y     parses as   (new X()).y     ← argumented: member binds after
///   new X.y       parses as   new (X.y)       ← bare: the whole member is the target
/// ```
///
/// The emitter sidesteps the split by **always printing the argument parens** —
/// a no-argument node is emitted canonically as `new X()`, never bare `new X`.
/// Every emitted `new` is therefore the argumented form and binds at
/// member/call strength, so `new X().y` needs no extra parens. (Dropping the
/// empty parens as a size win is a possible future optimization; it would
/// reintroduce the looser bare-form precedence.)
///
/// # The callee (`callee`) excludes a trailing call
///
/// The `new` target is a `MemberExpression`, which by grammar cannot *be* a
/// call. So if the callee's member spine bottoms out in a call, the target
/// must be parenthesised or the `(args)` we emit would bind to the *inner*
/// call instead:
///
/// ```text
///   new (f())()   NOT   new f()()   (which is (new f())() — a different program)
/// ```
///
/// The emitter's `emit_new` parenthesises exactly this case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

/// `a, b, c` — the **comma operator**. Evaluates each expression left to right
/// and yields the value of the **last** one; the earlier expressions are
/// evaluated only for their side effects. `expressions` holds the operands in
/// source order and always has length ≥ 2 (a single expression is just that
/// expression, not a sequence).
///
/// # Precedence — the loosest expression there is
///
/// The comma operator binds **looser than assignment** (it is the entry
/// production `Expression : AssignmentExpression { , AssignmentExpression }`).
/// A sequence used as a *sub-expression* almost always needs parentheses, or
/// the surrounding operator captures only one arm:
///
/// ```text
///   x = (a, b)     without parens `x = a, b` parses as `(x = a), b`
///   f((a, b), c)   without parens `f(a, b, c)` is a THREE-argument call
///   [(a, b), c]    without parens `[a, b, c]` is a THREE-element array
///   return (a, b)  a bare `return a, b` still works (statement position)
/// ```
///
/// The two places a sequence needs **no** parens are a statement-position
/// expression (`a, b, c;`) and a computed-member key (`obj[a, b]` — the `[ ]`
/// already delimits a full `Expression`). The emitter encodes this by tagging
/// `SequenceExpression` at the lowest precedence and emitting the four
/// assignment-position operands (call/`new` arguments, array elements,
/// assignment RHS) at assignment precedence so a sequence there wraps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub expressions: Vec<Expression>,
}

/// A **spread element** `...arg` — the `...` prefix that unpacks an iterable
/// (or, in an object literal, an object's own enumerable properties) into the
/// surrounding list. Three positions accept one:
///
/// ```text
///   f(...args)        spread a call/`new` argument list
///   [1, ...rest, 2]   spread an array-literal element
///   {...defaults}     spread object properties (object-spread)
/// ```
///
/// Unlike every other `Expression` variant, a `SpreadElement` is **not a
/// free-standing expression**: `...x` is a syntax error on its own, and the
/// grammar only permits it as an *argument* (call / `new`) or an *element*
/// (array literal). We nonetheless model it as an `Expression` variant so it
/// can slot directly into the existing `Vec<Expression>` argument/element
/// lists that `CallExpression`, `NewExpression`, and `ArrayExpression` already
/// carry — no parallel "argument-or-spread" enum is needed, and every AST
/// walker that already recurses those `Vec`s reaches the spread's inner
/// `argument` for free.
///
/// **Precedence.** The `argument` is an `AssignmentExpression` in the grammar,
/// so the emitter prints it at assignment precedence: a plain `...a`, a
/// conditional `...a?b:c`, or an assignment `...a=b` all print bare, while a
/// looser **sequence** argument is the one case that must be parenthesised —
/// `...(a,b)` — because a bare `...a,b` would spread only `a` and leave `,b`
/// as a sibling list element. The spread node itself is tagged at assignment
/// precedence too, which matches the assignment-position list slots it lives
/// in (`f(...a)`, `[...a]`) so it is never spuriously wrapped there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadElement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub argument: Box<Expression>,
}

/// A `yield` expression — the pause/resume operator that only appears inside a
/// generator function body:
///
/// ```text
///   yield              a bare yield, no operand      (delegate = false, argument = None)
///   yield value        yield a single value          (delegate = false, argument = Some)
///   yield* iterable    delegate to another iterable  (delegate = true,  argument = Some)
/// ```
///
/// # Two independent axes
///
/// A yield carries two bits of shape, kept as separate fields so the emitter
/// and every AST walker can reason about them independently:
///
/// - **`delegate`** — `false` for `yield` / `yield x`, `true` for `yield* x`.
///   The `*` turns a yield into a *delegating* yield that forwards every value
///   its `argument` iterable produces. A delegating yield **must** have an
///   argument (bare `yield*` is a syntax error); a non-delegating yield may or
///   may not.
/// - **`argument`** — `None` for a bare `yield`, `Some(expr)` otherwise. When
///   present it is an `AssignmentExpression` in the grammar (the loosest
///   operand), so the emitter prints it at assignment precedence.
///
/// | source        | delegate | argument      |
/// |---------------|----------|---------------|
/// | `yield`       | `false`  | `None`        |
/// | `yield x`     | `false`  | `Some(x)`     |
/// | `yield* xs`   | `true`   | `Some(xs)`    |
///
/// # Precedence
///
/// `yield` is an `AssignmentExpression` alternative in the grammar — it binds
/// looser than every operator except the comma. The emitter therefore tags it
/// at assignment precedence: as an assignment RHS or a conditional branch it
/// prints bare (`x = yield v`, `c ? yield a : yield b`), but any tighter parent
/// — a binary operand, a call argument that is *not* already an
/// assignment-position slot, a member object — wraps it into `(yield v)`.
///
/// Like the other operand-carrying nodes we model `argument` as a `Box` to keep
/// the `Expression` enum a fixed size regardless of how deep the yielded
/// expression nests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YieldExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub delegate: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub argument: Option<Box<Expression>>,
}

/// An `await` expression — the async-suspend operator that only appears inside
/// an async function body:
///
/// ```text
///   await promise      suspend until `promise` settles, then resume with its value
/// ```
///
/// # Shape
///
/// Unlike [`YieldExpression`], an `await` **always** has an operand — bare
/// `await` is a syntax error — so `argument` is a plain (non-optional)
/// `Box<Expression>`, not an `Option`. There is no delegating form (`await*`
/// does not exist), so no second axis is needed.
///
/// # Precedence
///
/// `await` is a **unary** operator in the grammar (`await UnaryExpression`),
/// binding exactly like the word-prefix unaries `typeof` / `void` / `delete`:
/// tighter than every binary operator, looser than member/call. So the emitter
/// tags it at unary precedence — `await a + b` prints bare on the left
/// (`await a+b`, i.e. `(await a)+b`), while a looser operand wraps
/// (`await (a+b)`), and a member/call parent wraps the whole await
/// (`(await p).x`, `(await f)()`). The operand likewise prints at unary
/// precedence.
///
/// Like the other operand-carrying nodes `argument` is a `Box` so the
/// `Expression` enum stays a fixed size regardless of nesting depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub argument: Box<Expression>,
}

/// A **dynamic `import()`** — `import(specifier)`, the runtime module-loading
/// form that returns a promise for the module namespace (distinct from a static
/// `import` *declaration*, and distinct from the [`ImportMeta`] meta-property).
/// ESTree calls this node `ImportExpression`, with a single `source` operand.
///
/// # Shape
///
/// Unlike the reserved-word leaf primaries ([`ThisExpression`], [`Super`],
/// [`NewTarget`], [`ImportMeta`]), a dynamic import **always** has exactly one
/// operand — the module specifier — so `source` is a plain (non-optional)
/// `Box<Expression>`. `import()` with no argument is a syntax error. There is no
/// second axis (options-bag / attributes are a later proposal not modelled
/// here).
///
/// # Precedence
///
/// `import(x)` is spelled as the `import` keyword immediately followed by a
/// parenthesised argument — syntactically a **call-like primary**. The literal
/// parens make the whole node atomic from the outside (it binds at
/// `PREC_PRIMARY`, like a `CallExpression`: `import(x).then(f)`,
/// `(await import(x))` compose without extra parens around the import itself).
/// The `source` sits *inside* those literal parens, so it is emitted at the
/// lowest (assignment) precedence and never needs its own wrapping — the parens
/// are part of the `import(...)` spelling, not a precedence artifact.
///
/// Like the other operand-carrying nodes `source` is a `Box` so the
/// `Expression` enum stays a fixed size regardless of nesting depth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub source: Box<Expression>,
}

/// The `this` keyword — a primary expression that reads the current
/// execution context's `this` binding:
///
/// ```text
///   this            the receiver of the enclosing method / the global-or-undefined
///   this.x          member access off the receiver
///   f(this)         pass the receiver as an argument
/// ```
///
/// # Shape
///
/// `this` is a **leaf** — it carries no operand and no payload beyond its
/// `cv` (the same shape as [`NullLiteral`] / [`UndefinedLiteral`]). ESTree
/// models it as its own `ThisExpression` node rather than an
/// `Identifier { name: "this" }`, because `this` is a reserved word: it can
/// never be a variable name, so passes that rename identifiers must never
/// touch it. Giving it a dedicated variant lets those passes pattern-match
/// it out structurally instead of string-comparing every identifier name.
///
/// # Precedence
///
/// `this` binds at **primary** strength — the tightest level, like an
/// identifier or a parenthesised group. It never needs wrapping in any
/// parent context, and no parent operand ever forces a paren around it. The
/// emitter therefore tags it `PREC_PRIMARY` and prints the bare keyword.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThisExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// The `super` keyword — a primary that names the *home object's* prototype
/// (in a method) or the *superclass constructor* (in a derived
/// constructor):
///
/// ```text
///   super.method()     call an overridden method on the prototype chain
///   super[key]         computed member access off the home object's proto
///   super(a, b)        delegate to the superclass constructor (constructor only)
/// ```
///
/// # Shape
///
/// `super` is a **leaf** — like [`ThisExpression`], it carries no operand and
/// no payload beyond its `cv`. ESTree models it as its own `Super` node
/// rather than an `Identifier { name: "super" }`, because `super` is a
/// reserved word: it can never be a variable name, so passes that rename
/// identifiers must never touch it. Note that `super` is *syntactically*
/// restricted — it may appear only as the object of a member access or as a
/// call callee, and only inside a method/derived-constructor — but that is
/// the **parser's** concern: the AST and emitter treat it as a plain leaf
/// primary and print it wherever it is placed.
///
/// # Precedence
///
/// `super` binds at **primary** strength — the tightest level, exactly like
/// `this`. It never needs wrapping in any parent context, and no parent
/// operand ever forces a paren around it. The emitter tags it `PREC_PRIMARY`
/// and prints the bare keyword.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Super {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// The `new.target` meta-property — a primary that reads the constructor a
/// function was invoked with (`undefined` for a plain call, the constructor
/// for a `new` call):
///
/// ```text
///   function F() { if (new.target) { … } }   // was F called with `new`?
///   class C { constructor() { new.target } } // the actual (sub)class
/// ```
///
/// # Shape
///
/// `new.target` is a **leaf** — like [`ThisExpression`] and [`Super`], it
/// carries no operand and no payload beyond its `cv`. In source it is spelled
/// with two tokens (`new` `.` `target`), but semantically it is an atomic
/// meta-property, so ESTree models it as its own `MetaProperty`-style node
/// (here `NewTarget`) rather than a member access. It can never be a variable
/// name, so the renaming passes must never touch it — a dedicated variant lets
/// those passes exclude it structurally.
///
/// # Precedence
///
/// `new.target` binds at **primary** strength — the tightest level, like
/// `this` / `super`. It never needs wrapping in any parent context, and no
/// parent operand ever forces a paren around it. The emitter tags it
/// `PREC_PRIMARY` and prints the literal two-token spelling `new.target`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTarget {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// `import.meta` — the module meta-property exposing host-provided metadata
/// about the current module (e.g. `import.meta.url`). It is the sibling of
/// [`NewTarget`]: ESTree models both as `MetaProperty` nodes, so `import.meta`
/// gets its own atomic variant here rather than being a member access of an
/// `import` identifier (`import` is a reserved word — there is no such
/// identifier to access).
///
/// # Shape
///
/// `import.meta` is a **leaf** — like [`ThisExpression`], [`Super`] and
/// [`NewTarget`] it carries no operand and no payload beyond its `cv`. In
/// source it is spelled with three tokens (`import` `.` `meta`), but
/// semantically it is one atomic meta-property. Because `import` can never be a
/// variable name, the renaming passes (`rename`, `rename-globals`,
/// `rename-properties`) must never touch it — a dedicated variant lets those
/// passes exclude it structurally instead of string-matching the spelling.
///
/// # Precedence
///
/// `import.meta` binds at **primary** strength — the tightest level, exactly
/// like `this` / `super` / `new.target`. It never needs wrapping in any parent
/// context (`import.meta.url`, `f(import.meta)`, `import.meta+1` all print
/// bare), and no parent operand ever forces a paren around it. The emitter tags
/// it `PREC_PRIMARY` and prints the literal three-token spelling `import.meta`.
/// The `.meta` is part of the fixed spelling, NOT a member access — a genuine
/// property read such as `import.meta.url` wraps this leaf in an outer
/// [`MemberExpression`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMeta {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

/// `obj.prop` or `obj[key]`. `computed = false` ↔ `obj.prop` (the
/// `property` is conventionally an [`Expression::Identifier`]);
/// `computed = true` ↔ `obj[key]` (the `property` is any expression).
///
/// The distinction between dot- and bracket-access matters: with
/// `Symbol` keys, `obj[sym]` and `obj.sym` reach different properties.
/// Per the ESTree spec the property is always typed `Expression` (the
/// JSON wire format makes them indistinguishable by shape — both write
/// `{"object": ..., "property": ..., "computed": bool}`). When
/// `computed = false`, the parser is required to emit an `Identifier`
/// as the `property`; tools that walk the tree can assert this if
/// they care.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub object: Box<Expression>,
    pub property: Box<Expression>,
    pub computed: bool,
}

/// Deprecated convenience alias kept for the brief window between
/// spec write-up and implementation; downstream code should use
/// `Box<Expression>` directly. Will be removed in Phase 1.1.
#[deprecated = "Use Box<Expression> directly on MemberExpression.property"]
pub type MemberProperty = Box<Expression>;

// ---------------------------------------------------------------------
// Optional chaining — `a?.b`, `a?.[k]`, `a?.()`  (ES2020)
// ---------------------------------------------------------------------
//
// # Why separate node types rather than an `optional: bool` flag?
//
// ESTree-7 marks optional links by setting `optional: true` on the regular
// `MemberExpression`/`CallExpression` and wrapping the whole chain in a
// `ChainExpression`. We instead give the **optional links their own node
// types** (`OptionalMemberExpression`, `OptionalCallExpression`) and keep a
// `ChainExpression` wrapper only as the chain-boundary marker. Two reasons:
//
//   1. **It matches our conformance target.** Google's Closure Compiler (the
//      thing this crate clones) represents optional access with *dedicated*
//      Rhino node kinds — `OPTCHAIN_GETPROP` (`a?.b`), `OPTCHAIN_GETELEM`
//      (`a?.[k]`), and `OPTCHAIN_CALL` (`a?.()`) — not a flag on the ordinary
//      `GETPROP`/`GETELEM`/`CALL`. Modelling the same shape keeps the printer
//      and passes aligned with upstream behaviour.
//   2. **It is purely additive.** `MemberExpression` and `CallExpression`
//      have ~150 construction sites across the workspace; bolting a new
//      required field onto them would touch every one. A new variant only
//      adds `match` arms where a pass actually cares.
//
// # How a chain is shaped
//
// A run of `?.` / `.` / `[]` / `()` suffixes builds a left-leaning spine, and
// the **whole spine** is wrapped once in a [`ChainExpression`] (the boundary
// at which the `undefined` short-circuit resolves). A non-optional suffix
// that follows an optional one is still an *ordinary* `MemberExpression` /
// `CallExpression` whose object/callee is the optional node — so no `optional`
// flag is needed to print `a?.b.c` (only the `b` link carries `?.`):
//
// ```text
//   a?.b       ChainExpression( OptionalMember{ a, b } )
//   a?.[k]     ChainExpression( OptionalMember{ a, k, computed } )
//   a?.()      ChainExpression( OptionalCall{ a, [] } )
//   a?.b.c     ChainExpression( Member{ OptionalMember{ a, b }, c } )
//   a?.b()     ChainExpression( Call{ OptionalMember{ a, b }, [] } )
// ```
//
// The printer is the unit under test in PR1; the bridge that *builds* these
// nodes from the grammar's `optional_chain_expression` rule is CLOC12.171 PR2.

/// `obj?.prop` (dot) or `obj?.[prop]` (computed) — an **optional** member
/// access. If `obj` is `null`/`undefined` the whole surrounding
/// [`ChainExpression`] short-circuits to `undefined` instead of throwing.
/// Structurally identical to [`MemberExpression`]; the distinct type is what
/// records that this link was written with `?.`. Mirrors Closure's
/// `OPTCHAIN_GETPROP` / `OPTCHAIN_GETELEM`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalMemberExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub object: Box<Expression>,
    pub property: Box<Expression>,
    pub computed: bool,
}

/// `callee?.(args)` — an **optional** call. If `callee` is `null`/`undefined`
/// the surrounding [`ChainExpression`] short-circuits to `undefined` rather
/// than throwing "not a function". Structurally a [`CallExpression`]; the
/// distinct type records the `?.(` spelling. Mirrors Closure's
/// `OPTCHAIN_CALL`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionalCallExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
}

/// The boundary wrapper around an optional chain (ESTree `ChainExpression`).
/// It marks *where* the `undefined` short-circuit resolves: everything inside
/// one `ChainExpression` shares a single short-circuit, so `a?.b.c` yields
/// `undefined` (not a `TypeError`) when `a` is nullish. It carries no syntax
/// of its own — the printer emits its `expression` transparently — so
/// wrapping is invisible in the output while remaining observable to passes
/// that must not hoist a `.c` out of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub expression: Box<Expression>,
}

// ---------------------------------------------------------------------
// Composites
// ---------------------------------------------------------------------

/// `[a, b, c]`. `None` in the elements vector represents an *elision*
/// (a hole) — `[1, , 3]` produces `Vec[Some(1), None, Some(3)]`.
/// Elisions are distinct from `undefined` (you can check via `in`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrayExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub elements: Vec<Option<Expression>>,
}

/// `{ key1: value1, key2: value2 }`. Member insertion order is
/// preserved per ES2015+ — `Object.keys` is observable. Supports
/// `Init` properties (`{k: v}`), getters, setters, shorthand
/// `{k}`, methods `{k() {}}`, and **object spread** `{...x}` (ES2018) —
/// see [`ObjectMember`] for how the two member shapes intermix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub properties: Vec<ObjectMember>,
}

/// One entry in an [`ObjectExpression`]'s `properties` list — either a normal
/// key/value [`Property`] or an object **spread** `...expr` (ES2018) that copies
/// another object's own enumerable properties in.
///
/// # Why an enum rather than two parallel vectors
///
/// ESTree intermixes `Property` and `SpreadElement` nodes in the *same*
/// `properties` array, and the order is **observable**: a later member's key
/// overrides an earlier one, and a spread may sit before, between, or after
/// plain properties (`{a: 1, ...o, b: 2}`). Modelling the list as a single
/// `Vec<ObjectMember>` preserves that source order for every walker for free —
/// splitting spreads into a side channel would lose the interleaving and
/// silently miscompile override semantics.
///
/// # Why the spread reuses [`SpreadElement`]
///
/// The object-spread carries exactly what the call/array spread carries — a
/// single `argument` expression and its provenance `cv` — and prints the same
/// way (`...expr`). Reusing the existing node keeps one spread shape and one
/// emit path (`emit_object_spread` mirrors `emit_spread`), rather than a
/// near-duplicate struct. ESTree names both `SpreadElement`, so this matches
/// the reference tree too.
///
/// ```text
///   {a: 1}          → Property        a normal key/value member
///   {...o}          → Spread          copy o's own enumerable props
///   {a: 1, ...o}    → [Property, Spread]   order is observable
/// ```
///
/// Serialised `#[serde(untagged)]`: the `Property` arm carries its own
/// `"type": "Property"` tag and the `Spread` arm carries `SpreadElement`'s
/// `argument` field, so a member deserialises unambiguously (only a spread has
/// `argument`; only a property has `kind`/`key`/`value`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ObjectMember {
    /// A normal `k: v` / shorthand / getter / setter / method property.
    Property(Property),
    /// An object spread `...expr`. Reuses [`SpreadElement`] (`{ cv, argument }`).
    Spread(SpreadElement),
}

/// One entry of an [`ObjectExpression`]. ESTree calls this `Property`;
/// when serialized inside an object expression it carries a
/// `"type": "Property"` tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "Property", rename_all = "camelCase")]
pub struct Property {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub kind: PropertyKind,
    pub key: PropertyKey,
    pub value: Box<Expression>,
    pub computed: bool,
    pub shorthand: bool,
    pub method: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyKind {
    Init,
    Get,
    Set,
}

/// The key side of a [`Property`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropertyKey {
    Identifier(Identifier),
    StringLiteral(StringLiteral),
    NumericLiteral(NumericLiteral),
    /// When the property is `[expr]: value` (computed = true).
    Expression(Box<Expression>),
}

// ---------------------------------------------------------------------
// FunctionExpression — a function used in value position
// ---------------------------------------------------------------------

/// `function (p1, p2) { body }` or the *named* form
/// `function f(p1, p2) { body }`, appearing where an **expression** is
/// expected — the right side of an assignment (`var g = function () {}`),
/// an argument (`arr.map(function (x) { return x; })`), a property value
/// (`{ run: function () {} }`), or the callee of an IIFE
/// (`(function () {})()`).
///
/// # How it differs from [`crate::FunctionDeclaration`]
///
/// A declaration *binds a name in the enclosing scope* and is a
/// statement; an expression *produces a function value* and its name
/// (if any) is visible **only inside its own body** (so a named
/// function expression can recurse by its own name without leaking that
/// name outward). That single semantic difference is exactly why `id`
/// here is `Option<Identifier>` — anonymous function expressions are the
/// common case — whereas a declaration's `id` is mandatory.
///
/// ```text
///   declaration:  function f(){}      f is bound in the outer scope
///   expression:   (function f(){})    f is bound ONLY inside the body
///   expression:   (function (){})     anonymous — no name at all
/// ```
///
/// `params`, `body`, `generator`, and `is_async` carry the identical
/// meaning as on [`crate::FunctionDeclaration`], and the two share the
/// [`FunctionParam`] and [`BlockStatement`] types so passes that walk a
/// function body do not care which form produced it.
///
/// Added in Phase 1.x (CLOC09 §"future Phase 1.x FunctionExpression").
/// Arrow functions, methods, getters/setters, and class expressions
/// remain Phase 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// `None` for an anonymous `function () {}`; `Some` for a named
    /// `function f() {}` expression (the name is body-local — see the
    /// type-level docs).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<Identifier>,
    pub params: Vec<FunctionParam>,
    pub body: BlockStatement,
    pub generator: bool,
    #[serde(rename = "async")]
    pub is_async: bool,
}

// ---------------------------------------------------------------------
// ClassExpression — `class [id] [extends S] { members }` in value position
// ---------------------------------------------------------------------

/// A **class expression** — the `class` keyword used where an *expression*
/// is expected (the right side of an assignment `var C = class {}`, an
/// argument `f(class {})`, an IIFE callee `(class {})`, etc.). The sibling
/// *declaration* form (`class C {}` as a statement) is a separate node that a
/// later arc adds; the two share the member sub-AST introduced here
/// (CLOC12.173).
///
/// ```text
///   class {}                     anonymous, empty
///   class C {}                   named
///   class C extends B {}         with heritage
///   class { m() {} }             with a method
///   class { static m() {} }      a static method
///   class { get x() {} }         an accessor
///   class { [k]() {} }           a computed-key method
/// ```
///
/// The shape mirrors [`FunctionExpression`] where it can: `id` is the optional
/// class name, `super_class` is the optional `extends` operand (any expression,
/// e.g. `extends mixin(Base)`), and `body` is the ordered list of members. Only
/// **methods** are modelled today (`ClassMember::Method`); instance/static
/// fields and `static { … }` blocks are additive follow-ups (see the spec).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// `None` for anonymous `class {}`; `Some` for a named `class C {}`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<Identifier>,
    /// The `extends <expr>` operand, if any. Any expression is legal here —
    /// an identifier (`extends Base`), a member (`extends ns.Base`), or a call
    /// (`extends mixin(Base)`) — so it is a boxed [`Expression`].
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "superClass")]
    pub super_class: Option<Box<Expression>>,
    /// The class body — the ordered member list between the braces. May be
    /// empty (`class {}`).
    pub body: Vec<ClassMember>,
}

/// One member of a class body. An enum (not a bare struct) because a later arc
/// adds field (`PropertyDefinition`) and `static { … }` (static-block) variants
/// alongside the method form; today only methods are representable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClassMember {
    /// A method / accessor / constructor: `m() {}`, `get x() {}`,
    /// `static m() {}`, `[k]() {}`, `constructor() {}`.
    Method(MethodDefinition),
    /// A **field** (class property): `x = 1;`, `y;`, `static z = 2;`,
    /// `[k] = v;`. A named slot with an *optional* initializer — no parameter
    /// list, no body. See [`PropertyDefinition`].
    Field(PropertyDefinition),
    /// A **static initialization block**: `static { … }`. A block of statements
    /// that runs *once, at class-definition time*, in the class's static scope.
    /// It has no name, no key, no parameter list, and no initializer — only a
    /// body, which is exactly a [`BlockStatement`]'s statement list, so the
    /// variant wraps `BlockStatement` directly rather than re-modelling it.
    /// (Private members `#x` and decorators are later additive variants.)
    StaticBlock(BlockStatement),
}

/// A method-like class member. Unlike an object-literal [`Property`] it can be
/// `static`, can be the `constructor`, and its value is *always* a
/// [`FunctionExpression`] (the method's params + body). The key reuses
/// [`PropertyKey`] — a method key has the same four shapes as a property key
/// (identifier / string / numeric / computed `[expr]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodDefinition {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The member name. For a computed key (`[expr]() {}`) this is
    /// [`PropertyKey::Expression`] and `computed` is `true`.
    pub key: PropertyKey,
    /// Which kind of member: an ordinary method, the constructor, or a
    /// getter/setter accessor.
    pub kind: MethodKind,
    /// The method body as a function value — its `params` and `body` are the
    /// method's parameter list and block. `id` is normally `None` (a method has
    /// no inner function name); `generator`/`is_async` carry `*`/`async`.
    pub value: FunctionExpression,
    /// `true` for a computed key `[expr]() {}`; `false` for a plain
    /// identifier / string / numeric key.
    pub computed: bool,
    /// `true` for a `static` member.
    #[serde(rename = "static")]
    pub is_static: bool,
}

/// A **class field** (property) member: `x = 1;`, `y;`, `static z = 2;`,
/// `[k] = v;`. The first non-method member kind (CLOC12.175).
///
/// # How it differs from [`MethodDefinition`]
///
/// A field is *not* a function-like member — it has:
///
/// | aspect      | `MethodDefinition`            | `PropertyDefinition` (field)      |
/// |-------------|-------------------------------|-----------------------------------|
/// | kind        | `MethodKind` (ctor/get/set/…) | none — a field is never any of    |
/// | value       | always a `FunctionExpression` | an *optional* plain `Expression`  |
/// | params/body | yes (the method function)     | none                              |
///
/// This matches the conformance target — Closure's Rhino uses a dedicated
/// `MEMBER_FIELD_DEF` node, distinct from `MEMBER_FUNCTION_DEF`. The `key`
/// reuses [`PropertyKey`] (a field key has the same four shapes as a method /
/// property key: identifier / string / numeric / computed `[expr]`), and
/// `computed` / `is_static` mirror [`MethodDefinition`]'s fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The field name. For a computed key (`[expr] = v`) this is
    /// [`PropertyKey::Expression`] and `computed` is `true`.
    pub key: PropertyKey,
    /// The initializer, if any. `Some` for `x = <expr>`; `None` for a bare
    /// `x;` field (declared but uninitialised).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<Expression>,
    /// `true` for a computed key `[expr] = v`; `false` for a plain
    /// identifier / string / numeric key.
    pub computed: bool,
    /// `true` for a `static` field.
    #[serde(rename = "static")]
    pub is_static: bool,
}

/// The four kinds of [`MethodDefinition`]. `Constructor` is distinct from
/// `Method` because the emitter and later passes treat the constructor
/// specially (e.g. it is never renamed and never a getter/setter). Mirrors
/// Closure's Rhino `MEMBER_FUNCTION_DEF` (method/constructor) vs `GETTER_DEF` /
/// `SETTER_DEF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MethodKind {
    /// `constructor() {}`.
    Constructor,
    /// An ordinary method `m() {}`.
    Method,
    /// A getter `get x() {}`.
    Get,
    /// A setter `set x(v) {}`.
    Set,
}

// ---------------------------------------------------------------------
// ArrowFunctionExpression — the `=>` form
// ---------------------------------------------------------------------

/// `x => x + 1`, `(a, b) => a + b`, `() => {}`, `async x => await f(x)`.
/// A function value written with the fat-arrow shorthand.
///
/// # How it differs from [`FunctionExpression`]
///
/// Three structural simplifications — each reflects a real syntactic
/// rule of arrow functions, not an accident of our encoding:
///
/// 1. **No `id`.** An arrow function is *always anonymous*. There is no
///    `x => x` form that also names itself, so — unlike
///    [`FunctionExpression`], whose `id` is `Option<Identifier>` — an
///    arrow simply has no name field. (This matters to renaming passes:
///    a [`FunctionExpression`] must protect its own body-local name from
///    substitution; an arrow has no such name to protect, only params.)
/// 2. **No `generator`.** `x =>*` is not valid syntax — arrows cannot be
///    generators — so there is no `generator` flag.
/// 3. **A dual-shape [`body`](ArrowFunctionExpression::body).** A
///    classic function body is *always* a brace-delimited
///    [`BlockStatement`]. An arrow body is *either* a block
///    (`x => { return x; }`) *or* a bare **expression**
///    (`x => x`, the "concise body"). That fork is modelled by
///    [`ArrowBody`].
///
/// `params` and `is_async` carry the identical meaning as on
/// [`FunctionExpression`], sharing the [`FunctionParam`] type.
///
/// ```text
///   x => x + 1              one param, concise body
///   (a, b) => a + b         two params, concise body
///   () => {}                zero params, empty block body
///   x => { return x; }      one param, block body
///   async x => f(x)         async, concise body
/// ```
///
/// Added in Phase 1.x (CLOC12.151). Object-literal concise bodies
/// (`() => ({ a: 1 })`), destructuring params, and default params are
/// deferred with the wider Phase 3 pattern work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowFunctionExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub params: Vec<FunctionParam>,
    pub body: ArrowBody,
    #[serde(rename = "async")]
    pub is_async: bool,
}

/// The body of an [`ArrowFunctionExpression`] — a brace-delimited block,
/// or a single "concise" expression.
///
/// # Why an `#[serde(untagged)]` enum works here
///
/// Every [`Expression`] serialises with an internal `{"type": "…"}`
/// discriminant; a [`BlockStatement`] is a plain struct with **no**
/// `type` field. So the two shapes are unambiguous by inspection:
///
/// ```text
///   concise:  { "type": "Identifier", "name": "x" }   ← has "type"
///   block:    { "body": [ … ] }                       ← no "type"
/// ```
///
/// With the [`Expression`](ArrowBody::Expression) arm listed **first**,
/// serde tries it first: concise-body JSON matches immediately, while
/// block-body JSON (lacking `type`) is rejected by the internally-tagged
/// [`Expression`] and falls through to [`Block`](ArrowBody::Block). The
/// expression is [`Box`]ed to break the `Expression → ArrowFunctionExpression
/// → ArrowBody → Expression` type cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArrowBody {
    /// Concise body: `x => x + 1`. The arrow's value is this expression.
    Expression(Box<Expression>),
    /// Block body: `x => { return x + 1; }`. Identical to a classic
    /// function body — statements, explicit `return`, etc.
    Block(BlockStatement),
}

// ---------------------------------------------------------------------
// TemplateLiteral — backtick template strings
// ---------------------------------------------------------------------

/// A template literal: `` `hello ${name}, you are ${age} years old` ``.
///
/// A template interleaves fixed **string parts** (the [`quasis`]) with
/// embedded **`${…}` expressions**. The two vectors are stored in parallel
/// and always satisfy the ESTree invariant
///
/// ```text
///   quasis.len() == expressions.len() + 1
/// ```
///
/// — there is one more string part than expression, because a template
/// both *begins* and *ends* with a (possibly empty) string part. Emit
/// interleaves them: quasi₀ `${` expr₀ `}` quasi₁ `${` expr₁ `}` … quasiₙ.
///
/// ```text
///   `abc`                quasis=["abc"]           expressions=[]
///   `a${x}b`             quasis=["a","b"]         expressions=[x]
///   `${x}${y}`           quasis=["","",""]        expressions=[x,y]
/// ```
///
/// [`quasis`]: TemplateLiteral::quasis
///
/// Added in Phase 1.x (CLOC12.154). *Tagged* templates
/// (`` tag`…` ``) are a separate node (`TaggedTemplateExpression`,
/// Phase 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateLiteral {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// The `n + 1` fixed string segments, in source order.
    pub quasis: Vec<TemplateElement>,
    /// The `n` embedded `${…}` expressions, in source order. Interleaved
    /// between consecutive [`quasis`](TemplateLiteral::quasis).
    pub expressions: Vec<Expression>,
}

/// One fixed string segment of a [`TemplateLiteral`] — the text between
/// backticks and `${`, between `}` and `${`, or between `}` and the
/// closing backtick.
///
/// ESTree splits the text into `raw` (verbatim source, escapes intact) and
/// `cooked` (escape-processed value). `cooked` is `None` when the segment
/// contains an illegal escape sequence that is only legal in a *tagged*
/// template — such a segment has no cooked value. `tail` marks the final
/// quasi (the one before the closing backtick), matching ESTree's
/// `TemplateElement.tail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateElement {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    /// Verbatim source text of this segment (escape sequences intact).
    pub raw: String,
    /// Escape-processed value, or `None` for an illegal escape that is only
    /// valid in a tagged template.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cooked: Option<String>,
    /// `true` for the final segment (before the closing backtick).
    pub tail: bool,
}

/// `` tag`a${x}b` `` — a **tagged** template. The `tag` expression is called
/// with the template's cooked/raw string parts and its `${…}` substitution
/// values, rather than the template producing a plain string. Common tags:
/// `String.raw`, `gql`, `css`, `html`.
///
/// # Structure
///
/// ```text
///   String.raw`a${x}b`
///   └── tag ──┘└─ quasi ─┘
/// ```
///
/// `tag` is any expression that evaluates to a function (typically an
/// identifier or a member access — `String.raw`, `styled.div`). `quasi` is the
/// [`TemplateLiteral`] being tagged (same node as an untagged template; the
/// `raw` segments are the ones handed to the tag, and a segment may have
/// `cooked = None` for an escape that is illegal in an untagged template but
/// legal here).
///
/// # Precedence and the seam
///
/// A tagged template binds at member/call strength (`PREC_PRIMARY`): the
/// backtick follows the tag directly with no separator (`` a.b`x` ``), and a
/// looser tag is parenthesised (`` (a,b)`x` `` — though such a tag is unusual).
/// The `tag`↔`` ` `` boundary never fuses, so no seam space is needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggedTemplateExpression {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
    pub tag: Box<Expression>,
    pub quasi: TemplateLiteral,
}

#[cfg(test)]
mod tests {
    //! Round-trip tests for every Phase 1 expression variant. Each
    //! variant gets at least one test for each tracing mode (traced /
    //! untraced) — proving the wire format works both ways per the
    //! CLOC09 amendment.
    //!
    //! The structural property we assert is: `from_str(to_string(x)) == x`.
    //! The `"type"` tag is asserted in a separate test per variant when
    //! the variant adds a new tag name.
    use super::*;

    fn roundtrip(expr: Expression) -> Expression {
        let json = serde_json::to_string(&expr).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    fn type_tag(expr: &Expression) -> String {
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(expr).unwrap()).unwrap();
        v["type"].as_str().unwrap().to_string()
    }

    #[test]
    fn identifier_roundtrips_traced_and_untraced() {
        let traced = Expression::Identifier(Identifier {
            cv: Some("id.1".to_string()),
            name: "x".to_string(),
        });
        let untraced = Expression::Identifier(Identifier {
            cv: None,
            name: "x".to_string(),
        });
        assert_eq!(traced.clone(), roundtrip(traced.clone()));
        assert_eq!(untraced.clone(), roundtrip(untraced.clone()));
        assert_eq!(type_tag(&traced), "Identifier");
    }

    #[test]
    fn numeric_literal_roundtrips() {
        let n = Expression::NumericLiteral(NumericLiteral {
            cv: Some("n.1".to_string()),
            value: 42.0,
            raw: "42".to_string(),
        });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "NumericLiteral");
    }

    #[test]
    fn numeric_literal_untraced_omits_cv_in_json() {
        let n = Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: 1.5,
            raw: "1.5".to_string(),
        });
        let json = serde_json::to_string(&n).expect("serialize");
        assert!(!json.contains("\"cv\""), "untraced should omit cv; got {}", json);
        assert_eq!(n.clone(), roundtrip(n));
    }

    #[test]
    fn string_literal_roundtrips() {
        let s = Expression::StringLiteral(StringLiteral {
            cv: Some("s.1".to_string()),
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
        });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "StringLiteral");
    }

    #[test]
    fn boolean_literal_roundtrips() {
        let b = Expression::BooleanLiteral(BooleanLiteral {
            cv: Some("b.1".to_string()),
            value: true,
        });
        assert_eq!(b.clone(), roundtrip(b.clone()));
        assert_eq!(type_tag(&b), "BooleanLiteral");
    }

    #[test]
    fn null_literal_roundtrips() {
        let n = Expression::NullLiteral(NullLiteral {
            cv: Some("null.1".to_string()),
        });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "NullLiteral");
    }

    #[test]
    fn undefined_literal_roundtrips_traced() {
        let u = Expression::UndefinedLiteral(UndefinedLiteral {
            cv: Some("u.1".to_string()),
        });
        assert_eq!(u.clone(), roundtrip(u.clone()));
        assert_eq!(type_tag(&u), "UndefinedLiteral");
    }

    #[test]
    fn undefined_literal_untraced_omits_cv() {
        let u = Expression::UndefinedLiteral(UndefinedLiteral { cv: None });
        let json = serde_json::to_string(&u).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(u.clone(), roundtrip(u));
    }

    #[test]
    fn this_expression_roundtrips_traced() {
        let t = Expression::ThisExpression(ThisExpression { cv: Some("this.1".to_string()) });
        assert_eq!(t.clone(), roundtrip(t.clone()));
        assert_eq!(type_tag(&t), "ThisExpression");
    }

    #[test]
    fn this_expression_untraced_omits_cv() {
        let t = Expression::ThisExpression(ThisExpression { cv: None });
        let json = serde_json::to_string(&t).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(t.clone(), roundtrip(t));
    }

    #[test]
    fn super_roundtrips_traced() {
        let s = Expression::Super(Super { cv: Some("super.1".to_string()) });
        assert_eq!(s.clone(), roundtrip(s.clone()));
        assert_eq!(type_tag(&s), "Super");
    }

    #[test]
    fn super_untraced_omits_cv() {
        let s = Expression::Super(Super { cv: None });
        let json = serde_json::to_string(&s).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(s.clone(), roundtrip(s));
    }

    #[test]
    fn new_target_roundtrips_traced() {
        let n = Expression::NewTarget(NewTarget { cv: Some("newTarget.1".to_string()) });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "NewTarget");
    }

    #[test]
    fn new_target_untraced_omits_cv() {
        let n = Expression::NewTarget(NewTarget { cv: None });
        let json = serde_json::to_string(&n).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(n.clone(), roundtrip(n));
    }

    #[test]
    fn import_meta_roundtrips_traced() {
        let n = Expression::ImportMeta(ImportMeta { cv: Some("importMeta.1".to_string()) });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "ImportMeta");
    }

    #[test]
    fn import_meta_untraced_omits_cv() {
        let n = Expression::ImportMeta(ImportMeta { cv: None });
        let json = serde_json::to_string(&n).expect("serialize");
        assert!(!json.contains("\"cv\""), "expected no cv key; got {}", json);
        assert_eq!(n.clone(), roundtrip(n));
    }

    #[test]
    fn import_expression_roundtrips_traced() {
        // import("m") — a dynamic import with a string-literal specifier.
        let n = Expression::ImportExpression(ImportExpression {
            cv: Some("importExpr.1".to_string()),
            source: Box::new(Expression::StringLiteral(StringLiteral {
                cv: None,
                value: "m".to_string(),
                raw: "\"m\"".to_string(),
            })),
        });
        assert_eq!(n.clone(), roundtrip(n.clone()));
        assert_eq!(type_tag(&n), "ImportExpression");
    }

    #[test]
    fn import_expression_untraced_omits_cv() {
        let n = Expression::ImportExpression(ImportExpression {
            cv: None,
            source: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "spec".to_string(),
            })),
        });
        let json = serde_json::to_string(&n).expect("serialize");
        // The node's own `cv` is omitted; the `source` operand is still present.
        assert!(
            json.starts_with("{\"type\":\"ImportExpression\",\"source\""),
            "expected no node cv key; got {}",
            json
        );
        assert_eq!(n.clone(), roundtrip(n));
    }

    #[test]
    fn bigint_literal_decimal_roundtrips() {
        // 123n  — decimal-source bigint
        let b = Expression::BigIntLiteral(BigIntLiteral {
            cv: Some("bi.1".to_string()),
            value: "123".to_string(),
            raw: "123n".to_string(),
        });
        assert_eq!(b.clone(), roundtrip(b.clone()));
        assert_eq!(type_tag(&b), "BigIntLiteral");
    }

    #[test]
    fn bigint_literal_hex_preserves_raw() {
        // 0x1fn → value "31", raw "0x1fn"
        let b = Expression::BigIntLiteral(BigIntLiteral {
            cv: None,
            value: "31".to_string(),
            raw: "0x1fn".to_string(),
        });
        let json = serde_json::to_string(&b).expect("serialize");
        // cv omitted; value + raw preserved as strings.
        assert!(!json.contains("\"cv\""), "cv key should be absent; got {}", json);
        assert!(json.contains("\"value\":\"31\""), "value mismatch; got {}", json);
        assert!(json.contains("\"raw\":\"0x1fn\""), "raw mismatch; got {}", json);
        assert_eq!(b.clone(), roundtrip(b));
    }

    #[test]
    fn binary_expression_with_plus_serializes_operator() {
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: Some("e.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"+\""), "got {}", json);
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "BinaryExpression");
    }

    #[test]
    fn every_binary_operator_round_trips() {
        // Each operator value: construct → serialize → deserialize →
        // structural equal.
        let ops = [
            (BinaryOperator::Eq, "=="),
            (BinaryOperator::NotEq, "!="),
            (BinaryOperator::StrictEq, "==="),
            (BinaryOperator::StrictNotEq, "!=="),
            (BinaryOperator::Lt, "<"),
            (BinaryOperator::LtEq, "<="),
            (BinaryOperator::Gt, ">"),
            (BinaryOperator::GtEq, ">="),
            (BinaryOperator::LeftShift, "<<"),
            (BinaryOperator::RightShift, ">>"),
            (BinaryOperator::UnsignedRightShift, ">>>"),
            (BinaryOperator::Add, "+"),
            (BinaryOperator::Sub, "-"),
            (BinaryOperator::Mul, "*"),
            (BinaryOperator::Div, "/"),
            (BinaryOperator::Mod, "%"),
            (BinaryOperator::Exp, "**"),
            (BinaryOperator::BitOr, "|"),
            (BinaryOperator::BitXor, "^"),
            (BinaryOperator::BitAnd, "&"),
            (BinaryOperator::In, "in"),
            (BinaryOperator::InstanceOf, "instanceof"),
        ];
        for (op, expected) in ops {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected), "op {:?}", op);
            let back: BinaryOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn logical_expression_distinct_from_binary() {
        // `a && b` is LogicalExpression, not BinaryExpression — the
        // short-circuit semantics are observable.
        let e = Expression::LogicalExpression(LogicalExpression {
            cv: None,
            operator: LogicalOperator::And,
            left: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "a".to_string(),
            })),
            right: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "b".to_string(),
            })),
        });
        assert_eq!(type_tag(&e), "LogicalExpression");
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"&&\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn every_logical_operator_round_trips() {
        for (op, expected) in [
            (LogicalOperator::And, "&&"),
            (LogicalOperator::Or, "||"),
            (LogicalOperator::NullishCoalescing, "??"),
        ] {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
            let back: LogicalOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn unary_expression_prefix_true() {
        let e = Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"prefix\":true"));
        assert!(json.contains("\"operator\":\"!\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn every_unary_operator_round_trips() {
        for (op, expected) in [
            (UnaryOperator::Negate, "-"),
            (UnaryOperator::Plus, "+"),
            (UnaryOperator::Not, "!"),
            (UnaryOperator::BitNot, "~"),
            (UnaryOperator::TypeOf, "typeof"),
            (UnaryOperator::Void, "void"),
            (UnaryOperator::Delete, "delete"),
        ] {
            let json = serde_json::to_string(&op).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
            let back: UnaryOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn assignment_expression_roundtrips() {
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: Some("ae.1".to_string()),
            operator: AssignmentOperator::Eq,
            left: AssignmentTarget::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            }),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 42.0,
                raw: "42".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"=\""));
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "AssignmentExpression");
    }

    #[test]
    fn assignment_to_member_target() {
        // `obj.prop = value`.
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: AssignmentOperator::AddEq,
            left: AssignmentTarget::MemberExpression(Box::new(MemberExpression {
                cv: None,
                object: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "obj".to_string(),
                })),
                property: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "prop".to_string(),
                })),
                computed: false,
            })),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"operator\":\"+=\""));
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn conditional_expression_roundtrips() {
        let e = Expression::ConditionalExpression(ConditionalExpression {
            cv: Some("c.1".to_string()),
            test: Box::new(Expression::BooleanLiteral(BooleanLiteral {
                cv: None,
                value: true,
            })),
            consequent: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
            alternate: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            })),
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ConditionalExpression");
    }

    #[test]
    fn call_expression_roundtrips() {
        let e = Expression::CallExpression(CallExpression {
            cv: Some("call.1".to_string()),
            callee: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "f".to_string(),
            })),
            arguments: vec![
                Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
                Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 2.0,
                    raw: "2".to_string(),
                }),
            ],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "CallExpression");
    }

    #[test]
    fn member_expression_dot_vs_bracket() {
        // `obj.foo` — computed = false. ESTree convention: property
        // is an Identifier when not computed.
        let dot = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            })),
            property: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "foo".to_string(),
            })),
            computed: false,
        });
        // `obj[expr]` — computed = true; here property is a NumericLiteral
        // to make the difference structurally observable on round-trip
        // (untagged enums can otherwise ambiguously resolve identical
        // shapes).
        let bracket = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "obj".to_string(),
            })),
            property: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 0.0,
                raw: "0".to_string(),
            })),
            computed: true,
        });
        assert_ne!(dot, bracket);
        assert_eq!(dot.clone(), roundtrip(dot.clone()));
        assert_eq!(bracket.clone(), roundtrip(bracket.clone()));
        assert_eq!(type_tag(&dot), "MemberExpression");
    }

    #[test]
    fn array_expression_preserves_elisions() {
        // `[1, , 3]` — middle element is a hole.
        let e = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![
                Some(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                None, // elision
                Some(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 3.0,
                    raw: "3".to_string(),
                })),
            ],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ArrayExpression");
    }

    #[test]
    fn object_expression_with_property() {
        // `{ foo: 1 }`.
        let e = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![ObjectMember::Property(Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::Identifier(Identifier {
                    cv: None,
                    name: "foo".to_string(),
                }),
                value: Box::new(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                computed: false,
                shorthand: false,
                method: false,
            })],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ObjectExpression");
    }

    #[test]
    fn object_member_spread_round_trips() {
        // `{...o}` — an `ObjectMember::Spread` survives serialise→deserialise
        // under the `#[serde(untagged)]` representation (only a spread carries
        // `argument`; only a property carries `kind`/`key`/`value`).
        let e = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![ObjectMember::Spread(SpreadElement {
                cv: None,
                argument: Box::new(Expression::Identifier(Identifier {
                    cv: None,
                    name: "o".to_string(),
                })),
            })],
        });
        assert_eq!(e.clone(), roundtrip(e.clone()));
    }

    #[test]
    fn property_kind_serializes_lowercase() {
        // ESTree uses lowercase "init" / "get" / "set".
        for (k, expected) in [
            (PropertyKind::Init, "init"),
            (PropertyKind::Get, "get"),
            (PropertyKind::Set, "set"),
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            assert_eq!(json, format!("\"{}\"", expected));
        }
    }

    // -----------------------------------------------------------------
    // FunctionExpression (Phase 1.x, CLOC09) — the expression sibling of
    // FunctionDeclaration. Round-trips + tag + the `id`-is-optional and
    // `async`-renames-to-JSON contracts.
    // -----------------------------------------------------------------
    use crate::declaration::FunctionParam as TestFunctionParam;
    use crate::statement::{BlockStatement as TestBlock, ReturnStatement, Statement};

    /// `function f(x) { return x; }` used in value position.
    fn named_fn_expr() -> Expression {
        Expression::FunctionExpression(FunctionExpression {
            cv: Some("fe.1".to_string()),
            id: Some(Identifier { cv: None, name: "f".to_string() }),
            params: vec![TestFunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body: TestBlock {
                cv: None,
                body: vec![Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(Expression::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    })),
                })],
            },
            generator: false,
            is_async: false,
        })
    }

    /// Anonymous `function () {}`.
    fn anon_fn_expr() -> Expression {
        Expression::FunctionExpression(FunctionExpression {
            cv: None,
            id: None,
            params: vec![],
            body: TestBlock { cv: None, body: vec![] },
            generator: false,
            is_async: false,
        })
    }

    #[test]
    fn function_expression_named_roundtrips_and_tags() {
        let e = named_fn_expr();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "FunctionExpression");
    }

    #[test]
    fn function_expression_anonymous_roundtrips() {
        let e = anon_fn_expr();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "FunctionExpression");
    }

    #[test]
    fn function_expression_anonymous_omits_id_in_json() {
        // `id: None` must NOT appear in the wire format — an anonymous
        // function expression has no `id` key at all (ESTree uses
        // `"id": null`, but our `skip_serializing_if` omits it; both
        // deserialize back to `None`, which is what round-trip asserts).
        let json = serde_json::to_string(&anon_fn_expr()).expect("serialize");
        assert!(!json.contains("\"id\""), "anonymous fn-expr should omit id; got {}", json);
    }

    #[test]
    fn function_expression_async_key_renames() {
        // The `is_async` field serializes as JSON `"async"` (ESTree),
        // exactly as on FunctionDeclaration.
        let mut e = anon_fn_expr();
        if let Expression::FunctionExpression(f) = &mut e {
            f.is_async = true;
        }
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"async\":true"), "expected async key; got {}", json);
        assert!(!json.contains("isAsync"), "must not leak Rust field name; got {}", json);
        assert_eq!(e.clone(), roundtrip(e));
    }

    // -----------------------------------------------------------------
    // ArrowFunctionExpression (CLOC12.151)
    // -----------------------------------------------------------------

    /// `x => x` — one param, concise (expression) body.
    fn concise_arrow() -> Expression {
        Expression::ArrowFunctionExpression(ArrowFunctionExpression {
            cv: Some("ar.1".to_string()),
            params: vec![TestFunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body: ArrowBody::Expression(Box::new(Expression::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            }))),
            is_async: false,
        })
    }

    /// `() => { return x; }` — zero params, block body.
    fn block_arrow() -> Expression {
        Expression::ArrowFunctionExpression(ArrowFunctionExpression {
            cv: None,
            params: vec![],
            body: ArrowBody::Block(TestBlock {
                cv: None,
                body: vec![Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(Expression::Identifier(Identifier {
                        cv: None,
                        name: "x".to_string(),
                    })),
                })],
            }),
            is_async: false,
        })
    }

    #[test]
    fn arrow_concise_body_roundtrips_and_tags() {
        let e = concise_arrow();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ArrowFunctionExpression");
    }

    #[test]
    fn arrow_block_body_roundtrips_and_tags() {
        let e = block_arrow();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "ArrowFunctionExpression");
    }

    #[test]
    fn arrow_body_variants_are_distinguished_on_the_wire() {
        // The untagged ArrowBody must round-trip WITHOUT collapsing the
        // concise/block distinction: a concise body carries a `"type"`
        // discriminant, a block body does not, so serde picks the right
        // arm back. Regression guard for the untagged-enum ordering.
        let concise = concise_arrow();
        let block = block_arrow();
        assert_ne!(concise, block);
        assert_eq!(concise.clone(), roundtrip(concise));
        assert_eq!(block.clone(), roundtrip(block));
    }

    #[test]
    fn arrow_async_key_renames() {
        // `is_async` serializes as JSON `"async"`, exactly as on the
        // function forms.
        let mut e = concise_arrow();
        if let Expression::ArrowFunctionExpression(a) = &mut e {
            a.is_async = true;
        }
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"async\":true"), "expected async key; got {}", json);
        assert!(!json.contains("isAsync"), "must not leak Rust field name; got {}", json);
        assert_eq!(e.clone(), roundtrip(e));
    }

    // -----------------------------------------------------------------
    // TemplateLiteral (CLOC12.154)
    // -----------------------------------------------------------------

    fn quasi(raw: &str, tail: bool) -> TemplateElement {
        TemplateElement { cv: None, raw: raw.to_string(), cooked: Some(raw.to_string()), tail }
    }

    /// `` `abc` `` — no substitutions: one quasi, no expressions.
    fn no_sub_template() -> Expression {
        Expression::TemplateLiteral(TemplateLiteral {
            cv: Some("tl.1".to_string()),
            quasis: vec![quasi("abc", true)],
            expressions: vec![],
        })
    }

    /// `` `a${x}b` `` — one substitution: two quasis, one expression.
    fn one_sub_template() -> Expression {
        Expression::TemplateLiteral(TemplateLiteral {
            cv: None,
            quasis: vec![quasi("a", false), quasi("b", true)],
            expressions: vec![Expression::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
        })
    }

    #[test]
    fn template_no_substitution_roundtrips_and_tags() {
        let e = no_sub_template();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "TemplateLiteral");
    }

    #[test]
    fn template_with_substitution_roundtrips() {
        let e = one_sub_template();
        assert_eq!(e.clone(), roundtrip(e.clone()));
        assert_eq!(type_tag(&e), "TemplateLiteral");
    }

    #[test]
    fn template_invariant_quasis_is_one_more_than_expressions() {
        // The ESTree structural invariant: a template both begins and ends
        // with a string part, so there is always exactly one more quasi
        // than expression.
        for tmpl in [no_sub_template(), one_sub_template()] {
            if let Expression::TemplateLiteral(t) = &tmpl {
                assert_eq!(t.quasis.len(), t.expressions.len() + 1);
                assert!(t.quasis.last().unwrap().tail, "the final quasi must be tail");
            }
        }
    }

    #[test]
    fn template_element_omits_none_cooked_in_json() {
        // A quasi with an illegal-in-untagged escape has `cooked: None`,
        // which is omitted from the wire format and round-trips back to None.
        let e = Expression::TemplateLiteral(TemplateLiteral {
            cv: None,
            quasis: vec![TemplateElement {
                cv: None,
                raw: "\\unicode".to_string(),
                cooked: None,
                tail: true,
            }],
            expressions: vec![],
        });
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(!json.contains("\"cooked\""), "None cooked should be omitted; got {}", json);
        assert_eq!(e.clone(), roundtrip(e));
    }

    // ---- ClassMember::Field / PropertyDefinition (CLOC12.175) --------

    /// Build a `class { <members> }` expression for round-tripping field
    /// members through the wire format.
    fn class_with(members: Vec<ClassMember>) -> Expression {
        Expression::ClassExpression(ClassExpression {
            cv: None,
            id: None,
            super_class: None,
            body: members,
        })
    }

    fn ident_key(name: &str) -> PropertyKey {
        PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() })
    }

    #[test]
    fn class_field_with_initializer_roundtrips() {
        // `class { x = 1 }` — an instance field with a numeric initializer.
        let e = class_with(vec![ClassMember::Field(PropertyDefinition {
            cv: None,
            key: ident_key("x"),
            value: Some(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 1.0,
                raw: "1".to_string(),
            })),
            computed: false,
            is_static: false,
        })]);
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn class_field_bare_omits_value_in_json() {
        // `class { y }` — a bare field: `value: None` must be omitted from the
        // wire format and round-trip back to `None`.
        let e = class_with(vec![ClassMember::Field(PropertyDefinition {
            cv: None,
            key: ident_key("y"),
            value: None,
            computed: false,
            is_static: false,
        })]);
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(!json.contains("\"value\""), "None value should be omitted; got {json}");
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn class_field_static_serializes_static_camel() {
        // `class { static z = 2 }` — the `is_static` flag serializes as the
        // ESTree `static` key, not `is_static`.
        let e = class_with(vec![ClassMember::Field(PropertyDefinition {
            cv: None,
            key: ident_key("z"),
            value: Some(Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            })),
            computed: false,
            is_static: true,
        })]);
        let json = serde_json::to_string(&e).expect("serialize");
        assert!(json.contains("\"static\":true"), "got {json}");
        assert!(!json.contains("is_static"), "got {json}");
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn class_field_computed_key_roundtrips() {
        // `class { [a+b] = v }` — a computed-key field. The key is a
        // `BinaryExpression`, unambiguously `PropertyKey::Expression` (unlike a
        // bare identifier `[k]`, which `#[serde(untagged)]` `PropertyKey` would
        // collapse to `Identifier` on the wire — a pre-existing limitation
        // shared with method keys), so it round-trips as a computed key.
        let e = class_with(vec![ClassMember::Field(PropertyDefinition {
            cv: None,
            key: PropertyKey::Expression(Box::new(Expression::BinaryExpression(
                BinaryExpression {
                    cv: None,
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::Identifier(Identifier {
                        cv: None,
                        name: "a".to_string(),
                    })),
                    right: Box::new(Expression::Identifier(Identifier {
                        cv: None,
                        name: "b".to_string(),
                    })),
                },
            ))),
            value: Some(Expression::Identifier(Identifier { cv: None, name: "v".to_string() })),
            computed: true,
            is_static: false,
        })]);
        assert_eq!(e.clone(), roundtrip(e));
    }

    // ---- ClassMember::StaticBlock (CLOC12.176) -----------------------

    #[test]
    fn class_static_block_empty_roundtrips() {
        // `class { static {} }` — an empty static-init block.
        let e = class_with(vec![ClassMember::StaticBlock(TestBlock { cv: None, body: vec![] })]);
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn class_static_block_with_statement_roundtrips() {
        // `class { static { return } }` — a static block whose body carries a
        // statement (the statement list round-trips through the wire format).
        let e = class_with(vec![ClassMember::StaticBlock(TestBlock {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement { cv: None, argument: None })],
        })]);
        assert_eq!(e.clone(), roundtrip(e));
    }

    #[test]
    fn class_static_block_interleaved_with_field_roundtrips() {
        // `class { x = 1; static {} }` — a field and a static block coexist in
        // one body, in source order, and both survive the round-trip.
        let e = class_with(vec![
            ClassMember::Field(PropertyDefinition {
                cv: None,
                key: ident_key("x"),
                value: Some(Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                })),
                computed: false,
                is_static: false,
            }),
            ClassMember::StaticBlock(TestBlock { cv: None, body: vec![] }),
        ]);
        assert_eq!(e.clone(), roundtrip(e));
    }
}

//! JavaScript code emitter for the Closure Compiler clone.
//!
//! Per [CLOC07](../../../specs/CLOC07-emit-and-source-map.md).
//! Step 3 of 4 in the autonomous-chain real-body rollout (after
//! constant-fold and fold-control-flow, before DCE).
//!
//! # What this version does (v0.2.0)
//!
//! - Walks every Phase 1 AST node recursively.
//! - Honors all three `EmitOptions`:
//!   - **`pretty`**: false → minified single-line output; true →
//!     2-space-indented multi-line output (block bodies and
//!     object/array contents).
//!   - **`ascii_only`**: true → escape non-ASCII codepoints in
//!     `StringLiteral` values as `\uXXXX` (or `\u{XXXXXX}` for
//!     codepoints above U+FFFF).
//!   - **`source_map`**: true → accumulate per-token mappings
//!     into a [`SourceMapBuilder`] and serialize the result into
//!     [`EmitOutput::source_map`]. false → omit the map entirely.
//! - Tracks `(line, column)` as it writes so each emitted token's
//!   position is known precisely. When the node has `cv: Some(id)`
//!   and `source_map` is enabled, the token's `(line, column,
//!   cv_id)` triple gets recorded.
//!
//! # Always-parenthesize policy in v1
//!
//! v1 always parenthesizes `BinaryExpression`, `LogicalExpression`,
//! `ConditionalExpression`, and `AssignmentExpression`. ECMAScript
//! precedence rules are a sizeable table; getting them right is
//! Phase 1.x work. The parens cost a few bytes but guarantee
//! correctness — `1 + 2 * 3` never round-trips as `(1 + 2) * 3`.
//!
//! Same reasoning for `ObjectExpression` at statement position:
//! `{a: 1}` at the start of a statement parses as a block, not
//! an object. We always wrap object expressions inside an
//! ExpressionStatement as `({a: 1})`. Other positions (e.g.
//! `const x = {a: 1};`) don't need the wrap, but it's idempotent
//! and we apply it uniformly for simplicity.
//!
//! # CV tracing modes (CLOC09 amendment)
//!
//! - **Traced input** (`cv: Some` on nodes): the emitter calls
//!   `SourceMapBuilder::add_mapping(line, col, cv_id)` per
//!   identifier-shaped or literal-shaped token. The resulting
//!   source map can be queried later to find which input byte
//!   produced any output byte.
//! - **Untraced input** (`cv: None` on nodes): no mappings are
//!   recorded. The `EmitOutput.source_map` still contains a
//!   valid (empty-mappings) v3 blob when `source_map = true`,
//!   so consumers don't need to special-case the untraced shape.
//!
//! # What v0.2.0 still skips
//!
//! - Real source-map VLQ encoding lives in
//!   `coding-adventures-closure-source-map`'s v2 — for now the
//!   `SourceMap` is shaped right but mappings field is empty.
//!   We still call `add_mapping`; the entries accumulate in the
//!   builder but the final string is empty until the encoder
//!   lands.
//! - Precedence-aware parens (drop redundant ones).
//! - JSDoc-block comment preservation.

use coding_adventures_closure_source_map::SourceMapBuilder;
use coding_adventures_correlation_vector::{CVLog, Contribution};
use coding_adventures_javascript_ast::{
    statement::TaggedStatement, ArrayExpression, AssignmentExpression, AssignmentOperator,
    AssignmentTarget, BigIntLiteral, BinaryExpression, BinaryOperator, BlockStatement,
    BooleanLiteral,
    BreakStatement, CallExpression, ConditionalExpression, ContinueStatement, Declaration,
    EmptyStatement, Expression, ExpressionStatement, ForInit, ForStatement, FunctionDeclaration,
    FunctionParam, Identifier, IfStatement, LabeledStatement, LogicalExpression, LogicalOperator,
    MemberExpression, NullLiteral, NumericLiteral, ObjectExpression, Program, ProgramItem,
    Property, PropertyKey, PropertyKind, ReturnStatement, Statement, StringLiteral,
    ThrowStatement, UnaryExpression, UnaryOperator, VarKind, VariableDeclaration,
    VariableDeclarator, WhileStatement,
};
use coding_adventures_type_sidecar::Sidecar;
use std::fmt;

// =====================================================================
// Public API — unchanged from v0.1.0
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    pub ascii_only: bool,
    pub pretty: bool,
    pub source_map: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            ascii_only: false,
            pretty: false,
            source_map: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmitOutput {
    pub code: String,
    pub source_map: Option<String>,
    pub contributions: Vec<Contribution>,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EmitError {
    UnknownCvId { id: String, site: &'static str },
    UnsupportedSidecarType { id: String, kind: String },
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::UnknownCvId { id, site } => {
                write!(f, "emit: AST referenced unknown CV id {:?} at {}", id, site)
            }
            EmitError::UnsupportedSidecarType { id, kind } => write!(
                f,
                "emit: don't know how to render sidecar type {:?} on id {:?}",
                kind, id
            ),
        }
    }
}

impl std::error::Error for EmitError {}

/// Emit JavaScript text + optional source map for `program`.
///
/// The v0.2.0 body walks every Phase 1 node and writes JS text.
/// `_sidecar` and the typing on `cv` (mutable so the future
/// source-map encoder can consult it) are kept in the signature
/// for forward-compat with the source-map v2 work.
pub fn emit(
    program: &Program,
    _sidecar: &Sidecar,
    cv: &mut CVLog,
    opts: &EmitOptions,
) -> Result<EmitOutput, EmitError> {
    let mut emitter = Emitter::new(opts);
    emitter.emit_program(program);

    let source_map = if opts.source_map {
        // Build the source map. Even when no mappings were
        // accumulated (untraced input), this produces a valid
        // v3 blob with empty mappings.
        Some(emitter.source_map.build(cv).to_json())
    } else {
        None
    };

    Ok(EmitOutput {
        code: emitter.out,
        source_map,
        contributions: Vec::new(),
    })
}

// =====================================================================
// Emitter — internal state for the recursive walk
// =====================================================================

struct Emitter<'a> {
    opts: &'a EmitOptions,
    out: String,
    /// Current 0-based line where the next character will be
    /// written. Bumped each time we emit `\n`.
    line: u32,
    /// Current 0-based column (in UTF-16 code units, per the
    /// source-map v3 spec). Reset to 0 on newline.
    col: u32,
    /// Current indent depth for pretty-printing (in 2-space
    /// units). Ignored entirely when `opts.pretty == false`.
    indent: u32,
    source_map: SourceMapBuilder,
}

impl<'a> Emitter<'a> {
    fn new(opts: &'a EmitOptions) -> Self {
        Self {
            opts,
            out: String::new(),
            line: 0,
            col: 0,
            indent: 0,
            source_map: SourceMapBuilder::new(),
        }
    }

    // ---- low-level write helpers ---------------------------------

    /// Write a string of characters that the caller guarantees
    /// contains no newlines. Bumps `col` by the UTF-16 length.
    fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            debug_assert!(ch != '\n', "use newline() for newlines");
            self.col += ch.len_utf16() as u32;
        }
        self.out.push_str(s);
    }

    /// Write a literal newline and reset the column. Bumps `line`.
    fn newline(&mut self) {
        self.out.push('\n');
        self.line += 1;
        self.col = 0;
    }

    /// Write the 2-space indent appropriate for the current
    /// nesting depth. Only emits anything when `opts.pretty`.
    fn indent_str(&mut self) {
        if !self.opts.pretty {
            return;
        }
        for _ in 0..self.indent {
            self.write_str("  ");
        }
    }

    /// Whitespace that's required between tokens (operator gaps,
    /// keyword↔identifier gaps). Always written.
    fn required_ws(&mut self) {
        self.write_str(" ");
    }

    /// Optional whitespace — written only when `pretty`.
    fn pretty_ws(&mut self) {
        if self.opts.pretty {
            self.write_str(" ");
        }
    }

    /// Record a mapping for an emitted token. No-op when the
    /// node's `cv` is `None` or when `source_map` is disabled.
    /// Position recorded is the column where the token *starts*.
    fn maybe_map(&mut self, cv: &Option<String>) {
        if !self.opts.source_map {
            return;
        }
        if let Some(id) = cv {
            self.source_map.add_mapping(self.line, self.col, id);
        }
    }

    // ---- Program & top-level -------------------------------------

    fn emit_program(&mut self, p: &Program) {
        for (i, item) in p.body.iter().enumerate() {
            if i > 0 && self.opts.pretty {
                self.newline();
            }
            self.emit_program_item(item);
        }
    }

    fn emit_program_item(&mut self, item: &ProgramItem) {
        match item {
            ProgramItem::Statement(s) => self.emit_statement(s),
            ProgramItem::Declaration(d) => {
                self.emit_declaration(d);
                // Declarations at top-level don't add their own
                // trailing semicolon — VariableDeclaration emits
                // one, FunctionDeclaration doesn't need one.
            }
        }
    }

    // ---- Statements ----------------------------------------------

    fn emit_statement(&mut self, s: &Statement) {
        match s {
            Statement::Tagged(t) => self.emit_tagged_statement(t),
            Statement::Declaration(d) => {
                self.emit_declaration(d);
            }
        }
    }

    fn emit_tagged_statement(&mut self, s: &TaggedStatement) {
        match s {
            TaggedStatement::ExpressionStatement(es) => {
                self.emit_expression_statement(es);
            }
            TaggedStatement::BlockStatement(b) => {
                self.emit_block_statement(b);
            }
            TaggedStatement::IfStatement(i) => self.emit_if(i),
            TaggedStatement::WhileStatement(w) => self.emit_while(w),
            TaggedStatement::ForStatement(f) => self.emit_for(f),
            TaggedStatement::ReturnStatement(r) => self.emit_return(r),
            TaggedStatement::BreakStatement(b) => self.emit_break(b),
            TaggedStatement::ContinueStatement(c) => self.emit_continue(c),
            TaggedStatement::LabeledStatement(l) => self.emit_labeled(l),
            TaggedStatement::ThrowStatement(t) => self.emit_throw(t),
            TaggedStatement::EmptyStatement(e) => self.emit_empty(e),
        }
    }

    fn emit_expression_statement(&mut self, es: &ExpressionStatement) {
        // Object expressions at the start of a statement parse as
        // blocks. The leading-token-disambiguation wrap (per CLOC12.10
        // / gap-024) covers that one case only. Everything else gets
        // precedence-aware emit at parent_prec = 0, which means no
        // wrapping unless an inner expression has a lower-precedence
        // child that requires it.
        let needs_paren = matches!(es.expression, Expression::ObjectExpression(_));
        self.maybe_map(&es.cv);
        if needs_paren {
            self.write_str("(");
        }
        // Statement position is the loosest binding context — every
        // expression's own precedence is >= 0, so the precedence
        // wrapper won't insert outer parens here. Inner precedence
        // requirements still propagate through child calls.
        self.emit_expression_inner(&es.expression, 0);
        if needs_paren {
            self.write_str(")");
        }
        self.write_str(";");
    }

    fn emit_block_statement(&mut self, b: &BlockStatement) {
        self.maybe_map(&b.cv);
        self.write_str("{");
        if b.body.is_empty() {
            self.write_str("}");
            return;
        }
        if self.opts.pretty {
            self.newline();
            self.indent += 1;
            for (i, s) in b.body.iter().enumerate() {
                if i > 0 {
                    self.newline();
                }
                self.indent_str();
                self.emit_statement(s);
            }
            self.indent -= 1;
            self.newline();
            self.indent_str();
        } else {
            for (i, s) in b.body.iter().enumerate() {
                if i > 0 {
                    self.pretty_ws();
                }
                self.emit_statement(s);
            }
        }
        self.write_str("}");
    }

    fn emit_if(&mut self, i: &IfStatement) {
        self.maybe_map(&i.cv);
        self.write_str("if");
        self.pretty_ws();
        self.write_str("(");
        self.emit_expression(&i.test);
        self.write_str(")");
        self.pretty_ws();
        self.emit_statement(&i.consequent);
        if let Some(alt) = &i.alternate {
            self.pretty_ws();
            self.write_str("else");
            self.required_ws();
            self.emit_statement(alt);
        }
    }

    fn emit_while(&mut self, w: &WhileStatement) {
        self.maybe_map(&w.cv);
        self.write_str("while");
        self.pretty_ws();
        self.write_str("(");
        self.emit_expression(&w.test);
        self.write_str(")");
        self.pretty_ws();
        self.emit_statement(&w.body);
    }

    fn emit_for(&mut self, f: &ForStatement) {
        self.maybe_map(&f.cv);
        self.write_str("for");
        self.pretty_ws();
        self.write_str("(");
        if let Some(init) = &f.init {
            match init {
                ForInit::VariableDeclaration(v) => {
                    self.emit_variable_declaration(v, /*top_level=*/ false);
                }
                ForInit::Expression(e) => self.emit_expression(e),
            }
        }
        self.write_str(";");
        if let Some(t) = &f.test {
            self.pretty_ws();
            self.emit_expression(t);
        }
        self.write_str(";");
        if let Some(u) = &f.update {
            self.pretty_ws();
            self.emit_expression(u);
        }
        self.write_str(")");
        self.pretty_ws();
        self.emit_statement(&f.body);
    }

    fn emit_return(&mut self, r: &ReturnStatement) {
        self.maybe_map(&r.cv);
        self.write_str("return");
        if let Some(arg) = &r.argument {
            self.required_ws();
            self.emit_expression(arg);
        }
        self.write_str(";");
    }

    fn emit_break(&mut self, b: &BreakStatement) {
        self.maybe_map(&b.cv);
        self.write_str("break");
        if let Some(label) = &b.label {
            self.required_ws();
            self.emit_identifier(label);
        }
        self.write_str(";");
    }

    fn emit_continue(&mut self, c: &ContinueStatement) {
        self.maybe_map(&c.cv);
        self.write_str("continue");
        if let Some(label) = &c.label {
            self.required_ws();
            self.emit_identifier(label);
        }
        self.write_str(";");
    }

    fn emit_empty(&mut self, e: &EmptyStatement) {
        self.maybe_map(&e.cv);
        self.write_str(";");
    }

    /// `label: stmt`. No trailing semicolon — the body statement
    /// supplies its own (every `emit_*_statement` write_str's `;` at
    /// the tail). For pretty mode we emit a single space after the
    /// colon to match upstream's `printer.cont(":");` + the body's
    /// own indentation; in compact mode there is no whitespace at all.
    ///
    /// Note: `label:` is not itself an expression so it doesn't enter
    /// the precedence ladder; statements live above expressions in the
    /// grammar.
    fn emit_labeled(&mut self, l: &LabeledStatement) {
        self.maybe_map(&l.cv);
        self.emit_identifier(&l.label);
        self.write_str(":");
        self.pretty_ws();
        self.emit_statement(&l.body);
    }

    /// `throw expr;` — keyword + REQUIRED whitespace + expression +
    /// `;`. The space is mandatory: without it `throw1` parses as an
    /// identifier in V8's relaxed mode and is ambiguous in others.
    /// Per ECMAScript §13.14, `throw` has no no-argument form, so we
    /// always emit the argument.
    fn emit_throw(&mut self, t: &ThrowStatement) {
        self.maybe_map(&t.cv);
        self.write_str("throw");
        self.required_ws();
        self.emit_expression(&t.argument);
        self.write_str(";");
    }

    // ---- Declarations --------------------------------------------

    fn emit_declaration(&mut self, d: &Declaration) {
        match d {
            Declaration::VariableDeclaration(v) => {
                self.emit_variable_declaration(v, /*top_level=*/ true);
            }
            Declaration::FunctionDeclaration(f) => self.emit_function_declaration(f),
        }
    }

    fn emit_variable_declaration(&mut self, v: &VariableDeclaration, with_semi: bool) {
        self.maybe_map(&v.cv);
        self.write_str(match v.kind {
            VarKind::Var => "var",
            VarKind::Let => "let",
            VarKind::Const => "const",
        });
        for (i, d) in v.declarations.iter().enumerate() {
            if i == 0 {
                self.required_ws();
            } else {
                self.write_str(",");
                self.pretty_ws();
            }
            self.emit_variable_declarator(d);
        }
        if with_semi {
            self.write_str(";");
        }
    }

    fn emit_variable_declarator(&mut self, d: &VariableDeclarator) {
        self.maybe_map(&d.cv);
        match &d.id {
            coding_adventures_javascript_ast::BindingTarget::Identifier(i) => {
                self.emit_identifier(i)
            }
        }
        if let Some(init) = &d.init {
            self.pretty_ws();
            self.write_str("=");
            self.pretty_ws();
            self.emit_expression(init);
        }
    }

    fn emit_function_declaration(&mut self, f: &FunctionDeclaration) {
        self.maybe_map(&f.cv);
        if f.is_async {
            self.write_str("async");
            self.required_ws();
        }
        self.write_str("function");
        if f.generator {
            self.write_str("*");
        }
        self.required_ws();
        self.emit_identifier(&f.id);
        self.write_str("(");
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            match p {
                FunctionParam::Identifier(id) => self.emit_identifier(id),
            }
        }
        self.write_str(")");
        self.pretty_ws();
        self.emit_block_statement(&f.body);
    }

    // ---- Expressions ---------------------------------------------

    /// Public entry: emit an expression assuming the loosest binding
    /// context (parent_prec = 0). Used by statement-position callers
    /// and control-position contexts (if-test, while-test, etc.)
    /// where the JS grammar surrounds the expression with its own
    /// punctuation and no parens are required.
    fn emit_expression(&mut self, e: &Expression) {
        self.emit_expression_inner(e, 0);
    }

    /// Precedence-aware expression emit. Wraps in parens when the
    /// child expression's own precedence is strictly less than the
    /// parent context's binding strength.
    ///
    /// Truth table for the wrap decision:
    ///
    /// | child  | parent | wrap? | why                                  |
    /// |--------|--------|-------|--------------------------------------|
    /// |   18   |   0    |  no   | top-level: anything binds tighter    |
    /// |   11   |   12   |  yes  | `a + b` inside `* c` needs parens    |
    /// |   12   |   11   |  no   | `a * b` inside `+ c` doesn't         |
    /// |   11   |   11   |  no   | left-assoc tie on the LEFT child     |
    /// |   11   |   12   |  yes  | (same as above, just emphasised)     |
    /// |   14   |   12   |  no   | unary `-x` inside `* c` no parens    |
    ///
    /// `expr_prec` resolves the child's own precedence. For primary
    /// expressions (identifiers, literals, call, member, array,
    /// object) the precedence is high enough that they never need
    /// wrapping. For binary / logical / conditional / assignment
    /// the precedence depends on the operator.
    fn emit_expression_inner(&mut self, e: &Expression, parent_prec: u8) {
        let my_prec = expr_prec(e);
        let needs_parens = my_prec < parent_prec;
        if needs_parens {
            self.write_str("(");
        }
        match e {
            Expression::Identifier(i) => self.emit_identifier(i),
            Expression::NumericLiteral(n) => self.emit_numeric(n),
            Expression::StringLiteral(s) => self.emit_string(s),
            Expression::BooleanLiteral(b) => self.emit_boolean(b),
            Expression::NullLiteral(n) => self.emit_null(n),
            Expression::BigIntLiteral(b) => self.emit_bigint(b),
            Expression::BinaryExpression(b) => self.emit_binary(b),
            Expression::LogicalExpression(l) => self.emit_logical(l),
            Expression::UnaryExpression(u) => self.emit_unary(u),
            Expression::AssignmentExpression(a) => self.emit_assignment(a),
            Expression::ConditionalExpression(c) => self.emit_conditional(c),
            Expression::CallExpression(c) => self.emit_call(c),
            Expression::MemberExpression(m) => self.emit_member(m),
            Expression::ArrayExpression(a) => self.emit_array(a),
            Expression::ObjectExpression(o) => self.emit_object(o),
        }
        if needs_parens {
            self.write_str(")");
        }
    }

    fn emit_identifier(&mut self, i: &Identifier) {
        self.maybe_map(&i.cv);
        self.write_str(&i.name);
    }

    fn emit_numeric(&mut self, n: &NumericLiteral) {
        self.maybe_map(&n.cv);
        self.write_str(&format_js_number(n.value));
    }

    fn emit_string(&mut self, s: &StringLiteral) {
        // CLOC12.11 / gap-026: quote-choice optimisation.
        //
        // Upstream's CodePrinter picks the quote style that minimises
        // required escape sequences. We do the same: count the number
        // of double-quote chars in the value; if it strictly exceeds
        // the single-quote count, the single-quote form is shorter.
        //
        // Truth table:
        //
        //   value content      dq count   sq count   choice
        //   -----------------  --------   --------   ---------
        //   hello                 0          0       double  (tie → "")
        //   o'malley              0          1       double
        //   she said "hi"         1          0       single
        //   "mixed 'both'"        2          2       double  (tie → "")
        //   one "two" three       1          0       single
        //
        // `ascii_only` always emits with double quotes (per upstream's
        // explicit ASCII escape rules — switching mid-mode would
        // confuse downstream readers).
        self.maybe_map(&s.cv);
        if self.opts.ascii_only {
            self.write_str(&format!("\"{}\"", escape_ascii_only(&s.value)));
        } else {
            let (quote_ch, escaped) = choose_quote_and_escape(&s.value);
            self.write_str(&format!("{quote_ch}{escaped}{quote_ch}"));
        }
    }

    fn emit_boolean(&mut self, b: &BooleanLiteral) {
        self.maybe_map(&b.cv);
        self.write_str(if b.value { "true" } else { "false" });
    }

    fn emit_null(&mut self, n: &NullLiteral) {
        self.maybe_map(&n.cv);
        self.write_str("null");
    }

    /// BigInt literals print their `raw` source representation
    /// verbatim. We keep `raw` (e.g. `"0x1fn"`, `"123n"`) instead of
    /// reformatting from `value` because hex/octal/binary radixes are
    /// part of the literal's source identity, and shorter-form
    /// rewriting (e.g. `1000000000n` → `1e9n`) isn't valid for bigints
    /// — there's no exponential bigint syntax. So no normalisation.
    fn emit_bigint(&mut self, b: &BigIntLiteral) {
        self.maybe_map(&b.cv);
        self.write_str(&b.raw);
    }

    fn emit_binary(&mut self, b: &BinaryExpression) {
        // Precedence-aware emit: left at my_prec (since `a + b + c`
        // groups left-to-right, the left child can be the same
        // precedence without parens), right at my_prec + 1 (the
        // right child must be strictly higher precedence to avoid
        // parens, because the operator is left-associative). The
        // outer wrap (if any) is the caller's responsibility — see
        // `emit_expression_inner`.
        self.maybe_map(&b.cv);
        let my_prec = binary_prec(b.operator);
        self.emit_expression_inner(&b.left, my_prec);
        // Binary operators always get spaces in our output —
        // makes `1 in obj` and `a instanceof b` unambiguous even
        // in minified mode.
        self.required_ws();
        self.write_str(binary_op_str(b.operator));
        self.required_ws();
        self.emit_expression_inner(&b.right, my_prec + 1);
    }

    fn emit_logical(&mut self, l: &LogicalExpression) {
        self.maybe_map(&l.cv);
        let my_prec = logical_prec(l.operator);
        self.emit_expression_inner(&l.left, my_prec);
        self.required_ws();
        self.write_str(logical_op_str(l.operator));
        self.required_ws();
        self.emit_expression_inner(&l.right, my_prec + 1);
    }

    fn emit_unary(&mut self, u: &UnaryExpression) {
        self.maybe_map(&u.cv);
        let s = unary_op_str(u.operator);
        self.write_str(s);
        // Word-shaped ops (typeof / void / delete) need a space
        // before their argument; symbol-shaped ops don't.
        if matches!(
            u.operator,
            UnaryOperator::TypeOf | UnaryOperator::Void | UnaryOperator::Delete
        ) {
            self.required_ws();
        }
        self.emit_expression(&u.argument);
    }

    fn emit_assignment(&mut self, a: &AssignmentExpression) {
        self.maybe_map(&a.cv);
        match &a.left {
            AssignmentTarget::Identifier(i) => self.emit_identifier(i),
            AssignmentTarget::MemberExpression(m) => self.emit_member(m),
        }
        self.pretty_ws();
        self.write_str(assignment_op_str(a.operator));
        self.pretty_ws();
        self.emit_expression(&a.right);
    }

    fn emit_conditional(&mut self, c: &ConditionalExpression) {
        // Conditional is right-associative with precedence PREC_CONDITIONAL.
        // - test:        must bind tighter than conditional itself
        // - consequent:  assignment-precedence in ESTree, here we use
        //                conditional-precedence (close enough for
        //                Phase 1 without SequenceExpression)
        // - alternate:   right-associative, so accepts conditional
        //                precedence on the right
        // Outer wrap (if any) is the caller's responsibility — see
        // `emit_expression_inner`.
        self.maybe_map(&c.cv);
        self.emit_expression_inner(&c.test, PREC_CONDITIONAL + 1);
        self.pretty_ws();
        self.write_str("?");
        self.pretty_ws();
        self.emit_expression_inner(&c.consequent, PREC_CONDITIONAL + 1);
        self.pretty_ws();
        self.write_str(":");
        self.pretty_ws();
        self.emit_expression_inner(&c.alternate, PREC_CONDITIONAL);
    }

    fn emit_call(&mut self, c: &CallExpression) {
        self.maybe_map(&c.cv);
        self.emit_expression(&c.callee);
        self.write_str("(");
        for (i, a) in c.arguments.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            self.emit_expression(a);
        }
        self.write_str(")");
    }

    fn emit_member(&mut self, m: &MemberExpression) {
        self.maybe_map(&m.cv);
        self.emit_expression(&m.object);
        if m.computed {
            self.write_str("[");
            self.emit_expression(&m.property);
            self.write_str("]");
        } else {
            self.write_str(".");
            self.emit_expression(&m.property);
        }
    }

    fn emit_array(&mut self, a: &ArrayExpression) {
        self.maybe_map(&a.cv);
        self.write_str("[");
        for (i, el) in a.elements.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            match el {
                Some(e) => self.emit_expression(e),
                None => {
                    // Elision. Empty position between commas.
                }
            }
        }
        self.write_str("]");
    }

    fn emit_object(&mut self, o: &ObjectExpression) {
        self.maybe_map(&o.cv);
        self.write_str("{");
        for (i, p) in o.properties.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            } else {
                self.pretty_ws();
            }
            self.emit_property(p);
        }
        if !o.properties.is_empty() {
            self.pretty_ws();
        }
        self.write_str("}");
    }

    fn emit_property(&mut self, p: &Property) {
        self.maybe_map(&p.cv);
        // get/set methods.
        match p.kind {
            PropertyKind::Init => {
                if p.shorthand {
                    self.emit_property_key(&p.key);
                    return;
                }
                if p.method {
                    self.emit_property_key(&p.key);
                    // Method value is a FunctionExpression (Phase
                    // 2). For Phase 1 we conservatively emit the
                    // value expression which may be a function
                    // literal or any expression — Phase 1 doesn't
                    // emit FunctionExpression yet, so any non-
                    // function value just gets emitted as-is.
                    self.emit_expression(&p.value);
                    return;
                }
                self.emit_property_key(&p.key);
                self.write_str(":");
                self.pretty_ws();
                self.emit_expression(&p.value);
            }
            PropertyKind::Get => {
                self.write_str("get");
                self.required_ws();
                self.emit_property_key(&p.key);
                self.emit_expression(&p.value);
            }
            PropertyKind::Set => {
                self.write_str("set");
                self.required_ws();
                self.emit_property_key(&p.key);
                self.emit_expression(&p.value);
            }
        }
    }

    fn emit_property_key(&mut self, k: &PropertyKey) {
        match k {
            PropertyKey::Identifier(i) => self.emit_identifier(i),
            PropertyKey::StringLiteral(s) => self.emit_string(s),
            PropertyKey::NumericLiteral(n) => self.emit_numeric(n),
            PropertyKey::Expression(e) => {
                self.write_str("[");
                self.emit_expression(e);
                self.write_str("]");
            }
        }
    }
}

// =====================================================================
// Helpers — operator strings, number formatting, string escaping
// =====================================================================

/// JavaScript-style number rendering — matches `String(x)` so
/// emitted output round-trips numerically. CLOC12.12 / gap-025:
/// for finite non-zero numbers we now compute BOTH the decimal and
/// exponential forms and return whichever is shorter. Ties pick
/// decimal (canonical).
///
/// Examples:
///
///   1                 →  "1"      (decimal shorter)
///   100               →  "100"    (tie → decimal)
///   1000000000        →  "1E9"    (decimal 10 chars vs expo 3)
///   0.5               →  "0.5"    (decimal shorter)
///   1.5e-10           →  "1.5E-10" (decimal 13 chars vs expo 7)
///   1e21              →  "1E21"   (expo shorter)
///   NaN / Infinity    →  unchanged from JS String(x)
fn format_js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if n == 0.0 {
        return "0".to_string();
    }
    let decimal = if n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    };
    let expo = format_exponential_uppercase(n);
    if expo.len() < decimal.len() {
        expo
    } else {
        decimal
    }
}

/// Build the JS-style exponential form for a finite non-zero
/// number. Rust's `{:e}` formatter produces `"1e9"` / `"1.5e-10"` /
/// `"1.2345e4"`; we just uppercase the `e`. We do *not* emit a `+`
/// for positive exponents — upstream's CodePrinter writes `1E9`,
/// not `1E+9`.
fn format_exponential_uppercase(n: f64) -> String {
    // Rust's `{:e}` never inserts a `+` for positive exponents and
    // strips trailing zeros from the mantissa fraction. So a direct
    // `e → E` substitution is sufficient.
    let mut s = format!("{:e}", n);
    // Convert "1.5e-10" → "1.5E-10"
    if let Some(pos) = s.find('e') {
        s.replace_range(pos..pos + 1, "E");
    }
    s
}

// =====================================================================
// Operator precedence (per CLOC12.10 / gap-024 + gap-027)
//
// The emitter inserts parens around expressions exactly when the
// child's own binding strength is *strictly* lower than the parent
// context's. The numeric values themselves don't matter — only their
// relative ordering. They follow the ECMAScript §13 expression
// grammar ladder, low → high.
//
// 0   — top level (statement position, control-test position, etc.)
//        Anything can sit here without parens.
// 1   — assignment (`=`, `+=`, …)
// 2   — conditional `? :`
// 3   — logical OR `||`, nullish coalescing `??`
// 4   — logical AND `&&`
// 5–7 — bitwise OR / XOR / AND
// 8   — equality (`==`, `!=`, `===`, `!==`)
// 9   — relational (`<`, `<=`, `>`, `>=`, `in`, `instanceof`)
// 10  — shift (`<<`, `>>`, `>>>`)
// 11  — additive (`+`, `-`)
// 12  — multiplicative (`*`, `/`, `%`)
// 13  — exponent (`**`)   right-associative
// 14  — prefix unary (`!`, `-`, `+`, `~`, `typeof`, `void`, `delete`)
// 17  — call / member / new (left-associative; never needs wrapping
//                            as a child)
// 18  — primary (literals, identifiers, parens) — atomic
//
// `binary_prec`, `logical_prec`, `expr_prec` resolve the precedence
// of a given AST node.
// =====================================================================

const PREC_CONDITIONAL: u8 = 2;
const PREC_UNARY: u8 = 14;
const PREC_PRIMARY: u8 = 18;
const PREC_ASSIGNMENT: u8 = 1;

fn binary_prec(op: BinaryOperator) -> u8 {
    use BinaryOperator::*;
    match op {
        BitOr => 5,
        BitXor => 6,
        BitAnd => 7,
        Eq | NotEq | StrictEq | StrictNotEq => 8,
        Lt | LtEq | Gt | GtEq | In | InstanceOf => 9,
        LeftShift | RightShift | UnsignedRightShift => 10,
        Add | Sub => 11,
        Mul | Div | Mod => 12,
        Exp => 13,
    }
}

fn logical_prec(op: LogicalOperator) -> u8 {
    use LogicalOperator::*;
    match op {
        Or | NullishCoalescing => 3,
        And => 4,
    }
}

/// Resolve the own precedence of any `Expression`. Used by
/// `emit_expression_inner` to decide whether to wrap a child in
/// parens given the parent's context precedence.
fn expr_prec(e: &Expression) -> u8 {
    match e {
        // Atomic / left-associative primaries — never need wrapping
        // from any parent (their precedence is higher than every
        // operator). Member/call left-associativity means they bind
        // tighter than unary as a unit; tagging them at PREC_PRIMARY
        // keeps emit_expression_inner from inserting unnecessary
        // parens in `f(x).y[z]` chains.
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::ArrayExpression(_)
        | Expression::ObjectExpression(_)
        | Expression::CallExpression(_)
        | Expression::MemberExpression(_) => PREC_PRIMARY,

        Expression::UnaryExpression(_) => PREC_UNARY,
        Expression::BinaryExpression(b) => binary_prec(b.operator),
        Expression::LogicalExpression(l) => logical_prec(l.operator),
        Expression::ConditionalExpression(_) => PREC_CONDITIONAL,
        Expression::AssignmentExpression(_) => PREC_ASSIGNMENT,
    }
}

fn binary_op_str(op: BinaryOperator) -> &'static str {
    use BinaryOperator::*;
    match op {
        Eq => "==",
        NotEq => "!=",
        StrictEq => "===",
        StrictNotEq => "!==",
        Lt => "<",
        LtEq => "<=",
        Gt => ">",
        GtEq => ">=",
        LeftShift => "<<",
        RightShift => ">>",
        UnsignedRightShift => ">>>",
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Mod => "%",
        Exp => "**",
        BitOr => "|",
        BitXor => "^",
        BitAnd => "&",
        In => "in",
        InstanceOf => "instanceof",
    }
}

fn logical_op_str(op: LogicalOperator) -> &'static str {
    match op {
        LogicalOperator::And => "&&",
        LogicalOperator::Or => "||",
        LogicalOperator::NullishCoalescing => "??",
    }
}

fn unary_op_str(op: UnaryOperator) -> &'static str {
    match op {
        UnaryOperator::Negate => "-",
        UnaryOperator::Plus => "+",
        UnaryOperator::Not => "!",
        UnaryOperator::BitNot => "~",
        UnaryOperator::TypeOf => "typeof",
        UnaryOperator::Void => "void",
        UnaryOperator::Delete => "delete",
    }
}

fn assignment_op_str(op: AssignmentOperator) -> &'static str {
    use AssignmentOperator::*;
    match op {
        Eq => "=",
        AddEq => "+=",
        SubEq => "-=",
        MulEq => "*=",
        DivEq => "/=",
        ModEq => "%=",
        ExpEq => "**=",
        LeftShiftEq => "<<=",
        RightShiftEq => ">>=",
        UnsignedRightShiftEq => ">>>=",
        BitOrEq => "|=",
        BitXorEq => "^=",
        BitAndEq => "&=",
    }
}

/// Pick the quote style (`'` or `"`) that yields the shorter
/// escaped string for `value`, then escape against that style.
/// Returns `(quote_char_as_str, escaped_body)`.
///
/// Algorithm: count occurrences of `'` and `"`. If `"` appears more
/// often, single-quote is shorter (because we'd escape fewer chars).
/// Ties keep double — that matches upstream `CodePrinter` and the
/// existing test expectations (`"foo"` is the canonical form).
///
/// Closes CLOC12 gap-026.
fn choose_quote_and_escape(value: &str) -> (&'static str, String) {
    let dq = value.chars().filter(|c| *c == '"').count();
    let sq = value.chars().filter(|c| *c == '\'').count();
    if dq > sq {
        ("'", escape_str_sq(value))
    } else {
        ("\"", escape_str_dq(value))
    }
}

/// Like [`escape_str_dq`] but for a single-quoted string — escape
/// `'` instead of `"`. Backslash and control char rules are
/// identical because they're independent of which quote wraps the
/// string.
fn escape_str_sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Escape `"` and `\` and control characters for inclusion in a
/// double-quoted JS string. Used when StringLiteral.raw is empty
/// (synthetic node).
fn escape_str_dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Like [`escape_str_dq`] but ALSO escapes every non-ASCII
/// codepoint to `\uXXXX` (or `\u{XXXXXX}` for chars above U+FFFF).
/// Used when `EmitOptions::ascii_only`.
fn escape_ascii_only(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c if c.is_ascii() => out.push(c),
            c if (c as u32) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push_str(&format!("\\u{{{:X}}}", c as u32)),
        }
    }
    out
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_tokens::EsVersion;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }
    fn untraced_program() -> Program {
        Program::new_untraced(EsVersion::Es2025, SourceType::Module)
    }

    fn num(v: f64) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: v,
            raw: v.to_string(),
        })
    }
    fn string(v: &str) -> Expression {
        Expression::StringLiteral(StringLiteral {
            cv: None,
            value: v.to_string(),
            raw: format!("\"{}\"", v),
        })
    }
    fn ident(name: &str) -> Expression {
        Expression::Identifier(Identifier {
            cv: None,
            name: name.to_string(),
        })
    }
    fn boolean(v: bool) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
    }

    fn emit_default(prog: Program) -> EmitOutput {
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
            .expect("emit should succeed")
    }

    fn emit_with(prog: Program, opts: EmitOptions) -> EmitOutput {
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        emit(&prog, &sidecar, &mut cv, &opts).expect("emit should succeed")
    }

    fn stmt(expr: Expression) -> ProgramItem {
        ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: expr,
        }))
    }

    // ---- defaults + empty -----------------------------------

    #[test]
    fn empty_program_emits_empty_string() {
        let out = emit_default(program());
        assert_eq!(out.code, "");
    }

    #[test]
    fn default_options_unchanged() {
        let o = EmitOptions::default();
        assert!(!o.ascii_only);
        assert!(!o.pretty);
        assert!(o.source_map);
    }

    // ---- basic expressions ----------------------------------

    #[test]
    fn binary_addition_emits_without_outer_parens() {
        // gap-024 closed in CLOC12.10: only ObjectExpression at
        // statement position needs the leading-token disambiguation
        // wrap. Plain binary expressions emit directly.
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0)),
            right: Box::new(num(3.0)),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "2 + 3;");
    }

    // ---- quote-choice (gap-026, CLOC12.11) -----------------

    /// Helper: build a synthetic StringLiteral (no `raw`, so the
    /// quote-choice path runs) and emit it as a single statement.
    fn emit_string_value(value: &str) -> String {
        let s = Expression::StringLiteral(StringLiteral {
            cv: None,
            value: value.to_string(),
            raw: String::new(),
        });
        emit_default(program().with_body(vec![stmt(s)])).code
    }

    // ---- number shortest-form (gap-025, CLOC12.12) ---------

    /// Helper: emit a synthetic NumericLiteral and return the code
    /// (without the trailing `;`).
    fn emit_number_value(v: f64) -> String {
        let n = Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: v,
            raw: String::new(),
        });
        let code = emit_default(program().with_body(vec![stmt(n)])).code;
        // strip trailing ";"
        code.trim_end_matches(';').to_string()
    }

    #[test]
    fn number_shortest_form_small_integers_stay_decimal() {
        assert_eq!(emit_number_value(0.0), "0");
        assert_eq!(emit_number_value(1.0), "1");
        assert_eq!(emit_number_value(42.0), "42");
        assert_eq!(emit_number_value(100.0), "100"); // tie 3=3 → decimal
        assert_eq!(emit_number_value(-7.0), "-7");
    }

    #[test]
    fn number_shortest_form_big_integers_switch_to_exponential() {
        // 1000000000 (10 chars) vs 1E9 (3 chars) → 1E9
        assert_eq!(emit_number_value(1_000_000_000.0), "1E9");
        // 5_000_000 vs 5E6 → 5E6
        assert_eq!(emit_number_value(5_000_000.0), "5E6");
    }

    #[test]
    fn number_shortest_form_small_decimals_stay_decimal() {
        assert_eq!(emit_number_value(0.5), "0.5");
        assert_eq!(emit_number_value(3.14), "3.14");
    }

    #[test]
    fn number_shortest_form_tiny_floats_switch_to_exponential() {
        // 1.5e-10 → "0.00000000015" (13) vs "1.5E-10" (7)
        assert_eq!(emit_number_value(1.5e-10), "1.5E-10");
    }

    #[test]
    fn number_shortest_form_nan_and_infinity_unchanged() {
        assert_eq!(emit_number_value(f64::NAN), "NaN");
        assert_eq!(emit_number_value(f64::INFINITY), "Infinity");
        assert_eq!(emit_number_value(f64::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn quote_choice_no_quotes_uses_double() {
        // No quotes either way — canonical form is double.
        assert_eq!(emit_string_value("hello"), "\"hello\";");
        assert_eq!(emit_string_value(""), "\"\";");
    }

    #[test]
    fn quote_choice_single_quotes_in_value_uses_double() {
        // Value contains `'`, no `"`. Double-quoted form needs no
        // escapes; single-quoted would.
        assert_eq!(emit_string_value("o'malley"), "\"o'malley\";");
        assert_eq!(emit_string_value("it's"), "\"it's\";");
    }

    #[test]
    fn quote_choice_double_quotes_in_value_switches_to_single() {
        // Value contains `"`. Single-quoted form avoids the escape.
        assert_eq!(emit_string_value("she said \"hi\""), "'she said \"hi\"';");
    }

    #[test]
    fn quote_choice_tie_picks_double() {
        // Value `'"` — exactly one of each. Tie breaks toward
        // double. We assert the leading byte only so the test
        // doesn't depend on the escape rendering of the single
        // quote inside (which is left untouched in a double-quoted
        // string).
        let out = emit_string_value("'\"");
        assert!(
            out.starts_with('"'),
            "tie should pick double-quote; got {out}"
        );
    }

    #[test]
    fn quote_choice_more_double_than_single_picks_single() {
        // Value `""x` — 2 doubles, 0 singles. Single-quoted wins
        // by 2 escapes saved.
        let out = emit_string_value("\"\"x");
        assert!(
            out.starts_with('\''),
            "majority-double value should pick single-quote; got {out}"
        );
    }

    #[test]
    fn string_concat_emits_without_outer_parens() {
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(string("foo")),
            right: Box::new(string("bar")),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "\"foo\" + \"bar\";");
    }

    #[test]
    fn unary_not_no_space() {
        let e = Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(boolean(true)),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "!true;");
    }

    #[test]
    fn typeof_has_required_space() {
        let e = Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::TypeOf,
            prefix: true,
            argument: Box::new(string("x")),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "typeof \"x\";");
    }

    // ---- variable + function declarations -------------------

    #[test]
    fn const_declaration_with_init() {
        let v = VariableDeclaration {
            cv: None,
            kind: VarKind::Const,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: coding_adventures_javascript_ast::BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: "x".to_string(),
                }),
                init: Some(num(42.0)),
            }],
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::VariableDeclaration(v),
        )]);
        let out = emit_default(prog);
        // Minified: no spaces around `=`. `const` and the identifier
        // are separated by required whitespace.
        assert_eq!(out.code, "const x=42;");
    }

    #[test]
    fn function_declaration_minified() {
        // function f(x) { return x; }
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: Some(ident("x")),
            })],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![FunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_default(prog);
        // No spaces inside `{` `}` or around params in minified
        // mode, but `function` and `return` keywords are followed
        // by required whitespace.
        assert_eq!(out.code, "function f(x){return x;}");
    }

    #[test]
    fn function_declaration_pretty_wraps_body() {
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: Some(ident("x")),
            })],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![FunctionParam::Identifier(Identifier {
                cv: None,
                name: "x".to_string(),
            })],
            body,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_with(
            prog,
            EmitOptions {
                pretty: true,
                ..Default::default()
            },
        );
        assert_eq!(out.code, "function f(x) {\n  return x;\n}");
    }

    // ---- arrays + objects -----------------------------------

    #[test]
    fn array_with_elision() {
        // [1, , 3]
        let a = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(num(1.0)), None, Some(num(3.0))],
        });
        let prog = program().with_body(vec![stmt(a)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "[1,,3];");
    }

    #[test]
    fn object_expression_at_statement_start_is_parenthesized() {
        // {a: 1, b: 2} as a top-level expression statement.
        let o = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::Identifier(Identifier {
                        cv: None,
                        name: "a".to_string(),
                    }),
                    value: Box::new(num(1.0)),
                    computed: false,
                    shorthand: false,
                    method: false,
                },
                Property {
                    cv: None,
                    kind: PropertyKind::Init,
                    key: PropertyKey::Identifier(Identifier {
                        cv: None,
                        name: "b".to_string(),
                    }),
                    value: Box::new(num(2.0)),
                    computed: false,
                    shorthand: false,
                    method: false,
                },
            ],
        });
        let prog = program().with_body(vec![stmt(o)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "({a:1,b:2});");
    }

    // ---- ascii_only -----------------------------------------

    #[test]
    fn ascii_only_escapes_unicode() {
        // String containing "café" — the é needs escaping.
        let s = Expression::StringLiteral(StringLiteral {
            cv: None,
            value: "café".to_string(),
            raw: "\"café\"".to_string(),
        });
        let prog = program().with_body(vec![stmt(s)]);
        let out = emit_with(
            prog,
            EmitOptions {
                ascii_only: true,
                ..Default::default()
            },
        );
        // é = U+00E9
        assert_eq!(out.code, "\"caf\\u00E9\";");
        // And the output is now pure-ASCII.
        assert!(out.code.is_ascii());
    }

    // ---- source map -----------------------------------------

    #[test]
    fn source_map_true_produces_some() {
        let prog = program();
        let out = emit_default(prog);
        assert!(out.source_map.is_some(), "default has source_map=true");
        let j: serde_json::Value =
            serde_json::from_str(out.source_map.as_ref().unwrap()).expect("valid JSON");
        assert_eq!(j["version"], 3);
    }

    #[test]
    fn source_map_false_omits_field() {
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        let out = emit_with(program(), opts);
        assert!(out.source_map.is_none());
    }

    #[test]
    fn untraced_program_still_emits() {
        // cv: None everywhere. Should produce identical output text
        // (no source map mappings, but the JSON shell is still there).
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0)),
            right: Box::new(num(3.0)),
        });
        let prog = untraced_program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "2 + 3;");
    }

    // ---- END-TO-END: pipeline produces real output ----------

    #[test]
    fn end_to_end_two_plus_three_pipelines_then_emits_five() {
        // This is the first test of the pipeline producing real
        // JavaScript output:
        //
        //   AST(2 + 3)  →  ConstantFoldPass  →  AST(5)  →  emit  →  "5;"
        //
        // Verifies the full chain works end-to-end with real,
        // observable output text.
        use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
        use coding_adventures_closure_pass_pipeline::PassPipeline;

        let two_plus_three = Expression::BinaryExpression(BinaryExpression {
            cv: Some("bin.1".to_string()),
            operator: BinaryOperator::Add,
            left: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: Some("n.l".to_string()),
                value: 2.0,
                raw: "2".to_string(),
            })),
            right: Box::new(Expression::NumericLiteral(NumericLiteral {
                cv: Some("n.r".to_string()),
                value: 3.0,
                raw: "3".to_string(),
            })),
        });
        let input = program().with_body(vec![ProgramItem::Statement(
            Statement::expression_statement(ExpressionStatement {
                cv: Some("es.1".to_string()),
                expression: two_plus_three,
            }),
        )]);

        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let pipeline_out = pipeline
            .run(input, &sidecar, &mut cv)
            .expect("pipeline should run cleanly");

        let emit_out = emit(
            &pipeline_out.program,
            &sidecar,
            &mut cv,
            &EmitOptions::default(),
        )
        .expect("emit should succeed");

        assert_eq!(
            emit_out.code, "5;",
            "2+3 should fold to 5 then emit as `5;`; got {:?}",
            emit_out.code
        );
    }

    // ---- Labeled + break (gap-009, CLOC12.13) ----------------

    /// Helper: emit a single labeled statement and return the code.
    fn emit_stmt(s: Statement) -> String {
        emit_default(program().with_body(vec![ProgramItem::Statement(s)])).code
    }

    #[test]
    fn labeled_call_statement_emits_label_colon_call() {
        // a: foo();
        let body = Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(ident("foo")),
                arguments: vec![],
            }),
        });
        let s = Statement::labeled_statement(LabeledStatement {
            cv: None,
            label: Identifier { cv: None, name: "a".to_string() },
            body: Box::new(body),
        });
        assert_eq!(emit_stmt(s), "a:foo();");
    }

    #[test]
    fn bare_break_statement_emits_break_semicolon() {
        let s = Statement::break_statement(BreakStatement { cv: None, label: None });
        assert_eq!(emit_stmt(s), "break;");
    }

    #[test]
    fn labeled_break_statement_emits_break_label_semicolon() {
        let s = Statement::break_statement(BreakStatement {
            cv: None,
            label: Some(Identifier { cv: None, name: "a".to_string() }),
        });
        assert_eq!(emit_stmt(s), "break a;");
    }

    #[test]
    fn label_wrapping_break_self_emits_label_colon_break_label() {
        // a: break a;
        // The exact upstream `testRemoveNoOpLabelledStatement` input.
        // DCE will not (yet) collapse this; the emitter just needs to
        // print it as-is.
        let inner = Statement::break_statement(BreakStatement {
            cv: None,
            label: Some(Identifier { cv: None, name: "a".to_string() }),
        });
        let s = Statement::labeled_statement(LabeledStatement {
            cv: None,
            label: Identifier { cv: None, name: "a".to_string() },
            body: Box::new(inner),
        });
        assert_eq!(emit_stmt(s), "a:break a;");
    }

    // ---- Throw (gap-020, CLOC12.14) -------------------------

    #[test]
    fn throw_numeric_literal_emits_throw_one_semicolon() {
        // throw 1;
        let s = Statement::throw_statement(ThrowStatement {
            cv: None,
            argument: num(1.0),
        });
        assert_eq!(emit_stmt(s), "throw 1;");
    }

    #[test]
    fn throw_identifier_emits_throw_e_semicolon() {
        // throw e;
        let s = Statement::throw_statement(ThrowStatement {
            cv: None,
            argument: ident("e"),
        });
        assert_eq!(emit_stmt(s), "throw e;");
    }

    #[test]
    fn throw_string_literal_emits_throw_quoted_semicolon() {
        // throw "oops";
        let s = Statement::throw_statement(ThrowStatement {
            cv: None,
            argument: string("oops"),
        });
        // quote-choice picks double quotes by default for plain strings
        assert_eq!(emit_stmt(s), "throw \"oops\";");
    }

    // ---- BigIntLiteral (gap-021, CLOC12.15) -----------------

    fn emit_expr(e: Expression) -> String {
        emit_default(program().with_body(vec![stmt(e)])).code
    }

    #[test]
    fn bigint_literal_decimal_emits_raw() {
        // 123n  → "123n;"
        let e = Expression::BigIntLiteral(BigIntLiteral {
            cv: None,
            value: "123".to_string(),
            raw: "123n".to_string(),
        });
        assert_eq!(emit_expr(e), "123n;");
    }

    #[test]
    fn bigint_literal_zero_emits_raw() {
        // 0n  → "0n;"
        let e = Expression::BigIntLiteral(BigIntLiteral {
            cv: None,
            value: "0".to_string(),
            raw: "0n".to_string(),
        });
        assert_eq!(emit_expr(e), "0n;");
    }

    #[test]
    fn bigint_literal_hex_preserves_radix() {
        // 0x1fn  — value "31" semantically, but emitter respects `raw`
        // so hex stays hex on the way out.
        let e = Expression::BigIntLiteral(BigIntLiteral {
            cv: None,
            value: "31".to_string(),
            raw: "0x1fn".to_string(),
        });
        assert_eq!(emit_expr(e), "0x1fn;");
    }

    // ---- EmitError --------------------------------------------

    #[test]
    fn emit_error_is_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let e = EmitError::UnknownCvId {
            id: "x".to_string(),
            site: "test",
        };
        assert_error(&e);
    }
}

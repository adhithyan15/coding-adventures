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
    AssignmentTarget, BinaryExpression, BinaryOperator, BlockStatement, BooleanLiteral,
    BreakStatement, CallExpression, ConditionalExpression, ContinueStatement, Declaration,
    EmptyStatement, Expression, ExpressionStatement, ForInit, ForStatement, FunctionDeclaration,
    FunctionParam, Identifier, IfStatement, LogicalExpression, LogicalOperator,
    MemberExpression, NullLiteral, NumericLiteral, ObjectExpression, Program, ProgramItem,
    Property, PropertyKey, PropertyKind, ReturnStatement, Statement, StringLiteral,
    UnaryExpression, UnaryOperator, VarKind, VariableDeclaration, VariableDeclarator,
    WhileStatement,
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
            TaggedStatement::EmptyStatement(e) => self.emit_empty(e),
        }
    }

    fn emit_expression_statement(&mut self, es: &ExpressionStatement) {
        // Object expressions at the start of a statement parse as
        // blocks. Wrap in parens unconditionally for safety in v1.
        let needs_paren = matches!(es.expression, Expression::ObjectExpression(_));
        self.maybe_map(&es.cv);
        if needs_paren {
            self.write_str("(");
        }
        self.emit_expression(&es.expression);
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

    fn emit_expression(&mut self, e: &Expression) {
        match e {
            Expression::Identifier(i) => self.emit_identifier(i),
            Expression::NumericLiteral(n) => self.emit_numeric(n),
            Expression::StringLiteral(s) => self.emit_string(s),
            Expression::BooleanLiteral(b) => self.emit_boolean(b),
            Expression::NullLiteral(n) => self.emit_null(n),
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
        self.maybe_map(&s.cv);
        if self.opts.ascii_only {
            // Re-render value with ASCII escapes (ignore raw,
            // which may contain bare Unicode).
            self.write_str(&format!("\"{}\"", escape_ascii_only(&s.value)));
        } else {
            // Use raw to preserve original quote style / escapes
            // when present; fall back to a generated form if raw
            // is empty (synthetic node).
            if s.raw.is_empty() {
                self.write_str(&format!("\"{}\"", escape_str_dq(&s.value)));
            } else {
                self.write_str(&s.raw);
            }
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

    fn emit_binary(&mut self, b: &BinaryExpression) {
        self.maybe_map(&b.cv);
        self.write_str("(");
        self.emit_expression(&b.left);
        // Binary operators always get spaces in our output —
        // makes `1 in obj` and `a instanceof b` unambiguous even
        // in minified mode.
        self.required_ws();
        self.write_str(binary_op_str(b.operator));
        self.required_ws();
        self.emit_expression(&b.right);
        self.write_str(")");
    }

    fn emit_logical(&mut self, l: &LogicalExpression) {
        self.maybe_map(&l.cv);
        self.write_str("(");
        self.emit_expression(&l.left);
        self.required_ws();
        self.write_str(logical_op_str(l.operator));
        self.required_ws();
        self.emit_expression(&l.right);
        self.write_str(")");
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
        self.maybe_map(&c.cv);
        self.write_str("(");
        self.emit_expression(&c.test);
        self.pretty_ws();
        self.write_str("?");
        self.pretty_ws();
        self.emit_expression(&c.consequent);
        self.pretty_ws();
        self.write_str(":");
        self.pretty_ws();
        self.emit_expression(&c.alternate);
        self.write_str(")");
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
/// emitted output round-trips numerically. Mirrors the helper of
/// the same name in `closure-pass-constant-fold` so both crates
/// produce consistent text for the same value.
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
    if n.fract() == 0.0 && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    n.to_string()
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
    fn binary_addition_with_parens() {
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(num(2.0)),
            right: Box::new(num(3.0)),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "(2 + 3);");
    }

    #[test]
    fn string_concat_with_parens() {
        let e = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: BinaryOperator::Add,
            left: Box::new(string("foo")),
            right: Box::new(string("bar")),
        });
        let prog = program().with_body(vec![stmt(e)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "(\"foo\" + \"bar\");");
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
        assert_eq!(out.code, "(2 + 3);");
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

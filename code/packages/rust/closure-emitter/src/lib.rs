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
    ArrowBody, ArrowFunctionExpression,
    TemplateElement, TemplateLiteral,
    BooleanLiteral,
    BreakStatement, CallExpression, CatchClause, ClassExpression, ClassMember, ConditionalExpression, ContinueStatement,
    Declaration, DebuggerStatement, DoWhileStatement,
    MethodDefinition, MethodKind,
    EmptyStatement, Expression, ExpressionStatement, ForInStatement, ForInit, ForOfStatement,
    ForStatement,
    ClassDeclaration, FunctionDeclaration, FunctionExpression,
    FunctionParam, Identifier, IfStatement, LabeledStatement, LogicalExpression, LogicalOperator,
    MemberExpression, NewExpression, NullLiteral, NumericLiteral, ObjectExpression, Program, ProgramItem, SequenceExpression,
    ObjectMember, Property, PropertyKey, PropertyKind, ReturnStatement, Statement, StringLiteral,
    SwitchCase, SwitchStatement, ThrowStatement, TryStatement, UnaryExpression, UnaryOperator, UpdateExpression, UpdateOperator,
    RegExpLiteral,
    UndefinedLiteral, VarKind, VariableDeclaration, VariableDeclarator, WhileStatement,
    TaggedTemplateExpression, SpreadElement, YieldExpression, AwaitExpression, ThisExpression,
    Super, NewTarget, ImportMeta, ImportExpression,
    ChainExpression, OptionalCallExpression, OptionalMemberExpression,
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
/// Stack size for the emitter worker thread (see [`emit`]).
///
/// The emitter is a recursive-descent tree walk: `emit_expression_inner`
/// → `emit_binary`/`emit_logical` → `emit_expression_inner` on the left
/// operand, once per operator. A deeply left-nested operator chain — the
/// shape the bridge builds for flat source like `1+1+…+1` (thousands of
/// terms) — therefore recurses once per operator. Past a few thousand
/// levels this overflows the caller's ordinary ~2 MiB stack, which is an
/// **uncatchable** `SIGSEGV`/abort: it kills the whole process, so a
/// `Result`-returning API cannot report it. closurec feeds *untrusted* JS
/// here, so it must not be crashable by pathological input.
///
/// 128 MiB comfortably absorbs the ~20 000-deep adversarial inputs that
/// motivated this, with a healthy margin above them. The margin matters:
/// `emit_expression_inner` is a wide `match` whose per-frame footprint grows
/// as new `Expression` variants are handled (e.g. the CLOC12.151
/// `ArrowFunctionExpression` arm), and per-frame cost also differs by target —
/// aarch64 (Apple-silicon CI) lays out larger frames than x86-64, so a stack
/// merely sized to *just* hold 20 000 levels on one target can overflow on
/// another. A 2× cushion keeps a modest future frame increase from re-breaking
/// the deep-emit DoS regression, while costing nothing for real code (the
/// thread reserves address space lazily; only touched pages fault in).
/// Emission is otherwise **byte-identical** to running on the caller's stack;
/// only the stack size differs.
const EMIT_STACK_SIZE: usize = 128 * 1024 * 1024;

pub fn emit(
    program: &Program,
    _sidecar: &Sidecar,
    cv: &mut CVLog,
    opts: &EmitOptions,
) -> Result<EmitOutput, EmitError> {
    // Run the recursive emission on a large-stack worker so deep (but valid)
    // ASTs emit without overflowing the native stack. `std::thread::scope`
    // lets the worker borrow `program`/`opts` without `'static`; we hand back
    // the owned `out` string and source-map builder and finish the (shallow)
    // source-map serialisation on the caller thread, where `cv` lives.
    let (out, source_map_builder) = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(EMIT_STACK_SIZE)
            .spawn_scoped(scope, || {
                let mut emitter = Emitter::new(opts);
                emitter.emit_program(program);
                (emitter.out, emitter.source_map)
            })
            .expect("failed to spawn emitter worker thread")
            .join()
            .expect("emitter worker thread panicked")
    });

    let source_map = if opts.source_map {
        // Build the source map. Even when no mappings were
        // accumulated (untraced input), this produces a valid
        // v3 blob with empty mappings.
        Some(source_map_builder.build(cv).to_json())
    } else {
        None
    };

    Ok(EmitOutput {
        code: out,
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
            TaggedStatement::DoWhileStatement(d) => self.emit_do_while(d),
            TaggedStatement::ForStatement(f) => self.emit_for(f),
            TaggedStatement::ForInStatement(f) => self.emit_for_in(f),
            TaggedStatement::ForOfStatement(f) => self.emit_for_of(f),
            TaggedStatement::ReturnStatement(r) => self.emit_return(r),
            TaggedStatement::BreakStatement(b) => self.emit_break(b),
            TaggedStatement::ContinueStatement(c) => self.emit_continue(c),
            TaggedStatement::LabeledStatement(l) => self.emit_labeled(l),
            TaggedStatement::ThrowStatement(t) => self.emit_throw(t),
            TaggedStatement::SwitchStatement(s) => self.emit_switch(s),
            TaggedStatement::TryStatement(t) => self.emit_try(t),
            TaggedStatement::EmptyStatement(e) => self.emit_empty(e),
            TaggedStatement::DebuggerStatement(d) => self.emit_debugger(d),
        }
    }

    fn emit_try(&mut self, t: &TryStatement) {
        // try <block> [ catch [(param)] <block> ] [ finally <block> ]
        // No `required_ws` anywhere: every boundary is keyword↔`{`/`}` or
        // `}`↔keyword, which lex cleanly with no separator (`try{…}catch{…}`).
        // `pretty_ws` adds readability spaces only in pretty mode.
        self.maybe_map(&t.cv);
        self.write_str("try");
        self.pretty_ws();
        self.emit_block_statement(&t.block);
        if let Some(h) = &t.handler {
            self.maybe_map(&h.cv);
            self.pretty_ws();
            self.write_str("catch");
            if let Some(param) = &h.param {
                self.pretty_ws();
                self.write_str("(");
                self.write_str(&param.name);
                self.write_str(")");
            }
            self.pretty_ws();
            self.emit_block_statement(&h.body);
        }
        if let Some(finalizer) = &t.finalizer {
            self.pretty_ws();
            self.write_str("finally");
            self.pretty_ws();
            self.emit_block_statement(finalizer);
        }
    }

    fn emit_expression_statement(&mut self, es: &ExpressionStatement) {
        // Object expressions at the start of a statement parse as
        // blocks. The leading-token-disambiguation wrap (per CLOC12.10
        // / gap-024) covers that one case only. Everything else gets
        // precedence-aware emit at parent_prec = 0, which means no
        // wrapping unless an inner expression has a lower-precedence
        // child that requires it.
        // A leading `{` parses as a block and a leading `function`
        // parses as a function *declaration* — both mis-parse a bare
        // expression statement, so wrap them. A leading `class` is the
        // same hazard: it parses as a class *declaration*, so a class
        // *expression* in statement position must be wrapped too. (The
        // general "leftmost token" problem — e.g. a call whose callee is a
        // function expression — is handled by each child's own precedence
        // wrap; this covers the direct cases.)
        let needs_paren = matches!(
            es.expression,
            Expression::ObjectExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ClassExpression(_)
        );
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
            // gap-030 part A: drop the trailing `;` of the
            // block's last statement in compact mode IFF that
            // `;` came from a real statement-terminator (not a
            // body slot). The `}` we're about to write
            // terminates the statement per ECMAScript §11.9
            // (Automatic Semicolon Insertion), so a true
            // terminator `;` is redundant noise that upstream
            // Closure doesn't emit either. But we MUST NOT pop
            // when the trailing `;` is structurally a body
            // (`if(x);`, `for(;;);`, `while(x);`, `lbl:;`)
            // because the grammar requires a Statement there
            // and `}` is not a valid Statement start. The
            // last-child type tells us which case we're in.
            if let Some(last) = b.body.last() {
                if last_stmt_uses_terminator_semi(last) {
                    self.pop_trailing_semi_if_compact();
                }
            }
        }
        self.write_str("}");
    }

    /// Pop a single trailing `;` from the emitter's output
    /// buffer in compact mode. Used by gap-030's
    /// drop-redundant-semi-before-`}` rule under the
    /// `last_stmt_uses_terminator_semi` gate. Pretty mode is
    /// intentionally untouched — visual clarity outranks byte
    /// minimization there.
    fn pop_trailing_semi_if_compact(&mut self) {
        if self.opts.pretty {
            return;
        }
        if self.out.ends_with(';') {
            self.out.pop();
            // Decrement column so any subsequent maybe_map call
            // stays accurate. ASCII `;` is one UTF-16 unit.
            self.col = self.col.saturating_sub(1);
        }
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

    fn emit_do_while(&mut self, d: &DoWhileStatement) {
        // `do <body> while ( <test> ) ;`
        //
        // Token-separation: `do` is a keyword sitting directly before the
        // body, so `do{…}` lexes cleanly when the body is a block, but a
        // bare-statement body would glue (`do foo()` must not become
        // `dofoo()`). Insert a required space only when the body is NOT a
        // block. After the body, a block ends in `}` and every other
        // statement ends in `;`, so the following `while` always lexes
        // cleanly. The trailing `;` is a real statement terminator (poppable
        // before a closing `}` via `last_stmt_uses_terminator_semi`).
        self.maybe_map(&d.cv);
        self.write_str("do");
        if matches!(
            d.body.as_ref(),
            Statement::Tagged(TaggedStatement::BlockStatement(_))
        ) {
            self.pretty_ws();
        } else {
            self.required_ws();
        }
        self.emit_statement(&d.body);
        self.pretty_ws();
        self.write_str("while");
        self.pretty_ws();
        self.write_str("(");
        self.emit_expression(&d.test);
        self.write_str(")");
        self.write_str(";");
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

    fn emit_for_in(&mut self, f: &ForInStatement) {
        // `for ( <left> in <right> ) <body>`
        //
        // The `in` keyword needs a separator on BOTH sides: the left ends in an
        // identifier (`var k` / `k` / `o.p`) and the right starts with one, so
        // `kin` / `inobj` would mis-lex. `required_ws` is always inserted (a
        // single space) — in the rare `a[b] in` / `in (x)` cases the space is
        // one redundant byte but never wrong, matching upstream Closure's
        // spacing around `in`.
        self.maybe_map(&f.cv);
        self.write_str("for");
        self.pretty_ws();
        self.write_str("(");
        match &f.left {
            ForInit::VariableDeclaration(v) => {
                self.emit_variable_declaration(v, /*top_level=*/ false);
            }
            ForInit::Expression(e) => self.emit_expression(e),
        }
        self.required_ws();
        self.write_str("in");
        self.required_ws();
        self.emit_expression(&f.right);
        self.write_str(")");
        self.pretty_ws();
        self.emit_statement(&f.body);
    }

    fn emit_for_of(&mut self, f: &ForOfStatement) {
        // `for ( <left> of <right> ) <body>` — identical to for-in but with the
        // `of` keyword, spaced on both sides for the same token-separation
        // reason (the left ends in an identifier and the right starts with one).
        self.maybe_map(&f.cv);
        self.write_str("for");
        self.pretty_ws();
        self.write_str("(");
        match &f.left {
            ForInit::VariableDeclaration(v) => {
                self.emit_variable_declaration(v, /*top_level=*/ false);
            }
            ForInit::Expression(e) => self.emit_expression(e),
        }
        self.required_ws();
        self.write_str("of");
        self.required_ws();
        self.emit_expression(&f.right);
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

    fn emit_debugger(&mut self, d: &DebuggerStatement) {
        // `debugger;` — the keyword plus a real terminator `;`. The keyword is
        // followed only by `;` (or, after the semi is popped, a `}`/EOF), so no
        // token-separation handling is needed.
        self.maybe_map(&d.cv);
        self.write_str("debugger;");
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

    /// `switch (discriminant) { case test: <consequent>; default: <consequent>; }`.
    ///
    /// Lays out as `switch(<expr>){case <test>:<stmts>case <test>:<stmts>default:<stmts>}`
    /// in compact mode. The closing `}` is emitted directly — no
    /// trailing `;` because a switch is itself a statement
    /// (the block-form, not the expression-form), and the inner
    /// statements terminate themselves.
    ///
    /// Per ECMAScript §13.12, each case clause's consequent is a
    /// list of statements (not a single statement), so we
    /// emit each one in order. Fallthrough is the default; an
    /// explicit `break` inside the consequent terminates the case.
    fn emit_switch(&mut self, s: &SwitchStatement) {
        self.maybe_map(&s.cv);
        self.write_str("switch");
        self.pretty_ws();
        self.write_str("(");
        self.emit_expression(&s.discriminant);
        self.write_str(")");
        self.pretty_ws();
        self.write_str("{");
        if self.opts.pretty && !s.cases.is_empty() {
            self.newline();
            self.indent += 1;
        }
        for (i, case) in s.cases.iter().enumerate() {
            if self.opts.pretty {
                if i > 0 {
                    self.newline();
                }
                self.indent_str();
            }
            self.emit_switch_case(case);
        }
        if self.opts.pretty && !s.cases.is_empty() {
            self.indent -= 1;
            self.newline();
            self.indent_str();
        }
        self.write_str("}");
    }

    fn emit_switch_case(&mut self, c: &SwitchCase) {
        self.maybe_map(&c.cv);
        match &c.test {
            Some(test) => {
                self.write_str("case");
                self.required_ws();
                self.emit_expression(test);
                self.write_str(":");
            }
            None => self.write_str("default:"),
        }
        if c.consequent.is_empty() {
            return;
        }
        if self.opts.pretty {
            self.newline();
            self.indent += 1;
            for (i, s) in c.consequent.iter().enumerate() {
                if i > 0 {
                    self.newline();
                }
                self.indent_str();
                self.emit_statement(s);
            }
            self.indent -= 1;
        } else {
            for s in &c.consequent {
                self.emit_statement(s);
            }
        }
    }

    // ---- Declarations --------------------------------------------

    fn emit_declaration(&mut self, d: &Declaration) {
        match d {
            Declaration::VariableDeclaration(v) => {
                self.emit_variable_declaration(v, /*top_level=*/ true);
            }
            Declaration::FunctionDeclaration(f) => self.emit_function_declaration(f),
            Declaration::ClassDeclaration(c) => self.emit_class_declaration(c),
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
        // gap-030 part B: emit a trailing `;` after the
        // function-declaration's closing `}` in compact mode.
        // Upstream Closure does this to normalise the
        // function-declaration output shape — even at EOF it's
        // a no-op `EmptyStatement`, but in concatenation
        // contexts (multiple top-level declarations) it keeps
        // the next statement unambiguously separated. Pretty
        // mode preserves the unparenthesised shape for
        // readability.
        if !self.opts.pretty {
            self.write_str(";");
        }
    }

    /// Emit a [`FunctionExpression`] — a function in *value* position.
    ///
    /// Byte-identical to [`Self::emit_function_declaration`] for the
    /// `function`/`*`/params/body run, with two deliberate differences:
    ///
    /// 1. **`id` is optional.** An anonymous `function () {}` prints no
    ///    name (`function(){}`); a named one prints it (`function f(){}`).
    ///    `required_ws()` is only invoked in the named case — it emits a
    ///    separating space *only when the adjacent tokens would otherwise
    ///    merge* (`function f`, but `function*f` needs none because `*`
    ///    already delimits), exactly as the declaration relies on.
    /// 2. **No trailing `;`.** A function *declaration* appends a
    ///    normalising `;` after its `}` (gap-030 part B); an expression
    ///    must not — it is embedded inside a larger expression/statement
    ///    whose own emitter owns the terminator. Appending one here would
    ///    corrupt e.g. `f(function(){})` into `f(function(){};)`.
    ///
    /// Parenthesisation in the two contexts where a bare
    /// `function (){}` would be mis-parsed — at the *start* of an
    /// expression statement (parses as a declaration) and as a *call
    /// callee* (`function(){}()` is a syntax error) — is handled by the
    /// precedence machinery: [`expr_prec`] tags `FunctionExpression`
    /// below `PREC_PRIMARY`, so a call/member parent wraps it, and
    /// [`Self::emit_expression_statement`] wraps a leading one.
    fn emit_function_expression(&mut self, f: &FunctionExpression) {
        self.maybe_map(&f.cv);
        if f.is_async {
            self.write_str("async");
            self.required_ws();
        }
        self.write_str("function");
        if f.generator {
            self.write_str("*");
        }
        if let Some(id) = &f.id {
            self.required_ws();
            self.emit_identifier(id);
        }
        self.emit_param_list_and_body(&f.params, &f.body);
    }

    /// Emit `(p1,p2){body}` — the shared parameter-list + block-body tail of a
    /// function value. Used by [`Self::emit_function_expression`] (after the
    /// `function[*][ id]` head) and by [`Self::emit_class_member`] (after the
    /// `[static ][get|set ][*]key` head), since a class method's value is a
    /// [`FunctionExpression`] whose params/body print identically.
    fn emit_param_list_and_body(&mut self, params: &[FunctionParam], body: &BlockStatement) {
        self.write_str("(");
        for (i, p) in params.iter().enumerate() {
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
        self.emit_block_statement(body);
    }

    /// Emit a [`ClassExpression`] — `class[ id][ extends S]{members}`.
    ///
    /// ```text
    ///   class {}                     → class{}
    ///   class C {}                   → class C{}
    ///   class C extends B {}         → class C extends B{}
    ///   class { m() {} }             → class{m(){}}
    ///   class { static get x() {} }  → class{static get x(){}}
    /// ```
    ///
    /// The `extends` operand is emitted at `PREC_PRIMARY`: a `LeftHandSide`
    /// superclass (identifier `extends B`, member `extends ns.B`, call
    /// `extends mixin(B)`) stays bare, while anything looser (a conditional
    /// `extends (a?b:c)`) is wrapped — which is exactly the grammar's
    /// requirement. Members print back-to-back with no separators (each carries
    /// its own `{…}`); the whole node's own wrapping in a tighter parent is
    /// handled by [`expr_prec`] (`PREC_UNARY`).
    fn emit_class(&mut self, c: &ClassExpression) {
        self.maybe_map(&c.cv);
        self.write_str("class");
        if let Some(id) = &c.id {
            self.required_ws();
            self.emit_identifier(id);
        }
        self.emit_class_tail(&c.super_class, &c.body);
    }

    /// Emit a [`ClassDeclaration`] — `class <id>[ extends S]{members}`.
    ///
    /// The *statement* form of a class. Byte-identical to [`Self::emit_class`]
    /// (the expression form) for the `[ extends S]{members}` tail — both call
    /// [`Self::emit_class_tail`] — with three deliberate differences, each the
    /// exact mirror of the [`FunctionDeclaration`] vs `FunctionExpression`
    /// split:
    ///
    /// 1. **`id` always prints.** A declaration's `id` is non-optional (a class
    ///    written as a statement must bind a name — `class {}` in statement
    ///    position is a syntax error), so — unlike the expression form's
    ///    `if let Some(id)` — the name is emitted unconditionally, with a
    ///    `required_ws()` after `class` exactly as
    ///    [`Self::emit_function_declaration`] does after `function`.
    /// 2. **No precedence wrap / no statement-start parenthesis.** A class
    ///    *expression* is tagged `PREC_UNARY` and
    ///    [`Self::emit_expression_statement`] wraps a leading one (`(class{});`)
    ///    because a statement-position `class` would otherwise parse as a
    ///    *declaration* — which is precisely what this node **is**. So the
    ///    declaration form has no `expr_prec` entry and is never wrapped.
    /// 3. **No trailing `;`.** [`Self::emit_function_declaration`] appends a
    ///    normalising `;` after its `}` (gap-030 part B); a **class**
    ///    declaration does not — upstream Closure terminates a class declaration
    ///    with its `}` alone (a class body is self-delimiting and, unlike a bare
    ///    `function(){}` value, subject to no ASI hazard). The PR3 conformance
    ///    port validates this against `CodePrinterTest`.
    fn emit_class_declaration(&mut self, c: &ClassDeclaration) {
        self.maybe_map(&c.cv);
        self.write_str("class");
        // `id` is required for a declaration — always emit it (with the
        // mandatory `class C` separating space).
        self.required_ws();
        self.emit_identifier(&c.id);
        self.emit_class_tail(&c.super_class, &c.body);
    }

    /// Emit the shared `[ extends S]{members}` tail of a class — the part after
    /// the `class[ id]` head that is identical for the expression
    /// ([`Self::emit_class`]) and declaration ([`Self::emit_class_declaration`])
    /// forms.
    ///
    /// The `extends` operand is emitted at `PREC_PRIMARY`: a `LeftHandSide`
    /// superclass (identifier `extends B`, member `extends ns.B`, call
    /// `extends mixin(B)`) stays bare, while anything looser (a conditional
    /// `extends (a?b:c)`) is wrapped — exactly the grammar's requirement.
    /// Members print back-to-back with no separators (each carries its own
    /// `{…}`).
    fn emit_class_tail(
        &mut self,
        super_class: &Option<Box<Expression>>,
        body: &[ClassMember],
    ) {
        if let Some(sup) = super_class {
            self.required_ws();
            self.write_str("extends");
            self.required_ws();
            self.emit_expression_inner(sup, PREC_PRIMARY);
        }
        // No space before the brace even in pretty mode — Closure prints
        // `class C{...}` (the members carry their own layout).
        self.write_str("{");
        for member in body {
            match member {
                ClassMember::Method(m) => self.emit_class_member(m),
            }
        }
        self.write_str("}");
    }

    /// Emit one [`MethodDefinition`]: `[static ][get|set ][*]key(params){body}`.
    ///
    /// Order of prefixes matches the grammar (and Closure): `static` first, then
    /// the accessor keyword (`get`/`set`) if any, then the generator `*`, then
    /// the key. A computed key (`[expr]`) is bracketed by `emit_property_key`
    /// (via `PropertyKey::Expression`), exactly as an object-literal computed
    /// key. `constructor` and an ordinary method share the same shape (no
    /// keyword prefix) — the `kind` only distinguishes them for the passes.
    fn emit_class_member(&mut self, m: &MethodDefinition) {
        self.maybe_map(&m.cv);
        if m.is_static {
            self.write_str("static");
            self.required_ws();
        }
        match m.kind {
            // `get`/`set` accessors: the keyword, then the key. Accessors are
            // never `async` or generators (the grammar forbids it), so the
            // `value`'s `is_async`/`generator` flags are not consulted here.
            MethodKind::Get => {
                self.write_str("get");
                self.required_ws();
            }
            MethodKind::Set => {
                self.write_str("set");
                self.required_ws();
            }
            // An ordinary method or the constructor. The method head allows
            // `async` and/or `*` before the key: grammar order is
            // `async [*] key`, so `async` prints first (with a space before the
            // key), then the generator `*`. For a plain `m(){}` both flags are
            // false and neither prints.
            MethodKind::Constructor | MethodKind::Method => {
                if m.value.is_async {
                    self.write_str("async");
                    self.required_ws();
                }
                if m.value.generator {
                    self.write_str("*");
                }
            }
        }
        self.emit_property_key(&m.key);
        self.emit_param_list_and_body(&m.value.params, &m.value.body);
    }

    /// Emit an [`ArrowFunctionExpression`] — the `=>` form.
    ///
    /// Three shape rules distinguish it from
    /// [`Self::emit_function_expression`]:
    ///
    /// 1. **Param parens are dropped for a single plain identifier.**
    ///    `x => x` prints without parens, matching Closure's minified
    ///    output; zero params (`() =>`) and two-or-more (`(a,b) =>`) keep
    ///    them. (Destructuring / default / rest params would force parens
    ///    too, but those aren't representable yet.)
    /// 2. **Dual body.** A [`ArrowBody::Block`] emits exactly like a
    ///    function body (`x => { return x }`); a [`ArrowBody::Expression`]
    ///    (concise body) emits the bare expression (`x => x + 1`) at
    ///    `PREC_ASSIGNMENT` — the grammar makes the concise body an
    ///    `AssignmentExpression`.
    /// 3. **Object-literal concise bodies are wrapped.** `() => ({a:1})` —
    ///    without the parens the leading `{` parses as a *block* body, so
    ///    a concise body that is an [`Expression::ObjectExpression`] gets
    ///    a disambiguating wrap. (The deeper leftmost-`{` case, e.g.
    ///    `() => ({}).x`, is not yet wrapped — see CLOC12-gaps.)
    ///
    /// Parenthesisation in the two mis-parse contexts — as a call callee
    /// (`(() => {})()`) and as a member object (`(() => {}).x`) — is
    /// handled by the precedence machinery: [`expr_prec`] tags an arrow at
    /// `PREC_ASSIGNMENT`, so a call/member parent (which emits at
    /// `PREC_PRIMARY`) wraps it. Unlike a function expression, an arrow at
    /// the *start* of an expression statement needs no wrap — `x => x;` is
    /// a valid statement — so [`Self::emit_expression_statement`] leaves it
    /// alone.
    fn emit_arrow_function_expression(&mut self, a: &ArrowFunctionExpression) {
        self.maybe_map(&a.cv);
        if a.is_async {
            self.write_str("async");
        }
        // Param list — single plain identifier drops the parens.
        if a.params.len() == 1 {
            // `async x=>` needs a separating space so `async` and the
            // param identifier don't merge into `asyncx`. The
            // parenthesised forms below begin with `(`, which
            // self-delimits, so no space is emitted there (`async()=>`).
            if a.is_async {
                self.required_ws();
            }
            match &a.params[0] {
                FunctionParam::Identifier(id) => self.emit_identifier(id),
            }
        } else {
            self.write_str("(");
            for (i, p) in a.params.iter().enumerate() {
                if i > 0 {
                    self.write_str(",");
                    self.pretty_ws();
                }
                match p {
                    FunctionParam::Identifier(id) => self.emit_identifier(id),
                }
            }
            self.write_str(")");
        }
        self.pretty_ws();
        self.write_str("=>");
        self.pretty_ws();
        match &a.body {
            ArrowBody::Block(b) => self.emit_block_statement(b),
            ArrowBody::Expression(e) => {
                // A concise body that starts with `{` (an object literal)
                // would otherwise be read as a block body.
                let wrap = matches!(**e, Expression::ObjectExpression(_));
                if wrap {
                    self.write_str("(");
                }
                self.emit_expression_inner(e, PREC_ASSIGNMENT);
                if wrap {
                    self.write_str(")");
                }
            }
        }
    }

    /// Emit a [`TemplateLiteral`] — a backtick template string.
    ///
    /// The `quasis` (fixed string parts) and `expressions` (`${…}` inserts)
    /// interleave, and the structural invariant `quasis.len() ==
    /// expressions.len() + 1` guarantees a quasi both opens and closes the
    /// run:
    ///
    /// ```text
    ///   `q0${e0}q1${e1}…qN`
    /// ```
    ///
    /// Each quasi is emitted from its **raw** text (escape sequences intact,
    /// exactly as written) so the template round-trips byte-for-byte; the
    /// `${` / `}` delimiters make each inserted expression an unambiguous
    /// full-expression context, so it is emitted at the loosest precedence
    /// (no wrapping — the braces already delimit it).
    fn emit_template_literal(&mut self, t: &TemplateLiteral) {
        self.maybe_map(&t.cv);
        self.write_str("`");
        for (i, quasi) in t.quasis.iter().enumerate() {
            self.emit_template_element(quasi);
            // Between quasi i and quasi i+1 sits expression i (there are
            // exactly `quasis.len() - 1` of them).
            if let Some(expr) = t.expressions.get(i) {
                self.write_str("${");
                self.emit_expression_inner(expr, 0);
                self.write_str("}");
            }
        }
        self.write_str("`");
    }

    /// Emit a **tagged** template — `` tag`a${x}b` ``.
    ///
    /// The tag is emitted at `PREC_PRIMARY` (member/call strength): a plain
    /// identifier or member-chain tag prints bare (`` a.b`x` ``), while any
    /// looser tag is parenthesised (`` (a,b)`x` ``, `` (a=b)`x` `` — unusual but
    /// handled defensively; a bare `` a,b`x` `` would tag only `b`). The
    /// template literal follows the tag directly — the `tag`↔`` ` `` boundary
    /// never token-fuses, so no separator is spent — reusing
    /// [`Self::emit_template_literal`] verbatim (so the quasi's `raw` segments
    /// and `${…}` substitutions round-trip exactly as an untagged template).
    fn emit_tagged_template(&mut self, t: &TaggedTemplateExpression) {
        self.maybe_map(&t.cv);
        self.emit_expression_inner(&t.tag, PREC_PRIMARY);
        self.emit_template_literal(&t.quasi);
    }

    /// Emit one [`TemplateElement`] — its verbatim `raw` text.
    ///
    /// A template quasi is the one *primary* token whose `raw` text may legally
    /// contain a **literal newline**: a multiline template preserves its interior
    /// line breaks byte-for-byte (`` `a⏎b` `` prints back with the newline
    /// intact). Every other token the emitter writes is single-line.
    ///
    /// The low-level [`Self::write_str`] deliberately forbids an embedded `'\n'`
    /// (it `debug_assert!`s the run is newline-free) because a raw newline must
    /// route through [`Self::newline`] to keep the source-map line/column
    /// bookkeeping correct — `write_str` only advances the *column*, whereas
    /// `newline` bumps the *line* and resets the column. So we split `raw` on
    /// `'\n'` and hand each line segment to `write_str`, emitting a real
    /// `newline()` between segments:
    ///
    /// ```text
    ///   raw = "a\nb\nc"   →   write_str("a") newline() write_str("b")
    ///                          newline() write_str("c")
    /// ```
    ///
    /// `str::split('\n')` yields `N + 1` pieces for `N` newlines, including an
    /// empty leading piece when `raw` starts with `'\n'` and an empty trailing
    /// piece when it ends with one; writing an empty `&str` is a harmless no-op,
    /// so the line count still lands exactly on the number of `'\n'`s. Other
    /// line-terminator bytes a raw may carry — a lone `'\r'` (as in a `\r\n`
    /// pair, where the `'\r'` rides on the end of the preceding segment) and the
    /// Unicode separators `U+2028` / `U+2029` — are written verbatim as ordinary
    /// characters: bytes round-trip exactly, and only their column bookkeeping is
    /// approximate, which is a source-map nicety, not an output-correctness bug.
    fn emit_template_element(&mut self, q: &TemplateElement) {
        self.maybe_map(&q.cv);
        let mut segments = q.raw.split('\n');
        // There is always at least one segment (`split` never yields empty).
        if let Some(first) = segments.next() {
            self.write_str(first);
            for segment in segments {
                self.newline();
                self.write_str(segment);
            }
        }
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
            Expression::UndefinedLiteral(u) => self.emit_undefined(u),
            Expression::RegExpLiteral(r) => self.emit_regexp(r),
            Expression::BinaryExpression(b) => self.emit_binary(b),
            Expression::LogicalExpression(l) => self.emit_logical(l),
            Expression::UnaryExpression(u) => self.emit_unary(u),
            Expression::UpdateExpression(u) => self.emit_update(u),
            Expression::AssignmentExpression(a) => self.emit_assignment(a),
            Expression::ConditionalExpression(c) => self.emit_conditional(c),
            Expression::CallExpression(c) => self.emit_call(c),
            Expression::NewExpression(n) => self.emit_new(n),
            Expression::SequenceExpression(s) => self.emit_sequence(s),
            Expression::MemberExpression(m) => self.emit_member(m),
            Expression::OptionalMemberExpression(m) => self.emit_optional_member(m),
            Expression::OptionalCallExpression(c) => self.emit_optional_call(c),
            Expression::ChainExpression(c) => self.emit_chain(c),
            Expression::ArrayExpression(a) => self.emit_array(a),
            Expression::ObjectExpression(o) => self.emit_object(o),
            Expression::FunctionExpression(f) => self.emit_function_expression(f),
            Expression::ArrowFunctionExpression(a) => self.emit_arrow_function_expression(a),
            Expression::ClassExpression(c) => self.emit_class(c),
            Expression::TemplateLiteral(t) => self.emit_template_literal(t),
            Expression::TaggedTemplateExpression(t) => self.emit_tagged_template(t),
            Expression::SpreadElement(s) => self.emit_spread(s),
            Expression::YieldExpression(y) => self.emit_yield(y),
            Expression::AwaitExpression(a) => self.emit_await(a),
            Expression::ImportExpression(e) => self.emit_import_expression(e),
            Expression::ThisExpression(t) => self.emit_this(t),
            Expression::Super(s) => self.emit_super(s),
            Expression::NewTarget(n) => self.emit_new_target(n),
            Expression::ImportMeta(n) => self.emit_import_meta(n),
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
        // Closure-style minification: `true` (4 chars) → `!0` (2), `false`
        // (5) → `!1` (2). `!0` evaluates to `true` and `!1` to `false` in
        // every context (`!` coerces its operand to a boolean and negates;
        // `!0 === true`, `!1 === false`), so the substitution is value-exact.
        //
        // PRECEDENCE: `!0` / `!1` are UnaryExpressions, NOT primaries, so they
        // bind LOOSER than member access, call, `new`, and tagged templates.
        // Emitting `true.x` naively as `!0.x` would reparse as `!(0.x)` — a
        // miscompile. We avoid this WITHOUT any local paren logic here:
        // `expr_prec` tags `BooleanLiteral` at `PREC_UNARY` (exactly like the
        // `void 0` UndefinedLiteral case), so `emit_expression_inner` inserts
        // the needed parens automatically in higher-precedence parents —
        // `(!0).x`, `(!0)()`, `new (!1)` — while leaving the common cases
        // (`x=!0`, `[!0]`, `f(!0)`, `a&&!0`, `return!0`) paren-free.
        self.write_str(if b.value { "!0" } else { "!1" });
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

    /// Emit the `undefined` literal as `void 0`.
    ///
    /// **Why `void 0` and not the keyword `undefined`?** In ECMAScript
    /// `undefined` is an *identifier*, not a reserved word. Code can
    /// legally do `var undefined = 1;` in non-strict mode (or just
    /// declare a local `undefined` parameter) and that binding shadows
    /// the global. Reading the identifier `undefined` from inside such
    /// a scope would yield the shadow value, not the genuine undefined.
    ///
    /// `void <expression>` always evaluates `<expression>` and then
    /// produces the **real** undefined value, regardless of any name
    /// in scope.  `void 0` is the shortest spelling — three characters
    /// vs nine for the keyword (plus shadow-safe). This matches upstream
    /// Closure Compiler's `CodePrinter` behaviour.
    fn emit_undefined(&mut self, u: &UndefinedLiteral) {
        self.maybe_map(&u.cv);
        self.write_str("void 0");
    }

    /// `/pattern/flags` — a regex literal. The source is reconstructed
    /// verbatim: `pattern` is the opaque body between the slashes (its own `\/`
    /// escapes are already part of the text) and `flags` is the trailing flag
    /// set. No escaping or quote-choice applies — a regex has exactly one
    /// spelling.
    fn emit_regexp(&mut self, r: &RegExpLiteral) {
        self.maybe_map(&r.cv);
        self.write_str("/");
        self.write_str(&r.pattern);
        self.write_str("/");
        self.write_str(&r.flags);
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
        // Operand precedences. Almost every binary operator is LEFT-associative,
        // so the left child accepts the same precedence (no parens for
        // `a+b+c`) and the right child must be strictly higher (`a-(b-c)`).
        //
        // `**` (exponentiation) is the exception on BOTH counts:
        //   • It is RIGHT-associative, so `a**b**c` is `a**(b**c)`: the RIGHT
        //     child accepts the same precedence (no parens) and it is the LEFT
        //     child that must be strictly higher.
        //   • Its grammar base is an `UpdateExpression`, NOT a `UnaryExpression`
        //     — `-a**2`, `~a**2`, `!a**2` are SYNTAX ERRORS. The base must
        //     therefore bind tighter than unary, so we require `PREC_UNARY + 1`
        //     on the left, which parenthesises a unary (or lower) base:
        //     `(-a)**2`.
        let (left_prec, right_prec) = if matches!(b.operator, BinaryOperator::Exp) {
            (PREC_UNARY + 1, my_prec)
        } else {
            (my_prec, my_prec + 1)
        };
        self.emit_expression_inner(&b.left, left_prec);
        let op = binary_op_str(b.operator);

        // Word-shaped operators MUST keep a space on both sides or they fuse
        // with their operands into a single identifier (`1 in obj`, not `1inobj`;
        // `a instanceof b`, not `ainstanceofb`).
        if matches!(b.operator, BinaryOperator::In | BinaryOperator::InstanceOf) {
            self.required_ws();
            self.write_str(op);
            self.required_ws();
            self.emit_expression_inner(&b.right, right_prec);
            return;
        }

        // Every other (symbolic) operator is emitted tight in compact mode
        // (`a+b`, `a&&b`, `a<<b`, `a===b`) — a space only in `pretty` mode. The
        // ONLY token-merge hazard is the additive operators `+` / `-`: if the
        // left operand already ends with the same sign, or the right operand
        // begins with it, dropping the space fuses the pair into the
        // increment/decrement token — `a+ +b` would become `a++b` (parsed
        // `a++ b`), a MISCOMPILE. No other operator can fuse: no operand begins
        // or ends with `<`,`>`,`&`,`|`,`*`,`/`,`%`,`^`,`=` in a way that forms a
        // different token, and the right operand can never lead with `*` or `/`
        // (so `/` cannot start a `/*`//`//` comment), since `++`/`--` are not
        // representable unary operators here. We guard both seams for `+`/`-`.
        let sign = match b.operator {
            BinaryOperator::Add => Some('+'),
            BinaryOperator::Sub => Some('-'),
            _ => None,
        };
        let left_needs_space =
            self.opts.pretty || sign.is_some_and(|sc| self.out.ends_with(sc));
        if left_needs_space {
            self.write_str(" ");
        }
        self.write_str(op);
        let right_needs_space =
            self.opts.pretty || sign.is_some_and(|sc| arg_starts_with_sign(&b.right, sc));
        if right_needs_space {
            self.write_str(" ");
        }
        self.emit_expression_inner(&b.right, right_prec);
    }

    fn emit_logical(&mut self, l: &LogicalExpression) {
        // `&&` / `||` / `??` are symbolic and carry no token-merge hazard (no
        // operand begins or ends with `&`, `|`, or `?`), so they emit tight in
        // compact mode and spaced only in `pretty` mode.
        self.maybe_map(&l.cv);
        let my_prec = logical_prec(l.operator);
        self.emit_expression_inner(&l.left, my_prec);
        if self.opts.pretty {
            self.write_str(" ");
        }
        self.write_str(logical_op_str(l.operator));
        if self.opts.pretty {
            self.write_str(" ");
        }
        self.emit_expression_inner(&l.right, my_prec + 1);
    }

    fn emit_unary(&mut self, u: &UnaryExpression) {
        self.maybe_map(&u.cv);
        let s = unary_op_str(u.operator);
        self.write_str(s);
        // Two things can go wrong between a prefix operator and its
        // argument; both produce a *miscompile*, not just ugly output.
        //
        // 1. Word-shaped ops (`typeof` / `void` / `delete`) need a space
        //    so the operator name doesn't fuse with the operand
        //    (`typeofx` is one identifier).
        //
        // 2. Sign ops (`-` / `+`) need a space when the argument would
        //    print a leading same-sign character, or the two signs fuse
        //    into the *decrement / increment* token: `-(-a)` must print
        //    `- -a`, never `--a` (which JS parses as `--a`, pre-decrement
        //    of `a`). See `arg_starts_with_sign`.
        if matches!(
            u.operator,
            UnaryOperator::TypeOf | UnaryOperator::Void | UnaryOperator::Delete
        ) {
            self.required_ws();
        } else if let Some(sign) = sign_op_char(u.operator) {
            if arg_starts_with_sign(&u.argument, sign) {
                self.required_ws();
            }
        }
        // Emit the argument at unary binding strength so that any
        // lower-precedence operand (binary, logical, conditional,
        // assignment, sequence) is parenthesised. Without this,
        // `!(a == b)` printed as `!a == b`, which JS reparses as
        // `(!a) == b` — a different program.
        self.emit_expression_inner(&u.argument, PREC_UNARY);
    }

    /// Emit an update expression — `++x` / `x++` / `--x` / `x--`.
    ///
    /// ```text
    ///   prefix:   <op><arg>     ++x   --x
    ///   postfix:  <arg><op>     x++   x--
    /// ```
    ///
    /// The operand is emitted at `PREC_UNARY` so anything looser is
    /// parenthesised (a valid update target — an identifier or member — is
    /// already tight, so this only matters defensively).
    ///
    /// **Seam hazards** are handled without a guard *here*:
    ///   * A *prefix* update after a sign operator (`a - --b`, `-(--x)`) would
    ///     fuse into `a---b` / `---x` and mis-tokenise; the binary/unary
    ///     emitters guard that seam by consulting [`arg_starts_with_sign`],
    ///     which reports a prefix update's leading `+`/`-`.
    ///   * A *postfix* update ends in `+`/`-`, so a following binary `+`/`-`
    ///     (`x++ + y`) would fuse; the binary emitter's left-seam check already
    ///     inspects the emitted output tail and inserts the space.
    /// The prefix operator's own seam with its operand never fuses: `++`/`--`
    /// are already maximal-munch tokens, so `++ +x` and `+++x` tokenise
    /// identically (and an update of a non-reference operand is invalid input
    /// anyway).
    fn emit_update(&mut self, u: &UpdateExpression) {
        self.maybe_map(&u.cv);
        let op = update_op_str(u.operator);
        if u.prefix {
            self.write_str(op);
            self.emit_expression_inner(&u.argument, PREC_UNARY);
        } else {
            self.emit_expression_inner(&u.argument, PREC_UNARY);
            self.write_str(op);
        }
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
        // The RHS is an `AssignmentExpression`, so it is emitted at
        // `PREC_ASSIGNMENT`: a looser *sequence* RHS wraps (`x=(a,b)`, never
        // `x=a,b` which parses as `(x=a),b`). Every other node is
        // `PREC_ASSIGNMENT` or higher and prints bare.
        self.emit_expression_inner(&a.right, PREC_ASSIGNMENT);
    }

    fn emit_conditional(&mut self, c: &ConditionalExpression) {
        // `cond ? consequent : alternate`. The ECMAScript grammar is
        //   ConditionalExpression :
        //     ShortCircuitExpression ? AssignmentExpression : AssignmentExpression
        // so BOTH branches are full AssignmentExpressions and need NO parens
        // around an assignment or a nested conditional. The `?`/`:` punctuation
        // delimits them, so `a ? b = 1 : c = 2` reparses identically to
        // `a ? (b=1) : (c=2)` and `a ? b ? c : d : e` to `a ? (b?c:d) : e`.
        //
        // We therefore emit both branches at `PREC_ASSIGNMENT` — which wraps
        // only a still-looser SequenceExpression (`a?(b,c):d` — a bare comma
        // branch would be captured by the enclosing statement), never an
        // assignment or nested conditional. Previously the consequent was
        // emitted at `PREC_CONDITIONAL + 1` and the alternate at
        // `PREC_CONDITIONAL`, so an assignment branch (`a?b=1:c`) was needlessly
        // parenthesised (`a?(b=1):c`).
        //
        // The TEST is different: it is a `ShortCircuitExpression`, which does
        // NOT include assignment or conditional, so a test that IS an
        // assignment or conditional MUST keep its parens — `a=1?b:c` parses as
        // `a=(1?b:c)`, not `(a=1)?b:c`. Emitting the test at
        // `PREC_CONDITIONAL + 1` keeps `(a=1)?b:c` / `(a?b:c)?d:e` correctly
        // wrapped.
        self.maybe_map(&c.cv);
        self.emit_expression_inner(&c.test, PREC_CONDITIONAL + 1);
        self.pretty_ws();
        self.write_str("?");
        self.pretty_ws();
        self.emit_expression_inner(&c.consequent, PREC_ASSIGNMENT);
        self.pretty_ws();
        self.write_str(":");
        self.pretty_ws();
        self.emit_expression_inner(&c.alternate, PREC_ASSIGNMENT);
    }

    fn emit_call(&mut self, c: &CallExpression) {
        self.maybe_map(&c.cv);
        // Same precedence requirement as `emit_member`'s object: the callee must
        // bind at least as tightly as the call, or its parens are required.
        // `emit_expression` (parent precedence 0) dropped them, so `(a||b)()`
        // became `a||b()` (`a||(b())`) and `(a=b)(c)` became `a=b(c)` — both
        // miscompiles. `PREC_PRIMARY` keeps `a.b()` / `f()()` paren-free and
        // wraps any lower-precedence callee.
        self.emit_expression_inner(&c.callee, PREC_PRIMARY);
        self.write_str("(");
        for (i, a) in c.arguments.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            // An argument is an `AssignmentExpression` in the grammar, so it is
            // emitted at `PREC_ASSIGNMENT`: a looser *sequence* argument wraps
            // (`f((a,b),c)`, never the three-argument `f(a,b,c)`), while every
            // other node — already `PREC_ASSIGNMENT` or higher — prints bare.
            self.emit_expression_inner(a, PREC_ASSIGNMENT);
        }
        self.write_str(")");
    }

    /// Emit a `new` expression — `new Ctor(a, b)`.
    ///
    /// ```text
    ///   new X()          →  new X()
    ///   new a.b.c()      →  new a.b.c()      member-chain callee, no wrap
    ///   new (f())()      →  new (f())()      call in the callee spine MUST wrap
    /// ```
    ///
    /// # Two seams to get right
    ///
    /// 1. **`new` is a word keyword.** Directly before an identifier or member
    ///    callee it would fuse (`newX` is one identifier), so a separator is
    ///    required — exactly like `typeof`/`void`/`delete` in [`emit_unary`].
    ///    When the callee is instead wrapped in parens (`new(f())()`) the `(`
    ///    already separates the tokens, so no space is spent.
    ///
    /// 2. **The callee cannot end in a call.** The ECMAScript grammar makes the
    ///    `new` target a `MemberExpression`, which excludes `CallExpression`.
    ///    So if the callee's member spine bottoms out in a call, emitting it
    ///    bare would let the `(args)` we append bind to that *inner* call:
    ///    `new f()()` reparses as `(new f())()`. We parenthesise the callee in
    ///    that case (`new (f())()`), which is the only shape that needs it — a
    ///    plain identifier or a pure member chain (`a.b.c`) is a valid target
    ///    as-is. See [`new_callee_needs_parens`].
    ///
    /// The non-wrapped callee is emitted at `PREC_PRIMARY`, which keeps member
    /// chains paren-free while still wrapping anything looser (a binary,
    /// conditional, etc. — not valid targets, but handled defensively).
    fn emit_new(&mut self, n: &NewExpression) {
        self.maybe_map(&n.cv);
        self.write_str("new");
        if new_callee_needs_parens(&n.callee) {
            self.write_str("(");
            self.emit_expression(&n.callee);
            self.write_str(")");
        } else {
            // `new`↔callee is a keyword↔word boundary — always separate.
            self.required_ws();
            self.emit_expression_inner(&n.callee, PREC_PRIMARY);
        }
        self.write_str("(");
        for (i, a) in n.arguments.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            // Assignment-position argument — a sequence wraps. See `emit_call`.
            self.emit_expression_inner(a, PREC_ASSIGNMENT);
        }
        self.write_str(")");
    }

    /// Emit a `SequenceExpression` — the comma operator `a, b, c`.
    ///
    /// The operands print comma-separated with no minified inter-operand space
    /// (`a,b,c`). Each operand is emitted at `PREC_ASSIGNMENT`: an operand that
    /// is itself a looser sequence (e.g. a pass built `(a,b),c`) is wrapped —
    /// but every non-sequence operand is `PREC_ASSIGNMENT` or higher, so it
    /// prints bare. The *sequence itself* is `PREC_SEQUENCE` (the loosest), so a
    /// parent that emits its child above statement level wraps the whole
    /// sequence — see the four assignment-position sites (`emit_call` /
    /// `emit_new` arguments, `emit_array` elements, `emit_assignment` RHS) and
    /// `expr_prec`.
    fn emit_sequence(&mut self, s: &SequenceExpression) {
        self.maybe_map(&s.cv);
        for (i, e) in s.expressions.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            self.emit_expression_inner(e, PREC_ASSIGNMENT);
        }
    }

    /// Emit a `SpreadElement` — the `...arg` unpack prefix.
    ///
    /// The three literal `.` characters print with no interior space, then the
    /// argument follows at `PREC_ASSIGNMENT`. That precedence is the crux: the
    /// argument grammar is an `AssignmentExpression`, so everything at or above
    /// assignment strength prints bare (`...a`, `...a.b`, `...f()`, `...a?b:c`,
    /// `...a=b`), while the one looser form — a **sequence** — is wrapped:
    /// `...(a,b)`. A bare `...a,b` would parse as *two* list slots (spread `a`,
    /// then plain `b`), a miscompile, so the parens are mandatory. There is no
    /// space between `...` and the argument (`...a`, never `... a`).
    fn emit_spread(&mut self, s: &SpreadElement) {
        self.maybe_map(&s.cv);
        self.write_str("...");
        self.emit_expression_inner(&s.argument, PREC_ASSIGNMENT);
    }

    /// Emit a `YieldExpression` — `yield`, `yield x`, or `yield* xs`.
    ///
    /// Three shapes, driven by the two independent fields `delegate` and
    /// `argument`:
    ///
    /// ```text
    ///   yield             delegate=false, argument=None    → "yield"
    ///   yield x           delegate=false, argument=Some    → "yield x"
    ///   yield* xs         delegate=true,  argument=Some    → "yield*xs"
    /// ```
    ///
    /// **Token separation.** `yield` is a *word* keyword, so a non-delegating
    /// `yield` followed by an argument needs a mandatory separator or the two
    /// would fuse into one identifier (`yieldx`): we emit `required_ws()`
    /// between them (`yield x`). A delegating `yield*` needs no separator — the
    /// `*` is punctuation that already terminates the keyword token, and no
    /// valid `AssignmentExpression` argument begins with `*`, so `yield*xs`
    /// tokenises unambiguously. (A delegating yield without an argument would be
    /// a syntax error upstream; if a malformed AST presents `delegate=true,
    /// argument=None` we still emit a bare `yield*`, leaving the invalidity
    /// visible rather than silently rewriting it.)
    ///
    /// **Precedence.** The argument prints at `PREC_ASSIGNMENT` — the yield
    /// operand grammar is an `AssignmentExpression`, so a conditional or
    /// assignment argument prints bare (`yield a?b:c`, `yield a=b`) while a
    /// looser sequence wraps (`yield (a,b)`). The wrapping of the *whole* yield
    /// in a tighter parent is handled by `expr_prec` (which tags it at
    /// `PREC_ASSIGNMENT`) plus `emit_expression_inner`, exactly as for arrows
    /// and assignments — no local paren logic is needed here.
    fn emit_yield(&mut self, y: &YieldExpression) {
        self.maybe_map(&y.cv);
        self.write_str("yield");
        if y.delegate {
            self.write_str("*");
        }
        if let Some(arg) = &y.argument {
            // A non-delegating `yield` must be separated from its argument;
            // after a delegating `yield*` the `*` already separates the tokens.
            if !y.delegate {
                self.required_ws();
            }
            self.emit_expression_inner(arg, PREC_ASSIGNMENT);
        }
    }

    /// Emit an `AwaitExpression` — `await x`.
    ///
    /// `await` is a *word-shaped* unary operator, so — exactly like
    /// `typeof` / `void` / `delete` in [`emit_unary`] — it always needs a
    /// mandatory separator before its operand, or the keyword would fuse into
    /// one identifier (`awaitx`). We emit `required_ws()` then the operand at
    /// `PREC_UNARY`, so a looser operand is parenthesised (`await (a+b)` for the
    /// binary `a+b`), while a member / call / unary operand prints bare
    /// (`await a.b`, `await f()`, `await -x`). The wrapping of the *whole* await
    /// in a tighter parent — `(await p).x`, `(await f)()` — is handled by
    /// `expr_prec` (which tags it at `PREC_UNARY`) plus `emit_expression_inner`,
    /// the same machinery that wraps a `typeof`/`void` in those positions.
    fn emit_await(&mut self, a: &AwaitExpression) {
        self.maybe_map(&a.cv);
        self.write_str("await");
        self.required_ws();
        self.emit_expression_inner(&a.argument, PREC_UNARY);
    }

    /// Emit an `ImportExpression` — a dynamic `import(specifier)`.
    ///
    /// This is the `import` keyword immediately followed by a *literal*
    /// parenthesised argument — syntactically a call-like primary. Unlike
    /// `await` (a word-shaped unary), no separator is needed after the keyword:
    /// the `(` follows `import` directly (`import(x)`, never `import (x)`). The
    /// `source` sits inside the literal parens, so it is emitted at
    /// `PREC_ASSIGNMENT` (the same level as a call argument): a looser *sequence*
    /// specifier wraps (`import((a,b))`), everything else prints bare
    /// (`import("m")`, `import(a.b)`, `import(f())`). The wrapping of the *whole*
    /// import in a tighter parent is a non-issue — as a `PREC_PRIMARY` node it
    /// is already atomic, so `import(x).then(f)` and `(await import(x))` compose
    /// without extra parens.
    fn emit_import_expression(&mut self, e: &ImportExpression) {
        self.maybe_map(&e.cv);
        self.write_str("import(");
        self.emit_expression_inner(&e.source, PREC_ASSIGNMENT);
        self.write_str(")");
    }

    /// `this` — a bare reserved-word keyword. It carries no operand, so the
    /// emit is simply the four characters `this` (after recording the source
    /// map anchor). No trailing separator is needed: as a `PREC_PRIMARY`
    /// leaf `this` is only ever followed by a member/call token (`this.x`,
    /// `this()`) or a punctuator, never by another word that would fuse.
    fn emit_this(&mut self, t: &ThisExpression) {
        self.maybe_map(&t.cv);
        self.write_str("this");
    }

    /// `super` — a bare reserved-word keyword, the sibling of `this`. Like
    /// `this` it carries no operand, so the emit is simply the five
    /// characters `super` (after recording the source-map anchor). As a
    /// `PREC_PRIMARY` leaf it is only ever followed by a member/call token
    /// (`super.x`, `super()`, `super[k]`) or a punctuator, never by another
    /// word that would fuse, so no trailing separator is needed.
    fn emit_super(&mut self, s: &Super) {
        self.maybe_map(&s.cv);
        self.write_str("super");
    }

    /// `new.target` — the meta-property, a leaf like `this` / `super`. It is
    /// spelled with two tokens plus a dot, so the emit is the literal ten
    /// characters `new.target` (after recording the source-map anchor). As a
    /// `PREC_PRIMARY` leaf it is only ever followed by a member/call token or a
    /// punctuator, never by another word that would fuse; the internal `.` is
    /// part of the spelling, not a member access, so no operand is walked.
    fn emit_new_target(&mut self, n: &NewTarget) {
        self.maybe_map(&n.cv);
        self.write_str("new.target");
    }

    /// `import.meta` — the module meta-property, a leaf like `new.target`. It is
    /// spelled with three tokens plus a dot, so the emit is the literal eleven
    /// characters `import.meta` (after recording the source-map anchor). As a
    /// `PREC_PRIMARY` leaf it is only ever followed by a member/call token or a
    /// punctuator, never by another word that would fuse; the internal `.` is
    /// part of the spelling, not a member access, so no operand is walked.
    fn emit_import_meta(&mut self, n: &ImportMeta) {
        self.maybe_map(&n.cv);
        self.write_str("import.meta");
    }

    fn emit_member(&mut self, m: &MemberExpression) {
        self.maybe_map(&m.cv);
        // The object must bind at least as tightly as member access, or the
        // parens that make it a unit are REQUIRED. Emitting it via
        // `emit_expression` (parent precedence 0) dropped them — `(a||b).c`
        // became `a||b.c` (i.e. `a||(b.c)`), a miscompile; likewise `(a+b).c`,
        // `(a=b).c`, `(a?b:c).d`, `(-a).b`. Member/call are `PREC_PRIMARY`, so
        // emitting the object at `PREC_PRIMARY` keeps `a.b.c` / `f().x`
        // paren-free while wrapping anything lower (binary, logical, unary,
        // conditional, assignment, sequence).
        self.emit_expression_inner(&m.object, PREC_PRIMARY);
        if m.computed {
            self.write_str("[");
            self.emit_expression(&m.property);
            self.write_str("]");
        } else {
            self.write_str(".");
            self.emit_expression(&m.property);
        }
    }

    /// `obj?.prop` / `obj?.[prop]` — an optional member access. Identical to
    /// [`Self::emit_member`] except the access operator is spelled `?.`
    /// (`?.` before a dot name, `?.[` before a computed key). The object binds
    /// at `PREC_PRIMARY` for the same reason as a plain member: a looser object
    /// (`(a||b)?.c`) must keep its parens.
    fn emit_optional_member(&mut self, m: &OptionalMemberExpression) {
        self.maybe_map(&m.cv);
        self.emit_expression_inner(&m.object, PREC_PRIMARY);
        if m.computed {
            self.write_str("?.[");
            self.emit_expression(&m.property);
            self.write_str("]");
        } else {
            self.write_str("?.");
            self.emit_expression(&m.property);
        }
    }

    /// `callee?.(args)` — an optional call. Identical to [`Self::emit_call`]
    /// except the call operator is spelled `?.(`. The callee binds at
    /// `PREC_PRIMARY`; each argument is emitted at `PREC_ASSIGNMENT` so a
    /// looser *sequence* argument wraps (`f?.((a,b))`).
    fn emit_optional_call(&mut self, c: &OptionalCallExpression) {
        self.maybe_map(&c.cv);
        self.emit_expression_inner(&c.callee, PREC_PRIMARY);
        self.write_str("?.(");
        for (i, a) in c.arguments.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            }
            self.emit_expression_inner(a, PREC_ASSIGNMENT);
        }
        self.write_str(")");
    }

    /// A [`ChainExpression`] is the transparent chain-boundary wrapper: it has
    /// no syntax of its own, so the printer simply descends into its inner
    /// expression. The inner spine is always a member/call/optional node at
    /// `PREC_PRIMARY`, so no parenthesisation is required here.
    fn emit_chain(&mut self, c: &ChainExpression) {
        self.maybe_map(&c.cv);
        self.emit_expression(&c.expression);
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
                // Assignment-position element — a sequence wraps
                // (`[(a,b),c]`, never the three-element `[a,b,c]`). See
                // `emit_call`.
                Some(e) => self.emit_expression_inner(e, PREC_ASSIGNMENT),
                None => {
                    // Elision. Empty position between commas.
                }
            }
        }
        // A TRAILING hole needs an extra comma. The loop writes one separating
        // comma *between* elements, so `[Some(1), None]` would print as `[1,]` —
        // but `[1,]` has length 1, whereas the source `[1,,]` has length 2 (a
        // trailing hole). Emitting one more comma when the last element is a hole
        // restores the count: `[1,,]`, `[,,]` (from `[None, None]`), etc. A
        // trailing *element* (e.g. `[1,2]`) is unaffected.
        if matches!(a.elements.last(), Some(None)) {
            self.write_str(",");
        }
        self.write_str("]");
    }

    fn emit_object(&mut self, o: &ObjectExpression) {
        self.maybe_map(&o.cv);
        self.write_str("{");
        for (i, member) in o.properties.iter().enumerate() {
            if i > 0 {
                self.write_str(",");
                self.pretty_ws();
            } else {
                self.pretty_ws();
            }
            match member {
                ObjectMember::Property(p) => self.emit_property(p),
                ObjectMember::Spread(s) => self.emit_object_spread(s),
            }
        }
        if !o.properties.is_empty() {
            self.pretty_ws();
        }
        self.write_str("}");
    }

    /// Emit an object-spread member `...expr` inside an object literal.
    ///
    /// Identical in shape to the call/array [`emit_spread`](Self::emit_spread):
    /// the three literal `.` characters then the `argument` at
    /// `PREC_ASSIGNMENT` with no interior space (`...o`, never `... o`). The
    /// assignment precedence is the crux — an object-literal member position is
    /// an `AssignmentExpression`, so everything at or above assignment strength
    /// prints bare (`...o`, `...o.p`, `...f()`, `...a?b:c`), while the one looser
    /// form, a **sequence**, must wrap (`...(a,b)`): a bare `...a,b` would spread
    /// only `a` and leave `,b` as a second (empty-keyed, invalid) member slot.
    fn emit_object_spread(&mut self, s: &SpreadElement) {
        self.maybe_map(&s.cv);
        self.write_str("...");
        self.emit_expression_inner(&s.argument, PREC_ASSIGNMENT);
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
            PropertyKey::StringLiteral(s) => {
                // Quote-stripping minification, matching Closure's CodePrinter:
                // a string key whose DECODED value is a valid identifier name may
                // drop its quotes — `{"abc":1}` → `{abc:1}`. This is sound only
                // under two carve-outs, both required:
                //
                //   • Non-identifier values MUST stay quoted, or the output is
                //     invalid / a different key:
                //       "a-b" → {a-b:1}  (SyntaxError)
                //       "a b" → {a b:1}  (SyntaxError)
                //       "x\ty"→ {x\ty:1} (SyntaxError)
                //       "1"   → {1:1}    (a numeric key, not the string "1")
                //     `is_identifier_name` (ASCII-only) rejects every one of
                //     these, so they route to `emit_string` and keep their quotes.
                //
                //   • `"__proto__"` MUST stay quoted even though it IS a valid
                //     identifier: the bare form `{__proto__: v}` is the prototype
                //     setter (B.3.1), whereas the quoted `{"__proto__": v}` is an
                //     ordinary own property. Dropping the quotes there would
                //     change runtime semantics, so it is explicitly excluded.
                if is_identifier_name(&s.value) && s.value != "__proto__" {
                    self.maybe_map(&s.cv);
                    self.write_str(&s.value);
                } else {
                    self.emit_string(s);
                }
            }
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

/// True when `s` is a valid ECMAScript identifier *name* in the ASCII subset:
/// a leading `A–Z a–z _ $` followed by zero or more `A–Z a–z 0–9 _ $`. Used by
/// `emit_property_key` to decide whether a quoted object key may be emitted bare
/// (`{"abc":1}` → `{abc:1}`).
///
/// We deliberately stay ASCII-only: a Unicode identifier key is always sound to
/// keep as a quoted string literal, so excluding it only forgoes a size win, it
/// never breaks output. Reserved words ARE legal property names (`{if: 1}`), so
/// they are intentionally NOT excluded here — the one name that needs special
/// handling, `__proto__`, is excluded at the call site because its bare form has
/// different semantics. (This mirrors the identically-named helper in
/// `closure-pass-constant-fold`; the two crates do not share a utility module.)
///
///   "abc"  → true      "a-b" → false (`-`)        "1ab" → false (digit lead)
///   "_$x"  → true      "a b" → false (space)       ""    → false (empty)
///   "if"   → true      "x\ty"→ false (`\`,`t` ok but `\` is not ident char)
fn is_identifier_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

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
        // Negative zero must keep its sign: `-0` is observably distinct from
        // `0` in JS (`1 / -0 === -Infinity`, `Object.is(-0, 0) === false`), so
        // dropping the sign is a miscompile. Rust's `-0.0 == 0.0` is `true`, so
        // we cannot rely on the equality alone — `is_sign_negative()` is what
        // distinguishes the two zeros. `-0` is also the minimal correct form.
        return if n.is_sign_negative() {
            "-0".to_string()
        } else {
            "0".to_string()
        };
    }
    // Integral values get the `{}`-of-`i64` spelling (no trailing `.0`), but
    // ONLY while they fit in an `i64`. `n as i64` is a SATURATING cast in Rust:
    // any `f64` at or above `i64::MAX` clamps to `9223372036854775807` and any
    // at or below `i64::MIN` clamps to `-9223372036854775808`. So an integral
    // literal like `12345678901234567890` (≈1.2e19, well above 2^63) would be
    // emitted as `9223372036854775807` — a different number entirely (a
    // miscompile). The i64 range boundary is 2^63 = 9223372036854775808.0
    // (note `i64::MAX` itself, 9223372036854775807, is not representable as an
    // `f64` — the nearest `f64` is exactly 2^63). We therefore only take the
    // i64 path when `|n| < 2^63`, where the cast is exact and lossless; larger
    // integral values fall through to `n.to_string()`, which prints the
    // shortest decimal that round-trips to the same `f64` (and the
    // exponential candidate below still gets a chance to be shorter).
    const I64_RANGE: f64 = 9_223_372_036_854_775_808.0; // 2^63
    let decimal = if n.fract() == 0.0 && n.abs() < I64_RANGE {
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

/// The comma operator — the loosest expression there is (below assignment). A
/// sequence sub-operand wraps under any parent that emits its child above the
/// statement level. `0` doubles as the "wrap nothing" sentinel used at
/// statement position and inside a computed-member key, which is exactly where
/// a bare sequence is legal.
const PREC_SEQUENCE: u8 = 0;
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
/// Does a `new` target need to be wrapped in parens?
///
/// The `new` operator's callee is a `MemberExpression` per the grammar, which
/// **cannot** itself be a call. If the callee's member spine bottoms out in a
/// [`Expression::CallExpression`], the `(args)` that `emit_new` appends would
/// otherwise bind to that inner call:
///
/// ```text
///   new f()()      reparses as   (new f())()        WRONG — wrap → new (f())()
///   new a.b().c()  reparses as   (new a.b()).c()    WRONG — wrap → new (a.b().c)()
///   new a.b.c()                  new (a.b.c)()      OK   — pure member chain
///   new X()                      new X()            OK   — plain identifier
/// ```
///
/// We walk only the **member-object spine** (`MemberExpression::object`): a
/// call reachable there is a call the appended `(args)` could attach to. Calls
/// nested inside an argument list or a computed-member key (`new a[f()].g()`
/// where `f()` is a key) are irrelevant — they are already closed off by their
/// own brackets — so we do not descend into those.
fn new_callee_needs_parens(callee: &Expression) -> bool {
    match callee {
        Expression::CallExpression(_) => true,
        Expression::MemberExpression(m) => new_callee_needs_parens(&m.object),
        _ => false,
    }
}

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
        // A regex literal `/…/g` is an atomic primary — it never needs
        // wrapping and no operand context forces a paren around it.
        | Expression::RegExpLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::ArrayExpression(_)
        | Expression::ObjectExpression(_)
        | Expression::CallExpression(_)
        | Expression::MemberExpression(_)
        // Optional-chain links (`a?.b`, `a?.[k]`, `a?.()`) bind exactly like
        // their non-optional member/call siblings — left-associative primaries.
        // The `ChainExpression` wrapper is transparent (it prints only its
        // inner expression), so it inherits the primary strength of the
        // member/call it wraps: `(a?.b).c` never needs extra parens.
        | Expression::OptionalMemberExpression(_)
        | Expression::OptionalCallExpression(_)
        | Expression::ChainExpression(_)
        // `this` is a reserved-word primary — a bare keyword that binds at the
        // tightest level, like an identifier. It never needs wrapping in any
        // parent (`this.x`, `this()`, `f(this)` are all valid bare), and no
        // operand context ever forces a paren around it.
        | Expression::ThisExpression(_)
        // `super` is a reserved-word primary, the sibling of `this`: a bare
        // keyword that binds at the tightest level. It is only ever the object
        // of a member access or a call callee (`super.m()`, `super[k]`,
        // `super()`) — all of which compose paren-free at primary strength.
        | Expression::Super(_)
        // `new.target` is a meta-property primary — an atomic two-token
        // spelling that binds at the tightest level, like `this`. It never
        // needs wrapping and never forces a paren around an operand (it has
        // none). The internal `.` is part of the spelling, not member access.
        | Expression::NewTarget(_)
        // `import.meta` is the module meta-property, the sibling of
        // `new.target`: an atomic three-token spelling that binds at the
        // tightest level. Same primary treatment — never wrapped, never forces
        // a paren; the internal `.meta` is part of the spelling, not access.
        | Expression::ImportMeta(_)
        // A dynamic `import(x)` is the `import` keyword plus a *literal*
        // parenthesised argument — a call-like primary. The parens make it
        // atomic from the outside, so it binds at the tightest level like a
        // `CallExpression`: `import(x).then(f)` composes without extra parens.
        | Expression::ImportExpression(_) => PREC_PRIMARY,

        Expression::UnaryExpression(_) => PREC_UNARY,
        // Update (`++x` / `x++`) binds a hair tighter than the pure unary
        // operators in the grammar, but tagging it at `PREC_UNARY` is the
        // safe conservative choice: it is loose enough that an
        // exponentiation base wraps it (`(x++)**2` — a bare `x++**2` is a
        // syntax error) and tight enough that a `!`/`typeof` parent does not
        // over-wrap it (`!x++`, `typeof x++` print bare, which is correct).
        Expression::UpdateExpression(_) => PREC_UNARY,
        // `emit_new` ALWAYS prints the argument parens — a no-argument `new X`
        // is emitted canonically as `new X()`. In that *argumented* spelling a
        // `new` is a `MemberExpression` in the grammar and binds at member/call
        // strength, so it tags at `PREC_PRIMARY` like a call: `new X().y` needs
        // no extra parens (it already means `(new X()).y`), and as a call
        // callee `new X().y()` stays paren-free. (Were we ever to drop the
        // empty parens — a future minification — the no-arg form would need the
        // looser bare-`NewExpression` precedence; we don't, so one tag suffices.)
        Expression::NewExpression(_) => PREC_PRIMARY,
        // The comma operator binds looser than every other expression — a
        // sequence sub-operand must be wrapped in almost every context.
        Expression::SequenceExpression(_) => PREC_SEQUENCE,
        // A function expression is primary-*ish*, but two contexts
        // mis-parse a bare one: as a call callee (`function(){}()` is a
        // syntax error) and as a member object (`function(){}.x`). Tag
        // it below PREC_PRIMARY (reusing PREC_UNARY, the same trick the
        // boolean/undefined cases use) so a call/member parent wraps it
        // into `(function(){})()` / `(function(){}).x`. Operator and
        // assignment parents bind looser than PREC_UNARY, so
        // `x=function(){}` and `function(){}+1` stay unwrapped (valid).
        // The remaining mis-parse — a function expression at the *start*
        // of an expression statement — is wrapped by
        // `emit_expression_statement`, not here.
        Expression::FunctionExpression(_) => PREC_UNARY,
        // A class expression behaves exactly like a function expression for
        // parenthesisation: it must wrap as a member object (`(class{}).x`) or
        // a call callee (`(class{})()`) — both mis-parse bare — and at the
        // start of an expression statement (a leading `class` parses as a class
        // *declaration*, wrapped by `emit_expression_statement`). Tagging it at
        // `PREC_UNARY` (below the `PREC_PRIMARY` at which member/call emit their
        // base) gives that wrapping, while looser assignment/binary parents
        // leave it bare (`x=class{}`, `class{}+1` are valid).
        Expression::ClassExpression(_) => PREC_UNARY,
        // An arrow function is an `AssignmentExpression` in the grammar —
        // the loosest-binding expression there is. Tagging it at
        // `PREC_ASSIGNMENT` makes a call/member parent wrap it into
        // `(() => {})()` / `(() => {}).x` (both mis-parse otherwise), while
        // an assignment RHS or a conditional branch — which also emit at
        // `PREC_ASSIGNMENT` — leave it unwrapped (`x=()=>y`, `c?()=>a:()=>b`
        // are all valid). Unlike a function expression it needs no
        // statement-start wrap, so `emit_expression_statement` ignores it.
        Expression::ArrowFunctionExpression(_) => PREC_ASSIGNMENT,
        // A template literal is a *primary* expression — a leaf token run
        // delimited by backticks. It never needs wrapping from any parent
        // (`` `x`.length ``, `` f`x` `` as a member/call base are all valid),
        // so it tags at `PREC_PRIMARY` like the array/object literals.
        Expression::TemplateLiteral(_) => PREC_PRIMARY,
        // A tagged template `` tag`x` `` binds at member/call strength — it is
        // left-associative like a call and can be a member/call base
        // (`` a`x`.length ``, `` a`x`() ``), so it tags at `PREC_PRIMARY`.
        Expression::TaggedTemplateExpression(_) => PREC_PRIMARY,
        // A spread `...arg` is not a free-standing operand — it only appears in
        // the assignment-position argument/element lists (`f(...a)`, `[...a]`),
        // which `emit_call` / `emit_new` / `emit_array` all print at
        // `PREC_ASSIGNMENT`. Tagging the spread there keeps it unwrapped in
        // those slots (`...a` never becomes the miscompile `(...a)`). It is
        // never emitted as a sub-operand of another operator (that would be
        // invalid JS), so no other context can observe this precedence.
        Expression::SpreadElement(_) => PREC_ASSIGNMENT,
        // `yield` / `yield* x` is an `AssignmentExpression` alternative in the
        // grammar — it binds looser than every operator except the comma. Tag it
        // at `PREC_ASSIGNMENT` so a tighter parent wraps it (`(yield x)+1`,
        // `f((yield x))` when the call slot is not itself assignment-position,
        // `(yield x).y`), while an assignment RHS or conditional branch — both
        // emitted at `PREC_ASSIGNMENT` — leave it bare (`x=yield v`,
        // `c?yield a:yield b`). This mirrors the arrow / assignment tags exactly.
        Expression::YieldExpression(_) => PREC_ASSIGNMENT,
        // `await x` is a unary operator in the grammar (`await UnaryExpression`),
        // binding exactly like the word unaries `typeof` / `void` / `delete`.
        // Tag it at `PREC_UNARY` so a member/call/new parent wraps the whole
        // await (`(await p).x`, `(await f)()`) while a binary/assignment parent
        // leaves it bare (`await a+b` = `(await a)+b`, `x=await p`).
        Expression::AwaitExpression(_) => PREC_UNARY,
        // `true`/`false` are emitted as `!0`/`!1` (see `emit_boolean`), which
        // are UnaryExpressions — precedence `PREC_UNARY`, NOT primary. Tagging
        // them here is what makes `emit_expression_inner` parenthesise them in
        // member/call/new parents (`(!0).x`, `(!0)()`), exactly as it does for
        // the `void 0` UndefinedLiteral below. Without this they would emit as
        // `!0.x` (parsed `!(0.x)`) — a miscompile.
        Expression::BooleanLiteral(_) => PREC_UNARY,
        // `void 0` is a UnaryExpression in disguise (CLOC12.16):
        // its precedence is unary, not primary, so that contexts
        // like `(void 0).x` and `(void 0)()` insert the necessary
        // parens automatically. Without this, the emit would
        // produce `void 0.x` which JS parses as `void (0.x)` — a
        // different expression.
        Expression::UndefinedLiteral(_) => PREC_UNARY,
        Expression::BinaryExpression(b) => binary_prec(b.operator),
        Expression::LogicalExpression(l) => logical_prec(l.operator),
        Expression::ConditionalExpression(_) => PREC_CONDITIONAL,
        Expression::AssignmentExpression(_) => PREC_ASSIGNMENT,
    }
}

/// The single character a *sign* prefix operator prints, or `None` for
/// every other unary operator. Only `-` and `+` can fuse with a same-sign
/// leading character in the argument to form the `--` / `++` token.
fn sign_op_char(op: UnaryOperator) -> Option<char> {
    match op {
        UnaryOperator::Negate => Some('-'),
        UnaryOperator::Plus => Some('+'),
        _ => None,
    }
}

/// The `++` / `--` operator as printed text.
fn update_op_str(op: UpdateOperator) -> &'static str {
    match op {
        UpdateOperator::Increment => "++",
        UpdateOperator::Decrement => "--",
    }
}

/// The leading sign character of an update operator (`+` for `++`, `-` for
/// `--`) — the character a *prefix* update prints first, and thus the one that
/// can fuse with a preceding sign operator.
fn update_op_lead_char(op: UpdateOperator) -> char {
    match op {
        UpdateOperator::Increment => '+',
        UpdateOperator::Decrement => '-',
    }
}

/// Would the unary argument `e`, emitted at `PREC_UNARY`, begin with the
/// character `sign` (`-` or `+`)? Used by [`emit_unary`] to decide whether
/// a separating space is required to avoid the `--` / `++` token fusion.
///
/// The argument is emitted at unary precedence, so anything that binds
/// *looser* than unary is parenthesised and therefore begins with `(`
/// (no fusion possible). The only operands that print a leading sign
/// without parens are:
///   * a nested unary with the same sign — `-(-a)` → inner prints `-a`;
///   * a negative numeric literal — `format_js_number` prints the
///     leading `-` (e.g. a constant-folded `-5`).
/// A `+` literal never prints a leading `+`, and a `BigIntLiteral`'s value
/// is always non-negative (the `-` of `-5n` is a `UnaryExpression`), so
/// only the nested-unary case matters for `+` and bigints cannot fuse.
fn arg_starts_with_sign(e: &Expression, sign: char) -> bool {
    match e {
        Expression::UnaryExpression(u) => sign_op_char(u.operator) == Some(sign),
        Expression::NumericLiteral(n) => sign == '-' && n.value.is_sign_negative(),
        // A *prefix* update prints its operator first, so `++x` begins with `+`
        // and `--x` begins with `-` — either can fuse with a preceding sign
        // operator (`a - --b` must print `a- --b`, never `a---b`, which JS
        // reparses as `(a--)-b`). A *postfix* update begins with its operand,
        // so recurse to see what that operand leads with.
        Expression::UpdateExpression(u) => {
            if u.prefix {
                update_op_lead_char(u.operator) == sign
            } else {
                arg_starts_with_sign(&u.argument, sign)
            }
        }
        _ => false,
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

/// Decide whether a block's last child statement ends in a `;`
/// that is a TRUE statement terminator (safe for gap-030's
/// drop-before-`}` rule to strip) versus a `;` that is
/// structurally a body slot (must NOT be stripped because the
/// grammar requires a Statement there, and `}` doesn't satisfy
/// that).
///
/// Truth table for the leaf statement types:
///
/// | Last child                  | Trailing `;` is... | Safe to pop? |
/// |-----------------------------|--------------------|--------------|
/// | ExpressionStatement         | terminator         | YES          |
/// | ReturnStatement             | terminator         | YES          |
/// | BreakStatement              | terminator         | YES          |
/// | ContinueStatement           | terminator         | YES          |
/// | ThrowStatement              | terminator         | YES          |
/// | EmptyStatement              | the statement      | NO (rare; preserve so empty bodies survive) |
/// | Declaration::*              | terminator-ish     | NO (FunctionDeclaration adds a part-B `;` we want to keep) |
/// | IfStatement                 | body's `;` maybe   | NO  |
/// | WhileStatement              | body's `;` maybe   | NO  |
/// | ForStatement                | body's `;` maybe   | NO  |
/// | LabeledStatement            | body's `;` maybe   | NO  |
/// | BlockStatement              | ends in `}`        | (no `;` to pop anyway) |
/// | SwitchStatement             | ends in `}`        | (no `;` to pop anyway) |
///
/// The pessimistic default for "compound" statements
/// (If/While/For/Labeled) reflects ECMAScript §13.7-§13.13:
/// each of those grammars takes a Statement in the body
/// position, and `;` (EmptyStatement) is a legal Statement,
/// while `}` is not. Stripping the `;` from
/// `function f(){for(;;);}` would produce
/// `function f(){for(;;)}` — a SyntaxError. Hence: only pop
/// after the listed leaf terminator types.
fn last_stmt_uses_terminator_semi(s: &Statement) -> bool {
    match s {
        Statement::Tagged(t) => matches!(
            t,
            TaggedStatement::ExpressionStatement(_)
                | TaggedStatement::ReturnStatement(_)
                | TaggedStatement::BreakStatement(_)
                | TaggedStatement::ContinueStatement(_)
                | TaggedStatement::ThrowStatement(_)
                // `do … while(x);` ends in a real terminator `;` that ASI can
                // supply before a closing `}`, so it is safe to pop. (Plain
                // `while(x)…` is NOT here: its trailing `;` is a body slot.)
                | TaggedStatement::DoWhileStatement(_)
                // `debugger;` ends in a real terminator `;`, poppable before a
                // closing `}` (ASI re-supplies it). The `;` is NOT a body slot.
                | TaggedStatement::DebuggerStatement(_)
        ),
        // Declarations are conservatively excluded. Both
        // VariableDeclaration and FunctionDeclaration end in
        // `;` in compact mode, but for the former the saving
        // is a single byte and for the latter the `;` is
        // gap-030's part-B addition that we explicitly want to
        // keep at top-level (popping it here would undo part
        // B's contribution).
        Statement::Declaration(_) => false,
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
            // U+2028 LINE SEPARATOR / U+2029 PARAGRAPH SEPARATOR: these are
            // line terminators in ECMAScript, so before ES2019 an UNESCAPED one
            // inside a string literal is a SyntaxError. They sit above 0x20, so
            // the control-char arm below does not catch them — escape explicitly.
            // (See `escape_ascii_only`, which already escapes them as non-ASCII.)
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
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
            // U+2028 LINE SEPARATOR / U+2029 PARAGRAPH SEPARATOR: line
            // terminators in ECMAScript, so an UNESCAPED one inside a string
            // literal is a SyntaxError before ES2019. They are above 0x20, so the
            // control-char arm below misses them — escape explicitly. (Mirrors
            // `escape_ascii_only`, which already escapes them as non-ASCII.)
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
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
    fn regexp(pattern: &str, flags: &str) -> Expression {
        Expression::RegExpLiteral(RegExpLiteral {
            cv: None,
            pattern: pattern.to_string(),
            flags: flags.to_string(),
        })
    }

    #[test]
    fn regexp_literal_reconstructs_slashes_and_flags() {
        // `/ab+c/gi` — pattern + flags round-trip verbatim.
        assert_eq!(emit_expr(regexp("ab+c", "gi")), "/ab+c/gi;");
        // No flags → a bare `/…/`.
        assert_eq!(emit_expr(regexp("a.b", "")), "/a.b/;");
        // Pattern-internal escapes/metachars are opaque text, emitted as-is.
        assert_eq!(emit_expr(regexp("\\d+\\/x", "u")), "/\\d+\\/x/u;");
    }

    // --- ClassExpression (CLOC12.173 PR1) ------------------------------

    /// An empty function body `{}` — the value of a no-op method.
    fn empty_block() -> BlockStatement {
        BlockStatement { cv: None, body: vec![] }
    }
    /// A method value `(){}` (no params, empty body).
    fn method_fn() -> FunctionExpression {
        FunctionExpression {
            cv: None,
            id: None,
            params: vec![],
            body: empty_block(),
            generator: false,
            is_async: false,
        }
    }
    /// A `MethodDefinition` with a plain identifier key `name` and empty body.
    fn method(name: &str, kind: MethodKind, is_static: bool) -> ClassMember {
        ClassMember::Method(MethodDefinition {
            cv: None,
            key: PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() }),
            kind,
            value: method_fn(),
            computed: false,
            is_static,
        })
    }
    /// Build a `ClassExpression` from optional name, optional superclass, and
    /// a member list.
    fn class_expr(
        id: Option<&str>,
        super_class: Option<Expression>,
        body: Vec<ClassMember>,
    ) -> Expression {
        Expression::ClassExpression(ClassExpression {
            cv: None,
            id: id.map(|n| Identifier { cv: None, name: n.to_string() }),
            super_class: super_class.map(Box::new),
            body,
        })
    }

    #[test]
    fn class_empty_wraps_at_statement_start() {
        // A leading `class` parses as a *declaration*, so a class *expression*
        // in statement position is wrapped — like `function`/`{`.
        assert_eq!(emit_expr(class_expr(None, None, vec![])), "(class{});");
    }

    #[test]
    fn class_named_and_heritage() {
        // `class C{}` and `class C extends B{}` (both statement-wrapped).
        assert_eq!(emit_expr(class_expr(Some("C"), None, vec![])), "(class C{});");
        assert_eq!(
            emit_expr(class_expr(Some("C"), Some(ident("B")), vec![])),
            "(class C extends B{});"
        );
    }

    #[test]
    fn class_extends_call_stays_bare_but_conditional_wraps() {
        // The `extends` operand is a LeftHandSideExpression: a call
        // `extends mixin(B)` stays bare, a conditional `extends (a?b:c)` wraps.
        let mixin = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("mixin")),
            arguments: vec![ident("B")],
        });
        assert_eq!(
            emit_expr(class_expr(None, Some(mixin), vec![])),
            "(class extends mixin(B){});"
        );
        let cond = Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("a")),
            consequent: Box::new(ident("b")),
            alternate: Box::new(ident("c")),
        });
        // The keyword→operand separator after `extends` is always emitted (it
        // is mandatory before an identifier operand like `mixin`); before a
        // parenthesised operand the space is redundant but harmless/valid. A
        // context-sensitive drop is a later minification refinement.
        assert_eq!(
            emit_expr(class_expr(None, Some(cond), vec![])),
            "(class extends (a?b:c){});"
        );
    }

    #[test]
    fn class_with_method() {
        assert_eq!(
            emit_expr(class_expr(None, None, vec![method("m", MethodKind::Method, false)])),
            "(class{m(){}});"
        );
    }

    #[test]
    fn class_static_and_accessors() {
        assert_eq!(
            emit_expr(class_expr(None, None, vec![method("m", MethodKind::Method, true)])),
            "(class{static m(){}});"
        );
        assert_eq!(
            emit_expr(class_expr(None, None, vec![method("x", MethodKind::Get, false)])),
            "(class{get x(){}});"
        );
        assert_eq!(
            emit_expr(class_expr(None, None, vec![method("x", MethodKind::Set, false)])),
            "(class{set x(){}});"
        );
    }

    #[test]
    fn class_computed_key_method() {
        // `[k](){}` — a computed key is bracketed via `PropertyKey::Expression`.
        let m = ClassMember::Method(MethodDefinition {
            cv: None,
            key: PropertyKey::Expression(Box::new(ident("k"))),
            kind: MethodKind::Method,
            value: method_fn(),
            computed: true,
            is_static: false,
        });
        assert_eq!(emit_expr(class_expr(None, None, vec![m])), "(class{[k](){}});");
    }

    #[test]
    fn class_as_call_argument_is_bare() {
        // As a call argument (emitted at PREC_ASSIGNMENT) a class expression
        // needs no wrap — only statement-start and member/callee contexts wrap.
        let c = class_expr(None, None, vec![]);
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("f")),
            arguments: vec![c],
        });
        assert_eq!(emit_expr(call), "f(class{});");
    }

    #[test]
    fn class_as_member_object_wraps() {
        // `(class{}).x` — a member parent (PREC_PRIMARY) wraps the class.
        let c = class_expr(None, None, vec![]);
        let m = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(c),
            property: Box::new(ident("x")),
            computed: false,
        });
        assert_eq!(emit_expr(m), "(class{}).x;");
    }
    fn boolean(v: bool) -> Expression {
        Expression::BooleanLiteral(BooleanLiteral { cv: None, value: v })
    }
    fn binary(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
        Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn member(object: Expression, prop: &str, computed: bool) -> Expression {
        Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(object),
            property: Box::new(ident(prop)),
            computed,
        })
    }

    #[test]
    fn exponentiation_base_and_right_precedence() {
        use BinaryOperator::*;
        // `(-a)**2` — a unary base of `**` MUST be parenthesised; `-a**2` is a
        // SyntaxError (the grammar base binds tighter than unary).
        let neg_a = unary(UnaryOperator::Negate, ident("a"));
        assert_eq!(emit_expr(binary(Exp, neg_a, num(2.0))), "(-a)**2;");
        // `**` is right-associative: a `**` on the RIGHT needs no parens.
        let b_pow_c = binary(Exp, ident("b"), ident("c"));
        assert_eq!(emit_expr(binary(Exp, ident("a"), b_pow_c)), "a**b**c;");
        // A `**` on the LEFT (left-grouped AST) DOES need parens.
        let a_pow_b = binary(Exp, ident("a"), ident("b"));
        assert_eq!(emit_expr(binary(Exp, a_pow_b, ident("c"))), "(a**b)**c;");
        // A unary RIGHT operand is legal without parens (`a**-b`).
        let neg_b = unary(UnaryOperator::Negate, ident("b"));
        assert_eq!(emit_expr(binary(Exp, ident("a"), neg_b)), "a**-b;");
    }

    #[test]
    fn member_and_call_object_below_member_precedence_is_parenthesised() {
        // `(a||b).c` — the object is a LogicalExpression (low precedence); the
        // parens making it a unit are REQUIRED. Emitting the object at parent
        // precedence 0 dropped them, yielding `a||b.c` (= `a||(b.c)`), a
        // miscompile. The object is now emitted at PREC_PRIMARY.
        let or = || {
            Expression::LogicalExpression(LogicalExpression {
                cv: None,
                operator: LogicalOperator::Or,
                left: Box::new(ident("a")),
                right: Box::new(ident("b")),
            })
        };
        assert_eq!(emit_expr(member(or(), "c", false)), "(a||b).c;");
        assert_eq!(emit_expr(member(or(), "c", true)), "(a||b)[c];"); // computed
        // A member object that is itself a member (PREC_PRIMARY) stays bare.
        assert_eq!(
            emit_expr(member(member(ident("a"), "b", false), "c", false)),
            "a.b.c;"
        );
        // Call callee has the same requirement.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(or()),
            arguments: Vec::new(),
        });
        assert_eq!(emit_expr(call), "(a||b)();");
    }
    fn unary(op: UnaryOperator, arg: Expression) -> Expression {
        Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: op,
            prefix: true,
            argument: Box::new(arg),
        })
    }

    // ---- optional chaining (`a?.b` / `a?.[k]` / `a?.()`) ----------------
    //
    // These exercise `emit_optional_member`, `emit_optional_call`, and the
    // transparent `emit_chain` wrapper (CLOC12.171 PR1). The printer is driven
    // from hand-constructed AST — the bridge that *builds* these nodes from the
    // grammar is PR2.

    fn opt_member(object: Expression, prop: &str, computed: bool) -> Expression {
        Expression::OptionalMemberExpression(OptionalMemberExpression {
            cv: None,
            object: Box::new(object),
            property: Box::new(ident(prop)),
            computed,
        })
    }
    fn opt_call(callee: Expression, arguments: Vec<Expression>) -> Expression {
        Expression::OptionalCallExpression(OptionalCallExpression {
            cv: None,
            callee: Box::new(callee),
            arguments,
        })
    }
    fn chain(inner: Expression) -> Expression {
        Expression::ChainExpression(ChainExpression {
            cv: None,
            expression: Box::new(inner),
        })
    }

    #[test]
    fn optional_member_dot_and_computed() {
        // `a?.b` — optional dot access spells the operator `?.`.
        assert_eq!(emit_expr(chain(opt_member(ident("a"), "b", false))), "a?.b;");
        // `a?.[b]` — optional computed access spells `?.[`…`]`.
        assert_eq!(emit_expr(chain(opt_member(ident("a"), "b", true))), "a?.[b];");
    }

    #[test]
    fn optional_call_prints_qmark_dot_parens() {
        // `a?.()` — optional call, no arguments.
        assert_eq!(emit_expr(chain(opt_call(ident("a"), vec![]))), "a?.();");
        // `a?.(b)` — one argument.
        assert_eq!(
            emit_expr(chain(opt_call(ident("a"), vec![ident("b")]))),
            "a?.(b);"
        );
    }

    #[test]
    fn optional_link_then_plain_link_only_marks_the_optional_one() {
        // `a?.b.c` — only the FIRST link is optional; the `.c` that follows is
        // an ordinary member whose object is the optional node. No `?.` leaks
        // onto `.c`.
        let inner = member(opt_member(ident("a"), "b", false), "c", false);
        assert_eq!(emit_expr(chain(inner)), "a?.b.c;");
        // `a?.b()` — a plain call on an optional member.
        let called = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(opt_member(ident("a"), "b", false)),
            arguments: vec![],
        });
        assert_eq!(emit_expr(chain(called)), "a?.b();");
    }

    #[test]
    fn chain_wrapper_is_transparent_and_object_precedence_is_kept() {
        // The `ChainExpression` wrapper adds no syntax: wrapping an optional
        // member prints exactly the same as the bare optional member.
        let bare = opt_member(ident("a"), "b", false);
        assert_eq!(emit_expr(chain(bare.clone())), emit_expr(bare));
        // The object still binds at PREC_PRIMARY: a looser object keeps parens.
        let or = Expression::LogicalExpression(LogicalExpression {
            cv: None,
            operator: LogicalOperator::Or,
            left: Box::new(ident("a")),
            right: Box::new(ident("b")),
        });
        assert_eq!(emit_expr(chain(opt_member(or, "c", false))), "(a||b)?.c;");
    }

    #[test]
    fn optional_call_sequence_argument_wraps() {
        // `a?.((b,c))` — a looser *sequence* argument must wrap, exactly as a
        // plain call argument does; a bare `a?.(b,c)` would be a two-argument
        // call, a different program.
        let seq = Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions: vec![ident("b"), ident("c")],
        });
        assert_eq!(emit_expr(chain(opt_call(ident("a"), vec![seq]))), "a?.((b,c));");
    }

    #[test]
    fn binary_operators_emit_tight_in_compact_mode() {
        use BinaryOperator::*;
        // Symbolic operators carry no space in compact mode.
        assert_eq!(emit_expr(binary(Add, ident("a"), ident("b"))), "a+b;");
        assert_eq!(emit_expr(binary(Mul, ident("a"), ident("b"))), "a*b;");
        assert_eq!(emit_expr(binary(StrictEq, ident("a"), ident("b"))), "a===b;");
        assert_eq!(emit_expr(binary(LeftShift, ident("a"), ident("b"))), "a<<b;");
        assert_eq!(emit_expr(binary(BitOr, ident("a"), ident("b"))), "a|b;");
        assert_eq!(emit_expr(binary(Exp, ident("a"), ident("b"))), "a**b;");
    }

    #[test]
    fn logical_operators_emit_tight_in_compact_mode() {
        use LogicalOperator::*;
        let logical = |op, l, r| {
            Expression::LogicalExpression(LogicalExpression {
                cv: None,
                operator: op,
                left: Box::new(l),
                right: Box::new(r),
            })
        };
        assert_eq!(emit_expr(logical(And, ident("a"), ident("b"))), "a&&b;");
        assert_eq!(emit_expr(logical(Or, ident("a"), ident("b"))), "a||b;");
    }

    #[test]
    fn word_operators_keep_their_spaces() {
        use BinaryOperator::*;
        // `in` / `instanceof` MUST stay spaced or they fuse into one identifier.
        assert_eq!(emit_expr(binary(In, ident("a"), ident("b"))), "a in b;");
        assert_eq!(
            emit_expr(binary(InstanceOf, ident("a"), ident("b"))),
            "a instanceof b;"
        );
    }

    #[test]
    fn additive_sign_hazard_keeps_a_minimal_space() {
        use BinaryOperator::*;
        // `a + (+b)` must NOT tighten to `a++b` (which parses as `a++ b`); the
        // right seam keeps one space. The left seam still tightens: `a+ +b`.
        assert_eq!(
            emit_expr(binary(Add, ident("a"), unary(UnaryOperator::Plus, ident("b")))),
            "a+ +b;"
        );
        // Likewise `a - (-b)` must not become `a--b`.
        assert_eq!(
            emit_expr(binary(Sub, ident("a"), unary(UnaryOperator::Negate, ident("b")))),
            "a- -b;"
        );
        // A negative numeric literal on the right is the same hazard.
        assert_eq!(emit_expr(binary(Sub, ident("a"), num(-1.0))), "a- -1;");
        // Mixed signs do NOT fuse, so no space is needed: `a+-b` = `a + (-b)`.
        assert_eq!(
            emit_expr(binary(Add, ident("a"), unary(UnaryOperator::Negate, ident("b")))),
            "a+-b;"
        );
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
        assert_eq!(out.code, "2+3;");
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

    #[test]
    fn line_and_paragraph_separators_are_escaped() {
        // U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR) are
        // ECMAScript line terminators. An UNESCAPED one inside a string literal
        // is a SyntaxError before ES2019, so the emitter must escape them even in
        // the default (non-`ascii_only`) mode. Double-quoted path:
        assert_eq!(emit_string_value("a\u{2028}b"), "\"a\\u2028b\";");
        assert_eq!(emit_string_value("a\u{2029}b"), "\"a\\u2029b\";");
        // Single-quoted path (value has more `"` than `'`, so single quotes are
        // chosen) escapes them too.
        assert_eq!(emit_string_value("\"\u{2028}"), "'\"\\u2028';");
        // `ascii_only` mode already escaped them (as non-ASCII) — confirm it
        // still produces the same ` ` form, not a `\u{...}` or raw byte.
        let s = Expression::StringLiteral(StringLiteral {
            cv: None,
            value: "a\u{2029}b".to_string(),
            raw: String::new(),
        });
        let out = emit_with(
            program().with_body(vec![stmt(s)]),
            EmitOptions {
                ascii_only: true,
                ..Default::default()
            },
        );
        assert_eq!(out.code, "\"a\\u2029b\";");
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
    fn negative_zero_keeps_its_sign() {
        // `-0` is observably distinct from `0` in JS (`1 / -0 === -Infinity`,
        // `Object.is(-0, 0) === false`), so the emitter must NOT drop the sign.
        // Rust's `-0.0 == 0.0` is `true`, which previously routed negative zero
        // through the `== 0.0` fast path and printed `"0"` — a miscompile.
        assert_eq!(format_js_number(-0.0), "-0");
        assert_eq!(format_js_number(0.0), "0");
        // Through the full emit path (synthetic NumericLiteral → emitted JS):
        assert_eq!(emit_number_value(-0.0), "-0");
        assert_eq!(emit_number_value(0.0), "0");
        // `0.0 * -1.0` is negative zero in IEEE-754 / JS — also preserved.
        assert_eq!(emit_number_value(0.0 * -1.0), "-0");
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
    fn number_large_integral_does_not_saturate_to_i64_bound() {
        // Regression: an integral f64 in [2^63, 1e21) used to be emitted via a
        // SATURATING `n as i64` cast, collapsing every such value to i64::MAX
        // ("9223372036854775807") — and every large negative one to i64::MIN.
        // That is a miscompile: the emitted literal denotes a different number.
        // The emitted text must round-trip to the SAME f64 as the source and
        // must never be the saturated constant.
        for &v in &[
            12_345_678_901_234_567_890.0_f64,  // ≈1.23e19, above 2^63
            18_446_744_073_709_551_615.0_f64,  // 2^64 − 1, ≈1.84e19
            1e20_f64,                          // integral, in [2^63, 1e21)
            -12_345_678_901_234_567_890.0_f64, // negative side (was i64::MIN)
        ] {
            let s = emit_number_value(v);
            assert_ne!(s, "9223372036854775807", "saturated to i64::MAX for {v}");
            assert_ne!(s, "-9223372036854775808", "saturated to i64::MIN for {v}");
            let back: f64 = s.parse().expect("emitted a parseable number literal");
            assert_eq!(back, v, "emitted {s:?} does not round-trip to {v}");
        }
    }

    #[test]
    fn number_values_within_i64_range_keep_exact_integer_spelling() {
        // Values that DO fit in i64 still take the lossless integer path
        // (the cast is exact there), then the shorter of decimal/exponential
        // wins as before.
        assert_eq!(emit_number_value(123.0), "123");
        assert_eq!(emit_number_value(100_000.0), "1E5");
        // The largest representable integral f64 strictly below 2^63 casts
        // exactly (no saturation) and round-trips.
        let near = 9_223_372_036_854_774_784.0_f64; // 2^63 − 1024, representable
        let s = emit_number_value(near);
        assert_eq!(s.parse::<f64>().unwrap(), near);
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
        assert_eq!(out.code, "\"foo\"+\"bar\";");
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
        // The `true` operand is itself minified to `!0` (see `emit_boolean`),
        // so `!true` emits as `!!0` — still demonstrating that the prefix `!`
        // abuts its operand with no space. `!!0 === !true === false`.
        assert_eq!(out.code, "!!0;");
    }

    /// `<name> <op> <rhs>` assignment expression helper.
    fn assign(name: &str, op: AssignmentOperator, rhs: Expression) -> Expression {
        Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: op,
            left: AssignmentTarget::Identifier(Identifier { cv: None, name: name.to_string() }),
            right: Box::new(rhs),
        })
    }

    fn conditional(test: Expression, cons: Expression, alt: Expression) -> Expression {
        Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(test),
            consequent: Box::new(cons),
            alternate: Box::new(alt),
        })
    }

    #[test]
    fn conditional_branches_do_not_parenthesize_assignments() {
        // `a ? b=1 : c=2` — both branches are AssignmentExpressions, which the
        // conditional grammar allows unparenthesised. Regression: the emitter
        // emitted the consequent/alternate at conditional precedence and wrapped
        // them, producing `a?(b=1):(c=2)`.
        let e = conditional(
            ident("a"),
            assign("b", AssignmentOperator::Eq, num(1.0)),
            assign("c", AssignmentOperator::Eq, num(2.0)),
        );
        let out = emit_default(program().with_body(vec![stmt(e)]));
        assert_eq!(out.code, "a?b=1:c=2;");
    }

    #[test]
    fn conditional_test_assignment_stays_parenthesized() {
        // `(a=1) ? b : c` — the TEST is a ShortCircuitExpression, which does
        // NOT include assignment, so the parens are REQUIRED: `a=1?b:c` parses
        // as `a=(1?b:c)`. This must survive (it is the soundness guard for the
        // branch de-parenthesisation above).
        let e = conditional(assign("a", AssignmentOperator::Eq, num(1.0)), ident("b"), ident("c"));
        let out = emit_default(program().with_body(vec![stmt(e)]));
        assert_eq!(out.code, "(a=1)?b:c;");
    }

    #[test]
    fn boolean_literals_minify_to_bang_zero_and_bang_one() {
        // Closure-style: `true` → `!0`, `false` → `!1` (value-exact, shorter).
        let t = emit_default(program().with_body(vec![stmt(boolean(true))]));
        assert_eq!(t.code, "!0;");
        let f = emit_default(program().with_body(vec![stmt(boolean(false))]));
        assert_eq!(f.code, "!1;");
    }

    #[test]
    fn boolean_as_member_object_is_parenthesized() {
        // `true.x` must NOT emit as `!0.x` (which reparses as `!(0.x)`); the
        // boolean is precedence `PREC_UNARY`, so the member-object emit wraps
        // it: `(!0).x`. This is the soundness guard for the `!0`/`!1` rewrite.
        let e = member(boolean(true), "x", false);
        let out = emit_default(program().with_body(vec![stmt(e)]));
        assert_eq!(out.code, "(!0).x;");
    }

    #[test]
    fn boolean_in_binary_needs_no_parens() {
        // Unary precedence (14) is higher than equality (`==`), so `true==1`
        // emits as `!0==1` with no parens — `!0==1` parses as `(!0)==1`.
        let e = binary(BinaryOperator::Eq, boolean(true), ident("a"));
        let out = emit_default(program().with_body(vec![stmt(e)]));
        assert_eq!(out.code, "!0==a;");
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

    // ---- prefix-unary precedence + token-adjacency ----------
    //
    // These pin the two miscompiles that the bridge fix unmasked: a unary
    // operator over a lower-precedence operand must parenthesise it, and
    // `-`/`+` over a same-sign operand must keep a separating space so the
    // pair never fuses into the `--`/`++` token.

    #[test]
    fn not_over_equality_parenthesises() {
        // `!(a == b)` must NOT print `!a == b` (which reparses as `(!a) == b`).
        let e = unary(UnaryOperator::Not, binary(BinaryOperator::Eq, ident("a"), ident("b")));
        assert_eq!(emit_expr(e), "!(a==b);");
    }

    #[test]
    fn negate_over_addition_parenthesises() {
        let e = unary(UnaryOperator::Negate, binary(BinaryOperator::Add, ident("a"), ident("b")));
        assert_eq!(emit_expr(e), "-(a+b);");
    }

    #[test]
    fn bitnot_over_bitor_parenthesises() {
        let e = unary(UnaryOperator::BitNot, binary(BinaryOperator::BitOr, ident("a"), ident("b")));
        assert_eq!(emit_expr(e), "~(a|b);");
    }

    #[test]
    fn unary_over_identifier_needs_no_parens() {
        assert_eq!(emit_expr(unary(UnaryOperator::Not, ident("a"))), "!a;");
    }

    #[test]
    fn double_not_does_not_parenthesise() {
        // `!!a` — equal precedence, no parens, and `!!` never fuses.
        let e = unary(UnaryOperator::Not, unary(UnaryOperator::Not, ident("a")));
        assert_eq!(emit_expr(e), "!!a;");
    }

    #[test]
    fn negate_over_negate_keeps_separating_space() {
        // `-(-a)` must print `- -a`, never `--a` (pre-decrement of `a`).
        let e = unary(UnaryOperator::Negate, unary(UnaryOperator::Negate, ident("a")));
        assert_eq!(emit_expr(e), "- -a;");
    }

    #[test]
    fn plus_over_plus_keeps_separating_space() {
        // `+(+a)` must print `+ +a`, never `++a` (pre-increment of `a`).
        let e = unary(UnaryOperator::Plus, unary(UnaryOperator::Plus, ident("a")));
        assert_eq!(emit_expr(e), "+ +a;");
    }

    #[test]
    fn negate_over_negative_literal_keeps_space() {
        // A folded `-(-5)` would print `5`, but an un-folded negative literal
        // under a `-` must still separate: `- -5`, never `--5`.
        let e = unary(UnaryOperator::Negate, num(-5.0));
        assert_eq!(emit_expr(e), "- -5;");
    }

    #[test]
    fn not_over_negate_needs_no_space() {
        // `!` and `-` don't fuse, so `!-a` needs neither parens nor a space.
        let e = unary(UnaryOperator::Not, unary(UnaryOperator::Negate, ident("a")));
        assert_eq!(emit_expr(e), "!-a;");
    }

    #[test]
    fn negate_inside_multiply_needs_no_parens() {
        // `-x * y`: unary binds tighter than `*`, so no parens around `-x`.
        let e = binary(
            BinaryOperator::Mul,
            unary(UnaryOperator::Negate, ident("x")),
            ident("y"),
        );
        assert_eq!(emit_expr(e), "-x*y;");
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
        // by required whitespace. After gap-030 the inner `;`
        // before `}` is dropped (ASI lets the brace terminate
        // the return statement) and a trailing `;` is added
        // after the function-declaration's closing `}` to
        // match upstream Closure v20240317.
        assert_eq!(out.code, "function f(x){return x};");
    }

    // ---- class declarations (CLOC12.174 PR1) ----------------
    //
    // The *statement* form of a class. Emitted at top level via
    // `ProgramItem::Declaration` (not `emit_expr`, which is for expression
    // position). Contrast with the ClassExpression tests above: a class
    // *expression* in statement position is parenthesised (`(class C{});`),
    // whereas the declaration is emitted bare with no wrap and — crucially —
    // NO trailing `;` (unlike `function f(){};`, gap-030 part B).

    /// Build a top-level `class <id>[ extends S]{members}` program and emit it
    /// in the default (minified) mode, returning the output code.
    fn emit_class_decl(id: &str, super_class: Option<Expression>, body: Vec<ClassMember>) -> String {
        let d = Declaration::ClassDeclaration(ClassDeclaration {
            cv: None,
            id: Identifier { cv: None, name: id.to_string() },
            super_class: super_class.map(Box::new),
            body,
        });
        emit_default(program().with_body(vec![ProgramItem::Declaration(d)])).code
    }

    #[test]
    fn class_declaration_empty_is_bare_and_unterminated() {
        // `class C {}` — bare (no wrapping paren, unlike the expression form's
        // `(class C{});`) and NO trailing `;` (unlike `function f(){};`).
        assert_eq!(emit_class_decl("C", None, vec![]), "class C{}");
    }

    #[test]
    fn class_declaration_heritage() {
        // `class C extends B {}` — the `extends` operand prints bare (an
        // identifier is a LeftHandSide, tighter than the class body).
        assert_eq!(
            emit_class_decl("C", Some(ident("B")), vec![]),
            "class C extends B{}"
        );
    }

    #[test]
    fn class_declaration_members_reuse_emit_class_member() {
        // A method, a static method, and get/set accessors — each printed by
        // the shared `emit_class_member` (the same helper the expression form
        // uses), back-to-back with no separators.
        assert_eq!(
            emit_class_decl("C", None, vec![method("m", MethodKind::Method, false)]),
            "class C{m(){}}"
        );
        assert_eq!(
            emit_class_decl("C", None, vec![method("m", MethodKind::Method, true)]),
            "class C{static m(){}}"
        );
        assert_eq!(
            emit_class_decl("C", None, vec![method("x", MethodKind::Get, false)]),
            "class C{get x(){}}"
        );
        assert_eq!(
            emit_class_decl("C", None, vec![method("x", MethodKind::Set, false)]),
            "class C{set x(){}}"
        );
    }

    #[test]
    fn class_declaration_constructor_and_computed_key() {
        // The constructor prints with no keyword prefix (its `kind` only
        // matters to the passes). A computed key `[k]` is bracketed.
        assert_eq!(
            emit_class_decl("C", None, vec![method("constructor", MethodKind::Constructor, false)]),
            "class C{constructor(){}}"
        );
        let computed = ClassMember::Method(MethodDefinition {
            cv: None,
            key: PropertyKey::Expression(Box::new(ident("k"))),
            kind: MethodKind::Method,
            value: method_fn(),
            computed: true,
            is_static: false,
        });
        assert_eq!(emit_class_decl("C", None, vec![computed]), "class C{[k](){}}");
    }

    #[test]
    fn class_declaration_full_shape_named_heritage_and_member() {
        // `class C extends B { m() {} }` — the whole shape in one assertion.
        assert_eq!(
            emit_class_decl("C", Some(ident("B")), vec![method("m", MethodKind::Method, false)]),
            "class C extends B{m(){}}"
        );
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

    // ---- gap-030: function-decl + block ASI policy ----------

    /// Block with multiple statements drops only the LAST `;`.
    /// `{a;b;}` → `{a;b}` in compact mode. The internal `;`
    /// between statements is preserved because there's no `}`
    /// adjacent to terminate the first statement via ASI.
    #[test]
    fn gap030_block_multi_stmts_drops_only_last_semi() {
        let mk = |name: &str| {
            Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: ident(name),
            })
        };
        let block = BlockStatement {
            cv: None,
            body: vec![mk("a"), mk("b")],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "g".to_string(),
            },
            params: vec![],
            body: block,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_default(prog);
        // Both intermediate `;` and the trailing function-decl
        // `;` after `}` follow gap-030's rules.
        assert_eq!(out.code, "function g(){a;b};");
    }

    /// Pretty mode preserves all `;`s for visual clarity. The
    /// gap-030 changes are compact-only.
    #[test]
    fn gap030_pretty_mode_unchanged() {
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::return_statement(ReturnStatement {
                cv: None,
                argument: Some(num(1.0)),
            })],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "h".to_string(),
            },
            params: vec![],
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
        // Inner `;` AND no trailing `;` after `}` — pretty mode
        // is untouched by gap-030. The visual delimiter
        // benefits of pretty-printing outrank byte-minimization.
        assert_eq!(out.code, "function h() {\n  return 1;\n}");
    }

    /// Regression test for the security-review-caught bug:
    /// when the block's last child is an `IfStatement` with an
    /// `EmptyStatement` body, the trailing `;` belongs to the
    /// EmptyStatement (which IS a body slot for the if), not
    /// to a statement terminator. Popping it would produce
    /// `function f(){if(x)};` which `}` cannot satisfy as a
    /// Statement at the body position — SyntaxError.
    ///
    /// The fix: gate the pop on
    /// `last_stmt_uses_terminator_semi()`, which returns
    /// `false` for IfStatement so the trailing `;` survives.
    #[test]
    fn gap030_does_not_pop_empty_body_of_if() {
        // Build: function f(){if(x);}
        let empty = Statement::Tagged(TaggedStatement::EmptyStatement(EmptyStatement {
            cv: None,
        }));
        let if_stmt = TaggedStatement::IfStatement(IfStatement {
            cv: None,
            test: ident("x"),
            consequent: Box::new(empty),
            alternate: None,
        });
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::Tagged(if_stmt)],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "f".to_string(),
            },
            params: vec![],
            body,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_default(prog);
        // The `;` inside `if(x);` MUST be preserved — it's the
        // IfStatement's body. Trailing `;` after `}` is the
        // gap-030 part-B addition for the function-declaration.
        assert_eq!(out.code, "function f(){if(x);};");
    }

    /// Same defense applied to `WhileStatement` with an
    /// `EmptyStatement` body — `while(x);` must not collapse to
    /// `while(x)`. Mirrors `gap030_does_not_pop_empty_body_of_if`.
    #[test]
    fn gap030_does_not_pop_empty_body_of_while() {
        let empty = Statement::Tagged(TaggedStatement::EmptyStatement(EmptyStatement {
            cv: None,
        }));
        let while_stmt = TaggedStatement::WhileStatement(WhileStatement {
            cv: None,
            test: ident("x"),
            body: Box::new(empty),
        });
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::Tagged(while_stmt)],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "g".to_string(),
            },
            params: vec![],
            body,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_default(prog);
        assert_eq!(out.code, "function g(){while(x);};");
    }

    /// Empty function body stays `{}` — no `;` to drop, no
    /// regression introduced by the pop-trailing-semi helper.
    /// Trailing `;` after `{}` still applies per gap-030 part B.
    #[test]
    fn gap030_empty_function_body_compact() {
        let body = BlockStatement {
            cv: None,
            body: vec![],
        };
        let f = FunctionDeclaration {
            cv: None,
            id: Identifier {
                cv: None,
                name: "noop".to_string(),
            },
            params: vec![],
            body,
            generator: false,
            is_async: false,
        };
        let prog = program().with_body(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(f),
        )]);
        let out = emit_default(prog);
        assert_eq!(out.code, "function noop(){};");
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

    /// Emit an ArrayExpression built from a hole-pattern (`'e'` = element `n`,
    /// `'_'` = hole) and return the code without the trailing `;`.
    fn emit_holes(pattern: &str) -> String {
        let elements = pattern
            .chars()
            .map(|c| if c == 'e' { Some(num(1.0)) } else { None })
            .collect();
        let a = Expression::ArrayExpression(ArrayExpression { cv: None, elements });
        emit_default(program().with_body(vec![stmt(a)]))
            .code
            .trim_end_matches(';')
            .to_string()
    }

    #[test]
    fn array_trailing_and_leading_holes_round_trip() {
        // A TRAILING hole needs an extra comma: `[Some(1), None]` must print as
        // `[1,,]` (length 2), NOT `[1,]` (length 1). Before the fix the emitter
        // wrote only the separating comma and silently shortened the array.
        assert_eq!(emit_holes("e_"), "[1,,]"); // trailing hole, length 2
        assert_eq!(emit_holes("__"), "[,,]"); // two holes, length 2
        assert_eq!(emit_holes("_"), "[,]"); // single hole, length 1
        assert_eq!(emit_holes("_e"), "[,1]"); // leading hole, length 2
        // Internal hole is unchanged by the trailing-hole fix (every element here
        // is the literal `1`): `[1,,1]`, length 3.
        assert_eq!(emit_holes("e_e"), "[1,,1]");
        // No trailing hole → no extra comma.
        assert_eq!(emit_holes("ee"), "[1,1]");
        assert_eq!(emit_holes("e"), "[1]");
        assert_eq!(emit_holes(""), "[]");
    }

    #[test]
    fn object_expression_at_statement_start_is_parenthesized() {
        // {a: 1, b: 2} as a top-level expression statement.
        let o = Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![
                ObjectMember::Property(Property {
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
                }),
                ObjectMember::Property(Property {
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
                }),
            ],
        });
        let prog = program().with_body(vec![stmt(o)]);
        let out = emit_default(prog);
        assert_eq!(out.code, "({a:1,b:2});");
    }

    // ---- object spread `{...o}` (CLOC12.170) ----
    //
    // A spread member prints `...` then its argument at `PREC_ASSIGNMENT` with
    // no interior space, exactly like a call/array spread. The object body is
    // wrapped in `(...)` here only because an object at statement-start needs it
    // (tested above) — the `...` printing is what these assert.

    /// Build a spread member `...arg`.
    fn spread_member(arg: Expression) -> ObjectMember {
        ObjectMember::Spread(SpreadElement { cv: None, argument: Box::new(arg) })
    }

    /// Build a plain `name: value` init member.
    fn init_member(name: &str, value: Expression) -> ObjectMember {
        ObjectMember::Property(Property {
            cv: None,
            kind: PropertyKind::Init,
            key: PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() }),
            value: Box::new(value),
            computed: false,
            shorthand: false,
            method: false,
        })
    }

    /// Emit an object literal (as a parenthesised statement) from its members.
    fn emit_object_members(members: Vec<ObjectMember>) -> String {
        let o = Expression::ObjectExpression(ObjectExpression { cv: None, properties: members });
        emit_default(program().with_body(vec![stmt(o)])).code
    }

    #[test]
    fn object_spread_sole_member_is_bare() {
        // `{...a}` — the spread argument prints bare.
        assert_eq!(emit_object_members(vec![spread_member(ident("a"))]), "({...a});");
    }

    #[test]
    fn object_spread_before_property() {
        // `{...a, b: 1}` — spread then a normal member, source order preserved.
        assert_eq!(
            emit_object_members(vec![spread_member(ident("a")), init_member("b", num(1.0))]),
            "({...a,b:1});"
        );
    }

    #[test]
    fn object_spread_after_property() {
        // `{a: 1, ...b}` — a normal member then a spread.
        assert_eq!(
            emit_object_members(vec![init_member("a", num(1.0)), spread_member(ident("b"))]),
            "({a:1,...b});"
        );
    }

    #[test]
    fn object_spread_call_argument_is_bare() {
        // `{...f()}` — a call binds tighter than assignment, so no wrap.
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("f")),
            arguments: vec![],
        });
        assert_eq!(emit_object_members(vec![spread_member(call)]), "({...f()});");
    }

    #[test]
    fn object_spread_sequence_argument_wraps() {
        // `{...(a, b)}` — a sequence is looser than the member comma, so it must
        // wrap; a bare `...a,b` would spread only `a` and leave `,b` as a second
        // (invalid) member slot.
        let seq = Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions: vec![ident("a"), ident("b")],
        });
        assert_eq!(emit_object_members(vec![spread_member(seq)]), "({...(a,b)});");
    }

    // ---- property-key quote stripping (emit_property_key) ----
    //
    // A `PropertyKey::StringLiteral` key drops its quotes ONLY when the decoded
    // `value` is a valid identifier name and is not `__proto__`. Everything else
    // stays quoted. These guard against the regression where every quoted object
    // key was emitted as a bare identifier (a miscompile for non-ident and
    // `__proto__` keys).

    /// Build the single-property object `{<string-key>: 1}` as an expression.
    fn obj_one_string_key(value: &str) -> Expression {
        Expression::ObjectExpression(ObjectExpression {
            cv: None,
            properties: vec![ObjectMember::Property(Property {
                cv: None,
                kind: PropertyKind::Init,
                key: PropertyKey::StringLiteral(StringLiteral {
                    cv: None,
                    value: value.to_string(),
                    // `raw` is unused by `emit_string` (it re-escapes from
                    // `value`), so a placeholder is fine here.
                    raw: String::new(),
                }),
                value: Box::new(num(1.0)),
                computed: false,
                shorthand: false,
                method: false,
            })],
        })
    }

    #[test]
    fn string_key_valid_identifier_drops_quotes() {
        // {"abc": 1} → {abc:1}  — the common minification.
        assert_eq!(emit_expr(obj_one_string_key("abc")), "({abc:1});");
        // Leading `_`/`$` and reserved words are valid identifier *names*.
        assert_eq!(emit_expr(obj_one_string_key("_$x")), "({_$x:1});");
        assert_eq!(emit_expr(obj_one_string_key("if")), "({if:1});");
    }

    #[test]
    fn string_key_non_identifier_stays_quoted() {
        // Hyphen, space, and leading digit are NOT identifier chars — emitting
        // any of these bare would be a SyntaxError or a different (numeric) key.
        assert_eq!(emit_expr(obj_one_string_key("a-b")), "({\"a-b\":1});");
        assert_eq!(emit_expr(obj_one_string_key("a b")), "({\"a b\":1});");
        assert_eq!(emit_expr(obj_one_string_key("123")), "({\"123\":1});");
        // A control char (tab) in the value re-escapes through `emit_string`.
        assert_eq!(emit_expr(obj_one_string_key("x\ty")), "({\"x\\ty\":1});");
    }

    #[test]
    fn string_key_proto_stays_quoted() {
        // `__proto__` IS a valid identifier name, but the bare form
        // `{__proto__: v}` is the prototype setter — a DIFFERENT object — so the
        // quoted own-property key must NOT be stripped.
        assert_eq!(
            emit_expr(obj_one_string_key("__proto__")),
            "({\"__proto__\":1});"
        );
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
        assert_eq!(out.code, "2+3;");
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

    // ---- SwitchStatement (gap-014, CLOC12.33) ----------------

    #[test]
    fn switch_empty_emits_braces() {
        // switch (x) {}
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![],
        });
        assert_eq!(emit_stmt(s), "switch(x){}");
    }

    #[test]
    fn switch_with_single_case_emits_case_test_colon_body() {
        // switch (x) { case 1: y; }
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![Statement::expression_statement(ExpressionStatement {
                    cv: None,
                    expression: ident("y"),
                })],
            }],
        });
        // `case 1:` then `y;` (the ExpressionStatement adds the `;`).
        assert_eq!(emit_stmt(s), "switch(x){case 1:y;}");
    }

    #[test]
    fn switch_with_default_emits_default_colon_body() {
        // switch (x) { default: y; }
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![SwitchCase {
                cv: None,
                test: None,
                consequent: vec![Statement::expression_statement(ExpressionStatement {
                    cv: None,
                    expression: ident("y"),
                })],
            }],
        });
        assert_eq!(emit_stmt(s), "switch(x){default:y;}");
    }

    #[test]
    fn switch_case_with_empty_consequent_emits_colon_only() {
        // switch (x) { case 1: }
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![],
            }],
        });
        assert_eq!(emit_stmt(s), "switch(x){case 1:}");
    }

    #[test]
    fn switch_with_two_cases_and_default_concatenates_in_order() {
        // switch (x) { case 1: a; case 2: b; default: c; }
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![
                SwitchCase {
                    cv: None,
                    test: Some(num(1.0)),
                    consequent: vec![Statement::expression_statement(ExpressionStatement {
                        cv: None,
                        expression: ident("a"),
                    })],
                },
                SwitchCase {
                    cv: None,
                    test: Some(num(2.0)),
                    consequent: vec![Statement::expression_statement(ExpressionStatement {
                        cv: None,
                        expression: ident("b"),
                    })],
                },
                SwitchCase {
                    cv: None,
                    test: None,
                    consequent: vec![Statement::expression_statement(ExpressionStatement {
                        cv: None,
                        expression: ident("c"),
                    })],
                },
            ],
        });
        assert_eq!(emit_stmt(s), "switch(x){case 1:a;case 2:b;default:c;}");
    }

    #[test]
    fn switch_with_break_in_consequent_emits_break_semicolon() {
        // switch (x) { case 1: break; }
        let s = Statement::switch_statement(SwitchStatement {
            cv: None,
            discriminant: ident("x"),
            cases: vec![SwitchCase {
                cv: None,
                test: Some(num(1.0)),
                consequent: vec![Statement::break_statement(BreakStatement {
                    cv: None,
                    label: None,
                })],
            }],
        });
        assert_eq!(emit_stmt(s), "switch(x){case 1:break;}");
    }

    // ---- BigIntLiteral (gap-021, CLOC12.15) -----------------

    fn emit_expr(e: Expression) -> String {
        emit_default(program().with_body(vec![stmt(e)])).code
    }

    /// A very deep left-nested operator chain — the shape the bridge builds for
    /// flat source like `1+1+…+1` (tens of thousands of terms) — must emit
    /// without overflowing the native stack. `emit_binary` recurses on `b.left`
    /// once per operator; on the caller's ordinary ~2 MiB stack this used to
    /// overflow (an uncatchable abort). Emission now runs on a large-stack
    /// worker (`EMIT_STACK_SIZE`), so arbitrarily deep valid ASTs emit fine —
    /// output is byte-identical to shallow emission, just with headroom.
    #[test]
    fn deeply_nested_binary_chain_emits_without_stack_overflow() {
        const N: usize = 20_000;
        let mut e = num(1.0);
        for _ in 0..N {
            e = Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(e),
                right: Box::new(num(1.0)),
            });
        }
        let prog = program().with_body(vec![stmt(e)]);
        // `emit` runs its recursion on the 64 MiB `EMIT_STACK_SIZE` worker, so
        // this depth emits fine even though the test itself runs on cargo's
        // ~2 MiB thread. Two caveats make this a *precise* regression test:
        //   • Without the worker, `emit_binary`'s per-operator recursion
        //     overflows here — so a regression re-breaks this test.
        //   • The 20 000-deep AST's own recursive `Drop` (walking the Box
        //     spine) would ALSO overflow this small thread — an orthogonal
        //     concern, out of scope here — so we `forget` it rather than let
        //     Drop mask the emit result. The process exits right after.
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
            .expect("emit should succeed")
            .code;
        // `1+1+…+1;` — N `+` operators joining N+1 ones, terminated by `;`.
        assert_eq!(out.matches('+').count(), N, "one `+` per operator");
        assert!(out.starts_with("1+1+1"), "left-to-right compact emit");
        assert!(out.ends_with(';'));
        std::mem::forget(prog);
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

    // ---- UndefinedLiteral (gap-001, CLOC12.16) ---------------

    #[test]
    fn undefined_literal_emits_void_zero() {
        let e = Expression::UndefinedLiteral(UndefinedLiteral { cv: None });
        assert_eq!(emit_expr(e), "void 0;");
    }

    #[test]
    fn undefined_literal_with_cv_emits_void_zero() {
        // Tracing on; the `void 0` text doesn't change but a CV
        // contribution should fire (covered by other tests of the
        // CV pathway — here we just pin the textual output).
        let e = Expression::UndefinedLiteral(UndefinedLiteral {
            cv: Some("u.42".to_string()),
        });
        assert_eq!(emit_expr(e), "void 0;");
    }

    // Note: a `(void 0).foo` integration test would pin the
    // PREC_UNARY wiring (security review nit), but the emitter's
    // `emit_member` currently writes `emit_expression(&object)`
    // — the precedence-free wrapper — so member-access doesn't
    // precedence-wrap its object today. That gap also affects
    // `(a + b).foo` etc., and fixing it cleanly belongs in a
    // separate PR that switches `emit_member` (and `emit_call`)
    // to use `emit_expression_inner(object, PREC_PRIMARY)`.
    // Tracked as a follow-up to CLOC12.10's paren-policy work.
    // The PREC_UNARY entry for UndefinedLiteral is still correct
    // (it'll start firing the moment the emit_member fix lands).

    // ---- try / catch / finally (CLOC19) -----------------------

    /// Build a single-expression-statement block `{ <ident>; }`.
    fn block_with(name: &str) -> BlockStatement {
        BlockStatement {
            cv: None,
            body: vec![Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: ident(name),
            })],
        }
    }

    fn emit_try_item(t: TryStatement) -> String {
        let item = ProgramItem::Statement(Statement::try_statement(t));
        emit_default(program().with_body(vec![item])).code
    }

    // ---- do / while (CLOC20) ----------------------------------

    fn emit_do_while_item(d: DoWhileStatement) -> String {
        let item = ProgramItem::Statement(Statement::do_while_statement(d));
        emit_default(program().with_body(vec![item])).code
    }

    #[test]
    fn do_while_block_body_emits_tight() {
        // do { a; } while (b)  — block body needs no separator after `do`
        // (`do{` lexes cleanly), and the loop ends in a real terminator `;`.
        let d = DoWhileStatement {
            cv: None,
            body: Box::new(Statement::block_statement(block_with("a"))),
            test: ident("b"),
        };
        assert_eq!(emit_do_while_item(d), "do{a}while(b);");
    }

    #[test]
    fn do_while_bare_body_inserts_separator() {
        // do a; while (b)  — a bare expression-statement body MUST be
        // separated from the `do` keyword (`doa` would mis-lex).
        let d = DoWhileStatement {
            cv: None,
            body: Box::new(Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: ident("a"),
            })),
            test: ident("b"),
        };
        assert_eq!(emit_do_while_item(d), "do a;while(b);");
    }

    #[test]
    fn do_while_pretty_mode_spaces() {
        let d = DoWhileStatement {
            cv: None,
            body: Box::new(Statement::block_statement(block_with("a"))),
            test: ident("b"),
        };
        let item = ProgramItem::Statement(Statement::do_while_statement(d));
        let out = emit_with(
            program().with_body(vec![item]),
            EmitOptions {
                pretty: true,
                ..EmitOptions::default()
            },
        );
        assert!(out.code.contains("do {"), "got:\n{}", out.code);
        assert!(out.code.contains("} while ("), "got:\n{}", out.code);
    }

    #[test]
    fn do_while_as_last_block_statement_pops_terminator_semi() {
        // Inside a block, the do-while's trailing `;` is redundant before the
        // closing `}` (ASI), so it is popped: `{do{a}while(b)}`.
        let d = DoWhileStatement {
            cv: None,
            body: Box::new(Statement::block_statement(block_with("a"))),
            test: ident("b"),
        };
        let outer = BlockStatement {
            cv: None,
            body: vec![Statement::do_while_statement(d)],
        };
        let item = ProgramItem::Statement(Statement::block_statement(outer));
        let code = emit_default(program().with_body(vec![item])).code;
        assert_eq!(code, "{do{a}while(b)}");
    }

    // ---- debugger (CLOC21) ------------------------------------

    #[test]
    fn debugger_statement_emits_keyword_and_semi() {
        let item = ProgramItem::Statement(Statement::debugger_statement(DebuggerStatement {
            cv: None,
        }));
        let code = emit_default(program().with_body(vec![item])).code;
        assert_eq!(code, "debugger;");
    }

    #[test]
    fn debugger_as_last_block_statement_pops_terminator_semi() {
        // Inside a block, the trailing `;` of `debugger;` is redundant before
        // the closing `}` (ASI), so it is popped: `{debugger}`.
        let outer = BlockStatement {
            cv: None,
            body: vec![Statement::debugger_statement(DebuggerStatement { cv: None })],
        };
        let item = ProgramItem::Statement(Statement::block_statement(outer));
        let code = emit_default(program().with_body(vec![item])).code;
        assert_eq!(code, "{debugger}");
    }

    #[test]
    fn debugger_followed_by_statement_keeps_semi() {
        // `debugger;` followed by another statement keeps its `;` (only the
        // last statement in a block pops it).
        let outer = BlockStatement {
            cv: None,
            body: vec![
                Statement::debugger_statement(DebuggerStatement { cv: None }),
                Statement::expression_statement(ExpressionStatement {
                    cv: None,
                    expression: ident("a"),
                }),
            ],
        };
        let item = ProgramItem::Statement(Statement::block_statement(outer));
        let code = emit_default(program().with_body(vec![item])).code;
        assert_eq!(code, "{debugger;a}");
    }

    // ---- for / in (CLOC22) ------------------------------------

    fn for_in_var_left(name: &str, kind: VarKind) -> ForInit {
        ForInit::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind,
            declarations: vec![VariableDeclarator {
                cv: None,
                id: coding_adventures_javascript_ast::BindingTarget::Identifier(Identifier {
                    cv: None,
                    name: name.to_string(),
                }),
                init: None,
            }],
        })
    }

    #[test]
    fn for_in_var_left_block_body_emits_with_spaced_in() {
        // `for (var k in obj) { a }` — `in` is spaced on both sides so `k in`
        // and `in obj` don't mis-lex.
        let f = ForInStatement {
            cv: None,
            left: for_in_var_left("k", VarKind::Var),
            right: ident("obj"),
            body: Box::new(Statement::block_statement(block_with("a"))),
        };
        let item = ProgramItem::Statement(Statement::for_in_statement(f));
        assert_eq!(
            emit_default(program().with_body(vec![item])).code,
            "for(var k in obj){a}"
        );
    }

    #[test]
    fn for_in_const_left_emits() {
        let f = ForInStatement {
            cv: None,
            left: for_in_var_left("k", VarKind::Const),
            right: ident("obj"),
            body: Box::new(Statement::block_statement(block_with("a"))),
        };
        let item = ProgramItem::Statement(Statement::for_in_statement(f));
        assert_eq!(
            emit_default(program().with_body(vec![item])).code,
            "for(const k in obj){a}"
        );
    }

    #[test]
    fn for_in_expression_left_bare_body_emits() {
        // `for (k in obj) a;` — existing-target left, bare-statement body.
        let f = ForInStatement {
            cv: None,
            left: ForInit::Expression(ident("k")),
            right: ident("obj"),
            body: Box::new(Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: ident("a"),
            })),
        };
        let item = ProgramItem::Statement(Statement::for_in_statement(f));
        assert_eq!(
            emit_default(program().with_body(vec![item])).code,
            "for(k in obj)a;"
        );
    }

    // ---- for / of (CLOC23) ------------------------------------

    #[test]
    fn for_of_var_left_block_body_emits_with_spaced_of() {
        // `for (var v of it) { a }` — `of` spaced on both sides.
        let f = ForOfStatement {
            cv: None,
            left: for_in_var_left("v", VarKind::Var),
            right: ident("it"),
            body: Box::new(Statement::block_statement(block_with("a"))),
        };
        let item = ProgramItem::Statement(Statement::for_of_statement(f));
        assert_eq!(
            emit_default(program().with_body(vec![item])).code,
            "for(var v of it){a}"
        );
    }

    #[test]
    fn for_of_expression_left_bare_body_emits() {
        // `for (v of it) a;` — existing-target left, bare-statement body.
        let f = ForOfStatement {
            cv: None,
            left: ForInit::Expression(ident("v")),
            right: ident("it"),
            body: Box::new(Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: ident("a"),
            })),
        };
        let item = ProgramItem::Statement(Statement::for_of_statement(f));
        assert_eq!(
            emit_default(program().with_body(vec![item])).code,
            "for(v of it)a;"
        );
    }

    #[test]
    fn try_catch_finally_emits_with_clean_token_boundaries() {
        // The whole point of the emitter's "no required_ws" claim: every
        // boundary (`}catch`, `)`/`{`, `}finally`) lexes cleanly with no
        // separator. The last statement in a block drops its `;` (the
        // emitter's minified ASI), so the tight form is exactly this.
        let t = TryStatement {
            cv: None,
            block: block_with("a"),
            handler: Some(CatchClause {
                cv: None,
                param: Some(Identifier {
                    cv: None,
                    name: "e".to_string(),
                }),
                body: block_with("b"),
            }),
            finalizer: Some(block_with("c")),
        };
        assert_eq!(emit_try_item(t), "try{a}catch(e){b}finally{c}");
    }

    #[test]
    fn try_with_optional_catch_binding_emits_no_parens() {
        // ES2019 `catch { … }` — handler with no param.
        let t = TryStatement {
            cv: None,
            block: block_with("a"),
            handler: Some(CatchClause {
                cv: None,
                param: None,
                body: block_with("b"),
            }),
            finalizer: None,
        };
        assert_eq!(emit_try_item(t), "try{a}catch{b}");
    }

    #[test]
    fn try_finally_without_catch_emits() {
        // A `try … finally` with no handler at all.
        let t = TryStatement {
            cv: None,
            block: block_with("a"),
            handler: None,
            finalizer: Some(block_with("c")),
        };
        assert_eq!(emit_try_item(t), "try{a}finally{c}");
    }

    #[test]
    fn try_catch_finally_pretty_mode_spaces_keywords() {
        let t = TryStatement {
            cv: None,
            block: block_with("a"),
            handler: Some(CatchClause {
                cv: None,
                param: Some(Identifier {
                    cv: None,
                    name: "e".to_string(),
                }),
                body: block_with("b"),
            }),
            finalizer: Some(block_with("c")),
        };
        let item = ProgramItem::Statement(Statement::try_statement(t));
        let out = emit_with(
            program().with_body(vec![item]),
            EmitOptions {
                pretty: true,
                ..EmitOptions::default()
            },
        );
        // Pretty mode inserts a space after each keyword and around the
        // blocks; the catch param keeps its `(e)` form.
        assert!(out.code.contains("try {"), "got:\n{}", out.code);
        assert!(out.code.contains("catch (e) {"), "got:\n{}", out.code);
        assert!(out.code.contains("finally {"), "got:\n{}", out.code);
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

    // ---- FunctionExpression (CLOC12.149) ----------------------

    /// Build a `FunctionExpression` value: `id` name (or anonymous),
    /// simple identifier params, and a body of statements.
    fn fexpr(id: Option<&str>, params: &[&str], body: Vec<Statement>) -> Expression {
        Expression::FunctionExpression(FunctionExpression {
            cv: None,
            id: id.map(|n| Identifier { cv: None, name: n.to_string() }),
            params: params
                .iter()
                .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
                .collect(),
            body: BlockStatement { cv: None, body },
            generator: false,
            is_async: false,
        })
    }

    /// An anonymous function expression at the START of an expression
    /// statement must be parenthesised — a leading `function` otherwise
    /// parses as a declaration.
    #[test]
    fn function_expression_at_statement_start_is_parenthesised() {
        assert_eq!(emit_expr(fexpr(None, &[], vec![])), "(function(){});");
    }

    /// An IIFE: the function-expression callee needs parens because
    /// `function(){}()` is a syntax error.
    #[test]
    fn function_expression_iife_wraps_callee() {
        let iife = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(fexpr(None, &[], vec![])),
            arguments: vec![],
        });
        assert_eq!(emit_expr(iife), "(function(){})();");
    }

    /// As a call ARGUMENT the function expression needs no parens (the
    /// argument context is the loosest), and crucially no stray `;` is
    /// appended after its body `}` — that trailing-`;` normalisation is
    /// a function *declaration* rule only.
    #[test]
    fn function_expression_as_argument_has_no_parens_or_trailing_semi() {
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("g")),
            arguments: vec![fexpr(None, &[], vec![])],
        });
        assert_eq!(emit_expr(call), "g(function(){});");
    }

    /// A *named* function expression prints its name (body-local), and
    /// params + a return body emit like the declaration form.
    #[test]
    fn named_function_expression_with_params_and_body() {
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("use")),
            arguments: vec![fexpr(
                Some("f"),
                &["a", "b"],
                vec![Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(ident("a")),
                })],
            )],
        });
        // The `;` after the last block statement is dropped in compact
        // mode (`{return a}`), so the only `;` is the outer statement's.
        assert_eq!(emit_expr(call), "use(function f(a,b){return a});");
    }

    /// Generator and async flags render their prefixes (`*` fuses with
    /// no separating space; `async` needs one).
    #[test]
    fn function_expression_generator_and_async_flags() {
        let mut generator = fexpr(None, &[], vec![]);
        if let Expression::FunctionExpression(f) = &mut generator {
            f.generator = true;
        }
        assert_eq!(emit_expr(generator), "(function*(){});");

        let async_call = {
            let mut fe = fexpr(None, &[], vec![]);
            if let Expression::FunctionExpression(f) = &mut fe {
                f.is_async = true;
            }
            Expression::CallExpression(CallExpression {
                cv: None,
                callee: Box::new(ident("h")),
                arguments: vec![fe],
            })
        };
        assert_eq!(emit_expr(async_call), "h(async function(){});");
    }

    // ---- ArrowFunctionExpression (CLOC12.151) -----------------

    /// Build an arrow with a *concise* (expression) body.
    fn arrow_concise(params: &[&str], body: Expression) -> Expression {
        Expression::ArrowFunctionExpression(ArrowFunctionExpression {
            cv: None,
            params: params
                .iter()
                .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
                .collect(),
            body: ArrowBody::Expression(Box::new(body)),
            is_async: false,
        })
    }

    /// Build an arrow with a *block* body.
    fn arrow_block(params: &[&str], body: Vec<Statement>) -> Expression {
        Expression::ArrowFunctionExpression(ArrowFunctionExpression {
            cv: None,
            params: params
                .iter()
                .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
                .collect(),
            body: ArrowBody::Block(BlockStatement { cv: None, body }),
            is_async: false,
        })
    }

    /// A single plain-identifier param drops its parens; a concise body
    /// prints bare. An arrow at statement start needs NO wrap (unlike a
    /// function expression) — `x=>x` is a valid expression statement.
    #[test]
    fn arrow_single_param_concise_body_no_parens() {
        assert_eq!(emit_expr(arrow_concise(&["x"], ident("x"))), "x=>x;");
    }

    /// Zero params keep the empty parens; two-or-more are parenthesised
    /// and comma-separated.
    #[test]
    fn arrow_zero_and_multi_params_are_parenthesised() {
        assert_eq!(emit_expr(arrow_block(&[], vec![])), "()=>{};");
        assert_eq!(emit_expr(arrow_concise(&["a", "b"], ident("a"))), "(a,b)=>a;");
    }

    /// A block body prints like a function body; the last statement drops
    /// its trailing `;` in compact mode, and the arrow adds none after `}`.
    #[test]
    fn arrow_block_body_returns() {
        let body = vec![Statement::return_statement(ReturnStatement {
            cv: None,
            argument: Some(ident("a")),
        })];
        assert_eq!(emit_expr(arrow_block(&["a"], body)), "a=>{return a};");
    }

    /// A concise body that is an object literal must be parenthesised —
    /// otherwise the leading `{` reads as a block body.
    #[test]
    fn arrow_object_literal_concise_body_is_wrapped() {
        let obj = Expression::ObjectExpression(ObjectExpression { cv: None, properties: vec![] });
        assert_eq!(emit_expr(arrow_concise(&[], obj)), "()=>({});");
    }

    /// An IIFE arrow: the callee is wrapped because `()=>{}()` is a
    /// syntax error.
    #[test]
    fn arrow_iife_wraps_callee() {
        let iife = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(arrow_block(&[], vec![])),
            arguments: vec![],
        });
        assert_eq!(emit_expr(iife), "(()=>{})();");
    }

    /// An arrow as a member object is wrapped — `()=>{}.x` is invalid.
    #[test]
    fn arrow_member_object_is_wrapped() {
        let m = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(arrow_block(&[], vec![])),
            property: Box::new(ident("x")),
            computed: false,
        });
        assert_eq!(emit_expr(m), "(()=>{}).x;");
    }

    /// As a call argument the arrow needs no parens (loosest context).
    #[test]
    fn arrow_as_argument_has_no_parens() {
        let call = Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(ident("g")),
            arguments: vec![arrow_concise(&["x"], ident("x"))],
        });
        assert_eq!(emit_expr(call), "g(x=>x);");
    }

    /// The `async` prefix needs a separating space before an
    /// unparenthesised single param (`async x=>x`) but not before `(`
    /// (`async()=>{}`).
    #[test]
    fn arrow_async_prefix() {
        let mut single = arrow_concise(&["x"], ident("x"));
        if let Expression::ArrowFunctionExpression(a) = &mut single {
            a.is_async = true;
        }
        assert_eq!(emit_expr(single), "async x=>x;");

        let mut zero = arrow_block(&[], vec![]);
        if let Expression::ArrowFunctionExpression(a) = &mut zero {
            a.is_async = true;
        }
        assert_eq!(emit_expr(zero), "async()=>{};");
    }

    // ---- TemplateLiteral (CLOC12.154) -------------------------

    fn tquasi(raw: &str, tail: bool) -> TemplateElement {
        TemplateElement { cv: None, raw: raw.to_string(), cooked: Some(raw.to_string()), tail }
    }

    fn template(quasis: Vec<TemplateElement>, expressions: Vec<Expression>) -> Expression {
        Expression::TemplateLiteral(TemplateLiteral { cv: None, quasis, expressions })
    }

    /// A no-substitution template prints its raw text between backticks.
    #[test]
    fn template_no_substitution() {
        assert_eq!(emit_expr(template(vec![tquasi("abc", true)], vec![])), "`abc`;");
    }

    /// A single `${…}` interleaves the two quasis around the expression.
    #[test]
    fn template_single_substitution() {
        let t = template(vec![tquasi("a", false), tquasi("b", true)], vec![ident("x")]);
        assert_eq!(emit_expr(t), "`a${x}b`;");
    }

    /// Adjacent substitutions have empty quasis between them; the run still
    /// opens and closes with a (possibly empty) quasi.
    #[test]
    fn template_adjacent_substitutions() {
        let t = template(
            vec![tquasi("", false), tquasi("", false), tquasi("", true)],
            vec![ident("x"), ident("y")],
        );
        assert_eq!(emit_expr(t), "`${x}${y}`;");
    }

    /// A `${…}` context is the loosest, so a low-precedence inner expression
    /// needs no parens — the braces already delimit it.
    #[test]
    fn template_substitution_needs_no_inner_parens() {
        let sum = Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: coding_adventures_javascript_ast::BinaryOperator::Add,
            left: Box::new(ident("a")),
            right: Box::new(ident("b")),
        });
        let t = template(vec![tquasi("", false), tquasi("", true)], vec![sum]);
        assert_eq!(emit_expr(t), "`${a+b}`;");
    }

    /// A template as a member-access object needs no wrapping — it's a
    /// primary expression (`` `abc`.length ``).
    #[test]
    fn template_as_member_object_is_not_wrapped() {
        let m = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(template(vec![tquasi("abc", true)], vec![])),
            property: Box::new(ident("length")),
            computed: false,
        });
        assert_eq!(emit_expr(m), "`abc`.length;");
    }

    /// gap-158: a no-substitution template whose `raw` carries a *literal*
    /// newline round-trips it verbatim — `emit_template_element` splits on
    /// `'\n'` and routes the break through `newline()` instead of tripping
    /// `write_str`'s no-embedded-newline assert.
    #[test]
    fn template_preserves_interior_newline() {
        assert_eq!(emit_expr(template(vec![tquasi("a\nb", true)], vec![])), "`a\nb`;");
    }

    /// gap-158: the newline-aware path also covers quasis *inside* a `${…}`
    /// substitution template — the leading quasi spans two lines around the
    /// insert.
    #[test]
    fn template_substitution_quasi_preserves_newline() {
        let t = template(vec![tquasi("a\nb", false), tquasi("c", true)], vec![ident("x")]);
        assert_eq!(emit_expr(t), "`a\nb${x}c`;");
    }

    /// gap-158: a bare `'\n'` quasi (leading + trailing empty split segments)
    /// emits just the newline — the empty segments write nothing.
    #[test]
    fn template_bare_newline_quasi() {
        assert_eq!(emit_expr(template(vec![tquasi("\n", true)], vec![])), "`\n`;");
    }

    // ---- UpdateExpression (CLOC12.158) ------------------------

    fn update(op: UpdateOperator, prefix: bool, arg: Expression) -> Expression {
        Expression::UpdateExpression(UpdateExpression {
            cv: None,
            operator: op,
            prefix,
            argument: Box::new(arg),
        })
    }

    fn binexpr(op: BinaryOperator, l: Expression, r: Expression) -> Expression {
        Expression::BinaryExpression(BinaryExpression {
            cv: None,
            operator: op,
            left: Box::new(l),
            right: Box::new(r),
        })
    }

    /// The four core shapes: prefix/postfix × increment/decrement.
    #[test]
    fn update_prefix_increment() {
        assert_eq!(emit_expr(update(UpdateOperator::Increment, true, ident("x"))), "++x;");
    }
    #[test]
    fn update_postfix_increment() {
        assert_eq!(emit_expr(update(UpdateOperator::Increment, false, ident("x"))), "x++;");
    }
    #[test]
    fn update_prefix_decrement() {
        assert_eq!(emit_expr(update(UpdateOperator::Decrement, true, ident("x"))), "--x;");
    }
    #[test]
    fn update_postfix_decrement() {
        assert_eq!(emit_expr(update(UpdateOperator::Decrement, false, ident("x"))), "x--;");
    }

    /// `a - (--b)` must print `a- --b`, never `a---b` (which JS reparses as
    /// `(a--)-b`). The binary `-` emitter inserts the seam space because
    /// `arg_starts_with_sign` reports the prefix `--`'s leading `-`.
    #[test]
    fn prefix_decrement_after_minus_needs_space() {
        let e = binexpr(
            BinaryOperator::Sub,
            ident("a"),
            update(UpdateOperator::Decrement, true, ident("b")),
        );
        assert_eq!(emit_expr(e), "a- --b;");
    }

    /// `a + (++b)` must print `a+ ++b`, never `a+++b` (which JS reparses as
    /// `(a++)+b`).
    #[test]
    fn prefix_increment_after_plus_needs_space() {
        let e = binexpr(
            BinaryOperator::Add,
            ident("a"),
            update(UpdateOperator::Increment, true, ident("b")),
        );
        assert_eq!(emit_expr(e), "a+ ++b;");
    }

    /// `(x++) + y`: the postfix `++` leaves the output ending in `+`, so the
    /// following binary `+` needs a left-seam space (`x++ +y`) or the `++`
    /// would swallow it into `x+++y` = `(x++)+y` — same value here, but the
    /// emitter guards the seam unconditionally via the output-tail check.
    #[test]
    fn postfix_increment_before_plus_needs_space() {
        let e = binexpr(
            BinaryOperator::Add,
            update(UpdateOperator::Increment, false, ident("x")),
            ident("y"),
        );
        assert_eq!(emit_expr(e), "x++ +y;");
    }

    /// A postfix update as a member-access object is parenthesised: `x++` is
    /// not a valid `MemberExpression` object, so `(x++).y` — the `PREC_UNARY`
    /// tag forces the wrap under the primary-precedence member parent.
    #[test]
    fn postfix_update_as_member_object_is_wrapped() {
        let m = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(update(UpdateOperator::Increment, false, ident("x"))),
            property: Box::new(ident("y")),
            computed: false,
        });
        assert_eq!(emit_expr(m), "(x++).y;");
    }

    /// A prefix update as an exponentiation base is parenthesised — a bare
    /// `++x**2` is a syntax error, so `(++x)**2`.
    #[test]
    fn prefix_update_as_exponent_base_is_wrapped() {
        let e = binexpr(
            BinaryOperator::Exp,
            update(UpdateOperator::Increment, true, ident("x")),
            num(2.0),
        );
        assert_eq!(emit_expr(e), "(++x)**2;");
    }

    // ---- NewExpression (CLOC12.159) ------------------------------------

    fn new_expr(callee: Expression, arguments: Vec<Expression>) -> Expression {
        Expression::NewExpression(NewExpression {
            cv: None,
            callee: Box::new(callee),
            arguments,
        })
    }
    fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
        Expression::CallExpression(CallExpression {
            cv: None,
            callee: Box::new(callee),
            arguments,
        })
    }

    /// `new X()` — plain identifier callee, empty args. A space separates the
    /// `new` keyword from the identifier so they do not fuse into `newX`.
    #[test]
    fn new_identifier_no_args() {
        assert_eq!(emit_expr(new_expr(ident("X"), vec![])), "new X();");
    }

    /// `new X(a, b)` — arguments are comma-separated, no trailing-space in the
    /// minified form.
    #[test]
    fn new_with_args() {
        assert_eq!(
            emit_expr(new_expr(ident("X"), vec![ident("a"), ident("b")])),
            "new X(a,b);"
        );
    }

    /// `new a.b.c()` — a pure member-chain callee is a valid `new` target and
    /// stays paren-free; the `new` keeps its separating space.
    #[test]
    fn new_member_chain_callee_not_wrapped() {
        let callee = member(member(ident("a"), "b", false), "c", false);
        assert_eq!(emit_expr(new_expr(callee, vec![])), "new a.b.c();");
    }

    /// `new (f())()` — a call in the callee spine MUST be parenthesised, or the
    /// appended `()` would bind to the inner call (`new f()()` = `(new f())()`,
    /// a different program). The wrapping paren also removes the need for the
    /// `new`-keyword space.
    #[test]
    fn new_call_callee_is_wrapped() {
        let callee = call(ident("f"), vec![]);
        assert_eq!(emit_expr(new_expr(callee, vec![])), "new(f())();");
    }

    /// `new a.b().c()` — the callee's member spine bottoms out in a call
    /// (`a.b()`), so the whole target is wrapped: `new (a.b().c)()`.
    #[test]
    fn new_callee_with_call_in_member_spine_is_wrapped() {
        let callee = member(call(member(ident("a"), "b", false), vec![]), "c", false);
        assert_eq!(emit_expr(new_expr(callee, vec![])), "new(a.b().c)();");
    }

    /// `(new X()).y` — an *argumented* `new` binds at member strength, so a
    /// member parent needs NO extra parens (the `new X()` groups on its own):
    /// `new X().y`.
    #[test]
    fn argumented_new_as_member_object_not_wrapped() {
        let m = member(new_expr(ident("X"), vec![ident("a")]), "y", false);
        assert_eq!(emit_expr(m), "new X(a).y;");
    }

    /// A no-argument `new X` is emitted canonically as `new X()` (the parens
    /// are always printed), so it binds at member strength and as a member
    /// object needs NO extra wrap: `new X().y`. The always-printed `()` is what
    /// makes `new X.y` (which would reparse as `new (X.y)`) unreachable.
    #[test]
    fn no_arg_new_as_member_object_prints_argumented() {
        let m = member(new_expr(ident("X"), vec![]), "y", false);
        assert_eq!(emit_expr(m), "new X().y;");
    }

    /// `new` nests: the inner `new X()` is a valid target (not a call), so no
    /// wrap is forced and the outer `new` keeps its keyword space:
    /// `new new X()()`.
    #[test]
    fn nested_new_inner_not_wrapped() {
        let inner = new_expr(ident("X"), vec![]);
        assert_eq!(emit_expr(new_expr(inner, vec![])), "new new X()();");
    }

    // ---- SequenceExpression (CLOC12.160) -------------------------------

    fn seq(exprs: Vec<Expression>) -> Expression {
        Expression::SequenceExpression(SequenceExpression { cv: None, expressions: exprs })
    }

    /// A sequence at statement position prints bare — the loosest expression,
    /// nothing captures it: `a,b,c;`.
    #[test]
    fn sequence_at_statement_is_bare() {
        assert_eq!(emit_expr(seq(vec![ident("a"), ident("b"), ident("c")])), "a,b,c;");
    }

    /// A sequence as a call argument MUST wrap, or `f(a,b)` would be a
    /// two-argument call instead of one sequence argument: `f((a,b));`.
    #[test]
    fn sequence_as_sole_call_arg_is_wrapped() {
        let e = call(ident("f"), vec![seq(vec![ident("a"), ident("b")])]);
        assert_eq!(emit_expr(e), "f((a,b));");
    }

    /// A sequence as ONE of several call arguments wraps so the arity is
    /// preserved: `f((a,b),c);` — never the three-argument `f(a,b,c)`.
    #[test]
    fn sequence_as_call_arg_preserves_arity() {
        let e = call(ident("f"), vec![seq(vec![ident("a"), ident("b")]), ident("c")]);
        assert_eq!(emit_expr(e), "f((a,b),c);");
    }

    /// A sequence as an array element wraps, or the element count changes:
    /// `[(a,b),c];` — never the three-element `[a,b,c]`.
    #[test]
    fn sequence_as_array_element_is_wrapped() {
        let e = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(seq(vec![ident("a"), ident("b")])), Some(ident("c"))],
        });
        assert_eq!(emit_expr(e), "[(a,b),c];");
    }

    /// A sequence as an assignment RHS wraps: `x=(a,b);` — a bare `x=a,b`
    /// reparses as `(x=a),b`, a different program.
    #[test]
    fn sequence_as_assignment_rhs_is_wrapped() {
        let e = Expression::AssignmentExpression(AssignmentExpression {
            cv: None,
            operator: AssignmentOperator::Eq,
            left: AssignmentTarget::Identifier(Identifier { cv: None, name: "x".to_string() }),
            right: Box::new(seq(vec![ident("a"), ident("b")])),
        });
        assert_eq!(emit_expr(e), "x=(a,b);");
    }

    /// A sequence as a computed-member key needs NO parens — the `[ ]` already
    /// delimits a full `Expression`: `a[b,c];` (which evaluates the key to `c`).
    #[test]
    fn sequence_as_computed_member_key_is_bare() {
        let e = Expression::MemberExpression(MemberExpression {
            cv: None,
            object: Box::new(ident("a")),
            property: Box::new(seq(vec![ident("b"), ident("c")])),
            computed: true,
        });
        assert_eq!(emit_expr(e), "a[b,c];");
    }

    /// A sequence as a conditional branch wraps (the branch is an
    /// `AssignmentExpression`): `x?(a,b):c;`.
    #[test]
    fn sequence_as_conditional_branch_is_wrapped() {
        let e = Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("x")),
            consequent: Box::new(seq(vec![ident("a"), ident("b")])),
            alternate: Box::new(ident("c")),
        });
        assert_eq!(emit_expr(e), "x?(a,b):c;");
    }

    /// A sequence as a unary operand wraps: `!(a,b);` — a bare `!a,b` parses as
    /// `(!a),b`.
    #[test]
    fn sequence_as_unary_operand_is_wrapped() {
        let e = Expression::UnaryExpression(UnaryExpression {
            cv: None,
            operator: UnaryOperator::Not,
            prefix: true,
            argument: Box::new(seq(vec![ident("a"), ident("b")])),
        });
        assert_eq!(emit_expr(e), "!(a,b);");
    }

    // ---- TaggedTemplateExpression (CLOC12.161) -------------------------

    /// Build a raw `TemplateLiteral` struct (not wrapped in `Expression`) for
    /// use as a tagged-template quasi.
    fn raw_template(quasis: Vec<TemplateElement>, expressions: Vec<Expression>) -> TemplateLiteral {
        TemplateLiteral { cv: None, quasis, expressions }
    }

    fn tagged(tag: Expression, quasi: TemplateLiteral) -> Expression {
        Expression::TaggedTemplateExpression(TaggedTemplateExpression {
            cv: None,
            tag: Box::new(tag),
            quasi,
        })
    }

    /// `` tag`abc` `` — an identifier tag directly precedes a no-substitution
    /// template; no separator between the tag and the backtick.
    #[test]
    fn tagged_identifier_no_sub() {
        let e = tagged(ident("tag"), raw_template(vec![tquasi("abc", true)], vec![]));
        assert_eq!(emit_expr(e), "tag`abc`;");
    }

    /// `` a.b`x` `` — a member-chain tag stays paren-free (member binds at
    /// `PREC_PRIMARY`, same as the tagged-template node).
    #[test]
    fn tagged_member_tag_not_wrapped() {
        let tag = member(ident("a"), "b", false);
        let e = tagged(tag, raw_template(vec![tquasi("x", true)], vec![]));
        assert_eq!(emit_expr(e), "a.b`x`;");
    }

    /// `` String.raw`a${x}b` `` — a substitution template as the quasi: the
    /// `${…}` parts round-trip through the reused template emitter.
    #[test]
    fn tagged_with_substitution() {
        let tag = member(ident("String"), "raw", false);
        let quasi = raw_template(vec![tquasi("a", false), tquasi("b", true)], vec![ident("x")]);
        let e = tagged(tag, quasi);
        assert_eq!(emit_expr(e), "String.raw`a${x}b`;");
    }

    /// A member access on a tagged template needs no parens — the tagged
    /// template is `PREC_PRIMARY`: `` a`x`.length ``.
    #[test]
    fn member_on_tagged_is_paren_free() {
        let inner = tagged(ident("a"), raw_template(vec![tquasi("x", true)], vec![]));
        let e = member(inner, "length", false);
        assert_eq!(emit_expr(e), "a`x`.length;");
    }

    /// A looser tag is parenthesised — a sequence tag would otherwise tag only
    /// its last operand: `` (a,b)`x` ``.
    #[test]
    fn sequence_tag_is_wrapped() {
        let tag = seq(vec![ident("a"), ident("b")]);
        let e = tagged(tag, raw_template(vec![tquasi("x", true)], vec![]));
        assert_eq!(emit_expr(e), "(a,b)`x`;");
    }

    // ---- SpreadElement (CLOC12.162) ------------------------------------

    fn spread(argument: Expression) -> Expression {
        Expression::SpreadElement(SpreadElement { cv: None, argument: Box::new(argument) })
    }

    /// `f(...a)` — a spread as the sole call argument prints bare, with no
    /// space between `...` and the argument.
    #[test]
    fn spread_as_sole_call_arg() {
        let e = call(ident("f"), vec![spread(ident("a"))]);
        assert_eq!(emit_expr(e), "f(...a);");
    }

    /// `f(a,...b,c)` — a spread interleaved with plain arguments keeps its
    /// position and the surrounding arity.
    #[test]
    fn spread_preserves_call_arity() {
        let e = call(ident("f"), vec![ident("a"), spread(ident("b")), ident("c")]);
        assert_eq!(emit_expr(e), "f(a,...b,c);");
    }

    /// `[1,...a,2]` — a spread as an array-literal element prints bare between
    /// its siblings.
    #[test]
    fn spread_as_array_element() {
        let e = Expression::ArrayExpression(ArrayExpression {
            cv: None,
            elements: vec![Some(num(1.0)), Some(spread(ident("a"))), Some(num(2.0))],
        });
        assert_eq!(emit_expr(e), "[1,...a,2];");
    }

    /// `new F(...a)` — a spread flows into a `new` argument list exactly as a
    /// call argument does (`emit_new` always prints the argument parens).
    #[test]
    fn spread_as_new_argument() {
        let e = new_expr(ident("F"), vec![spread(ident("a"))]);
        assert_eq!(emit_expr(e), "new F(...a);");
    }

    /// `f(...(a,b))` — a **sequence** spread argument is the one form that must
    /// wrap: a bare `...a,b` would spread only `a` and leave `,b` as a second
    /// list slot. This is the crux of the assignment-precedence tag.
    #[test]
    fn spread_sequence_argument_is_wrapped() {
        let e = call(ident("f"), vec![spread(seq(vec![ident("a"), ident("b")]))]);
        assert_eq!(emit_expr(e), "f(...(a,b));");
    }

    /// `f(...a?b:c)` — a conditional argument binds tighter than the sequence
    /// floor, so it prints bare (spread's operand grammar is an
    /// `AssignmentExpression`, which subsumes the conditional): no over-wrap.
    #[test]
    fn spread_conditional_argument_is_bare() {
        let cond = Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("a")),
            consequent: Box::new(ident("b")),
            alternate: Box::new(ident("c")),
        });
        let e = call(ident("f"), vec![spread(cond)]);
        assert_eq!(emit_expr(e), "f(...a?b:c);");
    }

    // =================================================================
    // YieldExpression (`yield` / `yield x` / `yield* xs`) — CLOC12.163
    // =================================================================

    /// Build a `YieldExpression` from its two axes.
    fn yld(delegate: bool, argument: Option<Expression>) -> Expression {
        Expression::YieldExpression(YieldExpression {
            cv: None,
            delegate,
            argument: argument.map(Box::new),
        })
    }

    /// `yield` — a bare yield with no operand prints just the keyword.
    #[test]
    fn yield_bare_prints_keyword_only() {
        assert_eq!(emit_expr(yld(false, None)), "yield;");
    }

    /// `yield a` — a non-delegating yield separates the keyword from its
    /// argument with a mandatory space (`yielda` would be one identifier).
    #[test]
    fn yield_value_has_required_space() {
        assert_eq!(emit_expr(yld(false, Some(ident("a")))), "yield a;");
    }

    /// `yield*xs` — a delegating yield needs no separator: the `*` already
    /// terminates the keyword token, so the argument abuts it.
    #[test]
    fn yield_delegate_needs_no_space() {
        assert_eq!(emit_expr(yld(true, Some(ident("xs")))), "yield*xs;");
    }

    /// `yield*a.b` — a delegating yield's argument may be a member chain; it
    /// binds tighter than assignment so it prints bare after `yield*`.
    #[test]
    fn yield_delegate_member_argument() {
        let e = yld(true, Some(member(ident("a"), "b", false)));
        assert_eq!(emit_expr(e), "yield*a.b;");
    }

    /// `yield a?b:c` — a conditional argument binds tighter than the sequence
    /// floor (yield's operand grammar is an `AssignmentExpression`), so it
    /// prints bare — no over-wrap.
    #[test]
    fn yield_conditional_argument_is_bare() {
        let cond = Expression::ConditionalExpression(ConditionalExpression {
            cv: None,
            test: Box::new(ident("a")),
            consequent: Box::new(ident("b")),
            alternate: Box::new(ident("c")),
        });
        assert_eq!(emit_expr(yld(false, Some(cond))), "yield a?b:c;");
    }

    /// `yield a=b` — an assignment argument also prints bare (it is exactly at
    /// the operand's assignment precedence).
    #[test]
    fn yield_assignment_argument_is_bare() {
        let e = yld(false, Some(assign("a", AssignmentOperator::Eq, ident("b"))));
        assert_eq!(emit_expr(e), "yield a=b;");
    }

    /// `yield (a,b)` — a **sequence** argument is the one form that must wrap:
    /// it binds looser than the assignment-precedence operand floor.
    #[test]
    fn yield_sequence_argument_is_wrapped() {
        let e = yld(false, Some(seq(vec![ident("a"), ident("b")])));
        assert_eq!(emit_expr(e), "yield (a,b);");
    }

    /// `(yield a)+1` — the whole yield binds looser than `+`, so a binary parent
    /// wraps it (`expr_prec` tags yield at `PREC_ASSIGNMENT`).
    #[test]
    fn yield_wrapped_as_binary_operand() {
        let e = binary(BinaryOperator::Add, yld(false, Some(ident("a"))), num(1.0));
        assert_eq!(emit_expr(e), "(yield a)+1;");
    }

    /// `(yield a).b` — a member parent binds at primary strength and wraps the
    /// looser yield object.
    #[test]
    fn yield_wrapped_as_member_object() {
        let e = member(yld(false, Some(ident("a"))), "b", false);
        assert_eq!(emit_expr(e), "(yield a).b;");
    }

    // =================================================================
    // AwaitExpression (`await x`) — CLOC12.164
    // =================================================================

    /// Build an `AwaitExpression` (named `aw` — `await` is a Rust keyword).
    fn aw(argument: Expression) -> Expression {
        Expression::AwaitExpression(AwaitExpression { cv: None, argument: Box::new(argument) })
    }

    /// `await p` — the keyword and operand are separated by a mandatory space
    /// (`awaitp` would be one identifier).
    #[test]
    fn await_value_requires_space() {
        assert_eq!(emit_expr(aw(ident("p"))), "await p;");
    }

    /// `await a.b` — a member operand binds tighter than unary, so it prints
    /// bare after `await`.
    #[test]
    fn await_member_operand_is_bare() {
        assert_eq!(emit_expr(aw(member(ident("a"), "b", false))), "await a.b;");
    }

    /// `await f()` — a call operand also binds tighter than unary → bare.
    #[test]
    fn await_call_operand_is_bare() {
        assert_eq!(emit_expr(aw(call(ident("f"), vec![]))), "await f();");
    }

    /// `await (a+b)` — a **binary** operand binds looser than unary, so it must
    /// wrap (a bare `await a+b` would parse as `(await a)+b`).
    #[test]
    fn await_binary_operand_is_wrapped() {
        let e = aw(binary(BinaryOperator::Add, ident("a"), ident("b")));
        assert_eq!(emit_expr(e), "await (a+b);");
    }

    /// `await p+1` — the whole await binds *tighter* than `+` (await is unary),
    /// so a binary parent leaves it bare on the left: `(await p)+1`.
    #[test]
    fn await_binds_tighter_than_binary_parent() {
        let e = binary(BinaryOperator::Add, aw(ident("p")), num(1.0));
        assert_eq!(emit_expr(e), "await p+1;");
    }

    /// `(await p).x` — a member parent binds at primary strength and wraps the
    /// looser await object.
    #[test]
    fn await_wrapped_as_member_object() {
        assert_eq!(emit_expr(member(aw(ident("p")), "x", false)), "(await p).x;");
    }

    /// `(await f)()` — a call callee likewise wraps the await.
    #[test]
    fn await_wrapped_as_call_callee() {
        assert_eq!(emit_expr(call(aw(ident("f")), vec![])), "(await f)();");
    }

    /// `await await p` — a nested await operand prints bare (await is exactly at
    /// the unary operand floor).
    #[test]
    fn await_nested_is_bare() {
        assert_eq!(emit_expr(aw(aw(ident("p")))), "await await p;");
    }

    // =================================================================
    // ThisExpression (`this`) — CLOC12.165
    // =================================================================

    /// Build a `ThisExpression`.
    fn this_expr() -> Expression {
        Expression::ThisExpression(ThisExpression { cv: None })
    }

    /// `this` — a bare keyword, printed verbatim.
    #[test]
    fn this_emits_bare_keyword() {
        assert_eq!(emit_expr(this_expr()), "this;");
    }

    /// `this.x` — `this` is a primary, so a member parent needs no parens.
    #[test]
    fn this_as_member_object_is_bare() {
        assert_eq!(emit_expr(member(this_expr(), "x", false)), "this.x;");
    }

    /// `this()` — as a call callee `this` stays bare (primary strength).
    #[test]
    fn this_as_call_callee_is_bare() {
        assert_eq!(emit_expr(call(this_expr(), vec![])), "this();");
    }

    /// `f(this)` — `this` as a call argument is a plain primary operand.
    #[test]
    fn this_as_call_argument_is_bare() {
        assert_eq!(emit_expr(call(ident("f"), vec![this_expr()])), "f(this);");
    }

    /// `this+1` — even a binary parent leaves the primary `this` bare.
    #[test]
    fn this_under_binary_parent_is_bare() {
        let e = binary(BinaryOperator::Add, this_expr(), num(1.0));
        assert_eq!(emit_expr(e), "this+1;");
    }

    // =================================================================
    // Super (`super`) — CLOC12.166
    // =================================================================

    /// Build a `Super`.
    fn super_expr() -> Expression {
        Expression::Super(Super { cv: None })
    }

    /// `super` — a bare keyword, printed verbatim.
    #[test]
    fn super_emits_bare_keyword() {
        assert_eq!(emit_expr(super_expr()), "super;");
    }

    /// `super.x` — `super` is a primary, so a member parent needs no parens.
    #[test]
    fn super_as_member_object_is_bare() {
        assert_eq!(emit_expr(member(super_expr(), "x", false)), "super.x;");
    }

    /// `super()` — as a call callee `super` stays bare (primary strength).
    #[test]
    fn super_as_call_callee_is_bare() {
        assert_eq!(emit_expr(call(super_expr(), vec![])), "super();");
    }

    /// `super.m()` — a method call off `super` composes without parens.
    #[test]
    fn super_method_call_is_bare() {
        assert_eq!(emit_expr(call(member(super_expr(), "m", false), vec![])), "super.m();");
    }

    /// `super+1` — even a binary parent leaves the primary `super` bare.
    #[test]
    fn super_under_binary_parent_is_bare() {
        let e = binary(BinaryOperator::Add, super_expr(), num(1.0));
        assert_eq!(emit_expr(e), "super+1;");
    }

    // =================================================================
    // NewTarget (`new.target`) — CLOC12.167
    // =================================================================

    /// Build a `NewTarget`.
    fn new_target_expr() -> Expression {
        Expression::NewTarget(NewTarget { cv: None })
    }

    /// `new.target` — the meta-property, printed as its literal spelling.
    #[test]
    fn new_target_emits_literal_spelling() {
        assert_eq!(emit_expr(new_target_expr()), "new.target;");
    }

    /// `new.target.x` — `new.target` is a primary, so a member parent needs no
    /// parens (the trailing `.x` is a real member access on top of it).
    #[test]
    fn new_target_as_member_object_is_bare() {
        assert_eq!(emit_expr(member(new_target_expr(), "x", false)), "new.target.x;");
    }

    /// `f(new.target)` — as a call argument it is a plain primary operand.
    #[test]
    fn new_target_as_call_argument_is_bare() {
        assert_eq!(emit_expr(call(ident("f"), vec![new_target_expr()])), "f(new.target);");
    }

    /// `new.target+1` — even a binary parent leaves the primary bare.
    #[test]
    fn new_target_under_binary_parent_is_bare() {
        let e = binary(BinaryOperator::Add, new_target_expr(), num(1.0));
        assert_eq!(emit_expr(e), "new.target+1;");
    }

    // =================================================================
    // ImportMeta (`import.meta`) — CLOC12.168
    // =================================================================

    /// Build an `ImportMeta`.
    fn import_meta_expr() -> Expression {
        Expression::ImportMeta(ImportMeta { cv: None })
    }

    /// `import.meta` — the module meta-property, printed as its literal spelling.
    #[test]
    fn import_meta_emits_literal_spelling() {
        assert_eq!(emit_expr(import_meta_expr()), "import.meta;");
    }

    /// `import.meta.url` — `import.meta` is a primary, so a member parent needs
    /// no parens (the trailing `.url` is a real member access on top of it).
    #[test]
    fn import_meta_as_member_object_is_bare() {
        assert_eq!(emit_expr(member(import_meta_expr(), "url", false)), "import.meta.url;");
    }

    /// `f(import.meta)` — as a call argument it is a plain primary operand.
    #[test]
    fn import_meta_as_call_argument_is_bare() {
        assert_eq!(emit_expr(call(ident("f"), vec![import_meta_expr()])), "f(import.meta);");
    }

    /// `import.meta+1` — even a binary parent leaves the primary bare.
    #[test]
    fn import_meta_under_binary_parent_is_bare() {
        let e = binary(BinaryOperator::Add, import_meta_expr(), num(1.0));
        assert_eq!(emit_expr(e), "import.meta+1;");
    }

    // =================================================================
    // ImportExpression (dynamic `import(x)`) — CLOC12.169
    // =================================================================

    /// Build a dynamic `import(source)`.
    fn import_expr(source: Expression) -> Expression {
        Expression::ImportExpression(ImportExpression { cv: None, source: Box::new(source) })
    }

    /// `import("m")` — a string-literal specifier prints inside the literal
    /// parens with no surrounding space.
    #[test]
    fn import_expression_string_specifier() {
        assert_eq!(emit_expr(import_expr(string("m"))), "import(\"m\");");
    }

    /// `import(x)` — an identifier specifier.
    #[test]
    fn import_expression_identifier_specifier() {
        assert_eq!(emit_expr(import_expr(ident("x"))), "import(x);");
    }

    /// `import(a+b)` — a binary specifier prints bare inside the parens (it is
    /// tighter than a sequence, so it needs no inner wrapping).
    #[test]
    fn import_expression_binary_specifier_is_bare() {
        let e = import_expr(binary(BinaryOperator::Add, ident("a"), ident("b")));
        assert_eq!(emit_expr(e), "import(a+b);");
    }

    /// `import((a,b))` — a *sequence* specifier is looser than assignment, so it
    /// wraps in its own parens inside the import argument.
    #[test]
    fn import_expression_sequence_specifier_wraps() {
        let seq = Expression::SequenceExpression(SequenceExpression {
            cv: None,
            expressions: vec![ident("a"), ident("b")],
        });
        assert_eq!(emit_expr(import_expr(seq)), "import((a,b));");
    }

    /// `import(x).then(f)` — the whole import is a `PREC_PRIMARY` call-like
    /// primary, so a member/call parent composes without wrapping it.
    #[test]
    fn import_expression_as_member_object_is_bare() {
        let e = call(member(import_expr(ident("x")), "then", false), vec![ident("f")]);
        assert_eq!(emit_expr(e), "import(x).then(f);");
    }
}

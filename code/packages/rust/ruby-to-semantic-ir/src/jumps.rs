//! Early `return`, `break` and `next`, lowered to SIR without a jump.
//!
//! # The problem (#18798, items 14 and 19)
//!
//! SIR has no `return` statement.  A function's value is its body
//! `Block`'s trailing `value`, and control leaves the function only by
//! reaching the end of that block.  The Ruby lowerer turned a `return x`,
//! a `break` or a `next` into a marker — `BuiltinCall("return", [x])` with
//! the `Divergent` effect — and no backend knew what to do with it:
//!
//! | backend                | `def f; return 1; rescue; 2; end; puts f`  |
//! |------------------------|--------------------------------------------|
//! | Python, JavaScript, Go | prints `2`: the runtime raised "unknown builtin: return", and the method's own `rescue` caught it |
//! | Rust                   | panics: `unknown builtin: return`          |
//! | C, Ruby                | refuse the program                         |
//!
//! # The idea: a flag, and guards
//!
//! Each function that jumps gets two locals the source cannot spell
//! (fresh names, `__jump_<n>` and `__ret_<n>`):
//!
//! | local    | holds                                                    |
//! |----------|----------------------------------------------------------|
//! | `__ret`  | the value a `return` (or a block's `next`) hands back    |
//! | `__jump` | what control is doing right now, as a small integer      |
//!
//! | `__jump` | meaning                                                  |
//! |----------|----------------------------------------------------------|
//! | `0`      | running normally                                         |
//! | `1`      | returning: leave every loop, skip every statement left   |
//! | `2`      | breaking: leave the innermost loop                       |
//! | `3`      | `next`: skip the rest of this iteration                  |
//!
//! A jump becomes two stores, and every statement that could have jumped
//! puts the statements after it behind a guard.  So
//!
//! ```ruby
//! def f(x)
//!   if x > 0
//!     return "pos"
//!   end
//!   "nonpos"
//! end
//! ```
//!
//! lowers as if it had been written
//!
//! ```ruby
//! def f(x)
//!   __ret = nil
//!   __jump = 0
//!   if x > 0
//!     __ret = "pos"
//!     __jump = 1
//!   end
//!   if __jump == 0
//!     __ret = "nonpos"     # the body's last value is stored, too
//!   end
//!   __ret
//! end
//! ```
//!
//! Loops add two rules.  A loop whose body can `break` or `return` also
//! tests the flag before each trip (`while` becomes `while (__jump == 0 ?
//! cond : false)`).  A `for` over a range (`ForRange`) that can leave early
//! becomes such a `while` too, counting in a local of its own, so a
//! `break` ends it at once — `for i in 0...2**62 do break end` must not
//! count to 2**62.  A `for` over a collection (`ForEach`), which has no
//! condition to test, runs its remaining trips with the whole body behind
//! the guard: they do nothing, and there are only as many as the
//! collection, which already exists, has elements.  And the flag is put
//! back to `0` where the jump ends: after the loop for a `break`, at the
//! end of the iteration for a `next`.
//!
//! The guards sit side by side, one per stretch of statements between two
//! jumps, never one inside the next, so the lowered tree is no deeper for a
//! method of a thousand guard clauses than for one of a single clause, and
//! the lowering is one pass over each list (see [`Ctx::lower_list`]).
//!
//! Everything this produces is ordinary SIR — locals, stores, `if`, `==` —
//! that every backend already runs, so one lowering fixes all six.  A
//! `return` inside `begin … rescue` is no longer an exception, so the
//! `rescue` cannot catch it, and an `ensure` still runs on the way out.
//!
//! # What is refused, and why
//!
//! A jump the flag cannot express is a compile error naming the keyword,
//! never a silent change of meaning:
//!
//! - **`return` inside a block** (`each do … return … end`) leaves the
//!   *method that called* the block.  A block is hoisted to a function of
//!   its own, and nothing here carries a flag from it back to its caller.
//! - **`break` inside a block** likewise ends the call that took the block.
//! - **`break`/`next` outside any loop or block** is a Ruby syntax error.
//! - **A jump inside an expression** (`x = (return 1)`, `f(return)`,
//!   `c && return`) — only a jump that is a statement of its own, or the
//!   value of an `if` branch, is lowered.
//!
//! `next` in a block (outside a loop in that block) ends the block's call
//! with its value, which is exactly a `return` from the hoisted function,
//! so it is lowered as one.  `redo` and `retry` are left alone.

use std::cell::RefCell;
use std::collections::HashSet;

use semantic_ir::walker::{walk_expr_default, walk_stmt_default, Visitor};
use semantic_ir::{Block, Effect, EffectSet, Expr, Scope, Span, Stmt, MAX_IR_DEPTH};

/// What kind of function body is being lowered.  It decides what `return`,
/// `break` and `next` mean outside any loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Body {
    /// A `def` (or a method a class or module defines).
    Method,
    /// The program's top level.  Ruby lets a script `return`: the rest of
    /// the file does not run.
    Script,
    /// A block (`do … end`, `{ … }`) hoisted to a function of its own.
    Block,
    /// A lambda body (`->(x) { … }`): `return` leaves the lambda.
    Lambda,
}

/// A jump keyword, as the lowerer's marker names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Jump {
    Return,
    Break,
    Next,
}

impl Jump {
    fn keyword(self) -> &'static str {
        match self {
            Jump::Return => "return",
            Jump::Break => "break",
            Jump::Next => "next",
        }
    }
}

/// The values of the `__jump` flag (see the module docs' table).
const RUNNING: i64 = 0;
const RETURNING: i64 = 1;
const BREAKING: i64 = 2;
const NEXTING: i64 = 3;

/// How many SIR levels one guard adds around the statements after it: the
/// `ExprStmt`, its `If`, and the `If`'s branch `Block`.
const GUARD_LEVELS: usize = 3;

/// A jump that cannot be lowered, with where it is.
#[derive(Debug)]
pub(crate) struct JumpError {
    pub message: String,
    pub span: Span,
}

/// The jump an expression *is*, if it is one of the lowerer's markers:
/// `BuiltinCall("return" | "break" | "next", [value])` with the
/// `Divergent` effect (see `lower_statement_inner`).  A user method can
/// never lower to this: a method call is a `DirectCall` or a `__method__`
/// envelope, and neither carries `Divergent`.
fn as_jump(expr: &Expr) -> Option<(Jump, &Expr, &Span)> {
    let Expr::BuiltinCall {
        name,
        args,
        effects,
        span,
    } = expr
    else {
        return None;
    };
    if !effects.contains(Effect::Divergent) || args.len() != 1 {
        return None;
    }
    let jump = match name.as_str() {
        "return" => Jump::Return,
        "break" => Jump::Break,
        "next" => Jump::Next,
        _ => return None,
    };
    Some((jump, &args[0], span))
}

/// The jump a statement *is*: an `ExprStmt` of a marker (see [`as_jump`]),
/// with the statement's span.
fn stmt_jump(stmt: &Stmt) -> Option<(Jump, &Expr, &Span)> {
    match stmt {
        Stmt::ExprStmt { expr, span } => as_jump(expr).map(|(jump, value, _)| (jump, value, span)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Scanning: which jumps does a piece of SIR contain?
// ---------------------------------------------------------------------------

/// The jumps found in a statement, block or expression.
///
/// `breaks` and `nexts` count only the ones that belong to a loop *around*
/// the scanned piece — a `break` inside a `while` that is itself inside the
/// piece ends that inner loop and goes no further.  A `return` always
/// leaves, however deep.
#[derive(Debug, Default, Clone, Copy)]
struct Exits {
    /// Any jump at all, however deeply nested.
    any: bool,
    returns: bool,
    breaks: bool,
    nexts: bool,
}

impl Exits {
    /// Can control leave the scanned piece early, skipping what follows it?
    fn leaves(self) -> bool {
        self.returns || self.breaks || self.nexts
    }
}

/// A read-only walk recording [`Exits`], counting loop nesting so an inner
/// loop's own `break`/`next` is not taken for one that leaves.
#[derive(Default)]
struct Scan {
    exits: Exits,
    loops: usize,
    first: Option<(Jump, Span)>,
}

impl Visitor for Scan {
    fn visit_stmt(&mut self, s: &Stmt, depth: usize) {
        let is_loop = matches!(
            s,
            Stmt::While { .. } | Stmt::ForEach { .. } | Stmt::ForRange { .. }
        );
        if is_loop {
            self.loops += 1;
        }
        walk_stmt_default(self, s, depth);
        if is_loop {
            self.loops -= 1;
        }
    }

    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        if let Some((jump, _, span)) = as_jump(e) {
            self.exits.any = true;
            if self.first.is_none() {
                self.first = Some((jump, span.clone()));
            }
            match jump {
                Jump::Return => self.exits.returns = true,
                Jump::Break if self.loops == 0 => self.exits.breaks = true,
                Jump::Next if self.loops == 0 => self.exits.nexts = true,
                _ => {}
            }
        }
        walk_expr_default(self, e, depth);
    }
}

fn scan_stmt(s: &Stmt) -> Exits {
    let mut scan = Scan::default();
    scan.visit_stmt(s, 0);
    scan.exits
}

fn scan_block(b: &Block) -> Exits {
    let mut scan = Scan::default();
    scan.visit_block(b, 0);
    scan.exits
}

fn scan_expr(e: &Expr) -> Exits {
    let mut scan = Scan::default();
    scan.visit_expr(e, 0);
    scan.exits
}

/// An expression with nothing to evaluate for: dropping it changes nothing.
fn is_inert(e: &Expr) -> bool {
    matches!(
        e,
        Expr::NilLit { .. }
            | Expr::BoolLit { .. }
            | Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::StrLit { .. }
            | Expr::SymLit { .. }
            | Expr::VarRef { .. }
    )
}

// ---------------------------------------------------------------------------
// The lowering
// ---------------------------------------------------------------------------

/// Lower every jump in a function `body` of kind `kind`, in place.
///
/// `fresh` mints a local name the source cannot spell (the lowerer's
/// `fresh_name`).  Returns whether anything changed — the caller then
/// declares the features the new locals and stores need.  A body with no
/// jump is left exactly as it was, so a program without `return`, `break`
/// or `next` lowers byte-for-byte as before.
pub(crate) fn lower_jumps(
    body: &mut Block,
    kind: Body,
    fresh: &mut dyn FnMut(&str) -> String,
) -> Result<bool, JumpError> {
    if !scan_block(body).any {
        return Ok(false);
    }
    let span = body.span.clone();
    let jump = fresh("__jump_");
    let ret = fresh("__ret_");
    let fresh = RefCell::new(fresh);
    let ctx = Ctx {
        kind,
        in_loop: false,
        jump: &jump,
        ret: &ret,
        fresh: &fresh,
    };

    // The body's own last value is the method's value when no `return`
    // ran, so it becomes one more statement, `__ret = <value>`, guarded
    // like any other.
    let value = std::mem::replace(&mut body.value, nil(&span));
    let mut list = std::mem::take(&mut body.stmts);
    list.extend(ctx.value_into(value, Some(&ret), &span, 0)?);
    let lowered = ctx.lower_list(list, 0)?;

    body.stmts = vec![
        Stmt::LetBinding {
            name: ret.clone(),
            sir_type: None,
            value: nil(&span),
            span: span.clone(),
        },
        Stmt::LetBinding {
            name: jump.clone(),
            sir_type: None,
            value: int(RUNNING, &span),
            span: span.clone(),
        },
    ];
    body.stmts.extend(lowered);
    body.value = local(&ret, &span);

    // Whatever is left was in a place a flag cannot reach: inside an
    // expression.  Refuse it by name rather than let a backend meet it.
    let mut scan = Scan::default();
    scan.visit_block(body, 0);
    if let Some((jump, span)) = scan.first {
        let kw = jump.keyword();
        return Err(JumpError {
            message: format!(
                "unsupported: `{kw}` inside an expression; only a `{kw}` that is a \
                 statement of its own, or the last statement of an `if` branch, can be lowered"
            ),
            span,
        });
    }
    Ok(true)
}

/// The lowerer's name minter, shared by every [`Ctx`] of one function.
type Fresh<'f> = RefCell<&'f mut dyn FnMut(&str) -> String>;

/// Where a list of statements sits: the function kind, whether a loop of
/// this function is around it, the two locals, and where to get a fresh
/// name (a `for` that becomes a `while` needs a counter and an end).
struct Ctx<'a, 'f> {
    kind: Body,
    in_loop: bool,
    jump: &'a str,
    ret: &'a str,
    fresh: &'a Fresh<'f>,
}

impl<'f> Ctx<'_, 'f> {
    /// The same context, inside a loop.
    fn in_a_loop(&self) -> Ctx<'_, 'f> {
        Ctx {
            kind: self.kind,
            in_loop: true,
            jump: self.jump,
            ret: self.ret,
            fresh: self.fresh,
        }
    }

    fn fresh(&self, prefix: &str) -> String {
        (self.fresh.borrow_mut())(prefix)
    }

    fn too_deep(span: &Span) -> JumpError {
        JumpError {
            message: format!(
                "unsupported: `return`/`break`/`next` nested more than {MAX_IR_DEPTH} \
                 statements deep"
            ),
            span: span.clone(),
        }
    }

    /// Lower a statement list.
    ///
    /// The list is cut into segments, each ending at a statement that jumps
    /// or may jump.  The first segment runs as it is; every later one goes
    /// behind its own `if __jump == 0`, side by side, not one inside the
    /// next — the flag only ever leaves `0` for the rest of the list, so a
    /// guard per segment skips exactly what a nested chain would:
    ///
    /// ```text
    /// s1; s2 (may jump)        s1; s2
    /// s3; s4 (may jump)   →    if __jump == 0 then s3; s4 end
    /// s5                       if __jump == 0 then s5 end
    /// ```
    ///
    /// So the depth of the result does not grow with the number of jumps
    /// (#18798 security review: a nested chain, built by recursing on the
    /// rest of the list and copying it once per jump, overflowed the stack
    /// on 5000 lines of `return 1`, and took seconds on 20 000 guard
    /// clauses).  This is one linear pass.
    ///
    /// A local first bound inside a guarded segment would be scoped to that
    /// guard's block, out of sight of the next one, so its `let` is hoisted:
    /// `let x = nil` before the first guard, and the original `let` becomes
    /// a store.  Ruby's locals are sequential, and nothing before the `let`
    /// could read the new local, so this changes no value.
    ///
    /// After a jump statement the guards are never true — those statements
    /// could not run in Ruby either — but they are kept, not dropped, so the
    /// lowered program still holds every statement the source wrote.
    fn lower_list(&self, stmts: Vec<Stmt>, depth: usize) -> Result<Vec<Stmt>, JumpError> {
        if depth >= MAX_IR_DEPTH {
            let span = stmts
                .first()
                .map_or_else(Span::synthetic, |s| s.span().clone());
            return Err(Self::too_deep(&span));
        }
        let mut out: Vec<Stmt> = Vec::new();
        let mut hoisted: Vec<Stmt> = Vec::new();
        let mut hoisted_names: HashSet<String> = HashSet::new();
        let mut guards: Vec<Stmt> = Vec::new();
        let mut current: Vec<Stmt> = Vec::new();
        let mut current_span: Option<Span> = None;
        let mut guarded = false;
        // Close the segment in `current`: the first one is emitted as it
        // is, every later one behind its own guard, its `let`s hoisted.
        let mut close = |current: &mut Vec<Stmt>,
                         span: Option<Span>,
                         guarded: bool,
                         out: &mut Vec<Stmt>,
                         guards: &mut Vec<Stmt>| {
            let segment = std::mem::take(current);
            if segment.is_empty() {
                return;
            }
            if !guarded {
                out.extend(segment);
                return;
            }
            let span = span.unwrap_or_else(Span::synthetic);
            let segment = segment
                .into_iter()
                .map(|stmt| match stmt {
                    Stmt::LetBinding {
                        name, value, span, ..
                    }
                    | Stmt::LetStarBinding {
                        name, value, span, ..
                    } => {
                        if hoisted_names.insert(name.clone()) {
                            hoisted.push(Stmt::LetBinding {
                                name: name.clone(),
                                sir_type: None,
                                value: nil(&span),
                                span: span.clone(),
                            });
                        }
                        self.store(&name, value, &span)
                    }
                    other => other,
                })
                .collect();
            guards.push(self.when_jump_is(RUNNING, segment, &span));
        };
        for stmt in stmts {
            let span = stmt.span().clone();
            let leaves = match stmt_jump(&stmt) {
                Some((jump, value, jump_span)) => {
                    let (value, jump_span) = (value.clone(), jump_span.clone());
                    current.extend(self.lower_jump(jump, value, &jump_span)?);
                    true
                }
                None => {
                    let leaves = scan_stmt(&stmt).leaves();
                    let level = depth + 1 + if guarded { GUARD_LEVELS } else { 0 };
                    current.extend(self.lower_stmt(stmt, level)?);
                    leaves
                }
            };
            current_span.get_or_insert(span);
            if leaves {
                close(
                    &mut current,
                    current_span.take(),
                    guarded,
                    &mut out,
                    &mut guards,
                );
                guarded = true;
            }
        }
        close(
            &mut current,
            current_span.take(),
            guarded,
            &mut out,
            &mut guards,
        );
        out.append(&mut hoisted);
        out.append(&mut guards);
        Ok(out)
    }

    /// One jump statement, as stores into the two locals.
    fn lower_jump(&self, jump: Jump, value: Expr, span: &Span) -> Result<Vec<Stmt>, JumpError> {
        let refuse = |message: &str| {
            Err(JumpError {
                message: message.to_string(),
                span: span.clone(),
            })
        };
        // A `return` from a block, or a block's `next` outside a loop.
        let returning = |value: Expr| {
            vec![
                self.store(self.ret, value, span),
                self.store(self.jump, int(RETURNING, span), span),
            ]
        };
        // A loop's `break`/`next`.  Ruby evaluates the value (it is the
        // value of the loop, which is a statement here), so a value with
        // an effect is kept as a statement.
        let looping = |value: Expr, flag: i64| {
            let mut out = Vec::new();
            if !is_inert(&value) {
                out.push(Stmt::ExprStmt {
                    expr: value,
                    span: span.clone(),
                });
            }
            out.push(self.store(self.jump, int(flag, span), span));
            out
        };
        match jump {
            Jump::Return if self.kind == Body::Block => refuse(
                "unsupported: `return` inside a block returns from the method around \
                 the block, which is not lowered yet (`next` leaves the block)",
            ),
            Jump::Return => Ok(returning(value)),
            Jump::Next if self.in_loop => Ok(looping(value, NEXTING)),
            Jump::Next if matches!(self.kind, Body::Block | Body::Lambda) => Ok(returning(value)),
            Jump::Next => refuse("`next` outside a loop or a block"),
            Jump::Break if self.in_loop => Ok(looping(value, BREAKING)),
            Jump::Break if self.kind == Body::Block => refuse(
                "unsupported: `break` inside a block ends the call that took the block, \
                 which is not lowered yet",
            ),
            Jump::Break => refuse("`break` outside a loop"),
        }
    }

    /// Lower one statement that is not itself a jump.  Only the statements
    /// that hold other statements can hold a lowerable jump; any other
    /// statement is returned unchanged (a jump inside it is reported by
    /// [`lower_jumps`]'s final scan).
    fn lower_stmt(&self, stmt: Stmt, depth: usize) -> Result<Vec<Stmt>, JumpError> {
        if depth >= MAX_IR_DEPTH {
            return Err(Self::too_deep(stmt.span()));
        }
        if !scan_stmt(&stmt).any {
            return Ok(vec![stmt]);
        }
        match stmt {
            Stmt::ExprStmt {
                expr:
                    Expr::If {
                        cond,
                        then_branch,
                        else_branch,
                        span: if_span,
                    },
                span,
            } if !scan_expr(&cond).any => Ok(vec![Stmt::ExprStmt {
                expr: Expr::If {
                    cond,
                    then_branch: Box::new(self.lower_branch(*then_branch, depth)?),
                    else_branch: Box::new(self.lower_branch(*else_branch, depth)?),
                    span: if_span,
                },
                span,
            }]),
            Stmt::While { cond, body, span } if !scan_expr(&cond).any => {
                let exits = scan_block(&body);
                let mut new_body = self.lower_loop_body(body, depth)?;
                if exits.nexts {
                    new_body
                        .stmts
                        .push(self.reset_after(NEXTING, &new_body.span.clone()));
                }
                // A `break` or `return` must stop the loop: test the flag
                // before the condition, which then is not evaluated.
                let cond = if exits.breaks || exits.returns {
                    self.while_running(cond, &span)
                } else {
                    cond
                };
                let mut out = vec![Stmt::While {
                    cond,
                    body: new_body,
                    span: span.clone(),
                }];
                if exits.breaks {
                    out.push(self.reset_after(BREAKING, &span));
                }
                Ok(out)
            }
            Stmt::ForEach {
                var,
                iter,
                body,
                span,
            } if !scan_expr(&iter).any => {
                let exits = scan_block(&body);
                let body = self.lower_counted_body(body, exits, depth)?;
                let mut out = vec![Stmt::ForEach {
                    var,
                    iter,
                    body,
                    span: span.clone(),
                }];
                if exits.breaks {
                    out.push(self.reset_after(BREAKING, &span));
                }
                Ok(out)
            }
            Stmt::ForRange {
                var,
                start,
                stop,
                step,
                body,
                span,
            } if !(scan_expr(&start).any || scan_expr(&stop).any || scan_expr(&step).any) => {
                let exits = scan_block(&body);
                if exits.breaks || exits.returns {
                    return self.lower_range_as_while(var, start, stop, step, body, span, depth);
                }
                let body = self.lower_counted_body(body, exits, depth)?;
                Ok(vec![Stmt::ForRange {
                    var,
                    start,
                    stop,
                    step,
                    body,
                    span,
                }])
            }
            // A jump inside `begin` leaves through it like any statement:
            // it is no exception, so no `rescue` sees it, and the `ensure`
            // statements (unguarded, as they come after the body) still run.
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                span,
            } => {
                let body = self.lower_list(body, depth)?;
                let mut lowered_rescues = Vec::with_capacity(rescues.len());
                for mut clause in rescues {
                    clause.body = self.lower_list(clause.body, depth)?;
                    lowered_rescues.push(clause);
                }
                let ensure_body = match ensure_body {
                    Some(stmts) => Some(self.lower_list(stmts, depth)?),
                    None => None,
                };
                Ok(vec![Stmt::TryCatch {
                    body,
                    rescues: lowered_rescues,
                    ensure_body,
                    span,
                }])
            }
            other => Ok(vec![other]),
        }
    }

    /// An `if` branch whose value is thrown away (the `if` is a statement):
    /// a value that jumps, or that would run after a jump in the branch,
    /// moves into the statements so the guards cover it.
    fn lower_branch(&self, branch: Block, depth: usize) -> Result<Block, JumpError> {
        let Block { stmts, value, span } = branch;
        let mut list = stmts;
        let value = if scan_expr(&value).any || list.iter().any(|s| scan_stmt(s).leaves()) {
            list.extend(self.value_into(value, None, &span, depth)?);
            nil(&span)
        } else {
            value
        };
        Ok(Block {
            stmts: self.lower_list(list, depth)?,
            value,
            span,
        })
    }

    /// A `while` body, in the loop's context.  (A loop body's value is
    /// already `nil`: see `loop_body` in the lowerer.)
    fn lower_loop_body(&self, body: Block, depth: usize) -> Result<Block, JumpError> {
        let Block { stmts, value, span } = body;
        let mut list = stmts;
        list.extend(self.value_into(value, None, &span, depth)?);
        Ok(Block {
            stmts: self.in_a_loop().lower_list(list, depth)?,
            value: nil(&span),
            span,
        })
    }

    /// A `for` body (`ForEach`, or a `ForRange` that only uses `next`).
    /// A `ForEach` has no condition to test, so after a `break` or `return`
    /// its remaining trips still happen — with the whole body, the store
    /// into the loop variable included, behind `if __jump == 0`, so they do
    /// nothing; there are only as many as its collection has elements.
    fn lower_counted_body(
        &self,
        body: Block,
        exits: Exits,
        depth: usize,
    ) -> Result<Block, JumpError> {
        let mut body = self.lower_loop_body(body, depth)?;
        let span = body.span.clone();
        if exits.breaks || exits.returns {
            let stmts = std::mem::take(&mut body.stmts);
            body.stmts = vec![self.when_jump_is(RUNNING, stmts, &span)];
        }
        if exits.nexts {
            body.stmts.push(self.reset_after(NEXTING, &span));
        }
        Ok(body)
    }

    /// A `ForRange` that can `break` or `return`, as a `while` that tests
    /// the flag, so the loop ends when it is left — a `ForRange` has no
    /// condition to test, and running out its remaining trips with the body
    /// disabled could take for ever (`for i in 0...2**62 do break end`):
    ///
    /// ```text
    /// let <ctr>  = start          // evaluated once, in this order, as
    /// let <stop> = stop           //   the ForRange evaluates them
    /// let <step> = step           // (only when `step` is not a literal)
    /// while (__jump == 0 ? <ctr> < <stop> : false)   // `>` for a negative step
    ///   let var = <ctr>
    ///   <ctr> = <ctr> + step      // before the body, so `next` cannot skip it
    ///   …body…
    /// ```
    #[allow(clippy::too_many_arguments)]
    fn lower_range_as_while(
        &self,
        var: String,
        start: Expr,
        stop: Expr,
        step: Expr,
        body: Block,
        span: Span,
        depth: usize,
    ) -> Result<Vec<Stmt>, JumpError> {
        let exits = scan_block(&body);
        let ctr = self.fresh("__for_counter_");
        let end = self.fresh("__for_end_");
        let let_ = |name: &str, value: Expr| Stmt::LetBinding {
            name: name.to_string(),
            sir_type: None,
            value,
            span: span.clone(),
        };
        let call = |name: &str, a: Expr, b: Expr| Expr::BuiltinCall {
            name: name.to_string(),
            args: vec![a, b],
            effects: EffectSet::PURE,
            span: span.clone(),
        };
        let mut out = vec![let_(&ctr, start), let_(&end, stop)];
        let (step, compare) = match step {
            Expr::IntLit { value, .. } => {
                let op = if value < 0 { ">" } else { "<" };
                (
                    int(value, &span),
                    call(op, local(&ctr, &span), local(&end, &span)),
                )
            }
            other => {
                let step_local = self.fresh("__for_step_");
                out.push(let_(&step_local, other));
                let ascending = call(">", local(&step_local, &span), int(0, &span));
                let compare = Expr::If {
                    cond: Box::new(ascending),
                    then_branch: Box::new(Block {
                        stmts: Vec::new(),
                        value: call("<", local(&ctr, &span), local(&end, &span)),
                        span: span.clone(),
                    }),
                    else_branch: Box::new(Block {
                        stmts: Vec::new(),
                        value: call(">", local(&ctr, &span), local(&end, &span)),
                        span: span.clone(),
                    }),
                    span: span.clone(),
                };
                (local(&step_local, &span), compare)
            }
        };
        let mut new_body = self.lower_loop_body(body, depth)?;
        let mut stmts = vec![
            let_(&var, local(&ctr, &span)),
            self.store(&ctr, call("+", local(&ctr, &span), step), &span),
        ];
        stmts.append(&mut new_body.stmts);
        if exits.nexts {
            stmts.push(self.reset_after(NEXTING, &span));
        }
        new_body.stmts = stmts;
        out.push(Stmt::While {
            cond: self.while_running(compare, &span),
            body: new_body,
            span: span.clone(),
        });
        if exits.breaks {
            out.push(self.reset_after(BREAKING, &span));
        }
        Ok(out)
    }

    /// `__jump == 0 ? cond : false` — a loop condition that is false once
    /// the loop has been left.
    fn while_running(&self, cond: Expr, span: &Span) -> Expr {
        Expr::If {
            cond: Box::new(self.jump_is(RUNNING, span)),
            then_branch: Box::new(Block {
                stmts: Vec::new(),
                value: cond,
                span: span.clone(),
            }),
            else_branch: Box::new(Block {
                stmts: Vec::new(),
                value: Expr::BoolLit {
                    value: false,
                    span: span.clone(),
                },
                span: span.clone(),
            }),
            span: span.clone(),
        }
    }

    /// Statements that put `value` into `target` (or, with no target, run
    /// it for its effect), with a jump in tail position kept as a jump
    /// statement so [`Self::lower_list`] lowers it.
    ///
    /// | `value`                         | statements                                |
    /// |---------------------------------|-------------------------------------------|
    /// | no jump in it                   | `target = value` (or `value`, if it does anything) |
    /// | a jump                          | the jump                                  |
    /// | `if c then a else b end`        | `if c then (a into target) else (b into target) end` |
    /// | anything else holding a jump    | `target = value`, left for the final scan to refuse |
    fn value_into(
        &self,
        value: Expr,
        target: Option<&str>,
        span: &Span,
        depth: usize,
    ) -> Result<Vec<Stmt>, JumpError> {
        if depth >= MAX_IR_DEPTH {
            return Err(Self::too_deep(span));
        }
        let plain = |value: Expr| match target {
            Some(t) => vec![self.store(t, value, span)],
            None if is_inert(&value) => Vec::new(),
            None => vec![Stmt::ExprStmt {
                expr: value,
                span: span.clone(),
            }],
        };
        if !scan_expr(&value).any {
            return Ok(plain(value));
        }
        if as_jump(&value).is_some() {
            return Ok(vec![Stmt::ExprStmt {
                expr: value,
                span: span.clone(),
            }]);
        }
        match value {
            Expr::If {
                cond,
                then_branch,
                else_branch,
                span: if_span,
            } if !scan_expr(&cond).any => {
                let into = |b: Block| -> Result<Block, JumpError> {
                    let Block { stmts, value, span } = b;
                    let mut stmts = stmts;
                    stmts.extend(self.value_into(value, target, &span, depth + GUARD_LEVELS)?);
                    Ok(Block {
                        stmts,
                        value: nil(&span),
                        span,
                    })
                };
                Ok(vec![Stmt::ExprStmt {
                    expr: Expr::If {
                        cond,
                        then_branch: Box::new(into(*then_branch)?),
                        else_branch: Box::new(into(*else_branch)?),
                        span: if_span,
                    },
                    span: span.clone(),
                }])
            }
            other => Ok(plain(other)),
        }
    }

    // ── small builders ──────────────────────────────────────────────────

    fn store(&self, name: &str, value: Expr, span: &Span) -> Stmt {
        Stmt::Assign {
            name: name.to_string(),
            scope: Scope::Local,
            value,
            span: span.clone(),
        }
    }

    /// `__jump == flag`.
    fn jump_is(&self, flag: i64, span: &Span) -> Expr {
        Expr::BuiltinCall {
            name: "==".to_string(),
            args: vec![local(self.jump, span), int(flag, span)],
            effects: EffectSet::PURE,
            span: span.clone(),
        }
    }

    /// `if __jump == flag then stmts end`, as a statement.
    fn when_jump_is(&self, flag: i64, stmts: Vec<Stmt>, span: &Span) -> Stmt {
        Stmt::ExprStmt {
            expr: Expr::If {
                cond: Box::new(self.jump_is(flag, span)),
                then_branch: Box::new(Block {
                    stmts,
                    value: nil(span),
                    span: span.clone(),
                }),
                else_branch: Box::new(Block {
                    stmts: Vec::new(),
                    value: nil(span),
                    span: span.clone(),
                }),
                span: span.clone(),
            },
            span: span.clone(),
        }
    }

    /// `if __jump == flag then __jump = 0 end`: the jump `flag` ends here.
    fn reset_after(&self, flag: i64, span: &Span) -> Stmt {
        let reset = self.store(self.jump, int(RUNNING, span), span);
        self.when_jump_is(flag, vec![reset], span)
    }
}

fn nil(span: &Span) -> Expr {
    Expr::NilLit { span: span.clone() }
}

fn int(value: i64, span: &Span) -> Expr {
    Expr::IntLit {
        value,
        span: span.clone(),
    }
}

fn local(name: &str, span: &Span) -> Expr {
    Expr::VarRef {
        name: name.to_string(),
        scope: Scope::Local,
        span: span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span::new("t", 3, 5, 3, 9)
    }

    fn jump(name: &str, value: Expr) -> Expr {
        Expr::BuiltinCall {
            name: name.to_string(),
            args: vec![value],
            effects: EffectSet::PURE.with(Effect::Divergent),
            span: span(),
        }
    }

    fn fresh() -> impl FnMut(&str) -> String {
        let mut n = 0;
        move |prefix: &str| {
            n += 1;
            format!("{prefix}{n}")
        }
    }

    /// A body with no jump is not touched at all.
    #[test]
    fn a_body_without_a_jump_is_unchanged() {
        let mut body = Block {
            stmts: vec![Stmt::ExprStmt {
                expr: int(1, &span()),
                span: span(),
            }],
            value: int(2, &span()),
            span: span(),
        };
        let before = body.clone();
        assert!(!lower_jumps(&mut body, Body::Method, &mut fresh()).unwrap());
        assert_eq!(body, before);
    }

    /// A jump the grammar cannot produce today, inside an expression, is
    /// refused with its own position rather than left for a backend.
    #[test]
    fn a_jump_inside_an_expression_is_refused_at_its_position() {
        let mut body = Block {
            stmts: vec![Stmt::LetBinding {
                name: "x".to_string(),
                sir_type: None,
                value: jump("return", int(1, &span())),
                span: span(),
            }],
            value: nil(&span()),
            span: Span::synthetic(),
        };
        let err = lower_jumps(&mut body, Body::Method, &mut fresh()).unwrap_err();
        assert!(
            err.message.contains("`return` inside an expression"),
            "{}",
            err.message
        );
        assert_eq!((err.span.start_line, err.span.start_col), (3, 5));
    }

    /// A user method named `next` is a call, never a jump: only the
    /// lowerer's `Divergent` marker is one.
    #[test]
    fn only_the_divergent_marker_is_a_jump() {
        let call = Expr::BuiltinCall {
            name: "next".to_string(),
            args: vec![nil(&span())],
            effects: EffectSet::PURE,
            span: span(),
        };
        assert!(as_jump(&call).is_none());
        assert!(as_jump(&jump("next", nil(&span()))).is_some());
        assert!(as_jump(&jump("redo", nil(&span()))).is_none());
    }

    /// The depth guard turns a nest past [`MAX_IR_DEPTH`] into an error,
    /// not a stack overflow.  (The scans stop at that depth too, as every
    /// SIR walk does, and the validator refuses such a module anyway.)
    #[test]
    fn a_too_deep_nest_is_an_error() {
        let stmt = Stmt::TryCatch {
            body: vec![Stmt::ExprStmt {
                expr: jump("return", nil(&span())),
                span: span(),
            }],
            rescues: Vec::new(),
            ensure_body: None,
            span: span(),
        };
        let mut mint = fresh();
        let mint: &mut dyn FnMut(&str) -> String = &mut mint;
        let cell = RefCell::new(mint);
        let ctx = Ctx {
            kind: Body::Method,
            in_loop: false,
            jump: "j",
            ret: "r",
            fresh: &cell,
        };
        let err = ctx.lower_stmt(stmt, MAX_IR_DEPTH).unwrap_err();
        assert!(err.message.contains("more than"), "{}", err.message);
    }
}

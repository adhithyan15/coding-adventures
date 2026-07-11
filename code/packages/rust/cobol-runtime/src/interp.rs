//! The interpreter: build the PICTURE-typed data model from WORKING-STORAGE and
//! execute the PROCEDURE DIVISION, capturing everything `DISPLAY`ed.

use crate::error::RuntimeError;
use crate::picture::Picture;
use crate::program::{ArithOp, Cond, Expr, Fig, Lit, Operand, Paragraph, Program, RelOp, Stmt};
use crate::value::{add, div, move_into_char, move_into_numeric, mul, pow, round, sub, Decimal};
use std::collections::HashMap;

/// Deepest chain of nested/recursive `PERFORM`s before we bail out with a clean
/// error. A paragraph that performs itself (directly or in a cycle) would
/// otherwise recurse until the native stack overflows — an uncatchable abort.
/// Each level spends several (debug-sized) frames — `exec_perform` →
/// `run_stmts` → `exec_stmt` → `exec_perform` — so the cap sits well under the
/// ~200-level overflow point of a default 2 MiB worker/test stack. Real programs
/// never nest `PERFORM` anywhere near this deep.
const MAX_PERFORM_DEPTH: usize = 100;

/// Fractional precision COMPUTE carries through an intermediate **division**
/// before the final round/truncate into the receiver. COBOL's standard defines
/// an intricate composite intermediate precision; we use a fixed generous scale
/// here (correct to this many places, then rounded to the receiver). This is a
/// documented simplification — see PL08 — not the full standard rule. 12 places
/// stays comfortably inside `i128` for realistic magnitudes.
const COMPUTE_DIV_SCALE: usize = 12;

/// One field in the data model. Elementary items carry a picture and character
/// storage; group items (no picture) are the concatenation of their children.
struct Item {
    level: u32,
    picture: Option<Picture>,
    storage: String,
    /// The operational sign of a signed numeric item (`PIC S9…`). The `storage`
    /// digits are always the magnitude; this carries the sign separately. Always
    /// `false` for unsigned and non-numeric items (they drop any sign).
    neg: bool,
    children: Vec<usize>,
}

/// A source value in flight during a `MOVE` or `DISPLAY`.
enum Src {
    Num(Decimal),
    Chars(String),
    Fig(Fig),
}

/// Turn an arithmetic result into a `Result`, reporting `i128` overflow (a value
/// beyond ~38 digits — larger than any real COBOL numeric field) as an error
/// rather than panicking or wrapping.
fn checked(r: Option<Decimal>) -> Result<Decimal, RuntimeError> {
    r.ok_or_else(|| RuntimeError::Unsupported("arithmetic overflow (result exceeds ~38 digits)".into()))
}

/// The character form of a source value for an alphanumeric comparison (a
/// figurative yields `""` here — it is expanded to the other operand's length
/// by the caller).
fn src_chars(src: &Src) -> String {
    match src {
        Src::Chars(s) => s.clone(),
        Src::Num(d) => d.digits(),
        Src::Fig(_) => String::new(),
    }
}

/// Expand a figurative constant to `n` characters.
fn fill_fig(f: &Fig, n: usize) -> String {
    match f {
        Fig::Zero => "0".repeat(n),
        Fig::Space => " ".repeat(n),
    }
}

/// Overpunch the sign onto the trailing (units) digit of a magnitude digit
/// string — the ASCII "zoned decimal" encoding COBOL uses to `DISPLAY` a signed
/// numeric-display field under the default `SIGN IS TRAILING`. The units digit
/// `d` becomes:
///
/// | d | 0 1 2 3 4 5 6 7 8 9 |
/// |---|---------------------|
/// | + | { A B C D E F G H I |
/// | − | } J K L M N O P Q R |
///
/// So `+123` → `12C` and `−123` → `12L`. A non-digit units position (there is
/// none for a real numeric field) is passed through unchanged.
fn overpunch_trailing(magnitude: &str, neg: bool) -> String {
    const POS: [char; 10] = ['{', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I'];
    const NEG: [char; 10] = ['}', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R'];
    let mut chars: Vec<char> = magnitude.chars().collect();
    if let Some(last) = chars.last_mut() {
        if let Some(d) = last.to_digit(10) {
            *last = if neg { NEG[d as usize] } else { POS[d as usize] };
        }
    }
    chars.into_iter().collect()
}

/// The running machine: the item table, a name→index map, the procedure's
/// paragraphs (with a name→index map for `PERFORM` targets), and captured output.
pub struct Machine {
    items: Vec<Item>,
    by_name: HashMap<String, usize>,
    paragraphs: Vec<Paragraph>,
    para_index: HashMap<String, usize>,
    /// Current `PERFORM` nesting depth, bounded by [`MAX_PERFORM_DEPTH`].
    perform_depth: usize,
    output: String,
}

impl Machine {
    /// Build the data model from a program's WORKING-STORAGE and initialise it.
    pub fn new(program: &Program) -> Result<Machine, RuntimeError> {
        // Index paragraphs by name for PERFORM lookup. A duplicated name keeps
        // its first occurrence (a PERFORM of an ambiguous name is unusual; the
        // first definition wins rather than erroring at build time).
        let mut para_index = HashMap::new();
        for (i, p) in program.paragraphs.iter().enumerate() {
            para_index.entry(p.name.clone()).or_insert(i);
        }
        let mut m = Machine {
            items: Vec::new(),
            by_name: HashMap::new(),
            paragraphs: program.paragraphs.clone(),
            para_index,
            perform_depth: 0,
            output: String::new(),
        };
        m.build_items(program)?;
        Ok(m)
    }

    fn build_items(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // `stack` holds indices of currently-open group ancestors.
        let mut stack: Vec<usize> = Vec::new();

        for def in &program.data {
            // Only the hierarchy levels 01–49 and the standalone 77 are modelled
            // in v0.1. Rejecting anything else is faithful COBOL (66/88 are
            // deferred features; 50+ are invalid) and bounds the item-tree depth
            // to ≤ 49, so `group_image` recursion can never overflow the stack.
            if !(1..=49).contains(&def.level) && def.level != 77 {
                return Err(RuntimeError::Unsupported(format!(
                    "level number {:02} (v0.1 supports 01–49 and 77)",
                    def.level
                )));
            }

            let picture = match &def.picture {
                Some(p) => Some(Picture::parse(p)?),
                None => None,
            };
            // Default initial content: zeros for numeric, spaces for character.
            let storage = match &picture {
                Some(p) if p.is_numeric() => "0".repeat(p.size()),
                Some(p) => " ".repeat(p.size()),
                None => String::new(),
            };
            let idx = self.items.len();
            self.items.push(Item {
                level: def.level,
                picture,
                storage,
                neg: false,
                children: Vec::new(),
            });

            // Register the name (duplicates need qualification — not yet supported).
            if let Some(name) = &def.name {
                if self.by_name.insert(name.clone(), idx).is_some() {
                    return Err(RuntimeError::DuplicateName(name.clone()));
                }
            }

            // Attach into the level hierarchy. 01 and 77 are top-level; 77 never
            // parents subordinates; 02–49 attach to the nearest shallower group.
            if def.level == 1 || def.level == 77 {
                stack.clear();
                if def.level == 1 {
                    stack.push(idx);
                }
            } else {
                while let Some(&top) = stack.last() {
                    if self.items[top].level >= def.level {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                if let Some(&parent) = stack.last() {
                    self.items[parent].children.push(idx);
                }
                stack.push(idx);
            }

            // Apply a VALUE clause as an initialising MOVE.
            if let Some(lit) = &def.value {
                let src = self.src_from_lit(lit)?;
                self.move_into(idx, src)?;
            }
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Execution
    // ----------------------------------------------------------------------

    /// Run the procedure division and return the captured console output.
    pub fn run(mut self, _program: &Program) -> Result<String, RuntimeError> {
        // Execute paragraphs top to bottom (COBOL falls through paragraph
        // boundaries). We clone each paragraph's statements to run them, since
        // executing borrows `self` mutably while the paragraphs live in `self`.
        let count = self.paragraphs.len();
        for i in 0..count {
            let stmts = self.paragraphs[i].stmts.clone();
            if self.run_stmts(&stmts)? {
                break; // STOP RUN
            }
        }
        // Falling off the end of the procedure division ends the run too.
        Ok(self.output)
    }

    /// Execute one statement. Returns `true` if it requested `STOP RUN` (which
    /// unwinds nested `IF` branches and ends the program).
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<bool, RuntimeError> {
        match stmt {
            Stmt::StopRun => return Ok(true),
            Stmt::Display(ops) => self.exec_display(ops)?,
            Stmt::Move { src, dsts } => self.exec_move(src, dsts)?,
            Stmt::Add { operands, to, giving } => self.exec_add(operands, to, giving)?,
            Stmt::Subtract { operands, from, giving } => self.exec_subtract(operands, from, giving)?,
            Stmt::Multiply { a, by, giving } => self.exec_multiply(a, by, giving)?,
            Stmt::Divide { divisor, dividend, giving } => {
                self.exec_divide(divisor, dividend, giving)?
            }
            Stmt::Compute { target, rounded, expr, on_size_error } => {
                return self.exec_compute(target, *rounded, expr, on_size_error);
            }
            Stmt::Perform { target, times } => return self.exec_perform(target, times),
            Stmt::If { cond, then_branch, else_branch } => {
                let branch = if self.eval_cond(cond)? { then_branch } else { else_branch };
                return self.run_stmts(branch);
            }
        }
        Ok(false)
    }

    /// Execute a sequence of statements, short-circuiting on `STOP RUN` (the
    /// returned `true` propagates up to unwind any enclosing branches). Shared by
    /// `IF` branches and `COMPUTE … ON SIZE ERROR` handlers.
    fn run_stmts(&mut self, stmts: &[Stmt]) -> Result<bool, RuntimeError> {
        for s in stmts {
            if self.exec_stmt(s)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `PERFORM target [n TIMES]` — run one paragraph out of line, `n` times
    /// (default 1), then return. A `TIMES` count that is zero or negative runs
    /// the paragraph zero times (COBOL's rule); a fractional count truncates.
    /// Nesting is bounded by [`MAX_PERFORM_DEPTH`] so a self-performing paragraph
    /// fails with a clean error instead of overflowing the stack. Returns `true`
    /// if a `STOP RUN` fired inside.
    fn exec_perform(&mut self, target: &str, times: &Option<Operand>) -> Result<bool, RuntimeError> {
        let idx = *self
            .para_index
            .get(target)
            .ok_or_else(|| RuntimeError::UndefinedName(target.into()))?;

        // Resolve the repeat count: a non-negative integer; ≤ 0 → zero times.
        let n: usize = match times {
            None => 1,
            Some(op) => {
                let d = self.operand_decimal(op)?;
                if d.neg {
                    0
                } else {
                    let int = d.int.trim_start_matches('0');
                    if int.is_empty() {
                        0
                    } else {
                        int.parse::<usize>().map_err(|_| {
                            RuntimeError::Unsupported("PERFORM … TIMES count is too large".into())
                        })?
                    }
                }
            }
        };

        self.perform_depth += 1;
        if self.perform_depth > MAX_PERFORM_DEPTH {
            self.perform_depth -= 1;
            return Err(RuntimeError::Unsupported(
                "PERFORM nesting too deep (a paragraph performing itself?)".into(),
            ));
        }
        let stmts = self.paragraphs[idx].stmts.clone();
        let mut stop = false;
        for _ in 0..n {
            if self.run_stmts(&stmts)? {
                stop = true;
                break;
            }
        }
        self.perform_depth -= 1;
        Ok(stop)
    }

    /// Evaluate a relational condition. Numeric when both sides are numeric;
    /// otherwise an alphanumeric (space-padded) character comparison — COBOL's
    /// rule. Figurative constants take the category/length of the other operand.
    fn eval_cond(&self, cond: &Cond) -> Result<bool, RuntimeError> {
        use std::cmp::Ordering;
        let l = self.src_from_operand(&cond.left)?;
        let r = self.src_from_operand(&cond.right)?;

        let ordering = match (&l, &r) {
            (Src::Num(a), Src::Num(b)) => a.cmp_value(b),
            (Src::Num(a), Src::Fig(Fig::Zero)) => a.cmp_value(&Decimal::zero()),
            (Src::Fig(Fig::Zero), Src::Num(b)) => Decimal::zero().cmp_value(b),
            _ => {
                // Alphanumeric comparison: build each side's characters, expand a
                // figurative to the other operand's length, then space-pad both
                // to equal length and compare.
                let mut ls = src_chars(&l);
                let mut rs = src_chars(&r);
                if let Src::Fig(f) = &l {
                    ls = fill_fig(f, rs.len().max(1));
                }
                if let Src::Fig(f) = &r {
                    rs = fill_fig(f, ls.len().max(1));
                }
                let width = ls.len().max(rs.len());
                let lp = format!("{ls:<width$}");
                let rp = format!("{rs:<width$}");
                lp.cmp(&rp)
            }
        };

        let base = match cond.op {
            RelOp::Greater => ordering == Ordering::Greater,
            RelOp::Less => ordering == Ordering::Less,
            RelOp::Equal => ordering == Ordering::Equal,
        };
        Ok(base ^ cond.negated)
    }

    fn exec_display(&mut self, ops: &[Operand]) -> Result<(), RuntimeError> {
        let mut line = String::new();
        for op in ops {
            line.push_str(&self.display_image(op)?);
        }
        self.output.push_str(&line);
        self.output.push('\n');
        Ok(())
    }

    fn exec_move(&mut self, src: &Operand, dsts: &[String]) -> Result<(), RuntimeError> {
        for dst in dsts {
            // Resolve the source afresh per receiver (its category can differ).
            let value = self.src_from_operand(src)?;
            let idx = *self.by_name.get(dst).ok_or_else(|| RuntimeError::UndefinedName(dst.clone()))?;
            self.move_into(idx, value)?;
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Arithmetic (fixed-point decimal, truncating; unsigned receivers)
    // ----------------------------------------------------------------------

    /// `ADD op… TO name [GIVING g]` → (name + op1 + … + opN) into g or name.
    fn exec_add(
        &mut self,
        operands: &[Operand],
        to: &str,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let mut acc = self.named_decimal(to)?;
        for op in operands {
            acc = checked(add(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_number(giving.as_deref().unwrap_or(to), acc)
    }

    /// `SUBTRACT op… FROM name [GIVING g]` → (name − op1 − … − opN) into g or name.
    fn exec_subtract(
        &mut self,
        operands: &[Operand],
        from: &str,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let mut acc = self.named_decimal(from)?;
        for op in operands {
            acc = checked(sub(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_number(giving.as_deref().unwrap_or(from), acc)
    }

    /// `MULTIPLY a BY b [GIVING g]` → (a × b) into g, or into b when no GIVING.
    fn exec_multiply(
        &mut self,
        a: &Operand,
        by: &Operand,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let product = checked(mul(&self.operand_decimal(a)?, &self.operand_decimal(by)?))?;
        let target = match (giving, by) {
            (Some(g), _) => g.clone(),
            (None, Operand::Ident(name)) => name.clone(),
            (None, _) => {
                return Err(RuntimeError::Unsupported(
                    "MULTIPLY … BY <literal> without GIVING has no receiver".into(),
                ))
            }
        };
        self.store_number(&target, product)
    }

    /// `DIVIDE a INTO b [GIVING g]` → (b ÷ a), truncated to the receiver's
    /// decimal places, stored in g, or in b (the dividend) when no GIVING.
    fn exec_divide(
        &mut self,
        divisor: &Operand,
        dividend: &Operand,
        giving: &Option<String>,
    ) -> Result<(), RuntimeError> {
        let d = self.operand_decimal(divisor)?;
        if d.is_zero() {
            return Err(RuntimeError::DivideByZero);
        }
        let n = self.operand_decimal(dividend)?;
        let target = match (giving, dividend) {
            (Some(g), _) => g.clone(),
            (None, Operand::Ident(name)) => name.clone(),
            (None, _) => {
                return Err(RuntimeError::Unsupported(
                    "DIVIDE … INTO <literal> without GIVING has no receiver".into(),
                ))
            }
        };
        // Compute to the receiver's fractional precision (COBOL truncates there
        // absent ROUNDED); store_number then aligns the integer part.
        let scale = self.numeric_dec_digits(&target)?;
        let quotient = checked(div(&n, &d, scale))?;
        self.store_number(&target, quotient)
    }

    /// The number of fractional digit positions of a named numeric receiver.
    fn numeric_dec_digits(&self, name: &str) -> Result<usize, RuntimeError> {
        Ok(self.numeric_dims(name)?.1)
    }

    /// The `(int_digits, dec_digits)` of a named numeric receiver.
    fn numeric_dims(&self, name: &str) -> Result<(usize, usize), RuntimeError> {
        let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.into()))?;
        match &self.items[idx].picture {
            Some(Picture::Numeric { int_digits, dec_digits, .. }) => Ok((*int_digits, *dec_digits)),
            _ => Err(RuntimeError::Unsupported(format!("arithmetic on non-numeric field {name}"))),
        }
    }

    // ----------------------------------------------------------------------
    // COMPUTE — expression evaluation, ROUNDED, ON SIZE ERROR
    // ----------------------------------------------------------------------

    /// `COMPUTE target [ROUNDED] = expr [ON SIZE ERROR …]`.
    ///
    /// Evaluate the expression, round or truncate to the receiver's decimal
    /// places, and store it — unless the result's integer part overflows the
    /// receiver (or a division by zero occurred). On such a **size error**: run
    /// the `ON SIZE ERROR` statements and leave the receiver unchanged if a
    /// handler was given; otherwise fall back to COBOL's handler-less behaviour
    /// (overflow truncates silently like `MOVE`; a zero divisor is a hard error).
    fn exec_compute(
        &mut self,
        target: &str,
        rounded: bool,
        expr: &Expr,
        on_size_error: &[Stmt],
    ) -> Result<bool, RuntimeError> {
        let (int_digits, dec_digits) = self.numeric_dims(target)?;

        let value = match self.eval_expr(expr) {
            Ok(v) => v,
            // Division by zero is a size-error condition: the handler catches it;
            // without one it stays a hard DivideByZero (as bare DIVIDE does).
            Err(RuntimeError::DivideByZero) if !on_size_error.is_empty() => {
                return self.run_stmts(on_size_error);
            }
            Err(e) => return Err(e),
        };

        // Round (half away from zero) or leave full precision for the store to
        // truncate at the receiver's decimal places.
        let final_value = if rounded {
            checked(round(&value, dec_digits))?
        } else {
            value
        };

        // Size error = the integer part does not fit. (Fractional truncation is
        // never a size error.)
        if final_value.int.trim_start_matches('0').len() > int_digits {
            if on_size_error.is_empty() {
                // No handler: COBOL truncates the high-order digits silently,
                // exactly as move_into_numeric already does.
                self.store_number(target, final_value)?;
                return Ok(false);
            }
            return self.run_stmts(on_size_error);
        }

        self.store_number(target, final_value)?;
        Ok(false)
    }

    /// Evaluate an arithmetic expression to an exact [`Decimal`]. Division is
    /// carried to [`COMPUTE_DIV_SCALE`] fractional digits; a zero divisor is a
    /// [`RuntimeError::DivideByZero`] (which `COMPUTE`'s caller may turn into a
    /// size error). Names must resolve to numeric items.
    fn eval_expr(&self, e: &Expr) -> Result<Decimal, RuntimeError> {
        match e {
            Expr::Num(s) => Decimal::parse_literal(s)
                .ok_or_else(|| RuntimeError::Unsupported(format!("numeric literal {s}"))),
            Expr::Var(name) => self.named_decimal(name),
            Expr::Unary { neg, operand } => {
                let mut d = self.eval_expr(operand)?;
                if *neg && !d.is_zero() {
                    d.neg = !d.neg;
                }
                Ok(d)
            }
            Expr::Binary { op, left, right } => {
                let a = self.eval_expr(left)?;
                let b = self.eval_expr(right)?;
                match op {
                    ArithOp::Add => checked(add(&a, &b)),
                    ArithOp::Sub => checked(sub(&a, &b)),
                    ArithOp::Mul => checked(mul(&a, &b)),
                    ArithOp::Div => {
                        if b.is_zero() {
                            return Err(RuntimeError::DivideByZero);
                        }
                        checked(div(&a, &b, COMPUTE_DIV_SCALE))
                    }
                    ArithOp::Pow => pow(&a, &b).ok_or_else(|| {
                        RuntimeError::Unsupported(
                            "COMPUTE ** with a negative, fractional, or oversized exponent".into(),
                        )
                    }),
                }
            }
        }
    }

    /// The numeric value of an operand (numeric literal, `ZERO`, or numeric
    /// item). Non-numeric operands are an error — you cannot do arithmetic on
    /// an alphanumeric value.
    fn operand_decimal(&self, op: &Operand) -> Result<Decimal, RuntimeError> {
        match self.src_from_operand(op)? {
            Src::Num(d) => Ok(d),
            Src::Fig(Fig::Zero) => Ok(Decimal::zero()),
            Src::Fig(Fig::Space) | Src::Chars(_) => {
                Err(RuntimeError::Unsupported("arithmetic on a non-numeric operand".into()))
            }
        }
    }

    /// The numeric value of a named field (must be a numeric item).
    fn named_decimal(&self, name: &str) -> Result<Decimal, RuntimeError> {
        let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.into()))?;
        match &self.items[idx].picture {
            Some(p) if p.is_numeric() => Ok(self.item_as_decimal(idx)),
            _ => Err(RuntimeError::Unsupported(format!("arithmetic on non-numeric field {name}"))),
        }
    }

    /// Store a computed number into a named receiver (reshaped to its picture;
    /// an unsigned receiver keeps the magnitude).
    fn store_number(&mut self, name: &str, value: Decimal) -> Result<(), RuntimeError> {
        let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.into()))?;
        self.move_into(idx, Src::Num(value))
    }

    // ----------------------------------------------------------------------
    // MOVE
    // ----------------------------------------------------------------------

    fn move_into(&mut self, dst: usize, src: Src) -> Result<(), RuntimeError> {
        let picture = self.items[dst]
            .picture
            .clone()
            .ok_or_else(|| RuntimeError::Unsupported("MOVE into a group item".into()))?;

        let (new_storage, new_neg) = match picture {
            Picture::Numeric { int_digits, dec_digits, signed } => {
                let d = match src {
                    Src::Num(d) => d,
                    Src::Fig(Fig::Zero) => Decimal::zero(),
                    Src::Fig(Fig::Space) => {
                        return Err(RuntimeError::Unsupported("MOVE SPACES to a numeric item".into()))
                    }
                    Src::Chars(_) => {
                        return Err(RuntimeError::Unsupported(
                            "MOVE of an alphanumeric value to a numeric item".into(),
                        ))
                    }
                };
                // A signed field keeps the sign (except on zero, which is
                // unsigned); an unsigned field drops it to magnitude.
                let neg = signed && d.neg && !d.is_zero();
                (move_into_numeric(&d, int_digits, dec_digits), neg)
            }
            Picture::Alphanumeric { size } | Picture::Alphabetic { size } => {
                let chars = match src {
                    Src::Chars(s) => s,
                    Src::Num(d) => d.digits(),
                    Src::Fig(Fig::Zero) => "0".repeat(size),
                    Src::Fig(Fig::Space) => " ".repeat(size),
                };
                (move_into_char(&chars, size), false)
            }
        };
        self.items[dst].storage = new_storage;
        self.items[dst].neg = new_neg;
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Source / display resolution
    // ----------------------------------------------------------------------

    fn src_from_lit(&self, lit: &Lit) -> Result<Src, RuntimeError> {
        match lit {
            Lit::Str(s) => Ok(Src::Chars(s.clone())),
            Lit::Fig(f) => Ok(Src::Fig(f.clone())),
            Lit::Num(s) => Decimal::parse_literal(s)
                .map(Src::Num)
                .ok_or_else(|| RuntimeError::Unsupported(format!("numeric literal {s}"))),
        }
    }

    fn src_from_operand(&self, op: &Operand) -> Result<Src, RuntimeError> {
        match op {
            Operand::Lit(lit) => self.src_from_lit(lit),
            Operand::Ident(name) => {
                let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                let item = &self.items[idx];
                match &item.picture {
                    Some(p) if p.is_numeric() => Ok(Src::Num(self.item_as_decimal(idx))),
                    Some(_) => Ok(Src::Chars(item.storage.clone())),
                    // A group item is treated as an alphanumeric string.
                    None => Ok(Src::Chars(self.group_image(idx))),
                }
            }
        }
    }

    /// A numeric item's value as a [`Decimal`], split by its implied decimal.
    /// A signed item carries its stored operational sign.
    fn item_as_decimal(&self, idx: usize) -> Decimal {
        let item = &self.items[idx];
        if let Some(Picture::Numeric { int_digits, .. }) = &item.picture {
            let int: String = item.storage.chars().take(*int_digits).collect();
            let frac: String = item.storage.chars().skip(*int_digits).collect();
            Decimal { neg: item.neg, int, frac }
        } else {
            Decimal::zero()
        }
    }

    /// The display image of an operand.
    fn display_image(&self, op: &Operand) -> Result<String, RuntimeError> {
        match op {
            Operand::Lit(Lit::Str(s)) => Ok(s.clone()),
            Operand::Lit(Lit::Num(s)) => Ok(s.clone()),
            Operand::Lit(Lit::Fig(Fig::Zero)) => Ok("0".into()),
            Operand::Lit(Lit::Fig(Fig::Space)) => Ok(" ".into()),
            Operand::Ident(name) => {
                let idx = *self.by_name.get(name).ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                Ok(self.item_image(idx))
            }
        }
    }

    /// An item's stored image (elementary → its storage; group → its children).
    /// A signed numeric item shows its sign as a trailing **overpunch** on the
    /// units digit — COBOL's default `DISPLAY` of a `PIC S9…` field.
    fn item_image(&self, idx: usize) -> String {
        let item = &self.items[idx];
        match &item.picture {
            Some(Picture::Numeric { signed: true, .. }) => overpunch_trailing(&item.storage, item.neg),
            Some(_) => item.storage.clone(),
            None => self.group_image(idx),
        }
    }

    /// A group item's image: the concatenation of its children's images.
    fn group_image(&self, idx: usize) -> String {
        let mut s = String::new();
        for &child in &self.items[idx].children {
            s.push_str(&self.item_image(child));
        }
        s
    }
}

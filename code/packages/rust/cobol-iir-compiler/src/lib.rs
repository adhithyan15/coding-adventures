//! # COBOL-60 → IIR compiler
//!
//! Lowers a parsed COBOL-60 program (the generic `GrammarASTNode` CST from
//! [`coding_adventures_cobol_parser`]) into an [`interpreter_ir::IIRModule`], so
//! COBOL runs on every execution backend the LANG VM AOT chain targets
//! (NativeAOT / LLVM / WASM / JVM / CLR / VM / JIT). See
//! [PL09](../../../specs/PL09-codegen.md); the tree-walk interpreter
//! [`coding_adventures_cobol_runtime`] is the semantic oracle.
//!
//! ## Data model — scaled `i64` numerics (PL09 D1)
//!
//! COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. Each elementary
//! item becomes one IIR register:
//!
//! * a **numeric** item (`PIC 9…`, optionally with an implied point `V`) is an
//!   `i64` holding its value **scaled** by its fractional-digit count — `PIC
//!   9(2)V9` holding 12.3 is the integer `123`. The display image is those digits
//!   zero-padded to the field width, with no point (`123`).
//! * an **alphanumeric** item (`PIC X`/`A`) is a `str` holding its stored,
//!   space-padded character image.
//!
//! ## Verbs lowered on this rung
//!
//! | COBOL | IIR |
//! | --- | --- |
//! | `VALUE <lit>` / `MOVE <lit> TO item` | the register's `const`/`str_const` — the literal formatted into the item's picture *at compile time* (reusing cobol-runtime's `move_into_numeric`/`move_into_char`) |
//! | `ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE … [GIVING r]` | `add`/`sub`/`mul`/`div` on the `i64` slots, then the result is reduced to the receiver's field (magnitude, low-order `int_digits` kept) |
//! | `DISPLAY op…` | each operand's image emitted, then `putchar('\n')`. A literal prints its source text; a numeric item prints via the fixed-width digit helper; an alphanumeric item prints via `print_str` |
//! | `STOP RUN` | `ret 0` |
//!
//! ### Why the compile-time formatting is exact
//!
//! A numeric literal reshapes only when it lands in a field: `MOVE 42 TO PIC 9(5)`
//! stores `00042`, `MOVE 123.456 TO PIC 9(2)V9` stores `234`. That shaping is the
//! receiver-picture logic in cobol-runtime's `move_into_numeric`; because a
//! literal's value is known at compile time, we run that very function and parse
//! the resulting digits into the slot's scaled `i64`. A numeric *literal* in a
//! `DISPLAY`, by contrast, shows its **source text** (`DISPLAY 42` → `42`).
//!
//! ### Runtime arithmetic
//!
//! On this rung arithmetic is **integer** (unsigned receivers). Values are added,
//! subtracted, multiplied, and (truncating) divided in `i64`, then the result is
//! stored into the receiver by taking its magnitude and keeping the low-order
//! `int_digits` digits — exactly the runtime's silent-overflow-truncation and
//! unsigned-magnitude rules. To keep `i64` products safe, an arithmetic operand
//! or receiver is capped at [`ARITH_MAX_DIGITS`] digits; a wider field is a clean
//! [`CompileError::Unsupported`].
//!
//! ### Deliberately a later rung (each a clean error, never wrong output)
//!
//! Scaled-decimal arithmetic (`V` fields, scale alignment), `ROUNDED`, `ON SIZE
//! ERROR`, item-to-item `MOVE` reshaping, `COMPUTE`, `IF`, `PERFORM`, `GO TO`,
//! group items, and signed numerics (`PIC S9…`) each land on their own PR.

use coding_adventures_cobol_parser::try_parse_cobol;
use coding_adventures_cobol_runtime::{move_into_char, move_into_numeric, Decimal, Picture};
use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::HashMap;

/// The widest numeric field the scaled-`i64` model can hold (a value below
/// 10^18 fits `i64`). A wider PICTURE is a clean [`CompileError::Unsupported`].
const NUMERIC_MAX_DIGITS: usize = 18;

/// The widest numeric field permitted as an **arithmetic** operand or receiver.
/// Bounded so the `i64` product of two operands cannot overflow (2·9 = 18
/// digits < `i64::MAX`). A wider field in an arithmetic verb is unsupported here.
const ARITH_MAX_DIGITS: usize = 9;

/// A COBOL → IIR compilation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The source did not parse (lexical or syntactic).
    Parse(String),
    /// A construct not yet lowered in this slice (e.g. scaled-decimal
    /// arithmetic, `PERFORM`).
    Unsupported(String),
    /// The CST was shaped unexpectedly (a malformed node), or a PICTURE the
    /// picture layer rejects (e.g. hostile width).
    Malformed(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Parse(m) => write!(f, "COBOL parse error: {m}"),
            CompileError::Unsupported(m) => write!(f, "COBOL not yet lowered to IIR: {m}"),
            CompileError::Malformed(m) => write!(f, "malformed COBOL CST: {m}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile COBOL `source` into an [`IIRModule`] with a single `main` returning
/// `i64` (its return value is the process exit code — always 0 in this slice).
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, CompileError> {
    let ast = try_parse_cobol(source).map_err(CompileError::Parse)?;
    let mut comp = Compiler::default();
    comp.emit_program(&ast)?;

    let mut main = IIRFunction::new("main", vec![], "i64", comp.instrs);
    // Every instruction is statically typed (`str` / `i64` / `void` hints, no
    // `"any"`); assert FullyTyped as the BASIC / FLOW-MATIC frontends do.
    main.type_status = FunctionTypeStatus::FullyTyped;

    let mut module = IIRModule::new(module_name, "cobol");
    module.functions.push(main);
    // The fixed-width digit-print helper a numeric `DISPLAY` calls is appended
    // only when one was emitted (resolution is by name, so order is irrelevant).
    if comp.needs_print {
        module.functions.push(print_padded_function());
    }
    module.entry_point = Some("main".to_string());
    Ok(module)
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// How an elementary item is represented in IIR.
enum ItemKind {
    /// An alphanumeric item: a `str` register holding its stored character image.
    Char { initial: String },
    /// A numeric item: an `i64` register holding its value scaled by `dec_digits`.
    Numeric { int_digits: usize, dec_digits: usize, initial: i64 },
}

/// A WORKING-STORAGE elementary item and the IIR register backing it.
struct Item {
    reg: String,
    kind: ItemKind,
}

impl Item {
    /// The item's display width in characters/digits.
    fn width(&self) -> usize {
        match &self.kind {
            ItemKind::Char { initial } => initial.chars().count(),
            ItemKind::Numeric { int_digits, dec_digits, .. } => int_digits + dec_digits,
        }
    }
}

#[derive(Default)]
struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Elementary items by COBOL data-name, in declaration order (for a stable
    /// init prologue) plus a name index for `MOVE` / `DISPLAY` / arithmetic.
    items: Vec<Item>,
    by_name: HashMap<String, usize>,
    /// Set when a numeric `DISPLAY` emits a call to the digit-print helper, so it
    /// is appended to the module.
    needs_print: bool,
    /// Unique-suffix counter for throwaway registers.
    tmp_counter: usize,
}

impl Compiler {
    fn emit(&mut self, op: &str, dest: Option<&str>, srcs: Vec<Operand>, type_hint: &str) {
        self.instrs.push(IIRInstr::new(op, dest.map(str::to_string), srcs, type_hint));
    }

    fn fresh(&mut self, prefix: &str) -> String {
        let n = self.tmp_counter;
        self.tmp_counter += 1;
        format!("{prefix}{n}")
    }

    fn emit_program(&mut self, program: &GrammarASTNode) -> Result<(), CompileError> {
        // 1. Build the item table from WORKING-STORAGE (if any).
        self.collect_items(program)?;

        // 2. Prologue: initialise every item register to its stored value, in
        //    declaration order, so every item is defined before any use.
        for idx in 0..self.items.len() {
            let reg = self.items[idx].reg.clone();
            match &self.items[idx].kind {
                ItemKind::Char { initial } => {
                    let initial = initial.clone();
                    self.emit("str_const", Some(&reg), vec![Operand::Str(initial)], "str");
                }
                ItemKind::Numeric { initial, .. } => {
                    let initial = *initial;
                    self.emit("const", Some(&reg), vec![Operand::Int(initial)], "i64");
                }
            }
        }

        // 3. The PROCEDURE DIVISION's statements, in document order. With no
        //    PERFORM / GO TO on this rung, control simply falls through.
        let pd = child_node(program, "procedure_division")
            .ok_or_else(|| CompileError::Malformed("program without a PROCEDURE DIVISION".into()))?;
        for para in child_nodes(pd, "paragraph") {
            for sentence in child_nodes(para, "sentence") {
                for stmt in child_nodes(sentence, "statement") {
                    self.emit_statement(stmt)?;
                }
            }
        }

        // 4. A trailing `ret 0` guarantees `main` returns even without a STOP RUN.
        self.emit_ret_zero();
        Ok(())
    }

    /// Populate [`Self::items`] from the DATA DIVISION's WORKING-STORAGE. Only
    /// elementary items (those with a PICTURE) are modelled on this rung.
    fn collect_items(&mut self, program: &GrammarASTNode) -> Result<(), CompileError> {
        // WORKING-STORAGE sits under `data_division > data_section`, so reach it
        // with a recursive find (the depth is fixed and tiny).
        let Some(ws) = find(program, "working_storage_section") else {
            return Ok(());
        };
        for entry in child_nodes(ws, "data_entry") {
            self.collect_entry(entry)?;
        }
        Ok(())
    }

    fn collect_entry(&mut self, entry: &GrammarASTNode) -> Result<(), CompileError> {
        // A data_entry is `NUMBER (NAME | FILLER) data_clause* DOT`. FILLER has
        // no referable name; with group items deferred it plays no role yet.
        let Some(name) = first_token(entry, "NAME") else {
            return Ok(());
        };
        // Each `data_clause` wraps one `picture_clause` or `value_clause`. The
        // PICTURE clause (if present) makes this an elementary item.
        let Some(pic_node) = find_clause(entry, "picture_clause") else {
            // A group item — deferred; an unregistered name errors honestly on use.
            return Ok(());
        };
        let pic_str = first_token(pic_node, "PIC_STRING")
            .ok_or_else(|| CompileError::Malformed("PICTURE clause without a picture string".into()))?;
        let picture = Picture::parse(&pic_str)
            .map_err(|e| CompileError::Malformed(format!("PICTURE {pic_str}: {e}")))?;

        // Signed numerics (`PIC S9…`) display via a trailing sign overpunch — a
        // later rung. Reject at declaration so we never emit a wrong image.
        if matches!(picture, Picture::Numeric { signed: true, .. }) {
            return Err(CompileError::Unsupported(format!(
                "signed numeric (PIC S9…) item {name} — sign overpunch is a later rung"
            )));
        }

        let value_lit = match find_clause(entry, "value_clause") {
            Some(vc) => Some(read_literal(
                child_node(vc, "literal")
                    .ok_or_else(|| CompileError::Malformed("VALUE without a literal".into()))?,
            )?),
            None => None,
        };

        let kind = match &picture {
            Picture::Numeric { int_digits, dec_digits, .. } => {
                if int_digits + dec_digits > NUMERIC_MAX_DIGITS {
                    return Err(CompileError::Unsupported(format!(
                        "numeric item {name} wider than {NUMERIC_MAX_DIGITS} digits exceeds the \
                         i64 value model — a later rung"
                    )));
                }
                // Initial scaled value: VALUE applied as an initialising MOVE
                // (else 0). We reuse the oracle's `move_into_numeric` to produce
                // the exact zero-filled digit string, then read it as the scaled
                // integer.
                let initial = match &value_lit {
                    Some(src) => {
                        let digits = format_into_picture(src, &picture)
                            .map_err(|m| CompileError::Unsupported(format!("VALUE {name}: {m}")))?;
                        parse_digits(&digits)
                    }
                    None => 0,
                };
                ItemKind::Numeric { int_digits: *int_digits, dec_digits: *dec_digits, initial }
            }
            Picture::Alphanumeric { .. } | Picture::Alphabetic { .. } => {
                let initial = match &value_lit {
                    Some(src) => format_into_picture(src, &picture)
                        .map_err(|m| CompileError::Unsupported(format!("VALUE {name}: {m}")))?,
                    None => default_char_image(&picture),
                };
                ItemKind::Char { initial }
            }
        };

        let reg = format!("itm_{}", sanitise(&name));
        let idx = self.items.len();
        self.items.push(Item { reg, kind });
        self.by_name.insert(name, idx);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    fn emit_statement(&mut self, stmt: &GrammarASTNode) -> Result<(), CompileError> {
        // A `statement` wraps exactly one verb node.
        let verb = stmt
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .ok_or_else(|| CompileError::Malformed("empty statement".into()))?;

        match verb.rule_name.as_str() {
            "display_stmt" => self.emit_display(verb),
            "move_stmt" => self.emit_move(verb),
            "stop_stmt" => self.emit_stop(verb),
            "add_stmt" => self.emit_add(verb),
            "subtract_stmt" => self.emit_subtract(verb),
            "multiply_stmt" => self.emit_multiply(verb),
            "divide_stmt" => self.emit_divide(verb),
            other => Err(CompileError::Unsupported(format!(
                "the {} statement is a later rung",
                verb_name(other)
            ))),
        }
    }

    /// `DISPLAY op…` — print each operand's image (no separator), then a newline.
    fn emit_display(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        for op in child_nodes(verb, "operand") {
            match read_operand(op)? {
                Operandy::Name(name) => {
                    let idx = self.item_index(&name)?;
                    match &self.items[idx].kind {
                        ItemKind::Char { .. } => {
                            let reg = self.items[idx].reg.clone();
                            self.emit("print_str", None, vec![Operand::Var(reg)], "void");
                        }
                        ItemKind::Numeric { .. } => {
                            let reg = self.items[idx].reg.clone();
                            let width = self.items[idx].width() as i64;
                            self.emit_print_numeric(&reg, width);
                        }
                    }
                }
                Operandy::Literal(src) => {
                    let image = display_image(&src);
                    let tmp = self.fresh("_d");
                    self.emit("str_const", Some(&tmp), vec![Operand::Str(image)], "str");
                    self.emit("print_str", None, vec![Operand::Var(tmp)], "void");
                }
            }
        }
        self.emit_newline();
        Ok(())
    }

    /// `MOVE src TO recv…` — the source must be a literal on this rung (an
    /// item-to-item move needs runtime picture reshaping). Each receiver is set
    /// to the literal formatted into *its* picture, computed at compile time.
    fn emit_move(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let src_node = child_node(verb, "operand")
            .ok_or_else(|| CompileError::Malformed("MOVE without a source".into()))?;
        let src = match read_operand(src_node)? {
            Operandy::Literal(lit) => lit,
            Operandy::Name(n) => {
                return Err(CompileError::Unsupported(format!(
                    "MOVE from item {n} (item-to-item reshaping is a later rung)"
                )))
            }
        };
        let dsts: Vec<String> = child_tokens(verb)
            .into_iter()
            .filter(|(k, _)| k == "NAME")
            .map(|(_, v)| v)
            .collect();
        if dsts.is_empty() {
            return Err(CompileError::Malformed("MOVE without a receiver".into()));
        }
        for dst in dsts {
            let idx = self.item_index(&dst)?;
            let reg = self.items[idx].reg.clone();
            match &self.items[idx].kind {
                ItemKind::Char { .. } => {
                    let picture = Picture::Alphanumeric { size: self.items[idx].width() };
                    let image = format_into_picture(&src, &picture)
                        .map_err(|m| CompileError::Unsupported(format!("MOVE into {dst}: {m}")))?;
                    self.emit("str_const", Some(&reg), vec![Operand::Str(image)], "str");
                }
                ItemKind::Numeric { int_digits, dec_digits, .. } => {
                    let picture = Picture::Numeric {
                        int_digits: *int_digits,
                        dec_digits: *dec_digits,
                        signed: false,
                    };
                    let digits = format_into_picture(&src, &picture)
                        .map_err(|m| CompileError::Unsupported(format!("MOVE into {dst}: {m}")))?;
                    let value = parse_digits(&digits);
                    self.emit("const", Some(&reg), vec![Operand::Int(value)], "i64");
                }
            }
        }
        Ok(())
    }

    /// `STOP RUN` → `ret 0`.
    fn emit_stop(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let has_run = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "RUN");
        if has_run {
            self.emit_ret_zero();
            Ok(())
        } else {
            Err(CompileError::Unsupported("STOP <literal> (only STOP RUN is modelled)".into()))
        }
    }

    // -----------------------------------------------------------------------
    // Arithmetic (integer, unsigned receivers)
    // -----------------------------------------------------------------------

    /// `ADD op… TO name [GIVING g]` — the accumulator starts at `name`'s value
    /// and each operand is added; the sum is stored into `g` (or `name`).
    fn emit_add(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.reject_rounded_or_size_error(verb, "ADD")?;
        let (to, giving) = self.two_targets(verb, "TO")?;
        let acc = self.fresh("_acc");
        let start = self.int_operand_from_name(&to)?;
        self.emit("mov", Some(&acc), vec![start], "i64");
        for op in child_nodes(verb, "operand") {
            let v = self.int_operand(op)?;
            self.emit("add", Some(&acc), vec![Operand::Var(acc.clone()), v], "i64");
        }
        self.store_into(giving.as_deref().unwrap_or(&to), &acc)
    }

    /// `SUBTRACT op… FROM name [GIVING g]` — `name` minus each operand.
    fn emit_subtract(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.reject_rounded_or_size_error(verb, "SUBTRACT")?;
        let (from, giving) = self.two_targets(verb, "FROM")?;
        let acc = self.fresh("_acc");
        let start = self.int_operand_from_name(&from)?;
        self.emit("mov", Some(&acc), vec![start], "i64");
        for op in child_nodes(verb, "operand") {
            let v = self.int_operand(op)?;
            self.emit("sub", Some(&acc), vec![Operand::Var(acc.clone()), v], "i64");
        }
        self.store_into(giving.as_deref().unwrap_or(&from), &acc)
    }

    /// `MULTIPLY a BY b [GIVING g]` — `a * b` into `g` (or `b`).
    fn emit_multiply(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.reject_rounded_or_size_error(verb, "MULTIPLY")?;
        let ops = child_nodes(verb, "operand");
        if ops.len() != 2 {
            return Err(CompileError::Malformed("MULTIPLY needs two operands".into()));
        }
        let a = self.int_operand(ops[0])?;
        let b = self.int_operand(ops[1])?;
        let prod = self.fresh("_prod");
        self.emit("mul", Some(&prod), vec![a, b], "i64");
        let target = self.giving_or_operand_name(verb, ops[1], "MULTIPLY … BY <literal>")?;
        self.store_into(&target, &prod)
    }

    /// `DIVIDE a INTO b [GIVING g]` — `b / a` (truncating) into `g` (or `b`).
    fn emit_divide(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.reject_rounded_or_size_error(verb, "DIVIDE")?;
        let ops = child_nodes(verb, "operand");
        if ops.len() != 2 {
            return Err(CompileError::Malformed("DIVIDE needs two operands".into()));
        }
        let divisor = self.int_operand(ops[0])?;
        let dividend = self.int_operand(ops[1])?;
        let quot = self.fresh("_quot");
        // Integer division truncates toward zero — COBOL's behaviour into an
        // integer receiver. (A zero divisor traps at run time, matching the
        // oracle's hard DivideByZero for the handler-less case.)
        self.emit("div", Some(&quot), vec![dividend, divisor], "i64");
        let target = self.giving_or_operand_name(verb, ops[1], "DIVIDE … INTO <literal>")?;
        self.store_into(&target, &quot)
    }

    /// Reduce `value_reg` to `target`'s field and store it: take the magnitude
    /// (unsigned receivers keep no sign) and the low-order `int_digits` digits
    /// (COBOL truncates high-order overflow silently), then move into the slot.
    fn store_into(&mut self, target: &str, value_reg: &str) -> Result<(), CompileError> {
        let idx = self.item_index(target)?;
        let (int_digits, dec_digits, reg) = match &self.items[idx].kind {
            ItemKind::Numeric { int_digits, dec_digits, .. } => {
                (*int_digits, *dec_digits, self.items[idx].reg.clone())
            }
            ItemKind::Char { .. } => {
                return Err(CompileError::Unsupported(format!(
                    "arithmetic into non-numeric field {target}"
                )))
            }
        };
        if dec_digits != 0 {
            return Err(CompileError::Unsupported(format!(
                "arithmetic into scaled-decimal field {target} (PIC …V…) is a later rung"
            )));
        }
        if int_digits > ARITH_MAX_DIGITS {
            return Err(CompileError::Unsupported(format!(
                "arithmetic into field {target} wider than {ARITH_MAX_DIGITS} digits is a later rung"
            )));
        }
        // magnitude: if value < 0, negate (fields ≤ 9 digits, so no i64::MIN).
        let acc = value_reg.to_string();
        let zero = self.fresh("_z");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        let neg = self.fresh("_neg");
        self.emit("cmp_lt", Some(&neg), vec![Operand::Var(acc.clone()), Operand::Var(zero.clone())], "i64");
        let skip = self.fresh("_absskip");
        self.emit("jmp_if_false", None, vec![Operand::Var(neg), Operand::Var(skip.clone())], "void");
        self.emit("sub", Some(&acc), vec![Operand::Var(zero), Operand::Var(acc.clone())], "i64");
        self.emit("label", None, vec![Operand::Var(skip)], "void");
        // truncate high-order: value % 10^int_digits.
        let modulus = 10i64.pow(int_digits as u32);
        let m = self.fresh("_m");
        self.emit("const", Some(&m), vec![Operand::Int(modulus)], "i64");
        self.emit("mod", Some(&acc), vec![Operand::Var(acc.clone()), Operand::Var(m)], "i64");
        self.emit("mov", Some(&reg), vec![Operand::Var(acc)], "i64");
        Ok(())
    }

    /// Reject `ROUNDED` / `ON SIZE ERROR` on an arithmetic verb (a later rung):
    /// ignoring them would silently produce wrong output.
    fn reject_rounded_or_size_error(
        &self,
        verb: &GrammarASTNode,
        name: &str,
    ) -> Result<(), CompileError> {
        if child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "ROUNDED") {
            return Err(CompileError::Unsupported(format!("{name} … ROUNDED is a later rung")));
        }
        if child_node(verb, "size_error").is_some() {
            return Err(CompileError::Unsupported(format!(
                "{name} … ON SIZE ERROR is a later rung"
            )));
        }
        Ok(())
    }

    /// The `(to/from, giving)` receiver names of an ADD/SUBTRACT: the direct NAME
    /// tokens after the keyword are `[to]` or `[to, giving]`.
    fn two_targets(
        &self,
        verb: &GrammarASTNode,
        _keyword: &str,
    ) -> Result<(String, Option<String>), CompileError> {
        let names: Vec<String> = child_tokens(verb)
            .into_iter()
            .filter(|(k, _)| k == "NAME")
            .map(|(_, v)| v)
            .collect();
        match names.as_slice() {
            [to] => Ok((to.clone(), None)),
            [to, giving] => Ok((to.clone(), Some(giving.clone()))),
            _ => Err(CompileError::Malformed("arithmetic verb without a receiver".into())),
        }
    }

    /// The receiver of a MULTIPLY/DIVIDE: the `GIVING` name if present, else the
    /// second operand, which must be a data-name (a literal has nowhere to land).
    fn giving_or_operand_name(
        &self,
        verb: &GrammarASTNode,
        operand: &GrammarASTNode,
        no_giving_msg: &str,
    ) -> Result<String, CompileError> {
        if let Some(g) = first_token(verb, "NAME") {
            return Ok(g);
        }
        match read_operand(operand)? {
            Operandy::Name(n) => Ok(n),
            Operandy::Literal(_) => Err(CompileError::Unsupported(format!(
                "{no_giving_msg} without GIVING has no receiver"
            ))),
        }
    }

    /// An arithmetic operand as an IIR `i64` [`Operand`]: an integer literal
    /// becomes a `const`-free immediate; a data-name becomes its register.
    /// Fractional literals/fields, alphanumerics, and over-wide fields are a
    /// later rung.
    fn int_operand(&self, op: &GrammarASTNode) -> Result<Operand, CompileError> {
        match read_operand(op)? {
            Operandy::Literal(Src::Num(s)) => {
                let d = Decimal::parse_literal(&s)
                    .ok_or_else(|| CompileError::Malformed(format!("numeric literal {s}")))?;
                Ok(Operand::Int(integer_decimal(&d).map_err(CompileError::Unsupported)?))
            }
            Operandy::Literal(Src::Zero) => Ok(Operand::Int(0)),
            Operandy::Literal(_) => Err(CompileError::Unsupported(
                "an alphanumeric operand in arithmetic".into(),
            )),
            Operandy::Name(n) => self.int_operand_from_name(&n),
        }
    }

    /// An integer numeric item's register as an arithmetic operand.
    fn int_operand_from_name(&self, name: &str) -> Result<Operand, CompileError> {
        let idx = self.item_index(name)?;
        match &self.items[idx].kind {
            ItemKind::Numeric { int_digits, dec_digits, .. } => {
                if *dec_digits != 0 {
                    return Err(CompileError::Unsupported(format!(
                        "arithmetic on scaled-decimal field {name} (PIC …V…) is a later rung"
                    )));
                }
                if *int_digits > ARITH_MAX_DIGITS {
                    return Err(CompileError::Unsupported(format!(
                        "arithmetic on field {name} wider than {ARITH_MAX_DIGITS} digits is a later rung"
                    )));
                }
                Ok(Operand::Var(self.items[idx].reg.clone()))
            }
            ItemKind::Char { .. } => Err(CompileError::Unsupported(format!(
                "arithmetic on non-numeric field {name}"
            ))),
        }
    }

    /// Emit the digit helper call that prints a numeric item's `width`-digit,
    /// zero-padded image.
    fn emit_print_numeric(&mut self, reg: &str, width: i64) {
        self.needs_print = true;
        let w = self.fresh("_w");
        self.emit("const", Some(&w), vec![Operand::Int(width)], "i64");
        let ret = self.fresh("_pr");
        self.emit(
            "call",
            Some(&ret),
            vec![Operand::Var(PRINT_HELPER.into()), Operand::Var(reg.to_string()), Operand::Var(w)],
            "i64",
        );
    }

    /// `putchar('\n')` — the record terminator every `DISPLAY` appends.
    fn emit_newline(&mut self) {
        let t = self.fresh("_nl");
        self.emit("const", Some(&t), vec![Operand::Int(b'\n' as i64)], "i64");
        self.emit("call_builtin", None, vec![Operand::Var("putchar".into()), Operand::Var(t)], "void");
    }

    fn emit_ret_zero(&mut self) {
        let z = self.fresh("_ret");
        self.emit("const", Some(&z), vec![Operand::Int(0)], "i64");
        self.emit("ret", None, vec![Operand::Var(z)], "i64");
    }

    /// The item index for a data-name, or a clean error if it is not a declared
    /// elementary item (a group item or an undeclared name).
    fn item_index(&self, name: &str) -> Result<usize, CompileError> {
        self.by_name.get(name).copied().ok_or_else(|| {
            CompileError::Unsupported(format!(
                "reference to {name} (a group item or undeclared name — a later rung)"
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// The fixed-width digit-print helper
// ---------------------------------------------------------------------------

const PRINT_HELPER: &str = "__cob_print_padded";

/// `__cob_print_padded(v, w)` prints exactly `w` decimal digits of the
/// **non-negative** value `v`, most-significant first, zero-padded — and
/// truncates any digits beyond `w` (COBOL's high-order overflow). It recurses on
/// `v/10, w-1`, printing this level's digit `v % 10` after the recursion; when
/// `w` reaches 0 it returns. `w` is the field width (≤ 18), so the recursion is
/// shallow. Numeric slots are stored as their magnitude, so `v` is never
/// negative here.
fn print_padded_function() -> IIRFunction {
    fn mk(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
        IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
    }
    fn var(name: &str) -> Operand {
        Operand::Var(name.to_string())
    }

    let body = vec![
        // if w <= 0 → return 0.
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        mk("cmp_lt", Some("more"), vec![var("zero"), var("w")], "i64"), // 0 < w  ⟺  w > 0
        mk("jmp_if_false", None, vec![var("more"), var("base")], "void"),
        // recurse on (v/10, w-1).
        mk("const", Some("ten"), vec![Operand::Int(10)], "i64"),
        mk("div", Some("vq"), vec![var("v"), var("ten")], "i64"),
        mk("const", Some("one"), vec![Operand::Int(1)], "i64"),
        mk("sub", Some("wm"), vec![var("w"), var("one")], "i64"),
        mk("call", Some("_r"), vec![var(PRINT_HELPER), var("vq"), var("wm")], "i64"),
        // print this level's digit: (v % 10) + '0'.
        mk("mod", Some("d"), vec![var("v"), var("ten")], "i64"),
        mk("const", Some("c0"), vec![Operand::Int(b'0' as i64)], "i64"),
        mk("add", Some("c"), vec![var("d"), var("c0")], "i64"),
        mk("call_builtin", None, vec![var("putchar"), var("c")], "void"),
        mk("const", Some("z1"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z1")], "i64"),
        // base case.
        mk("label", None, vec![var("base")], "void"),
        mk("const", Some("z2"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z2")], "i64"),
    ];
    let mut f = IIRFunction::new(
        PRINT_HELPER,
        vec![("v".into(), "i64".into()), ("w".into(), "i64".into())],
        "i64",
        body,
    );
    f.type_status = FunctionTypeStatus::FullyTyped;
    f
}

// ---------------------------------------------------------------------------
// Literals & picture formatting (the reuse boundary with cobol-runtime)
// ---------------------------------------------------------------------------

/// A source value in flight: a literal or a data-name reference.
enum Operandy {
    Literal(Src),
    Name(String),
}

/// A literal source, mirroring the runtime's `Src` for the values this rung
/// handles.
enum Src {
    /// A numeric literal, kept as source text (a *literal* `DISPLAY` shows it
    /// verbatim); parsed to a [`Decimal`] when it lands in a numeric field.
    Num(String),
    /// A quoted alphanumeric literal (already unquoted by the lexer).
    Str(String),
    /// `ZERO` / `ZEROS` / `ZEROES`.
    Zero,
    /// `SPACE` / `SPACES`.
    Space,
}

/// Read an `operand` node into either a literal or a data-name reference.
fn read_operand(op: &GrammarASTNode) -> Result<Operandy, CompileError> {
    if let Some(lit) = child_node(op, "literal") {
        return Ok(Operandy::Literal(read_literal(lit)?));
    }
    if let Some(name) = first_token(op, "NAME") {
        return Ok(Operandy::Name(name));
    }
    Err(CompileError::Malformed("unrecognised operand".into()))
}

/// Read a `literal` node (NUMBER / STRING / figurative) into a [`Src`].
fn read_literal(lit: &GrammarASTNode) -> Result<Src, CompileError> {
    if let Some(fig) = child_node(lit, "figurative") {
        let word = child_tokens(fig).into_iter().map(|(_, v)| v).next().unwrap_or_default();
        return match word.as_str() {
            "ZERO" | "ZEROS" | "ZEROES" => Ok(Src::Zero),
            "SPACE" | "SPACES" => Ok(Src::Space),
            other => Err(CompileError::Unsupported(format!("figurative constant {other}"))),
        };
    }
    for (kind, val) in child_tokens(lit) {
        match kind.as_str() {
            "NUMBER" => return Ok(Src::Num(val)),
            "STRING" => return Ok(Src::Str(val)),
            _ => {}
        }
    }
    Err(CompileError::Malformed("unrecognised literal".into()))
}

/// A literal operand's `DISPLAY` image: its source text (numeric literals are
/// **not** picture-shaped), the string as-is, or the figurative's one glyph.
fn display_image(src: &Src) -> String {
    match src {
        Src::Num(s) => s.clone(),
        Src::Str(s) => s.clone(),
        Src::Zero => "0".into(),
        Src::Space => " ".into(),
    }
}

/// The default initial image of a character item with no VALUE: spaces.
fn default_char_image(picture: &Picture) -> String {
    " ".repeat(picture.size())
}

/// Format a literal source into a receiver picture's stored image — the exact
/// transform the oracle performs in `move_into`, reusing cobol-runtime's own
/// picture/value logic so the result is byte-identical. Returns a message for
/// the category errors the runtime also rejects.
fn format_into_picture(src: &Src, picture: &Picture) -> Result<String, String> {
    match picture {
        Picture::Numeric { int_digits, dec_digits, .. } => {
            let d = match src {
                Src::Num(s) => Decimal::parse_literal(s).ok_or_else(|| format!("numeric literal {s}"))?,
                Src::Zero => Decimal::zero(),
                Src::Space => return Err("MOVE SPACES to a numeric item".into()),
                Src::Str(_) => return Err("MOVE of an alphanumeric value to a numeric item".into()),
            };
            Ok(move_into_numeric(&d, *int_digits, *dec_digits))
        }
        Picture::Alphanumeric { size } | Picture::Alphabetic { size } => {
            let chars = match src {
                Src::Str(s) => s.clone(),
                Src::Num(s) => {
                    Decimal::parse_literal(s).ok_or_else(|| format!("numeric literal {s}"))?.digits()
                }
                Src::Zero => "0".repeat(*size),
                Src::Space => " ".repeat(*size),
            };
            Ok(move_into_char(&chars, *size))
        }
    }
}

/// Parse a run of decimal digits (the scaled image `move_into_numeric` produced)
/// into its `i64` value. The image is at most [`NUMERIC_MAX_DIGITS`] digits, so
/// it always fits; a leading run of zeros parses fine.
fn parse_digits(digits: &str) -> i64 {
    digits.parse::<i64>().unwrap_or(0)
}

/// An integer-valued [`Decimal`] as `i64`, for an arithmetic operand. A non-zero
/// fractional part (a scaled-decimal literal) or an over-wide magnitude is a
/// later rung.
fn integer_decimal(d: &Decimal) -> Result<i64, String> {
    if d.frac.chars().any(|c| c != '0') {
        return Err(format!("scaled-decimal literal {}.{} in arithmetic is a later rung", d.int, d.frac));
    }
    let int_part = d.int.trim_start_matches('0');
    if int_part.len() > ARITH_MAX_DIGITS {
        return Err(format!(
            "numeric literal wider than {ARITH_MAX_DIGITS} digits in arithmetic is a later rung"
        ));
    }
    let mag: i64 = if int_part.is_empty() { 0 } else { int_part.parse().map_err(|_| "numeric literal".to_string())? };
    Ok(if d.neg { -mag } else { mag })
}

// ---------------------------------------------------------------------------
// CST helpers
// ---------------------------------------------------------------------------

fn child_nodes<'a>(n: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(x) if x.rule_name == rule => Some(x),
            _ => None,
        })
        .collect()
}

fn child_node<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(n, rule).into_iter().next()
}

/// Recursively find the first descendant node with the given rule name.
fn find<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    if n.rule_name == rule {
        return Some(n);
    }
    for c in &n.children {
        if let ASTNodeOrToken::Node(child) = c {
            if let Some(found) = find(child, rule) {
                return Some(found);
            }
        }
    }
    None
}

/// Find a specific data clause inside a `data_entry`: each clause is wrapped in
/// a `data_clause` node, so we look one level down through those wrappers.
fn find_clause<'a>(entry: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(entry, "data_clause").into_iter().find_map(|dc| child_node(dc, rule))
}

fn child_tokens(n: &GrammarASTNode) -> Vec<(String, String)> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some((t.effective_type_name().to_string(), t.value.clone())),
            _ => None,
        })
        .collect()
}

fn first_token(n: &GrammarASTNode, type_name: &str) -> Option<String> {
    child_tokens(n).into_iter().find(|(k, _)| k == type_name).map(|(_, v)| v)
}

/// A register-safe identifier from a COBOL data-name (hyphens → underscores).
fn sanitise(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// A human-readable verb name for the "not yet lowered" error.
fn verb_name(rule: &str) -> &str {
    match rule {
        "compute_stmt" => "COMPUTE",
        "perform_stmt" => "PERFORM",
        "goto_stmt" => "GO TO",
        "if_stmt" => "IF",
        "accept_stmt" => "ACCEPT",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(lines: &[&str]) -> String {
        lines.iter().map(|l| format!("000000 {l}")).collect::<Vec<_>>().join("\n")
    }

    fn wrap(data: &[&str], proc: &[&str]) -> String {
        let mut lines = vec!["IDENTIFICATION DIVISION.", "PROGRAM-ID. P."];
        if !data.is_empty() {
            lines.push("DATA DIVISION.");
            lines.push("WORKING-STORAGE SECTION.");
            lines.extend_from_slice(data);
        }
        lines.push("PROCEDURE DIVISION.");
        lines.push("MAIN.");
        lines.extend_from_slice(proc);
        program(&lines)
    }

    fn ops(module: &IIRModule) -> Vec<String> {
        module.functions[0].instructions.iter().map(|i| i.op.clone()).collect()
    }

    #[test]
    fn add_compiles_to_i64_ops_and_validates() {
        let module = compile_source(
            &wrap(&["01  R  PIC 9(3) VALUE 10."], &["ADD 5 3 TO R.", "DISPLAY R.", "STOP RUN."]),
            "add",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"add".to_string()));
        assert!(os.contains(&"mod".to_string())); // field reduction
        assert!(os.contains(&"call".to_string())); // numeric DISPLAY helper
        // The print helper function was appended.
        assert!(module.functions.iter().any(|f| f.name == PRINT_HELPER));
    }

    #[test]
    fn subtract_multiply_divide_validate() {
        for body in [
            "SUBTRACT 3 FROM R.",
            "MULTIPLY 7 BY R.",
            "DIVIDE 5 INTO R.",
            "ADD 2 3 TO R GIVING R.",
        ] {
            let module = compile_source(
                &wrap(&["01  R  PIC 9(3) VALUE 20."], &[body, "STOP RUN."]),
                "arith",
            )
            .unwrap();
            assert!(module.validate().is_empty(), "{body}: {:?}", module.validate());
        }
    }

    #[test]
    fn scaled_decimal_arithmetic_is_unsupported() {
        let err = compile_source(
            &wrap(&["01  R  PIC 9(2)V9 VALUE 0."], &["ADD 1 TO R.", "STOP RUN."]),
            "v",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn rounded_and_on_size_error_are_unsupported() {
        for body in ["ADD 1 TO R ROUNDED.", "ADD 1 TO R ON SIZE ERROR DISPLAY \"OVR\"."] {
            let err = compile_source(
                &wrap(&["01  R  PIC 9(3) VALUE 0."], &[body, "STOP RUN."]),
                "x",
            )
            .unwrap_err();
            assert!(matches!(err, CompileError::Unsupported(_)), "{body}: got {err:?}");
        }
    }

    #[test]
    fn numeric_value_still_zero_fills() {
        let module = compile_source(
            &wrap(&["01  N  PIC 9(5) VALUE 42."], &["DISPLAY N.", "STOP RUN."]),
            "n",
        )
        .unwrap();
        assert!(module.validate().is_empty());
        // The item's init const carries the scaled value 42 (displayed as 00042).
        assert!(module.functions[0]
            .instructions
            .iter()
            .any(|i| i.op == "const" && matches!(i.srcs.first(), Some(Operand::Int(42)))));
    }

    #[test]
    fn over_wide_numeric_field_is_unsupported() {
        let err = compile_source(
            &wrap(&["01  N  PIC 9(19) VALUE 0."], &["STOP RUN."]),
            "w",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn multiply_by_literal_without_giving_is_unsupported() {
        // MULTIPLY 2 BY 3 (no GIVING) has no receiver.
        let err = compile_source(
            &wrap(&["01  R  PIC 9(3)."], &["MULTIPLY 2 BY 3.", "STOP RUN."]),
            "m",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn char_display_and_parse_error_paths_preserved() {
        // Alphanumeric DISPLAY still uses print_str, and a program with no
        // PROCEDURE DIVISION still surfaces a parse error.
        let m = compile_source(
            &wrap(&["01  W  PIC X(3) VALUE \"HI\"."], &["DISPLAY W.", "STOP RUN."]),
            "c",
        )
        .unwrap();
        assert!(ops(&m).contains(&"print_str".to_string()));
        let err = compile_source(&program(&["IDENTIFICATION DIVISION.", "PROGRAM-ID. P."]), "p")
            .unwrap_err();
        assert!(matches!(err, CompileError::Parse(_)), "got {err:?}");
    }
}

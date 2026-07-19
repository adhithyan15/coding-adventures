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
//! Group items (and the alphanumeric side of item-to-item `MOVE`/comparison)
//! each remain a later rung — a clean [`CompileError`], never wrong output.
//! Scaled-decimal arithmetic, `ROUNDED`, `ON SIZE ERROR`, `COMPUTE`, `IF`,
//! `PERFORM`, `GO TO`, and signed numerics (`PIC S9…`, trailing overpunch) are
//! all lowered.

use coding_adventures_cobol_parser::try_parse_cobol;
use coding_adventures_cobol_runtime::{
    move_into_char, move_into_numeric, Decimal, Picture, COMPUTE_DIV_SCALE, MAX_POW_EXP,
};
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
    // The signed printer calls the digit-print helper, so that must be present too.
    if comp.needs_print_signed {
        module.functions.push(print_signed_function());
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
    /// A `signed` (`PIC S9…`) item keeps its sign in the `i64` and shows it as a
    /// trailing overpunch on `DISPLAY`; an unsigned item stores only magnitude.
    Numeric { int_digits: usize, dec_digits: usize, signed: bool, initial: i64 },
}

/// A WORKING-STORAGE elementary item and the IIR register backing it.
struct Item {
    reg: String,
    kind: ItemKind,
}

/// A level-88 condition-name: the index of the item it qualifies (its
/// "conditional variable") and the value-set that makes it true. The name holds
/// when the variable equals any single value or falls within any `THRU` range.
struct CondName {
    var: usize,
    values: Vec<ValueSpec>,
}

/// One item of a level-88 `VALUE` clause: a single value or an inclusive
/// `lo THRU hi` range.
enum ValueSpec {
    Single(Src),
    Range(Src, Src),
}

/// A value-test resolved to the variable's scaled `i64` representation, ready to
/// emit: equality with one value, or membership in an inclusive range.
enum ValueTest {
    Eq(i64),
    InRange(i64, i64),
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

/// A PROCEDURE DIVISION paragraph: its name and the statement nodes it holds,
/// borrowed from the CST so `PERFORM` can re-emit them inline.
struct Paragraph<'a> {
    name: String,
    stmts: Vec<&'a GrammarASTNode>,
}

/// The deepest `PERFORM` nesting we inline before refusing — a bound on both the
/// emitted code size and the compiler's own recursion. Real programs nest a
/// handful deep; a self- or mutually-`PERFORM`ing loop (which the tree-walk
/// oracle also caps) trips this as a clean error instead of unbounded expansion.
const MAX_PERFORM_DEPTH: usize = 64;

/// A hard ceiling on emitted instructions — a backstop against `PERFORM`
/// inlining blowing up the program (e.g. a paragraph that performs itself twice,
/// which the depth bound alone would let grow exponentially). Real programs are
/// nowhere near this; exceeding it is a clean error, not an out-of-memory.
const MAX_EMIT_INSTRS: usize = 300_000;

/// The largest number of primaries a single `COMPUTE` expression may contain
/// (parity with the oracle's cap). The grammar's `{ … }` repetition is *flat*,
/// so the parser's depth cap does not bound how *wide* a chain is — `A + A + …`
/// folds into a same-depth `AExpr` tree whose recursive evaluation and `Drop`
/// would otherwise overflow the native stack. Real expressions have a handful of
/// operands; exhausting this budget is a clean error.
const MAX_EXPR_OPERANDS: usize = 1024;

#[derive(Default)]
struct Compiler<'a> {
    instrs: Vec<IIRInstr>,
    /// Elementary items by COBOL data-name, in declaration order (for a stable
    /// init prologue) plus a name index for `MOVE` / `DISPLAY` / arithmetic.
    items: Vec<Item>,
    by_name: HashMap<String, usize>,
    /// Level-88 condition-names → the item they qualify and the value that makes
    /// them true. A bare `IF IS-OK` lowers to `items[var] == value`.
    conditions: HashMap<String, CondName>,
    /// Set when a numeric `DISPLAY` emits a call to the digit-print helper, so it
    /// is appended to the module.
    needs_print: bool,
    /// Set when a signed-numeric `DISPLAY` emits a call to the overpunch printer.
    needs_print_signed: bool,
    /// Unique-suffix counter for throwaway registers.
    tmp_counter: usize,
    /// PROCEDURE DIVISION paragraphs in document order, and a name → index map,
    /// for `GO TO` (jump to a paragraph label) and `PERFORM` (inline a range).
    paras: Vec<Paragraph<'a>>,
    para_index: HashMap<String, usize>,
    /// Current `PERFORM` inline depth (recursion / code-size bound).
    perform_depth: usize,
}

impl<'a> Compiler<'a> {
    fn emit(&mut self, op: &str, dest: Option<&str>, srcs: Vec<Operand>, type_hint: &str) {
        self.instrs.push(IIRInstr::new(op, dest.map(str::to_string), srcs, type_hint));
    }

    fn fresh(&mut self, prefix: &str) -> String {
        let n = self.tmp_counter;
        self.tmp_counter += 1;
        format!("{prefix}{n}")
    }

    fn emit_program(&mut self, program: &'a GrammarASTNode) -> Result<(), CompileError> {
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

        // 3. Collect the PROCEDURE DIVISION paragraphs (name + statement nodes),
        //    so `GO TO`/`PERFORM` can reference them, then emit each in document
        //    order behind its label. Control falls through paragraph to paragraph
        //    exactly as COBOL does; a `GO TO`/`PERFORM` redirects it.
        let pd = child_node(program, "procedure_division")
            .ok_or_else(|| CompileError::Malformed("program without a PROCEDURE DIVISION".into()))?;
        for para in child_nodes(pd, "paragraph") {
            let name = first_token(para, "NAME").unwrap_or_default();
            let stmts: Vec<&'a GrammarASTNode> = child_nodes(para, "sentence")
                .into_iter()
                .flat_map(|s| child_nodes(s, "statement"))
                .collect();
            if !name.is_empty() {
                self.para_index.insert(name.clone(), self.paras.len());
            }
            self.paras.push(Paragraph { name, stmts });
        }
        for idx in 0..self.paras.len() {
            if !self.paras[idx].name.is_empty() {
                let label = para_label(&self.paras[idx].name);
                self.emit("label", None, vec![Operand::Var(label)], "void");
            }
            let stmts = self.paras[idx].stmts.clone();
            for stmt in stmts {
                self.emit_statement(stmt)?;
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
        // A level-88 entry declares a boolean condition-name over the most recent
        // item (its "conditional variable"). It takes no storage and no picture —
        // register the name → (variable, value) and return.
        let level = first_token(entry, "NUMBER").and_then(|s| s.parse::<u32>().ok());
        if level == Some(88) {
            return self.collect_condition_name(&name, entry);
        }
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

        // A plain item's VALUE is a single literal; a multi-value or THRU-range
        // VALUE is only meaningful on a level-88 condition-name.
        let value_lit = match find_clause(entry, "value_clause") {
            Some(vc) => {
                let mut specs = read_value_specs(vc)?;
                match (specs.len(), specs.pop()) {
                    (1, Some(ValueSpec::Single(src))) => Some(src),
                    (0, _) => None,
                    _ => {
                        return Err(CompileError::Unsupported(
                            "a multi-value or THRU-range VALUE is only allowed on a level-88 entry"
                                .into(),
                        ))
                    }
                }
            }
            None => None,
        };

        let kind = match &picture {
            Picture::Numeric { int_digits, dec_digits, signed } => {
                if int_digits + dec_digits > NUMERIC_MAX_DIGITS {
                    return Err(CompileError::Unsupported(format!(
                        "numeric item {name} wider than {NUMERIC_MAX_DIGITS} digits exceeds the \
                         i64 value model — a later rung"
                    )));
                }
                // Initial scaled value: VALUE applied as an initialising MOVE
                // (else 0). We reuse the oracle's `move_into_numeric` to produce
                // the exact zero-filled magnitude digits, then read it as the
                // scaled integer — carrying the sign into a signed field.
                let initial = match &value_lit {
                    Some(src) => {
                        let digits = format_into_picture(src, &picture)
                            .map_err(|m| CompileError::Unsupported(format!("VALUE {name}: {m}")))?;
                        let mag = parse_digits(&digits);
                        if *signed && literal_is_negative(src) { -mag } else { mag }
                    }
                    None => 0,
                };
                ItemKind::Numeric {
                    int_digits: *int_digits,
                    dec_digits: *dec_digits,
                    signed: *signed,
                    initial,
                }
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

    /// Register a level-88 condition-name: it qualifies the item defined just
    /// before it and is true when that item holds any of the `VALUE` items (a
    /// single value or an inclusive `THRU` range).
    fn collect_condition_name(&mut self, name: &str, entry: &GrammarASTNode) -> Result<(), CompileError> {
        let vc = find_clause(entry, "value_clause")
            .ok_or_else(|| CompileError::Malformed(format!("level-88 {name} without a VALUE")))?;
        let values = read_value_specs(vc)?;
        if values.is_empty() {
            return Err(CompileError::Malformed(format!("level-88 {name} without a VALUE")));
        }
        let var = self.items.len().checked_sub(1).ok_or_else(|| {
            CompileError::Unsupported(format!("level-88 {name} must follow an item"))
        })?;
        self.conditions.insert(name.to_string(), CondName { var, values });
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
            "if_stmt" => self.emit_if(verb),
            "compute_stmt" => self.emit_compute(verb),
            "goto_stmt" => self.emit_goto(verb),
            "perform_stmt" => self.emit_perform(verb),
            "set_stmt" => self.emit_set(verb),
            "evaluate_stmt" => self.emit_evaluate(verb),
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
                            if self.item_signed(idx) {
                                self.emit_print_signed(&reg, width);
                            } else {
                                self.emit_print_numeric(&reg, width);
                            }
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
        let src = read_operand(src_node)?;
        let dsts: Vec<String> = child_tokens(verb)
            .into_iter()
            .filter(|(k, _)| k == "NAME")
            .map(|(_, v)| v)
            .collect();
        if dsts.is_empty() {
            return Err(CompileError::Malformed("MOVE without a receiver".into()));
        }
        for dst in dsts {
            match &src {
                Operandy::Literal(lit) => self.move_literal_into(lit, &dst)?,
                // Item-to-item MOVE reshapes the source into the receiver's
                // picture: numeric→numeric rescales the implied point; a
                // character move truncates or space-pads to the receiver's size.
                // Cross-category (numeric↔alphanumeric) moves are a later rung.
                Operandy::Name(name) => {
                    let src_idx = self.item_index(name)?;
                    let didx = self.item_index(&dst)?;
                    match (&self.items[src_idx].kind, &self.items[didx].kind) {
                        (ItemKind::Numeric { .. }, ItemKind::Numeric { .. }) => {
                            // Re-validate widths and rescale (truncating, never
                            // rounding) the source into the receiver's decimals.
                            self.numeric_index(name)?;
                            let (src_int, src_scale) = self.numeric_dims(src_idx);
                            let src_reg = self.items[src_idx].reg.clone();
                            self.store_scaled(&dst, &src_reg, src_scale, src_int, false)?;
                        }
                        (ItemKind::Char { .. }, ItemKind::Char { .. }) => {
                            self.move_char_item(src_idx, didx);
                        }
                        _ => {
                            return Err(CompileError::Unsupported(format!(
                                "cross-category MOVE from {name} into {dst} \
                                 (numeric↔alphanumeric) is a later rung"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `MOVE <literal> TO item` — the literal formatted into the receiver's
    /// picture at compile time and emitted as the register's constant.
    fn move_literal_into(&mut self, lit: &Src, dst: &str) -> Result<(), CompileError> {
        let idx = self.item_index(dst)?;
        let reg = self.items[idx].reg.clone();
        match &self.items[idx].kind {
            ItemKind::Char { .. } => {
                let picture = Picture::Alphanumeric { size: self.items[idx].width() };
                let image = format_into_picture(lit, &picture)
                    .map_err(|m| CompileError::Unsupported(format!("MOVE into {dst}: {m}")))?;
                self.emit("str_const", Some(&reg), vec![Operand::Str(image)], "str");
            }
            ItemKind::Numeric { int_digits, dec_digits, signed, .. } => {
                let signed = *signed;
                let picture = Picture::Numeric {
                    int_digits: *int_digits,
                    dec_digits: *dec_digits,
                    signed,
                };
                let digits = format_into_picture(lit, &picture)
                    .map_err(|m| CompileError::Unsupported(format!("MOVE into {dst}: {m}")))?;
                let mag = parse_digits(&digits);
                // A signed receiver keeps the sign; an unsigned one drops it.
                let value = if signed && literal_is_negative(lit) { -mag } else { mag };
                self.emit("const", Some(&reg), vec![Operand::Int(value)], "i64");
            }
        }
        Ok(())
    }

    /// `SET cond-name TO TRUE` — assign the condition-name's conditional variable
    /// the value that makes it hold: the **first** of its `VALUE` items (a range's
    /// low bound). The value is formatted into the variable's picture at compile
    /// time — the same `const`-into-slot store `MOVE <literal>` emits. Numeric
    /// variable only, matching the test path; an alphanumeric one is a later rung.
    fn emit_set(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let cond_name = first_token(verb, "NAME")
            .ok_or_else(|| CompileError::Malformed("SET without a condition-name".into()))?;
        let cn = self.conditions.get(&cond_name).ok_or_else(|| {
            CompileError::Unsupported(format!("reference to condition-name {cond_name} (undeclared)"))
        })?;
        let var = cn.var;
        let (int_digits, dec_digits, signed) = match &self.items[var].kind {
            ItemKind::Numeric { int_digits, dec_digits, signed, .. } => (*int_digits, *dec_digits, *signed),
            ItemKind::Char { .. } => {
                return Err(CompileError::Unsupported(
                    "SET … TO TRUE on an alphanumeric conditional variable is a later rung".into(),
                ))
            }
        };
        let picture = Picture::Numeric { int_digits, dec_digits, signed };
        let src = match cn.values.first() {
            Some(ValueSpec::Single(s)) | Some(ValueSpec::Range(s, _)) => s,
            None => {
                return Err(CompileError::Malformed(format!("condition-name {cond_name} has no VALUE")))
            }
        };
        let value = scale_num_value(src, &picture, signed, &cond_name)?;
        let reg = self.items[var].reg.clone();
        self.emit("const", Some(&reg), vec![Operand::Int(value)], "i64");
        Ok(())
    }

    /// `EVALUATE subject WHEN v… WHEN OTHER … END-EVALUATE` — COBOL's case
    /// statement, lowered as a branch cascade (a chain of `IF`s). Each value branch
    /// tests the subject against its value-list — a single value is `cmp_eq`, a
    /// `THRU` range is `and(cmp_ge, cmp_le)`, and several values `OR`-fold — exactly
    /// the level-88-ranges boolean machinery; a mismatch jumps to the next branch, a
    /// match runs the branch and jumps to the end (no fall-through). `WHEN OTHER`
    /// runs unconditionally once reached. Branches (and the values within each) are
    /// emitted by **iteration**, so thousands of `WHEN`s / values stay flat.
    /// Numeric subject/value this rung; an alphanumeric one is a later rung
    /// ([`Self::read_arith_term`] rejects it cleanly).
    fn emit_evaluate(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let subject_node = child_node(verb, "operand")
            .ok_or_else(|| CompileError::Malformed("EVALUATE without a subject".into()))?;
        let subject = self.read_arith_term(subject_node)?;
        let end_lbl = self.fresh("eval_end");
        for wb in child_nodes(verb, "when_branch") {
            let is_other = child_tokens(wb).iter().any(|(k, v)| k == "KEYWORD" && v == "OTHER");
            let stmts = child_nodes(wb, "statement");
            if is_other {
                for s in stmts {
                    self.emit_statement(s)?;
                }
                self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
                continue;
            }
            let cond = self.emit_when_match(&subject, wb)?;
            let next_lbl = self.fresh("when_next");
            self.emit("jmp_if_false", None, vec![Operand::Var(cond), Operand::Var(next_lbl.clone())], "void");
            for s in stmts {
                self.emit_statement(s)?;
            }
            self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
            self.emit("label", None, vec![Operand::Var(next_lbl)], "void");
        }
        self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
        Ok(())
    }

    /// Emit a boolean register that is true when the subject matches any value in a
    /// `when_branch`'s list: a single value → `cmp_eq(subject, value)`; a `THRU`
    /// range → `and(cmp_ge(subject, lo), cmp_le(subject, hi))`; the whole list
    /// `OR`-folds. Comparisons align the subject and each value to a common scale.
    fn emit_when_match(&mut self, subject: &Term, wb: &GrammarASTNode) -> Result<String, CompileError> {
        let mut acc: Option<String> = None;
        for wv in child_nodes(wb, "when_value") {
            let ops = child_nodes(wv, "operand");
            let b = match ops.as_slice() {
                [one] => {
                    let value = self.read_arith_term(one)?;
                    self.emit_scaled_cmp("cmp_eq", subject, &value)
                }
                [lo, hi] => {
                    let lo = self.read_arith_term(lo)?;
                    let hi = self.read_arith_term(hi)?;
                    let ge = self.emit_scaled_cmp("cmp_ge", subject, &lo);
                    let le = self.emit_scaled_cmp("cmp_le", subject, &hi);
                    let r = self.fresh("_wrng");
                    self.emit("and", Some(&r), vec![Operand::Var(ge), Operand::Var(le)], "i64");
                    r
                }
                _ => return Err(CompileError::Malformed("a WHEN value must be `operand` or `operand THRU operand`".into())),
            };
            acc = Some(match acc {
                None => b,
                Some(prev) => {
                    let r = self.fresh("_wor");
                    self.emit("or", Some(&r), vec![Operand::Var(prev), Operand::Var(b)], "i64");
                    r
                }
            });
        }
        acc.ok_or_else(|| CompileError::Malformed("WHEN without a value".into()))
    }

    /// Emit `op(left, right)` (a `cmp_*`) with both terms taken to a common scale,
    /// returning the boolean register.
    fn emit_scaled_cmp(&mut self, op: &str, left: &Term, right: &Term) -> String {
        let w = self.term_scale(left).max(self.term_scale(right));
        let a = self.emit_term_at_scale(left, w);
        let b = self.emit_term_at_scale(right, w);
        let out = self.fresh("_wcmp");
        self.emit(op, Some(&out), vec![a, b], "i64");
        out
    }

    /// `MOVE src-item TO recv-item` for two **character** items — reshape the
    /// source's stored image into the receiver's picture, exactly as the oracle's
    /// `move_into_char` does: the source is always its declared `m` characters, so
    /// a receiver of `n` characters keeps the leftmost `n` (truncating on the
    /// right) when `n ≤ m`, or left-justifies and space-pads on the right when
    /// `n > m`. Both sizes are known at compile time, so the reshape is a single
    /// fixed `str_slice` or `str_concat`.
    fn move_char_item(&mut self, src_idx: usize, didx: usize) {
        let m = self.items[src_idx].width();
        let n = self.items[didx].width();
        let src_reg = self.items[src_idx].reg.clone();
        let recv_reg = self.items[didx].reg.clone();
        if n <= m {
            let start = self.fresh("_s0");
            self.emit("const", Some(&start), vec![Operand::Int(0)], "i64");
            let end = self.fresh("_sn");
            self.emit("const", Some(&end), vec![Operand::Int(n as i64)], "i64");
            self.emit(
                "str_slice",
                Some(&recv_reg),
                vec![Operand::Var(src_reg), Operand::Var(start), Operand::Var(end)],
                "str",
            );
        } else {
            let pad = self.spaces_const(n - m);
            self.emit(
                "str_concat",
                Some(&recv_reg),
                vec![Operand::Var(src_reg), Operand::Var(pad)],
                "str",
            );
        }
    }

    /// A `str` register holding `k` spaces — the right-padding for a character
    /// reshape or comparison.
    fn spaces_const(&mut self, k: usize) -> String {
        let reg = self.fresh("_sp");
        self.emit("str_const", Some(&reg), vec![Operand::Str(" ".repeat(k))], "str");
        reg
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
    // Conditionals
    // -----------------------------------------------------------------------

    /// `IF condition then-stmts [ELSE else-stmts]` — a relational test over a
    /// three-way branch. The condition lowers to a boolean register; then a
    /// `jmp_if_false` skips the then-branch to the else-branch (a jump past it
    /// closes the then-branch). Nested `IF`s recurse through `emit_statement`.
    fn emit_if(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let cond_node = child_node(verb, "condition")
            .ok_or_else(|| CompileError::Malformed("IF without a condition".into()))?;
        let cond = self.emit_condition(cond_node)?;

        // Split the statement children at the ELSE keyword (as the oracle does).
        let mut then_stmts = Vec::new();
        let mut else_stmts = Vec::new();
        let mut seen_else = false;
        for child in &verb.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "ELSE" && t.effective_type_name() == "KEYWORD" => {
                    seen_else = true;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                    if seen_else {
                        else_stmts.push(n);
                    } else {
                        then_stmts.push(n);
                    }
                }
                _ => {}
            }
        }

        let else_lbl = self.fresh("if_else");
        let end_lbl = self.fresh("if_end");
        self.emit("jmp_if_false", None, vec![Operand::Var(cond), Operand::Var(else_lbl.clone())], "void");
        for stmt in then_stmts {
            self.emit_statement(stmt)?;
        }
        self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
        self.emit("label", None, vec![Operand::Var(else_lbl)], "void");
        for stmt in else_stmts {
            self.emit_statement(stmt)?;
        }
        self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
        Ok(())
    }

    /// Evaluate a `condition` to a boolean `i64` register (1 = true). A condition
    /// is either a `relation` (`operand relop operand`) or a bare level-88
    /// `condition_name`; dispatch on which the grammar produced.
    /// Evaluate a `condition` to a boolean `i64` register. A condition is a
    /// `disjunction` of `AND`-joined simple conditions; `AND` binds tighter than
    /// `OR`. Each leaf (relation / condition-name / parenthesised group) already
    /// yields a `0`/`1` boolean, so `AND`/`OR` fold with the bitwise `and`/`or`
    /// ops — exactly logical AND/OR on `0`/`1`, and byte-identical to the oracle's
    /// short-circuit `&&`/`||` because COBOL relations have no side effects and a
    /// comparison never faults (full evaluation gives the same boolean).
    fn emit_condition(&mut self, cond: &GrammarASTNode) -> Result<String, CompileError> {
        let disjunction = child_node(cond, "disjunction")
            .ok_or_else(|| CompileError::Malformed("empty condition".into()))?;
        self.emit_disjunction(disjunction)
    }

    /// `disjunction = conjunction { "OR" conjunction }` — fold with bitwise `or`.
    fn emit_disjunction(&mut self, node: &GrammarASTNode) -> Result<String, CompileError> {
        let mut acc: Option<String> = None;
        for child in child_nodes(node, "conjunction") {
            let b = self.emit_conjunction(child)?;
            acc = Some(match acc {
                None => b,
                Some(prev) => {
                    let r = self.fresh("_or");
                    self.emit("or", Some(&r), vec![Operand::Var(prev), Operand::Var(b)], "i64");
                    r
                }
            });
        }
        acc.ok_or_else(|| CompileError::Malformed("empty disjunction".into()))
    }

    /// `conjunction = negation { "AND" negation }` — fold with bitwise `and`.
    fn emit_conjunction(&mut self, node: &GrammarASTNode) -> Result<String, CompileError> {
        let mut acc: Option<String> = None;
        for child in child_nodes(node, "negation") {
            let b = self.emit_negation(child)?;
            acc = Some(match acc {
                None => b,
                Some(prev) => {
                    let r = self.fresh("_and");
                    self.emit("and", Some(&r), vec![Operand::Var(prev), Operand::Var(b)], "i64");
                    r
                }
            });
        }
        acc.ok_or_else(|| CompileError::Malformed("empty conjunction".into()))
    }

    /// `negation = [ "NOT" ] simple_condition` — a leading `NOT` inverts the
    /// boolean. The leaf yields a `0`/`1` value, so the inverse is `xor` with `1`
    /// (`0 ^ 1 = 1`, `1 ^ 1 = 0`) — a logical NOT. (IIR's `not` is *bitwise* `~x`,
    /// which would not map `0`/`1` to `1`/`0`, so `xor` is the right op.) The result
    /// is still `0`/`1` and feeds `jmp_if_false` like any other condition boolean.
    fn emit_negation(&mut self, node: &GrammarASTNode) -> Result<String, CompileError> {
        let simple = child_node(node, "simple_condition")
            .ok_or_else(|| CompileError::Malformed("negation without a condition".into()))?;
        let inner = self.emit_simple_condition(simple)?;
        let negated = child_tokens(node).iter().any(|(k, v)| k == "KEYWORD" && v == "NOT");
        if !negated {
            return Ok(inner);
        }
        let one = self.fresh("_one");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let out = self.fresh("_not");
        self.emit("xor", Some(&out), vec![Operand::Var(inner), Operand::Var(one)], "i64");
        Ok(out)
    }

    /// `simple_condition = relation | condition_name | "(" condition ")"`.
    fn emit_simple_condition(&mut self, node: &GrammarASTNode) -> Result<String, CompileError> {
        if let Some(relation) = child_node(node, "relation") {
            return self.emit_relation(relation);
        }
        if let Some(cn) = child_node(node, "condition_name") {
            let name = first_token(cn, "NAME")
                .ok_or_else(|| CompileError::Malformed("condition-name without a NAME".into()))?;
            return self.emit_condition_name(&name);
        }
        if let Some(inner) = child_node(node, "condition") {
            return self.emit_condition(inner);
        }
        Err(CompileError::Malformed(
            "condition must be a relation, condition-name, or parenthesised".into(),
        ))
    }

    /// Evaluate a `relation` (`operand relop operand`) to a boolean `i64` register.
    /// A **numeric** comparison aligns the operands to a common scale and applies
    /// `cmp_gt`/`cmp_lt`/`cmp_eq` (`NOT` inverts the relation); an **alphanumeric**
    /// comparison space-pads both sides to a common length and applies `str_cmp`
    /// (COBOL's rule). A numeric operand compared with an alphanumeric one is a
    /// later rung.
    fn emit_relation(&mut self, relation: &GrammarASTNode) -> Result<String, CompileError> {
        let operands = child_nodes(relation, "operand");
        if operands.len() != 2 {
            return Err(CompileError::Malformed("relation must be operand relop operand".into()));
        }
        let op = self.relation_op(relation)?;

        // Classify each operand: `None` = numeric (literal / numeric item / a
        // numeric figurative), `Some` = a character value.
        let ls = self.str_operand(operands[0])?;
        let rs = self.str_operand(operands[1])?;
        match (ls, rs) {
            (None, None) => {
                let left = self.read_arith_term(operands[0])?;
                let right = self.read_arith_term(operands[1])?;
                let w = self.term_scale(&left).max(self.term_scale(&right));
                let a = self.emit_term_at_scale(&left, w);
                let b = self.emit_term_at_scale(&right, w);
                let cond_reg = self.fresh("_cond");
                self.emit(op, Some(&cond_reg), vec![a, b], "i64");
                Ok(cond_reg)
            }
            (Some(a), Some(b)) => self.emit_str_condition(a, b, op),
            _ => Err(CompileError::Unsupported(
                "comparing a numeric operand with an alphanumeric one is a later rung".into(),
            )),
        }
    }

    /// Evaluate a level-88 condition-name to a boolean `i64` register: does its
    /// conditional variable equal any single value, or fall within any inclusive
    /// `THRU` range? This rung compares a **numeric** variable against numeric
    /// values; an alphanumeric conditional variable is a later rung.
    ///
    /// Each value-item becomes one boolean (`cmp_eq` for a single value; `and` of
    /// `cmp_ge`/`cmp_le` for a range) and they are OR-folded with `or` — because
    /// each `cmp_*` yields `0`/`1`, bitwise `and`/`or` are exactly logical AND/OR,
    /// and the combined `i64` feeds `jmp_if_false` like any relational condition.
    fn emit_condition_name(&mut self, name: &str) -> Result<String, CompileError> {
        // Phase 1 (immutable): resolve the variable and scale every value into the
        // slot's representation, so the emitted constants compare exactly.
        let cn = self.conditions.get(name).ok_or_else(|| {
            CompileError::Unsupported(format!("reference to condition-name {name} (undeclared)"))
        })?;
        let var = cn.var;
        let (int_digits, dec_digits, signed) = match &self.items[var].kind {
            ItemKind::Numeric { int_digits, dec_digits, signed, .. } => (*int_digits, *dec_digits, *signed),
            ItemKind::Char { .. } => {
                return Err(CompileError::Unsupported(
                    "a level-88 condition-name on an alphanumeric item is a later rung".into(),
                ))
            }
        };
        let picture = Picture::Numeric { int_digits, dec_digits, signed };
        let mut tests: Vec<ValueTest> = Vec::with_capacity(cn.values.len());
        for spec in &cn.values {
            tests.push(match spec {
                ValueSpec::Single(src) => ValueTest::Eq(scale_num_value(src, &picture, signed, name)?),
                ValueSpec::Range(lo, hi) => ValueTest::InRange(
                    scale_num_value(lo, &picture, signed, name)?,
                    scale_num_value(hi, &picture, signed, name)?,
                ),
            });
        }
        let slot = self.items[var].reg.clone();

        // Phase 2 (mutable): emit one boolean per test, OR-folded.
        let mut acc: Option<String> = None;
        for test in tests {
            let b = self.emit_value_test(&slot, test);
            acc = Some(match acc {
                None => b,
                Some(prev) => {
                    let or = self.fresh("_c88or");
                    self.emit("or", Some(&or), vec![Operand::Var(prev), Operand::Var(b)], "i64");
                    or
                }
            });
        }
        // `values` is non-empty (enforced at registration), so `acc` is always set.
        acc.ok_or_else(|| CompileError::Malformed(format!("level-88 {name} has no VALUE")))
    }

    /// Emit one value-test against the variable's slot, returning a `0`/`1`
    /// boolean register: `cmp_eq` for a single value, or `and(cmp_ge, cmp_le)` for
    /// an inclusive range.
    fn emit_value_test(&mut self, slot: &str, test: ValueTest) -> String {
        match test {
            ValueTest::Eq(v) => {
                let vreg = self.fresh("_c88");
                self.emit("const", Some(&vreg), vec![Operand::Int(v)], "i64");
                let b = self.fresh("_c88eq");
                self.emit("cmp_eq", Some(&b), vec![Operand::Var(slot.to_string()), Operand::Var(vreg)], "i64");
                b
            }
            ValueTest::InRange(lo, hi) => {
                let loreg = self.fresh("_c88lo");
                self.emit("const", Some(&loreg), vec![Operand::Int(lo)], "i64");
                let ge = self.fresh("_c88ge");
                self.emit("cmp_ge", Some(&ge), vec![Operand::Var(slot.to_string()), Operand::Var(loreg)], "i64");
                let hireg = self.fresh("_c88hi");
                self.emit("const", Some(&hireg), vec![Operand::Int(hi)], "i64");
                let le = self.fresh("_c88le");
                self.emit("cmp_le", Some(&le), vec![Operand::Var(slot.to_string()), Operand::Var(hireg)], "i64");
                let b = self.fresh("_c88rng");
                self.emit("and", Some(&b), vec![Operand::Var(ge), Operand::Var(le)], "i64");
                b
            }
        }
    }

    /// Parse a `relop` node to the `cmp_*` op the relation lowers to. Each operator
    /// resolves to a base relation plus a *baseline* negation — the symbols `>=`,
    /// `<=`, `<>` already mean "not <", "not >", "not =" — and a written `NOT`
    /// composes with that baseline by XOR. `NOT` inverts the *relation* directly
    /// (`GREATER` → `cmp_le`, …), so the op still yields one boolean the
    /// `cmp_*`/`str_cmp`-vs-0 result `jmp_if_false` consumes; inverting the boolean
    /// itself with `cmp_eq … 0` would be a type mismatch (see
    /// [`Self::emit_relation`]).
    fn relation_op(&self, cond: &GrammarASTNode) -> Result<&'static str, CompileError> {
        let relop = child_node(cond, "relop")
            .ok_or_else(|| CompileError::Malformed("condition without a relational operator".into()))?;
        let toks = child_tokens(relop);
        let explicit_not = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "NOT");
        let (base, baseline_neg) = toks
            .iter()
            .find_map(|(k, v)| match (k.as_str(), v.as_str()) {
                ("KEYWORD", "GREATER") | ("GT", _) => Some(("GREATER", false)),
                ("KEYWORD", "LESS") | ("LT", _) => Some(("LESS", false)),
                ("KEYWORD", "EQUAL") | ("EQ", _) => Some(("EQUAL", false)),
                ("GE", _) => Some(("LESS", true)),
                ("LE", _) => Some(("GREATER", true)),
                ("NE", _) => Some(("EQUAL", true)),
                _ => None,
            })
            .ok_or_else(|| CompileError::Malformed("unrecognised relational operator".into()))?;
        Ok(match (base, explicit_not ^ baseline_neg) {
            ("GREATER", false) => "cmp_gt",
            ("GREATER", true) => "cmp_le",
            ("LESS", false) => "cmp_lt",
            ("LESS", true) => "cmp_ge",
            ("EQUAL", false) => "cmp_eq",
            _ => "cmp_ne", // ("EQUAL", true)
        })
    }

    /// Read a `condition` operand for an **alphanumeric** comparison. Returns
    /// `None` for a numeric operand (a numeric literal or numeric item — those
    /// take the numeric path). A character item or string literal is a
    /// fixed-length string; `SPACE`/`ZERO` figuratives become fills whose length
    /// is resolved from the other operand (COBOL's rule).
    fn str_operand(&mut self, op: &GrammarASTNode) -> Result<Option<StrOperand>, CompileError> {
        match read_operand(op)? {
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Char { .. } => {
                        let len = self.items[idx].width();
                        Ok(Some(StrOperand::Fixed { reg: self.items[idx].reg.clone(), len }))
                    }
                    ItemKind::Numeric { .. } => Ok(None),
                }
            }
            Operandy::Literal(Src::Str(s)) => {
                let len = s.len();
                let reg = self.fresh("_sl");
                self.emit("str_const", Some(&reg), vec![Operand::Str(s)], "str");
                Ok(Some(StrOperand::Fixed { reg, len }))
            }
            Operandy::Literal(Src::Space) => Ok(Some(StrOperand::Fig(' '))),
            // `ZERO` is numeric against a numeric operand but the digit `'0'`
            // against an alphanumeric one; carry it as a figurative and let the
            // pairing in `emit_condition` decide.
            Operandy::Literal(Src::Zero) => Ok(Some(StrOperand::Fig('0'))),
            Operandy::Literal(Src::Num(_)) => Ok(None),
        }
    }

    /// Emit an alphanumeric comparison: resolve any figurative to the other
    /// operand's length, space-pad both sides to their common (max) length, then
    /// `str_cmp` and apply the relation against zero. `str_cmp` returns an `i64`
    /// ordering (−1/0/1), so `cmp_* … 0` is an integer comparison (no `Bool`
    /// mismatch). Two figuratives with no fixed length to borrow is a later rung.
    fn emit_str_condition(
        &mut self,
        a: StrOperand,
        b: StrOperand,
        op: &str,
    ) -> Result<String, CompileError> {
        let ((a_reg, a_len), (b_reg, b_len)) = match (a, b) {
            (StrOperand::Fixed { reg: ar, len: al }, StrOperand::Fixed { reg: br, len: bl }) => {
                ((ar, al), (br, bl))
            }
            (StrOperand::Fixed { reg: ar, len: al }, StrOperand::Fig(c)) => {
                ((ar, al), (self.fig_const(c, al), al))
            }
            (StrOperand::Fig(c), StrOperand::Fixed { reg: br, len: bl }) => {
                ((self.fig_const(c, bl), bl), (br, bl))
            }
            (StrOperand::Fig(_), StrOperand::Fig(_)) => {
                return Err(CompileError::Unsupported(
                    "comparing two figurative constants is a later rung".into(),
                ));
            }
        };
        let width = a_len.max(b_len);
        let ap = self.pad_spaces(a_reg, a_len, width);
        let bp = self.pad_spaces(b_reg, b_len, width);
        let cmp = self.fresh("_scmp");
        self.emit("str_cmp", Some(&cmp), vec![Operand::Var(ap), Operand::Var(bp)], "i64");
        let zero = self.fresh("_z");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        let cond_reg = self.fresh("_cond");
        self.emit(op, Some(&cond_reg), vec![Operand::Var(cmp), Operand::Var(zero)], "i64");
        Ok(cond_reg)
    }

    /// A `str` register holding character `c` repeated `len` times — a figurative
    /// constant expanded to the length its comparison partner requires.
    fn fig_const(&mut self, c: char, len: usize) -> String {
        let reg = self.fresh("_fig");
        self.emit("str_const", Some(&reg), vec![Operand::Str(c.to_string().repeat(len))], "str");
        reg
    }

    /// Right-pad `reg` (currently `len` characters) with spaces to `width`,
    /// returning the register to compare. When already at `width`, `reg` is used
    /// as-is.
    fn pad_spaces(&mut self, reg: String, len: usize, width: usize) -> String {
        if len >= width {
            return reg;
        }
        let pad = self.spaces_const(width - len);
        let out = self.fresh("_pad");
        self.emit("str_concat", Some(&out), vec![Operand::Var(reg), Operand::Var(pad)], "str");
        out
    }

    // -----------------------------------------------------------------------
    // COMPUTE — arithmetic expressions with operator precedence
    // -----------------------------------------------------------------------

    /// `COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR …]`.
    ///
    /// The expression is evaluated in the same scaled-`i64` model as the other
    /// verbs, bottom-up: every node carries a compile-time `(scale, int_bound)`,
    /// and each combination is guarded so the `i64` can never silently wrap — an
    /// expression whose intermediate could exceed 18 digits is a clean
    /// [`CompileError::Unsupported`], never wrong output. `+ - *` and unary minus
    /// evaluate **exactly** (matching the oracle's exact `Decimal`); a
    /// **top-level** division reproduces the DIVIDE verb's one-guard-digit
    /// rounding (already proven byte-identical to the oracle), including its
    /// zero-divisor → size-error branch. A division **nested** inside a larger
    /// expression reproduces the oracle's fixed scale-12 intermediate (see
    /// [`Self::eval_div_nested`]); `**` with a compile-time non-negative integer
    /// exponent unrolls to repeated multiplication (see [`Self::eval_pow`]). A `**`
    /// with a variable, negative, fractional, or oversized exponent — and a
    /// COMPUTE that pairs `ON SIZE ERROR` with a nested division — are each a
    /// later rung.
    fn emit_compute(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let target = first_token(verb, "NAME")
            .ok_or_else(|| CompileError::Malformed("COMPUTE without a receiver".into()))?;
        // The receiver must be a numeric item within the arithmetic width.
        self.numeric_index(&target)?;
        let rounded = has_rounded(verb);
        let handler = size_error_handler(verb);
        let expr_node = child_node(verb, "arith_expr")
            .ok_or_else(|| CompileError::Malformed("COMPUTE without an expression".into()))?;
        let expr = self.parse_compute(expr_node)?;

        // A top-level division is the DIVIDE verb in disguise — route it through
        // the same scale/rounding/zero-divisor machinery so it matches the oracle.
        if let AExpr::Div(dividend, divisor) = expr {
            return self.emit_compute_div(&target, &dividend, &divisor, rounded, &handler);
        }

        // A division *nested* inside a larger expression lowers (see
        // [`Self::eval_div_nested`]) but its zero-divisor is a hard fault, not a
        // routed size error. So a COMPUTE that both carries an `ON SIZE ERROR`
        // handler and contains a nested division stays a later rung — the handler
        // could not catch a zero divisor buried mid-expression without wrapping
        // the whole evaluation in a skip, which this rung does not do.
        if !handler.is_empty() && aexpr_contains_div(&expr) {
            return Err(CompileError::Unsupported(
                "a COMPUTE with ON SIZE ERROR and a nested division is a later rung".into(),
            ));
        }

        let val = self.eval_aexpr(&expr)?;
        self.store_scaled_handled(&target, &val.reg, val.scale, val.int_bound, rounded, &handler)
    }

    /// Lower a (non-top-level-division) `COMPUTE` expression to an [`Eval`]:
    /// an `i64` register plus its `(scale, int_bound)`. Every combining step is
    /// overflow-guarded, so a result that could exceed the `i64` model is a clean
    /// error rather than a silent wrap.
    fn eval_aexpr(&mut self, e: &AExpr) -> Result<Eval, CompileError> {
        match e {
            AExpr::Num(d) => {
                let scale = d.frac.len();
                let reg = self.fresh("_cn");
                self.emit("const", Some(&reg), vec![Operand::Int(decimal_scaled_to(d, scale))], "i64");
                Ok(Eval { reg, scale, int_bound: d.int.trim_start_matches('0').len() })
            }
            AExpr::Var(idx) => {
                let (int_digits, dec_digits) = self.numeric_dims(*idx);
                // Copy the slot so the expression never clobbers the live item.
                let slot = self.items[*idx].reg.clone();
                let reg = self.fresh("_v");
                self.emit("mov", Some(&reg), vec![Operand::Var(slot)], "i64");
                Ok(Eval { reg, scale: dec_digits, int_bound: int_digits })
            }
            AExpr::Neg(inner) => {
                let v = self.eval_aexpr(inner)?;
                let zero = self.fresh("_z");
                self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                let out = self.fresh("_neg");
                self.emit("sub", Some(&out), vec![Operand::Var(zero), Operand::Var(v.reg)], "i64");
                Ok(Eval { reg: out, scale: v.scale, int_bound: v.int_bound })
            }
            AExpr::Add(l, r) | AExpr::Sub(l, r) => {
                let is_sub = matches!(e, AExpr::Sub(..));
                let a = self.eval_aexpr(l)?;
                let b = self.eval_aexpr(r)?;
                // Align both to a common scale (up-scaling is exact); the sum's
                // integer part is at most one digit wider than the wider operand.
                let w = a.scale.max(b.scale);
                let int_bound = a.int_bound.max(b.int_bound) + 1;
                if int_bound + w > NUMERIC_MAX_DIGITS {
                    return Err(CompileError::Unsupported(
                        "a COMPUTE add/subtract whose result could exceed 18 digits is a later rung".into(),
                    ));
                }
                let ar = self.scaled_up(&a, w);
                let br = self.scaled_up(&b, w);
                let out = self.fresh("_sum");
                self.emit(if is_sub { "sub" } else { "add" }, Some(&out), vec![Operand::Var(ar), Operand::Var(br)], "i64");
                Ok(Eval { reg: out, scale: w, int_bound })
            }
            AExpr::Mul(l, r) => {
                let a = self.eval_aexpr(l)?;
                let b = self.eval_aexpr(r)?;
                // Scales add; the product's magnitude is `< 10^(int+scale sum)`.
                let scale = a.scale + b.scale;
                let int_bound = a.int_bound + b.int_bound;
                if int_bound + scale > NUMERIC_MAX_DIGITS {
                    return Err(CompileError::Unsupported(
                        "a COMPUTE multiply whose product could exceed 18 digits is a later rung".into(),
                    ));
                }
                let out = self.fresh("_prod");
                self.emit("mul", Some(&out), vec![Operand::Var(a.reg), Operand::Var(b.reg)], "i64");
                Ok(Eval { reg: out, scale, int_bound })
            }
            AExpr::Div(l, r) => self.eval_div_nested(l, r),
            AExpr::Pow(base, exponent) => self.eval_pow(base, exponent),
        }
    }

    /// A division **nested** inside a larger COMPUTE expression. The oracle
    /// evaluates every COMPUTE division at a fixed intermediate precision of
    /// [`COMPUTE_DIV_SCALE`] fractional digits (its `div(a, b, 12)`), truncating
    /// toward zero, then lets the surrounding operators combine that scale-12
    /// value exactly — so to stay byte-identical this reproduces exactly that
    /// scale-12 quotient:
    ///
    /// ```text
    ///   numerator   = a · 10^(b.scale + 12)
    ///   denominator = b · 10^(a.scale)
    ///   quotient    = numerator / denominator      (i64 div, truncates toward 0)
    /// ```
    ///
    /// The result carries scale 12. Because dividing by a fraction (`b < 1`) can
    /// *grow* the integer part, the quotient's integer bound is `a.int_bound +
    /// a.scale + b.scale` (its magnitude is `< 10^(that + 12)`). The oracle keeps
    /// this exact in `i128`; here the intermediates are `i64`, so a case whose
    /// numerator or denominator could exceed the 18-digit model is a clean later
    /// rung — never a silent wrap. A zero divisor faults the emitted `div`,
    /// matching the oracle's hard `DivideByZero` (the handler-present case is
    /// filtered out earlier in [`Self::emit_compute`]).
    fn eval_div_nested(&mut self, l: &AExpr, r: &AExpr) -> Result<Eval, CompileError> {
        let a = self.eval_aexpr(l)?;
        let b = self.eval_aexpr(r)?;
        let result_scale = COMPUTE_DIV_SCALE;
        // numerator magnitude < 10^(a.int_bound + a.scale + b.scale + 12).
        let num_digits = a.int_bound + a.scale + b.scale + result_scale;
        // denominator magnitude < 10^(b.int_bound + b.scale + a.scale).
        let den_digits = b.int_bound + b.scale + a.scale;
        if num_digits > NUMERIC_MAX_DIGITS || den_digits > NUMERIC_MAX_DIGITS {
            return Err(CompileError::Unsupported(
                "a COMPUTE nested-division intermediate at scale 12 could overflow i64 — \
                 a later rung"
                    .into(),
            ));
        }
        // numerator = a · 10^(b.scale + 12).
        let numerator = self.scaled_by_pow10(&a.reg, b.scale + result_scale);
        // denominator = b · 10^(a.scale).
        let denominator = self.scaled_by_pow10(&b.reg, a.scale);
        let quot = self.fresh("_ndiv");
        // i64 division truncates toward zero, as COBOL (and the oracle) does.
        self.emit("div", Some(&quot), vec![Operand::Var(numerator), Operand::Var(denominator)], "i64");
        Ok(Eval { reg: quot, scale: result_scale, int_bound: a.int_bound + a.scale + b.scale })
    }

    /// A register holding `reg · 10^p` (reusing `reg` when `p == 0`). The caller
    /// guarantees the product fits the `i64` model.
    fn scaled_by_pow10(&mut self, reg: &str, p: usize) -> String {
        if p == 0 {
            return reg.to_string();
        }
        let pr = self.fresh("_p10");
        self.emit("const", Some(&pr), vec![Operand::Int(10i64.pow(p as u32))], "i64");
        let out = self.fresh("_scl");
        self.emit("mul", Some(&out), vec![Operand::Var(reg.to_string()), Operand::Var(pr)], "i64");
        out
    }

    /// `base ** exponent` where the exponent is a **compile-time non-negative
    /// integer** `e`. The oracle computes `base**e` by multiplying `1` by `base`
    /// `e` times, so the result's magnitude is `base_scaled^e` and its scale is
    /// `e · base.scale` — exactly what unrolling `e−1` register multiplies of the
    /// base gives. A variable, negative, fractional, or oversized (`> MAX_POW_EXP`)
    /// exponent is a clean later rung, matching the oracle's `pow` returning
    /// `None`. `e = 0` yields the constant `1` regardless of the base (never
    /// evaluated — COBOL's `x ** 0 = 1`, and the oracle never touches `base`).
    fn eval_pow(&mut self, base: &AExpr, exponent: &AExpr) -> Result<Eval, CompileError> {
        let e = const_nonneg_int(exponent).ok_or_else(|| {
            CompileError::Unsupported(
                "COMPUTE ** with a non-constant, negative, or fractional exponent is a later rung"
                    .into(),
            )
        })?;
        if e > MAX_POW_EXP {
            return Err(CompileError::Unsupported(
                "COMPUTE ** with an exponent past the oracle's limit is a later rung".into(),
            ));
        }
        // `x ** 0 = 1` — an exact integer one at scale 0. The base is not
        // evaluated, matching the oracle (which returns `1` without reading it).
        if e == 0 {
            let reg = self.fresh("_pow0");
            self.emit("const", Some(&reg), vec![Operand::Int(1)], "i64");
            return Ok(Eval { reg, scale: 0, int_bound: 1 });
        }
        // The result carries `e` copies of the base's scale and integer bound;
        // guard the widest intermediate (the final product) against the 18-digit
        // model so the `i64` can never silently wrap.
        let b = self.eval_aexpr(base)?;
        let e = e as usize;
        let scale = b.scale * e;
        let int_bound = b.int_bound * e;
        if int_bound + scale > NUMERIC_MAX_DIGITS {
            return Err(CompileError::Unsupported(
                "a COMPUTE ** whose result could exceed 18 digits is a later rung".into(),
            ));
        }
        // Unroll: acc = base, then multiply by the base `e − 1` more times. Each
        // product is `base_scaled^k`, the scaled representation at scale `k·base.scale`.
        let mut acc = b.reg.clone();
        for _ in 1..e {
            let out = self.fresh("_pow");
            self.emit("mul", Some(&out), vec![Operand::Var(acc), Operand::Var(b.reg.clone())], "i64");
            acc = out;
        }
        Ok(Eval { reg: acc, scale, int_bound })
    }

    /// Return a register holding `v`'s value at working scale `w` (`w ≥ v.scale`,
    /// so scaling up is exact). Reuses `v`'s own register when already at `w`.
    fn scaled_up(&mut self, v: &Eval, w: usize) -> String {
        if v.scale == w {
            return v.reg.clone();
        }
        let p = 10i64.pow((w - v.scale) as u32);
        let pr = self.fresh("_up");
        self.emit("const", Some(&pr), vec![Operand::Int(p)], "i64");
        let out = self.fresh("_us");
        self.emit("mul", Some(&out), vec![Operand::Var(v.reg.clone()), Operand::Var(pr)], "i64");
        out
    }

    /// A top-level `COMPUTE r = <dividend> / <divisor>`: evaluate both operands
    /// exactly, then divide at the receiver's precision (plus a guard digit when
    /// `ROUNDED`) — the very computation the DIVIDE verb performs, so the result
    /// (including its half-away rounding) is byte-identical to the oracle. A zero
    /// divisor is a size error: a handler catches it before the faulting division;
    /// without one the emitted `div` faults, matching the oracle's DivideByZero.
    fn emit_compute_div(
        &mut self,
        target: &str,
        dividend: &AExpr,
        divisor: &AExpr,
        rounded: bool,
        handler: &[&GrammarASTNode],
    ) -> Result<(), CompileError> {
        let num = self.eval_aexpr(dividend)?;
        let den = self.eval_aexpr(divisor)?;
        let recv_dec = self.numeric_dims(self.numeric_index(target)?).1;
        let w = recv_dec + usize::from(rounded);

        // quotient at scale w = (num · 10^e) / den, where e = den.scale + w − num.scale.
        if den.scale + w < num.scale {
            return Err(CompileError::Unsupported(
                "a COMPUTE division whose dividend has more fractional digits than the result \
                 precision is a later rung"
                    .into(),
            ));
        }
        let e = den.scale + w - num.scale;
        // numerator < 10^(num.int_bound + num.scale + e) = 10^(num.int_bound + den.scale + w).
        if num.int_bound + den.scale + w > NUMERIC_MAX_DIGITS {
            return Err(CompileError::Unsupported(
                "a COMPUTE division intermediate at the requested precision could overflow i64 — \
                 a later rung"
                    .into(),
            ));
        }

        let numerator = if e > 0 {
            let pr = self.fresh("_dp");
            self.emit("const", Some(&pr), vec![Operand::Int(10i64.pow(e as u32))], "i64");
            let n = self.fresh("_num");
            self.emit("mul", Some(&n), vec![Operand::Var(num.reg), Operand::Var(pr)], "i64");
            n
        } else {
            num.reg
        };

        let end_lbl = if handler.is_empty() {
            None
        } else {
            let zero = self.fresh("_z");
            self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
            let iszero = self.fresh("_dz");
            self.emit("cmp_eq", Some(&iszero), vec![Operand::Var(den.reg.clone()), Operand::Var(zero)], "i64");
            let (cont, end) = (self.fresh("dz_cont"), self.fresh("dz_end"));
            self.emit("jmp_if_false", None, vec![Operand::Var(iszero), Operand::Var(cont.clone())], "void");
            for h in handler {
                self.emit_statement(h)?;
            }
            self.emit("jmp", None, vec![Operand::Var(end.clone())], "void");
            self.emit("label", None, vec![Operand::Var(cont)], "void");
            Some(end)
        };

        let quot = self.fresh("_quot");
        // i64 division truncates toward zero, as COBOL does.
        self.emit("div", Some(&quot), vec![Operand::Var(numerator), Operand::Var(den.reg)], "i64");
        self.store_scaled_handled(target, &quot, w, num.int_bound + den.scale, rounded, handler)?;
        if let Some(end) = end_lbl {
            self.emit("label", None, vec![Operand::Var(end)], "void");
        }
        Ok(())
    }

    // -- COMPUTE expression parser (mirrors the oracle's precedence cascade) --

    /// Parse a `COMPUTE` expression tree, bounding its operand count against a
    /// stack-overflow DoS (parity with the oracle's [`MAX_EXPR_OPERANDS`]).
    fn parse_compute(&self, node: &GrammarASTNode) -> Result<AExpr, CompileError> {
        let mut budget = MAX_EXPR_OPERANDS;
        self.read_compute_expr(node, &mut budget)
    }

    /// `arith_expr = arith_term { ( "+" | "-" ) arith_term }` — additive,
    /// left-associative.
    fn read_compute_expr(&self, node: &GrammarASTNode, budget: &mut usize) -> Result<AExpr, CompileError> {
        let mut expr: Option<AExpr> = None;
        let mut sub_pending: Option<bool> = None; // Some(true) = subtract
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.read_compute_term(n, budget)?;
                    expr = Some(match (expr.take(), sub_pending.take()) {
                        (Some(left), Some(is_sub)) => {
                            let (l, r) = (Box::new(left), Box::new(operand));
                            if is_sub { AExpr::Sub(l, r) } else { AExpr::Add(l, r) }
                        }
                        _ => operand,
                    });
                }
                ASTNodeOrToken::Token(t) => match t.effective_type_name() {
                    "PLUS" => sub_pending = Some(false),
                    "MINUS" => sub_pending = Some(true),
                    _ => {}
                },
            }
        }
        expr.ok_or_else(|| CompileError::Malformed("empty COMPUTE expression".into()))
    }

    /// `arith_term = arith_factor { ( "*" | "/" ) arith_factor }` — multiplicative,
    /// left-associative.
    fn read_compute_term(&self, node: &GrammarASTNode, budget: &mut usize) -> Result<AExpr, CompileError> {
        let mut expr: Option<AExpr> = None;
        let mut div_pending: Option<bool> = None; // Some(true) = divide
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.read_compute_factor(n, budget)?;
                    expr = Some(match (expr.take(), div_pending.take()) {
                        (Some(left), Some(is_div)) => {
                            let (l, r) = (Box::new(left), Box::new(operand));
                            if is_div { AExpr::Div(l, r) } else { AExpr::Mul(l, r) }
                        }
                        _ => operand,
                    });
                }
                ASTNodeOrToken::Token(t) => match t.effective_type_name() {
                    "STAR" => div_pending = Some(false),
                    "SLASH" => div_pending = Some(true),
                    _ => {}
                },
            }
        }
        expr.ok_or_else(|| CompileError::Malformed("empty COMPUTE term".into()))
    }

    /// `arith_factor = arith_unary { "**" arith_unary }` — exponentiation, folded
    /// right-associatively so `A ** B ** C` = `A ** (B ** C)` (matching the oracle).
    fn read_compute_factor(&self, node: &GrammarASTNode, budget: &mut usize) -> Result<AExpr, CompileError> {
        let units = child_nodes(node, "arith_unary");
        if units.is_empty() {
            return Err(CompileError::Malformed("empty COMPUTE factor".into()));
        }
        // Read every operand (charging the stack-overflow budget identically to
        // the oracle), then fold **right-associatively**: `A ** B ** C` becomes
        // `A ** (B ** C)`, matching the oracle's right-to-left `**`.
        let mut operands = Vec::with_capacity(units.len());
        for u in &units {
            operands.push(self.read_compute_unary(u, budget)?);
        }
        let mut acc = operands.pop().expect("factor has at least one operand");
        while let Some(base) = operands.pop() {
            acc = AExpr::Pow(Box::new(base), Box::new(acc));
        }
        Ok(acc)
    }

    /// `arith_unary = [ "+" | "-" ] arith_primary` — a leading minus negates.
    fn read_compute_unary(&self, node: &GrammarASTNode, budget: &mut usize) -> Result<AExpr, CompileError> {
        let neg = child_tokens(node).iter().any(|(k, _)| k == "MINUS");
        let prim = child_node(node, "arith_primary")
            .ok_or_else(|| CompileError::Malformed("unary operator without an operand".into()))?;
        let e = self.read_compute_primary(prim, budget)?;
        Ok(if neg { AExpr::Neg(Box::new(e)) } else { e })
    }

    /// `arith_primary = NUMBER | NAME | "(" arith_expr ")"`. Charges one unit of
    /// the operand budget (the stack-overflow backstop).
    fn read_compute_primary(&self, node: &GrammarASTNode, budget: &mut usize) -> Result<AExpr, CompileError> {
        *budget = budget
            .checked_sub(1)
            .ok_or_else(|| CompileError::Unsupported("COMPUTE expression too large".into()))?;
        if let Some(inner) = child_node(node, "arith_expr") {
            return self.read_compute_expr(inner, budget);
        }
        for (k, v) in child_tokens(node) {
            match k.as_str() {
                "NUMBER" => {
                    let d = Decimal::parse_literal(&v)
                        .ok_or_else(|| CompileError::Malformed(format!("numeric literal {v}")))?;
                    if d.int.trim_start_matches('0').len() + d.frac.len() > ARITH_MAX_DIGITS {
                        return Err(CompileError::Unsupported(format!(
                            "numeric literal {v} wider than {ARITH_MAX_DIGITS} digits in COMPUTE is a later rung"
                        )));
                    }
                    return Ok(AExpr::Num(d));
                }
                "NAME" => return Ok(AExpr::Var(self.numeric_index(&v)?)),
                _ => {}
            }
        }
        Err(CompileError::Malformed("empty COMPUTE primary".into()))
    }

    // -----------------------------------------------------------------------
    // Control flow: GO TO / PERFORM
    // -----------------------------------------------------------------------

    /// `GO [TO] para` — an unconditional jump to a paragraph's label. Forward and
    /// back references both resolve (all paragraph labels exist before emission).
    fn emit_goto(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let name = first_token(verb, "NAME")
            .ok_or_else(|| CompileError::Malformed("GO TO without a paragraph name".into()))?;
        if !self.para_index.contains_key(&name) {
            return Err(CompileError::Malformed(format!("GO TO undefined paragraph {name}")));
        }
        self.emit("jmp", None, vec![Operand::Var(para_label(&name))], "void");
        Ok(())
    }

    /// `PERFORM para [THRU para2] [n TIMES | UNTIL cond | VARYING …]`.
    ///
    /// The performed paragraph range is **inlined** at the call site (COBOL's
    /// out-of-line-but-returns semantics): the range's statements run, then
    /// control falls through to just after the PERFORM — exactly what inlining
    /// gives, since a `STOP RUN` inside returns and a `GO TO` inside jumps away at
    /// top level (both abandon the fall-through). A recursive/self-`PERFORM` (or a
    /// code-size blow-up) trips a depth / instruction bound as a clean error.
    fn emit_perform(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        if self.instrs.len() > MAX_EMIT_INSTRS {
            return Err(CompileError::Unsupported(
                "program too large to expand (a recursive PERFORM?) — a later rung".into(),
            ));
        }
        self.perform_depth += 1;
        if self.perform_depth > MAX_PERFORM_DEPTH {
            self.perform_depth -= 1;
            return Err(CompileError::Unsupported(
                "PERFORM nesting too deep (a paragraph performing itself?)".into(),
            ));
        }
        let result = self.emit_perform_inner(verb);
        self.perform_depth -= 1;
        result
    }

    fn emit_perform_inner(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let names: Vec<String> = child_tokens(verb)
            .into_iter()
            .filter(|(k, _)| k == "NAME")
            .map(|(_, v)| v)
            .collect();
        if names.is_empty() {
            return Err(CompileError::Malformed("PERFORM without a paragraph".into()));
        }
        let toks = child_tokens(verb);
        let has_thru = toks.iter().any(|(k, v)| k == "KEYWORD" && (v == "THRU" || v == "THROUGH"));
        let start = &names[0];
        let end = if has_thru {
            names.get(1).ok_or_else(|| CompileError::Malformed("PERFORM THRU without an end paragraph".into()))?
        } else {
            start
        };
        let si = *self
            .para_index
            .get(start)
            .ok_or_else(|| CompileError::Malformed(format!("PERFORM undefined paragraph {start}")))?;
        let ei = *self
            .para_index
            .get(end)
            .ok_or_else(|| CompileError::Malformed(format!("PERFORM undefined paragraph {end}")))?;
        if ei < si {
            return Err(CompileError::Unsupported(
                "PERFORM … THRU with a range that runs backwards is a later rung".into(),
            ));
        }

        if let Some(v) = child_node(verb, "perform_varying") {
            self.emit_perform_varying(v, si, ei)
        } else if toks.iter().any(|(k, x)| k == "KEYWORD" && x == "UNTIL") {
            let cond = child_node(verb, "condition")
                .ok_or_else(|| CompileError::Malformed("PERFORM UNTIL without a condition".into()))?;
            self.emit_perform_until(cond, si, ei)
        } else if toks.iter().any(|(k, x)| k == "KEYWORD" && x == "TIMES") {
            let op = child_node(verb, "operand")
                .ok_or_else(|| CompileError::Malformed("PERFORM … TIMES without a count".into()))?;
            self.emit_perform_times(op, si, ei)
        } else {
            self.inline_range(si, ei)
        }
    }

    /// Inline every statement of paragraphs `si..=ei`, in order.
    fn inline_range(&mut self, si: usize, ei: usize) -> Result<(), CompileError> {
        for idx in si..=ei {
            let stmts = self.paras[idx].stmts.clone();
            for stmt in stmts {
                self.emit_statement(stmt)?;
            }
        }
        Ok(())
    }

    /// `PERFORM range n TIMES` — a counted loop (runs zero times for n ≤ 0).
    fn emit_perform_times(
        &mut self,
        count_op: &GrammarASTNode,
        si: usize,
        ei: usize,
    ) -> Result<(), CompileError> {
        let n = self.integer_operand_value(count_op, "PERFORM … TIMES count")?;
        let cnt = self.fresh("_cnt");
        self.emit("mov", Some(&cnt), vec![n], "i64");
        let (top, done) = (self.fresh("perf_top"), self.fresh("perf_done"));
        let zero = self.fresh("_z");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        let more = self.fresh("_more");
        self.emit("cmp_gt", Some(&more), vec![Operand::Var(cnt.clone()), Operand::Var(zero)], "i64");
        self.emit("jmp_if_false", None, vec![Operand::Var(more), Operand::Var(done.clone())], "void");
        self.inline_range(si, ei)?;
        let one = self.fresh("_one");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        self.emit("sub", Some(&cnt), vec![Operand::Var(cnt.clone()), Operand::Var(one)], "i64");
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(done)], "void");
        Ok(())
    }

    /// `PERFORM range UNTIL cond` — tests **before** the body (may run zero times).
    fn emit_perform_until(
        &mut self,
        cond: &GrammarASTNode,
        si: usize,
        ei: usize,
    ) -> Result<(), CompileError> {
        let (top, done) = (self.fresh("perf_top"), self.fresh("perf_done"));
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        let c = self.emit_condition(cond)?;
        self.emit("jmp_if_true", None, vec![Operand::Var(c), Operand::Var(done.clone())], "void");
        self.inline_range(si, ei)?;
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(done)], "void");
        Ok(())
    }

    /// `PERFORM range VARYING id FROM x BY y UNTIL cond` — a counted loop over the
    /// induction variable `id`, tested before the body.
    fn emit_perform_varying(
        &mut self,
        vnode: &GrammarASTNode,
        si: usize,
        ei: usize,
    ) -> Result<(), CompileError> {
        let id = first_token(vnode, "NAME")
            .ok_or_else(|| CompileError::Malformed("PERFORM VARYING without an induction variable".into()))?;
        let ops = child_nodes(vnode, "operand");
        if ops.len() != 2 {
            return Err(CompileError::Malformed("PERFORM VARYING needs FROM and BY operands".into()));
        }
        let cond = child_node(vnode, "condition")
            .ok_or_else(|| CompileError::Malformed("PERFORM VARYING without an UNTIL condition".into()))?;

        self.move_operand_into(ops[0], &id)?; // id = x
        let (top, done) = (self.fresh("perf_top"), self.fresh("perf_done"));
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        let c = self.emit_condition(cond)?;
        self.emit("jmp_if_true", None, vec![Operand::Var(c), Operand::Var(done.clone())], "void");
        self.inline_range(si, ei)?;
        self.increment_by(&id, ops[1])?; // id = id + y
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(done)], "void");
        Ok(())
    }

    /// The integer value of an operand as an `i64` [`Operand`] — for a `TIMES`
    /// count. Requires a scale-0 (integer) operand.
    fn integer_operand_value(
        &mut self,
        op: &GrammarASTNode,
        what: &str,
    ) -> Result<Operand, CompileError> {
        let term = self.read_arith_term(op)?;
        if self.term_scale(&term) != 0 {
            return Err(CompileError::Unsupported(format!("{what} must be an integer")));
        }
        Ok(self.emit_term_at_scale(&term, 0))
    }

    /// `id = <operand>` — store an operand's value into a numeric item (as `MOVE`).
    fn move_operand_into(&mut self, op: &GrammarASTNode, dst: &str) -> Result<(), CompileError> {
        match read_operand(op)? {
            Operandy::Literal(lit) => self.move_literal_into(&lit, dst),
            Operandy::Name(name) => {
                let src_idx = self.numeric_index(&name)?;
                let (src_int, src_scale) = self.numeric_dims(src_idx);
                let src_reg = self.items[src_idx].reg.clone();
                self.store_scaled(dst, &src_reg, src_scale, src_int, false)
            }
        }
    }

    /// `id = id + <operand>` — the VARYING step, over the scaled arithmetic path.
    fn increment_by(&mut self, id: &str, op: &GrammarASTNode) -> Result<(), CompileError> {
        let base = Term::Item(self.numeric_index(id)?);
        let step = self.read_arith_term(op)?;
        let w = self.term_scale(&base).max(self.term_scale(&step));
        let max_int = self.term_int_digits(&base).max(self.term_int_digits(&step));
        if max_int + w + 1 > 18 {
            return Err(CompileError::Unsupported(
                "PERFORM VARYING step could overflow the i64 intermediate — a later rung".into(),
            ));
        }
        let acc = self.fresh("_acc");
        let b = self.emit_term_at_scale(&base, w);
        self.emit("mov", Some(&acc), vec![b], "i64");
        let s = self.emit_term_at_scale(&step, w);
        self.emit("add", Some(&acc), vec![Operand::Var(acc.clone()), s], "i64");
        self.store_scaled(id, &acc, w, max_int + 1, false)
    }

    // -----------------------------------------------------------------------
    // Arithmetic (unsigned receivers; scaled fixed-point for ADD/SUBTRACT)
    // -----------------------------------------------------------------------

    /// `ADD op… TO name [GIVING g]` — the accumulator starts at `name`'s value
    /// and each operand is added; the sum is stored into `g` (or `name`).
    fn emit_add(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.emit_additive(verb, "TO", false)
    }

    /// `SUBTRACT op… FROM name [GIVING g]` — `name` minus each operand.
    fn emit_subtract(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        self.emit_additive(verb, "FROM", true)
    }

    /// The shared body of `ADD`/`SUBTRACT`, exact over an implied decimal point.
    ///
    /// COBOL adds by the *value*, aligning implied points. We compute at a common
    /// **working scale** `w` (the largest fractional-digit count among the base
    /// field and the operands — so every term scales *up* to `w` without loss),
    /// accumulate, then store into the receiver at *its* scale (rounding or
    /// truncating any excess). `ROUNDED` and `ON SIZE ERROR` are both honoured.
    fn emit_additive(
        &mut self,
        verb: &GrammarASTNode,
        keyword: &str,
        is_subtract: bool,
    ) -> Result<(), CompileError> {
        let name = if is_subtract { "SUBTRACT" } else { "ADD" };
        let rounded = has_rounded(verb);
        let (base, giving) = self.two_targets(verb, keyword)?;

        // Terms summed: the base field first, then every operand.
        let mut terms = vec![Term::Item(self.numeric_index(&base)?)];
        for op in child_nodes(verb, "operand") {
            terms.push(self.read_arith_term(op)?);
        }
        let w = terms.iter().map(|t| self.term_scale(t)).max().unwrap_or(0);

        // Guard the intermediate against `i64` overflow. Each term at scale `w`
        // has magnitude < 10^(max_int_digits + w); the running sum of `k` of them
        // is < k · 10^(max_int_digits + w) < 10^(max_int_digits + w + digits(k)).
        // Requiring that exponent ≤ 18 keeps the accumulator below 10^18 <
        // i64::MAX, so a hostile "many wide operands" ADD can never wrap (which
        // would silently diverge from the exact oracle). Real programs never
        // approach this; it lands as a clean error, not wrong output.
        let max_int = terms.iter().map(|t| self.term_int_digits(t)).max().unwrap_or(0);
        let count_digits = decimal_len(terms.len());
        if max_int + w + count_digits > 18 {
            return Err(CompileError::Unsupported(format!(
                "{name} of {} terms into a {}-integer-digit field at scale {w} could overflow the \
                 i64 intermediate — a later rung",
                terms.len(),
                max_int
            )));
        }

        // acc = base value, scaled to w; then add/subtract each operand at w.
        let acc = self.fresh("_acc");
        let base_op = self.emit_term_at_scale(&terms[0], w);
        self.emit("mov", Some(&acc), vec![base_op], "i64");
        for term in &terms[1..] {
            let v = self.emit_term_at_scale(term, w);
            let op = if is_subtract { "sub" } else { "add" };
            self.emit(op, Some(&acc), vec![Operand::Var(acc.clone()), v], "i64");
        }

        // The accumulator's integer part is < 10^(max_int + digits(k)) (a sum of
        // k terms each with < max_int integer digits); store_scaled uses this to
        // bound any up-scale into a wider-scale GIVING receiver.
        let recv = giving.clone().unwrap_or(base);
        let handler = size_error_handler(verb);
        self.store_scaled_handled(&recv, &acc, w, max_int + count_digits, rounded, &handler)
    }

    /// `MULTIPLY a BY b [GIVING g]` — `a * b` into `g` (or `b`). The raw product
    /// of the two scaled `i64` slots carries scale `sa + sb`; store_scaled then
    /// rounds/truncates it to the receiver's scale. Each operand is ≤ 9 digits, so
    /// the product is < 10^18 and never overflows `i64`.
    fn emit_multiply(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let rounded = has_rounded(verb);
        let ops = child_nodes(verb, "operand");
        if ops.len() != 2 {
            return Err(CompileError::Malformed("MULTIPLY needs two operands".into()));
        }
        let at = self.read_arith_term(ops[0])?;
        let bt = self.read_arith_term(ops[1])?;
        let (sa, sb) = (self.term_scale(&at), self.term_scale(&bt));
        let a = self.emit_term_at_scale(&at, sa);
        let b = self.emit_term_at_scale(&bt, sb);
        let prod = self.fresh("_prod");
        self.emit("mul", Some(&prod), vec![a, b], "i64");
        let target = self.giving_or_operand_name(verb, ops[1], "MULTIPLY … BY <literal>")?;
        // The product's integer part is < 10^(a_int + b_int); store_scaled bounds
        // any up-scale into a wider-scale receiver by that count.
        let prod_int = self.term_int_digits(&at) + self.term_int_digits(&bt);
        let handler = size_error_handler(verb);
        self.store_scaled_handled(&target, &prod, sa + sb, prod_int, rounded, &handler)
    }

    /// `DIVIDE a INTO b [GIVING g]` — `b / a` into `g` (or `b`).
    ///
    /// The quotient is computed at working scale `w` = the receiver's `dec_digits`
    /// (plus one guard digit when `ROUNDED`), by scaling the dividend up so the
    /// integer division lands `w` fractional digits: with `b = B/10^sb` and
    /// `a = A/10^sa`, the value scaled by `10^w` is `floor(B·10^(sa+w−sb) / A)`.
    /// store_scaled then truncates or rounds `w → dec_digits`. A dividend with
    /// more fractional digits than the result precision, or an intermediate that
    /// would exceed `i64`, is a clean error.
    fn emit_divide(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let rounded = has_rounded(verb);
        let handler = size_error_handler(verb);
        let ops = child_nodes(verb, "operand");
        if ops.len() != 2 {
            return Err(CompileError::Malformed("DIVIDE needs two operands".into()));
        }
        let divisor_t = self.read_arith_term(ops[0])?; // a
        let dividend_t = self.read_arith_term(ops[1])?; // b
        let (sa, sb) = (self.term_scale(&divisor_t), self.term_scale(&dividend_t));
        let target = self.giving_or_operand_name(verb, ops[1], "DIVIDE … INTO <literal>")?;
        let recv_dec = self.numeric_dims(self.numeric_index(&target)?).1;
        let w = recv_dec + usize::from(rounded);

        if sa + w < sb {
            return Err(CompileError::Unsupported(
                "DIVIDE with a dividend of more fractional digits than the result precision \
                 is a later rung"
                    .into(),
            ));
        }
        let e = sa + w - sb;
        let b_int = self.term_int_digits(&dividend_t);
        // numerator = b_scaled · 10^e < 10^(b_int + sa + w); keep it in i64.
        if b_int + sa + w > 18 {
            return Err(CompileError::Unsupported(
                "DIVIDE intermediate at the requested precision could overflow i64 — a later rung"
                    .into(),
            ));
        }

        let b_val = self.emit_term_at_scale(&dividend_t, sb);
        let num = if e > 0 {
            let pr = self.fresh("_dp");
            self.emit("const", Some(&pr), vec![Operand::Int(10i64.pow(e as u32))], "i64");
            let n = self.fresh("_num");
            self.emit("mul", Some(&n), vec![b_val, Operand::Var(pr)], "i64");
            Operand::Var(n)
        } else {
            b_val
        };
        let a_val = self.emit_term_at_scale(&divisor_t, sa);

        // A zero divisor is a size-error condition (as COMPUTE treats it): with an
        // ON SIZE ERROR handler it is caught before the (faulting) division; without
        // one the emitted `div` faults at run time, matching the oracle's hard
        // DivideByZero. When caught, jump past the division and store to a shared end.
        let end_lbl = if handler.is_empty() {
            None
        } else {
            let zero = self.fresh("_z");
            self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
            let iszero = self.fresh("_dz");
            self.emit("cmp_eq", Some(&iszero), vec![a_val.clone(), Operand::Var(zero)], "i64");
            let (cont, end) = (self.fresh("dz_cont"), self.fresh("dz_end"));
            self.emit("jmp_if_false", None, vec![Operand::Var(iszero), Operand::Var(cont.clone())], "void");
            for h in &handler {
                self.emit_statement(h)?;
            }
            self.emit("jmp", None, vec![Operand::Var(end.clone())], "void");
            self.emit("label", None, vec![Operand::Var(cont)], "void");
            Some(end)
        };

        let quot = self.fresh("_quot");
        // i64 division truncates toward zero, as COBOL does.
        self.emit("div", Some(&quot), vec![num, a_val], "i64");
        // The quotient at scale w is < 10^(b_int + sa + w); its integer part is
        // < 10^(b_int + sa). store_scaled only down-scales here (w ≥ dec_digits),
        // so the up-scale bound is a formality; the handler catches overflow.
        self.store_scaled_handled(&target, &quot, w, b_int + sa, rounded, &handler)?;
        if let Some(end) = end_lbl {
            self.emit("label", None, vec![Operand::Var(end)], "void");
        }
        Ok(())
    }

    /// Store a computed `value_reg` (carrying `value_scale` fractional digits)
    /// into a numeric `target`. Three steps, matching the oracle's `store_result`
    /// for an unsigned receiver:
    ///
    /// 1. **rescale** the value from `value_scale` to the receiver's `dec_digits`
    ///    — scaling up loses nothing; scaling down rounds (half away from zero)
    ///    when `rounded`, else truncates toward zero;
    /// 2. take the **magnitude** (unsigned receivers keep no sign);
    /// 3. keep the low-order `int_digits + dec_digits` digits (COBOL's silent
    ///    high-order overflow truncation),
    ///
    /// then move it into the slot. The source register is copied first, so a
    /// data-name source (an item-to-item `MOVE`) is never clobbered.
    ///
    /// `value_max_int` is a bound on the value's integer-part digit count (its
    /// magnitude is `< 10^(value_max_int + value_scale)`). When the receiver has
    /// *more* fractional digits than `value_scale`, step 1 multiplies the whole
    /// value up — pushing its magnitude to `< 10^(value_max_int + dec_digits)`.
    /// That must stay below `10^18 < i64::MAX`, so an up-scale that could exceed
    /// it is a clean error (a later rung), never a silent wrap. Down-scaling and
    /// equal scales only shrink the value, so they need no bound here.
    fn store_scaled(
        &mut self,
        target: &str,
        value_reg: &str,
        value_scale: usize,
        value_max_int: usize,
        rounded: bool,
    ) -> Result<(), CompileError> {
        self.store_scaled_handled(target, value_reg, value_scale, value_max_int, rounded, &[])
    }

    /// The general store: like [`Self::store_scaled`], but with an optional
    /// `ON SIZE ERROR` handler. When the (rounded, magnitude) value's integer part
    /// overflows the receiver — `acc ≥ 10^(int+dec)` — a non-empty `handler` runs
    /// its statements and the receiver is left **unchanged**; an empty handler
    /// truncates the high-order digits silently (COBOL's handler-less rule).
    fn store_scaled_handled(
        &mut self,
        target: &str,
        value_reg: &str,
        value_scale: usize,
        value_max_int: usize,
        rounded: bool,
        handler: &[&GrammarASTNode],
    ) -> Result<(), CompileError> {
        let idx = self.numeric_index(target)?;
        let (int_digits, dec_digits) = self.numeric_dims(idx);
        let signed = self.item_signed(idx);
        let reg = self.items[idx].reg.clone();

        if dec_digits > value_scale && value_max_int + dec_digits > 18 {
            return Err(CompileError::Unsupported(format!(
                "up-scaling a {value_max_int}-integer-digit value into {target} (scale {dec_digits}) \
                 could overflow the i64 intermediate — a later rung"
            )));
        }

        // The value at the receiver's scale (keeps its sign), and its magnitude —
        // the overflow test is on the magnitude, and an *unsigned* receiver stores
        // the magnitude while a *signed* receiver keeps the sign.
        let acc = self.fresh("_acc");
        self.emit("mov", Some(&acc), vec![Operand::Var(value_reg.to_string())], "i64");
        self.rescale(&acc, value_scale, dec_digits, rounded);
        let mag = self.fresh("_mag");
        self.emit("mov", Some(&mag), vec![Operand::Var(acc.clone())], "i64");
        self.emit_abs(&mag);
        let modulus = 10i64.pow((int_digits + dec_digits) as u32);

        if handler.is_empty() {
            // Silent high-order truncation: keep the low-order magnitude digits
            // (`mag mod 10^(int+dec)`), re-applying the sign for a signed receiver.
            let m = self.fresh("_m");
            self.emit("const", Some(&m), vec![Operand::Int(modulus)], "i64");
            let kept = self.fresh("_kept");
            self.emit("mod", Some(&kept), vec![Operand::Var(mag), Operand::Var(m)], "i64");
            let stored = if signed { self.reapply_sign(&kept, &acc) } else { kept };
            self.emit("mov", Some(&reg), vec![Operand::Var(stored)], "i64");
        } else {
            // ON SIZE ERROR: if the magnitude ≥ 10^(int+dec) the integer part does
            // not fit — run the handler and leave the receiver unchanged; else the
            // value fits, so store it (signed keeps the sign, unsigned its magnitude).
            let m = self.fresh("_m");
            self.emit("const", Some(&m), vec![Operand::Int(modulus)], "i64");
            let ovf = self.fresh("_ovf");
            self.emit("cmp_ge", Some(&ovf), vec![Operand::Var(mag.clone()), Operand::Var(m)], "i64");
            let (ovf_lbl, end_lbl) = (self.fresh("se_ovf"), self.fresh("se_end"));
            self.emit("jmp_if_true", None, vec![Operand::Var(ovf), Operand::Var(ovf_lbl.clone())], "void");
            let fit = if signed { acc } else { mag };
            self.emit("mov", Some(&reg), vec![Operand::Var(fit)], "i64");
            self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
            self.emit("label", None, vec![Operand::Var(ovf_lbl)], "void");
            for h in handler {
                self.emit_statement(h)?;
            }
            self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
        }
        Ok(())
    }

    /// Return a register holding `magnitude` with the sign of `signed_ref` applied
    /// (negated when `signed_ref < 0`). Used to re-sign a truncated magnitude for
    /// a signed receiver, without relying on any backend's signed-remainder rule.
    fn reapply_sign(&mut self, magnitude: &str, signed_ref: &str) -> String {
        let out = self.fresh("_sgn");
        self.emit("mov", Some(&out), vec![Operand::Var(magnitude.to_string())], "i64");
        let zero = self.fresh("_z");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        let neg = self.fresh("_neg");
        self.emit("cmp_lt", Some(&neg), vec![Operand::Var(signed_ref.to_string()), Operand::Var(zero.clone())], "i64");
        let done = self.fresh("_sgndone");
        self.emit("jmp_if_false", None, vec![Operand::Var(neg), Operand::Var(done.clone())], "void");
        self.emit("sub", Some(&out), vec![Operand::Var(zero), Operand::Var(out.clone())], "i64");
        self.emit("label", None, vec![Operand::Var(done)], "void");
        out
    }

    /// Rescale register `acc` from `from` to `to` fractional digits in place.
    /// Up-scaling multiplies (exact). Down-scaling divides, truncating toward
    /// zero — or, when `rounded`, biasing by half a unit first so truncation
    /// rounds **half away from zero** (the bias's sign follows `acc`'s).
    fn rescale(&mut self, acc: &str, from: usize, to: usize, rounded: bool) {
        use std::cmp::Ordering;
        match from.cmp(&to) {
            Ordering::Equal => {}
            Ordering::Less => {
                let p = 10i64.pow((to - from) as u32);
                let pr = self.fresh("_up");
                self.emit("const", Some(&pr), vec![Operand::Int(p)], "i64");
                self.emit("mul", Some(acc), vec![Operand::Var(acc.into()), Operand::Var(pr)], "i64");
            }
            Ordering::Greater => {
                let p = 10i64.pow((from - to) as u32);
                if rounded {
                    // bias = (acc < 0) ? -(p/2) : (p/2); acc = (acc + bias) / p.
                    let half = self.fresh("_half");
                    self.emit("const", Some(&half), vec![Operand::Int(p / 2)], "i64");
                    let bias = self.fresh("_bias");
                    self.emit("mov", Some(&bias), vec![Operand::Var(half.clone())], "i64");
                    let zero = self.fresh("_z");
                    self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                    let neg = self.fresh("_neg");
                    self.emit("cmp_lt", Some(&neg), vec![Operand::Var(acc.into()), Operand::Var(zero.clone())], "i64");
                    let done = self.fresh("_biasdone");
                    self.emit("jmp_if_false", None, vec![Operand::Var(neg), Operand::Var(done.clone())], "void");
                    self.emit("sub", Some(&bias), vec![Operand::Var(zero), Operand::Var(half)], "i64");
                    self.emit("label", None, vec![Operand::Var(done)], "void");
                    self.emit("add", Some(acc), vec![Operand::Var(acc.into()), Operand::Var(bias)], "i64");
                }
                let pr = self.fresh("_dn");
                self.emit("const", Some(&pr), vec![Operand::Int(p)], "i64");
                self.emit("div", Some(acc), vec![Operand::Var(acc.into()), Operand::Var(pr)], "i64");
            }
        }
    }

    /// Replace register `acc` with its magnitude (negate when negative). Every
    /// value reaching here is bounded well below `10^18` (arithmetic operands and
    /// receivers are ≤ 9 digits, and `COMPUTE` intermediates are guarded to
    /// `< 10^18` at each step), so it is never `i64::MIN` and negation is safe.
    fn emit_abs(&mut self, acc: &str) {
        let zero = self.fresh("_z");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        let neg = self.fresh("_neg");
        self.emit("cmp_lt", Some(&neg), vec![Operand::Var(acc.into()), Operand::Var(zero.clone())], "i64");
        let skip = self.fresh("_absskip");
        self.emit("jmp_if_false", None, vec![Operand::Var(neg), Operand::Var(skip.clone())], "void");
        self.emit("sub", Some(acc), vec![Operand::Var(zero), Operand::Var(acc.into())], "i64");
        self.emit("label", None, vec![Operand::Var(skip)], "void");
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

    /// The `(int_digits, dec_digits)` of a numeric item by index.
    fn numeric_dims(&self, idx: usize) -> (usize, usize) {
        match &self.items[idx].kind {
            ItemKind::Numeric { int_digits, dec_digits, .. } => (*int_digits, *dec_digits),
            ItemKind::Char { .. } => (0, 0),
        }
    }

    /// Whether a numeric item is signed (`PIC S9…`) — it keeps its sign in the
    /// `i64` slot and displays a trailing overpunch.
    fn item_signed(&self, idx: usize) -> bool {
        matches!(self.items[idx].kind, ItemKind::Numeric { signed: true, .. })
    }

    /// The item index of a numeric data-name usable in arithmetic: it must be a
    /// declared numeric item no wider than [`ARITH_MAX_DIGITS`] digits (so the
    /// scaled `i64` arithmetic cannot overflow).
    fn numeric_index(&self, name: &str) -> Result<usize, CompileError> {
        let idx = self.item_index(name)?;
        match &self.items[idx].kind {
            ItemKind::Numeric { int_digits, dec_digits, .. } => {
                if int_digits + dec_digits > ARITH_MAX_DIGITS {
                    return Err(CompileError::Unsupported(format!(
                        "arithmetic on field {name} wider than {ARITH_MAX_DIGITS} digits is a later rung"
                    )));
                }
                Ok(idx)
            }
            ItemKind::Char { .. } => Err(CompileError::Unsupported(format!(
                "arithmetic on non-numeric field {name}"
            ))),
        }
    }

    /// Read an ADD/SUBTRACT operand into a [`Term`] (a literal value or a numeric
    /// item). Alphanumeric operands and over-wide literals are a later rung.
    fn read_arith_term(&self, op: &GrammarASTNode) -> Result<Term, CompileError> {
        match read_operand(op)? {
            Operandy::Literal(Src::Num(s)) => {
                let d = Decimal::parse_literal(&s)
                    .ok_or_else(|| CompileError::Malformed(format!("numeric literal {s}")))?;
                if d.int.trim_start_matches('0').len() + d.frac.len() > ARITH_MAX_DIGITS {
                    return Err(CompileError::Unsupported(format!(
                        "numeric literal {s} wider than {ARITH_MAX_DIGITS} digits in arithmetic is a later rung"
                    )));
                }
                Ok(Term::Lit(d))
            }
            Operandy::Literal(Src::Zero) => Ok(Term::Lit(Decimal::zero())),
            Operandy::Literal(_) => {
                Err(CompileError::Unsupported("an alphanumeric operand in arithmetic".into()))
            }
            Operandy::Name(n) => Ok(Term::Item(self.numeric_index(&n)?)),
        }
    }

    /// A term's fractional-digit count (its scale).
    fn term_scale(&self, term: &Term) -> usize {
        match term {
            Term::Lit(d) => d.frac.len(),
            Term::Item(idx) => self.numeric_dims(*idx).1,
        }
    }

    /// A term's integer-digit count (its magnitude's width before the point) —
    /// used only to bound the `i64` accumulator against overflow.
    fn term_int_digits(&self, term: &Term) -> usize {
        match term {
            Term::Lit(d) => d.int.trim_start_matches('0').len(),
            Term::Item(idx) => self.numeric_dims(*idx).0,
        }
    }

    /// Emit an `i64` [`Operand`] carrying the term's value at working scale `w`
    /// (≥ the term's own scale, so scaling up is exact). A literal is folded to a
    /// `const` immediate; an item is read from its register and scaled up at run
    /// time when needed.
    fn emit_term_at_scale(&mut self, term: &Term, w: usize) -> Operand {
        match term {
            Term::Lit(d) => Operand::Int(decimal_scaled_to(d, w)),
            Term::Item(idx) => {
                let (_, scale) = self.numeric_dims(*idx);
                let reg = self.items[*idx].reg.clone();
                if scale == w {
                    Operand::Var(reg)
                } else {
                    let p = 10i64.pow((w - scale) as u32);
                    let pr = self.fresh("_ts");
                    self.emit("const", Some(&pr), vec![Operand::Int(p)], "i64");
                    let out = self.fresh("_tv");
                    self.emit("mul", Some(&out), vec![Operand::Var(reg), Operand::Var(pr)], "i64");
                    Operand::Var(out)
                }
            }
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

    /// Emit the overpunch-printer call for a signed numeric item's `width`-digit
    /// image (its sign shows as a trailing overpunch on the units digit). The
    /// helper itself calls the plain digit printer, so both are needed.
    fn emit_print_signed(&mut self, reg: &str, width: i64) {
        self.needs_print = true;
        self.needs_print_signed = true;
        let w = self.fresh("_w");
        self.emit("const", Some(&w), vec![Operand::Int(width)], "i64");
        let ret = self.fresh("_pr");
        self.emit(
            "call",
            Some(&ret),
            vec![Operand::Var(SIGNED_PRINT_HELPER.into()), Operand::Var(reg.to_string()), Operand::Var(w)],
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

const SIGNED_PRINT_HELPER: &str = "__cob_print_signed";

/// `__cob_print_signed(v, w)` prints a signed numeric item's `w`-digit image with
/// its sign folded into a trailing **overpunch** on the units digit — COBOL's
/// default `DISPLAY` of a `PIC S9…` field. It prints the leading `w−1` magnitude
/// digits via [`PRINT_HELPER`], then one overpunch character for the units digit:
///
/// | units digit | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
/// |-------------|---|---|---|---|---|---|---|---|---|---|
/// | positive    | { | A | B | C | D | E | F | G | H | I |
/// | negative    | } | J | K | L | M | N | O | P | Q | R |
///
/// So `+123` → `12C`, `−123` → `12L`, and (unsigned) zero → `00{`. The codes are
/// arithmetic: a non-zero units digit `d` is `(neg ? 73 : 64) + d` (`'A'..'I'` /
/// `'J'..'R'`), and a zero units digit is `neg ? 125 : 123` (`'}'` / `'{'`).
fn print_signed_function() -> IIRFunction {
    fn mk(op: &str, dest: Option<&str>, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
        IIRInstr::new(op, dest.map(str::to_string), srcs, ty)
    }
    fn var(name: &str) -> Operand {
        Operand::Var(name.to_string())
    }

    let body = vec![
        mk("const", Some("zero"), vec![Operand::Int(0)], "i64"),
        // neg = v < 0; mag = |v|.
        mk("cmp_lt", Some("neg"), vec![var("v"), var("zero")], "i64"),
        mk("mov", Some("mag"), vec![var("v")], "i64"),
        mk("jmp_if_false", None, vec![var("neg"), var("notneg")], "void"),
        mk("sub", Some("mag"), vec![var("zero"), var("v")], "i64"),
        mk("label", None, vec![var("notneg")], "void"),
        // units = mag % 10; rest = mag / 10.
        mk("const", Some("ten"), vec![Operand::Int(10)], "i64"),
        mk("mod", Some("units"), vec![var("mag"), var("ten")], "i64"),
        mk("div", Some("rest"), vec![var("mag"), var("ten")], "i64"),
        // print the leading w-1 digits of the magnitude.
        mk("const", Some("one"), vec![Operand::Int(1)], "i64"),
        mk("sub", Some("wm1"), vec![var("w"), var("one")], "i64"),
        mk("call", Some("_r"), vec![var(PRINT_HELPER), var("rest"), var("wm1")], "i64"),
        // nzcode = (neg ? 73 : 64) + units.
        mk("const", Some("c64"), vec![Operand::Int(64)], "i64"),
        mk("const", Some("c73"), vec![Operand::Int(73)], "i64"),
        mk("mov", Some("nzbase"), vec![var("c64")], "i64"),
        mk("jmp_if_false", None, vec![var("neg"), var("nzdone")], "void"),
        mk("mov", Some("nzbase"), vec![var("c73")], "i64"),
        mk("label", None, vec![var("nzdone")], "void"),
        mk("add", Some("nzcode"), vec![var("nzbase"), var("units")], "i64"),
        // zcode = neg ? 125 : 123.
        mk("const", Some("c123"), vec![Operand::Int(123)], "i64"),
        mk("const", Some("c125"), vec![Operand::Int(125)], "i64"),
        mk("mov", Some("zcode"), vec![var("c123")], "i64"),
        mk("jmp_if_false", None, vec![var("neg"), var("zdone")], "void"),
        mk("mov", Some("zcode"), vec![var("c125")], "i64"),
        mk("label", None, vec![var("zdone")], "void"),
        // code = (units == 0) ? zcode : nzcode.
        mk("cmp_eq", Some("iszero"), vec![var("units"), var("zero")], "i64"),
        mk("mov", Some("code"), vec![var("nzcode")], "i64"),
        mk("jmp_if_false", None, vec![var("iszero"), var("usecode")], "void"),
        mk("mov", Some("code"), vec![var("zcode")], "i64"),
        mk("label", None, vec![var("usecode")], "void"),
        mk("call_builtin", None, vec![var("putchar"), var("code")], "void"),
        mk("const", Some("z0"), vec![Operand::Int(0)], "i64"),
        mk("ret", None, vec![var("z0")], "i64"),
    ];
    let mut f = IIRFunction::new(
        SIGNED_PRINT_HELPER,
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

/// Whether a literal source denotes a negative value — only a numeric literal
/// can (`ZERO`/`SPACE`/strings are non-negative). A negative *zero* is treated
/// as non-negative, matching COBOL's unsigned zero.
fn literal_is_negative(src: &Src) -> bool {
    match src {
        Src::Num(s) => Decimal::parse_literal(s).is_some_and(|d| d.neg && !d.is_zero()),
        _ => false,
    }
}

/// A summed term of an `ADD`/`SUBTRACT`: a literal value or a numeric item.
enum Term {
    Lit(Decimal),
    Item(usize),
}

/// An operand of an **alphanumeric** comparison: either a known-length string
/// (a character item's slot or a string-literal `str_const`) or a figurative
/// constant whose length is resolved from the operand it is compared against.
enum StrOperand {
    Fixed { reg: String, len: usize },
    Fig(char),
}

/// A parsed `COMPUTE` arithmetic expression — the grammar's precedence cascade
/// (`arith_expr` → `arith_term` → `arith_factor` → `arith_unary` →
/// `arith_primary`) folded into a tree, mirroring the oracle's `Expr`. Leaves
/// are numeric literals and numeric-item indices; `+ - * /` fold
/// left-associatively and `**` right-associatively (COBOL's rule), so the tree
/// shape matches the oracle's exactly.
enum AExpr {
    /// A numeric literal (its exact decimal value).
    Num(Decimal),
    /// A numeric item, by its slot index.
    Var(usize),
    /// Unary minus.
    Neg(Box<AExpr>),
    Add(Box<AExpr>, Box<AExpr>),
    Sub(Box<AExpr>, Box<AExpr>),
    Mul(Box<AExpr>, Box<AExpr>),
    Div(Box<AExpr>, Box<AExpr>),
    /// `base ** exponent`. Evaluated when the exponent is a **compile-time
    /// non-negative integer** (unrolled into repeated multiplication, exactly as
    /// the oracle's `pow` multiplies `1` by `base` `exponent` times); a variable,
    /// negative, fractional, or oversized exponent is a clean later rung.
    Pow(Box<AExpr>, Box<AExpr>),
}

/// A `COMPUTE` sub-expression lowered to an `i64` register, with the
/// compile-time bounds needed to keep every downstream operation from silently
/// wrapping: `scale` is its fractional-digit count and `int_bound` bounds its
/// integer-part digits (magnitude `< 10^(int_bound + scale)`).
struct Eval {
    reg: String,
    scale: usize,
    int_bound: usize,
}

/// Does an arithmetic verb carry the `ROUNDED` keyword?
fn has_rounded(verb: &GrammarASTNode) -> bool {
    child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "ROUNDED")
}

/// The number of decimal digits in `n` (≥ 1). Used to bound how much an operand
/// count can enlarge the accumulator.
fn decimal_len(n: usize) -> usize {
    let mut n = n;
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

/// A [`Decimal`]'s value scaled by `10^w` as an `i64` — the fixed-point integer a
/// numeric slot at scale `w` would hold. `w` is at least the literal's own scale
/// (chosen as a working maximum), so no fractional digit is lost. Reuses the
/// oracle's `move_into_numeric` to format the magnitude, then applies the sign.
fn decimal_scaled_to(d: &Decimal, w: usize) -> i64 {
    let mag = parse_digits(&move_into_numeric(d, NUMERIC_MAX_DIGITS, w));
    if d.neg {
        -mag
    } else {
        mag
    }
}

/// A `**` exponent that is a compile-time **non-negative integer** literal,
/// returned as its magnitude — otherwise `None`, so a variable, parenthesised
/// expression, negative, or fractional exponent stays a clean later rung. The
/// acceptance rule mirrors the oracle's `pow`: a fractional part of anything but
/// zeros is rejected, and a negative sign is rejected unless the value is zero
/// (`-0 = 0`). An integer past `u128` fails to parse (caught here) and one past
/// `MAX_POW_EXP` is rejected by the caller — both matching the oracle's `None`.
/// Whether a COMPUTE expression tree contains a division anywhere. Used to
/// decline `ON SIZE ERROR` alongside a nested division (whose zero divisor this
/// rung faults rather than routing to the handler).
fn aexpr_contains_div(e: &AExpr) -> bool {
    match e {
        AExpr::Div(..) => true,
        AExpr::Neg(inner) => aexpr_contains_div(inner),
        AExpr::Add(l, r) | AExpr::Sub(l, r) | AExpr::Mul(l, r) | AExpr::Pow(l, r) => {
            aexpr_contains_div(l) || aexpr_contains_div(r)
        }
        AExpr::Num(_) | AExpr::Var(_) => false,
    }
}

fn const_nonneg_int(e: &AExpr) -> Option<u128> {
    let AExpr::Num(d) = e else { return None };
    // A non-zero fractional digit means it is not an integer.
    if d.frac.chars().any(|c| c != '0') {
        return None;
    }
    let int_is_zero = d.int.chars().all(|c| c == '0');
    // A negative sign is allowed only on zero.
    if d.neg && !int_is_zero {
        return None;
    }
    let trimmed = d.int.trim_start_matches('0');
    if trimmed.is_empty() {
        Some(0)
    } else {
        trimmed.parse().ok()
    }
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

/// Format a level-88 `VALUE` literal into its conditional variable's picture at
/// compile time and read back the signed scaled `i64` — the same reuse
/// `MOVE <literal>` relies on, so the comparison constant matches the slot's
/// stored representation exactly.
fn scale_num_value(
    src: &Src,
    picture: &Picture,
    signed: bool,
    name: &str,
) -> Result<i64, CompileError> {
    let digits = format_into_picture(src, picture)
        .map_err(|m| CompileError::Unsupported(format!("level-88 {name} VALUE: {m}")))?;
    let mag = parse_digits(&digits);
    Ok(if signed && literal_is_negative(src) { -mag } else { mag })
}

/// Read every `value_item` of a `value_clause` into a [`ValueSpec`]. A
/// `value_item` is `literal [ (THRU|THROUGH) literal ]` — two literals form an
/// inclusive range, one a single value.
fn read_value_specs(vc: &GrammarASTNode) -> Result<Vec<ValueSpec>, CompileError> {
    let mut specs = Vec::new();
    for item in child_nodes(vc, "value_item") {
        let lits = child_nodes(item, "literal");
        let spec = match lits.as_slice() {
            [one] => ValueSpec::Single(read_literal(one)?),
            [lo, hi] => ValueSpec::Range(read_literal(lo)?, read_literal(hi)?),
            _ => {
                return Err(CompileError::Malformed(
                    "a VALUE item must be `literal` or `literal THRU literal`".into(),
                ))
            }
        };
        specs.push(spec);
    }
    Ok(specs)
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

/// The IIR label for a PROCEDURE DIVISION paragraph.
fn para_label(name: &str) -> String {
    format!("para_{}", sanitise(name))
}

/// An arithmetic verb's `ON SIZE ERROR` handler statements (empty if none).
fn size_error_handler(verb: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    match child_node(verb, "size_error") {
        Some(se) => child_nodes(se, "statement"),
        None => vec![],
    }
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
    fn scaled_add_now_compiles() {
        // ADD into a V field is supported (PR3): the scaled-i64 path validates.
        let module = compile_source(
            &wrap(&["01  R  PIC 9(2)V9 VALUE 0."], &["ADD 1.5 TO R.", "DISPLAY R.", "STOP RUN."]),
            "v",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
    }

    #[test]
    fn add_rounded_and_on_size_error_both_compile() {
        // Both ROUNDED and ON SIZE ERROR are honoured on ADD now.
        for body in ["ADD 1 TO R ROUNDED.", "ADD 1 TO R ON SIZE ERROR DISPLAY \"OVR\"."] {
            let m = compile_source(
                &wrap(&["01  R  PIC 9(3) VALUE 0."], &[body, "STOP RUN."]),
                "r",
            )
            .unwrap();
            assert!(m.validate().is_empty(), "{body}: {:?}", m.validate());
        }
    }

    #[test]
    fn wide_operand_at_high_scale_add_is_a_clean_error() {
        // A 9-fraction-digit receiver forces working scale 9; a 9-integer-digit
        // operand scaled to that would exceed the i64 intermediate. The guard
        // rejects cleanly rather than emitting code that wraps (and would diverge
        // from the exact oracle).
        let err = compile_source(
            &wrap(&["01  R  PIC V9(9) VALUE 0."], &["ADD 999999999 TO R.", "STOP RUN."]),
            "ov",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn add_giving_wider_scale_receiver_overflow_is_a_clean_error() {
        // ADD of a wide integer field GIVING a high-scale receiver: the store-time
        // up-scale (×10^9) would overflow the i64 intermediate. Must be rejected,
        // not silently wrapped. (A is 9(9); GIVING C at scale 9.)
        let err = compile_source(
            &wrap(
                &["01  A  PIC 9(9) VALUE 0.", "01  C  PIC V9(9) VALUE 0."],
                &["ADD 999999999 TO A GIVING C.", "STOP RUN."],
            ),
            "gv",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn multiply_into_wide_v_receiver_overflow_is_a_clean_error() {
        // 999999999 × 999999999 ≈ 10^18 fits, but up-scaling ×10 into a V9 field
        // overflows the i64 intermediate — a clean error, not a silent wrap.
        let err = compile_source(
            &wrap(
                &["01  R  PIC 9(8)V9 VALUE 0."],
                &["MULTIPLY 999999999 BY 999999999 GIVING R.", "STOP RUN."],
            ),
            "mv",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn if_compiles_with_branches_and_validates() {
        let module = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 5."],
                &["IF N GREATER 3 DISPLAY \"BIG\" ELSE DISPLAY \"SMALL\".", "STOP RUN."],
            ),
            "if",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"cmp_gt".to_string()));
        assert!(os.contains(&"jmp_if_false".to_string()));
        assert!(os.contains(&"label".to_string()));
    }

    #[test]
    fn if_negation_inverts_the_relation() {
        // IS NOT GREATER lowers to cmp_le (not an integer boolean inversion).
        let module = compile_source(
            &wrap(&["01  N  PIC 9(3) VALUE 3."], &["IF N IS NOT GREATER THAN 5 DISPLAY \"OK\".", "STOP RUN."]),
            "n",
        )
        .unwrap();
        assert!(ops(&module).contains(&"cmp_le".to_string()));
    }

    #[test]
    fn not_over_a_condition_emits_xor() {
        // `NOT (…)` inverts the group's boolean with `xor` (IIR `not` is bitwise, so
        // xor-with-1 is the logical negation). The `or` inside the group is present.
        let m = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 5."],
                &["IF NOT (N < 3 OR N > 9) DISPLAY \"X\".", "STOP RUN."],
            ),
            "c",
        )
        .unwrap();
        let ops = ops(&m);
        assert!(ops.contains(&"xor".to_string()), "expected `xor`: {ops:?}");
        assert!(ops.contains(&"or".to_string()), "expected `or`: {ops:?}");
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn evaluate_lowers_to_a_cmp_eq_cascade() {
        // EVALUATE lowers to a cmp_eq + jmp_if_false cascade — one cmp_eq per value
        // WHEN. A 3-value EVALUATE plus WHEN OTHER validates and emits the cascade.
        let m = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 5."],
                &[
                    "EVALUATE N",
                    "WHEN 1 DISPLAY \"A\"",
                    "WHEN 2 DISPLAY \"B\"",
                    "WHEN 5 DISPLAY \"C\"",
                    "WHEN OTHER DISPLAY \"D\"",
                    "END-EVALUATE.",
                    "STOP RUN.",
                ],
            ),
            "ev",
        )
        .unwrap();
        let n_cmp_eq = ops(&m).iter().filter(|o| *o == "cmp_eq").count();
        assert_eq!(n_cmp_eq, 3, "one cmp_eq per value WHEN: {:?}", ops(&m));
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn evaluate_multi_value_and_range_when_or_folds() {
        // `WHEN 1 2 5 THRU 7` OR-folds cmp_eq (singles) and and(cmp_ge,cmp_le)
        // (the range): so `or`, `and`, `cmp_ge`, `cmp_le` all appear.
        let m = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 5."],
                &["EVALUATE N", "WHEN 1 2 5 THRU 7 DISPLAY \"X\"", "END-EVALUATE.", "STOP RUN."],
            ),
            "ev",
        )
        .unwrap();
        let ops = ops(&m);
        for want in ["or", "and", "cmp_ge", "cmp_le", "cmp_eq"] {
            assert!(ops.contains(&want.to_string()), "expected `{want}`: {ops:?}");
        }
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn evaluate_on_an_alphanumeric_subject_is_deferred() {
        // A non-numeric subject needs a string compare — a clean later rung.
        let err = compile_source(
            &wrap(
                &["01  W  PIC X(3) VALUE \"ABC\"."],
                &["EVALUATE W", "WHEN \"ABC\" DISPLAY \"Y\"", "END-EVALUATE.", "STOP RUN."],
            ),
            "ev",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn compound_conditions_fold_with_bitwise_and_or() {
        // `A AND B` emits `and`; `A OR B` emits `or`; a parenthesised group nests.
        let m = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 5."],
                &["IF (N > 1 OR N > 9) AND N < 8 DISPLAY \"X\".", "STOP RUN."],
            ),
            "c",
        )
        .unwrap();
        let ops = ops(&m);
        assert!(ops.contains(&"and".to_string()), "expected `and`: {ops:?}");
        assert!(ops.contains(&"or".to_string()), "expected `or`: {ops:?}");
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn symbolic_relops_map_to_the_right_cmp_op() {
        // Each symbol lowers to its `cmp_*`: `>=`→cmp_ge, `<=`→cmp_le, `<>`→cmp_ne,
        // `>`→cmp_gt, `<`→cmp_lt, `=`→cmp_eq. A `NOT` before a symbol composes with
        // its baseline negation: `NOT >=` ≡ `<` → cmp_lt.
        for (body, want) in [
            ("IF N > 5 DISPLAY \"X\".", "cmp_gt"),
            ("IF N < 5 DISPLAY \"X\".", "cmp_lt"),
            ("IF N = 5 DISPLAY \"X\".", "cmp_eq"),
            ("IF N >= 5 DISPLAY \"X\".", "cmp_ge"),
            ("IF N <= 5 DISPLAY \"X\".", "cmp_le"),
            ("IF N <> 5 DISPLAY \"X\".", "cmp_ne"),
            ("IF N NOT >= 5 DISPLAY \"X\".", "cmp_lt"),
        ] {
            let m = compile_source(&wrap(&["01  N  PIC 9(3) VALUE 5."], &[body, "STOP RUN."]), "r").unwrap();
            assert!(ops(&m).contains(&want.to_string()), "{body} → expected {want}: {:?}", ops(&m));
        }
    }

    #[test]
    fn alphanumeric_comparison_and_move_now_compile() {
        // Space-padded character comparison and character item-to-item MOVE both
        // lower now (str_cmp / str_slice / str_concat), and validate.
        let m = compile_source(
            &wrap(
                &["01  W  PIC X(4) VALUE \"AB\".", "01  V  PIC X(2)."],
                &["MOVE W TO V.", "IF W EQUAL \"AB\" DISPLAY \"M\".", "STOP RUN."],
            ),
            "a",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "char MOVE truncates via str_slice");
        assert!(os.contains(&"str_cmp".to_string()), "alphanumeric IF compares via str_cmp");
    }

    #[test]
    fn cross_category_move_is_deferred() {
        // A numeric→alphanumeric (or reverse) item MOVE needs runtime int↔string
        // conversion — a clean later rung, never wrong output.
        let err = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(4)."],
                &["MOVE N TO W.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn level_88_condition_name_lowers() {
        // A level-88 condition-name over a numeric item lowers to a valid cmp_eq.
        let m = compile_source(
            &wrap(
                &["01  STATUS-CODE  PIC 9 VALUE 1.", "88  IS-OK  VALUE 1."],
                &["IF IS-OK DISPLAY \"OK\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn level_88_on_an_alphanumeric_item_is_deferred() {
        // A condition-name whose conditional variable is alphanumeric needs a
        // string compare — a clean later rung, matching the oracle's own deferral.
        let err = compile_source(
            &wrap(
                &["01  FLAG  PIC X VALUE \"Y\".", "88  IS-YES  VALUE \"Y\"."],
                &["IF IS-YES DISPLAY \"YES\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn level_88_multi_value_and_range_lower() {
        // A condition-name with several values and a THRU range lowers to valid
        // IIR (an OR-fold of cmp_eq and and(cmp_ge, cmp_le)).
        let m = compile_source(
            &wrap(
                &["01  N  PIC 99 VALUE 3.", "88  COND  VALUE 1 5 THRU 7 9."],
                &["IF COND DISPLAY \"Y\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn multi_value_on_a_plain_item_is_rejected() {
        // A multi-value / THRU-range VALUE is only meaningful on a level-88 entry;
        // on a plain item it is a clean error, matching the oracle.
        for data in ["01  N  PIC 99 VALUE 1 2 3.", "01  N  PIC 99 VALUE 1 THRU 5."] {
            let err = compile_source(&wrap(&[data], &["STOP RUN."]), "c88").unwrap_err();
            assert!(matches!(err, CompileError::Unsupported(_)), "{data}: got {err:?}");
        }
    }

    #[test]
    fn set_condition_name_to_true_lowers() {
        // SET cond-name TO TRUE lowers to a const store of the first value.
        let m = compile_source(
            &wrap(
                &["01  N  PIC 99 VALUE 0.", "88  COND  VALUE 3 THRU 6."],
                &["SET COND TO TRUE.", "DISPLAY N.", "STOP RUN."],
            ),
            "set",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn set_an_undeclared_condition_name_is_an_error() {
        let err = compile_source(
            &wrap(&["01  N  PIC 99 VALUE 0."], &["SET NOPE TO TRUE.", "STOP RUN."]),
            "set",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn scaled_multiply_divide_now_compile() {
        // MULTIPLY/DIVIDE on a V operand, and their ROUNDED, are supported (PR3b).
        for body in ["MULTIPLY 2.5 BY R.", "DIVIDE 2 INTO R ROUNDED."] {
            let module = compile_source(
                &wrap(&["01  R  PIC 9(2)V9 VALUE 1."], &[body, "STOP RUN."]),
                "md",
            )
            .unwrap();
            assert!(module.validate().is_empty(), "{body}: {:?}", module.validate());
        }
    }

    #[test]
    fn multiply_divide_on_size_error_now_compile() {
        // ON SIZE ERROR is honoured on MULTIPLY/DIVIDE (incl. the zero-divisor guard).
        for body in [
            "MULTIPLY 2 BY R ON SIZE ERROR DISPLAY \"O\".",
            "DIVIDE 2 INTO R ON SIZE ERROR DISPLAY \"O\".",
        ] {
            let m = compile_source(
                &wrap(&["01  R  PIC 9(3) VALUE 1."], &[body, "STOP RUN."]),
                "se",
            )
            .unwrap();
            assert!(m.validate().is_empty(), "{body}: {:?}", m.validate());
        }
    }

    #[test]
    fn compute_expression_compiles_and_validates() {
        // Precedence, parens, a top-level division, and unary minus all lower to
        // valid IIR.
        for body in [
            "COMPUTE R = A + B * C.",
            "COMPUTE R = (A + B) * C.",
            "COMPUTE R ROUNDED = A / B.",
            "COMPUTE R = -B + A ON SIZE ERROR DISPLAY \"O\".",
        ] {
            let m = compile_source(
                &wrap(
                    &[
                        "01  A  PIC 9(3) VALUE 10.",
                        "01  B  PIC 9(3) VALUE 3.",
                        "01  C  PIC 9(3) VALUE 2.",
                        "01  R  PIC 9(4)V99.",
                    ],
                    &[body, "STOP RUN."],
                ),
                "compute",
            )
            .unwrap();
            assert!(m.validate().is_empty(), "{body}: {:?}", m.validate());
        }
    }

    #[test]
    fn compute_variable_exponent_is_deferred() {
        // A `**` whose exponent is not a compile-time non-negative integer is a
        // clean error — never wrong output. `C ** B` (variable exponent),
        // `C ** -2` (negative), `C ** 1.5` (fractional), and an oversized exponent
        // all stay a later rung. (Nested division, by contrast, now lowers — see
        // `compute_nested_division_lowers`.)
        for body in [
            "COMPUTE R = C ** B.",
            "COMPUTE R = C ** -2.",
            "COMPUTE R = C ** 1.5.",
            "COMPUTE R = C ** 99999.",
        ] {
            let err = compile_source(
                &wrap(
                    &[
                        "01  A  PIC 9(3) VALUE 10.",
                        "01  B  PIC 9(3) VALUE 3.",
                        "01  C  PIC 9(3) VALUE 2.",
                        "01  R  PIC 9(4)V99.",
                    ],
                    &[body, "STOP RUN."],
                ),
                "compute",
            )
            .unwrap_err();
            assert!(matches!(err, CompileError::Unsupported(_)), "{body}: got {err:?}");
        }
    }

    #[test]
    fn compute_nested_division_lowers() {
        // Division inside a larger expression now lowers to valid IIR (scale-12
        // intermediate). Both `A / B + C` and `A / B * C` compile and validate.
        for body in ["COMPUTE R = A / B + C.", "COMPUTE R = A / B * C.", "COMPUTE R = C + A / B."] {
            let m = compile_source(
                &wrap(
                    &[
                        "01  A  PIC 9(3) VALUE 10.",
                        "01  B  PIC 9(3) VALUE 3.",
                        "01  C  PIC 9(3) VALUE 2.",
                        "01  R  PIC 9(4)V99.",
                    ],
                    &[body, "STOP RUN."],
                ),
                "compute",
            )
            .unwrap();
            assert!(m.validate().is_empty(), "{body}: {:?}", m.validate());
        }
    }

    #[test]
    fn compute_nested_division_with_size_error_handler_is_deferred() {
        // A nested division's zero divisor faults rather than routing to the
        // handler, so a COMPUTE that pairs ON SIZE ERROR with a nested division is
        // a clean later rung. (A top-level division with a handler still lowers.)
        let err = compile_source(
            &wrap(
                &["01  A  PIC 9(3) VALUE 10.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9(4)V99."],
                &["COMPUTE R = A / B + 1 ON SIZE ERROR DISPLAY \"O\".", "STOP RUN."],
            ),
            "compute",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn compute_nested_division_over_wide_intermediate_is_deferred() {
        // The scale-12 intermediate squeezes the i64 integer range; a dividend
        // wide enough that `int + scale + 12` exceeds 18 digits is a clean later
        // rung, never a silent wrap.
        let err = compile_source(
            &wrap(
                &["01  A  PIC 9(9) VALUE 1.", "01  B  PIC 9(3) VALUE 3.", "01  R  PIC 9(4)V99."],
                &["COMPUTE R = A / B + 1.", "STOP RUN."],
            ),
            "compute",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn compute_exponentiation_with_a_literal_exponent_lowers() {
        // A `**` with a compile-time non-negative integer exponent lowers to valid
        // IIR — a square, a cube, the identity `** 1`, and `** 0` (the base is not
        // even read).
        for body in [
            "COMPUTE R = A ** 2.",
            "COMPUTE R = A ** 3.",
            "COMPUTE R = A ** 1.",
            "COMPUTE R = A ** 0.",
        ] {
            let m = compile_source(
                &wrap(&["01  A  PIC 9(2) VALUE 3.", "01  R  PIC 9(6)."], &[body, "STOP RUN."]),
                "pow",
            )
            .unwrap();
            assert!(m.validate().is_empty(), "{body}: {:?}", m.validate());
        }
    }

    #[test]
    fn compute_exponentiation_overflowing_the_model_is_deferred() {
        // The compile-time bound is conservative: `int_digits · exponent` for a
        // 3-digit base raised to the 10th could reach 30 digits, past the 18-digit
        // i64 model — so it is a clean later rung, never a silent wrap.
        let err = compile_source(
            &wrap(&["01  A  PIC 9(3) VALUE 2.", "01  R  PIC 9(9)."], &["COMPUTE R = A ** 10.", "STOP RUN."]),
            "pow",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn signed_numeric_compiles_and_appends_the_overpunch_helper() {
        // A signed field now lowers (was a deferred error): it keeps its sign and
        // DISPLAY routes through the overpunch printer.
        let m = compile_source(
            &wrap(
                &["01  N  PIC S9(3) VALUE -12."],
                &["ADD 5 TO N.", "DISPLAY N.", "STOP RUN."],
            ),
            "signed",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        // The signed value initialises negative, and the overpunch helper is present.
        assert!(m.functions[0]
            .instructions
            .iter()
            .any(|i| i.op == "const" && matches!(i.srcs.first(), Some(Operand::Int(-12)))));
        assert!(m.functions.iter().any(|f| f.name == SIGNED_PRINT_HELPER));
        assert!(m.functions.iter().any(|f| f.name == PRINT_HELPER));
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

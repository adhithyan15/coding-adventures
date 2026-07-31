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

/// Rejection message for a reference modification used outside the two contexts
/// this rung supports (DISPLAY operands and alphanumeric comparison operands).
/// Numeric/arithmetic and MOVE-source uses are a later rung.
const REFMOD_CONTEXT_MSG: &str =
    "reference modification is only supported in DISPLAY and comparison contexts on this rung";

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

/// A resolved CONVERTING `from`/`to` operand: a compile-time ASCII string LITERAL,
/// or a data-name (`PIC X` item) reduced to its backing register plus its declared
/// width. Both carry a compile-time-known length (a literal's char count, an item's
/// declared width), so the equal-length check stays entirely at build time — the
/// exact analogue of the oracle's [`ConvertOperand`](coding-adventures-cobol-runtime).
enum ConvOperand {
    /// A string literal — its own bytes bake the compile-time table.
    Literal(String),
    /// A `PIC X` item: `reg` is its string register, `width` its declared width.
    Item { reg: String, width: usize },
}

impl ConvOperand {
    /// The operand's length in characters — a literal's char count, or the item's
    /// declared width. Both are compile-time constants, so the equal-length check
    /// mixing a literal on one side with an item on the other never touches runtime.
    fn len(&self) -> usize {
        match self {
            ConvOperand::Literal(s) => s.chars().count(),
            ConvOperand::Item { width, .. } => *width,
        }
    }
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

/// A value-test for an alphanumeric (`PIC X`) level-88, resolved to owned strings
/// so the mutable emit phase holds no borrow into `self.conditions`: equality with
/// one discrete string, or membership in an inclusive `lo THRU hi` string range.
/// Mirrors [`ValueTest`] for the alphanumeric `str_cmp` path.
enum StrValueTest {
    Eq(String),
    InRange(String, String),
}

/// Whether every level-88 `VALUE` item is a STRING value: a discrete string
/// literal (`Single(Src::Str)`) OR an inclusive `THRU` range BOTH of whose bounds
/// are string literals (`Range(Src::Str, Src::Str)`). This is the accept predicate
/// for a condition-name on an ALPHANUMERIC (`PIC X`) conditional variable:
///
///   * a discrete string VALUE reads (equality) and SETs (store) exactly like
///     `MOVE "…" TO item`;
///   * a string `THRU` range reads as an inclusive `lo <= var <= hi` alphanumeric
///     comparison and SETs to its low bound `lo` — both through the SAME `str_cmp`
///     machinery an `IF`/`MOVE` uses.
///
/// A range with a NON-string bound (`"A" THRU 5`), a numeric/figurative VALUE, or
/// a mixed string/numeric list still fails → still a later rung, so this returns
/// `false` and the caller rejects. The predicate is logically IDENTICAL to the
/// oracle's (`Lit::Str` there, `Src::Str` here), so both engines accept and reject
/// the very same programs.
fn all_str_values(values: &[ValueSpec]) -> bool {
    values.iter().all(|v| match v {
        ValueSpec::Single(s) => matches!(s, Src::Str(_)),
        ValueSpec::Range(lo, hi) => matches!(lo, Src::Str(_)) && matches!(hi, Src::Str(_)),
    })
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
    /// Whether the most recent non-88 data entry was an UNNAMED (`FILLER`) item.
    /// A `FILLER` takes no `by_name` slot and — with group items deferred — is not
    /// pushed to [`Self::items`], so a following level-88's `var = items.len()-1`
    /// would silently bind to the LAST NAMED item instead of the FILLER, diverging
    /// from the oracle (which does model the FILLER). This flag lets
    /// [`Self::collect_condition_name`] reject a level-88 whose conditional variable
    /// is a FILLER — co-totally with the oracle, which rejects the same via an empty
    /// `var_name`. It is set true for a FILLER entry, false for a NAMED non-88 entry,
    /// and LEFT UNCHANGED for a level-88 (so several 88s after one FILLER all reject).
    prev_entry_unnamed_filler: bool,
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
        // no referable name; with group items deferred it plays no role yet. Record
        // that the most recent non-88 entry was a FILLER, so a following level-88 —
        // which would bind `var` to the last NAMED item, not this FILLER — is
        // rejected co-totally with the oracle instead of silently diverging.
        let Some(name) = first_token(entry, "NAME") else {
            self.prev_entry_unnamed_filler = true;
            return Ok(());
        };
        // A level-88 entry declares a boolean condition-name over the most recent
        // item (its "conditional variable"). It takes no storage and no picture —
        // register the name → (variable, value) and return. It leaves the
        // FILLER flag UNCHANGED (so several 88s after one FILLER all reject).
        let level = first_token(entry, "NUMBER").and_then(|s| s.parse::<u32>().ok());
        if level == Some(88) {
            return self.collect_condition_name(&name, entry);
        }
        // A NAMED non-88 data entry (elementary or group) clears the FILLER flag:
        // a following level-88 now qualifies this named item.
        self.prev_entry_unnamed_filler = false;
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
        // A level-88 whose conditional variable is an UNNAMED (FILLER) item is a
        // later rung — rejected BEFORE `checked_sub` so the message matches the
        // oracle's whether or not a prior named item exists. The FILLER is not in
        // `self.items` here, so `var` would otherwise bind to the wrong (last named)
        // item; the oracle rejects the same case via an empty `var_name`.
        if self.prev_entry_unnamed_filler {
            return Err(CompileError::Unsupported(
                "a level-88 condition-name on an unnamed (FILLER) conditional variable is a later rung"
                    .into(),
            ));
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
            "string_stmt" => self.emit_string(verb),
            "unstring_stmt" => self.emit_unstring(verb),
            "inspect_stmt" => self.emit_inspect(verb),
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
                Operandy::RefMod { base, start, len } => {
                    let (reg, _len) = self.ref_mod_slice(&base, &start, &len)?;
                    self.emit("print_str", None, vec![Operand::Var(reg)], "void");
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
                        // Cross-category **numeric → alphanumeric**: a numeric
                        // source (`PIC 9(i)V9(d)` or `PIC S9(i)V9(d)`) moved into an
                        // alphanumeric receiver is treated by COBOL as though it
                        // were an alphanumeric item holding its digit characters —
                        // the item's `(i + d)`-digit zero-padded magnitude image,
                        // the integer part followed by the fractional part with NO
                        // decimal point (an INTEGER source, `d = 0`, is the special
                        // case). This is the very image `Decimal::digits()` yields
                        // (`int + frac`) and, for an unsigned integer, the digits
                        // `DISPLAY` shows.
                        //
                        // For a SIGNED source (`PIC S9…`) the image additionally
                        // carries the operational sign as a TRAILING OVERPUNCH on the
                        // units (last) digit — the same zoned-decimal encoding the
                        // runtime already produces when it `DISPLAY`s a signed field,
                        // and the same table `__cob_print_signed` uses:
                        //
                        //   | units u  | 0 1 2 3 4 5 6 7 8 9 |
                        //   | positive | { A B C D E F G H I |
                        //   | negative | } J K L M N O P Q R |
                        //
                        // So `S9(3) = +123` → `"12C"`, `= −123` → `"12L"`, and
                        // `S9V9 = −4.2` → `"4K"`. Note the overpunch is driven by the
                        // *item* being signed, not by the value's sign: a signed
                        // POSITIVE value still overpunches (its `{A…I` positive row),
                        // which is exactly why an unsigned source (`"123"`) and a
                        // signed positive source (`"12C"`) differ. An unsigned source
                        // has no sign, so its image is the plain magnitude (unchanged
                        // from before this rung).
                        //
                        // The image is then moved by the alphanumeric rules
                        // (LEFT-justified, space-padded on the right if wider,
                        // truncated on the right if narrower). We build that
                        // `(i + d)`-character image at run time from the scaled
                        // numeric slot — whose magnitude is already `value * 10^d`, so
                        // its full `(i + d)` digits ARE the image (no point inserted)
                        // — then feed it through the same char-store path a
                        // same-category alphanumeric MOVE uses, so the compiler and
                        // the oracle emit byte-identical bytes.
                        (
                            ItemKind::Numeric { signed, .. },
                            ItemKind::Char { .. },
                        ) => {
                            let signed = *signed;
                            let (int_d, dec_d) = self.numeric_dims(src_idx);
                            let n = int_d + dec_d;
                            let src_reg = self.items[src_idx].reg.clone();
                            let image = if signed {
                                self.emit_signed_num_alpha_image(&src_reg, n)
                            } else {
                                self.emit_num_digit_string(&src_reg, n)
                            };
                            self.move_str_into_char(&image, n, didx);
                        }
                        // Cross-category **alphanumeric → numeric** (the reverse
                        // direction): an alphanumeric source (`PIC X(m)`) moved into
                        // a numeric receiver `PIC [S]9(i)V9(d)` — SIGNED or unsigned;
                        // `d` may be 0 (an INTEGER) or > 0 (a SCALED receiver).
                        //
                        // A SIGNED receiver is handled here too (guard relaxed to drop
                        // the `signed: false` constraint). The WHY: an alphanumeric
                        // source carries NO operational sign — COBOL does not read an
                        // overpunch from a plain `PIC X` source — so the receiver
                        // stores the folded MAGNITUDE and its sign is ALWAYS POSITIVE.
                        // `store_scaled` already respects the receiver's signedness: it
                        // reads `self.item_signed(idx)` and re-applies the sign of the
                        // (already-positive) folded value via `reapply_sign`, a no-op
                        // for a positive value → a positive value is stored, byte-
                        // identical to the oracle. DISPLAY of the signed field then
                        // overpunches the units digit on its POSITIVE row (`{A…I`),
                        // via the same `overpunch_trailing` path signed DISPLAY already
                        // uses — untouched here. The fold/scale rule is IDENTICAL to
                        // the unsigned path below.
                        //
                        // COBOL reads the source's `m` characters as an unsigned
                        // integer `V` (fold `V = V*10 + (byte - '0')` left-to-right),
                        // and that folded integer IS the receiver's scaled-slot
                        // magnitude directly: it fills the receiver's `(i + d)` digit
                        // positions RIGHT-justified, with the implied point sitting
                        // `d` places from the right. So the receiver's slot is
                        // `V mod 10^(i+d)` — left-zero-padded when the source is
                        // shorter than `(i + d)`, high-order-truncated when longer.
                        // This is NOT the arithmetic decimal-align rule: `V` is not
                        // multiplied by `10^d`; the fold already lands at scale `d`.
                        //
                        //   MOVE "042"   TO 9(2)V9  → V=42    → slot 042 → reads 4.2
                        //   MOVE "42"    TO 9(2)V9  → V=42    → slot 042 → reads 4.2
                        //   MOVE "12345" TO 9(2)V9  → V=12345 → slot 345 → reads 34.5
                        //
                        // We fold the `m` bytes into an `i64` and store it through the
                        // SAME numeric-store helper a numeric MOVE/COMPUTE uses. The
                        // key is the source scale we hand `store_scaled`: because the
                        // fold already IS the slot magnitude at scale `d`, we claim
                        // the receiver's OWN scale `d` as the value scale. Then
                        // `store_scaled` rescales `d → d` (a no-op — no shift) and
                        // keeps the low-order `(i + d)` digits (`mag mod 10^(i+d)`),
                        // which is exactly `V mod 10^(i+d)`. Passing scale `0` instead
                        // would up-shift by `10^d` (the wrong, arithmetic rule). For
                        // `d = 0` this reproduces the old integer-receiver behaviour
                        // byte-for-byte. `value_max_int = m` only feeds the up-scale
                        // overflow guard, which never fires here (from-scale equals
                        // to-scale, so there is no up-shift), so its exact value is
                        // immaterial; `m` (the source width) is a safe upper bound.
                        //
                        // This is byte-identical to the oracle (which folds the
                        // identical per-character arithmetic and stores via
                        // `move_into_numeric` with the fold split at scale `d`). This
                        // rung scopes to an ALL-DIGIT source; a non-digit byte runs
                        // the same `(byte - '0')` arithmetic on both engines
                        // (defined-but-unspecified, identical), so no reject is needed
                        // and no test exercises it.
                        (
                            ItemKind::Char { .. },
                            ItemKind::Numeric { dec_digits: d, .. },
                        ) => {
                            let d = *d;
                            let m = self.items[src_idx].width();
                            // Guard the `i64` fold: an all-digit source of ≤ 18
                            // characters stays below `10^18 < i64::MAX`, so the fold
                            // never overflows on either engine; a wider source is a
                            // clean later rung (both engines reject it identically).
                            if m > NUMERIC_MAX_DIGITS {
                                return Err(CompileError::Unsupported(format!(
                                    "alphanumeric → numeric MOVE from {name} into {dst}: a source \
                                     wider than {NUMERIC_MAX_DIGITS} characters is a later rung \
                                     (its {m}-digit fold could overflow the i64 intermediate)"
                                )));
                            }
                            let src_reg = self.items[src_idx].reg.clone();
                            let value = self.emit_str_to_int(&src_reg, m);
                            // Take the MAGNITUDE of the fold BEFORE storing, exactly
                            // as the oracle does (`value.unsigned_abs()`). This is the
                            // crux of the SIGNED-receiver relaxation: a source byte
                            // below `'0'` — most commonly a SPACE (an uninitialised
                            // `PIC X` is spaces) — makes `(byte - '0')` negative, so
                            // the raw fold goes NEGATIVE. For an UNSIGNED receiver
                            // `store_scaled` abs-es internally and ignores the sign, so
                            // this was invisible; but for a SIGNED receiver
                            // `store_scaled` re-applies the sign of the value it is
                            // handed (`reapply_sign`), which would wrongly store a
                            // NEGATIVE value and overpunch DISPLAY on the negative row.
                            // Absing here makes the handed value non-negative →
                            // `reapply_sign` is a genuine no-op → a POSITIVE value is
                            // stored, byte-identical to the oracle (which builds
                            // `Decimal { neg: false }`). For an all-digit source the
                            // fold is already non-negative, so this is a no-op and the
                            // unsigned path's output is unchanged.
                            self.emit_abs(&value);
                            // Claim the receiver's scale `d` for the fold — see the
                            // note above: the fold already IS the slot magnitude at
                            // scale `d`, so `store_scaled` does no shift and keeps the
                            // low-order `(i + d)` digits.
                            self.store_scaled(&dst, &value, d, m, false)?;
                        }
                        // The Char↔Numeric MOVE matrix is now COMPLETE. `ItemKind`
                        // has exactly two variants (Char, Numeric), so the four arms
                        // above — (Numeric,Numeric), (Char,Char), (Numeric,Char) and
                        // (Char,Numeric), covering both directions and both
                        // signednesses — are EXHAUSTIVE over `(kind, kind)`. A
                        // catch-all `_` reject arm would now be unreachable, which the
                        // compiler flags as an `unreachable_patterns` warning (CI
                        // denies warnings), so we omit it rather than let a dead arm
                        // lie about what is unsupported. Should a THIRD item kind ever
                        // be added, the non-exhaustive match will fail to compile and
                        // force this cross-category logic to be revisited deliberately
                        // — a stronger guarantee than a silent catch-all.
                    }
                }
                // Reference-modification SOURCE `base(start:len)` moved into an
                // ALPHANUMERIC receiver. `ref_mod_slice` emits the SAME `str_slice`
                // DISPLAY/comparison use (so the slice bytes already agree with the
                // oracle) and reports its length as a `SliceLen` — compile-time
                // constant for a literal:literal (or literal:) refmod, run-time for a
                // computed (data-name) index. `move_slice_into_char` then fits the
                // slice to the receiver's width by the ordinary alphanumeric char
                // rule (LEFT-justify; space-pad on the right if wider; truncate on
                // the right if narrower) — the same reshape a same-category char MOVE
                // performs — so the receiver holds byte-identical bytes to the oracle
                // (`move_into` → `move_into_char`). A NUMERIC receiver (de-editing a
                // slice into a numeric field) stays a later rung on both engines. The
                // slice is byte-based here and char-based in the oracle; they coincide
                // on the ASCII-prefix windows this rung targets (a multi-byte char
                // inside/after the window is the pre-existing refmod byte-vs-char chip
                // shared with DISPLAY/comparison, not introduced here).
                Operandy::RefMod { base, start, len } => {
                    let didx = self.item_index(&dst)?;
                    match &self.items[didx].kind {
                        ItemKind::Char { .. } => {
                            let (reg, slice_len) = self.ref_mod_slice(base, start, len)?;
                            self.move_slice_into_char(&reg, slice_len, didx);
                        }
                        ItemKind::Numeric { .. } => {
                            return Err(CompileError::Unsupported(format!(
                                "MOVE of a reference-modification source into the numeric \
                                 receiver {dst} is a later rung (an alphanumeric receiver \
                                 is supported)"
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
    /// low bound).
    ///
    /// A **numeric** variable takes the first value formatted into its picture at
    /// compile time — the same `const`-into-slot store `MOVE <literal>` emits.
    ///
    /// An **alphanumeric** (`PIC X`) variable is supported for STRING values: when
    /// every VALUE item is a discrete string literal or a string `THRU` range
    /// ([`all_str_values`]), SET stores the FIRST value's string into the slot
    /// exactly as `MOVE "…" TO item` — a leading discrete string `s` stores `s`; a
    /// leading range `lo THRU _` stores its LOW bound `lo`, mirroring the numeric
    /// arm. [`format_into_picture`] fits the string to the receiver width and it is
    /// emitted as the slot's `str_const`, byte-identical to the oracle's
    /// `move_into_char`. A range with a NON-string bound, a numeric/figurative
    /// VALUE, or a mixed list on an alphanumeric variable stays a later rung —
    /// rejected identically to the oracle.
    fn emit_set(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let cond_name = first_token(verb, "NAME")
            .ok_or_else(|| CompileError::Malformed("SET without a condition-name".into()))?;
        let cn = self.conditions.get(&cond_name).ok_or_else(|| {
            CompileError::Unsupported(format!("reference to condition-name {cond_name} (undeclared)"))
        })?;
        let var = cn.var;
        // An alphanumeric slot stores a discrete string exactly as `MOVE "…" TO
        // item`; extract the fitted image while the (immutable) `cn` borrow is live,
        // then emit. A THRU range or a non-string VALUE stays a later rung.
        if matches!(&self.items[var].kind, ItemKind::Char { .. }) {
            if !all_str_values(&cn.values) {
                return Err(CompileError::Unsupported(
                    "SET … TO TRUE on an alphanumeric conditional variable needs string VALUEs (a \
                     discrete string or a string THRU range; a range with a non-string bound or a \
                     numeric/figurative VALUE is a later rung)"
                        .into(),
                ));
            }
            let picture = Picture::Alphanumeric { size: self.items[var].width() };
            // The FIRST value's string: a discrete string, or a range's LOW bound.
            let src = match cn.values.first() {
                Some(ValueSpec::Single(s)) | Some(ValueSpec::Range(s, _)) => s,
                None => {
                    return Err(CompileError::Malformed(format!(
                        "condition-name {cond_name} has no VALUE"
                    )))
                }
            };
            let image = format_into_picture(src, &picture)
                .map_err(|m| CompileError::Unsupported(format!("SET {cond_name}: {m}")))?;
            let reg = self.items[var].reg.clone();
            self.emit("str_const", Some(&reg), vec![Operand::Str(image)], "str");
            return Ok(());
        }
        let (int_digits, dec_digits, signed) = match &self.items[var].kind {
            ItemKind::Numeric { int_digits, dec_digits, signed, .. } => (*int_digits, *dec_digits, *signed),
            ItemKind::Char { .. } => unreachable!("the Char case returned above"),
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
            let cond = self.emit_when_match(subject_node, wb)?;
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

    /// Emit a boolean register that is true when the EVALUATE subject matches any
    /// value in a `when_branch`'s list. Each subject-vs-value comparison is routed
    /// through [`Self::emit_operand_relation`] — the *same* category dispatch an
    /// `IF subject <relop> value` relation uses — so EVALUATE inherits IF's full
    /// category handling (numeric, alphanumeric, and mixed numeric↔alphanumeric
    /// with unsigned/signed/scaled images, figuratives, and ZERO routing) and its
    /// deferral set by construction. A single value `[v]` is `cmp_eq(subject, v)`;
    /// a `THRU` range `[lo, hi]` is `and(cmp_ge(subject, lo), cmp_le(subject, hi))`;
    /// the whole value-list `OR`-folds.
    fn emit_when_match(&mut self, subject: &GrammarASTNode, wb: &GrammarASTNode) -> Result<String, CompileError> {
        let mut acc: Option<String> = None;
        for wv in child_nodes(wb, "when_value") {
            let ops = child_nodes(wv, "operand");
            let b = match ops.as_slice() {
                [one] => self.emit_operand_relation(subject, one, "cmp_eq")?,
                [lo, hi] => {
                    let ge = self.emit_operand_relation(subject, lo, "cmp_ge")?;
                    let le = self.emit_operand_relation(subject, hi, "cmp_le")?;
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

    /// Build, at run time, the `n`-character zero-padded decimal image of an
    /// **unsigned integer** numeric slot `num_reg` (holding its magnitude as an
    /// `i64`) and return the fresh `str` register that holds it. This is exactly
    /// the digit string a `DISPLAY` of the same `PIC 9(n)` item prints — but
    /// materialised as a *string* rather than putchar'd — so a numeric→alphanumeric
    /// MOVE and a DISPLAY of the source agree digit-for-digit.
    ///
    /// The image is assembled most-significant digit first. For the digit at
    /// 0-based position `k` (from the left of an `n`-wide field), the place value
    /// is `p = 10^(n-1-k)`, and the digit itself is
    ///
    /// ```text
    ///   d = (num / p) % 10
    /// ```
    ///
    /// The `/ p` shifts that digit down to the units place and the `% 10` keeps a
    /// single digit — so a value with **more** than `n` digits silently drops its
    /// high-order digits (COBOL's high-order overflow, matching the recursive
    /// `__cob_print_padded` helper). A single digit `d` (0..=9) is turned into its
    /// 1-character string by slicing the constant lookup table `"0123456789"` at
    /// `[d, d+1)` — no per-digit branch table needed — and the `n` pieces are
    /// concatenated left to right onto an initially empty accumulator. Because each
    /// piece is exactly one character, the result is exactly `n` characters wide,
    /// the same fixed-width image the oracle's `Decimal::digits()` yields for the
    /// item.
    ///
    /// ```text
    ///   PIC 9(3) holding 42  (slot = 42)         n = 3
    ///     k=0: p=100  d=(42/100)%10 = 0  -> "0"
    ///     k=1: p=10   d=(42/10)%10  = 4  -> "4"
    ///     k=2: p=1    d=(42/1)%10   = 2  -> "2"
    ///   result = "042"
    /// ```
    fn emit_num_digit_string(&mut self, num_reg: &str, n: usize) -> String {
        // The shared digit lookup table and the constant 10 divisor/modulus.
        let table = self.fresh("_ndtbl");
        self.emit("str_const", Some(&table), vec![Operand::Str("0123456789".into())], "str");
        let ten = self.fresh("_ndten");
        self.emit("const", Some(&ten), vec![Operand::Int(10)], "i64");
        let one = self.fresh("_ndone");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        // result = "" — the accumulator we build the n digits into.
        let result = self.fresh("_ndres");
        self.emit("str_const", Some(&result), vec![Operand::Str(String::new())], "str");
        for k in 0..n {
            // q = num / 10^(n-1-k); for the units place (p == 1) that is num itself.
            let place = 10i64.pow((n - 1 - k) as u32);
            let q = if place == 1 {
                num_reg.to_string()
            } else {
                let pr = self.fresh("_ndp");
                self.emit("const", Some(&pr), vec![Operand::Int(place)], "i64");
                let q = self.fresh("_ndq");
                self.emit("div", Some(&q), vec![Operand::Var(num_reg.to_string()), Operand::Var(pr)], "i64");
                q
            };
            // d = q % 10 (this position's digit); d1 = d + 1 (slice end).
            let d = self.fresh("_ndd");
            self.emit("mod", Some(&d), vec![Operand::Var(q), Operand::Var(ten.clone())], "i64");
            let d1 = self.fresh("_ndd1");
            self.emit("add", Some(&d1), vec![Operand::Var(d.clone()), Operand::Var(one.clone())], "i64");
            // ch = table[d..d+1] — the 1-character string for this digit.
            let ch = self.fresh("_ndch");
            self.emit(
                "str_slice",
                Some(&ch),
                vec![Operand::Var(table.clone()), Operand::Var(d), Operand::Var(d1)],
                "str",
            );
            // result = result + ch.
            self.emit(
                "str_concat",
                Some(&result),
                vec![Operand::Var(result.clone()), Operand::Var(ch)],
                "str",
            );
        }
        result
    }

    /// Build the `n`-character alphanumeric image of a **signed** DISPLAY numeric
    /// slot `slot_reg`: the `n`-digit zero-padded MAGNITUDE with the operational
    /// sign folded into a TRAILING OVERPUNCH on the units (last) digit. This is the
    /// same zoned-decimal encoding the runtime's `overpunch_trailing` produces and
    /// `__cob_print_signed` prints on `DISPLAY`:
    ///
    /// | units u  | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
    /// |----------|---|---|---|---|---|---|---|---|---|---|
    /// | positive | { | A | B | C | D | E | F | G | H | I |
    /// | negative | } | J | K | L | M | N | O | P | Q | R |
    ///
    /// The image's leading `n − 1` characters are the magnitude's high digits; only
    /// the units digit is overpunched. The sign is the *value's* sign (a signed
    /// POSITIVE value takes the positive row), so `S9(3) = +123` → `"12C"`,
    /// `= −123` → `"12L"`, and `S9V9 = −4.2` → `"4K"`.
    ///
    /// Rather than branch to pick a positive/negative lookup table, both rows are
    /// laid end to end in ONE 20-character constant — positive `{…I` at indices
    /// `0..=9`, negative `}…R` at indices `10..=19` — and indexed by
    ///
    /// ```text
    ///   neg   = (slot < 0) ? 1 : 0        (cmp_lt yields 0/1)
    ///   units = |slot| % 10
    ///   idx   = units + neg*10            (the 0..19 slot in the combined table)
    /// ```
    ///
    /// so `table[idx..idx+1]` is exactly `overpunch_trailing`'s chosen character:
    /// `POS[u]` at index `u`, `NEG[u]` at index `10 + u`. Because the units digit is
    /// `|slot| % 10` and the sign is `slot < 0`, the byte matches the oracle (which
    /// overpunches the magnitude image's last digit by the identical table). For
    /// `n == 1` the leading slice `image[0..0]` is empty, so the result is just the
    /// one overpunch character. The finished `n`-char image feeds the same
    /// char-store path the unsigned image and same-category alphanumeric MOVE use.
    fn emit_signed_num_alpha_image(&mut self, slot_reg: &str, n: usize) -> String {
        // neg = (slot < 0) ? 1 : 0; mag = |slot|.
        let zero = self.fresh("_soz");
        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
        let neg = self.fresh("_soneg");
        self.emit(
            "cmp_lt",
            Some(&neg),
            vec![Operand::Var(slot_reg.to_string()), Operand::Var(zero)],
            "i64",
        );
        let mag = self.fresh("_somag");
        self.emit("mov", Some(&mag), vec![Operand::Var(slot_reg.to_string())], "i64");
        self.emit_abs(&mag);
        // The n-digit magnitude image (unsigned digits, most-significant first).
        let image = self.emit_num_digit_string(&mag, n);
        // idx = (mag % 10) + neg*10 — the position in the combined overpunch table.
        let ten = self.fresh("_soten");
        self.emit("const", Some(&ten), vec![Operand::Int(10)], "i64");
        let units = self.fresh("_sou");
        self.emit("mod", Some(&units), vec![Operand::Var(mag.clone()), Operand::Var(ten.clone())], "i64");
        let off = self.fresh("_sooff");
        self.emit("mul", Some(&off), vec![Operand::Var(neg), Operand::Var(ten)], "i64");
        let idx = self.fresh("_soidx");
        self.emit("add", Some(&idx), vec![Operand::Var(units), Operand::Var(off)], "i64");
        let one = self.fresh("_soone");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let idx1 = self.fresh("_soidx1");
        self.emit("add", Some(&idx1), vec![Operand::Var(idx.clone()), Operand::Var(one)], "i64");
        // ch = table[idx..idx+1]: positive row {…I at 0..9, negative row }…R at 10..19.
        let table = self.fresh("_sotbl");
        self.emit(
            "str_const",
            Some(&table),
            vec![Operand::Str("{ABCDEFGHI}JKLMNOPQR".into())],
            "str",
        );
        let ch = self.fresh("_soch");
        self.emit(
            "str_slice",
            Some(&ch),
            vec![Operand::Var(table), Operand::Var(idx), Operand::Var(idx1)],
            "str",
        );
        // result = image[0..n-1] ++ ch — replace the units digit with its overpunch.
        let start = self.fresh("_sos0");
        self.emit("const", Some(&start), vec![Operand::Int(0)], "i64");
        let head_end = self.fresh("_sohe");
        self.emit("const", Some(&head_end), vec![Operand::Int((n - 1) as i64)], "i64");
        let head = self.fresh("_sohead");
        self.emit(
            "str_slice",
            Some(&head),
            vec![Operand::Var(image), Operand::Var(start), Operand::Var(head_end)],
            "str",
        );
        let result = self.fresh("_sores");
        self.emit("str_concat", Some(&result), vec![Operand::Var(head), Operand::Var(ch)], "str");
        result
    }

    /// Store a run-time `str` register `src_reg` of compile-time-known length
    /// `src_len` into an alphanumeric receiver item `didx`, by the **alphanumeric**
    /// MOVE rule — LEFT-justified, space-padded on the right when the receiver is
    /// wider, truncated on the right when narrower. This is the string-source twin
    /// of [`Self::move_char_item`] (which reshapes one item into another); both
    /// funnel numeric→alphanumeric and alphanumeric→alphanumeric moves through the
    /// identical `str_slice` / `str_concat` reshape the oracle's `move_into_char`
    /// performs, so the stored bytes agree.
    fn move_str_into_char(&mut self, src_reg: &str, src_len: usize, didx: usize) {
        let recv_w = self.items[didx].width();
        let recv_reg = self.items[didx].reg.clone();
        if recv_w <= src_len {
            // Receiver no wider than the source: keep the leftmost `recv_w`.
            let start = self.fresh("_ms0");
            self.emit("const", Some(&start), vec![Operand::Int(0)], "i64");
            let end = self.fresh("_msn");
            self.emit("const", Some(&end), vec![Operand::Int(recv_w as i64)], "i64");
            self.emit(
                "str_slice",
                Some(&recv_reg),
                vec![Operand::Var(src_reg.to_string()), Operand::Var(start), Operand::Var(end)],
                "str",
            );
        } else {
            // Receiver wider: left-justify and space-pad the tail.
            let pad = self.spaces_const(recv_w - src_len);
            self.emit(
                "str_concat",
                Some(&recv_reg),
                vec![Operand::Var(src_reg.to_string()), Operand::Var(pad)],
                "str",
            );
        }
    }

    /// Fit a reference-modification slice — held in `reg` with length `slice_len`
    /// — into the alphanumeric receiver `didx`, by the ordinary alphanumeric char
    /// rule: LEFT-justify, space-pad the tail when the receiver is wider than the
    /// slice, truncate on the right when narrower. This is the very reshape
    /// [`move_into_char`](../../cobol-runtime) applies on the oracle side, so the
    /// receiver ends up holding byte-identical bytes.
    ///
    /// Two length regimes:
    ///
    /// * **compile-time constant** (`SliceLen::Const`, a literal:literal or
    ///   literal: refmod) — the slice width is known, so this defers to
    ///   [`Self::move_str_into_char`], the SAME const-width char fit a plain
    ///   alphanumeric-item MOVE uses.
    ///
    /// * **run-time** (`SliceLen::Runtime`, a computed data-name index) — the slice
    ///   width is unknown at compile time, so neither branch of `move_str_into_char`
    ///   can be chosen statically. Instead we lower a single width-agnostic form:
    ///   concatenate `recv_w` trailing spaces onto the slice (making the result at
    ///   least `recv_w` characters for ANY slice length `L ≥ 0`), then keep the
    ///   leftmost `recv_w`. For `L ≥ recv_w` that is the slice's first `recv_w`
    ///   characters (right-truncation); for `L < recv_w` it is the `L` slice
    ///   characters followed by `recv_w − L` spaces (right space-pad) — exactly
    ///   `move_into_char`'s two cases, so the bytes match the oracle regardless of
    ///   the run-time length.
    fn move_slice_into_char(&mut self, reg: &str, slice_len: SliceLen, didx: usize) {
        match slice_len {
            SliceLen::Const(len) => self.move_str_into_char(reg, len, didx),
            SliceLen::Runtime { .. } => {
                let recv_w = self.items[didx].width();
                let recv_reg = self.items[didx].reg.clone();
                // slice ++ recv_w spaces  →  length L + recv_w ≥ recv_w always.
                let pad = self.spaces_const(recv_w);
                let padded = self.fresh("_mrc");
                self.emit(
                    "str_concat",
                    Some(&padded),
                    vec![Operand::Var(reg.to_string()), Operand::Var(pad)],
                    "str",
                );
                // Keep the leftmost recv_w characters of the padded slice.
                let start = self.fresh("_mrs0");
                self.emit("const", Some(&start), vec![Operand::Int(0)], "i64");
                let end = self.fresh("_mrsn");
                self.emit("const", Some(&end), vec![Operand::Int(recv_w as i64)], "i64");
                self.emit(
                    "str_slice",
                    Some(&recv_reg),
                    vec![Operand::Var(padded), Operand::Var(start), Operand::Var(end)],
                    "str",
                );
            }
        }
    }

    /// Fold an alphanumeric source register `src_reg` of compile-time width `m`
    /// into the unsigned integer it denotes, and return the fresh `i64` register
    /// holding it. This is the run-time twin of the oracle's per-character fold in
    /// the reverse (alphanumeric → numeric) MOVE: the `m` bytes are read
    /// left-to-right as decimal digits, accumulating
    ///
    /// ```text
    ///   value = 0
    ///   for k in 0..m:  d = src[k] - '0';  value = value*10 + d
    /// ```
    ///
    /// so the source `"042"` folds to `0*10+0 → 0`, `0*10+4 → 4`, `4*10+2 → 42`.
    /// Reading each byte with the IIR `str_index` op and subtracting the constant
    /// `'0'` (48) yields that position's digit; the running `value` is the integer
    /// the whole field denotes — and, per the reverse-MOVE rule, IS the receiver's
    /// scaled-slot magnitude directly (the fold is *not* re-scaled). The
    /// receiver-width truncation (keep the low-order `(i + d)` digits,
    /// `value mod 10^(i+d)`) is applied later by [`Self::store_scaled`] — the same
    /// numeric-store helper a numeric MOVE/COMPUTE uses, handed the receiver's own
    /// scale `d` so it does no shift — so the compiled result matches the oracle,
    /// which runs the identical fold and stores through `move_into_numeric` with the
    /// fold split at scale `d` (`d = 0` for an integer receiver).
    ///
    /// The caller has already bounded `m ≤ 18`, so the `i64` fold of an all-digit
    /// source (`< 10^18 < i64::MAX`) never overflows.
    fn emit_str_to_int(&mut self, src_reg: &str, m: usize) -> String {
        let value = self.fresh("_a2nv");
        self.emit("const", Some(&value), vec![Operand::Int(0)], "i64");
        let ten = self.fresh("_a2n10");
        self.emit("const", Some(&ten), vec![Operand::Int(10)], "i64");
        let zero_byte = self.fresh("_a2n0");
        self.emit("const", Some(&zero_byte), vec![Operand::Int(b'0' as i64)], "i64");
        for k in 0..m {
            // c = src[k] (a byte, as i64); d = c - '0' (this position's digit).
            let kreg = self.fresh("_a2nk");
            self.emit("const", Some(&kreg), vec![Operand::Int(k as i64)], "i64");
            let c = self.fresh("_a2nc");
            self.emit(
                "str_index",
                Some(&c),
                vec![Operand::Var(src_reg.to_string()), Operand::Var(kreg)],
                "i64",
            );
            let d = self.fresh("_a2nd");
            self.emit("sub", Some(&d), vec![Operand::Var(c), Operand::Var(zero_byte.clone())], "i64");
            // value = value*10 + d.
            self.emit("mul", Some(&value), vec![Operand::Var(value.clone()), Operand::Var(ten.clone())], "i64");
            self.emit("add", Some(&value), vec![Operand::Var(value.clone()), Operand::Var(d)], "i64");
        }
        value
    }

    /// Emit the constant-index `str_slice` for a reference modification
    /// `base(start:len)` and return `(sliced_reg, actual_len)`.
    ///
    /// COBOL reference modification is 1-based: `base(start:len)` selects the
    /// characters at 1-based positions `start .. start+len-1`, i.e. the 0-based
    /// half-open byte range `[start-1, start-1+len)`. An omitted `len` runs to
    /// the end of the item, so `len = width - (start-1)`.
    ///
    /// ```text
    ///   base = "ABCDE"  (width 5)          positions:  1 2 3 4 5
    ///   base(2:3)  ->  start0 = 1, len 3  ->  slice [1,4)  ->  "BCD"
    ///   base(3:)   ->  start0 = 2, len 2  ->  slice [2,5)  ->  "CDE"
    /// ```
    ///
    /// Two paths share one lowering:
    ///
    /// * **literal:literal** (or `literal:`) — both indices are compile-time
    ///   constants, so this mirrors [`Self::move_char_item`]'s const-index
    ///   `str_slice`: two `const` i64 registers feed a `str_slice` producing a
    ///   fresh `str`. Bounds are validated at compile time (`start >= 1`,
    ///   `start-1+len <= width`); an out-of-range *constant* refmod is a
    ///   compile-time [`CompileError::Unsupported`], never a run-time trap.
    ///
    /// * **computed** — the moment either index is a data-name, `start0` and
    ///   `end` are built with `const`/`sub`/`add` over the index registers and
    ///   fed to `str_slice`. Bounds are checked **at run time**: the emitted
    ///   `str_slice` traps (in the VM/wasm backends) exactly when
    ///   `start0 < 0 || end < start0 || end > width`. The oracle's
    ///   `refmod_string` applies the identical predicate, so an in-range program
    ///   slices byte-identically and an out-of-range one errors on both engines.
    ///
    /// The base must be an alphanumeric item; a computed index must be an
    /// unsigned integer item.
    fn ref_mod_slice(
        &mut self,
        base: &str,
        start: &RefIndex,
        len: &Option<RefIndex>,
    ) -> Result<(String, SliceLen), CompileError> {
        let idx = self.item_index(base)?;
        let width = match &self.items[idx].kind {
            ItemKind::Char { .. } => self.items[idx].width(),
            ItemKind::Numeric { .. } => {
                return Err(CompileError::Unsupported(
                    "reference modification of a numeric item is a later rung".into(),
                ));
            }
        };
        // Constant-fold the literal:literal (and literal:) case so #8673's output
        // — and its compile-time out-of-range reject — is preserved exactly.
        if let (RefIndex::Lit(s), l) = (start, len) {
            if let Some(actual) = const_refmod_len(*s, l, width)? {
                let src_reg = self.items[idx].reg.clone();
                let start0 = *s - 1;
                let start_reg = self.fresh("_rm0");
                self.emit("const", Some(&start_reg), vec![Operand::Int(start0 as i64)], "i64");
                let end_reg = self.fresh("_rmn");
                self.emit("const", Some(&end_reg), vec![Operand::Int((start0 + actual) as i64)], "i64");
                let out = self.fresh("_rm");
                self.emit(
                    "str_slice",
                    Some(&out),
                    vec![Operand::Var(src_reg), Operand::Var(start_reg), Operand::Var(end_reg)],
                    "str",
                );
                return Ok((out, SliceLen::Const(actual)));
            }
        }
        // Computed path: at least one index is a data-name. Read start/len into
        // i64 registers and compute the 0-based half-open [start0, end) bounds,
        // letting the run-time str_slice bounds check enforce the range.
        let src_reg = self.items[idx].reg.clone();
        let start_reg = self.refmod_index_reg(start)?;
        let one = self.fresh("_rm1");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let start0 = self.fresh("_rm0");
        self.emit("sub", Some(&start0), vec![Operand::Var(start_reg), Operand::Var(one)], "i64");
        let end = match len {
            Some(l) => {
                let len_reg = self.refmod_index_reg(l)?;
                let e = self.fresh("_rme");
                self.emit("add", Some(&e), vec![Operand::Var(start0.clone()), Operand::Var(len_reg)], "i64");
                e
            }
            None => {
                // Omitted length runs to the end of the item.
                let e = self.fresh("_rmw");
                self.emit("const", Some(&e), vec![Operand::Int(width as i64)], "i64");
                e
            }
        };
        // The slice's run-time length = end - start0 (needed to space-pad it to a
        // common width in a comparison).
        let len_reg = self.fresh("_rml");
        self.emit("sub", Some(&len_reg), vec![Operand::Var(end.clone()), Operand::Var(start0.clone())], "i64");
        let out = self.fresh("_rm");
        self.emit(
            "str_slice",
            Some(&out),
            vec![Operand::Var(src_reg), Operand::Var(start0), Operand::Var(end)],
            "str",
        );
        Ok((out, SliceLen::Runtime { len_reg, max_len: width }))
    }

    /// Read a reference-modification index ([`RefIndex`]) into a fresh `i64`
    /// register. A literal becomes a `const`; a data-name must be an **unsigned
    /// integer** item (`PIC 9…`, no `S`, no decimals) — its live slot is copied
    /// so evaluating the index never clobbers it. A signed, fractional, or
    /// non-numeric index item is a later rung.
    fn refmod_index_reg(&mut self, ix: &RefIndex) -> Result<String, CompileError> {
        match ix {
            RefIndex::Lit(v) => {
                let reg = self.fresh("_rmi");
                self.emit("const", Some(&reg), vec![Operand::Int(*v as i64)], "i64");
                Ok(reg)
            }
            RefIndex::Name(name) => {
                let iidx = self.item_index(name)?;
                match &self.items[iidx].kind {
                    ItemKind::Numeric { dec_digits: 0, signed: false, .. } => {
                        let slot = self.items[iidx].reg.clone();
                        let reg = self.fresh("_rmi");
                        self.emit("mov", Some(&reg), vec![Operand::Var(slot)], "i64");
                        Ok(reg)
                    }
                    ItemKind::Numeric { .. } => Err(CompileError::Unsupported(format!(
                        "a signed or fractional reference-modification index ({name}) is a later rung"
                    ))),
                    ItemKind::Char { .. } => Err(CompileError::Unsupported(format!(
                        "a non-numeric reference-modification index ({name}) is a later rung"
                    ))),
                }
            }
        }
    }

    /// A `str` register holding `k` spaces — the right-padding for a character
    /// reshape or comparison.
    fn spaces_const(&mut self, k: usize) -> String {
        let reg = self.fresh("_sp");
        self.emit("str_const", Some(&reg), vec![Operand::Str(" ".repeat(k))], "str");
        reg
    }

    /// `STRING s… DELIMITED BY {SIZE | delim} INTO t` — concatenate the sending
    /// fields with a `str_concat` chain, then overlay the result onto the receiver
    /// from the left. COBOL's STRING writes only what it produced and leaves the
    /// rest of `t` UNCHANGED (no space-fill, unlike `MOVE`), truncating at `t`'s
    /// width. The overlay is byte-identical to the `cobol-runtime` oracle's
    /// `exec_string`.
    ///
    /// **`DELIMITED BY SIZE` (`delim = None`).** Every field is taken in full, and
    /// each source and the receiver have a compile-time-known length, so BOTH the
    /// concatenation and the overlay are fixed `str_slice`/`str_concat` sequences:
    ///
    ///   * result longer than `t`  →  `t = str_slice(concat, 0, width)` (truncate);
    ///   * result shorter than `t` →  `t = str_concat(concat, str_slice(t, len, width))`
    ///     — the head is the whole concatenation, the preserved tail is the
    ///     receiver's old `[len, width)` bytes.
    ///
    /// **`DELIMITED BY delim` (`delim = Some`).** Each field contributes only its
    /// PREFIX up to the first delimiter char — a DATA-dependent boundary — so we
    /// emit a genuine per-field scan loop (the same shape UNSTRING uses) and the
    /// running length becomes a RUNTIME value. The overlay therefore also runs at
    /// run time: `clen = str_len(concat); take = min(clen, W); t = concat[0,take] ++
    /// t[take,W]` — the preserved tail `t[take,W]` gives STRING's no-space-fill
    /// rule exactly as the compile-time branch does. The delimiter is reduced by
    /// the SAME `single_delim_code` UNSTRING uses, and must be ASCII: the scan
    /// compares BYTES while the oracle scans CHARS, so a non-ASCII delimiter (and a
    /// non-ASCII string-literal sending field under an active delimiter) is a clean
    /// later rung on both engines.
    fn emit_string(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        // `WITH POINTER` and the two OVERFLOW imperatives are now MODELLED (see the
        // handling below), so nothing is rejected up front.
        let toks = child_tokens(verb);
        let delim_node = child_node(verb, "string_delim")
            .ok_or_else(|| CompileError::Malformed("STRING without DELIMITED BY".into()))?;
        let is_size = child_tokens(delim_node).iter().any(|(k, v)| k == "KEYWORD" && v == "SIZE");
        // Reduce a real delimiter to a single byte-code register (or `None` for
        // `DELIMITED BY SIZE`). The delimiter operand is nested UNDER `string_delim`,
        // so it never collides with the sending-field `operand` children of `verb`.
        let delim_code: Option<String> = if is_size {
            None
        } else {
            let dop = child_node(delim_node, "operand")
                .ok_or_else(|| CompileError::Malformed("STRING DELIMITED BY without a delimiter".into()))?;
            // A single but NON-ASCII string literal (`","` is ASCII, `"é"` is one
            // char but two bytes) is a clean later rung with the SAME message the
            // oracle emits — checked before `single_delim_code`, whose byte-length
            // test would otherwise mislabel it as "multi-character". A multi-char
            // literal (ASCII or not) still reaches `single_delim_code` and is
            // rejected as multi-character, matching the oracle's char-count test.
            if let Operandy::Literal(Src::Str(s)) = read_operand(dop)? {
                if s.chars().count() == 1 && !s.is_ascii() {
                    return Err(CompileError::Unsupported(
                        "STRING with a non-ASCII delimiter is a later rung".into(),
                    ));
                }
            }
            Some(self.single_delim_code(dop, "STRING")?)
        };
        // The sending fields are the DIRECT `operand` children (the delimiter
        // operand is a grandchild under `string_delim`, so it does not collide).
        let sources = child_nodes(verb, "operand");
        if sources.is_empty() {
            return Err(CompileError::Malformed("STRING without a sending field".into()));
        }

        // Resolve the receiver — an alphanumeric item this rung — up front (shared).
        let target = first_token(verb, "NAME")
            .ok_or_else(|| CompileError::Malformed("STRING without an INTO receiver".into()))?;
        let didx = self.item_index(&target)?;
        let width = match &self.items[didx].kind {
            ItemKind::Char { .. } => self.items[didx].width(),
            ItemKind::Numeric { .. } => {
                return Err(CompileError::Unsupported(
                    "STRING into a numeric receiver is a later rung".into(),
                ))
            }
        };
        let recv = self.items[didx].reg.clone();

        // The optional `WITH POINTER p` phrase. `INTO t` always precedes it, so the
        // receiver is the first direct NAME (resolved above) and the pointer NAME is
        // the first NAME after the `POINTER` keyword. (Sending-field identifiers are
        // nested under `operand` nodes, so they are not direct NAME tokens here.)
        let ptr_pos = toks.iter().position(|(k, v)| k == "KEYWORD" && v == "POINTER");
        let pointer_name: Option<String> = ptr_pos.and_then(|pp| {
            toks[pp + 1..].iter().find(|(k, _)| k == "NAME").map(|(_, v)| v.clone())
        });
        // Validate the pointer item's picture at BUILD time — it must be an unsigned
        // integer `PIC 9(n)` (n ≤ 18 so the value fits the `i64` we store it in), the
        // same class INSPECT's counter demands. A signed, fractional, non-numeric, or
        // group pointer is a clean later rung, rejected here with the SAME message the
        // oracle raises at exec time so the accept/reject sets stay co-total. We
        // capture `(index, int_digits)` for the run-time read/write-back below; the
        // range check on the pointer's VALUE cannot be done here (it is a run-time
        // datum), so it is emitted as a guard, exactly like the oracle.
        let pointer: Option<(usize, usize)> = match &pointer_name {
            Some(pname) => {
                let pidx = self.item_index(pname)?;
                let ptr_int_digits = match &self.items[pidx].kind {
                    ItemKind::Numeric { signed: true, .. } => {
                        return Err(CompileError::Unsupported(format!(
                            "STRING … WITH POINTER: a signed pointer {pname} is a later rung"
                        )))
                    }
                    ItemKind::Numeric { dec_digits, .. } if *dec_digits != 0 => {
                        return Err(CompileError::Unsupported(format!(
                            "STRING … WITH POINTER: a non-integer pointer {pname} is a later rung"
                        )))
                    }
                    ItemKind::Numeric { int_digits, .. } if *int_digits > 18 => {
                        return Err(CompileError::Unsupported(format!(
                            "STRING … WITH POINTER: a pointer {pname} wider than 18 digits is a later rung"
                        )))
                    }
                    ItemKind::Numeric { int_digits, .. } => *int_digits,
                    ItemKind::Char { .. } => {
                        return Err(CompileError::Unsupported(format!(
                            "STRING … WITH POINTER: a non-numeric pointer {pname} is a later rung"
                        )))
                    }
                };
                Some((pidx, ptr_int_digits))
            }
            None => None,
        };

        // Each arm yields the `overflow` i64 register (1 / 0) selecting the ON /
        // NOT ON OVERFLOW imperative below. The comparison MUST be the identical one
        // the oracle uses so the accept/skip decision is byte-identical.
        let overflow: String = match &delim_code {
            // `DELIMITED BY SIZE` — every boundary is compile-time-known.
            None => {
                let mut pieces: Vec<(String, usize)> = Vec::with_capacity(sources.len());
                for op in sources {
                    pieces.push(self.string_source(op)?);
                }
                // Concatenate left-to-right; the total length is known at compile time.
                let (mut concat, mut total) = pieces[0].clone();
                for (reg, len) in &pieces[1..] {
                    let out = self.fresh("_scat");
                    self.emit(
                        "str_concat",
                        Some(&out),
                        vec![Operand::Var(concat), Operand::Var(reg.clone())],
                        "str",
                    );
                    concat = out;
                    total += len;
                }
                // `WITH POINTER p`: the overlay offset `p-1` is a RUN-TIME value, so
                // the compile-time slicing below no longer applies — hand off to the
                // shared run-time overlay. The concat length is compile-time-known
                // here, materialised as a `const` so the overlay helper is uniform.
                // The helper returns the overflow flag (out-of-range OR drop).
                if let Some((pidx, ptr_int_digits)) = pointer {
                    let clen = self.fresh("_stclen");
                    self.emit("const", Some(&clen), vec![Operand::Int(total as i64)], "i64");
                    let pname = pointer_name.as_deref().expect("pointer present");
                    self.emit_string_pointer_overlay(
                        &recv, &concat, &clen, width, pname, pidx, ptr_int_digits,
                    )?
                } else {
                    if total >= width {
                        // Truncate at the receiver width; the whole receiver is
                        // overwritten.
                        let start = self.str_index(0);
                        let end = self.str_index(width as i64);
                        self.emit(
                            "str_slice",
                            Some(&recv),
                            vec![Operand::Var(concat), Operand::Var(start), Operand::Var(end)],
                            "str",
                        );
                    } else {
                        // Preserve the receiver's tail `[total, width)`: the head is
                        // the entire concatenation (length exactly `total`), then
                        // re-append the old tail read from the receiver's register.
                        let start = self.str_index(total as i64);
                        let end = self.str_index(width as i64);
                        let tail = self.fresh("_stail");
                        self.emit(
                            "str_slice",
                            Some(&tail),
                            vec![Operand::Var(recv.clone()), Operand::Var(start), Operand::Var(end)],
                            "str",
                        );
                        self.emit(
                            "str_concat",
                            Some(&recv),
                            vec![Operand::Var(concat), Operand::Var(tail)],
                            "str",
                        );
                    }
                    // No pointer: overflow ⇔ concat longer than the receiver
                    // (`total > width`), a COMPILE-TIME-known boolean — `total ==
                    // width` fills exactly, dropping nothing, so it is NOT overflow.
                    let ov = self.fresh("_stof");
                    self.emit("const", Some(&ov), vec![Operand::Int((total > width) as i64)], "i64");
                    ov
                }
            }
            // `DELIMITED BY delim` — each field's prefix is a run-time value.
            Some(d_reg) => {
                let mut concat: Option<String> = None;
                for op in sources {
                    // A non-ASCII string-LITERAL field under an active delimiter is a
                    // later rung (its prefix boundary differs byte-vs-char); guard it
                    // BEFORE lowering. A non-ASCII IDENTIFIER field is the pre-existing
                    // byte-vs-char chip and is not guarded here.
                    if let Operandy::Literal(Src::Str(s)) = read_operand(op)? {
                        if !s.is_ascii() {
                            return Err(CompileError::Unsupported(
                                "STRING with a non-ASCII sending field under DELIMITED BY is a later rung"
                                    .into(),
                            ));
                        }
                    }
                    // Lower the field to a string register (its compile-time length is
                    // irrelevant now — the prefix boundary is found at run time).
                    let (field_reg, _len) = self.string_source(op)?;
                    let prefix = self.emit_prefix_before_delim(&field_reg, d_reg);
                    concat = Some(match concat {
                        None => prefix,
                        Some(acc) => {
                            let out = self.fresh("_scat");
                            self.emit(
                                "str_concat",
                                Some(&out),
                                vec![Operand::Var(acc), Operand::Var(prefix)],
                                "str",
                            );
                            out
                        }
                    });
                }
                // `sources` is non-empty, so `concat` is always `Some` here.
                let concat = concat.expect("at least one sending field");
                // The concatenation's length is a run-time value here.
                let clen = self.fresh("_sclen");
                self.emit("str_len", Some(&clen), vec![Operand::Var(concat.clone())], "i64");
                // `WITH POINTER p`: overlay at the run-time offset `p-1` via the
                // shared helper (byte-identical to the SIZE-branch pointer path),
                // which returns the overflow flag. The no-pointer run-time overlay
                // below (start at 0) is unchanged.
                if let Some((pidx, ptr_int_digits)) = pointer {
                    let pname = pointer_name.as_deref().expect("pointer present");
                    self.emit_string_pointer_overlay(
                        &recv, &concat, &clen, width, pname, pidx, ptr_int_digits,
                    )?
                } else {
                    // Run-time overlay: take = min(clen, W); the receiver becomes
                    // concat[0,take] ++ recv[take,W] (the preserved tail). overflow ⇔
                    // clen > W (some sending chars dropped) — the SAME run-time test.
                    let wconst = self.fresh("_scw");
                    self.emit("const", Some(&wconst), vec![Operand::Int(width as i64)], "i64");
                    let take = self.fresh("_sctk");
                    self.emit("mov", Some(&take), vec![Operand::Var(clen.clone())], "i64");
                    let gt = self.fresh("_scgt");
                    self.emit("cmp_gt", Some(&gt), vec![Operand::Var(clen.clone()), Operand::Var(wconst.clone())], "i64");
                    let noclip = self.fresh("sc_noclip");
                    self.emit("jmp_if_false", None, vec![Operand::Var(gt.clone()), Operand::Var(noclip.clone())], "void");
                    self.emit("mov", Some(&take), vec![Operand::Var(wconst.clone())], "i64");
                    self.emit("label", None, vec![Operand::Var(noclip)], "void");
                    // head = concat[0, take].
                    let z0 = self.str_index(0);
                    let head = self.fresh("_schd");
                    self.emit(
                        "str_slice",
                        Some(&head),
                        vec![Operand::Var(concat), Operand::Var(z0), Operand::Var(take.clone())],
                        "str",
                    );
                    // tail = recv[take, W] — the receiver bytes STRING left untouched.
                    let tail = self.fresh("_sctail");
                    self.emit(
                        "str_slice",
                        Some(&tail),
                        vec![Operand::Var(recv.clone()), Operand::Var(take), Operand::Var(wconst)],
                        "str",
                    );
                    self.emit(
                        "str_concat",
                        Some(&recv),
                        vec![Operand::Var(head), Operand::Var(tail)],
                        "str",
                    );
                    // `gt` (clen > W) IS the overflow flag; reuse it directly.
                    gt
                }
            }
        };

        // # ON OVERFLOW / NOT ON OVERFLOW dispatch
        //
        // The two imperatives are direct `statement` child nodes of `string_stmt`,
        // appearing ONLY after the `ON OVERFLOW` / `NOT ON OVERFLOW` keyword tokens.
        // Split them at the `NOT` keyword exactly as the oracle reader and `emit_if`'s
        // ELSE split do:
        //
        //   STRING … ON OVERFLOW  <A…>   NOT ON OVERFLOW  <B…>
        //                         └ on ┘    ▲NOT flips     └ not_on ┘
        //
        // A nested statement's own `NOT` is buried inside its `statement` node, never
        // a direct token child here, so the split is unambiguous. We then emit the
        // usual `jmp_if_false`/branch/`label` skeleton `emit_if` uses, guarding on the
        // `overflow` register computed above.
        let mut on_stmts: Vec<&GrammarASTNode> = Vec::new();
        let mut not_stmts: Vec<&GrammarASTNode> = Vec::new();
        let mut seen_not = false;
        for child in &verb.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "NOT" && t.effective_type_name() == "KEYWORD" => {
                    seen_not = true;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                    if seen_not {
                        not_stmts.push(n);
                    } else {
                        on_stmts.push(n);
                    }
                }
                _ => {}
            }
        }
        // Nothing to run when both clauses are absent — skip the branch skeleton
        // entirely (a plain STRING lowers exactly as before this rung).
        if on_stmts.is_empty() && not_stmts.is_empty() {
            return Ok(());
        }
        let not_lbl = self.fresh("st_notov");
        let end_lbl = self.fresh("st_ovend");
        self.emit("jmp_if_false", None, vec![Operand::Var(overflow), Operand::Var(not_lbl.clone())], "void");
        for stmt in on_stmts {
            self.emit_statement(stmt)?;
        }
        self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
        self.emit("label", None, vec![Operand::Var(not_lbl)], "void");
        for stmt in not_stmts {
            self.emit_statement(stmt)?;
        }
        self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
        Ok(())
    }

    /// Emit the `STRING … WITH POINTER p` overlay: place `concat` (its length in
    /// register `clen`) into the receiver `recv` (a `size`-wide alphanumeric item)
    /// starting at the 1-based position held by the unsigned-integer pointer item
    /// `pidx`, then write the pointer back. Byte-identical to the oracle's
    /// `exec_string` pointer arm.
    ///
    /// The pointer's VALUE is a RUN-TIME datum, so the offset and the out-of-range
    /// decision are emitted as run-time IIR (which is exactly why the range can't be
    /// checked at compile time — only the pointer's *picture* is):
    ///
    /// ```text
    ///   pv = <pointer register>;  one = 1;  W = size
    ///   if pv < 1  jmp st_end          # pv == 0 (PIC 9 is unsigned) → overflow
    ///   if pv > W  jmp st_end          # start past the receiver end → overflow
    ///   start = pv - 1                 # 0-based overlay offset
    ///   avail = W - start              # room from start to the receiver end (≥ 1)
    ///   cp = min(clen, avail)          # chars actually placed (excess is dropped)
    ///   end = start + cp
    ///   recv = recv[0,start] ++ concat[0,cp] ++ recv[end,W]   # keep the untouched runs
    ///   p := pv + cp                   # 1-based position one past the last char
    /// st_end:                          # out-of-range lands here: recv + p unchanged
    /// ```
    ///
    /// `avail` is `≥ 1` because the guard has already established `1 ≤ pv ≤ W`, so
    /// `start ≤ W-1`. When the content does not all fit (`clen > avail`) the excess
    /// is dropped — ISO's overflow — and `cp = avail`, giving `p := W + 1`. `pv = 1`
    /// (start 0) reproduces the no-pointer overlay exactly, the correctness anchor.
    /// The out-of-range jump skips BOTH the overlay and the write-back, leaving the
    /// receiver and pointer with their prior values, matching the oracle's early
    /// no-movement path.
    ///
    /// Returns the `overflow` i64 register (1 / 0) the caller uses to select the
    /// `ON OVERFLOW` vs `NOT ON OVERFLOW` imperative — `true` when the pointer is out
    /// of range OR the content was dropped (`clen > avail`), matching the oracle.
    #[allow(clippy::too_many_arguments)]
    fn emit_string_pointer_overlay(
        &mut self,
        recv: &str,
        concat: &str,
        clen: &str,
        width: usize,
        pname: &str,
        pidx: usize,
        ptr_int_digits: usize,
    ) -> Result<String, CompileError> {
        let pv = self.items[pidx].reg.clone();
        let one = self.fresh("_stpone");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        let wconst = self.fresh("_stpw");
        self.emit("const", Some(&wconst), vec![Operand::Int(width as i64)], "i64");
        let st_end = self.fresh("st_end");
        // The `overflow` flag drives ON / NOT ON OVERFLOW. We PRE-SEED it to 1 and
        // let the out-of-range guards fall through to `st_end` with it still set
        // (out-of-range IS overflow); the in-range path OVERWRITES it with the drop
        // test `clen > avail` below. This matches the oracle exactly: overflow = true
        // when pv∉[1,W], else overflow = (src.len() > avail).
        let overflow = self.fresh("_stpof");
        self.emit("const", Some(&overflow), vec![Operand::Int(1)], "i64");
        // Out of range: pv < 1 (i.e. pv == 0, since PIC 9 is unsigned) …
        let lt1 = self.fresh("_stplt");
        self.emit(
            "cmp_lt",
            Some(&lt1),
            vec![Operand::Var(pv.clone()), Operand::Var(one.clone())],
            "i64",
        );
        self.emit("jmp_if_true", None, vec![Operand::Var(lt1), Operand::Var(st_end.clone())], "void");
        // … or pv > W (start past the receiver end).
        let gt = self.fresh("_stpgt");
        self.emit(
            "cmp_gt",
            Some(&gt),
            vec![Operand::Var(pv.clone()), Operand::Var(wconst.clone())],
            "i64",
        );
        self.emit("jmp_if_true", None, vec![Operand::Var(gt), Operand::Var(st_end.clone())], "void");
        // start = pv - 1 (0-based overlay offset).
        let start = self.fresh("_stps");
        self.emit(
            "sub",
            Some(&start),
            vec![Operand::Var(pv.clone()), Operand::Var(one)],
            "i64",
        );
        // avail = W - start (room from start to the receiver end; ≥ 1 here).
        let avail = self.fresh("_stpav");
        self.emit(
            "sub",
            Some(&avail),
            vec![Operand::Var(wconst.clone()), Operand::Var(start.clone())],
            "i64",
        );
        // cp = min(clen, avail): what actually fits (the rest is dropped).
        let cp = self.fresh("_stpcp");
        self.emit("mov", Some(&cp), vec![Operand::Var(clen.to_string())], "i64");
        let over = self.fresh("_stpov");
        self.emit(
            "cmp_gt",
            Some(&over),
            vec![Operand::Var(clen.to_string()), Operand::Var(avail.clone())],
            "i64",
        );
        // In range: overflow ⇔ a drop occurred (`clen > avail`). Overwrite the
        // pre-seeded 1 with this exact boolean — the same test the oracle uses.
        self.emit("mov", Some(&overflow), vec![Operand::Var(over.clone())], "i64");
        let keep = self.fresh("st_keep");
        self.emit("jmp_if_false", None, vec![Operand::Var(over), Operand::Var(keep.clone())], "void");
        self.emit("mov", Some(&cp), vec![Operand::Var(avail)], "i64");
        self.emit("label", None, vec![Operand::Var(keep)], "void");
        // end = start + cp.
        let end = self.fresh("_stpe");
        self.emit(
            "add",
            Some(&end),
            vec![Operand::Var(start.clone()), Operand::Var(cp.clone())],
            "i64",
        );
        // recv = recv[0,start] ++ concat[0,cp] ++ recv[end,W]: overwrite only the
        // filled run, keeping the receiver's head (before start) and tail (from end).
        let z0 = self.str_index(0);
        let headpre = self.fresh("_stph");
        self.emit(
            "str_slice",
            Some(&headpre),
            vec![Operand::Var(recv.to_string()), Operand::Var(z0.clone()), Operand::Var(start)],
            "str",
        );
        let mid = self.fresh("_stpm");
        self.emit(
            "str_slice",
            Some(&mid),
            vec![Operand::Var(concat.to_string()), Operand::Var(z0), Operand::Var(cp.clone())],
            "str",
        );
        let tail = self.fresh("_stpt");
        self.emit(
            "str_slice",
            Some(&tail),
            vec![Operand::Var(recv.to_string()), Operand::Var(end), Operand::Var(wconst)],
            "str",
        );
        let hm = self.fresh("_stphm");
        self.emit("str_concat", Some(&hm), vec![Operand::Var(headpre), Operand::Var(mid)], "str");
        self.emit("str_concat", Some(recv), vec![Operand::Var(hm), Operand::Var(tail)], "str");
        // Write the pointer back to `pv + cp` (1-based, one past the last char
        // stored), reshaped into its `PIC 9(n)` picture through the same numeric path
        // ADD/UNSTRING use — byte-identical to the oracle's `store_result`.
        let resume = self.fresh("_stpres");
        self.emit("add", Some(&resume), vec![Operand::Var(pv), Operand::Var(cp)], "i64");
        self.store_scaled(pname, &resume, 0, ptr_int_digits + 1, false)?;
        // The out-of-range guard lands here, skipping the overlay AND the write-back
        // (with `overflow` still 1 from its pre-seed).
        self.emit("label", None, vec![Operand::Var(st_end)], "void");
        Ok(overflow)
    }

    /// Emit the run-time scan that returns a fresh `str` register holding the
    /// prefix of `field_reg` up to (but not including) the first byte equal to the
    /// delimiter code `d_reg` — the per-field contribution of `STRING … DELIMITED
    /// BY delim`. A field with no delimiter yields its whole image; a field
    /// starting with the delimiter yields the empty string. The loop is the same
    /// shape UNSTRING's field scan uses:
    ///
    /// ```text
    ///   flen = str_len(F);  j = 0
    /// top:  if j >= flen        jmp done      # ran off the end, no delimiter
    ///       if F[j] == d_reg    jmp done      # delimiter found at j
    ///       j = j + 1;  jmp top
    /// done:
    ///   prefix = str_slice(F, 0, j)
    /// ```
    fn emit_prefix_before_delim(&mut self, field_reg: &str, d_reg: &str) -> String {
        let flen = self.fresh("_spfl");
        self.emit("str_len", Some(&flen), vec![Operand::Var(field_reg.to_string())], "i64");
        let j = self.fresh("_spj");
        self.emit("const", Some(&j), vec![Operand::Int(0)], "i64");
        let top = self.fresh("sp_top");
        let done = self.fresh("sp_done");
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        let ge = self.fresh("_spge");
        self.emit("cmp_ge", Some(&ge), vec![Operand::Var(j.clone()), Operand::Var(flen)], "i64");
        self.emit("jmp_if_true", None, vec![Operand::Var(ge), Operand::Var(done.clone())], "void");
        let c = self.fresh("_spc");
        self.emit("str_index", Some(&c), vec![Operand::Var(field_reg.to_string()), Operand::Var(j.clone())], "i64");
        let eq = self.fresh("_speq");
        self.emit("cmp_eq", Some(&eq), vec![Operand::Var(c), Operand::Var(d_reg.to_string())], "i64");
        self.emit("jmp_if_true", None, vec![Operand::Var(eq), Operand::Var(done.clone())], "void");
        let one = self.fresh("_sp1");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&j), vec![Operand::Var(j.clone()), Operand::Var(one)], "i64");
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(done)], "void");
        let z0 = self.str_index(0);
        let prefix = self.fresh("_sppfx");
        self.emit(
            "str_slice",
            Some(&prefix),
            vec![Operand::Var(field_reg.to_string()), Operand::Var(z0), Operand::Var(j)],
            "str",
        );
        prefix
    }

    /// A `STRING` sending field lowered to a `(register, length)` pair. An
    /// alphanumeric item contributes its whole fixed-width slot; a string literal
    /// its text; a numeric literal its lexed source digits verbatim (the same text
    /// the oracle concatenates). A figurative constant SPACE/ZERO lowers to its
    /// single-character image — SPACE→`" "`, ZERO→`"0"` — a 1-char `str_const`
    /// reducing to the string-literal path. A numeric item as a sending field is a
    /// later rung.
    ///
    /// A reference-modification sending field — `WS(start:len)` — lowers to the
    /// slice register the shared [`Self::ref_mod_slice`] emits (the same `str_slice`
    /// DISPLAY / comparison / MOVE-source take), paired with its length. Only
    /// **constant (literal) indices** are accepted: they yield a
    /// [`SliceLen::Const`], a compile-time-known length the `(reg, usize)` STRING
    /// image contract can carry. A **computed (data-name) index** would yield a
    /// [`SliceLen::Runtime`] length known only at run time, which this contract
    /// cannot express, so it is rejected up front — before any slice code is emitted
    /// — keeping the reject co-total with the oracle's identical refusal.
    fn string_source(&mut self, op: &GrammarASTNode) -> Result<(String, usize), CompileError> {
        match read_operand(op)? {
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Char { .. } => Ok((self.items[idx].reg.clone(), self.items[idx].width())),
                    ItemKind::Numeric { .. } => Err(CompileError::Unsupported(
                        "a numeric item as a STRING sending field is a later rung".into(),
                    )),
                }
            }
            Operandy::Literal(Src::Str(s)) => {
                let len = s.chars().count();
                let reg = self.fresh("_slit");
                self.emit("str_const", Some(&reg), vec![Operand::Str(s)], "str");
                Ok((reg, len))
            }
            Operandy::Literal(Src::Num(s)) => {
                let len = s.chars().count();
                let reg = self.fresh("_snum");
                self.emit("str_const", Some(&reg), vec![Operand::Str(s)], "str");
                Ok((reg, len))
            }
            // SPACE→" ", ZERO→"0" reduce to the single-char string-literal
            // sending-field path (mirrors the oracle's `string_source_chars`);
            // both images are ASCII so the non-ASCII guard passes unchanged.
            Operandy::Literal(Src::Space) => {
                let reg = self.fresh("_slit");
                self.emit("str_const", Some(&reg), vec![Operand::Str(" ".into())], "str");
                Ok((reg, 1))
            }
            Operandy::Literal(Src::Zero) => {
                let reg = self.fresh("_slit");
                self.emit("str_const", Some(&reg), vec![Operand::Str("0".into())], "str");
                Ok((reg, 1))
            }
            Operandy::RefMod { base, start, len } => {
                // Reject a computed (data-name) index BEFORE emitting any slice code
                // (avoid dead instructions), keeping the (reg, usize) compile-time-
                // length contract and staying co-total with the oracle's identical
                // reject.
                let const_ix = matches!(start, RefIndex::Lit(_))
                    && len.as_ref().is_none_or(|l| matches!(l, RefIndex::Lit(_)));
                if !const_ix {
                    return Err(CompileError::Unsupported(
                        "a computed reference modification as a STRING sending field is a later rung"
                            .into(),
                    ));
                }
                let (reg, slice_len) = self.ref_mod_slice(&base, &start, &len)?;
                match slice_len {
                    SliceLen::Const(n) => Ok((reg, n)),
                    // Unreachable given the const_ix guard, but keep total:
                    SliceLen::Runtime { .. } => Err(CompileError::Unsupported(
                        "a computed reference modification as a STRING sending field is a later rung"
                            .into(),
                    )),
                }
            }
        }
    }

    /// A fresh `i64` register holding the constant `k` — a compile-time-known
    /// `str_slice` bound.
    fn str_index(&mut self, k: i64) -> String {
        let reg = self.fresh("_sidx");
        self.emit("const", Some(&reg), vec![Operand::Int(k)], "i64");
        reg
    }

    /// `UNSTRING source DELIMITED BY delim INTO r1 [r2 …]` — the inverse of
    /// STRING: scan the alphanumeric `source` left-to-right and split it on the
    /// SINGLE-character `delim` into successive receivers.
    ///
    /// Where STRING's boundaries are all compile-time-known (a fixed
    /// `str_slice`/`str_concat`), UNSTRING's are DATA-dependent — the delimiter
    /// falls wherever the run-time bytes put it — so we emit a genuine scan LOOP.
    /// The source register `S`, its length `len = str_len(S)`, and a cursor `p`
    /// (i64, init 0) drive the whole statement; the delimiter is reduced to a
    /// single byte code `D` (a `const` for a 1-char literal, or `str_index(item,0)`
    /// for a `PIC X(1)` item). Each receiver `r_i` (there are a compile-time-known
    /// `n` of them) unrolls to a block:
    ///
    /// ```text
    ///   if p <= len  (else jump to us_skip — leave r_i UNCHANGED):
    ///     j = p
    ///   us_top:  if j >= len   jmp us_found          # end of source
    ///            if S[j] == D   jmp us_found          # delimiter here
    ///            j = j + 1;  jmp us_top
    ///   us_found:
    ///     piece = str_slice(S, p, j)                  # the field [p, j)
    ///     take  = min(str_len(piece), W)              # W = r_i's width
    ///     r_i   = str_slice(piece,0,take) ++ spaces(W - take)   # MOVE semantics
    ///     p = j + 1                                   # step past the delimiter
    ///   us_skip:
    /// ```
    ///
    /// Because `p` never moves when a receiver is skipped, once the source is
    /// exhausted (`p > len`, a field having run off the end WITHOUT a trailing
    /// delimiter) this receiver AND every later one is left unchanged — the
    /// per-receiver guard alone gives the oracle's "remaining receivers keep their
    /// prior VALUE" rule. `p == len` (a trailing delimiter) still passes the guard
    /// and yields one final EMPTY field (all spaces). The
    /// `str_slice(piece,0,take) ++ spaces(W-take)` reshape is exactly the oracle's
    /// alphanumeric `move_into` (left-justify, space-pad, truncate), so a compiled
    /// program matches the `cobol-runtime` oracle byte-for-byte. `WITH POINTER`,
    /// `ON OVERFLOW`, a multi-character delimiter, and a numeric/group source or
    /// receiver are later rungs (clean `Unsupported`).
    fn emit_unstring(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        // `WITH POINTER` and the two OVERFLOW imperatives are now MODELLED (see the
        // handling below), so nothing is rejected up front — the DIRECT sibling of
        // emit_string's ON OVERFLOW dispatch.
        let toks = child_tokens(verb);
        // The two direct `operand` children are the source and the delimiter, in
        // order (a reference-modification suffix nests under an operand, so it is
        // never a third top-level operand).
        let ops = child_nodes(verb, "operand");
        let (source_node, delim_node) = match ops.as_slice() {
            [s, d] => (*s, *d),
            _ => {
                return Err(CompileError::Malformed(
                    "UNSTRING needs a source and a DELIMITED BY delimiter".into(),
                ))
            }
        };
        // The source supplies the field text from ONE of two providers, and the
        // whole scan below reads it purely as a string register (`str_len` /
        // `str_index` / `str_slice`) — so only how we obtain `s_reg` differs:
        //   * an alphanumeric data-name → the item's own char register; or
        //   * a STRING literal → a fresh `str_const` register holding its bytes
        //     (a `str_const` register behaves identically to an item's char
        //     register under str_len/str_index/str_slice, exactly as the
        //     `spaces_const` register does further down this same routine).
        // A figurative SPACE/ZERO literal source is now accepted as its single-char
        // ASCII image (SPACE->" ", ZERO->"0"), reducing to the string-literal source
        // scan; a NUMERIC literal source stays a later rung, matching the oracle's
        // read-time rejects. A REFERENCE-MODIFIED source `base(start:len)`
        // is supported by routing the sliced characters through the SHARED
        // `ref_mod_slice` helper — the identical slice DISPLAY / comparisons emit,
        // so the source register is byte-for-byte what the oracle's `refmod_string`
        // produces. Only how we obtain `s_reg` changes; the scan below is untouched.
        let s_reg = match read_operand(source_node)? {
            Operandy::Name(source_name) => {
                let sidx = self.item_index(&source_name)?;
                match &self.items[sidx].kind {
                    ItemKind::Char { .. } => {}
                    ItemKind::Numeric { .. } => {
                        return Err(CompileError::Unsupported(
                            "UNSTRING of a numeric source is a later rung".into(),
                        ))
                    }
                }
                self.items[sidx].reg.clone()
            }
            Operandy::Literal(Src::Str(s)) => {
                // The downstream scan reads `s_reg` with BYTE-based IIR string ops
                // (str_len/str_index/str_slice), whereas the oracle scans a literal
                // by CHARACTER — the two agree only for ASCII (one byte per char).
                // A non-ASCII literal source is a clean later rung on BOTH engines,
                // keeping the accept/reject sets co-total.
                if !s.is_ascii() {
                    return Err(CompileError::Unsupported(
                        "UNSTRING of a non-ASCII literal source is a later rung".into(),
                    ));
                }
                let reg = self.fresh("_ussrc");
                self.emit("str_const", Some(&reg), vec![Operand::Str(s)], "str");
                reg
            }
            Operandy::Literal(Src::Num(_)) => {
                return Err(CompileError::Unsupported(
                    "UNSTRING of a numeric-literal source is a later rung".into(),
                ))
            }
            // A figurative SPACE/ZERO source: SPACE->" ", ZERO->"0" reduce to the
            // single-char ASCII literal-source scan (mirrors the oracle's read-time
            // map). Both images are known-ASCII, so no ASCII check is needed.
            Operandy::Literal(Src::Space) => {
                let reg = self.fresh("_ussrc");
                self.emit("str_const", Some(&reg), vec![Operand::Str(" ".into())], "str");
                reg
            }
            Operandy::Literal(Src::Zero) => {
                let reg = self.fresh("_ussrc");
                self.emit("str_const", Some(&reg), vec![Operand::Str("0".into())], "str");
                reg
            }
            Operandy::RefMod { base, start, len } => {
                // The source characters are the ref-mod slice `base(start:len)`.
                // `ref_mod_slice` emits the SAME `str_slice` DISPLAY/comparison
                // use (constant-folded for literal indices, register-computed for a
                // data-name index) and enforces the SAME numeric-base and
                // out-of-range rejects — so the slice register is byte-identical to
                // the oracle's `refmod_string`, and everything downstream reads it
                // exactly like a plain item's char register. We only need the
                // register; the length metadata (used by comparisons) is unused
                // here because the scan measures the source with `str_len`.
                let (reg, _len) = self.ref_mod_slice(&base, &start, &len)?;
                reg
            }
        };

        // The delimiter reduced to a single byte code register.
        let d_reg = self.single_delim_code(delim_node, "UNSTRING")?;

        // Split the NAME tokens at the optional `POINTER` keyword — the grammar is
        // flat (`INTO NAME { NAME } [ WITH POINTER NAME ]`), so every receiver NAME
        // precedes `POINTER` and the pointer NAME is the first NAME after it. (This
        // mirrors the oracle reader; taking "the last NAME" blindly would misread a
        // single-receiver `INTO r WITH POINTER p` as two receivers.)
        let ptr_pos = toks.iter().position(|(k, v)| k == "KEYWORD" && v == "POINTER");
        let pointer_name: Option<String> = ptr_pos.and_then(|pp| {
            toks[pp + 1..].iter().find(|(k, _)| k == "NAME").map(|(_, v)| v.clone())
        });
        let targets: Vec<String> = toks
            .iter()
            .enumerate()
            .filter(|(i, (k, _))| k == "NAME" && ptr_pos.is_none_or(|pp| *i < pp))
            .map(|(_, (_, v))| v.clone())
            .collect();
        if targets.is_empty() {
            return Err(CompileError::Malformed("UNSTRING without an INTO receiver".into()));
        }

        // Validate the pointer item's picture at BUILD time — it must be an unsigned
        // integer `PIC 9(n)` (n ≤ 18 so the value fits the `i64` we store it in),
        // the same class INSPECT's counter demands. A signed, fractional, non-
        // numeric, or group pointer is a clean later rung, rejected here with the
        // SAME message the oracle raises at exec time so the accept/reject sets stay
        // co-total. We capture `(index, int_digits)` for the run-time read/write-back
        // below; the range check on the pointer's VALUE cannot be done here (it is a
        // run-time datum), so it is emitted as a guard, exactly like the oracle.
        let pointer: Option<(usize, usize)> = match &pointer_name {
            Some(pname) => {
                let pidx = self.item_index(pname)?;
                let ptr_int_digits = match &self.items[pidx].kind {
                    ItemKind::Numeric { signed: true, .. } => {
                        return Err(CompileError::Unsupported(format!(
                            "UNSTRING … WITH POINTER: a signed pointer {pname} is a later rung"
                        )))
                    }
                    ItemKind::Numeric { dec_digits, .. } if *dec_digits != 0 => {
                        return Err(CompileError::Unsupported(format!(
                            "UNSTRING … WITH POINTER: a non-integer pointer {pname} is a later rung"
                        )))
                    }
                    ItemKind::Numeric { int_digits, .. } if *int_digits > 18 => {
                        return Err(CompileError::Unsupported(format!(
                            "UNSTRING … WITH POINTER: a pointer {pname} wider than 18 digits is a later rung"
                        )))
                    }
                    ItemKind::Numeric { int_digits, .. } => *int_digits,
                    ItemKind::Char { .. } => {
                        return Err(CompileError::Unsupported(format!(
                            "UNSTRING … WITH POINTER: a non-numeric pointer {pname} is a later rung"
                        )))
                    }
                };
                Some((pidx, ptr_int_digits))
            }
            None => None,
        };
        let mut recvs: Vec<(usize, usize)> = Vec::with_capacity(targets.len());
        for t in &targets {
            let idx = self.item_index(t)?;
            let width = match &self.items[idx].kind {
                ItemKind::Char { .. } => self.items[idx].width(),
                ItemKind::Numeric { .. } => {
                    return Err(CompileError::Unsupported(
                        "UNSTRING into a numeric receiver is a later rung".into(),
                    ))
                }
            };
            recvs.push((idx, width));
        }

        // # ON OVERFLOW / NOT ON OVERFLOW split
        //
        // The two imperatives are direct `statement` child nodes of `unstring_stmt`,
        // appearing ONLY after the `ON OVERFLOW` / `NOT ON OVERFLOW` keyword tokens.
        // Split them at the `NOT` keyword exactly as the oracle reader and emit_if's
        // ELSE split do — a nested statement's own `NOT` is buried inside its
        // `statement` node, never a direct token child here, so the split is
        // unambiguous. We collect the node refs NOW (no emission) so we can decide
        // whether to plumb the `overflow` flag at all: a plain UNSTRING with neither
        // clause lowers EXACTLY as before this rung.
        //
        //   UNSTRING … ON OVERFLOW  <A…>   NOT ON OVERFLOW  <B…>
        //                           └ on ┘    ▲NOT flips     └ not_on ┘
        let mut on_stmts: Vec<&GrammarASTNode> = Vec::new();
        let mut not_stmts: Vec<&GrammarASTNode> = Vec::new();
        let mut seen_not = false;
        for child in &verb.children {
            match child {
                ASTNodeOrToken::Token(t)
                    if t.value == "NOT" && t.effective_type_name() == "KEYWORD" =>
                {
                    seen_not = true;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                    if seen_not {
                        not_stmts.push(n);
                    } else {
                        on_stmts.push(n);
                    }
                }
                _ => {}
            }
        }
        let has_clauses = !on_stmts.is_empty() || !not_stmts.is_empty();

        // len = str_len(S).
        let len = self.fresh("_uslen");
        self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.clone())], "i64");

        // The initial 0-based scan cursor `p`. Without a pointer it is 0 (today's
        // behaviour). With `WITH POINTER q`, `q` holds a 1-BASED start position, so
        // `p = q_value − 1`; but first the out-of-range guard: if `q_value == 0` or
        // `q_value > len` (ISO's overflow) we must leave every receiver AND the
        // pointer UNCHANGED. We emit a jump to `us_end` — placed AFTER the receiver
        // loop and the write-back — so the whole operation is skipped, matching the
        // oracle's early `return Ok(())`. The pointer's VALUE cannot be range-checked
        // at build time (it is a run-time datum), which is exactly why this is a
        // run-time guard rather than a compile-time reject.
        let p = self.fresh("_usp");
        let us_end = self.fresh("us_end");
        // The `overflow` flag drives ON / NOT ON OVERFLOW — computed ONLY when a
        // clause is present so a plain UNSTRING lowers byte-identically to before.
        // We PRE-SEED it to 1 and let the out-of-range pointer guards fall through to
        // `us_end` with it still set (out-of-range IS overflow, mirroring the
        // oracle); the in-range path OVERWRITES it after the scan with `p <= len`
        // (source not yet exhausted). This is the SAME pre-seed trick STRING's
        // emit_string_pointer_overlay uses.
        let overflow = if has_clauses {
            let ov = self.fresh("_usof");
            self.emit("const", Some(&ov), vec![Operand::Int(1)], "i64");
            Some(ov)
        } else {
            None
        };
        match &pointer {
            Some((pidx, _)) => {
                let pv = self.items[*pidx].reg.clone();
                let one = self.fresh("_uspone");
                self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
                // out of range if pv < 1 (i.e. pv == 0, since PIC 9 is unsigned) …
                let lt1 = self.fresh("_usplt");
                self.emit("cmp_lt", Some(&lt1), vec![Operand::Var(pv.clone()), Operand::Var(one.clone())], "i64");
                self.emit("jmp_if_true", None, vec![Operand::Var(lt1), Operand::Var(us_end.clone())], "void");
                // … or pv > len (start past the source).
                let gtlen = self.fresh("_uspgt");
                self.emit("cmp_gt", Some(&gtlen), vec![Operand::Var(pv.clone()), Operand::Var(len.clone())], "i64");
                self.emit("jmp_if_true", None, vec![Operand::Var(gtlen), Operand::Var(us_end.clone())], "void");
                // In range: p = pv − 1.
                self.emit("sub", Some(&p), vec![Operand::Var(pv), Operand::Var(one)], "i64");
            }
            None => {
                self.emit("const", Some(&p), vec![Operand::Int(0)], "i64");
            }
        }

        for (idx, width) in recvs {
            let recv_reg = self.items[idx].reg.clone();
            let skip = self.fresh("us_skip");
            // Guard: process this receiver only while `p <= len`; otherwise jump
            // past its block, leaving the receiver register (its prior VALUE)
            // untouched.
            let le = self.fresh("_usle");
            self.emit("cmp_le", Some(&le), vec![Operand::Var(p.clone()), Operand::Var(len.clone())], "i64");
            self.emit("jmp_if_false", None, vec![Operand::Var(le), Operand::Var(skip.clone())], "void");

            // Scan for the next delimiter (or end-of-source): j runs from p.
            let j = self.fresh("_usj");
            self.emit("mov", Some(&j), vec![Operand::Var(p.clone())], "i64");
            let top = self.fresh("us_top");
            let found = self.fresh("us_found");
            self.emit("label", None, vec![Operand::Var(top.clone())], "void");
            let ge = self.fresh("_usge");
            self.emit("cmp_ge", Some(&ge), vec![Operand::Var(j.clone()), Operand::Var(len.clone())], "i64");
            self.emit("jmp_if_true", None, vec![Operand::Var(ge), Operand::Var(found.clone())], "void");
            let c = self.fresh("_usc");
            self.emit("str_index", Some(&c), vec![Operand::Var(s_reg.clone()), Operand::Var(j.clone())], "i64");
            let eq = self.fresh("_useq");
            self.emit("cmp_eq", Some(&eq), vec![Operand::Var(c), Operand::Var(d_reg.clone())], "i64");
            self.emit("jmp_if_true", None, vec![Operand::Var(eq), Operand::Var(found.clone())], "void");
            let one = self.fresh("_us1");
            self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
            self.emit("add", Some(&j), vec![Operand::Var(j.clone()), Operand::Var(one)], "i64");
            self.emit("jmp", None, vec![Operand::Var(top)], "void");
            self.emit("label", None, vec![Operand::Var(found)], "void");

            // piece = S[p, j) — the field up to the delimiter (or end).
            let piece = self.fresh("_uspc");
            self.emit(
                "str_slice",
                Some(&piece),
                vec![Operand::Var(s_reg.clone()), Operand::Var(p.clone()), Operand::Var(j.clone())],
                "str",
            );
            // take = min(str_len(piece), W).
            let plen = self.fresh("_uspl");
            self.emit("str_len", Some(&plen), vec![Operand::Var(piece.clone())], "i64");
            let wconst = self.fresh("_usw");
            self.emit("const", Some(&wconst), vec![Operand::Int(width as i64)], "i64");
            let take = self.fresh("_ustk");
            self.emit("mov", Some(&take), vec![Operand::Var(plen.clone())], "i64");
            let gt = self.fresh("_usgt");
            self.emit("cmp_gt", Some(&gt), vec![Operand::Var(plen), Operand::Var(wconst.clone())], "i64");
            let noclip = self.fresh("us_noclip");
            self.emit("jmp_if_false", None, vec![Operand::Var(gt), Operand::Var(noclip.clone())], "void");
            self.emit("mov", Some(&take), vec![Operand::Var(wconst.clone())], "i64");
            self.emit("label", None, vec![Operand::Var(noclip)], "void");
            // head = piece[0, take)  (left-justified content, truncated at W).
            let z0 = self.str_index(0);
            let head = self.fresh("_ushd");
            self.emit(
                "str_slice",
                Some(&head),
                vec![Operand::Var(piece), Operand::Var(z0), Operand::Var(take.clone())],
                "str",
            );
            // pad = spaces(W)[0, W - take)  — the right-padding to the full width.
            let padlen = self.fresh("_uspad");
            self.emit("sub", Some(&padlen), vec![Operand::Var(wconst), Operand::Var(take)], "i64");
            let spaces = self.spaces_const(width);
            let z0b = self.str_index(0);
            let pad = self.fresh("_uspd");
            self.emit(
                "str_slice",
                Some(&pad),
                vec![Operand::Var(spaces), Operand::Var(z0b), Operand::Var(padlen)],
                "str",
            );
            // r_i = head ++ pad  (exactly the oracle's alphanumeric move_into).
            self.emit(
                "str_concat",
                Some(&recv_reg),
                vec![Operand::Var(head), Operand::Var(pad)],
                "str",
            );
            // Advance the cursor past the delimiter: p = j + 1.
            let one2 = self.fresh("_us1b");
            self.emit("const", Some(&one2), vec![Operand::Int(1)], "i64");
            self.emit("add", Some(&p), vec![Operand::Var(j), Operand::Var(one2)], "i64");
            self.emit("label", None, vec![Operand::Var(skip)], "void");
        }

        // In range (the out-of-range pointer guard skipped straight to `us_end` with
        // `overflow` still pre-seeded to 1): the source is exhausted iff the final
        // cursor ran past its end, so overflow ⇔ `p <= len`. This is the IDENTICAL
        // comparison the oracle applies (`overflow = p <= src.len()`), so the
        // accept/skip decision is byte-identical. Emitted BEFORE the `us_end` label
        // so the out-of-range jump bypasses it, keeping overflow = 1 there.
        if let Some(ov) = &overflow {
            let le_ov = self.fresh("_usofle");
            self.emit("cmp_le", Some(&le_ov), vec![Operand::Var(p.clone()), Operand::Var(len.clone())], "i64");
            self.emit("mov", Some(ov), vec![Operand::Var(le_ov)], "i64");
        }

        // Write the pointer back to the 1-based resume position: `min(p, len) + 1`.
        // `p` is the scan's final 0-based cursor, sitting one past the terminating
        // delimiter; for a field that ran to end-of-source that step is a phantom
        // one past the end, so clamp to `len` (removing the phantom) before restoring
        // 1-basing with `+ 1`. This is byte-identical to the oracle's
        // `min(p, src.len()) + 1`, stored through the same numeric path so it reshapes
        // into the pointer's `PIC 9(n)` picture the same way.
        if let Some((_, ptr_int_digits)) = pointer {
            // clamped = min(p, len).
            let clamped = self.fresh("_uspc2");
            self.emit("mov", Some(&clamped), vec![Operand::Var(p.clone())], "i64");
            let over = self.fresh("_uspov");
            self.emit("cmp_gt", Some(&over), vec![Operand::Var(p.clone()), Operand::Var(len.clone())], "i64");
            let keep = self.fresh("us_keep");
            self.emit("jmp_if_false", None, vec![Operand::Var(over), Operand::Var(keep.clone())], "void");
            self.emit("mov", Some(&clamped), vec![Operand::Var(len.clone())], "i64");
            self.emit("label", None, vec![Operand::Var(keep)], "void");
            // resume = clamped + 1, reshaped into the pointer's picture.
            let one = self.fresh("_uspr1");
            self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
            let resume = self.fresh("_uspres");
            self.emit("add", Some(&resume), vec![Operand::Var(clamped), Operand::Var(one)], "i64");
            let pname = pointer_name.as_deref().expect("pointer present");
            self.store_scaled(pname, &resume, 0, ptr_int_digits + 1, false)?;
        }

        // The out-of-range guard (only emitted WITH a pointer) lands here, skipping
        // every move and the write-back so the receivers and pointer keep their prior
        // values — and, with a clause present, arriving with `overflow` still 1.
        // Emitted only when a pointer exists, so the no-pointer lowering is
        // byte-identical to before (no dangling label).
        if pointer_name.is_some() {
            self.emit("label", None, vec![Operand::Var(us_end)], "void");
        }

        // # ON OVERFLOW / NOT ON OVERFLOW dispatch
        //
        // With the `overflow` register settled (see the split + pre-seed above), emit
        // the usual `jmp_if_false`/branch/`label` skeleton emit_if uses, guarding on
        // it. When both clauses are absent there is nothing to run — `overflow` is
        // `None` and the whole skeleton is skipped, so a plain UNSTRING lowers exactly
        // as before this rung.
        if let Some(ov) = overflow {
            let not_lbl = self.fresh("us_notov");
            let end_lbl = self.fresh("us_ovend");
            self.emit("jmp_if_false", None, vec![Operand::Var(ov), Operand::Var(not_lbl.clone())], "void");
            for stmt in on_stmts {
                self.emit_statement(stmt)?;
            }
            self.emit("jmp", None, vec![Operand::Var(end_lbl.clone())], "void");
            self.emit("label", None, vec![Operand::Var(not_lbl)], "void");
            for stmt in not_stmts {
                self.emit_statement(stmt)?;
            }
            self.emit("label", None, vec![Operand::Var(end_lbl)], "void");
        }
        Ok(())
    }

    /// A single delimiter byte reduced to a fresh i64 register: a `const` of the
    /// byte for a 1-character string literal, a `const` of the single ASCII byte
    /// for a figurative constant SPACE (0x20) / ZERO (0x30), the single byte of a
    /// **constant reference-modified** slice of length 1 (`D(2:1)` → `str_index` of
    /// its one byte, joining the literal path), or `str_index(item, 0)` for a `PIC
    /// X(1)` item. A multi-character delimiter, a numeric delimiter, a **computed**
    /// (data-name index) reference-modified delimiter, and a numeric/wider delimiter
    /// item are later rungs. (COBOL source is ASCII, so a "1-character" literal is a
    /// single byte; the scan loop compares the source's bytes against this code.)
    ///
    /// The constant-refmod slice is materialised by the shared [`Self::ref_mod_slice`]
    /// (carrying its numeric-base reject and its compile-time out-of-range trap); its
    /// `SliceLen::Const`/`Runtime` split is co-total with the oracle's `const_ix`
    /// predicate, so a form one engine calls constant the other does too.
    ///
    /// Shared by `UNSTRING … DELIMITED BY delim` and `INSPECT … FOR ALL delim`
    /// (which both scan for a single byte); `verb` names the caller so the
    /// later-rung message reads naturally.
    fn single_delim_code(
        &mut self,
        op: &GrammarASTNode,
        verb: &str,
    ) -> Result<String, CompileError> {
        match read_operand(op)? {
            Operandy::Literal(Src::Str(s)) => {
                let bytes = s.as_bytes();
                if bytes.len() != 1 {
                    return Err(CompileError::Unsupported(format!(
                        "{verb} with a multi-character delimiter is a later rung"
                    )));
                }
                let reg = self.fresh("_usd");
                self.emit("const", Some(&reg), vec![Operand::Int(bytes[0] as i64)], "i64");
                Ok(reg)
            }
            Operandy::Literal(Src::Num(_)) => Err(CompileError::Unsupported(format!(
                "{verb} with a numeric-literal delimiter is a later rung"
            ))),
            // SPACE→" " (0x20) / ZERO→"0" (0x30): a figurative constant reduces to
            // its single ASCII byte, exactly like the 1-byte string-literal arm above.
            Operandy::Literal(Src::Space) => {
                let reg = self.fresh("_usd");
                self.emit("const", Some(&reg), vec![Operand::Int(b' ' as i64)], "i64");
                Ok(reg)
            }
            Operandy::Literal(Src::Zero) => {
                let reg = self.fresh("_usd");
                self.emit("const", Some(&reg), vec![Operand::Int(b'0' as i64)], "i64");
                Ok(reg)
            }
            Operandy::RefMod { base, start, len } => {
                // `ref_mod_slice` materialises the slice into `reg` and reports its
                // length. `SliceLen::Const(1)` is the CONSTANT single-char refmod this
                // rung supports: its slice reg is a 1-char `str`, so `str_index(reg, 0)`
                // yields the same scan byte a `PIC X(1)` item does. A `Const(_ != 1)` is
                // a multi-character delimiter; a `SliceLen::Runtime` (any data-name index)
                // is a computed refmod — a run-time length the compile-time contract
                // cannot carry. Both are later rungs, rejected co-total with the oracle.
                let (reg, slice_len) = self.ref_mod_slice(&base, &start, &len)?;
                match slice_len {
                    SliceLen::Const(1) => {
                        let zero = self.str_index(0);
                        let out = self.fresh("_usdc");
                        self.emit(
                            "str_index",
                            Some(&out),
                            vec![Operand::Var(reg), Operand::Var(zero)],
                            "i64",
                        );
                        Ok(out)
                    }
                    SliceLen::Const(_) => Err(CompileError::Unsupported(format!(
                        "{verb} with a multi-character delimiter is a later rung"
                    ))),
                    SliceLen::Runtime { .. } => Err(CompileError::Unsupported(format!(
                        "{verb} with a computed reference-modified delimiter is a later rung"
                    ))),
                }
            }
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Char { .. } => {
                        if self.items[idx].width() != 1 {
                            return Err(CompileError::Unsupported(format!(
                                "{verb} with a delimiter item wider than one character is a later rung"
                            )));
                        }
                        let reg = self.items[idx].reg.clone();
                        let zero = self.str_index(0);
                        let out = self.fresh("_usdc");
                        self.emit(
                            "str_index",
                            Some(&out),
                            vec![Operand::Var(reg), Operand::Var(zero)],
                            "i64",
                        );
                        Ok(out)
                    }
                    ItemKind::Numeric { .. } => Err(CompileError::Unsupported(format!(
                        "{verb} with a numeric delimiter item is a later rung"
                    ))),
                }
            }
        }
    }

    /// `INSPECT source TALLYING counter FOR ALL delim` — count the (non-
    /// overlapping, left-to-right) occurrences of the SINGLE-character `delim` in
    /// the alphanumeric `source` and **ADD** that count to the integer `counter`.
    /// INSPECT *adds* to the counter; it does NOT zero it first, so the net effect
    /// is `counter := counter + occurrences`.
    ///
    /// Like UNSTRING this is a data-dependent scan, so we emit a genuine LOOP over
    /// the source: its register `S`, its length `len = str_len(S)`, a cursor `j`
    /// (i64, init 0), and a count accumulator `cnt` (i64, init 0). At each position
    /// `S[j]` (read with `str_index`) is compared to the delimiter byte `D`, and
    /// `cnt` is bumped on a match:
    ///
    /// ```text
    ///   cnt = 0;  j = 0;  len = str_len(S)
    /// insp_top:  if j >= len   jmp insp_end
    ///            if S[j] == D   cnt = cnt + 1        # (skip-around when not equal)
    ///            j = j + 1;  jmp insp_top
    /// insp_end:
    ///   counter := (counter_value + cnt) reduced into counter's picture
    /// ```
    ///
    /// `FOR LEADING delim` is also supported — it counts only the run of
    /// consecutive `delim` characters at the START of the source, breaking out of
    /// the scan at the first non-match (see [`Self::emit_inspect_tallying`]).
    ///
    /// The count is folded into the counter with the SAME numeric-store path `ADD`
    /// uses (`store_scaled`, which mirrors the oracle's `store_result`/
    /// `move_into_numeric`), so a compiled program matches `cobol-runtime`'s
    /// `exec_inspect` byte-for-byte. Every later-rung form — a `CHARACTERS` tally,
    /// `BEFORE`/`AFTER` phrases, several counters or `FOR` phrases, and a
    /// multi-character/figurative/wider delimiter or a numeric source or a
    /// non-integer/non-numeric counter — is a clean `Unsupported`, accepted by the
    /// grammar and rejected here. (Both combined halves may independently be
    /// `LEADING`: `TALLYING … FOR LEADING … REPLACING LEADING` is now supported.)
    fn emit_inspect(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        // This rung supports a LONE `TALLYING … FOR ALL`, a LONE `REPLACING ALL …
        // BY …`, or the COMBINED `TALLYING … REPLACING` in one INSPECT. The
        // combined form runs the two lowerings in the ISO order — tally FIRST
        // (counting the ORIGINAL bytes into the counter), replace SECOND (rewriting
        // the source) — which is what makes a shared delimiter/search char correct.
        let has_tally = child_node(verb, "inspect_tallying").is_some();
        let has_repl = child_node(verb, "inspect_replacing").is_some();
        let has_conv = child_node(verb, "inspect_converting").is_some();
        // The source is the first (and only top-level) `operand`; shared by both
        // the TALLYING and REPLACING forms. It must be a plain alphanumeric item.
        let source_node = child_node(verb, "operand")
            .ok_or_else(|| CompileError::Malformed("INSPECT without a source".into()))?;
        let source_name = match read_operand(source_node)? {
            Operandy::Name(n) => n,
            Operandy::Literal(_) => {
                return Err(CompileError::Unsupported(
                    "INSPECT of a literal source is a later rung".into(),
                ))
            }
            Operandy::RefMod { .. } => {
                return Err(CompileError::Unsupported(
                    "INSPECT of a reference-modified source is a later rung".into(),
                ))
            }
        };
        let sidx = self.item_index(&source_name)?;
        let source_width = match &self.items[sidx].kind {
            ItemKind::Char { .. } => self.items[sidx].width(),
            ItemKind::Numeric { .. } => {
                return Err(CompileError::Unsupported(
                    "INSPECT of a numeric source is a later rung".into(),
                ))
            }
        };
        let s_reg = self.items[sidx].reg.clone();

        // Dispatch on the phrases present. The COMBINED form composes the two
        // existing lowerings in ISO order on the SAME source register: the tally
        // loop reads the original bytes into the counter FIRST, then the replace
        // rebuild overwrites the source — so a shared delimiter/search character is
        // counted before it is substituted. The two lone forms are the single
        // branches of that same composition.
        // CONVERTING is a STANDALONE alternative — the grammar never lets it appear
        // beside TALLYING/REPLACING — so it dispatches on its own before the
        // tally/replace composition.
        if has_conv {
            return self.emit_inspect_converting(verb, &s_reg, source_width);
        }
        match (has_tally, has_repl) {
            (true, true) => {
                // The combined form's TALLYING half supports BOTH `FOR ALL` and
                // `FOR LEADING`: `allow_leading = true` lets a combined
                // `TALLYING … FOR LEADING … REPLACING` count only the leading run
                // (`emit_inspect_tallying` already emits the `leading ? end : nobump`
                // branch that stops at the first non-match). It also now accepts its
                // OWN optional `{BEFORE|AFTER}` region (`allow_region = true`): the
                // window rides on the `inspect_tallying` phrase child, is scanned over
                // the ORIGINAL source (the tally loop only reads), and bounds the
                // count. A combined `FOR LEADING` half carrying a region is now
                // SUPPORTED (this rung): `allow_leading_region = true` lets
                // `emit_inspect_tallying` emit the SAME window-anchored LEADING lowering
                // the standalone `FOR LEADING … BEFORE/AFTER` path uses — byte-identical.
                // `allow_characters = true` (THIS rung): a combined `TALLYING … FOR
                // CHARACTERS … REPLACING` is now supported. `emit_inspect_tallying`
                // routes `FOR CHARACTERS` into the SAME "count every window position"
                // lowering the STANDALONE `FOR CHARACTERS` tally (#60) uses — `cnt =
                // len(S)` with no region, `cnt = end - start` with one — running over
                // the ORIGINAL bytes BEFORE the REPLACING half overwrites the source, in
                // ISO tally-then-replace order. Matching the oracle's combined-form
                // accept. (The REPLACING half's OWN `CHARACTERS` form is a DIFFERENT node
                // — `emit_inspect_replacing_characters` — now ALSO supported in the
                // combined form; see the REPLACING-half dispatch just below.)
                self.emit_inspect_tallying(verb, &s_reg, true, true, true, true)?;
                // The combined REPLACING half is EITHER a lone `REPLACING CHARACTERS BY
                // x` (THIS rung) OR an `ALL`/`LEADING … BY …` substitution. Mirroring
                // the oracle reader (and the lone-`(false, true)` dispatch below), we
                // DETECT a lone CHARACTERS item FIRST — a SINGLE `replace_item` carrying
                // the CHARACTERS keyword — and route it to the SAME standalone
                // `emit_inspect_replacing_characters` lowering the lone form (#61/#80)
                // uses, running AFTER the tally emit over the ORIGINAL bytes (which the
                // read-only count loop left untouched), in ISO tally-then-replace order.
                // A MULTI-item (2+) or standalone-`CHARACTERS`-elsewhere REPLACING half
                // still flows through `emit_inspect_replacing`/`inspect_replacing_all`,
                // which rejects it co-total with the oracle.
                let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
                    CompileError::Unsupported(
                        "INSPECT without a REPLACING clause is a later rung".into(),
                    )
                })?;
                let repl_items = child_nodes(replacing, "replace_item");
                if let [ri] = repl_items.as_slice() {
                    let toks = child_tokens(ri);
                    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
                        // The CHARACTERS fill overwrites `s_reg` and reads its own
                        // optional `{BEFORE|AFTER}` region off `ri` — the SAME window
                        // scan the lone form uses, over the post-tally (== original)
                        // storage. Byte-identical to the oracle's
                        // `inspect_replace_characters`.
                        return self.emit_inspect_replacing_characters(ri, &s_reg, source_width);
                    }
                }
                // The ALL/LEADING path. The combined REPLACING half supports BOTH `ALL`
                // and `LEADING` (`allow_leading = true`), each with its OWN INDEPENDENT
                // `{BEFORE|AFTER}` region (`allow_region = true`) scanned over the same
                // original bytes the count saw, and a `REPLACING LEADING … BEFORE/AFTER`
                // half (`allow_leading_region = true`) — byte-identical to the oracle.
                // `inspect_replacing_all` (inside `emit_inspect_replacing`) rejects a
                // MULTI-item half co-total with the oracle's `read_inspect_replacing_all`.
                self.emit_inspect_replacing(verb, &s_reg, source_width, true, true, true)
            }
            // A lone REPLACING. Dispatch on the number of replace items, mirroring the
            // oracle's `read_statement`: a lone `REPLACING CHARACTERS BY x` routes to
            // `emit_inspect_replacing_characters` (below); exactly ONE `ALL`/`LEADING` item
            // keeps the full single-item lowering (both `ALL` and `LEADING`, `REPLACING
            // ALL` may carry a `{BEFORE|AFTER}` region, and the STANDALONE `REPLACING
            // LEADING … BEFORE/AFTER` is supported via `allow_leading_region = true`); TWO
            // OR MORE items take the multi-item lowering (a MIX of `ALL`/`LEADING`/
            // `CHARACTERS`, single-char, each with its OWN optional region — one
            // left-to-right first-match-wins pass). Counting the same `replace_item`
            // children the oracle counts keeps the two engines' accept/reject sets
            // co-total.
            (false, true) => {
                let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
                    CompileError::Unsupported(
                        "INSPECT without a REPLACING clause is a later rung".into(),
                    )
                })?;
                let items = child_nodes(replacing, "replace_item");
                // Detect a lone `REPLACING CHARACTERS BY x` FIRST — a SINGLE replace
                // item whose tokens carry the CHARACTERS keyword — mirroring the
                // oracle's `read_statement`. (A MULTI-item list containing CHARACTERS
                // stays a later rung via `inspect_replacing_multi`; the CHARACTERS
                // reject in `inspect_replacing_all` still guards the combined form.)
                if let [ri] = items.as_slice() {
                    let toks = child_tokens(ri);
                    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
                        return self.emit_inspect_replacing_characters(ri, &s_reg, source_width);
                    }
                }
                if items.len() >= 2 {
                    self.emit_inspect_replacing_multi(verb, &s_reg, source_width)
                } else {
                    self.emit_inspect_replacing(verb, &s_reg, source_width, true, true, true)
                }
            }
            // A lone TALLYING (or neither, which `inspect_tally_all` rejects).
            // Dispatch on the number of `FOR` items UNDER THE SOLE counter, mirroring
            // the oracle's `read_statement`: exactly ONE `tally_item` keeps the full
            // single-item lowering (both `ALL` and `LEADING`, `FOR ALL` may carry a
            // `{BEFORE|AFTER}` region, and the STANDALONE `FOR LEADING … BEFORE/AFTER`
            // is supported via `allow_leading_region = true`); TWO OR MORE `tally_item`s
            // under one `tally_for` take the multi-item lowering (`ALL`-only,
            // single-char, no region — one first-match-per-position pass into the shared
            // counter). The multi lowering fires ONLY when there is EXACTLY ONE
            // `tally_for`: SEVERAL counters stays a later rung, rejected unchanged by
            // `emit_inspect_tallying`/`inspect_tally_all`. Counting the same `tally_item`
            // children the oracle counts keeps the two engines' accept/reject sets
            // co-total.
            _ => {
                if let Some(tallying) = child_node(verb, "inspect_tallying") {
                    let fors = child_nodes(tallying, "tally_for");
                    // TWO OR MORE `tally_for` groups take the NEW multi-COUNTER lowering:
                    // each group has its own counter, and ALL groups' delimiters form ONE
                    // combined priority list scanned in a single left-to-right pass (see
                    // `emit_inspect_tally_counters`). Dispatching PURELY on `fors.len() >=
                    // 2` — BEFORE the single-`tally_for` multi-item branch — keeps this
                    // co-total with the oracle's `read_statement`: several counters is no
                    // longer rejected here (that reject still guards the COMBINED
                    // `TALLYING … REPLACING` form via `inspect_tally_all`).
                    if fors.len() >= 2 {
                        return self.emit_inspect_tally_counters(verb, &s_reg);
                    }
                    if fors.len() == 1 && child_nodes(fors[0], "tally_item").len() >= 2 {
                        return self.emit_inspect_tally_multi(verb, &s_reg);
                    }
                }
                // `allow_characters = true`: the STANDALONE `FOR CHARACTERS` form is
                // supported this rung.
                self.emit_inspect_tallying(verb, &s_reg, true, true, true, true)
            }
        }
    }

    /// `INSPECT source TALLYING counter FOR ALL delim` (or `FOR LEADING delim`) —
    /// the count loop and counter store, factored out of [`Self::emit_inspect`] so
    /// the combined tally-then-replace form can emit it FIRST (over the original
    /// source bytes) and share the exact ADD-into-counter store path. The loop only
    /// reads `s_reg`; it never writes it, so a following REPLACING still sees the
    /// original image.
    ///
    /// `FOR ALL` counts EVERY match; `FOR LEADING` counts only the run of
    /// consecutive matches at the START, stopping at the first non-match. The two
    /// share the identical loop — the ONLY difference is where the "not equal"
    /// branch jumps: `FOR ALL` skips just the `cnt += 1` and keeps scanning
    /// (`nobump`), while `FOR LEADING` breaks out of the loop entirely (`end`).
    ///
    /// `allow_leading` gates whether `FOR LEADING` is accepted. Both the lone
    /// TALLYING and the combined tally-then-replace path now pass `true` (a
    /// combined `TALLYING … FOR LEADING … REPLACING` is supported); the guard is
    /// retained so any future caller that must forbid `FOR LEADING` can pass
    /// `false` and get the clean later-rung diagnostic.
    ///
    /// `allow_region` gates whether a `{BEFORE|AFTER} x` region is accepted. Both the
    /// lone TALLYING and the combined path now pass `true` (a `FOR ALL … BEFORE/AFTER`
    /// region is supported on either). When a region IS present, we first scan the
    /// source for the FIRST occurrence of the single region delimiter, derive the
    /// window `[start, end)` with the ISO not-found asymmetry (BEFORE→whole source,
    /// AFTER→empty), and bound the count loop to that window. With NO region, nothing
    /// extra is emitted — the lowering is byte-identical to the pre-region code.
    ///
    /// `allow_leading_region` gates the `FOR LEADING … BEFORE/AFTER` form. BOTH the
    /// lone TALLYING and (as of this rung) the combined `TALLYING … REPLACING` path
    /// pass `true`, so a combined `TALLYING … FOR LEADING … BEFORE/AFTER` emits the
    /// same window-anchored lowering the standalone form does — byte-identical to the
    /// oracle. (The `!allow_leading_region` guard below is retained honestly so the
    /// gate reads uniformly with the other later-rung guards.) When a leading
    /// count DOES carry a region, the scan is ANCHORED at the window
    /// start: the loop counter is initialised to `start` and bounded by the window
    /// `end` (not `0..len`), so `FOR LEADING` counts the run beginning at the window
    /// start — e.g. `FOR LEADING "a" AFTER "X"` on "aaXaab" counts the two a's after
    /// the X, not the leading "aa" before it. (`FOR ALL`'s lowering is untouched: it
    /// still scans `0..len` and uses the in-window guard, so it stays byte-identical.)
    fn emit_inspect_tallying(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
        allow_leading: bool,
        allow_region: bool,
        allow_leading_region: bool,
        allow_characters: bool,
    ) -> Result<(), CompileError> {
        // Extract the single `FOR ALL`/`FOR LEADING delim [{BEFORE|AFTER} x]` (or
        // `FOR CHARACTERS [{BEFORE|AFTER} x]`) phrase (rejecting the later rungs).
        let (counter_name, delim_node, leading, characters, region) = inspect_tally_all(verb)?;
        // `FOR CHARACTERS` is supported BOTH standalone AND (as of this rung) on the
        // combined `TALLYING … REPLACING` form's TALLYING half — both callers pass
        // `allow_characters = true`. The guard is retained honestly so any future
        // caller that must forbid a CHARACTERS tally can pass `false` and get the clean
        // later-rung diagnostic, reading uniformly with the other gates below. (The
        // combined form's REPLACING-half CHARACTERS is now supported too, on a DIFFERENT
        // path — a lone CHARACTERS half routes to `emit_inspect_replacing_characters`
        // before reaching `inspect_replacing_all` — not this flag.)
        if characters && !allow_characters {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING … FOR CHARACTERS in a combined TALLYING/REPLACING is a later rung"
                    .into(),
            ));
        }
        if leading && !allow_leading {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING … FOR LEADING combined with REPLACING is a later rung".into(),
            ));
        }
        if region.is_some() && !allow_region {
            return Err(CompileError::Unsupported(
                "INSPECT combined TALLYING … REPLACING with a BEFORE/AFTER region is a later rung"
                    .into(),
            ));
        }
        // The combined form still defers a LEADING half that carries a region (only
        // the standalone `FOR LEADING … BEFORE/AFTER` is supported this rung). The
        // message matches the standalone reject the reader used to raise, so both
        // engines and both forms diagnose it identically.
        if leading && region.is_some() && !allow_leading_region {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING … FOR LEADING with a BEFORE/AFTER region is a later rung".into(),
            ));
        }

        // The counter must be an unsigned integer numeric item (`PIC 9(n)`).
        let cidx = self.numeric_index(&counter_name)?;
        let (int_digits, dec_digits) = self.numeric_dims(cidx);
        if dec_digits != 0 {
            return Err(CompileError::Unsupported(format!(
                "INSPECT TALLYING into a non-integer counter {counter_name} is a later rung"
            )));
        }
        if self.item_signed(cidx) {
            return Err(CompileError::Unsupported(format!(
                "INSPECT TALLYING into a signed counter {counter_name} is a later rung"
            )));
        }
        let counter_reg = self.items[cidx].reg.clone();

        // `FOR CHARACTERS` is the "count every position" form: instead of scanning for
        // delimiter matches, the count is simply the NUMBER OF POSITIONS in the region
        // window. There is NO delimiter to reduce (`delim_node` is `None`), so we skip
        // `single_delim_code` and the per-position match loop entirely and emit only:
        //   * `len = str_len(S)`;
        //   * the optional `{BEFORE|AFTER} x` window `[start, end)` (the SAME window the
        //     ALL/LEADING count uses, so the not-found asymmetry is inherited verbatim);
        //   * `cnt = end - start` when a region is present, else `cnt = len`.
        // This is exactly the oracle's `window.len()` — with no region that is
        // `len(S)`, with a region it is `end - start` of the identical window — so the
        // count value is byte-identical across the two engines. A `BEFORE x` with `x`
        // absent yields the whole source (`end = len`, `start = 0`); an `AFTER x` with
        // `x` absent yields 0 (`start = end = len`), both by the shared window helper.
        if characters {
            let len = self.fresh("_insplen");
            self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");
            let region_window = self.emit_inspect_region_window(region, s_reg, &len)?;
            let cnt = match region_window {
                Some((start, win_end)) => {
                    // count = end - start (the window length).
                    let c = self.fresh("_inspclen");
                    self.emit(
                        "sub",
                        Some(&c),
                        vec![Operand::Var(win_end), Operand::Var(start)],
                        "i64",
                    );
                    c
                }
                // No region → count the WHOLE source: `len(S)`.
                None => len,
            };
            // counter := counter_value + cnt, reduced into the counter's picture — the
            // SAME numeric-store ADD the ALL/LEADING path uses below, so the reshape
            // (COBOL's silent high-order truncation) matches the oracle's
            // `store_result(counter, counter + count)`.
            let sum = self.fresh("_inspsum");
            self.emit("add", Some(&sum), vec![Operand::Var(counter_reg), Operand::Var(cnt)], "i64");
            return self.store_scaled(&counter_name, &sum, 0, int_digits + 1, false);
        }

        // The delimiter reduced to a single byte code register (ALL/LEADING only —
        // `delim_node` is `Some` here because `characters` is `false`).
        let d_reg = self.single_delim_code(
            delim_node.expect("ALL/LEADING tally carries a delimiter node"),
            "INSPECT",
        )?;

        // cnt = 0; j = 0; len = str_len(S).
        let cnt = self.fresh("_inspc");
        self.emit("const", Some(&cnt), vec![Operand::Int(0)], "i64");
        let len = self.fresh("_insplen");
        self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");
        let j = self.fresh("_inspj");
        self.emit("const", Some(&j), vec![Operand::Int(0)], "i64");

        // The optional `{BEFORE|AFTER} x` window `[start, end)`. When present we scan
        // the source ONCE for the first occurrence of the single region delimiter and
        // derive the bounds; when absent nothing here is emitted (byte-identical to
        // the pre-region lowering) and the count runs over the whole source.
        let region_window = self.emit_inspect_region_window(region, s_reg, &len)?;

        // Anchor the STANDALONE `FOR LEADING … BEFORE/AFTER` scan at the window start:
        // re-seat the loop counter to `start` and bound the loop by the window `end`
        // (instead of `0..len`), so `FOR LEADING`'s stop-on-first-mismatch runs from
        // the window start — the ISO window-anchored rule. The `const j = 0` above is
        // simply overwritten by this `mov` (a couple of extra IR ops on this path
        // only). For `FOR ALL`, and for `FOR LEADING` with NO region, `loop_bound`
        // stays `len` and nothing extra is emitted, so those lowerings are unchanged.
        let leading_windowed = leading && region_window.is_some();
        let loop_bound = if leading_windowed {
            let (start, win_end) = region_window.as_ref().expect("region present");
            self.emit("mov", Some(&j), vec![Operand::Var(start.clone())], "i64");
            win_end.clone()
        } else {
            len.clone()
        };

        let top = self.fresh("insp_top");
        let end = self.fresh("insp_end");
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        // if j >= loop_bound jmp end. (`loop_bound` is `len` for every case except the
        // window-anchored leading scan, where it is the window end.)
        let ge = self.fresh("_inspge");
        self.emit("cmp_ge", Some(&ge), vec![Operand::Var(j.clone()), Operand::Var(loop_bound.clone())], "i64");
        self.emit("jmp_if_true", None, vec![Operand::Var(ge), Operand::Var(end.clone())], "void");
        // On a non-match: FOR ALL skips just the bump and keeps scanning (`nobump`);
        // FOR LEADING breaks out of the loop entirely (`end`) — that stop-on-first-
        // mismatch is the whole difference between the two forms.
        let c = self.fresh("_inspc0");
        self.emit("str_index", Some(&c), vec![Operand::Var(s_reg.to_string()), Operand::Var(j.clone())], "i64");
        let eq = self.fresh("_inspeq");
        self.emit("cmp_eq", Some(&eq), vec![Operand::Var(c), Operand::Var(d_reg.clone())], "i64");
        let nobump = self.fresh("insp_nobump");
        let mismatch_target = if leading { end.clone() } else { nobump.clone() };
        self.emit("jmp_if_false", None, vec![Operand::Var(eq), Operand::Var(mismatch_target)], "void");
        // Region guard for the `FOR ALL` window (only when a `{BEFORE|AFTER}` window is
        // present AND we are NOT window-anchored): a matching character OUTSIDE
        // `[start, end)` is skipped (jump to `nobump`, keep scanning) — `j < start` or
        // `j >= end` means "not in the region". The window-anchored `FOR LEADING` scan
        // already starts at `start` and stops at the window `end`, so it needs no
        // per-position guard; only `FOR ALL` (which scans the whole `0..len`) does.
        if !leading_windowed {
            if let Some((start, end_bound)) = &region_window {
                let lt = self.fresh("_insplt");
                self.emit("cmp_lt", Some(&lt), vec![Operand::Var(j.clone()), Operand::Var(start.clone())], "i64");
                self.emit("jmp_if_true", None, vec![Operand::Var(lt), Operand::Var(nobump.clone())], "void");
                let ge2 = self.fresh("_inspge2");
                self.emit("cmp_ge", Some(&ge2), vec![Operand::Var(j.clone()), Operand::Var(end_bound.clone())], "i64");
                self.emit("jmp_if_true", None, vec![Operand::Var(ge2), Operand::Var(nobump.clone())], "void");
            }
        }
        let one = self.fresh("_insp1");
        self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&cnt), vec![Operand::Var(cnt.clone()), Operand::Var(one)], "i64");
        self.emit("label", None, vec![Operand::Var(nobump)], "void");
        // j = j + 1; jmp top.
        let one2 = self.fresh("_insp1b");
        self.emit("const", Some(&one2), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&j), vec![Operand::Var(j.clone()), Operand::Var(one2)], "i64");
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(end)], "void");

        // counter := counter_value + cnt, reduced into the counter's picture — the
        // exact numeric-store ADD uses (COBOL's silent high-order truncation), so
        // this matches the oracle's `store_result(counter, counter + cnt)`.
        let sum = self.fresh("_inspsum");
        self.emit("add", Some(&sum), vec![Operand::Var(counter_reg), Operand::Var(cnt)], "i64");
        self.store_scaled(&counter_name, &sum, 0, int_digits + 1, false)
    }

    /// Emit the runtime window `[start, end)` for an optional `{BEFORE|AFTER} x`
    /// INSPECT region, returning `Some((start_reg, end_reg))` (both i64 registers)
    /// when a region is present, or `None` when it is absent (so the caller emits
    /// nothing extra and counts the whole source).
    ///
    /// A region is bounded by the FIRST (leftmost) occurrence of the single region
    /// delimiter `x`. We scan the source once, recording a `found` flag and the
    /// first index `fidx`, then derive the bounds with the ISO not-found asymmetry:
    ///
    /// ```text
    ///   fidx = 0; found = 0; rj = 0
    /// rtop: if rj >= len jmp rdone          # ran off the end → not found
    ///       if S[rj] != x   jmp rcont
    ///       fidx = rj; found = 1; jmp rdone # first match → record and stop
    /// rcont: rj = rj + 1; jmp rtop
    /// rdone:
    ///   BEFORE:  start = 0;   end = found ? fidx     : len   # absent → WHOLE source
    ///   AFTER:   end   = len; start = found ? fidx+1  : len   # absent → EMPTY window
    /// ```
    ///
    /// The BEFORE→whole / AFTER→empty split is the whole subtlety of the rung, and it
    /// mirrors the oracle's `inspect_tally` window exactly, so both engines agree
    /// byte-for-byte. A multi-character region delimiter is rejected by
    /// `single_delim_code`, exactly like the tally delimiter.
    fn emit_inspect_region_window(
        &mut self,
        region: Option<(RegionKind, &GrammarASTNode)>,
        s_reg: &str,
        len: &str,
    ) -> Result<Option<(String, String)>, CompileError> {
        let (kind, rdelim_node) = match region {
            None => return Ok(None),
            Some(r) => r,
        };
        // The region delimiter reduced to a single byte code register.
        let rd = self.single_delim_code(rdelim_node, "INSPECT")?;

        // found = 0; fidx = 0; rj = 0.
        let found = self.fresh("_insprf");
        self.emit("const", Some(&found), vec![Operand::Int(0)], "i64");
        let fidx = self.fresh("_insprfi");
        self.emit("const", Some(&fidx), vec![Operand::Int(0)], "i64");
        let rj = self.fresh("_insprj");
        self.emit("const", Some(&rj), vec![Operand::Int(0)], "i64");

        let rtop = self.fresh("insp_rtop");
        let rcont = self.fresh("insp_rcont");
        let rdone = self.fresh("insp_rdone");
        self.emit("label", None, vec![Operand::Var(rtop.clone())], "void");
        // if rj >= len jmp rdone (ran off the end without a match → found stays 0).
        let rge = self.fresh("_insprge");
        self.emit("cmp_ge", Some(&rge), vec![Operand::Var(rj.clone()), Operand::Var(len.to_string())], "i64");
        self.emit("jmp_if_true", None, vec![Operand::Var(rge), Operand::Var(rdone.clone())], "void");
        // if S[rj] != x jmp rcont.
        let rc = self.fresh("_insprc");
        self.emit("str_index", Some(&rc), vec![Operand::Var(s_reg.to_string()), Operand::Var(rj.clone())], "i64");
        let req = self.fresh("_inspreq");
        self.emit("cmp_eq", Some(&req), vec![Operand::Var(rc), Operand::Var(rd.clone())], "i64");
        self.emit("jmp_if_false", None, vec![Operand::Var(req), Operand::Var(rcont.clone())], "void");
        // Match: record the FIRST index and stop scanning.
        self.emit("mov", Some(&fidx), vec![Operand::Var(rj.clone())], "i64");
        self.emit("const", Some(&found), vec![Operand::Int(1)], "i64");
        self.emit("jmp", None, vec![Operand::Var(rdone.clone())], "void");
        // rcont: rj = rj + 1; jmp rtop.
        self.emit("label", None, vec![Operand::Var(rcont)], "void");
        let rone = self.fresh("_inspr1");
        self.emit("const", Some(&rone), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&rj), vec![Operand::Var(rj.clone()), Operand::Var(rone)], "i64");
        self.emit("jmp", None, vec![Operand::Var(rtop)], "void");
        self.emit("label", None, vec![Operand::Var(rdone)], "void");

        // Derive [start, end) from (kind, found, fidx). The `skip` label guards the
        // "found" adjustment: when `found == 0` the default (whole for BEFORE, empty
        // for AFTER) stands.
        let start = self.fresh("_insprs");
        let end = self.fresh("_inspre");
        match kind {
            RegionKind::Before => {
                // start = 0; end = found ? fidx : len (absent → whole source).
                self.emit("const", Some(&start), vec![Operand::Int(0)], "i64");
                self.emit("mov", Some(&end), vec![Operand::Var(len.to_string())], "i64");
                let skip = self.fresh("insp_bskip");
                self.emit("jmp_if_false", None, vec![Operand::Var(found.clone()), Operand::Var(skip.clone())], "void");
                self.emit("mov", Some(&end), vec![Operand::Var(fidx.clone())], "i64");
                self.emit("label", None, vec![Operand::Var(skip)], "void");
            }
            RegionKind::After => {
                // end = len; start = found ? fidx+1 : len (absent → empty window).
                self.emit("mov", Some(&end), vec![Operand::Var(len.to_string())], "i64");
                self.emit("mov", Some(&start), vec![Operand::Var(len.to_string())], "i64");
                let skip = self.fresh("insp_askip");
                self.emit("jmp_if_false", None, vec![Operand::Var(found.clone()), Operand::Var(skip.clone())], "void");
                let rone2 = self.fresh("_inspr1b");
                self.emit("const", Some(&rone2), vec![Operand::Int(1)], "i64");
                self.emit("add", Some(&start), vec![Operand::Var(fidx.clone()), Operand::Var(rone2)], "i64");
                self.emit("label", None, vec![Operand::Var(skip)], "void");
            }
        }
        Ok(Some((start, end)))
    }

    /// `INSPECT source REPLACING ALL x BY y` (or `REPLACING LEADING x BY y`) —
    /// rebuild the alphanumeric `source` with the single character `x` replaced by
    /// the single character `y`, in place. Because both are single characters the
    /// width `W` is unchanged, so the result is a per-position map that we UNROLL
    /// over the compile-time-known `W`. For `ALL`, at each position `j` the output
    /// character is `S[j] == x ? y : S[j]`.
    ///
    /// `REPLACING LEADING` replaces only the run of consecutive `x` characters at
    /// the START of the source, stopping at the first non-`x`. We thread a runtime
    /// `active` flag (i64, init 1 = still inside the leading run) through the
    /// unroll: position `j` is replaced iff `active AND (S[j] == x)` — i.e. every
    /// position `0..=j` equalled `x`. Once a mismatch clears `active`, it stays 0
    /// for all later positions, so a later `x` is left unchanged. That is the ONLY
    /// difference from `ALL`; when `leading` is false the extra `and` folds away
    /// and the emitted unroll is byte-identical to the original `ALL` lowering.
    ///
    /// ```text
    ///   result = ""
    ///   active = 1                            # LEADING only
    ///   for j in 0..W:                        # W is known at compile time
    ///       eq = (S[j] == x)
    ///       in_win = (start <= j < end)       # region only (else always true)
    ///       use_repl = LEADING ? (active AND eq AND in_win) : (eq AND in_win)
    ///       if use_repl    result = result ++ y_str
    ///       else           result = result ++ S[j, j+1)
    ///       # LEADING: decay the run ONLY on an in-window mismatch, so positions
    ///       # before the window leave the run untouched (anchored at `start`):
    ///       active = active AND (eq OR NOT in_win)
    ///   source := result                      # exactly W chars, width unchanged
    /// ```
    ///
    /// The search `x` is reduced to a byte code with the shared
    /// [`Self::single_delim_code`] (so `str_index(S, j)` compares against it); the
    /// replacement `y` is reduced to a 1-character string with the parallel
    /// [`Self::single_delim_str`] so it can be concatenated. Both share
    /// UNSTRING/TALLYING's single-character validation, so a multi-character/
    /// figurative/wider/numeric `x` or `y` is a clean later-rung `Unsupported`.
    /// The rebuilt string is copied into the source register — the same W-wide
    /// alphanumeric image the oracle's `move_into` produces, byte-for-byte.
    ///
    /// `allow_leading` is `true` on both the lone REPLACING path and the combined
    /// tally-then-replace path — a combined `TALLYING … REPLACING LEADING` now
    /// lowers exactly like a lone `REPLACING LEADING`, independent of the TALLYING
    /// half's own leading flag.
    ///
    /// `allow_region` gates whether a `{BEFORE|AFTER} z` region is accepted. Both the
    /// lone REPLACING and the combined path now pass `true`. When a region is present
    /// we reuse [`Self::emit_inspect_region_window`] — the SAME window the TALLYING
    /// side emits — to derive `[start, end)` over the ORIGINAL source. For `ALL`, each
    /// unrolled position `j` is replaced iff `start <= j < end AND S[j] == x`. For
    /// `LEADING` with a region the run is ANCHORED at the window start: a position
    /// OUTSIDE `[start, end)` keeps its original character AND leaves `active`
    /// untouched (characters before `start` neither begin nor break the run), so the
    /// leading substitution genuinely starts at `start`. With NO region the extra
    /// guard folds away and the emitted unroll is byte-identical to the pre-region
    /// `ALL`/`LEADING` lowerings.
    ///
    /// `allow_leading_region` gates the `REPLACING LEADING … BEFORE/AFTER` form. BOTH
    /// the lone REPLACING and (as of this rung) the combined `TALLYING … REPLACING`
    /// path pass `true`, so a combined `TALLYING … REPLACING LEADING … BEFORE/AFTER`
    /// emits the same window-anchored LEADING-run unroll the standalone form does —
    /// byte-identical to the oracle. (The `!allow_leading_region` guard below is
    /// retained honestly so the gate reads uniformly with the other later-rung guards.)
    fn emit_inspect_replacing(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
        width: usize,
        allow_leading: bool,
        allow_region: bool,
        allow_leading_region: bool,
    ) -> Result<(), CompileError> {
        // The single `ALL`/`LEADING x BY y [{BEFORE|AFTER} z]` phrase (rejecting the
        // later rungs).
        let (search_node, replace_node, leading, region) = inspect_replacing_all(verb)?;
        if leading && !allow_leading {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING … REPLACING LEADING is a later rung".into(),
            ));
        }
        if region.is_some() && !allow_region {
            return Err(CompileError::Unsupported(
                "INSPECT combined TALLYING … REPLACING with a BEFORE/AFTER region is a later rung"
                    .into(),
            ));
        }
        // The combined form still defers a LEADING half that carries a region (only the
        // standalone `REPLACING LEADING … BEFORE/AFTER` is supported this rung). The
        // message matches the standalone reject the reader used to raise.
        if leading && region.is_some() && !allow_leading_region {
            return Err(CompileError::Unsupported(
                "INSPECT REPLACING LEADING with a BEFORE/AFTER region is a later rung".into(),
            ));
        }
        // x → a byte code (for the per-position compare); y → a 1-char string
        // (for the concatenation). Both share the single-character validation.
        let x_reg = self.single_delim_code(search_node, "INSPECT REPLACING")?;
        let y_reg = self.single_delim_str(replace_node, "INSPECT REPLACING")?;

        // The optional `{BEFORE|AFTER} z` window `[start, end)`, derived over the
        // ORIGINAL source (before the unroll overwrites `s_reg`). We reuse the tally
        // side's `emit_inspect_region_window`, which needs the runtime length; with no
        // region nothing here is emitted and the guard below folds away.
        let region_window = match region {
            None => None,
            Some(_) => {
                let len = self.fresh("_irlen");
                self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");
                self.emit_inspect_region_window(region, s_reg, &len)?
            }
        };

        // result = "" — the accumulator we build W characters into.
        let result = self.fresh("_irres");
        self.emit("str_const", Some(&result), vec![Operand::Str(String::new())], "str");

        // active = 1 — still inside the leading run (LEADING only; unused for ALL).
        let active = self.fresh("_iractive");
        if leading {
            self.emit("const", Some(&active), vec![Operand::Int(1)], "i64");
        }

        for j in 0..width {
            // c = S[j]  (the source byte at this position).
            let jc = self.str_index(j as i64);
            let c = self.fresh("_irc");
            self.emit(
                "str_index",
                Some(&c),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc.clone())],
                "i64",
            );
            // eq = (c == x).
            let eq = self.fresh("_ireq");
            self.emit("cmp_eq", Some(&eq), vec![Operand::Var(c), Operand::Var(x_reg.clone())], "i64");
            // in_region = (j >= start) AND (j < end) when a `{BEFORE|AFTER}` window
            // applies (both `ALL` and the STANDALONE `LEADING`); `None` with no region.
            // `j` is the compile-time constant for this unrolled position, materialised
            // into a register so it can be compared against the runtime window bounds.
            let in_region = match &region_window {
                Some((start, end_bound)) => {
                    let jreg = self.fresh("_irjr");
                    self.emit("const", Some(&jreg), vec![Operand::Int(j as i64)], "i64");
                    let ge = self.fresh("_irge");
                    self.emit(
                        "cmp_ge",
                        Some(&ge),
                        vec![Operand::Var(jreg.clone()), Operand::Var(start.clone())],
                        "i64",
                    );
                    let lt = self.fresh("_irlt");
                    self.emit(
                        "cmp_lt",
                        Some(&lt),
                        vec![Operand::Var(jreg), Operand::Var(end_bound.clone())],
                        "i64",
                    );
                    let ir = self.fresh("_irin");
                    self.emit("and", Some(&ir), vec![Operand::Var(ge), Operand::Var(lt)], "i64");
                    Some(ir)
                }
                None => None,
            };
            // use_repl — whether to substitute at this position:
            //   plain ALL        -> eq
            //   ALL + region     -> in_region AND eq
            //   LEADING          -> active AND eq
            //   LEADING + region -> (active AND eq) AND in_region  (run anchored at the
            //                        window start; positions outside the window never
            //                        replace and — below — never break the run)
            let branch = match (leading, &in_region) {
                (false, None) => eq.clone(),
                (false, Some(ir)) => {
                    let use_repl = self.fresh("_iruse");
                    self.emit(
                        "and",
                        Some(&use_repl),
                        vec![Operand::Var(ir.clone()), Operand::Var(eq.clone())],
                        "i64",
                    );
                    use_repl
                }
                (true, None) => {
                    let use_repl = self.fresh("_iruse");
                    self.emit(
                        "and",
                        Some(&use_repl),
                        vec![Operand::Var(active.clone()), Operand::Var(eq.clone())],
                        "i64",
                    );
                    use_repl
                }
                (true, Some(ir)) => {
                    let am = self.fresh("_iruse");
                    self.emit(
                        "and",
                        Some(&am),
                        vec![Operand::Var(active.clone()), Operand::Var(eq.clone())],
                        "i64",
                    );
                    let use_repl = self.fresh("_iruse2");
                    self.emit(
                        "and",
                        Some(&use_repl),
                        vec![Operand::Var(am), Operand::Var(ir.clone())],
                        "i64",
                    );
                    use_repl
                }
            };
            let use_orig = self.fresh("ir_orig");
            let done = self.fresh("ir_done");
            // On a match (within the run, for LEADING), append the replacement `y`;
            // otherwise the original char.
            self.emit("jmp_if_false", None, vec![Operand::Var(branch), Operand::Var(use_orig.clone())], "void");
            self.emit(
                "str_concat",
                Some(&result),
                vec![Operand::Var(result.clone()), Operand::Var(y_reg.clone())],
                "str",
            );
            self.emit("jmp", None, vec![Operand::Var(done.clone())], "void");
            self.emit("label", None, vec![Operand::Var(use_orig)], "void");
            // orig = S[j, j+1) — the source character unchanged.
            let jc1 = self.str_index(j as i64 + 1);
            let orig = self.fresh("_irorig");
            self.emit(
                "str_slice",
                Some(&orig),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc), Operand::Var(jc1)],
                "str",
            );
            self.emit(
                "str_concat",
                Some(&result),
                vec![Operand::Var(result.clone()), Operand::Var(orig)],
                "str",
            );
            self.emit("label", None, vec![Operand::Var(done)], "void");
            // Decay the leading run. (LEADING only; ALL never reads `active`.)
            if leading {
                match &in_region {
                    // No region: active := active AND eq — once a non-match clears it,
                    // it sticks at 0 for every later position, so LEADING never
                    // replaces past the first gap. Byte-identical to the pre-region
                    // leading lowering.
                    None => {
                        self.emit(
                            "and",
                            Some(&active),
                            vec![Operand::Var(active.clone()), Operand::Var(eq)],
                            "i64",
                        );
                    }
                    // With a region, decay ONLY on an IN-WINDOW mismatch:
                    //   active := active AND (eq OR NOT in_region)
                    // A position OUTSIDE the window has `NOT in_region == 1`, so the OR
                    // is 1 and `active` is left unchanged — characters before the window
                    // neither start nor break the run, anchoring it at the window start.
                    Some(ir) => {
                        let zero = self.fresh("_irz");
                        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                        let not_in = self.fresh("_irni");
                        self.emit(
                            "cmp_eq",
                            Some(&not_in),
                            vec![Operand::Var(ir.clone()), Operand::Var(zero)],
                            "i64",
                        );
                        let keep = self.fresh("_irkeep");
                        self.emit(
                            "or",
                            Some(&keep),
                            vec![Operand::Var(eq), Operand::Var(not_in)],
                            "i64",
                        );
                        self.emit(
                            "and",
                            Some(&active),
                            vec![Operand::Var(active.clone()), Operand::Var(keep)],
                            "i64",
                        );
                    }
                }
            }
        }

        // source := result. `result` is exactly W chars (each of the W pieces is a
        // single character), so this is the same fixed-width image the oracle
        // stores. Copy through an empty concat so the source register (read during
        // the loop) is only overwritten now, after the last read.
        let empty = self.fresh("_irempty");
        self.emit("str_const", Some(&empty), vec![Operand::Str(String::new())], "str");
        self.emit(
            "str_concat",
            Some(s_reg),
            vec![Operand::Var(result), Operand::Var(empty)],
            "str",
        );
        Ok(())
    }

    /// `INSPECT source REPLACING CHARACTERS BY x [{BEFORE|AFTER} z]` — overwrite the
    /// alphanumeric `source` positions with the single replacement character `x`. With
    /// no region the WHOLE field becomes `x`s; with a `{BEFORE|AFTER}` region only the
    /// window positions become `x` and the rest keep their original char. Width is
    /// unchanged either way.
    ///
    /// This reuses [`Self::emit_inspect_replacing`]'s rebuild scaffold. In the NO-region
    /// fast path every position becomes `x` UNCONDITIONALLY — there is no `S[j]`-vs-
    /// search compare, so we need not even read the source bytes: we append the
    /// 1-character replacement string `width` times (the picture's compile-time CHAR
    /// width) into a fresh accumulator, then copy it back. With a region we UNROLL over
    /// `0..width` and, at each position `j`, append `x` iff `start <= j < end` else the
    /// original `S[j, j+1)` char — the ALL-with-region structure MINUS the `S[j] == x`
    /// compare (CHARACTERS replaces EVERY in-window position). The window `[start, end)`
    /// is derived ONCE over the ORIGINAL source via the SAME
    /// [`Self::emit_inspect_region_window`] the ALL/region and TALLYING sides use.
    ///
    /// # Byte-basis co-totality (why `width` copies, not `str_len(S)` copies)
    ///
    /// The oracle fills `n = storage.len()` (BYTE-length) copies of `x` and then
    /// stores through `move_into`, which re-pads/truncates to the picture's CHAR size.
    /// For an ASCII field `str_len == width`, so this is just `width` copies. For a
    /// non-ASCII source whose byte length exceeds its char width (e.g.
    /// `PIC X(5) VALUE "café"` = 5 chars / 6 bytes) the oracle's `n = 6` copies cap to
    /// the picture's 5 chars. Emitting exactly `width` copies here reproduces that
    /// capped image byte-for-byte on both engines. With a region the per-position unroll
    /// emits exactly `width` pieces for the SAME reason.
    ///
    /// # Guards (IDENTICAL to the oracle)
    ///
    ///   3. A `{BEFORE|AFTER}` region is now ACCEPTED (THIS rung), lowered via
    ///      `emit_inspect_region_window` (a BYTE span). On an ASCII source it coincides
    ///      with the oracle's CHAR span byte-for-byte; a non-ASCII source (byte window
    ///      splitting a multi-byte char, or a multi-byte char inside the window) is the
    ///      PRE-EXISTING byte-vs-char chip shared with every other region form.
    ///   2. A single-char but NON-ASCII **literal** `x` is a later rung, so the
    ///      byte-based compiler stays co-total with the char-based oracle. Applied to
    ///      LITERALS only: a `PIC X(1)` *item* replacement is co-total under the fill.
    ///   1. `x` is a SINGLE character — the shared [`Self::single_delim_str`] check.
    fn emit_inspect_replacing_characters(
        &mut self,
        ri: &GrammarASTNode,
        s_reg: &str,
        width: usize,
    ) -> Result<(), CompileError> {
        // The lone DIRECT `operand` child is the replacement `x` (the `BY` operand); a
        // region's delimiter lives on the nested `inspect_region` child, not here.
        let replace_node = child_node(ri, "operand").ok_or_else(|| {
            CompileError::Malformed("INSPECT REPLACING CHARACTERS without a BY replacement".into())
        })?;
        // Guard 2 — a single-char but non-ASCII LITERAL replacement is deferred so the
        // messages/gating match the oracle exactly (`single_delim_str`'s byte-based
        // check would otherwise diagnose `"é"` as "multi-character" rather than as a
        // non-ASCII replacement). Applied to LITERALS only: an item is not gated.
        if let Operandy::Literal(Src::Str(s)) = read_operand(replace_node)? {
            if s.chars().count() == 1 && !s.is_ascii() {
                return Err(CompileError::Unsupported(
                    "INSPECT REPLACING CHARACTERS with a non-ASCII replacement is a later rung"
                        .into(),
                ));
            }
        }
        // Guard 1 — the single replacement char as a 1-character string register,
        // reusing REPLACING ALL's validation (multi-character/figurative/wider/numeric
        // operands are rejected with the shared messages).
        let y_reg = self.single_delim_str(replace_node, "INSPECT REPLACING")?;

        // The optional `{BEFORE|AFTER} z` region → `Option<(RegionKind, delim_node)>`,
        // extracted with the SAME keyword/operand logic `inspect_replacing_all` uses.
        // (The former Guard-3 reject is lifted; the oracle lifts the mirror guard.)
        let region: Option<(RegionKind, &GrammarASTNode)> =
            match child_node(ri, "inspect_region") {
                None => None,
                Some(region_node) => {
                    let rtoks = child_tokens(region_node);
                    let kind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                        RegionKind::Before
                    } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                        RegionKind::After
                    } else {
                        return Err(CompileError::Unsupported(
                            "INSPECT region without a BEFORE or AFTER keyword".into(),
                        ));
                    };
                    let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                        CompileError::Malformed(
                            "INSPECT BEFORE/AFTER region without a delimiter".into(),
                        )
                    })?;
                    Some((kind, rdelim))
                }
            };
        // Derive the window `[start, end)` over the ORIGINAL source (before the unroll
        // overwrites `s_reg`). With no region nothing is emitted and the per-position
        // guard folds away — the fast path below is byte-identical to the pre-region
        // lowering. `emit_inspect_region_window` needs the runtime BYTE length.
        let region_window = match region {
            None => None,
            Some(_) => {
                let len = self.fresh("_irclen");
                self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");
                self.emit_inspect_region_window(region, s_reg, &len)?
            }
        };

        // result = "" — the accumulator we build `width` characters into.
        let result = self.fresh("_ircres");
        self.emit("str_const", Some(&result), vec![Operand::Str(String::new())], "str");
        for j in 0..width {
            match &region_window {
                // No region: append `x` unconditionally (fast path — the fill never
                // reads `s_reg`).
                None => {
                    self.emit(
                        "str_concat",
                        Some(&result),
                        vec![Operand::Var(result.clone()), Operand::Var(y_reg.clone())],
                        "str",
                    );
                }
                // With a region: append `x` iff `start <= j < end`, else the original
                // `S[j, j+1)` char. `j` is this unrolled position's compile-time
                // constant, materialised into a register to compare against the runtime
                // window bounds. This is `emit_inspect_replacing`'s ALL-with-region
                // conditional-append idiom MINUS the `S[j] == x` compare.
                Some((start, end_bound)) => {
                    let jreg = self.fresh("_ircjr");
                    self.emit("const", Some(&jreg), vec![Operand::Int(j as i64)], "i64");
                    let ge = self.fresh("_ircge");
                    self.emit(
                        "cmp_ge",
                        Some(&ge),
                        vec![Operand::Var(jreg.clone()), Operand::Var(start.clone())],
                        "i64",
                    );
                    let lt = self.fresh("_irclt");
                    self.emit(
                        "cmp_lt",
                        Some(&lt),
                        vec![Operand::Var(jreg), Operand::Var(end_bound.clone())],
                        "i64",
                    );
                    let in_win = self.fresh("_ircin");
                    self.emit("and", Some(&in_win), vec![Operand::Var(ge), Operand::Var(lt)], "i64");
                    let use_orig = self.fresh("irc_orig");
                    let done = self.fresh("irc_done");
                    self.emit(
                        "jmp_if_false",
                        None,
                        vec![Operand::Var(in_win), Operand::Var(use_orig.clone())],
                        "void",
                    );
                    self.emit(
                        "str_concat",
                        Some(&result),
                        vec![Operand::Var(result.clone()), Operand::Var(y_reg.clone())],
                        "str",
                    );
                    self.emit("jmp", None, vec![Operand::Var(done.clone())], "void");
                    self.emit("label", None, vec![Operand::Var(use_orig)], "void");
                    // orig = S[j, j+1) — the source character unchanged.
                    let jc = self.str_index(j as i64);
                    let jc1 = self.str_index(j as i64 + 1);
                    let orig = self.fresh("_ircorig");
                    self.emit(
                        "str_slice",
                        Some(&orig),
                        vec![Operand::Var(s_reg.to_string()), Operand::Var(jc), Operand::Var(jc1)],
                        "str",
                    );
                    self.emit(
                        "str_concat",
                        Some(&result),
                        vec![Operand::Var(result.clone()), Operand::Var(orig)],
                        "str",
                    );
                    self.emit("label", None, vec![Operand::Var(done)], "void");
                }
            }
        }
        // source := result. `result` is exactly `width` characters, the same
        // fixed-width image the oracle stores after its `move_into` cap. Copy through
        // an empty concat so the source register (read during a region unroll) is only
        // overwritten now, after the last read.
        let empty = self.fresh("_ircempty");
        self.emit("str_const", Some(&empty), vec![Operand::Str(String::new())], "str");
        self.emit(
            "str_concat",
            Some(s_reg),
            vec![Operand::Var(result), Operand::Var(empty)],
            "str",
        );
        Ok(())
    }

    /// `INSPECT source REPLACING LEADING a BY x ALL b BY y …` — the multi-item
    /// REPLACING lowering, now with a possible `LEADING` item (THIS rung lifts the
    /// multi-item LEADING reject): ONE left-to-right pass with FIRST-MATCH-WINS and NO
    /// RE-CHAINING, matching `exec_inspect_replacing_multi` byte-for-byte.
    ///
    /// The subtlety the ordered chain encodes: at each source position the items are
    /// considered IN WRITTEN ORDER and the FIRST ELIGIBLE item wins; the byte a
    /// replacement produces is NEVER re-examined (we always read `S[j]` from the
    /// original source register, never from `result`). We UNROLL over the compile-time
    /// width `W`; at each position `j` we emit an ordered if-else CHAIN, one link per
    /// item:
    ///
    /// ```text
    ///   result = ""
    ///   active_k = 1                                   # per LEADING item only
    ///   # each item's window [start_k, end_k) is computed ONCE, before the unroll,
    ///   # over the ORIGINAL source (via emit_inspect_region_window); a region-less
    ///   # item has no window and its `in_win` guard folds away.
    ///   for j in 0..W:
    ///     c = S[j]
    ///     # decision — first eligible item wins; a LEADING item ALSO needs active_k:
    ///     if (s0<=j<e0) && c==x0 && (ALL_0 || active_0): result += y0; goto done_j
    ///     if (s1<=j<e1) && c==x1 && (ALL_1 || active_1): result += y1; goto done_j
    ///     …
    ///     result += S[j, j+1)                          # no item matched → original
    ///   done_j:
    ///     # update EVERY LEADING run, independent of which item won:
    ///     for each LEADING item k:  active_k = active_k AND (eq_k OR NOT in_win_k)
    ///   S = result
    /// ```
    ///
    /// The early `goto done_j` on a match is exactly first-match-wins: once item `k`
    /// fires, items `k+1…` are skipped for this position. And because every compare
    /// tests `c` (the ORIGINAL `S[j]`), never the just-appended replacement, a
    /// produced character is never chained into a second replacement —
    /// `ALL "a" BY "b" ALL "b" BY "z"` over `"ab"` compiles to `"bz"`, not `"zz"`.
    ///
    /// # The `LEADING` active-flag machine (this rung — twin of the tally side)
    ///
    /// This is the byte-producing twin of [`Self::emit_inspect_tally_multi`] — the ONLY
    /// difference is that the decision loop, instead of `cnt += 1`, EMITS the item's
    /// replacement string (and on no match the original char). A `LEADING` item carries
    /// a compile-time-threaded `active` register (i64, init 1 = its run is still alive),
    /// materialised before the unroll — exactly the single-item `emit_inspect_replacing`
    /// LEADING flag, generalised to a list. In the decision chain a `LEADING` link is
    /// eligible only while `active` is still `1`; AFTER the per-position `done_j` label
    /// (the convergence point BOTH a match and the no-match fall-through reach), EVERY
    /// `LEADING` item's `active` is decayed INDEPENDENTLY of which item won:
    /// `active := active AND (eq OR NOT in_win)` — a run breaks at the FIRST in-window
    /// mismatch, a matching char keeps it alive even if a higher-priority item claimed
    /// the position, and positions OUTSIDE the window never touch `active` (anchoring
    /// the run at the window start). We RECOMPUTE `eq`/`in_win` per leading item in the
    /// update section (the chain's registers may not have been reached on an early
    /// `goto done_j`), mirroring the oracle's separate active-update pass and the tally
    /// side's `cont` section.
    ///
    /// # Per-item regions
    ///
    /// Each item may carry its OWN optional `{BEFORE|AFTER} x` window. We reuse
    /// [`Self::emit_inspect_region_window`] — the SAME window the single-item region
    /// emitter and the TALLYING side emit — to derive each item's `[start, end)` over
    /// the ORIGINAL source ONCE, before the unroll (the runtime length is materialised
    /// once, shared across all items that carry a region). At each unrolled position
    /// `j` a region-carrying item's link tests `start <= j < end AND c == x`; a
    /// region-less item's link is just `c == x` (its guard folds away). This is the
    /// exact composition of the pre-existing multi-item first-match chain with the
    /// single-item region gate and the LEADING active flag, so the emitted output
    /// matches `exec_inspect_replacing_multi` byte-for-byte.
    ///
    /// Each item's search reduces to a byte code (`single_delim_code`, for the
    /// compare) and its replacement to a 1-char string (`single_delim_str`, for the
    /// concat), sharing the SAME single-character validation the single-item path
    /// uses — so a multi-character/figurative/wider/numeric operand is rejected
    /// identically. The read-side `inspect_replacing_multi` has already rejected
    /// CHARACTERS/FIRST items, so every item here is a single-char `{ALL|LEADING}` pair
    /// with an OPTIONAL region. Like the single-item emitter, the width-`W` `result` is
    /// copied back into `s_reg` through an empty concat AFTER the last read, so the
    /// source register is not overwritten mid-scan.
    fn emit_inspect_replacing_multi(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
        width: usize,
    ) -> Result<(), CompileError> {
        // The written-order list of `(search_node, replace_node, kind, region)` items (the
        // reader accepts a MIX of `ALL`/`LEADING`/`CHARACTERS`; each item MAY carry its own
        // region — a `CHARACTERS` item's `search_node` is `None`). If ANY item has a region
        // we materialise the runtime length ONCE, over the ORIGINAL source, since
        // `emit_inspect_region_window` needs it (and all region windows are computed over
        // that same original, before the unroll overwrites `s_reg` at the very end).
        let item_nodes = inspect_replacing_multi(verb)?;
        let len = if item_nodes.iter().any(|(_, _, _, region)| region.is_some()) {
            let l = self.fresh("_irmlen");
            self.emit("str_len", Some(&l), vec![Operand::Var(s_reg.to_string())], "i64");
            Some(l)
        } else {
            None
        };
        // Reduce each item to a `(search byte code, replacement 1-char string, kind,
        // active, window)` tuple, IN ORDER, so the per-position chain below can walk them
        // written-first. The window (when present) is derived by the SAME
        // `emit_inspect_region_window` the single-item region emitter uses, so both
        // engines narrow to identical slices. A `CHARACTERS` item has NO search (`None`) —
        // it is the always-eligible catch-all, so we skip `single_delim_code` (nothing to
        // validate/compare) and allocate no `active`. A `LEADING` item ALSO allocates a
        // runtime `active` register (i64, init `1` = its run is still alive), materialised
        // HERE — before the unroll — exactly like the single-item `emit_inspect_replacing`
        // LEADING flag and the tally side's `emit_inspect_tally_multi`. Operands, window,
        // and active flag are resolved BEFORE the unroll (mirroring the oracle, which
        // resolves chars/window per item up front, then inits `active`).
        let mut regs: Vec<ResolvedReplaceLeadingItem> = Vec::with_capacity(item_nodes.len());
        for (search_node, replace_node, kind, region) in item_nodes {
            let x_reg = match kind {
                ReplaceKind::Characters => None,
                _ => Some(self.single_delim_code(search_node.unwrap(), "INSPECT REPLACING")?),
            };
            let y_reg = self.single_delim_str(replace_node, "INSPECT REPLACING")?;
            let window = match region {
                None => None,
                Some(_) => {
                    let len = len.as_ref().expect("length materialised when a region is present");
                    self.emit_inspect_region_window(region, s_reg, len)?
                }
            };
            let active = if matches!(kind, ReplaceKind::Leading) {
                let a = self.fresh("_irmact");
                self.emit("const", Some(&a), vec![Operand::Int(1)], "i64");
                Some(a)
            } else {
                None
            };
            regs.push((x_reg, y_reg, kind, active, window));
        }

        // result = "" — the accumulator we build W characters into.
        let result = self.fresh("_irmres");
        self.emit("str_const", Some(&result), vec![Operand::Str(String::new())], "str");

        for j in 0..width {
            // c = S[j] — read ONCE from the original source and compared against
            // every item's search (so a replacement is never re-examined).
            let jc = self.str_index(j as i64);
            let c = self.fresh("_irmc");
            self.emit(
                "str_index",
                Some(&c),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc.clone())],
                "i64",
            );
            let done = self.fresh("irm_done");
            // The ordered DECISION chain: item 0, then item 1, … On the FIRST ELIGIBLE
            // item we append that item's replacement and jump to `done`, skipping the
            // rest — first-match-wins. A miss jumps to the next link (`next`); after the
            // last link's `next` we fall through to the no-match branch below. Eligibility
            // by kind:
            //   * `ALL`        — `(start <= j < end) AND c == x`;
            //   * `LEADING`    — as `ALL`, ALSO requiring its `active` run flag still `1`;
            //   * `CHARACTERS` — `(start <= j < end)` ALONE (the always-eligible catch-all:
            //                    NO search compare). A region-less `CHARACTERS` item is
            //                    UNCONDITIONALLY eligible — it claims every position — so we
            //                    append its replacement and `jmp done` with no predicate.
            for (x_reg, y_reg, kind, active, window) in &regs {
                // The item's window predicate: `Some(in_win_reg)` for a regioned item,
                // `None` for a region-less item (whole source ⇒ always in window). `j` is
                // the compile-time position materialised into a register so it can be
                // compared against the runtime bounds.
                let in_win = match window {
                    None => None,
                    Some((start, end_bound)) => {
                        let jreg = self.fresh("_irmjr");
                        self.emit("const", Some(&jreg), vec![Operand::Int(j as i64)], "i64");
                        let ge = self.fresh("_irmge");
                        self.emit(
                            "cmp_ge",
                            Some(&ge),
                            vec![Operand::Var(jreg.clone()), Operand::Var(start.clone())],
                            "i64",
                        );
                        let lt = self.fresh("_irmlt");
                        self.emit(
                            "cmp_lt",
                            Some(&lt),
                            vec![Operand::Var(jreg), Operand::Var(end_bound.clone())],
                            "i64",
                        );
                        let inw = self.fresh("_irmin");
                        self.emit("and", Some(&inw), vec![Operand::Var(ge), Operand::Var(lt)], "i64");
                        Some(inw)
                    }
                };
                // `base` = eligibility BEFORE the LEADING active-gate. For `ALL`/`LEADING`
                // it AND-folds the search compare `c == x` under the window; for
                // `CHARACTERS` it is the window predicate ALONE (`None` = unconditionally
                // eligible).
                let base: Option<String> = match kind {
                    ReplaceKind::Characters => in_win,
                    _ => {
                        let eq = self.fresh("_irmeq");
                        self.emit(
                            "cmp_eq",
                            Some(&eq),
                            vec![
                                Operand::Var(c.clone()),
                                Operand::Var(x_reg.as_ref().unwrap().clone()),
                            ],
                            "i64",
                        );
                        match in_win {
                            None => Some(eq),
                            Some(inw) => {
                                let m = self.fresh("_irmm");
                                self.emit(
                                    "and",
                                    Some(&m),
                                    vec![Operand::Var(inw), Operand::Var(eq)],
                                    "i64",
                                );
                                Some(m)
                            }
                        }
                    }
                };
                // A `LEADING` item ALSO AND-gates on its `active` run flag; `ALL`/
                // `CHARACTERS` never read `active`, so their eligibility is `base` alone.
                let matched: Option<String> = match (kind, active) {
                    (ReplaceKind::Leading, Some(a)) => {
                        let m = self.fresh("_irmel");
                        self.emit(
                            "and",
                            Some(&m),
                            vec![Operand::Var(base.unwrap()), Operand::Var(a.clone())],
                            "i64",
                        );
                        Some(m)
                    }
                    _ => base,
                };
                // Emit the guarded append. When `matched` is `None` (an unconditional
                // `CHARACTERS` catch-all), the position is always claimed: append the
                // replacement and `jmp done` with no test and no `next` label (any later
                // chain link is then unreachable — a region-less `CHARACTERS` item shadows
                // everything after it in written order, exactly the oracle's
                // first-eligible-item semantics).
                let next = match matched {
                    Some(m) => {
                        let n = self.fresh("irm_next");
                        self.emit(
                            "jmp_if_false",
                            None,
                            vec![Operand::Var(m), Operand::Var(n.clone())],
                            "void",
                        );
                        Some(n)
                    }
                    None => None,
                };
                self.emit(
                    "str_concat",
                    Some(&result),
                    vec![Operand::Var(result.clone()), Operand::Var(y_reg.clone())],
                    "str",
                );
                self.emit("jmp", None, vec![Operand::Var(done.clone())], "void");
                if let Some(n) = next {
                    self.emit("label", None, vec![Operand::Var(n)], "void");
                }
            }
            // No item matched — append the ORIGINAL character `S[j, j+1)` unchanged.
            let jc1 = self.str_index(j as i64 + 1);
            let orig = self.fresh("_irmorig");
            self.emit(
                "str_slice",
                Some(&orig),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc), Operand::Var(jc1)],
                "str",
            );
            self.emit(
                "str_concat",
                Some(&result),
                vec![Operand::Var(result.clone()), Operand::Var(orig)],
                "str",
            );
            // `done`: the per-position CONVERGENCE point BOTH a match (via `jmp done`) and
            // the no-match fall-through reach — it hosts the LEADING run updates, so every
            // LEADING item's run flag is decayed INDEPENDENTLY of which item (if any) won
            // this position. We RECOMPUTE `eq`/`in_win` per leading item (the chain's
            // registers may not have been reached on an early `jmp done`), mirroring the
            // oracle's separate active-update pass and the tally side's `cont` section.
            self.emit("label", None, vec![Operand::Var(done)], "void");
            for (x_reg, _, kind, active, window) in &regs {
                // Only `LEADING` items carry a run flag; `ALL` and `CHARACTERS` are skipped
                // (a `CHARACTERS` item has NO search and no run — it never decays anything),
                // exactly mirroring the oracle's active-update pass.
                let (ReplaceKind::Leading, Some(a)) = (*kind, active) else { continue };
                // eq2 = (c == x).
                let eq2 = self.fresh("_irmeq2");
                self.emit(
                    "cmp_eq",
                    Some(&eq2),
                    vec![Operand::Var(c.clone()), Operand::Var(x_reg.as_ref().unwrap().clone())],
                    "i64",
                );
                match window {
                    // No region: active := active AND eq2 — once an (in-window ⇒
                    // whole-source) mismatch clears it, it sticks at 0, so the run never
                    // revives. Byte-identical to the single-item LEADING decay.
                    None => {
                        self.emit(
                            "and",
                            Some(a),
                            vec![Operand::Var(a.clone()), Operand::Var(eq2)],
                            "i64",
                        );
                    }
                    // With a region, decay ONLY on an IN-WINDOW mismatch:
                    //   active := active AND (eq2 OR NOT in_win)
                    // A position OUTSIDE the window has `NOT in_win == 1`, so the OR is 1
                    // and `active` is left unchanged — characters before the window neither
                    // start nor break the run, anchoring it at the window start (identical
                    // to the single-item REPLACING LEADING+region decay). `j` is the
                    // compile-time position materialised into a register.
                    Some((start, end_bound)) => {
                        let jreg = self.fresh("_irmjr2");
                        self.emit("const", Some(&jreg), vec![Operand::Int(j as i64)], "i64");
                        let ge = self.fresh("_irmge2");
                        self.emit(
                            "cmp_ge",
                            Some(&ge),
                            vec![Operand::Var(jreg.clone()), Operand::Var(start.clone())],
                            "i64",
                        );
                        let lt = self.fresh("_irmlt2");
                        self.emit(
                            "cmp_lt",
                            Some(&lt),
                            vec![Operand::Var(jreg), Operand::Var(end_bound.clone())],
                            "i64",
                        );
                        let inw = self.fresh("_irmin2");
                        self.emit("and", Some(&inw), vec![Operand::Var(ge), Operand::Var(lt)], "i64");
                        let zero = self.fresh("_irmz");
                        self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                        let not_in = self.fresh("_irmni");
                        self.emit(
                            "cmp_eq",
                            Some(&not_in),
                            vec![Operand::Var(inw), Operand::Var(zero)],
                            "i64",
                        );
                        let keep = self.fresh("_irmkeep");
                        self.emit(
                            "or",
                            Some(&keep),
                            vec![Operand::Var(eq2), Operand::Var(not_in)],
                            "i64",
                        );
                        self.emit(
                            "and",
                            Some(a),
                            vec![Operand::Var(a.clone()), Operand::Var(keep)],
                            "i64",
                        );
                    }
                }
            }
        }

        // source := result. `result` is exactly W chars (each position emitted one
        // piece), so this is the same fixed-width image the oracle stores. Copy
        // through an empty concat so `s_reg` (read all through the loop) is only
        // overwritten now, after the last read.
        let empty = self.fresh("_irmempty");
        self.emit("str_const", Some(&empty), vec![Operand::Str(String::new())], "str");
        self.emit(
            "str_concat",
            Some(s_reg),
            vec![Operand::Var(result), Operand::Var(empty)],
            "str",
        );
        Ok(())
    }

    /// `INSPECT source TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|
    /// AFTER} q] …` — one INSPECT whose SINGLE counter carries TWO OR MORE `FOR ALL`
    /// items, each with its OWN optional `{BEFORE|AFTER} x` window, counted in a SINGLE
    /// left-to-right pass with FIRST-MATCH-PER-POSITION into the shared counter. The
    /// count-side analogue of [`Self::emit_inspect_replacing_multi`], and the
    /// multi-delimiter analogue of [`Self::emit_inspect_tallying`].
    ///
    /// The items form an ordered priority list, each carrying its OWN window. At each
    /// source position they are tried IN WRITTEN ORDER and the FIRST item whose window
    /// CONTAINS the position AND whose delimiter matches bumps the shared count by 1,
    /// then the scan advances (a single-char match is a normal one-position step). The
    /// per-position `break` is what makes DUPLICATE items NOT double-count a position:
    /// `FOR ALL "a" ALL "a"` over `"aa"` adds 2 — each `a` is counted once by the first
    /// item, the second never fires at that position — exactly matching the oracle's
    /// `exec_inspect_tally_multi`.
    ///
    /// PER-ITEM WINDOWS: each item's optional region is derived by the SAME
    /// [`Self::emit_inspect_region_window`] the single-item region emitter (and the
    /// merged multi-item REPLACING-region emitter) uses, so both engines narrow to
    /// identical `[start, end)` slices. The runtime source length is materialised ONCE
    /// (needed by the window helper), before the loop. A region-less item has NO window
    /// emitted — its per-position compare folds down to the delimiter equality alone
    /// (whole-source window).
    ///
    /// Unlike the REPLACING emitter (which rebuilds a FIXED-width string and so unrolls
    /// `0..width` at compile time), the tally does not build a string, so — like
    /// [`Self::emit_inspect_tallying`] — it emits a genuine RUNTIME loop over
    /// `len = str_len(S)`, and the per-item window compare gates on the RUNTIME position
    /// register `j`:
    ///
    /// ```text
    ///   len = str_len(S)
    ///   [start_k, end_k) = region_window(item_k)      # per item with a region
    ///   active_k = 1                                   # per LEADING item only
    ///   cnt = 0;  j = 0
    /// top:  if j >= len jmp end
    ///       c = S[j]
    ///       # tally decision — first ELIGIBLE item wins, then stop:
    ///       if (s0<=j<e0) && c==D0 && (ALL_0 || active_0) { cnt += 1; jmp cont }
    ///       if (s1<=j<e1) && c==D1 && (ALL_1 || active_1) { cnt += 1; jmp cont }
    ///       …                                    # after the last: fall through (no match)
    /// cont: # update EVERY LEADING item's run flag, independent of which item tallied:
    ///       for each LEADING item k:  if (sk<=j<ek) && c != Dk { active_k = 0 }
    ///       j = j + 1;  jmp top
    /// end:
    ///   counter := (counter_value + cnt) reduced into counter's picture
    /// ```
    ///
    /// A `LEADING` item counts only its CONSECUTIVE run anchored at its window start: it
    /// is eligible only while `active_k` is still `1` (every prior in-window position
    /// equalled `Dk`), and its run breaks at the FIRST in-window mismatch. The active
    /// update at `cont` runs for EVERY leading item regardless of which item tallied —
    /// so a matching char keeps a run alive even if a higher-priority item claimed that
    /// position; positions outside the window neither begin nor break the run. This is
    /// the runtime-loop analogue of the single-item `emit_inspect_replacing` LEADING
    /// active flag (whose loop is a compile-time unroll instead).
    ///
    /// The count folds into the counter with the SAME numeric-store path (`store_scaled`,
    /// which mirrors the oracle's `store_result`) the single-item tally uses, so a
    /// compiled program matches `cobol-runtime`'s `exec_inspect_tally_multi`
    /// byte-for-byte. Each delimiter reduces to a byte code via the SAME
    /// `single_delim_code` the single-item path uses, so a multi-character/figurative/
    /// wider/numeric delimiter is rejected identically.
    ///
    /// Non-ASCII-clean: the tally only COUNTS (it never `str_slice`s the source into a
    /// new string), and each window is content-defined (bounded by the first ASCII
    /// region delimiter), so this byte-index scan and the oracle's char-index scan
    /// cover the SAME substring and count the SAME ASCII matches even on a non-ASCII
    /// source — no UTF-8-boundary trap.
    ///
    /// The read-side `inspect_tally_multi` has already rejected CHARACTERS items and
    /// SEVERAL counters, so here every item is a single-char `ALL` OR `LEADING` delimiter
    /// with an optional region, under one counter.
    fn emit_inspect_tally_multi(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
    ) -> Result<(), CompileError> {
        // The counter name and the written-order `(delim_node, kind, region)` items (the
        // reader has enforced the one-counter scope bound; each item is `ALL`, `LEADING`,
        // or `CHARACTERS` and MAY carry its own region — a `CHARACTERS` item's `delim_node`
        // is `None`).
        let (counter_name, item_nodes) = inspect_tally_multi(verb)?;

        // The counter must be an unsigned integer numeric item (`PIC 9(n)`) — the SAME
        // validation `emit_inspect_tallying` performs for the single-item form.
        let cidx = self.numeric_index(&counter_name)?;
        let (int_digits, dec_digits) = self.numeric_dims(cidx);
        if dec_digits != 0 {
            return Err(CompileError::Unsupported(format!(
                "INSPECT TALLYING into a non-integer counter {counter_name} is a later rung"
            )));
        }
        if self.item_signed(cidx) {
            return Err(CompileError::Unsupported(format!(
                "INSPECT TALLYING into a signed counter {counter_name} is a later rung"
            )));
        }
        let counter_reg = self.items[cidx].reg.clone();

        // len = str_len(S). Materialised ONCE up front — the per-item window helper
        // needs it, and the loop below reuses it for the `j >= len` bound. (The tally
        // does not build a fixed-width string, so the length is a genuine runtime value.)
        let len = self.fresh("_itmlen");
        self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");

        // Reduce each item to a `(delimiter byte code, kind, active, window)` tuple, IN
        // ORDER, so the per-position chain walks them written-first. The window (when
        // present) is derived by the SAME `emit_inspect_region_window` the single-item
        // region emitter uses, so both engines narrow to identical slices. A `CHARACTERS`
        // item has NO delimiter (`None`) — it is the always-eligible catch-all, so we skip
        // `single_delim_code` (nothing to validate/compare) and allocate no `active`. A
        // `LEADING` item ALSO allocates a runtime `active` register (i64, init `1` = its
        // run is still alive), materialised HERE — before the loop — exactly like the
        // single-item `emit_inspect_replacing` LEADING active flag. Resolving all of them
        // BEFORE the loop means an invalid delimiter/region delimiter aborts without
        // emitting the loop, mirroring the oracle's resolve-first order.
        let mut regs: Vec<ResolvedTallyLeadingItem> = Vec::with_capacity(item_nodes.len());
        for (dn, kind, region) in item_nodes {
            let d_reg = match kind {
                TallyKind::Characters => None,
                _ => Some(self.single_delim_code(dn.unwrap(), "INSPECT")?),
            };
            let window = self.emit_inspect_region_window(region, s_reg, &len)?;
            let active = if matches!(kind, TallyKind::Leading) {
                let a = self.fresh("_itmact");
                self.emit("const", Some(&a), vec![Operand::Int(1)], "i64");
                Some(a)
            } else {
                None
            };
            regs.push((d_reg, kind, active, window));
        }

        // cnt = 0; j = 0. A genuine runtime loop over the source positions.
        let cnt = self.fresh("_itmc");
        self.emit("const", Some(&cnt), vec![Operand::Int(0)], "i64");
        let j = self.fresh("_itmj");
        self.emit("const", Some(&j), vec![Operand::Int(0)], "i64");

        let top = self.fresh("itm_top");
        let end = self.fresh("itm_end");
        // `cont` is the per-position CONVERGENCE point: BOTH a tally match (via `jmp cont`)
        // and a no-match fall-through reach it, and it hosts the LEADING run updates before
        // the j-advance — so every LEADING item's run flag is updated INDEPENDENTLY of
        // which item (if any) tallied this position.
        let cont = self.fresh("itm_cont");
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        // if j >= len jmp end.
        let ge = self.fresh("_itmge");
        self.emit(
            "cmp_ge",
            Some(&ge),
            vec![Operand::Var(j.clone()), Operand::Var(len.clone())],
            "i64",
        );
        self.emit("jmp_if_true", None, vec![Operand::Var(ge), Operand::Var(end.clone())], "void");
        // c = S[j] — read ONCE, then compared against each delimiter in written order.
        let c = self.fresh("_itmc0");
        self.emit(
            "str_index",
            Some(&c),
            vec![Operand::Var(s_reg.to_string()), Operand::Var(j.clone())],
            "i64",
        );
        // The ordered TALLY-DECISION chain: item 0, then 1, … On the FIRST eligible item
        // we bump `cnt` and jump to `cont` (the LEADING-update + j-advance section),
        // skipping the rest of the chain — first-match-per-position, so a position is
        // counted at most once even if several (or duplicate) items would match it. A
        // miss jumps to the next link (`next`); after the last link's `next` we fall
        // through to `cont` with no bump (matched no item). Eligibility by kind:
        //   * `ALL`        — `(start <= j < end) AND c == D`;
        //   * `LEADING`    — as `ALL`, ALSO requiring its `active` run flag still `1`
        //                    (every prior in-window position equalled its delimiter);
        //   * `CHARACTERS` — `(start <= j < end)` ALONE (the always-eligible catch-all:
        //                    NO delimiter compare). A region-less `CHARACTERS` item is
        //                    UNCONDITIONALLY eligible — it claims every position — so we
        //                    emit an unconditional bump + `jmp cont` with no predicate.
        for (d_reg, kind, active, window) in &regs {
            // The item's window predicate: `Some(in_win_reg)` for a regioned item,
            // `None` for a region-less item (whole source ⇒ always in window). `j` is the
            // RUNTIME loop position register, compared directly against the runtime bounds
            // (no compile-time `const` needed, unlike the REPLACING unroll whose `j` is a
            // compile-time constant).
            let in_win = match window {
                None => None,
                Some((start, end_bound)) => {
                    let ge2 = self.fresh("_itmge2");
                    self.emit(
                        "cmp_ge",
                        Some(&ge2),
                        vec![Operand::Var(j.clone()), Operand::Var(start.clone())],
                        "i64",
                    );
                    let lt = self.fresh("_itmlt");
                    self.emit(
                        "cmp_lt",
                        Some(&lt),
                        vec![Operand::Var(j.clone()), Operand::Var(end_bound.clone())],
                        "i64",
                    );
                    let inw = self.fresh("_itmin");
                    self.emit("and", Some(&inw), vec![Operand::Var(ge2), Operand::Var(lt)], "i64");
                    Some(inw)
                }
            };
            // `base` = eligibility BEFORE the LEADING active-gate. For `ALL`/`LEADING` it
            // AND-folds the delimiter compare `c == D` under the window; for `CHARACTERS`
            // it is the window predicate ALONE (`None` = unconditionally eligible).
            let base: Option<String> = match kind {
                TallyKind::Characters => in_win,
                _ => {
                    let eq = self.fresh("_itmeq");
                    self.emit(
                        "cmp_eq",
                        Some(&eq),
                        vec![
                            Operand::Var(c.clone()),
                            Operand::Var(d_reg.as_ref().unwrap().clone()),
                        ],
                        "i64",
                    );
                    match in_win {
                        None => Some(eq),
                        Some(inw) => {
                            let m = self.fresh("_itmm");
                            self.emit(
                                "and",
                                Some(&m),
                                vec![Operand::Var(inw), Operand::Var(eq)],
                                "i64",
                            );
                            Some(m)
                        }
                    }
                }
            };
            // A `LEADING` item ALSO AND-gates on its `active` run flag; `ALL`/`CHARACTERS`
            // never read `active`, so their eligibility is `base` alone.
            let matched: Option<String> = match (kind, active) {
                (TallyKind::Leading, Some(a)) => {
                    let m = self.fresh("_itmel");
                    self.emit(
                        "and",
                        Some(&m),
                        vec![Operand::Var(base.unwrap()), Operand::Var(a.clone())],
                        "i64",
                    );
                    Some(m)
                }
                _ => base,
            };
            // Emit the guarded bump. When `matched` is `None` (an unconditional
            // `CHARACTERS` catch-all), the position is always claimed: bump and `jmp cont`
            // with no test and no `next` label (any later chain link is then unreachable —
            // a region-less `CHARACTERS` item shadows everything after it in written
            // order, exactly the oracle's first-eligible-item semantics).
            let next = match matched {
                Some(m) => {
                    let n = self.fresh("itm_next");
                    self.emit(
                        "jmp_if_false",
                        None,
                        vec![Operand::Var(m), Operand::Var(n.clone())],
                        "void",
                    );
                    Some(n)
                }
                None => None,
            };
            let one = self.fresh("_itm1");
            self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
            self.emit("add", Some(&cnt), vec![Operand::Var(cnt.clone()), Operand::Var(one)], "i64");
            self.emit("jmp", None, vec![Operand::Var(cont.clone())], "void");
            if let Some(n) = next {
                self.emit("label", None, vec![Operand::Var(n)], "void");
            }
        }
        // cont: update EVERY LEADING item's run, then j = j + 1; jmp top. The updates run
        // whether or not a position tallied and INDEPENDENTLY of which item tallied — a
        // matching char keeps a run alive even if a higher-priority item claimed it. Each
        // LEADING run breaks at the FIRST IN-WINDOW mismatch; positions outside the window
        // never touch `active`, so the run is anchored at the window start. We RECOMPUTE
        // `eq`/`in_win` per leading item here (the chain's registers may not have been
        // reached on an early `jmp cont`), mirroring the oracle's separate active-update
        // pass and the single-item `emit_inspect_replacing` LEADING decay.
        self.emit("label", None, vec![Operand::Var(cont)], "void");
        for (d_reg, kind, active, window) in &regs {
            // Only `LEADING` items carry a run flag; `ALL` and `CHARACTERS` are skipped
            // (a `CHARACTERS` item has NO delimiter and no run — it never decays anything),
            // exactly mirroring the oracle's active-update pass.
            let (TallyKind::Leading, Some(a)) = (*kind, active) else { continue };
            // eq2 = (c == D).
            let eq2 = self.fresh("_itmeq2");
            self.emit(
                "cmp_eq",
                Some(&eq2),
                vec![Operand::Var(c.clone()), Operand::Var(d_reg.as_ref().unwrap().clone())],
                "i64",
            );
            match window {
                // No region: active := active AND eq2 — once an (in-window ⇒ whole-source)
                // mismatch clears it, it sticks at 0, so the run never revives.
                None => {
                    self.emit(
                        "and",
                        Some(a),
                        vec![Operand::Var(a.clone()), Operand::Var(eq2)],
                        "i64",
                    );
                }
                // With a region, decay ONLY on an IN-WINDOW mismatch:
                //   active := active AND (eq2 OR NOT in_win)
                // A position OUTSIDE the window has `NOT in_win == 1`, so the OR is 1 and
                // `active` is left unchanged — characters before the window neither start
                // nor break the run, anchoring it at the window start (identical to the
                // single-item REPLACING LEADING+region decay).
                Some((start, end_bound)) => {
                    let ge3 = self.fresh("_itmge3");
                    self.emit(
                        "cmp_ge",
                        Some(&ge3),
                        vec![Operand::Var(j.clone()), Operand::Var(start.clone())],
                        "i64",
                    );
                    let lt2 = self.fresh("_itmlt2");
                    self.emit(
                        "cmp_lt",
                        Some(&lt2),
                        vec![Operand::Var(j.clone()), Operand::Var(end_bound.clone())],
                        "i64",
                    );
                    let inw = self.fresh("_itmin2");
                    self.emit("and", Some(&inw), vec![Operand::Var(ge3), Operand::Var(lt2)], "i64");
                    let zero = self.fresh("_itmz");
                    self.emit("const", Some(&zero), vec![Operand::Int(0)], "i64");
                    let not_in = self.fresh("_itmni");
                    self.emit(
                        "cmp_eq",
                        Some(&not_in),
                        vec![Operand::Var(inw), Operand::Var(zero)],
                        "i64",
                    );
                    let keep = self.fresh("_itmkeep");
                    self.emit("or", Some(&keep), vec![Operand::Var(eq2), Operand::Var(not_in)], "i64");
                    self.emit(
                        "and",
                        Some(a),
                        vec![Operand::Var(a.clone()), Operand::Var(keep)],
                        "i64",
                    );
                }
            }
        }
        let one2 = self.fresh("_itm1b");
        self.emit("const", Some(&one2), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&j), vec![Operand::Var(j.clone()), Operand::Var(one2)], "i64");
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(end)], "void");

        // counter := counter_value + cnt, reduced into the counter's picture — the
        // exact numeric-store ADD (and the single-item tally) uses, so this matches the
        // oracle's `store_result(counter, counter + cnt)`.
        let sum = self.fresh("_itmsum");
        self.emit("add", Some(&sum), vec![Operand::Var(counter_reg), Operand::Var(cnt)], "i64");
        self.store_scaled(&counter_name, &sum, 0, int_digits + 1, false)
    }

    /// `INSPECT src TALLYING c1 FOR ALL a [{BEFORE|AFTER} p] [ALL b …] c2 FOR ALL d …` —
    /// several counters, each with its OWN delimiter list, and each delimiter item now
    /// carrying its OWN optional `{BEFORE|AFTER}` region window, folded through ONE combined
    /// priority list in a SINGLE runtime pass. This generalises
    /// [`Self::emit_inspect_tally_multi`] from one shared counter to a list of `(counter,
    /// delimiter, window)` entries where the matched entry's OWN counter is bumped.
    ///
    /// ISO COMBINED-PRIORITY-LIST-ACROSS-COUNTERS semantics (the crux): all delimiters of
    /// all groups, flattened in WRITTEN ORDER (group 1's items first, then group 2's, …),
    /// form ONE ordered priority list, each entry carrying its item's `[start, end)`
    /// window. At each source position the flattened list is walked in order and the FIRST
    /// entry whose window contains the position AND whose delimiter matches bumps ITS OWN
    /// group's accumulator, then the scan advances (single-char ⇒ a normal one-position
    /// step). The per-position `break` (a `jmp` to `cont`) means an earlier group's
    /// (in-window) delimiter CONSUMES the position — a character it claims NEVER reaches a
    /// later group's delimiter — so `"aa" TALLYING C1 FOR ALL "a" C2 FOR ALL "a"` gives
    /// C1 += 2, C2 += 0. A position matching no in-window delimiter falls through to `cont`
    /// with no bump.
    ///
    /// ```text
    ///   acc_0 = 0; acc_1 = 0; …            # one accumulator per GROUP
    ///   len = str_len(S)                   # windows + the j-bound both need it
    ///   # per flat entry: [start, end) window (region-less item → whole source)
    ///   j = 0
    /// top:  if j >= len jmp end
    ///       c = S[j]
    ///       if (s0 <= j < e0) AND c == flat[0].delim { acc[flat[0].group] += 1; jmp cont }
    ///       if (s1 <= j < e1) AND c == flat[1].delim { acc[flat[1].group] += 1; jmp cont }
    ///       …                                                 # first in-window match wins
    /// cont: j = j + 1;  jmp top
    /// end:
    ///   for each group g:  counter_g := counter_g + acc_g   # INSPECT ADDS; never clears
    /// ```
    ///
    /// Each item's `[start, end)` window is derived by the SAME `emit_inspect_region_window`
    /// the single-item region emitter uses (a region-less item folds to `eq` alone — the
    /// whole-source window), materialised BEFORE the loop, so both engines narrow to
    /// identical slices. Each group keeps its OWN accumulator (indexed by GROUP, not by
    /// counter name), so two groups that name the SAME counter stay separate through the
    /// loop and are BOTH added into that one item afterwards. The final adds run
    /// sequentially and each reads the counter's storage register FRESH
    /// (`self.items[idx].reg`, which `store_scaled` mutates via `mov`), so a shared counter
    /// accumulates both shares correctly — mirroring the oracle's per-add `named_decimal`
    /// re-read. Each delimiter reduces to a byte code via the SAME `single_delim_code` the
    /// single-item path uses, and each counter is validated unsigned-integer exactly as
    /// `emit_inspect_tally_multi` validates its lone counter, so the compiled program
    /// matches `cobol-runtime`'s `exec_inspect_tally_counters` byte-for-byte and the
    /// accept/reject sets stay co-total. The read-side `inspect_tally_counters` has already
    /// rejected LEADING/CHARACTERS items.
    fn emit_inspect_tally_counters(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
    ) -> Result<(), CompileError> {
        // The counter names and, per group, the written-order delimiter nodes (the reader
        // has enforced the `ALL`-only, no-region scope bound on every item of every group).
        let groups = inspect_tally_counters(verb)?;

        // Validate EVERY counter (unsigned integer `PIC 9(n)`) and capture its item index
        // FIRST — resolving all counters and all delimiters before emitting the loop means
        // an invalid group aborts with nothing emitted. `int_digits` is remembered per
        // group for the final store's overflow bound.
        let mut counter_info: Vec<(String, usize, usize)> = Vec::with_capacity(groups.len());
        for (counter_name, _) in &groups {
            let cidx = self.numeric_index(counter_name)?;
            let (int_digits, dec_digits) = self.numeric_dims(cidx);
            if dec_digits != 0 {
                return Err(CompileError::Unsupported(format!(
                    "INSPECT TALLYING into a non-integer counter {counter_name} is a later rung"
                )));
            }
            if self.item_signed(cidx) {
                return Err(CompileError::Unsupported(format!(
                    "INSPECT TALLYING into a signed counter {counter_name} is a later rung"
                )));
            }
            counter_info.push((counter_name.clone(), cidx, int_digits));
        }

        // One accumulator register per GROUP, all init 0 (kept separate even when two
        // groups share a counter name — they are summed into that one item at the end).
        let mut accs: Vec<String> = Vec::with_capacity(groups.len());
        for _ in &groups {
            let acc = self.fresh("_itcacc");
            self.emit("const", Some(&acc), vec![Operand::Int(0)], "i64");
            accs.push(acc);
        }

        // len = str_len(S). Materialised ONCE up front — the per-item window helper needs
        // it, and the loop below reuses it for the `j >= len` bound. (The tally builds no
        // fixed-width string, so the length is a genuine runtime value.)
        let len = self.fresh("_itclen");
        self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");

        // Flatten every delimiter to `(group_index, byte_code_reg, window)` in WRITTEN
        // ORDER, so the per-position chain walks all groups' items group-1-first. Each
        // item's window (when present) is derived by the SAME `emit_inspect_region_window`
        // the single-item region emitter uses, so both engines narrow to identical slices.
        // Resolving every delimiter AND window up front (via the SAME `single_delim_code`
        // the single-item path uses, so an invalid delimiter/region delimiter rejects
        // identically) means a bad operand aborts before the loop, mirroring the oracle.
        let mut flat: Vec<FlatCounterDelim> = Vec::new();
        for (gi, (_counter, item_nodes)) in groups.iter().enumerate() {
            for (dn, region) in item_nodes {
                let d_reg = self.single_delim_code(dn, "INSPECT")?;
                let window = self.emit_inspect_region_window(*region, s_reg, &len)?;
                flat.push((gi, d_reg, window));
            }
        }

        // j = 0. A genuine runtime loop (the tally builds no string).
        let j = self.fresh("_itcj");
        self.emit("const", Some(&j), vec![Operand::Int(0)], "i64");

        let top = self.fresh("itc_top");
        let end = self.fresh("itc_end");
        let cont = self.fresh("itc_cont");
        self.emit("label", None, vec![Operand::Var(top.clone())], "void");
        // if j >= len jmp end.
        let ge = self.fresh("_itcge");
        self.emit(
            "cmp_ge",
            Some(&ge),
            vec![Operand::Var(j.clone()), Operand::Var(len.clone())],
            "i64",
        );
        self.emit("jmp_if_true", None, vec![Operand::Var(ge), Operand::Var(end.clone())], "void");
        // c = S[j] — read ONCE, then compared against each flattened delimiter in order.
        let c = self.fresh("_itcc0");
        self.emit(
            "str_index",
            Some(&c),
            vec![Operand::Var(s_reg.to_string()), Operand::Var(j.clone())],
            "i64",
        );
        // The ordered chain across ALL groups: on the FIRST match bump THAT group's
        // accumulator and jump to `cont` (the j-advance), skipping the rest of the chain —
        // first-match-wins across counters, so an earlier group consumes the position and a
        // later group never sees it. A miss jumps to the next link; after the last link's
        // `next` we fall through to `cont` with no bump (matched no delimiter).
        for (gi, d_reg, window) in &flat {
            let eq = self.fresh("_itceq");
            self.emit(
                "cmp_eq",
                Some(&eq),
                vec![Operand::Var(c.clone()), Operand::Var(d_reg.clone())],
                "i64",
            );
            // Gate the compare by this item's window: `matched = (start <= j < end) AND
            // (c == D)`. `j` is the RUNTIME loop position register, compared directly
            // against the runtime bounds. A region-less item folds down to `eq` alone (no
            // window emitted → whole-source window) — byte-identical to the old lowering.
            let matched = match window {
                None => eq,
                Some((start, end_bound)) => {
                    let ge2 = self.fresh("_itcge2");
                    self.emit(
                        "cmp_ge",
                        Some(&ge2),
                        vec![Operand::Var(j.clone()), Operand::Var(start.clone())],
                        "i64",
                    );
                    let lt = self.fresh("_itclt");
                    self.emit(
                        "cmp_lt",
                        Some(&lt),
                        vec![Operand::Var(j.clone()), Operand::Var(end_bound.clone())],
                        "i64",
                    );
                    let inw = self.fresh("_itcin");
                    self.emit("and", Some(&inw), vec![Operand::Var(ge2), Operand::Var(lt)], "i64");
                    let m = self.fresh("_itcm");
                    self.emit("and", Some(&m), vec![Operand::Var(inw), Operand::Var(eq)], "i64");
                    m
                }
            };
            let next = self.fresh("itc_next");
            self.emit(
                "jmp_if_false",
                None,
                vec![Operand::Var(matched), Operand::Var(next.clone())],
                "void",
            );
            let one = self.fresh("_itc1");
            self.emit("const", Some(&one), vec![Operand::Int(1)], "i64");
            self.emit(
                "add",
                Some(&accs[*gi]),
                vec![Operand::Var(accs[*gi].clone()), Operand::Var(one)],
                "i64",
            );
            self.emit("jmp", None, vec![Operand::Var(cont.clone())], "void");
            self.emit("label", None, vec![Operand::Var(next)], "void");
        }
        // cont: j = j + 1; jmp top.
        self.emit("label", None, vec![Operand::Var(cont)], "void");
        let one2 = self.fresh("_itc1b");
        self.emit("const", Some(&one2), vec![Operand::Int(1)], "i64");
        self.emit("add", Some(&j), vec![Operand::Var(j.clone()), Operand::Var(one2)], "i64");
        self.emit("jmp", None, vec![Operand::Var(top)], "void");
        self.emit("label", None, vec![Operand::Var(end)], "void");

        // For each group: counter := counter_value + acc, reduced into the counter's
        // picture. Read the counter's storage register FRESH here (re-fetch by index) so a
        // shared counter's second group adds on top of the first group's already-stored
        // value — the same ADD-into-counter store path the single-item tally uses.
        for ((counter_name, cidx, int_digits), acc) in counter_info.iter().zip(accs.iter()) {
            let counter_reg = self.items[*cidx].reg.clone();
            let sum = self.fresh("_itcsum");
            self.emit(
                "add",
                Some(&sum),
                vec![Operand::Var(counter_reg), Operand::Var(acc.clone())],
                "i64",
            );
            self.store_scaled(counter_name, &sum, 0, int_digits + 1, false)?;
        }
        Ok(())
    }

    /// `INSPECT source CONVERTING from TO to` — translate the alphanumeric `source`
    /// through a per-character **translation table** built from the two EQUAL-length
    /// string literals `from` and `to`: at each source position the character is
    /// replaced by `to[k]` where `k` is the FIRST index at which it equals `from[k]`
    /// (leftmost wins if `from` repeats a character), and left unchanged if it
    /// matches no `from` character. Both literals are single-byte (ASCII) this rung,
    /// so — exactly like [`Self::emit_inspect_replacing`] — the width `W` is
    /// unchanged and the result is a per-position map that we UNROLL over the
    /// compile-time-known `W`:
    ///
    /// ```text
    ///   result = ""
    ///   for j in 0..W:                       # W is known at compile time
    ///       c = S[j]
    ///       if      c == from[0]   result ++= to[0]
    ///       else if c == from[1]   result ++= to[1]
    ///       …                                # first match wins (leftmost k)
    ///       else                   result ++= S[j, j+1)   # unchanged
    ///   source := result                     # exactly W chars, width unchanged
    /// ```
    ///
    /// Each `from`/`to` operand is a string LITERAL or a data-name (`PIC X` item):
    ///
    ///   * a **literal** bakes its bytes at compile time — each `from[k]` a `const`
    ///     compare target, each `to[k]` a 1-character `str_const`;
    ///   * a **data-name** cannot be baked (its bytes live in the item's storage), so
    ///     its table entries become RUNTIME reads emitted ONCE, before the per-position
    ///     loop: `from[k] = str_index(item, k)` (a byte) and `to[k] =
    ///     str_slice(item, k, k+1)` (a 1-char string). These reads are LOOP-INVARIANT —
    ///     the `from`/`to` item does not change while we translate — so hoisting them
    ///     out of the loop is not just an optimisation but the correctness invariant:
    ///     it also guarantees a `from`/`to` that ALIASES the source is read while the
    ///     source still holds its ORIGINAL bytes (the source register is overwritten
    ///     only at the very end), matching the oracle's up-front table build.
    ///
    /// Whether a table entry is a baked `const` or a runtime read, the per-position
    /// first-match-wins chain below is byte-IDENTICAL — it consumes the `from_consts`/
    /// `to_consts` registers the same way — so the two operand kinds share one lowering.
    /// The chain mirrors the oracle's char→char map (earliest `from` occurrence wins),
    /// so the compiled program is byte-identical to `cobol-runtime`'s CONVERTING path
    /// on ASCII operands. An unequal-length pair (each side's length is the literal's
    /// char count OR the item's declared width, all compile-time), a non-ASCII LITERAL,
    /// and a figurative/reference-modified `from`/`to` are clean later-rung
    /// `Unsupported`s. A non-ASCII BYTE in a data-name item's runtime storage cannot be
    /// (and is not) statically rejected — it is the pre-existing byte-vs-char operand
    /// chip shared with the literal-source scans, so the ASCII case is byte-identical
    /// and non-ASCII item content stays that shared chip. A numeric/group item AS
    /// `from`/`to` is rejected identically to the oracle.
    ///
    /// An optional `{BEFORE|AFTER} x` region narrows the translation to a sub-slice
    /// of the source. When present we reuse [`Self::emit_inspect_region_window`] —
    /// the SAME window the TALLYING and REPLACING sides emit — to derive `[start,
    /// end)` over the ORIGINAL source, then at each unrolled position `j` translate
    /// through the table iff `start <= j < end`; a position outside the window keeps
    /// its original character. With NO region the extra guard folds away and the
    /// emitted unroll is byte-identical to the pre-region CONVERTING lowering.
    fn emit_inspect_converting(
        &mut self,
        verb: &GrammarASTNode,
        s_reg: &str,
        width: usize,
    ) -> Result<(), CompileError> {
        // The `CONVERTING from TO to [{BEFORE|AFTER} x]` phrase. Each `from`/`to`
        // resolves to a LITERAL (its bytes baked now) or a data-name ITEM (its bytes
        // read at run time), decided by `converting_operand`.
        let (from_node, to_node, region) = inspect_converting_pair(verb)?;
        let from = self.converting_operand(from_node, "from")?;
        let to = self.converting_operand(to_node, "to")?;
        // The table pairs `from[k]` with `to[k]`, so the two must be equal length —
        // each side's length is a compile-time constant (a literal's char count OR an
        // item's declared width), so this check stays entirely at build time even when
        // the two sides mix a literal with a data-name.
        if from.len() != to.len() {
            return Err(CompileError::Unsupported(
                "INSPECT CONVERTING with unequal-length FROM/TO operands is a later rung".into(),
            ));
        }
        // This rung compares raw bytes (`str_index` yields a byte), so the table
        // characters must be single-byte ASCII for the byte compare to equal the char
        // map the oracle builds. A non-ASCII LITERAL is statically rejectable and stays
        // a later rung. A data-name's runtime bytes CANNOT be statically inspected — a
        // non-ASCII byte in an item's storage is the pre-existing byte-vs-char operand
        // chip (shared with the literal-source scans), so we cannot and do not reject
        // it here; the ASCII case is byte-identical to the oracle.
        for op in [&from, &to] {
            if let ConvOperand::Literal(s) = op {
                if !s.is_ascii() {
                    return Err(CompileError::Unsupported(
                        "INSPECT CONVERTING with a non-ASCII FROM/TO operand is a later rung".into(),
                    ));
                }
            }
        }

        // The optional `{BEFORE|AFTER} x` window `[start, end)`, derived over the
        // ORIGINAL source (before the unroll overwrites `s_reg`). We reuse the tally
        // side's `emit_inspect_region_window`, which needs the runtime length; with no
        // region nothing here is emitted and the per-position guard below folds away.
        let region_window = match region {
            None => None,
            Some(_) => {
                let len = self.fresh("_iclen");
                self.emit("str_len", Some(&len), vec![Operand::Var(s_reg.to_string())], "i64");
                self.emit_inspect_region_window(region, s_reg, &len)?
            }
        };

        // Build the table entries once, BEFORE the per-position loop (and before the
        // loop's final write-back to the source register). For a LITERAL these are
        // compile-time `const`(byte)/`str_const`(1-char) bakes; for a data-name they
        // are loop-invariant RUNTIME reads of the item's storage — see the helpers.
        // Either way the result is `W` `from`-compare registers (i64 bytes) and `W`
        // `to`-concat registers (1-char strings), consumed identically by the chain.
        let from_consts = self.converting_from_consts(&from);
        let to_consts = self.converting_to_consts(&to);

        // result = "" — the accumulator we build W characters into.
        let result = self.fresh("_icres");
        self.emit("str_const", Some(&result), vec![Operand::Str(String::new())], "str");

        for j in 0..width {
            // c = S[j]  (the source byte at this position), read ONCE and reused by
            // every table compare.
            let jc = self.str_index(j as i64);
            let c = self.fresh("_icc");
            self.emit(
                "str_index",
                Some(&c),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc.clone())],
                "i64",
            );
            let pos_done = self.fresh("ic_done");
            // When a `{BEFORE|AFTER}` region is active and this position lies OUTSIDE
            // the window `[start, end)`, skip the whole table chain and keep the
            // original character — the exact analogue of the REPLACING region guard.
            // `pos_orig` labels the "append the original source char" fall-through,
            // which the out-of-window jump targets directly. (No region ⇒ this whole
            // block is elided and the lowering is byte-identical to the original.)
            let pos_orig = region_window.as_ref().map(|_| self.fresh("ic_orig"));
            if let Some((start, end_bound)) = &region_window {
                // in_region = (j >= start) AND (j < end); j is the compile-time
                // constant for this unrolled position, materialised into a register.
                let jreg = self.fresh("_icjr");
                self.emit("const", Some(&jreg), vec![Operand::Int(j as i64)], "i64");
                let ge = self.fresh("_icge");
                self.emit(
                    "cmp_ge",
                    Some(&ge),
                    vec![Operand::Var(jreg.clone()), Operand::Var(start.clone())],
                    "i64",
                );
                let lt = self.fresh("_iclt");
                self.emit(
                    "cmp_lt",
                    Some(&lt),
                    vec![Operand::Var(jreg), Operand::Var(end_bound.clone())],
                    "i64",
                );
                let in_region = self.fresh("_icin");
                self.emit("and", Some(&in_region), vec![Operand::Var(ge), Operand::Var(lt)], "i64");
                // Out of window → jump straight to the original-append fall-through.
                self.emit(
                    "jmp_if_false",
                    None,
                    vec![Operand::Var(in_region), Operand::Var(pos_orig.clone().unwrap())],
                    "void",
                );
            }
            // First-match-wins chain over the table: on the earliest `from[k]` that
            // equals `c`, append `to[k]` and jump past the rest.
            for (fc, tc) in from_consts.iter().zip(to_consts.iter()) {
                let eq = self.fresh("_iceq");
                self.emit("cmp_eq", Some(&eq), vec![Operand::Var(c.clone()), Operand::Var(fc.clone())], "i64");
                let next_k = self.fresh("ic_next");
                self.emit("jmp_if_false", None, vec![Operand::Var(eq), Operand::Var(next_k.clone())], "void");
                self.emit(
                    "str_concat",
                    Some(&result),
                    vec![Operand::Var(result.clone()), Operand::Var(tc.clone())],
                    "str",
                );
                self.emit("jmp", None, vec![Operand::Var(pos_done.clone())], "void");
                self.emit("label", None, vec![Operand::Var(next_k)], "void");
            }
            // No table entry matched (or the position is outside the region window):
            // append the original source character.
            if let Some(po) = &pos_orig {
                self.emit("label", None, vec![Operand::Var(po.clone())], "void");
            }
            let jc1 = self.str_index(j as i64 + 1);
            let orig = self.fresh("_icorig");
            self.emit(
                "str_slice",
                Some(&orig),
                vec![Operand::Var(s_reg.to_string()), Operand::Var(jc), Operand::Var(jc1)],
                "str",
            );
            self.emit(
                "str_concat",
                Some(&result),
                vec![Operand::Var(result.clone()), Operand::Var(orig)],
                "str",
            );
            self.emit("label", None, vec![Operand::Var(pos_done)], "void");
        }

        // source := result — exactly W chars (each of the W pieces is one
        // character), the same fixed-width image the oracle stores. Copy through an
        // empty concat so the source register (read during the loop) is overwritten
        // only now, after the last read (no read-after-write hazard).
        let empty = self.fresh("_icempty");
        self.emit("str_const", Some(&empty), vec![Operand::Str(String::new())], "str");
        self.emit(
            "str_concat",
            Some(s_reg),
            vec![Operand::Var(result), Operand::Var(empty)],
            "str",
        );
        Ok(())
    }

    /// Resolve a CONVERTING `from`/`to` operand node into a [`ConvOperand`]. A string
    /// literal is carried by value (its bytes are baked later); a data-name resolves
    /// to its backing register and declared width via the shared item-index helper.
    /// A **constant** reference modification `base(start:len)` resolves to the slice
    /// register [`Self::ref_mod_slice`] materialises plus its compile-time-known
    /// length — a `ConvOperand::Item` in every downstream respect, since the slice
    /// register IS an alphanumeric string register of a static width (the const `len`,
    /// or `base_width - start + 1` when omitted). A numeric/group item (`item_index`
    /// rejects a group/undeclared name; the `Numeric` arm rejects a numeric item). A
    /// figurative constant SPACE / ZERO is accepted, mapped to the single-character
    /// literal `" "` / `"0"` — reducing to the `ConvOperand::Literal` path in every
    /// downstream respect. A numeric literal and a *computed* reference modification
    /// are clean later rungs — rejected with the SAME messages the oracle's
    /// `read_converting_operand`/`converting_operand_str` use, so both engines accept
    /// and reject the very same programs. `which` names the position
    /// (`"from"`/`"to"`) for the diagnostic.
    ///
    /// Takes `&mut self` because the const-refmod path emits the slice's `str_slice`
    /// (via `ref_mod_slice`) here, BEFORE the per-position loop — so a refmod whose
    /// base aliases the source is sliced from the original bytes, matching the
    /// oracle's up-front `converting_operand_str`.
    fn converting_operand(
        &mut self,
        op: &GrammarASTNode,
        which: &str,
    ) -> Result<ConvOperand, CompileError> {
        match read_operand(op)? {
            Operandy::Literal(Src::Str(s)) => Ok(ConvOperand::Literal(s)),
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Char { .. } => Ok(ConvOperand::Item {
                        reg: self.items[idx].reg.clone(),
                        width: self.items[idx].width(),
                    }),
                    ItemKind::Numeric { .. } => Err(CompileError::Unsupported(format!(
                        "INSPECT CONVERTING with a numeric {which} item is a later rung"
                    ))),
                }
            }
            Operandy::RefMod { base, start, len } => {
                // `ref_mod_slice` materialises the slice into `reg` and reports its
                // length: `SliceLen::Const(n)` for a literal:literal (or literal:)
                // refmod — the CONSTANT case this rung supports. Its Const/Runtime
                // split is co-total with the oracle's `const_ix` predicate (#67), so a
                // form one engine treats as constant the other does too. A Const slice
                // register + width `n` behave EXACTLY like a data-name's `(reg, width)`
                // in `converting_from_consts`/`converting_to_consts`, so we hand it back
                // as a `ConvOperand::Item`. A `SliceLen::Runtime` (any data-name index)
                // is a COMPUTED refmod — a run-time length the compile-time table
                // contract cannot carry — rejected here with the same message class the
                // oracle uses.
                let (reg, slice_len) = self.ref_mod_slice(&base, &start, &len)?;
                match slice_len {
                    SliceLen::Const(n) => Ok(ConvOperand::Item { reg, width: n }),
                    SliceLen::Runtime { .. } => Err(CompileError::Unsupported(format!(
                        "INSPECT CONVERTING with a computed reference-modified {which} operand is a later rung"
                    ))),
                }
            }
            Operandy::Literal(Src::Num(_)) => Err(CompileError::Unsupported(format!(
                "INSPECT CONVERTING with a numeric-literal {which} operand is a later rung"
            ))),
            // A figurative constant SPACE / ZERO reduces to a single-character
            // literal — SPACE→" " (0x20), ZERO→"0" (0x30), both ASCII — so it takes
            // the exact `ConvOperand::Literal` path a string literal does: the
            // equal-length check and the ASCII-literal guard below then apply
            // unchanged. Mirrors the oracle's `read_converting_operand`.
            Operandy::Literal(Src::Space) => Ok(ConvOperand::Literal(" ".into())),
            Operandy::Literal(Src::Zero) => Ok(ConvOperand::Literal("0".into())),
        }
    }

    /// Bake the `from[0..len]` compare bytes (i64 registers) for a CONVERTING
    /// operand. A LITERAL bakes one compile-time `const` per byte; a DATA-NAME emits
    /// one `str_index(item, k)` runtime read per position — a byte, `i64`. The item
    /// reads are LOOP-INVARIANT (the `from` item does not change during the translate)
    /// and are emitted ONCE here, before the per-position loop, so a `from` that
    /// aliases the source is read while the source still holds its original bytes.
    fn converting_from_consts(&mut self, op: &ConvOperand) -> Vec<String> {
        match op {
            ConvOperand::Literal(s) => s
                .as_bytes()
                .iter()
                .map(|&b| {
                    let reg = self.fresh("_icfrom");
                    self.emit("const", Some(&reg), vec![Operand::Int(b as i64)], "i64");
                    reg
                })
                .collect(),
            ConvOperand::Item { reg: item, width } => {
                let item = item.clone();
                (0..*width)
                    .map(|k| {
                        let ki = self.str_index(k as i64);
                        let reg = self.fresh("_icfrom");
                        self.emit(
                            "str_index",
                            Some(&reg),
                            vec![Operand::Var(item.clone()), Operand::Var(ki)],
                            "i64",
                        );
                        reg
                    })
                    .collect()
            }
        }
    }

    /// Bake the `to[0..len]` concat pieces (1-character `str` registers) for a
    /// CONVERTING operand. A LITERAL bakes one 1-char `str_const` per byte; a
    /// DATA-NAME emits one `str_slice(item, k, k+1)` runtime read per position — the
    /// 1-character string at `k`. Loop-invariant and emitted once, exactly like the
    /// `from` side (and read before the source write-back), so the two operand kinds
    /// feed the per-position chain identically.
    fn converting_to_consts(&mut self, op: &ConvOperand) -> Vec<String> {
        match op {
            ConvOperand::Literal(s) => s
                .as_bytes()
                .iter()
                .map(|&b| {
                    let reg = self.fresh("_icto");
                    self.emit(
                        "str_const",
                        Some(&reg),
                        vec![Operand::Str((b as char).to_string())],
                        "str",
                    );
                    reg
                })
                .collect(),
            ConvOperand::Item { reg: item, width } => {
                let item = item.clone();
                (0..*width)
                    .map(|k| {
                        let ki = self.str_index(k as i64);
                        let ki1 = self.str_index(k as i64 + 1);
                        let reg = self.fresh("_icto");
                        self.emit(
                            "str_slice",
                            Some(&reg),
                            vec![Operand::Var(item.clone()), Operand::Var(ki), Operand::Var(ki1)],
                            "str",
                        );
                        reg
                    })
                    .collect()
            }
        }
    }

    /// A single replacement character reduced to a fresh **string** register: a
    /// `str_const` of the 1-character string for a 1-char literal, a `str_const`
    /// of the single ASCII character for a figurative constant SPACE→" " / ZERO→"0",
    /// the 1-char slice register of a **constant reference-modified** operand of
    /// length 1 (`E(1:1)` → that slice reg, already a 1-char string), or the item's
    /// own register for a `PIC X(1)` item (its storage is already exactly one
    /// character wide). The parallel of [`Self::single_delim_code`] (which yields a
    /// byte code for a *scan*); this yields a 1-char string for a *concat*. The same
    /// later-rung rejections apply: a multi-character literal, a numeric operand, a
    /// **computed** reference-modified operand, and a numeric/wider item.
    ///
    /// The constant-refmod slice is materialised by the shared [`Self::ref_mod_slice`]
    /// exactly as in [`Self::single_delim_code`]; here the `SliceLen::Const(1)` slice
    /// register IS already the 1-char string this helper wants, so it is handed back
    /// directly (no `str_index` — that is the byte-code twin's job).
    fn single_delim_str(
        &mut self,
        op: &GrammarASTNode,
        verb: &str,
    ) -> Result<String, CompileError> {
        match read_operand(op)? {
            Operandy::Literal(Src::Str(s)) => {
                if s.len() != 1 {
                    return Err(CompileError::Unsupported(format!(
                        "{verb} with a multi-character delimiter is a later rung"
                    )));
                }
                let reg = self.fresh("_usds");
                self.emit("str_const", Some(&reg), vec![Operand::Str(s)], "str");
                Ok(reg)
            }
            Operandy::Literal(Src::Num(_)) => Err(CompileError::Unsupported(format!(
                "{verb} with a numeric-literal delimiter is a later rung"
            ))),
            // SPACE→" " / ZERO→"0": a figurative constant reduces to its single
            // ASCII character as a 1-char string, like the 1-char literal arm above.
            Operandy::Literal(Src::Space) => {
                let reg = self.fresh("_usds");
                self.emit("str_const", Some(&reg), vec![Operand::Str(" ".into())], "str");
                Ok(reg)
            }
            Operandy::Literal(Src::Zero) => {
                let reg = self.fresh("_usds");
                self.emit("str_const", Some(&reg), vec![Operand::Str("0".into())], "str");
                Ok(reg)
            }
            Operandy::RefMod { base, start, len } => {
                // Same Const/Runtime split as `single_delim_code`, but a `Const(1)`
                // slice register IS already the 1-char string this concat helper wants
                // (no `str_index` needed). A `Const(_ != 1)` is a multi-character
                // operand; a `Runtime` is a computed refmod — both later rungs,
                // rejected co-total with the oracle.
                let (reg, slice_len) = self.ref_mod_slice(&base, &start, &len)?;
                match slice_len {
                    SliceLen::Const(1) => Ok(reg),
                    SliceLen::Const(_) => Err(CompileError::Unsupported(format!(
                        "{verb} with a multi-character delimiter is a later rung"
                    ))),
                    SliceLen::Runtime { .. } => Err(CompileError::Unsupported(format!(
                        "{verb} with a computed reference-modified delimiter is a later rung"
                    ))),
                }
            }
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Char { .. } => {
                        if self.items[idx].width() != 1 {
                            return Err(CompileError::Unsupported(format!(
                                "{verb} with a delimiter item wider than one character is a later rung"
                            )));
                        }
                        Ok(self.items[idx].reg.clone())
                    }
                    ItemKind::Numeric { .. } => Err(CompileError::Unsupported(format!(
                        "{verb} with a numeric delimiter item is a later rung"
                    ))),
                }
            }
        }
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
    /// (COBOL's rule).
    ///
    /// A **mixed** relation — one operand an unsigned-integer numeric item, the
    /// other alphanumeric (a `PIC X` item or a string literal) — is COBOL's
    /// "compare a numeric with a non-numeric" case: the numeric operand is treated
    /// **as though moved to an alphanumeric field**, i.e. by its *n*-digit
    /// zero-padded decimal image ([`Self::emit_num_digit_string`] — the exact bytes
    /// a numeric→alphanumeric MOVE or a DISPLAY of the same item yields), and the
    /// comparison then proceeds by the **alphanumeric byte rule**: the shorter side
    /// is space-padded on the right to the longer's length and the two are compared
    /// byte-by-byte (the very same `str_cmp` path an all-alphanumeric relation
    /// takes, [`Self::emit_str_condition`]). So `IF NUM = "042"` with
    /// `NUM PIC 9(3) = 42` compares `"042"` = `"042"` → true, while `IF NUM = "42"`
    /// compares `"042"` vs `"42 "` (space-padded) → false. Because both engines
    /// build the identical image and run the identical space-padded `str_cmp`, the
    /// oracle (whose `Decimal::digits()` yields the same image) agrees byte-for-byte.
    ///
    /// An unsigned SCALED operand (`PIC 9(i)V9(d)`) uses its `(i + d)`-digit image
    /// (int part then frac part, no point) — so `IF S = "042"` with `S PIC 9(2)V9 =
    /// 4.2` is true. A SIGNED (`PIC S9…`) operand, integer or scaled, uses that same
    /// magnitude image with the operational sign folded into a TRAILING OVERPUNCH on
    /// the units digit ([`Self::emit_signed_num_alpha_image`] — the same bytes the
    /// signed numeric→alphanumeric MOVE produces), so `IF S = "12L"` with
    /// `S PIC S9(3) = -123` is true and `= "12C"` (the positive image) is false;
    /// ordering follows the byte comparison of those images. A numeric literal (a
    /// different pairing — kept out of scope) or a group item in a mixed relation is
    /// still a clean later rung (see [`Self::num_digit_str_operand`]).
    fn emit_relation(&mut self, relation: &GrammarASTNode) -> Result<String, CompileError> {
        let operands = child_nodes(relation, "operand");
        if operands.len() != 2 {
            return Err(CompileError::Malformed("relation must be operand relop operand".into()));
        }
        let op = self.relation_op(relation)?;
        self.emit_operand_relation(operands[0], operands[1], op)
    }

    /// The category-dispatching core of a two-operand relation, shared by `IF`
    /// relations ([`Self::emit_relation`]) and `EVALUATE`'s per-WHEN comparison
    /// ([`Self::emit_when_match`]). Given the two operand grammar nodes and a
    /// `cmp_*` op string, classify each side and route the comparison so that an
    /// `EVALUATE subject WHEN value` comparison is byte-identical to `IF subject
    /// <relop> value` for every category pair (including mixed
    /// numeric↔alphanumeric). Returns the boolean `i64` condition register.
    fn emit_operand_relation(
        &mut self,
        left: &GrammarASTNode,
        right: &GrammarASTNode,
        op: &str,
    ) -> Result<String, CompileError> {
        // Classify each operand: `None` = numeric (literal / numeric item / a
        // numeric figurative), `Some` = a character value.
        let ls = self.str_operand(left)?;
        let rs = self.str_operand(right)?;
        // The `ZERO` figurative is NUMERIC when paired with a numeric operand
        // (matching the oracle, whose mixed-comparison gate excludes `Fig::Zero`
        // and numeric-compares `Num` vs `Fig::Zero`): `numeric = ZERO` is the
        // numeric comparison `numeric = 0`, NOT an alphanumeric one — so a SIGNED
        // item is never sent through the overpunch-string path against ZERO (which
        // would compare e.g. `"00{"` ≠ `"000"` and silently miscompile the ubiquitous
        // `IF BALANCE = ZERO`). `str_operand` carries ZERO as `Fig('0')`; resolve the
        // pairing here. ZERO stays alphanumeric only against a character operand, and
        // ZERO-vs-ZERO stays a string compare (both `Some`), as the oracle does.
        let numeric_relation = matches!(
            (&ls, &rs),
            (None, None)
                | (None, Some(StrOperand::Fig('0')))
                | (Some(StrOperand::Fig('0')), None)
        );
        if numeric_relation {
            let left = self.read_arith_term(left)?;
            let right = self.read_arith_term(right)?;
            let w = self.term_scale(&left).max(self.term_scale(&right));
            let a = self.emit_term_at_scale(&left, w);
            let b = self.emit_term_at_scale(&right, w);
            let cond_reg = self.fresh("_cond");
            self.emit(op, Some(&cond_reg), vec![a, b], "i64");
            return Ok(cond_reg);
        }
        match (ls, rs) {
            (None, None) => unreachable!("numeric_relation handled the (None, None) pairing"),
            (Some(a), Some(b)) => self.emit_str_condition(a, b, op),
            // Mixed numeric ↔ alphanumeric: build the numeric side's digit image
            // and feed both operands through the same alphanumeric `str_cmp` path
            // (preserving left→right operand order). Only an unsigned-integer
            // numeric item is modelled; every other numeric shape is a later rung.
            (None, Some(b)) => {
                let a = self.num_digit_str_operand(left)?;
                self.emit_str_condition(a, b, op)
            }
            (Some(a), None) => {
                let b = self.num_digit_str_operand(right)?;
                self.emit_str_condition(a, b, op)
            }
        }
    }

    /// Build the [`StrOperand`] for the **numeric** side of a mixed
    /// numeric↔alphanumeric relation: its *n*-digit zero-padded decimal image
    /// ([`Self::emit_num_digit_string`]), the exact bytes a numeric→alphanumeric
    /// MOVE (or a DISPLAY) of the same item produces, so the comparison then
    /// proceeds by the identical alphanumeric byte rule the oracle uses (whose
    /// `Decimal::digits()` yields the same image).
    ///
    /// An **unsigned** numeric ITEM — integer (`PIC 9(n)`) or scaled
    /// (`PIC 9(i)V9(d)`) — has an unambiguous image (`int + frac`, no point), so it
    /// is accepted with its plain magnitude image. A **signed** numeric item
    /// (`PIC S9…`, integer or scaled) is also accepted: its image is the same
    /// `(i + d)`-digit magnitude with the operational sign folded into a TRAILING
    /// OVERPUNCH on the units digit ([`Self::emit_signed_num_alpha_image`]), exactly
    /// the bytes the signed numeric→alphanumeric MOVE produces. Still rejected here:
    ///
    /// * a **numeric literal** against an alphanumeric operand is a *different*
    ///   pairing (kept out of scope) and is rejected here too;
    /// * a **group** item never reaches this method — its name is unregistered on
    ///   this rung, so [`Self::str_operand`] → [`Self::item_index`] already errored.
    fn num_digit_str_operand(&mut self, op: &GrammarASTNode) -> Result<StrOperand, CompileError> {
        match read_operand(op)? {
            Operandy::Name(name) => {
                let idx = self.item_index(&name)?;
                match &self.items[idx].kind {
                    ItemKind::Numeric { int_digits, dec_digits, signed: false, .. } => {
                        // The digit image is the full `(i + d)`-digit magnitude —
                        // integer part then fractional part, no decimal point — the
                        // exact bytes `Decimal::digits()` yields (`int + frac`). The
                        // scaled slot already holds `value * 10^d`, so its `(i + d)`
                        // digits ARE the image (an INTEGER operand, `d = 0`, is the
                        // special case).
                        let n = *int_digits + *dec_digits;
                        let num_reg = self.items[idx].reg.clone();
                        let reg = self.emit_num_digit_string(&num_reg, n);
                        Ok(StrOperand::Fixed { reg, len: n })
                    }
                    ItemKind::Numeric { int_digits, dec_digits, signed: true, .. } => {
                        // A SIGNED numeric item's comparison image is its
                        // `(i + d)`-digit zero-padded MAGNITUDE with the operational
                        // sign folded into a TRAILING OVERPUNCH on the units digit —
                        // the exact bytes a signed numeric→alphanumeric MOVE of the
                        // same item produces (see `emit_signed_num_alpha_image`). Once
                        // built, the mixed comparison proceeds by the identical
                        // alphanumeric byte rule the oracle uses (whose
                        // `overpunch_trailing(magnitude, neg)` yields the same image).
                        // For example `PIC S9(3) = -123` compares equal to `"12L"`,
                        // `= +123` equal to `"12C"`, and a scaled `PIC S9V9 = -4.2`
                        // equal to `"4K"`. A value that truncates to a zero magnitude
                        // stores `neg = false` (COBOL has no negative zero), so its
                        // image is `"00{"` — matching the oracle byte-for-byte.
                        let n = *int_digits + *dec_digits;
                        let num_reg = self.items[idx].reg.clone();
                        let reg = self.emit_signed_num_alpha_image(&num_reg, n);
                        Ok(StrOperand::Fixed { reg, len: n })
                    }
                    // `str_operand` classified this operand as numeric (`None`); a
                    // character item would have been `Some`. Unreachable in practice,
                    // but handled honestly rather than with `unreachable!`.
                    ItemKind::Char { .. } => Err(CompileError::Malformed(format!(
                        "operand {name} classified as both numeric and alphanumeric"
                    ))),
                }
            }
            Operandy::Literal(Src::Num(_)) | Operandy::Literal(Src::Zero) => {
                Err(CompileError::Unsupported(
                    "a numeric literal compared with an alphanumeric operand is a later rung \
                     (a different pairing)"
                        .into(),
                ))
            }
            // Every remaining operand shape is alphanumeric and would have been
            // `Some` in `str_operand`, so it never reaches the numeric side.
            _ => Err(CompileError::Malformed(
                "unexpected operand shape on the numeric side of a mixed comparison".into(),
            )),
        }
    }

    /// Evaluate a level-88 condition-name to a boolean `i64` register: does its
    /// conditional variable equal any single value, or fall within any inclusive
    /// `THRU` range?
    ///
    /// A **numeric** variable compares its slot against each numeric VALUE / `THRU`
    /// range: each value-item becomes one boolean (`cmp_eq` for a single value;
    /// `and` of `cmp_ge`/`cmp_le` for a range) OR-folded with `or` — because each
    /// `cmp_*` yields `0`/`1`, bitwise `and`/`or` are exactly logical AND/OR, and
    /// the combined `i64` feeds `jmp_if_false` like any relational condition.
    ///
    /// An **alphanumeric** (`PIC X`) variable is supported for STRING values: when
    /// every VALUE item is a discrete string literal or a string `THRU` range
    /// ([`all_str_values`]), each value becomes a boolean over the SAME alphanumeric
    /// `str_cmp` path ([`Self::emit_str_condition`]) an `IF var = "…"` /
    /// `IF var >= "…"` relation runs — the variable's slot against the value's
    /// `str_const`, space-padded to a common width:
    ///
    ///   * a discrete string `s` is `cmp_eq(var, s)`;
    ///   * an inclusive range `lo THRU hi` is `and(cmp_ge(var, lo), cmp_le(var, hi))`
    ///     — exactly how the numeric range and an alphanumeric `IF var >= "…"` lower.
    ///
    /// The value-list OR-folds with `or`, mirroring the numeric fold exactly.
    /// Reusing `emit_str_condition` is what makes the read byte-identical to the
    /// oracle's `compare_operands`. A `THRU` range with a NON-string bound, a
    /// numeric/figurative VALUE, or a mixed list on an alphanumeric variable stays a
    /// later rung — rejected identically to the oracle.
    fn emit_condition_name(&mut self, name: &str) -> Result<String, CompileError> {
        // Phase 1 (immutable): resolve the variable and, per its kind, gather the
        // owned data phase 2 needs — numeric constants, or the discrete string
        // VALUEs — so the mutable emits below hold no borrow into `self.conditions`.
        let cn = self.conditions.get(name).ok_or_else(|| {
            CompileError::Unsupported(format!("reference to condition-name {name} (undeclared)"))
        })?;
        let var = cn.var;
        // Alphanumeric slot: an OR-fold of alphanumeric tests against each string
        // VALUE, via the very `str_cmp` path `IF var = "…"` / `IF var >= "…"` uses.
        if matches!(&self.items[var].kind, ItemKind::Char { .. }) {
            if !all_str_values(&cn.values) {
                return Err(CompileError::Unsupported(
                    "a level-88 condition-name on an alphanumeric item needs string VALUEs (a \
                     discrete string or a string THRU range; a range with a non-string bound or a \
                     numeric/figurative VALUE is a later rung)"
                        .into(),
                ));
            }
            // Phase 1 (immutable): gather owned string tests so the emits below hold
            // no borrow into `self.conditions`. `all_str_values` guarantees every
            // item is a string `Single` or a string `Range`.
            let str_tests: Vec<StrValueTest> = cn
                .values
                .iter()
                .map(|v| match v {
                    ValueSpec::Single(Src::Str(s)) => StrValueTest::Eq(s.clone()),
                    ValueSpec::Range(Src::Str(lo), Src::Str(hi)) => {
                        StrValueTest::InRange(lo.clone(), hi.clone())
                    }
                    _ => unreachable!("all_str_values guarantees string Singles/Ranges"),
                })
                .collect();
            let slot_reg = self.items[var].reg.clone();
            let slot_len = self.items[var].width();
            // Build the fixed subject `StrOperand` `str_operand` builds for a Char
            // item; a fresh clone per comparison since `emit_str_condition` consumes
            // it.
            let subject = |reg: &str, len| StrOperand::Fixed { reg: reg.to_string(), len };
            // Phase 2 (mutable): emit one boolean per test, OR-folded with `or`.
            let mut acc: Option<String> = None;
            for test in str_tests {
                let b = match test {
                    // Discrete string: the value's `str_const` as a fixed `StrOperand`,
                    // then the shared space-padded `str_cmp` equality — byte-identical
                    // to `IF var = "…"`.
                    StrValueTest::Eq(s) => {
                        let vlen = s.len();
                        let vreg = self.fresh("_sl");
                        self.emit("str_const", Some(&vreg), vec![Operand::Str(s)], "str");
                        let value = StrOperand::Fixed { reg: vreg, len: vlen };
                        self.emit_str_condition(subject(&slot_reg, slot_len), value, "cmp_eq")?
                    }
                    // Inclusive range: `and(cmp_ge(var, lo), cmp_le(var, hi))` — the
                    // SAME `str_cmp` lowering an alphanumeric `IF var >= "…"` /
                    // `IF var <= "…"` relation uses, and the same `and` the numeric
                    // range emits.
                    StrValueTest::InRange(lo, hi) => {
                        let lo_len = lo.len();
                        let lo_reg = self.fresh("_sl");
                        self.emit("str_const", Some(&lo_reg), vec![Operand::Str(lo)], "str");
                        let lo_op = StrOperand::Fixed { reg: lo_reg, len: lo_len };
                        let ge = self.emit_str_condition(subject(&slot_reg, slot_len), lo_op, "cmp_ge")?;
                        let hi_len = hi.len();
                        let hi_reg = self.fresh("_sl");
                        self.emit("str_const", Some(&hi_reg), vec![Operand::Str(hi)], "str");
                        let hi_op = StrOperand::Fixed { reg: hi_reg, len: hi_len };
                        let le = self.emit_str_condition(subject(&slot_reg, slot_len), hi_op, "cmp_le")?;
                        let r = self.fresh("_c88rng");
                        self.emit("and", Some(&r), vec![Operand::Var(ge), Operand::Var(le)], "i64");
                        r
                    }
                };
                acc = Some(match acc {
                    None => b,
                    Some(prev) => {
                        let or = self.fresh("_c88or");
                        self.emit("or", Some(&or), vec![Operand::Var(prev), Operand::Var(b)], "i64");
                        or
                    }
                });
            }
            // `values` is non-empty (enforced at registration), so `acc` is set.
            return acc.ok_or_else(|| CompileError::Malformed(format!("level-88 {name} has no VALUE")));
        }
        let (int_digits, dec_digits, signed) = match &self.items[var].kind {
            ItemKind::Numeric { int_digits, dec_digits, signed, .. } => (*int_digits, *dec_digits, *signed),
            ItemKind::Char { .. } => unreachable!("the Char case returned above"),
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
            Operandy::RefMod { base, start, len } => {
                let (reg, slice_len) = self.ref_mod_slice(&base, &start, &len)?;
                Ok(Some(match slice_len {
                    SliceLen::Const(len) => StrOperand::Fixed { reg, len },
                    SliceLen::Runtime { len_reg, max_len } => {
                        StrOperand::Runtime { reg, len_reg, max_len }
                    }
                }))
            }
        }
    }

    /// Emit an alphanumeric comparison: resolve any figurative to the other
    /// operand's length, space-pad both sides to their common (max) length, then
    /// `str_cmp` and apply the relation against zero. `str_cmp` returns an `i64`
    /// ordering (−1/0/1), so `cmp_* … 0` is an integer comparison (no `Bool`
    /// mismatch). Two figuratives — neither with a length to borrow — each resolve
    /// to a single fill character (`ZERO` → `"0"`, `SPACE` → `"  "`… width 1),
    /// matching the oracle (whose `src_chars` of a figurative is empty, so both
    /// `fill_fig` to `len().max(1)` = 1); e.g. `IF ZERO = SPACE` is `"0"` vs `" "`.
    fn emit_str_condition(
        &mut self,
        a: StrOperand,
        b: StrOperand,
        op: &str,
    ) -> Result<String, CompileError> {
        // Each operand contributes a compile-time *upper bound* on its length (a
        // figurative has none — it borrows the other side's). The common width is
        // the max of those bounds. Padding **both** operands with trailing spaces
        // to any common width ≥ their actual lengths yields the same `str_cmp`
        // result as padding to the exact max-of-actual-lengths COBOL prescribes —
        // trailing spaces past the first differing position never change the
        // ordering — so a run-time-length slice compares byte-identically to the
        // oracle even though its exact length is unknown at compile time.
        let a_max = str_operand_max_len(&a);
        let b_max = str_operand_max_len(&b);
        let width = match (a_max, b_max) {
            (Some(x), Some(y)) => x.max(y),
            (Some(x), None) | (None, Some(x)) => x,
            // Two figuratives: neither has a length to borrow, so each resolves to
            // a single fill character (width 1) — exactly the oracle's behaviour
            // (`src_chars` of a figurative is empty → both `fill_fig` to `.max(1)`).
            (None, None) => 1,
        };
        let ap = self.materialize_str_to_width(a, width);
        let bp = self.materialize_str_to_width(b, width);
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

    /// Materialise a comparison operand into a `str` register of **exactly**
    /// `width` characters, ready for `str_cmp`:
    ///   * `Fixed` — compile-time space padding ([`Self::pad_spaces`]);
    ///   * `Fig`   — the figurative character repeated `width` times;
    ///   * `Runtime` — a computed-refmod slice padded to `width` *at run time*
    ///     ([`Self::pad_runtime`]), since its length is only known then.
    fn materialize_str_to_width(&mut self, op: StrOperand, width: usize) -> String {
        match op {
            StrOperand::Fixed { reg, len } => self.pad_spaces(reg, len, width),
            StrOperand::Fig(c) => self.fig_const(c, width),
            StrOperand::Runtime { reg, len_reg, .. } => self.pad_runtime(reg, len_reg, width),
        }
    }

    /// Right-pad a run-time slice `reg` (of run-time length `len_reg`, which is
    /// `<= width`) with spaces to exactly `width` characters. The padding count
    /// `needed = width - len` is computed at run time; the spaces come from
    /// slicing a `width`-space constant to `needed` — the same trick UNSTRING
    /// uses to size a run-time-length space fill.
    fn pad_runtime(&mut self, reg: String, len_reg: String, width: usize) -> String {
        if width == 0 {
            return reg;
        }
        let wconst = self.fresh("_pw");
        self.emit("const", Some(&wconst), vec![Operand::Int(width as i64)], "i64");
        let needed = self.fresh("_pn");
        self.emit("sub", Some(&needed), vec![Operand::Var(wconst), Operand::Var(len_reg)], "i64");
        let spaces = self.spaces_const(width);
        let z0 = self.fresh("_pz");
        self.emit("const", Some(&z0), vec![Operand::Int(0)], "i64");
        let padslice = self.fresh("_ps");
        self.emit(
            "str_slice",
            Some(&padslice),
            vec![Operand::Var(spaces), Operand::Var(z0), Operand::Var(needed)],
            "str",
        );
        let out = self.fresh("_pad");
        self.emit("str_concat", Some(&out), vec![Operand::Var(reg), Operand::Var(padslice)], "str");
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
            Operandy::RefMod { .. } => Err(CompileError::Unsupported(REFMOD_CONTEXT_MSG.into())),
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
            Operandy::RefMod { .. } => Err(CompileError::Unsupported(REFMOD_CONTEXT_MSG.into())),
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
            Operandy::RefMod { .. } => Err(CompileError::Unsupported(REFMOD_CONTEXT_MSG.into())),
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

/// A source value in flight: a literal, a data-name reference, or a reference
/// modification of a data-name.
enum Operandy {
    Literal(Src),
    Name(String),
    /// `base(start:len)` / `base(start:)` — a reference modification selecting
    /// `len` characters of alphanumeric item `base` from 1-based position
    /// `start`; an omitted `len` runs to the end of the item. `start` and `len`
    /// are each a [`RefIndex`]: an integer literal *or* a data-name whose value is
    /// only known at run time (a **computed** reference modification). A
    /// literal:literal refmod is still folded to a constant slice; the moment
    /// either index is a data-name the lowering takes the run-time `str_slice`
    /// path (see [`Emitter::ref_mod_slice`]).
    RefMod { base: String, start: RefIndex, len: Option<RefIndex> },
}

/// One index (start or length) of a reference modification. Either a
/// compile-time integer literal, or a data-name read at run time — the
/// distinction the lowering uses to choose between the constant-fold slice and
/// the computed `str_slice`.
#[derive(Clone)]
enum RefIndex {
    /// A plain integer NUMBER literal, e.g. the `2` and `3` in `WS(2:3)`.
    Lit(usize),
    /// A data-name whose (integer, unsigned) value is the index at run time,
    /// e.g. the `J` and `K` in `WS(J:K)`.
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

/// Read an `operand` node into a literal, a data-name reference, or a reference
/// modification.
///
/// The grammar is `operand = NAME [ LPAREN operand COLON [ operand ] RPAREN ] |
/// literal ;`. A bare NAME (no parenthesised suffix) is an [`Operandy::Name`]
/// exactly as before. When the reference-modification suffix is present the
/// inner start/length operands appear as nested `operand` child nodes; each is
/// read into a [`RefIndex`] — a plain integer NUMBER literal *or* a data-name (a
/// **computed** reference modification, lowered to a run-time `str_slice`).
fn read_operand(op: &GrammarASTNode) -> Result<Operandy, CompileError> {
    if let Some(lit) = child_node(op, "literal") {
        return Ok(Operandy::Literal(read_literal(lit)?));
    }
    if let Some(name) = first_token(op, "NAME") {
        let inner = child_nodes(op, "operand");
        if !inner.is_empty() {
            let start = read_refmod_index(inner[0])?;
            let len = match inner.get(1) {
                Some(l) => Some(read_refmod_index(l)?),
                None => None,
            };
            return Ok(Operandy::RefMod { base: name, start, len });
        }
        return Ok(Operandy::Name(name));
    }
    Err(CompileError::Malformed("unrecognised operand".into()))
}

/// Validate a **constant** reference modification and return its length, or
/// signal that it is not fully constant.
///
/// * `Ok(Some(actual_len))` — start (and length, if present) are literals and
///   the slice is in range: the caller folds it to a constant `str_slice`.
/// * `Ok(None)` — the length is a data-name, so the whole refmod is *computed*:
///   the caller takes the run-time path.
/// * `Err(..)` — a literal:literal refmod that is out of range (`start < 1` or
///   the slice runs past the item), rejected at compile time as in #8673.
///
/// The subtractive bounds test (`actual_len > width - start0`, reached only once
/// `start0 <= width`) avoids the `start0 + actual_len` overflow a crafted
/// `WS(1e19:1e19)` would otherwise cause.
fn const_refmod_len(
    start: usize,
    len: &Option<RefIndex>,
    width: usize,
) -> Result<Option<usize>, CompileError> {
    let literal_len = match len {
        None => None,
        Some(RefIndex::Lit(l)) => Some(*l),
        // A data-name length is not compile-time constant — take the run path.
        Some(RefIndex::Name(_)) => return Ok(None),
    };
    if start < 1 {
        return Err(CompileError::Malformed(
            "reference modification start position must be at least 1".into(),
        ));
    }
    let start0 = start - 1;
    let actual_len = literal_len.unwrap_or(width.saturating_sub(start0));
    if start0 > width || actual_len > width - start0 {
        return Err(CompileError::Unsupported(format!(
            "constant reference modification ({start}:{}) runs past the {width}-character item — a later rung",
            literal_len.map(|l| l.to_string()).unwrap_or_default()
        )));
    }
    Ok(Some(actual_len))
}

/// Read a reference-modification start or length subnode into a [`RefIndex`]:
/// a plain integer NUMBER literal becomes [`RefIndex::Lit`]; a bare data-name
/// becomes [`RefIndex::Name`] (a *computed* index resolved at run time). Any
/// other form — a signed/fractional literal, a figurative, or a nested
/// reference modification as the index itself — is a later rung.
fn read_refmod_index(op: &GrammarASTNode) -> Result<RefIndex, CompileError> {
    let unsupported = |m: &str| CompileError::Unsupported(m.into());
    // A bare data-name index (no `literal` child, just a NAME): computed refmod.
    if child_node(op, "literal").is_none() {
        // Reject a nested reference modification used *as* an index (`WS(A(1:1):2)`).
        if child_nodes(op, "operand").is_empty() {
            if let Some(name) = first_token(op, "NAME") {
                return Ok(RefIndex::Name(name));
            }
        }
        return Err(unsupported(
            "a reference-modified reference-modification index is a later rung",
        ));
    }
    let lit = child_node(op, "literal").unwrap();
    match read_literal(lit)? {
        Src::Num(s) => s
            .parse::<usize>()
            .map(RefIndex::Lit)
            .map_err(|_| unsupported("a signed or fractional reference-modification index is a later rung")),
        _ => Err(unsupported(
            "a non-integer reference-modification index is a later rung",
        )),
    }
}

/// Which side of a delimiter an `INSPECT … BEFORE`/`AFTER` region selects — the
/// compiler-side mirror of the oracle's `RegionKind`.
#[derive(Clone, Copy)]
enum RegionKind {
    Before,
    After,
}

/// The parsed pieces of a `TALLYING counter FOR ALL|LEADING delim [{BEFORE|AFTER}
/// x]` (or `FOR CHARACTERS [{BEFORE|AFTER} x]`) phrase: `(counter_name, delim_node,
/// leading, characters, region)`, where `region` is the optional `{BEFORE|AFTER} x`
/// window as `(kind, region_delim_node)`. `delim_node` is `None` on the `CHARACTERS`
/// path (that form carries no delimiter operand — it counts positions), and `Some`
/// otherwise. The node references borrow from the `inspect_stmt` CST the reader was
/// handed.
type TallyPhrase<'a> =
    (String, Option<&'a GrammarASTNode>, bool, bool, Option<(RegionKind, &'a GrammarASTNode)>);

/// Extract the supported `TALLYING counter FOR ALL delim [{BEFORE|AFTER} x]` /
/// `FOR LEADING delim` / `FOR CHARACTERS [{BEFORE|AFTER} x]` phrase from an
/// `inspect_stmt`, returning `(counter_name, delim_operand_node, leading,
/// characters, region)` where `leading` is `true` for `FOR LEADING` (count only the
/// leading run) and `false` for `FOR ALL` (count every occurrence), `characters` is
/// `true` for `FOR CHARACTERS` (count every POSITION in the window; `delim_node` is
/// then `None` and `leading` is `false`), and `region` carries an optional
/// `{BEFORE|AFTER} x` window as `(kind, region_delim_node)`. Rejects every later-rung
/// form the grammar also accepts:
///   * more than one `TALLYING` counter (`{ tally_for }`) — one counter this rung;
///   * more than one `FOR` phrase on that counter (`{ tally_item }`).
///
/// `CHARACTERS` is now ACCEPTED for this single-item single-counter phrase (count =
/// window length); only the multi-item / multi-counter `CHARACTERS` forms remain
/// later rungs, rejected in `inspect_tally_multi` / `inspect_tally_counters`.
///
/// A `FOR LEADING` phrase carrying a region is now ACCEPTED — the STANDALONE
/// `FOR LEADING … BEFORE/AFTER` form is supported this rung (the count anchors the
/// leading run at the window start). The COMBINED form still defers a LEADING half
/// with a region; `emit_inspect_tallying` re-imposes that via `allow_leading_region`.
///
/// (`REPLACING` and a non-alphanumeric source are rejected by the caller. A
/// multi-character region delimiter is rejected by `single_delim_code` at emit time,
/// exactly like the tally delimiter, so both engines diagnose it identically.)
fn inspect_tally_all(verb: &GrammarASTNode) -> Result<TallyPhrase<'_>, CompileError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    let fors = child_nodes(tallying, "tally_for");
    let tf = match fors.as_slice() {
        [one] => *one,
        _ => {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING with several counters is a later rung".into(),
            ))
        }
    };
    let counter = first_token(tf, "NAME")
        .ok_or_else(|| CompileError::Malformed("INSPECT TALLYING without a counter".into()))?;
    let items = child_nodes(tf, "tally_item");
    let ti = match items.as_slice() {
        [one] => *one,
        _ => {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING with several FOR phrases is a later rung".into(),
            ))
        }
    };
    let toks = child_tokens(ti);
    // `FOR CHARACTERS` is the "count every position" form. The grammar's CHARACTERS
    // branch of `tally_item` is `CHARACTERS { inspect_region }` — it carries NO
    // delimiter operand — so on this path `delim_node` is `None` and the emitter counts
    // the window LENGTH rather than scanning for delimiter matches. Detected at the
    // SAME point the oracle's reader detects it, so the two engines' accept sets match.
    let characters = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS");
    // `FOR LEADING` is supported (count only the leading run); `FOR ALL` is the
    // default. The keyword selects the scan's stop-on-mismatch behaviour. CHARACTERS
    // never carries LEADING, so this stays `false` on that path.
    let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
    // A `{BEFORE|AFTER} x` region now PARSES into `Option<(RegionKind, node)>` (it
    // used to be rejected wholesale here) REGARDLESS of `leading`/`characters`: the
    // STANDALONE `FOR LEADING … BEFORE/AFTER` and `FOR CHARACTERS … BEFORE/AFTER` forms
    // are supported this rung (see `emit_inspect_tallying`). The COMBINED form still
    // defers a LEADING half with a region; `emit_inspect_tallying` re-imposes that via
    // its `allow_leading_region` flag, so relaxing this shared reader does not leak the
    // combination into the combined form.
    let region = match child_node(ti, "inspect_region") {
        None => None,
        Some(region_node) => {
            let rtoks = child_tokens(region_node);
            let kind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                RegionKind::Before
            } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                RegionKind::After
            } else {
                return Err(CompileError::Unsupported(
                    "INSPECT region without a BEFORE or AFTER keyword".into(),
                ));
            };
            let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                CompileError::Malformed("INSPECT BEFORE/AFTER region without a delimiter".into())
            })?;
            Some((kind, rdelim))
        }
    };
    // The CHARACTERS path has no delimiter operand to read (`delim_node = None`); the
    // ALL/LEADING path reads its single-char delimiter as before.
    let delim = if characters {
        None
    } else {
        Some(child_node(ti, "operand").ok_or_else(|| {
            CompileError::Malformed("INSPECT TALLYING FOR ALL/LEADING without a delimiter".into())
        })?)
    };
    Ok((counter, delim, leading, characters, region))
}

/// One `ALL delim [{BEFORE|AFTER} x]` item of a MULTI-item TALLYING clause:
/// `(delim_node, region)`, where `region` is the optional `{BEFORE|AFTER} x` window as
/// `(kind, region_delim_node)` — the SAME shape [`TallyPhrase`]'s region carries, and
/// the count-side analogue of [`ReplaceItem`]. `ALL`-only by construction (the
/// multi-item scope bound), so — unlike [`TallyPhrase`] — it carries no
/// `leading`/`characters` flags, but each item now carries its OWN optional region
/// window (this rung lifts the region reject).
type TallyItem<'a> = (&'a GrammarASTNode, Option<(RegionKind, &'a GrammarASTNode)>);

/// Which flavour of tally item a SINGLE-counter multi-item TALLYING clause holds — the
/// compiler-side mirror of the oracle's `TallyMultiKind`. Picking an explicit enum (over a
/// bare `leading: bool`) makes the illegal "LEADING and ALSO CHARACTERS" state
/// UNREPRESENTABLE and keeps the CST-side dispatch co-total with the oracle reader:
/// `ALL delim` → [`TallyKind::All`], `LEADING delim` → [`TallyKind::Leading`],
/// `CHARACTERS` → [`TallyKind::Characters`] (no delimiter operand, always-eligible
/// catch-all).
#[derive(Clone, Copy, PartialEq, Eq)]
enum TallyKind {
    /// `ALL delim` — eligible at an in-window position whose char equals the delimiter.
    All,
    /// `LEADING delim` — like `All`, but only while this item's leading run is unbroken.
    Leading,
    /// `CHARACTERS` — eligible at EVERY in-window position; carries no delimiter operand.
    Characters,
}

/// One `{ALL|LEADING} delim [{BEFORE|AFTER} x]` — or bare `CHARACTERS [{BEFORE|AFTER} x]` —
/// item of a SINGLE-counter multi-item TALLYING clause: `(delim_node, kind, region)`.
/// Extends [`TallyItem`] with a [`TallyKind`] tag this rung lifts into the multi-item list
/// (a `CHARACTERS` item alongside others is now admitted). `delim_node` is `None` on the
/// `CHARACTERS` path (that form carries no delimiter operand — it counts positions) and
/// `Some` otherwise. The several-COUNTERS path stays `ALL`-only and keeps [`TallyItem`].
/// Named so [`inspect_tally_multi`]'s return type stays legible (and below clippy's
/// type-complexity threshold) — the compiler-side analogue of the oracle's
/// `TallyMultiLeadingItem`.
type TallyLeadingItem<'a> =
    (Option<&'a GrammarASTNode>, TallyKind, Option<(RegionKind, &'a GrammarASTNode)>);

/// One `counter FOR ALL a [{BEFORE|AFTER} p] ALL b … ` group of a MULTI-counter
/// `TALLYING` list: the counter name plus its written-order [`TallyItem`]s (each a
/// delimiter node + its OWN optional region window). Named so
/// [`inspect_tally_counters`]'s return type stays legible (and below clippy's
/// type-complexity threshold) — the compiler-side analogue of the oracle's
/// `TallyCounterGroup`.
type TallyCounterGroup<'a> = (String, Vec<TallyItem<'a>>);

/// One resolved item of a single-counter multi-item `TALLYING` scan (with possible
/// `LEADING`/`CHARACTERS` items): `(delimiter_byte_code_reg, kind, active_reg, window)`
/// where `delimiter_byte_code_reg` is `None` for a `CHARACTERS` item (no delimiter to
/// compare), `active_reg` is the per-`LEADING`-item runtime run flag register (i64, init
/// 1; `None` for `ALL`/`CHARACTERS`), and `window` is the optional `[start, end)`
/// byte-bound register pair for the item's `{BEFORE|AFTER}` region (`None` = whole-source).
/// Named so `emit_inspect_tally_multi`'s resolved vector stays below clippy's
/// type-complexity threshold.
type ResolvedTallyLeadingItem =
    (Option<String>, TallyKind, Option<String>, Option<(String, String)>);

/// One resolved item of a multi-item `REPLACING` unroll (with possible
/// `LEADING`/`CHARACTERS` items): `(search_byte_code_reg, replacement_1char_str_reg, kind,
/// active_reg, window)`. The replace-side twin of [`ResolvedTallyLeadingItem`]: it adds
/// the replacement-string register (the tally has nothing to emit, only to count) but
/// carries the SAME `kind`/`active_reg`/`window` fields. `search_byte_code_reg` is `None`
/// for a `CHARACTERS` item (no search to compare), `active_reg` is the per-`LEADING`-item
/// runtime run flag register (i64, init 1; `None` for `ALL`/`CHARACTERS`), and `window` is
/// the optional `[start, end)` byte-bound register pair for the item's `{BEFORE|AFTER}`
/// region (`None` = whole-source). Named so `emit_inspect_replacing_multi`'s resolved
/// vector stays below clippy's type-complexity threshold.
type ResolvedReplaceLeadingItem =
    (Option<String>, String, ReplaceKind, Option<String>, Option<(String, String)>);

/// One entry of the FLATTENED combined-priority list a multi-counter `TALLYING` scan
/// walks per position: `(group_index, delimiter_byte_code_reg, window)` where `window` is
/// the optional `[start, end)` byte-bound register pair for the item's `{BEFORE|AFTER}`
/// region (`None` = whole-source, region-less). Named so
/// `emit_inspect_tally_counters`'s flat vector stays below clippy's type-complexity
/// threshold.
type FlatCounterDelim = (usize, String, Option<(String, String)>);

/// Extract the `TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|AFTER} q]
/// …` phrase of a multi-item INSPECT whose SOLE counter carries TWO OR MORE `FOR`
/// items, returning `(counter_name, items)` with the `(delim_node, region)` items in
/// WRITTEN ORDER — the compiler-side analogue of the oracle's `read_inspect_tally_multi`,
/// counting the SAME `tally_item` children so the two engines' accept/reject sets stay
/// co-total. Only called after the caller has confirmed EXACTLY ONE `tally_for` with
/// `>= 2` items; a single item keeps [`inspect_tally_all`] and all its capabilities
/// (LEADING, region), and SEVERAL counters (more than one `tally_for`) stays a later
/// rung.
///
/// Scope (this rung, IDENTICAL messages to the oracle reader): each item is `ALL`,
/// `LEADING`, OR `CHARACTERS` — the multi-item `CHARACTERS` reject is LIFTED this rung
/// (LEADING and the per-item region were lifted earlier). Each item carries a [`TallyKind`]
/// tag, an OPTIONAL delimiter node (`Some` for `ALL`/`LEADING`, `None` for `CHARACTERS`),
/// AND its OWN optional `{BEFORE|AFTER} x` region, parsed with the SAME keyword/operand
/// extraction `inspect_tally_all` uses on the single-item side. A `CHARACTERS` item is the
/// always-eligible catch-all (no delimiter, no run). Any item violating the remaining scope
/// is a clean later-rung `Unsupported`. A multi-character/figurative/wider/numeric delimiter
/// on an `ALL`/`LEADING` item is NOT rejected here — it falls to the SAME `single_delim_code`
/// check the single-item emitter uses. (The MULTI-COUNTER and COMBINED `TALLYING …
/// REPLACING` forms keep rejecting `CHARACTERS` in `inspect_tally_counters` / the combined
/// caller, not here.)
fn inspect_tally_multi(
    verb: &GrammarASTNode,
) -> Result<(String, Vec<TallyLeadingItem<'_>>), CompileError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    // Exactly one counter (`tally_for`): several counters is a later rung, diagnosed
    // with the SAME message `inspect_tally_all` raises so the reject is uniform.
    let fors = child_nodes(tallying, "tally_for");
    let tf = match fors.as_slice() {
        [one] => *one,
        _ => {
            return Err(CompileError::Unsupported(
                "INSPECT TALLYING with several counters is a later rung".into(),
            ))
        }
    };
    let counter = first_token(tf, "NAME")
        .ok_or_else(|| CompileError::Malformed("INSPECT TALLYING without a counter".into()))?;
    let mut items = Vec::new();
    for ti in child_nodes(tf, "tally_item") {
        let toks = child_tokens(ti);
        // Classify the item into exactly one [`TallyKind`] variant, matching the oracle
        // reader. `CHARACTERS` in a multi-item list is now ACCEPTED (this rung) — the
        // always-eligible catch-all, no delimiter operand. `LEADING` (also lifted, #65)
        // counts only its run; `ALL` is the default. The keyword picks per-item count
        // semantics threaded to `emit_inspect_tally_multi`.
        let kind = if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
            TallyKind::Characters
        } else if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING") {
            TallyKind::Leading
        } else {
            TallyKind::All
        };
        // A `{BEFORE|AFTER} x` region on an item is ACCEPTED for EVERY kind: parse it into
        // `Option<(RegionKind, node)>` with the SAME keyword/operand extraction
        // `inspect_tally_all` uses on the single-item side. The region contributes its own
        // delimiter operand under the `inspect_region` child, not a direct child of
        // `tally_item`, so an `ALL`/`LEADING` item's DIRECT `operand` child below is still
        // exactly the tally delimiter.
        let region = match child_node(ti, "inspect_region") {
            None => None,
            Some(region_node) => {
                let rtoks = child_tokens(region_node);
                let rkind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                    RegionKind::Before
                } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                    RegionKind::After
                } else {
                    return Err(CompileError::Unsupported(
                        "INSPECT region without a BEFORE or AFTER keyword".into(),
                    ));
                };
                let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                    CompileError::Malformed(
                        "INSPECT BEFORE/AFTER region without a delimiter".into(),
                    )
                })?;
                Some((rkind, rdelim))
            }
        };
        // A `CHARACTERS` item carries NO delimiter operand (grammar branch
        // `CHARACTERS { inspect_region }`), so we must NOT read an `operand` child there —
        // the delimiter node is `None`. `ALL`/`LEADING` read their single delimiter node.
        let delim = match kind {
            TallyKind::Characters => None,
            _ => Some(child_node(ti, "operand").ok_or_else(|| {
                CompileError::Malformed(
                    "INSPECT TALLYING FOR ALL/LEADING without a delimiter".into(),
                )
            })?),
        };
        items.push((delim, kind, region));
    }
    Ok((counter, items))
}

/// Extract the `TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d …` phrase of a MULTI-counter
/// INSPECT (`>= 2` `tally_for` groups), returning the `(counter_name, delim_nodes)` groups
/// in WRITTEN ORDER (and, within each group, the single-char delimiter nodes in written
/// order) — the compiler-side analogue of the oracle's `read_inspect_tally_counters`,
/// walking the SAME `tally_for`/`tally_item` children so the two engines' accept/reject
/// sets stay co-total. Only called after the caller has confirmed `>= 2` `tally_for`
/// groups; exactly ONE group keeps the single-counter readers (`inspect_tally_all` /
/// `inspect_tally_multi`) and all their capabilities UNCHANGED.
///
/// Scope bound (this rung, IDENTICAL messages to the oracle reader): every item of every
/// group must be a plain `FOR ALL` item with NO `LEADING`/`CHARACTERS`; each item MAY now
/// carry its OWN optional `{BEFORE|AFTER}` region (the region reject is LIFTED this rung),
/// parsed with the SAME keyword/operand extraction `inspect_tally_all` uses on the
/// single-item side. Any violating item is a clean later-rung `Unsupported`. A
/// multi-character/figurative/wider/numeric delimiter is NOT rejected here — it falls to
/// the SAME `single_delim_code` check the single-item emitter uses. The counters are
/// validated (unsigned integer) in `emit_inspect_tally_counters`, exactly as the
/// single-item path validates its counter.
fn inspect_tally_counters(verb: &GrammarASTNode) -> Result<Vec<TallyCounterGroup<'_>>, CompileError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    let mut groups = Vec::new();
    for tf in child_nodes(tallying, "tally_for") {
        let counter = first_token(tf, "NAME")
            .ok_or_else(|| CompileError::Malformed("INSPECT TALLYING without a counter".into()))?;
        let mut items = Vec::new();
        for ti in child_nodes(tf, "tally_item") {
            let toks = child_tokens(ti);
            if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
                return Err(CompileError::Unsupported(
                    "INSPECT TALLYING … FOR CHARACTERS is a later rung".into(),
                ));
            }
            if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING") {
                return Err(CompileError::Unsupported(
                    "INSPECT TALLYING with several counters and a LEADING item is a later rung"
                        .into(),
                ));
            }
            // A `{BEFORE|AFTER} x` region on an item is now ACCEPTED (this rung): parse it
            // into `Option<(RegionKind, node)>` with the SAME keyword/operand extraction
            // `inspect_tally_all` uses on the single-item side. The region contributes its
            // own delimiter operand under the `inspect_region` child, not a direct child of
            // `tally_item`, so the DIRECT `operand` child below is still exactly the tally
            // delimiter.
            let region = match child_node(ti, "inspect_region") {
                None => None,
                Some(region_node) => {
                    let rtoks = child_tokens(region_node);
                    let kind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                        RegionKind::Before
                    } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                        RegionKind::After
                    } else {
                        return Err(CompileError::Unsupported(
                            "INSPECT region without a BEFORE or AFTER keyword".into(),
                        ));
                    };
                    let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                        CompileError::Malformed(
                            "INSPECT BEFORE/AFTER region without a delimiter".into(),
                        )
                    })?;
                    Some((kind, rdelim))
                }
            };
            let delim = child_node(ti, "operand").ok_or_else(|| {
                CompileError::Malformed("INSPECT TALLYING FOR ALL without a delimiter".into())
            })?;
            items.push((delim, region));
        }
        groups.push((counter, items));
    }
    Ok(groups)
}

/// The parsed pieces of a `REPLACING ALL|LEADING search BY replace [{BEFORE|AFTER}
/// x]` phrase: `(search_node, replace_node, leading, region)`, where `region` is the
/// optional `{BEFORE|AFTER} x` window as `(kind, region_delim_node)` — the exact
/// analogue of [`TallyPhrase`]'s region on the count side.
type ReplacePhrase<'a> =
    (&'a GrammarASTNode, &'a GrammarASTNode, bool, Option<(RegionKind, &'a GrammarASTNode)>);

/// Which flavour of replace item a MULTI-item REPLACING clause holds — the compiler-side
/// mirror of the oracle's `ReplaceMultiKind` and the replace-side twin of [`TallyKind`].
/// Picking an explicit enum (over a bare `leading: bool`) makes the illegal "LEADING and
/// ALSO CHARACTERS" state UNREPRESENTABLE and keeps the CST-side dispatch co-total with
/// the oracle reader: `ALL a BY x` → [`ReplaceKind::All`], `LEADING a BY x` →
/// [`ReplaceKind::Leading`], `CHARACTERS BY x` → [`ReplaceKind::Characters`] (no search
/// operand, always-eligible catch-all).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplaceKind {
    /// `ALL a BY x` — eligible at an in-window position whose char equals the search.
    All,
    /// `LEADING a BY x` — like `All`, but only while this item's leading run is unbroken.
    Leading,
    /// `CHARACTERS BY x` — eligible at EVERY in-window position; carries no search operand.
    Characters,
}

/// One `{ALL|LEADING} search BY replace [{BEFORE|AFTER} x]` — or bare
/// `CHARACTERS BY replace [{BEFORE|AFTER} x]` — item of a MULTI-item REPLACING clause:
/// `(search_node, replace_node, kind, region)`, where `search_node` is `None` on the
/// `CHARACTERS` path (that form carries no search operand — it overwrites positions) and
/// `Some` otherwise, `kind` is the [`ReplaceKind`] tag, and `region` is the optional
/// `{BEFORE|AFTER} x` window as `(kind, region_delim_node)` — the SAME shape
/// [`ReplacePhrase`] carries. THIS rung lifts the multi-item `CHARACTERS` reject, so —
/// mirroring the tally side's multi-item kind tag (#81) — each item now carries a
/// [`ReplaceKind`] AND its OWN optional region window. The replacement node is ALWAYS
/// present (every variant carries a `BY x`).
type ReplaceItem<'a> = (
    Option<&'a GrammarASTNode>,
    &'a GrammarASTNode,
    ReplaceKind,
    Option<(RegionKind, &'a GrammarASTNode)>,
);

/// Extract the supported `REPLACING ALL search BY replace [{BEFORE|AFTER} x]` /
/// `REPLACING LEADING search BY replace` phrase from an `inspect_stmt`, returning
/// `(search_node, replace_node, leading, region)` where `leading` is `true` for
/// `REPLACING LEADING` (replace only the leading run) and `false` for `REPLACING
/// ALL` (replace every occurrence), and `region` carries an optional `{BEFORE|AFTER}
/// x` window as `(kind, region_delim_node)`. Rejects every later-rung form the
/// grammar also accepts:
///   * more than one replace item (`{ replace_item }`) — one `x BY y` this rung;
///   * a `CHARACTERS` or `FIRST` replacement (only `ALL`/`LEADING` this rung).
///
/// A `REPLACING LEADING` phrase carrying a region is now ACCEPTED — the STANDALONE
/// `REPLACING LEADING … BEFORE/AFTER` form is supported this rung (the substitution
/// anchors the leading run at the window start), mirroring the `FOR LEADING …
/// BEFORE/AFTER` support on the count side. Both `ALL` and `LEADING` are accepted
/// here, whether the phrase is lone or combined with `TALLYING`; for the combined
/// form `emit_inspect_replacing` re-imposes the deferral of a LEADING half with a
/// region via `allow_leading_region`. (A non-alphanumeric source is rejected by the
/// caller; a multi-character/wider/figurative search, replacement, or region
/// delimiter is rejected by `single_delim_code`/`single_delim_str`.)
fn inspect_replacing_all(verb: &GrammarASTNode) -> Result<ReplacePhrase<'_>, CompileError> {
    let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a REPLACING clause is a later rung".into())
    })?;
    let items = child_nodes(replacing, "replace_item");
    let ri = match items.as_slice() {
        [one] => *one,
        _ => {
            return Err(CompileError::Unsupported(
                "INSPECT REPLACING with several replace items is a later rung".into(),
            ))
        }
    };
    let toks = child_tokens(ri);
    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
        return Err(CompileError::Unsupported(
            "INSPECT REPLACING CHARACTERS is a later rung".into(),
        ));
    }
    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "FIRST") {
        return Err(CompileError::Unsupported(
            "INSPECT REPLACING FIRST is a later rung".into(),
        ));
    }
    // `REPLACING LEADING` is now supported (replace only the leading run);
    // `REPLACING ALL` is the default. The keyword selects the stop-at-first-
    // mismatch behaviour threaded into the unroll.
    let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
    // A `{BEFORE|AFTER} x` region now PARSES into `Option<(RegionKind, node)>` (it
    // used to be rejected wholesale here) REGARDLESS of `leading`, using the SAME
    // keyword/operand extraction as `inspect_tally_all` on the count side: the
    // STANDALONE `REPLACING LEADING … BEFORE/AFTER` form is supported this rung (the
    // substitution anchors the leading run at the window start — see
    // `emit_inspect_replacing`). The COMBINED form still defers a LEADING half with a
    // region; `emit_inspect_replacing` re-imposes that via its `allow_leading_region`
    // flag, so relaxing this shared reader does not leak the combination.
    let region = match child_node(ri, "inspect_region") {
        None => None,
        Some(region_node) => {
            let rtoks = child_tokens(region_node);
            let kind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                RegionKind::Before
            } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                RegionKind::After
            } else {
                return Err(CompileError::Unsupported(
                    "INSPECT region without a BEFORE or AFTER keyword".into(),
                ));
            };
            let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                CompileError::Malformed("INSPECT BEFORE/AFTER region without a delimiter".into())
            })?;
            Some((kind, rdelim))
        }
    };
    // `ALL`/`LEADING search BY replace` — the two `operand` children are the
    // search (first) and the replacement (second), in order. (A `{BEFORE|AFTER}`
    // region contributes its own operand nested under `inspect_region`, not a direct
    // child of `replace_item`, so these two direct `operand` children are exactly the
    // search and replacement.)
    let ops = child_nodes(ri, "operand");
    match ops.as_slice() {
        [s, r] => Ok((*s, *r, leading, region)),
        _ => Err(CompileError::Malformed(
            "INSPECT REPLACING ALL/LEADING without a search and a BY replacement".into(),
        )),
    }
}

/// Extract the `REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` phrase of a
/// multi-item INSPECT, returning the `(search_node, replace_node)` pairs in WRITTEN
/// ORDER — the compiler-side analogue of the oracle's `read_inspect_replacing_multi`,
/// counting the SAME `replace_item` children so the two engines' accept/reject sets
/// stay co-total. Only called after the caller has confirmed `>= 2` items; a single
/// item keeps [`inspect_replacing_all`] and all its capabilities.
///
/// Scope (this rung, IDENTICAL messages to the oracle reader): each item is `ALL`,
/// `LEADING`, OR `CHARACTERS` — the multi-item `CHARACTERS` reject is LIFTED this rung
/// (LEADING and the per-item region were lifted earlier), mirroring the tally side's
/// `inspect_tally_multi` (#81). Each item carries a [`ReplaceKind`] tag, an OPTIONAL
/// search node (`Some` for `ALL`/`LEADING`, `None` for `CHARACTERS`), the ALWAYS-present
/// `BY` replacement node, AND its OWN optional `{BEFORE|AFTER} x` region, parsed with the
/// SAME keyword/operand extraction [`inspect_replacing_all`] uses on the single-item side.
/// A `CHARACTERS` item is the always-eligible catch-all (no search, no run). `FIRST` is
/// still deferred. Any violating item is a clean later-rung `Unsupported`. A
/// multi-character/figurative/wider/numeric search on an `ALL`/`LEADING` item, replacement,
/// or region delimiter is NOT rejected here — it falls to the SAME
/// `single_delim_code`/`single_delim_str` checks the single-item emitter uses. (The
/// COMBINED `TALLYING … REPLACING` form keeps rejecting `CHARACTERS` in its REPLACING half
/// via `inspect_replacing_all`, reached through the combined caller, not here.)
fn inspect_replacing_multi(verb: &GrammarASTNode) -> Result<Vec<ReplaceItem<'_>>, CompileError> {
    let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a REPLACING clause is a later rung".into())
    })?;
    let mut items = Vec::new();
    for ri in child_nodes(replacing, "replace_item") {
        let toks = child_tokens(ri);
        if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "FIRST") {
            return Err(CompileError::Unsupported(
                "INSPECT REPLACING FIRST is a later rung".into(),
            ));
        }
        // Classify the item into exactly one [`ReplaceKind`] variant, matching the oracle
        // reader. `CHARACTERS` in a multi-item list is now ACCEPTED (THIS rung) — the
        // always-eligible catch-all, no search operand. `LEADING` (also lifted, #71)
        // replaces only its run; `ALL` is the default. The keyword picks per-item
        // substitution semantics threaded to `emit_inspect_replacing_multi`. (A LONE
        // `REPLACING CHARACTERS BY x` or `REPLACING LEADING` is still supported via the
        // single-item path, not here.)
        let kind = if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
            ReplaceKind::Characters
        } else if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING") {
            ReplaceKind::Leading
        } else {
            ReplaceKind::All
        };
        // A `{BEFORE|AFTER} x` region on an item is ACCEPTED for EVERY kind: parse it into
        // `Option<(RegionKind, node)>` with the SAME keyword/operand extraction
        // `inspect_replacing_all` uses on the single-item side. The region contributes its
        // own delimiter operand under the `inspect_region` child, not a direct child of
        // `replace_item`, so an `ALL`/`LEADING` item's two DIRECT `operand` children (and a
        // `CHARACTERS` item's one) below are still exactly the search/replacement.
        let region = match child_node(ri, "inspect_region") {
            None => None,
            Some(region_node) => {
                let rtoks = child_tokens(region_node);
                let rkind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                    RegionKind::Before
                } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                    RegionKind::After
                } else {
                    return Err(CompileError::Unsupported(
                        "INSPECT region without a BEFORE or AFTER keyword".into(),
                    ));
                };
                let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                    CompileError::Malformed(
                        "INSPECT BEFORE/AFTER region without a delimiter".into(),
                    )
                })?;
                Some((rkind, rdelim))
            }
        };
        // A `CHARACTERS` item carries NO search operand (grammar branch
        // `CHARACTERS "BY" operand { inspect_region }`), so its SOLE direct `operand` child
        // is the replacement — the search node is `None`. `ALL`/`LEADING` read their two
        // direct operands: the search (first) and the replacement (second). A region's
        // delimiter rides on the `inspect_region` child, not here.
        let ops = child_nodes(ri, "operand");
        match kind {
            ReplaceKind::Characters => match ops.as_slice() {
                [r] => items.push((None, *r, kind, region)),
                _ => {
                    return Err(CompileError::Malformed(
                        "INSPECT REPLACING CHARACTERS without a BY replacement".into(),
                    ))
                }
            },
            _ => match ops.as_slice() {
                [s, r] => items.push((Some(*s), *r, kind, region)),
                _ => {
                    return Err(CompileError::Malformed(
                        "INSPECT REPLACING ALL/LEADING without a search and a BY replacement"
                            .into(),
                    ))
                }
            },
        }
    }
    Ok(items)
}

/// The parsed pieces of a `CONVERTING from TO to [{BEFORE|AFTER} x]` phrase:
/// `(from_node, to_node, region)`, where `region` is the optional `{BEFORE|AFTER} x`
/// window as `(kind, region_delim_node)` — the exact analogue of the region on the
/// TALLYING ([`TallyPhrase`]) and REPLACING ([`ReplacePhrase`]) sides.
type ConvertPhrase<'a> =
    (&'a GrammarASTNode, &'a GrammarASTNode, Option<(RegionKind, &'a GrammarASTNode)>);

/// Extract the `CONVERTING from TO to [{BEFORE|AFTER} x]` phrase from an
/// `inspect_stmt`, returning `(from_node, to_node, region)`. A `{BEFORE|AFTER} x`
/// region now PARSES into `Option<(RegionKind, node)>` (it used to be rejected
/// wholesale here), using the SAME keyword/operand extraction as `inspect_tally_all`
/// and `inspect_replacing_all` on the count/replace sides; a multi-character region
/// delimiter stays a later rung, rejected by `single_delim_code` at emit time.
/// (Unequal-length/non-ASCII/non-literal `from`/`to` are rejected by the caller.)
fn inspect_converting_pair(verb: &GrammarASTNode) -> Result<ConvertPhrase<'_>, CompileError> {
    let converting = child_node(verb, "inspect_converting").ok_or_else(|| {
        CompileError::Unsupported("INSPECT without a CONVERTING clause is a later rung".into())
    })?;
    let region = match child_node(converting, "inspect_region") {
        None => None,
        Some(region_node) => {
            let rtoks = child_tokens(region_node);
            let kind = if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
                RegionKind::Before
            } else if rtoks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
                RegionKind::After
            } else {
                return Err(CompileError::Unsupported(
                    "INSPECT region without a BEFORE or AFTER keyword".into(),
                ));
            };
            let rdelim = child_node(region_node, "operand").ok_or_else(|| {
                CompileError::Malformed("INSPECT BEFORE/AFTER region without a delimiter".into())
            })?;
            Some((kind, rdelim))
        }
    };
    // `from TO to` — the two `operand` children are the FROM (first) and the TO
    // (second), in order. (A `{BEFORE|AFTER}` region contributes its own operand
    // nested under `inspect_region`, not a direct child of `inspect_converting`, so
    // these two direct `operand` children are exactly the FROM and TO.)
    let ops = child_nodes(converting, "operand");
    match ops.as_slice() {
        [f, t] => Ok((*f, *t, region)),
        _ => Err(CompileError::Malformed(
            "INSPECT CONVERTING without a FROM and a TO operand".into(),
        )),
    }
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
#[derive(Clone)]
enum StrOperand {
    /// A string whose length is known at compile time (a character item's slot,
    /// a string literal, or a *constant-index* reference modification).
    Fixed { reg: String, len: usize },
    /// A **computed** reference-modification slice: its content register plus a
    /// run-time `i64` register holding its length, and a compile-time upper bound
    /// (`max_len`, the base item's width) used to size the common comparison
    /// width. The slice is right-padded with spaces to that width *at run time*.
    Runtime { reg: String, len_reg: String, max_len: usize },
    Fig(char),
}

/// The compile-time upper bound on a comparison operand's length, or `None` for
/// a figurative constant (which has no length of its own and borrows the other
/// operand's).
fn str_operand_max_len(op: &StrOperand) -> Option<usize> {
    match op {
        StrOperand::Fixed { len, .. } => Some(*len),
        StrOperand::Runtime { max_len, .. } => Some(*max_len),
        StrOperand::Fig(_) => None,
    }
}

/// The length of a reference-modification slice: known at compile time (a
/// constant-index refmod) or only at run time (a computed one).
enum SliceLen {
    /// A compile-time-constant length — the literal:literal refmod path.
    Const(usize),
    /// A run-time length: the `i64` register holding it, plus the compile-time
    /// upper bound (the base item's width).
    Runtime { len_reg: String, max_len: usize },
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
    fn evaluate_on_an_alphanumeric_subject_lowers_to_str_cmp() {
        // A character subject compares each WHEN value with str_cmp; a THRU range is
        // and(cmp_ge, cmp_le) over str_cmp results.
        let m = compile_source(
            &wrap(
                &["01  W  PIC X(3) VALUE \"ABC\"."],
                &[
                    "EVALUATE W",
                    "WHEN \"ABC\" DISPLAY \"Y\"",
                    "WHEN \"AAA\" THRU \"MMM\" DISPLAY \"R\"",
                    "END-EVALUATE.",
                    "STOP RUN.",
                ],
            ),
            "ev",
        )
        .unwrap();
        let ops = ops(&m);
        assert!(ops.contains(&"str_cmp".to_string()), "expected `str_cmp`: {ops:?}");
        assert!(ops.contains(&"and".to_string()), "expected `and` for the range: {ops:?}");
        assert!(m.validate().is_empty(), "{:?}", m.validate());
    }

    #[test]
    fn evaluate_numeric_value_against_alphanumeric_subject_is_deferred() {
        // A numeric WHEN value against a character subject is a later rung.
        let err = compile_source(
            &wrap(
                &["01  W  PIC X(3) VALUE \"ABC\"."],
                &["EVALUATE W", "WHEN 5 DISPLAY \"Y\"", "END-EVALUATE.", "STOP RUN."],
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
    fn unsigned_integer_numeric_to_alphanumeric_move_lowers() {
        // `MOVE PIC 9(3) TO PIC X(4)` is now supported: the digit image is built at
        // run time (str_slice off a digit table) and char-moved (str_concat pad).
        let m = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE 42.", "01  W  PIC X(4)."],
                &["MOVE N TO W.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "digit image slices off the table");
        assert!(os.contains(&"str_concat".to_string()), "char reshape pads via str_concat");
    }

    #[test]
    fn signed_numeric_to_alphanumeric_move_lowers() {
        // A SIGNED numeric source into an alphanumeric receiver is now supported:
        // its magnitude image is built (str_slice off the digit table) and the units
        // digit is overpunched by indexing the combined `{…I}…R` sign table
        // (str_slice) before the char reshape (str_concat).
        let m = compile_source(
            &wrap(
                &["01  S  PIC S9(3) VALUE 42.", "01  W  PIC X(4)."],
                &["MOVE S TO W.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "overpunch char slices off the sign table");
        assert!(os.contains(&"str_concat".to_string()), "image reshaped/padded via str_concat");
    }

    #[test]
    fn signed_numeric_vs_alphanumeric_comparison_now_compiles() {
        // A SIGNED numeric operand in a mixed relation is now supported: its
        // overpunched magnitude image is built (str_slice off the digit table plus the
        // sign-table overpunch) and the comparison runs the alphanumeric byte rule
        // (str_cmp). Previously this was a clean `Unsupported`.
        let m = compile_source(
            &wrap(
                &["01  S  PIC S9(3) VALUE -123."],
                &["IF S = \"12L\" DISPLAY \"T\" ELSE DISPLAY \"F\".", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "overpunch char slices off the sign table");
        assert!(os.contains(&"str_cmp".to_string()), "mixed relation compares via str_cmp");
    }

    #[test]
    fn numeric_literal_vs_alphanumeric_comparison_is_still_a_later_rung() {
        // A numeric LITERAL against an alphanumeric operand is a different pairing,
        // still out of scope on both engines.
        let err = compile_source(
            &wrap(&["01  W  PIC X(3) VALUE \"042\"."], &["IF 42 = W DISPLAY \"Y\".", "STOP RUN."]),
            "x",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn figurative_vs_figurative_comparison_now_compiles() {
        // `IF ZERO = SPACE` (two figurative constants) now compiles — each resolves to
        // a single fill character and both flow through the alphanumeric str_cmp path.
        let m = compile_source(
            &wrap(
                &["01  D  PIC X(1)."],
                &["IF ZERO = SPACE DISPLAY \"E\" ELSE DISPLAY \"N\".", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        assert!(ops(&m).contains(&"str_cmp".to_string()), "figuratives compare via str_cmp");
    }

    #[test]
    fn signed_numeric_to_group_receiver_is_deferred() {
        // A GROUP on either side of the move is still a later rung — the compiler
        // models no group items, so a group RECEIVER is `Unsupported` (the oracle
        // rejects it too, as "MOVE into a group item").
        let err = compile_source(
            &wrap(
                &[
                    "01  S  PIC S9(3) VALUE -12.",
                    "01  G.",
                    "    05  A  PIC X(2).",
                    "    05  B  PIC X(1).",
                ],
                &["MOVE S TO G.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn scaled_numeric_to_alphanumeric_move_lowers() {
        // An UNSIGNED SCALED source (`PIC 9(2)V9`) into an alphanumeric receiver is
        // now supported: its full (int + frac) digit image is built at run time (the
        // same str_slice-off-a-table loop, over `int + dec` digits — no point) and
        // char-moved (str_concat pad).
        let m = compile_source(
            &wrap(
                &["01  F  PIC 9(2)V9 VALUE 4.2.", "01  W  PIC X(4)."],
                &["MOVE F TO W.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "digit image slices off the table");
        assert!(os.contains(&"str_concat".to_string()), "char reshape pads via str_concat");
    }

    #[test]
    fn alphanumeric_to_unsigned_integer_move_lowers() {
        // The REVERSE direction (alphanumeric source → UNSIGNED INTEGER receiver)
        // is now supported: the source's bytes are folded left-to-right into the
        // integer (`str_index` reads each byte, `mul`/`add` accumulate) and stored
        // with the receiver-width truncation (`mod`).
        let m = compile_source(
            &wrap(
                &["01  W  PIC X(3) VALUE \"042\".", "01  N  PIC 9(3)."],
                &["MOVE W TO N.", "STOP RUN."],
            ),
            "x",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_index".to_string()), "each source byte is read via str_index");
        assert!(os.contains(&"mod".to_string()), "receiver-width truncation via mod");
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
    fn level_88_on_an_alphanumeric_item_with_a_discrete_string_value_lowers() {
        // A condition-name over an alphanumeric variable with a discrete string
        // VALUE now lowers to valid IIR (a `str_cmp`-based equality OR-fold).
        let m = compile_source(
            &wrap(
                &["01  FLAG  PIC X VALUE \"Y\".", "88  IS-YES  VALUE \"Y\"."],
                &["IF IS-YES DISPLAY \"YES\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_cmp".to_string()), "expected `str_cmp`: {os:?}");
    }

    #[test]
    fn level_88_alphanumeric_string_thru_range_lowers() {
        // A `THRU` range with STRING bounds on an alphanumeric conditional variable
        // now lowers to valid IIR: an `and(cmp_ge, cmp_le)` over the `str_cmp` path.
        let m = compile_source(
            &wrap(
                &["01  GRADE  PIC X VALUE \"C\".", "88  PASSING  VALUE \"A\" THRU \"D\"."],
                &["IF PASSING DISPLAY \"pass\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_cmp".to_string()), "expected `str_cmp`: {os:?}");
    }

    #[test]
    fn level_88_alphanumeric_numeric_bound_thru_range_is_still_a_later_rung() {
        // A `THRU` range with a NON-string (numeric) bound on an alphanumeric
        // conditional variable stays deferred — a clean later rung, matching the
        // oracle's own deferral.
        let err = compile_source(
            &wrap(
                &["01  GRADE  PIC X VALUE \"C\".", "88  IN-RANGE  VALUE \"A\" THRU 5."],
                &["IF IN-RANGE DISPLAY \"YES\".", "STOP RUN."],
            ),
            "c88",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn level_88_alphanumeric_numeric_value_is_still_a_later_rung() {
        // A numeric VALUE on an alphanumeric conditional variable stays deferred.
        let err = compile_source(
            &wrap(
                &["01  FLAG  PIC X VALUE \"5\".", "88  IS-FIVE  VALUE 5."],
                &["IF IS-FIVE DISPLAY \"YES\".", "STOP RUN."],
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

    #[test]
    fn refmod_display_lowers_to_a_str_slice() {
        // A reference modification in DISPLAY lowers to a constant-index str_slice.
        let m = compile_source(
            &wrap(&["01  WS  PIC X(5) VALUE \"ABCDE\"."], &["DISPLAY WS(2:3).", "STOP RUN."]),
            "rm",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "refmod DISPLAY slices: {os:?}");
        assert!(os.contains(&"print_str".to_string()));
    }

    #[test]
    fn refmod_computed_start_lowers_to_a_runtime_str_slice() {
        // A data-name start (`WS(J:2)`) is a *computed* reference modification —
        // it lowers to a run-time `str_slice` fed by `sub`/`add` over the index
        // registers, not a compile-time reject.
        let m = compile_source(
            &wrap(
                &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J  PIC 9 VALUE 2."],
                &["DISPLAY WS(J:2).", "STOP RUN."],
            ),
            "rmj",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_slice".to_string()), "computed refmod slices: {os:?}");
        assert!(os.contains(&"sub".to_string()), "start0 = start - 1 at run time: {os:?}");
    }

    #[test]
    fn refmod_signed_index_item_is_a_later_rung() {
        // A signed index item (`PIC S9`) is a later rung — the run-time slice model
        // reads an unsigned integer index only.
        let err = compile_source(
            &wrap(
                &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J  PIC S9 VALUE 2."],
                &["DISPLAY WS(J:2).", "STOP RUN."],
            ),
            "rmsj",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_fractional_index_item_is_a_later_rung() {
        // A fractional index item (`PIC 9V9`) is a later rung.
        let err = compile_source(
            &wrap(
                &["01  WS  PIC X(5) VALUE \"ABCDE\".", "01  J  PIC 9V9 VALUE 2.0."],
                &["DISPLAY WS(J:2).", "STOP RUN."],
            ),
            "rmfj",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_computed_as_move_source_into_alnum_receiver_compiles() {
        // A reference-modification MOVE source into an ALPHANUMERIC receiver is now
        // supported (this rung), including a computed (data-name) index — it lowers
        // to the run-time slice-fit path rather than being rejected as a later rung.
        compile_source(
            &wrap(
                &[
                    "01  WS  PIC X(5) VALUE \"ABCDE\".",
                    "01  J  PIC 9 VALUE 2.",
                    "01  DST PIC X(3).",
                ],
                &["MOVE WS(J:2) TO DST.", "STOP RUN."],
            ),
            "rmmv",
        )
        .expect("computed refmod MOVE source into an alphanumeric receiver compiles");
    }

    #[test]
    fn refmod_as_move_source_into_numeric_receiver_is_a_later_rung() {
        // The remaining boundary: a refmod MOVE source into a NUMERIC receiver
        // (de-editing a slice into a numeric field) is still a later rung.
        let err = compile_source(
            &wrap(
                &[
                    "01  WS  PIC X(5) VALUE \"12345\".",
                    "01  NUM PIC 9(3).",
                ],
                &["MOVE WS(1:3) TO NUM.", "STOP RUN."],
            ),
            "rmmvn",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_of_a_numeric_item_is_a_later_rung() {
        // Reference modification is defined on alphanumeric items; a numeric item
        // is a later rung.
        let err = compile_source(
            &wrap(&["01  N  PIC 9(5) VALUE 12345."], &["DISPLAY N(2:3).", "STOP RUN."]),
            "rmn",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_out_of_range_constant_is_a_later_rung() {
        // A constant slice past the end of the item is rejected at compile time
        // (a later rung) — never lowered to a runtime trap.
        let err = compile_source(
            &wrap(&["01  WS  PIC X(3) VALUE \"ABC\"."], &["DISPLAY WS(2:5).", "STOP RUN."]),
            "rmoob",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn refmod_huge_indices_do_not_overflow() {
        // Both start and length parse as full `usize`, so `start-1 + len` would
        // overflow: a crafted program must be a clean Unsupported reject, never a
        // panic (the subtractive bounds test guarantees this).
        let err = compile_source(
            &wrap(
                &["01  WS  PIC X(5) VALUE \"ABCDE\"."],
                &["DISPLAY WS(18446744073709551615:18446744073709551615).", "STOP RUN."],
            ),
            "rmhuge",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn string_compiles_to_str_ops_and_validates() {
        // The happy path lowers to string primitives (a concat chain + the
        // slice/concat overlay) and passes IIR validation.
        let module = compile_source(
            &wrap(
                &[
                    "01  A  PIC X(3) VALUE \"ABC\".",
                    "01  B  PIC X(2) VALUE \"DE\".",
                    "01  T  PIC X(10) VALUE SPACES.",
                ],
                &["STRING A B DELIMITED BY SIZE INTO T.", "DISPLAY T.", "STOP RUN."],
            ),
            "str",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_concat".to_string()), "STRING concatenates sources");
        assert!(os.contains(&"str_slice".to_string()), "STRING preserves the receiver tail");
    }

    #[test]
    fn string_delimited_by_ascii_delimiter_emits_a_scan_loop() {
        // A real single-char ASCII delimiter is now supported: each field is
        // truncated at its first delimiter char by a run-time scan (str_index +
        // cmp_eq), then the prefixes are concatenated and overlaid.
        let module = compile_source(
            &wrap(
                &["01  A  PIC X(5) VALUE \"AB-CD\".", "01  T  PIC X(6) VALUE SPACES."],
                &["STRING A DELIMITED BY \"-\" INTO T.", "DISPLAY T.", "STOP RUN."],
            ),
            "str_delim",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_index".to_string()), "STRING delim scans for the delimiter");
        assert!(os.contains(&"cmp_eq".to_string()), "STRING delim compares each byte");
        assert!(os.contains(&"str_slice".to_string()), "STRING delim slices the prefix");
    }

    #[test]
    fn string_non_ascii_literal_delimiter_is_a_later_rung() {
        // A single but NON-ASCII delimiter (`"é"`, one char / two bytes) would make
        // the byte-based scan diverge from the char-based oracle — a clean
        // Unsupported on both engines, keeping them co-total.
        let err = compile_source(
            &wrap(
                &["01  A  PIC X(3) VALUE \"ABC\".", "01  T  PIC X(6) VALUE SPACES."],
                &["STRING A DELIMITED BY \"é\" INTO T.", "STOP RUN."],
            ),
            "str_delim",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn string_with_pointer_emits_a_guarded_overlay() {
        // WITH POINTER over a `PIC 9(n)` pointer now compiles: the overlay reads the
        // pointer register to seat the start, guards the out-of-range case (`cmp_lt`
        // for p < 1, `cmp_gt` for p > size), computes `p - 1` (a `sub`), and writes
        // the resume position back (`store_scaled` reshape). It passes validation.
        let module = compile_source(
            &wrap(
                &[
                    "01  A  PIC X(3) VALUE \"ABC\".",
                    "01  T  PIC X(6) VALUE SPACES.",
                    "01  P  PIC 9(2) VALUE 1.",
                ],
                &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
            ),
            "str_ptr",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"cmp_lt".to_string()), "guards p < 1 (out of range low)");
        assert!(os.contains(&"cmp_gt".to_string()), "guards p > size (out of range high)");
        assert!(os.contains(&"sub".to_string()), "start offset p - 1");
    }

    #[test]
    fn string_with_signed_pointer_is_a_later_rung() {
        // The pointer must be an UNSIGNED integer. A signed pointer (`PIC S9`) is a
        // clean later rung, rejected at build time with the same message the oracle
        // raises at exec time.
        let err = compile_source(
            &wrap(
                &[
                    "01  A  PIC X(3) VALUE \"ABC\".",
                    "01  T  PIC X(6) VALUE SPACES.",
                    "01  P  PIC S9(2) VALUE 1.",
                ],
                &["STRING A DELIMITED BY SIZE INTO T WITH POINTER P.", "STOP RUN."],
            ),
            "str_ptr_signed",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn unstring_compiles_to_scan_loop_and_validates() {
        // The happy path lowers to a data-dependent scan LOOP: str_len over the
        // source, a str_index/cmp scan for each delimiter, and a
        // str_slice/str_concat reshape into each receiver. It passes IIR
        // validation.
        let module = compile_source(
            &wrap(
                &[
                    "01  S  PIC X(5) VALUE \"A,B,C\".",
                    "01  R1 PIC X(3) VALUE SPACES.",
                    "01  R2 PIC X(3) VALUE SPACES.",
                    "01  R3 PIC X(3) VALUE SPACES.",
                ],
                &[
                    "UNSTRING S DELIMITED BY \",\" INTO R1 R2 R3.",
                    "STOP RUN.",
                ],
            ),
            "unstr",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_len".to_string()), "scans the source length");
        assert!(os.contains(&"str_index".to_string()), "reads source bytes to find the delimiter");
        assert!(os.contains(&"str_slice".to_string()), "cuts each field and pads it");
        assert!(os.contains(&"str_concat".to_string()), "reshapes the field into the receiver");
    }

    #[test]
    fn unstring_with_pointer_emits_a_guarded_scan() {
        // WITH POINTER over a `PIC 9(n)` pointer now compiles: the scan reads the
        // pointer register to seat the start, guards the out-of-range case, and
        // writes the resume position back (`store_scaled` reshape). A `sub` (p − 1)
        // start offset and the extra `cmp_lt`/`cmp_gt` guards are the tell.
        let module = compile_source(
            &wrap(
                &[
                    "01  S  PIC X(3) VALUE \"A,B\".",
                    "01  R1 PIC X(3) VALUE SPACES.",
                    "01  P  PIC 9(2) VALUE 1.",
                ],
                &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
            ),
            "unstr_ptr",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"cmp_lt".to_string()), "guards p < 1 (out of range low)");
        assert!(os.contains(&"cmp_gt".to_string()), "guards p > len (out of range high)");
        assert!(os.contains(&"sub".to_string()), "start offset p - 1");
    }

    #[test]
    fn unstring_with_signed_pointer_is_a_later_rung() {
        // The pointer must be an UNSIGNED integer. A signed pointer (`PIC S9`) is a
        // clean later rung, rejected at build time with the same message the oracle
        // raises at exec time.
        let err = compile_source(
            &wrap(
                &[
                    "01  S  PIC X(3) VALUE \"A,B\".",
                    "01  R1 PIC X(3) VALUE SPACES.",
                    "01  P  PIC S9(2) VALUE 1.",
                ],
                &["UNSTRING S DELIMITED BY \",\" INTO R1 WITH POINTER P.", "STOP RUN."],
            ),
            "unstr_ptr_signed",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn unstring_multi_character_delimiter_is_a_later_rung() {
        // A single delimiter CHARACTER only this rung; a 2-char literal needs a
        // multi-char scan — a clean Unsupported.
        let err = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"A::B\".", "01  R1 PIC X(3) VALUE SPACES."],
                &["UNSTRING S DELIMITED BY \"::\" INTO R1.", "STOP RUN."],
            ),
            "unstr_multi",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn unstring_numeric_receiver_is_a_later_rung() {
        // A numeric receiver would need numeric editing on receipt — a later rung.
        let err = compile_source(
            &wrap(
                &["01  S  PIC X(3) VALUE \"1,2\".", "01  N  PIC 9(3) VALUE 0."],
                &["UNSTRING S DELIMITED BY \",\" INTO N.", "STOP RUN."],
            ),
            "unstr_num",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_compiles_to_scan_loop_and_validates() {
        // The happy path lowers to a data-dependent count LOOP (str_len over the
        // source, a str_index/cmp scan bumping a counter register, then an add into
        // the counter slot) and passes IIR validation.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(6) VALUE \"BANANA\".", "01  C  PIC 9(3) VALUE 0."],
                &["INSPECT S TALLYING C FOR ALL \"A\".", "DISPLAY C.", "STOP RUN."],
            ),
            "insp",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_len".to_string()), "scans the source length");
        assert!(os.contains(&"str_index".to_string()), "reads source bytes to match the delimiter");
        assert!(os.contains(&"cmp_eq".to_string()), "compares each byte to the delimiter");
        assert!(os.contains(&"add".to_string()), "bumps the count and folds it into the counter");
    }

    #[test]
    fn inspect_replacing_compiles_to_source_rebuild_and_validates() {
        // The happy REPLACING path unrolls a per-position rebuild of the source:
        // str_index reads each byte, cmp_eq tests it against the search, and
        // str_slice/str_concat splice either the replacement or the original
        // character into the accumulator. It passes IIR validation.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"ABABA\"."],
                &["INSPECT S REPLACING ALL \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
            ),
            "insp_repl",
        )
        .unwrap();
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_index".to_string()), "reads each source byte");
        assert!(os.contains(&"cmp_eq".to_string()), "compares each byte to the search");
        assert!(os.contains(&"str_concat".to_string()), "rebuilds the source string");
    }

    #[test]
    fn inspect_replacing_characters_now_compiles() {
        // REPLACING CHARACTERS BY replaces every position UNCONDITIONALLY — now
        // supported (fill the whole field with the replacement char). Unlike
        // REPLACING ALL there is no per-position compare, so the lowering rebuilds the
        // source purely by concatenating the replacement `width` times: `str_concat`
        // appears, but NO `cmp_eq` (nothing is compared).
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"ABABA\"."],
                &["INSPECT S REPLACING CHARACTERS BY \"X\".", "STOP RUN."],
            ),
            "insp_repl_chars",
        )
        .expect("compiles");
        assert!(module.validate().is_empty(), "{:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"str_concat".to_string()), "rebuilds the source string");
        assert!(
            !os.contains(&"cmp_eq".to_string()),
            "CHARACTERS replaces unconditionally — no per-position compare"
        );
    }

    #[test]
    fn inspect_replacing_leading_now_compiles() {
        // A lone REPLACING LEADING replaces only the run of consecutive matches at
        // the start of the source — now supported. It threads an `active` flag
        // (an extra `and` per position) through the same per-position unroll and
        // must compile to a valid module.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AABBB\"."],
                &["INSPECT S REPLACING LEADING \"A\" BY \"X\".", "DISPLAY S.", "STOP RUN."],
            ),
            "insp_repl_lead",
        )
        .expect("lone REPLACING LEADING should compile");
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"and".to_string()), "leading run uses an `and` active flag: {os:?}");
    }

    #[test]
    fn inspect_combined_tally_replacing_leading_now_compiles() {
        // A combined `TALLYING … REPLACING LEADING` is now supported — the LEADING
        // replace half threads the same `active` run flag inside the combined form.
        // The module must compile, validate, and carry the leading-run `and`.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
                &[
                    "INSPECT S TALLYING C FOR ALL \"B\" REPLACING LEADING \"A\" BY \"X\".",
                    "DISPLAY S.",
                    "STOP RUN.",
                ],
            ),
            "insp_comb_repl_lead",
        )
        .expect("combined TALLYING … REPLACING LEADING should compile");
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
        let os = ops(&module);
        assert!(os.contains(&"and".to_string()), "leading run uses an `and` active flag: {os:?}");
    }

    #[test]
    fn inspect_replacing_multi_character_search_is_a_later_rung() {
        // A 2-char search needs a multi-char scan — a clean Unsupported.
        let err = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AB::B\"."],
                &["INSPECT S REPLACING ALL \"::\" BY \"XY\".", "STOP RUN."],
            ),
            "insp_repl_multi",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_replacing_several_items_now_compiles() {
        // Two `ALL … BY …` replace items in one INSPECT is now supported (the
        // multi-item first-match-wins path) — it must compile cleanly to a valid
        // module. (Byte-for-byte agreement with the oracle is pinned by the e2e
        // `assert_matches_oracle` cases.)
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"ABABA\"."],
                &["INSPECT S REPLACING ALL \"A\" BY \"X\" ALL \"B\" BY \"Y\".", "STOP RUN."],
            ),
            "insp_repl_many",
        )
        .expect("multi-item INSPECT REPLACING should compile");
        assert!(module.validate().is_empty(), "module should validate: {:?}", module.validate());
    }

    #[test]
    fn inspect_tallying_replacing_now_compiles() {
        // The combined INSPECT … TALLYING … REPLACING form is now supported — it
        // composes the two existing lowerings (tally FIRST, then replace), so it
        // must compile cleanly to a valid module.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"ABABA\".", "01  C  PIC 9(3) VALUE 0."],
                &[
                    "INSPECT S TALLYING C FOR ALL \"A\" REPLACING ALL \"B\" BY \"X\".",
                    "STOP RUN.",
                ],
            ),
            "insp_tr",
        )
        .expect("combined INSPECT should compile");
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
    }

    #[test]
    fn inspect_combined_tally_for_leading_now_compiles() {
        // The combined form's TALLYING half may now be FOR LEADING: it counts only
        // the leading run of the delimiter, then the REPLACING ALL rebuild runs. It
        // reuses the lone-TALLYING `leading` lowering, so the module must compile
        // cleanly and the tally loop's leading-run break emits an `and` active flag.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
                &[
                    "INSPECT S TALLYING C FOR LEADING \"A\" REPLACING ALL \"B\" BY \"X\".",
                    "STOP RUN.",
                ],
            ),
            "insp_tr_lead",
        )
        .expect("combined TALLYING FOR LEADING with REPLACING ALL should compile");
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
    }

    #[test]
    fn inspect_multi_character_delimiter_is_a_later_rung() {
        // A single delimiter CHARACTER only this rung; a 2-char literal is rejected.
        let err = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AB::B\".", "01  C  PIC 9(3) VALUE 0."],
                &["INSPECT S TALLYING C FOR ALL \"::\".", "STOP RUN."],
            ),
            "insp_multi",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn inspect_leading_tally_now_compiles() {
        // A lone FOR LEADING tally counts only the run of consecutive matches at the
        // start of the source — now supported; it must compile to a valid module.
        let module = compile_source(
            &wrap(
                &["01  S  PIC X(5) VALUE \"AABBB\".", "01  C  PIC 9(3) VALUE 0."],
                &["INSPECT S TALLYING C FOR LEADING \"A\".", "STOP RUN."],
            ),
            "insp_lead",
        )
        .expect("lone FOR LEADING tally should compile");
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
    }

    // Alphanumeric → numeric MOVE lowering shapes.

    #[test]
    fn alphanumeric_to_signed_numeric_move_lowers() {
        // A SIGNED receiver (`PIC S9`) is now supported (guard relaxed to any numeric
        // receiver). The alphanumeric source has NO operational sign, so the fold's
        // MAGNITUDE is stored POSITIVE: we `emit_abs` the fold before `store_scaled`
        // so `reapply_sign` is a no-op. The lowering still reads each byte via
        // `str_index` and truncates to receiver width via `mod`, exactly like the
        // unsigned path.
        let m = compile_source(
            &wrap(
                &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC S9(3)."],
                &["MOVE A TO N.", "STOP RUN."],
            ),
            "a2n_signed",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_index".to_string()), "each source byte is read via str_index");
        assert!(os.contains(&"mod".to_string()), "receiver-width truncation via mod");
    }

    #[test]
    fn alphanumeric_to_scaled_numeric_move_lowers() {
        // A SCALED receiver (`PIC 9(i)V9(d)`, non-zero fractional digits) is now
        // supported: the source bytes fold into the integer, which IS the scaled
        // slot magnitude at scale `d`; `store_scaled` keeps the low-order (i+d)
        // digits via `mod`. No up-scale `mul` is emitted (from-scale == to-scale).
        let m = compile_source(
            &wrap(
                &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC 9(2)V9."],
                &["MOVE A TO N.", "STOP RUN."],
            ),
            "a2n_scaled",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_index".to_string()), "each source byte is read via str_index");
        assert!(os.contains(&"mod".to_string()), "receiver-width truncation via mod");
    }

    #[test]
    fn alphanumeric_to_signed_scaled_numeric_move_lowers() {
        // A SIGNED SCALED receiver (`PIC S9V9`) is now supported too — the signed
        // guard relaxation composes with the scaled path: the fold's magnitude is
        // the scaled-slot value at scale `d`, stored POSITIVE (source has no sign).
        let m = compile_source(
            &wrap(
                &["01  A  PIC X(3) VALUE \"042\".", "01  N  PIC S9V9."],
                &["MOVE A TO N.", "STOP RUN."],
            ),
            "a2n_signed_scaled",
        )
        .unwrap();
        assert!(m.validate().is_empty(), "{:?}", m.validate());
        let os = ops(&m);
        assert!(os.contains(&"str_index".to_string()), "each source byte is read via str_index");
        assert!(os.contains(&"mod".to_string()), "receiver-width truncation via mod");
    }

    #[test]
    fn alphanumeric_to_numeric_wide_source_is_a_later_rung() {
        // A source wider than 18 characters could overflow the i64 fold — a later
        // rung on both engines.
        let err = compile_source(
            &wrap(
                &["01  A  PIC X(19) VALUE \"0000000000000000042\".", "01  N  PIC 9(3)."],
                &["MOVE A TO N.", "STOP RUN."],
            ),
            "a2n_wide",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn group_to_numeric_move_is_a_later_rung() {
        // A GROUP source (no picture) into a numeric receiver is a later rung.
        let err = compile_source(
            &wrap(
                &[
                    "01  G.",
                    "    05  A  PIC X(2) VALUE \"04\".",
                    "    05  B  PIC X(1) VALUE \"2\".",
                    "01  N  PIC 9(3).",
                ],
                &["MOVE G TO N.", "STOP RUN."],
            ),
            "grp2n",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }
}

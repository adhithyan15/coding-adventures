//! # COBOL-60 → IIR compiler
//!
//! Lowers a parsed COBOL-60 program (the generic `GrammarASTNode` CST from
//! [`coding_adventures_cobol_parser`]) into an [`interpreter_ir::IIRModule`], so
//! COBOL runs on every execution backend the LANG VM AOT chain targets
//! (NativeAOT / LLVM / WASM / JVM / CLR / VM / JIT). See
//! [PL09](../../../specs/PL09-codegen.md); the tree-walk interpreter
//! [`coding_adventures_cobol_runtime`] is the semantic oracle.
//!
//! ## This slice (v0.1 — the `DISPLAY` / `MOVE` / `STOP RUN` core)
//!
//! COBOL's WORKING-STORAGE is a **PICTURE-typed** data model. The first rung
//! lowers the three verbs that need no arithmetic — and it does so as **pure
//! string I/O**, which is the crucial simplification that makes it exact:
//!
//! | COBOL | IIR |
//! | --- | --- |
//! | elementary item `01 N PIC 9(5)` | one `str` register holding its stored PICTURE image |
//! | `VALUE <lit>` | the register's initial `str_const` (literal formatted into the picture *at compile time*) |
//! | `MOVE <lit> TO item` | re-`str_const` the receiver to the literal, formatted into *its* picture |
//! | `DISPLAY op…` | each operand's image `print_str`'d in turn, then `putchar('\n')` |
//! | `STOP RUN` | `ret 0` |
//!
//! ### Why compile-time formatting is exact
//!
//! A COBOL numeric item does not display as a plain integer — `PIC 9(5)` holding
//! 42 displays as `00042`, and `PIC 9(2)V9` holding 123.456 displays as `234`
//! (truncated, implied point). That shaping is the receiver-picture logic in
//! [`coding_adventures_cobol_runtime`]'s `move_into_numeric` / `move_into_char`.
//! Because this rung has **no arithmetic**, every value a program stores is
//! already known at compile time (a `VALUE` clause or a `MOVE` of a *literal*).
//! So we call the very same picture/value functions the oracle uses, at compile
//! time, and emit the resulting digit string as a `str` constant — the DISPLAYed
//! bytes are byte-identical to the interpreter's by construction.
//!
//! A numeric *literal*, by contrast, displays as its **source text** (`DISPLAY 42`
//! prints `42`, not `00042`) — COBOL only reshapes a value when it lands in a
//! field. We honour that distinction: a literal operand prints its token text; a
//! data-name operand prints its item register's stored image.
//!
//! ### Deliberately a later rung (each a clean [`CompileError::Unsupported`],
//! never wrong output)
//!
//! Item-to-item `MOVE` (needs runtime picture reshaping), arithmetic
//! (`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`/`COMPUTE`), `IF`, `PERFORM`, `GO TO`,
//! group items, and signed numerics (`PIC S9…`, trailing-overpunch display) each
//! come on their own rung. A program that reaches one gets a descriptive error,
//! matching the honest-failure discipline of the runtime.

use coding_adventures_cobol_parser::try_parse_cobol;
use coding_adventures_cobol_runtime::{move_into_char, move_into_numeric, Decimal, Picture};
use interpreter_ir::function::FunctionTypeStatus;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::HashMap;

/// A COBOL → IIR compilation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// The source did not parse (lexical or syntactic).
    Parse(String),
    /// A construct not yet lowered in this slice (e.g. arithmetic, `PERFORM`).
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
    // `"any"`); assert FullyTyped as the BASIC / FLOW-MATIC frontends do, so the
    // auto-inference does not downgrade `main` to PartiallyTyped over the
    // `"void"` control hints.
    main.type_status = FunctionTypeStatus::FullyTyped;

    let mut module = IIRModule::new(module_name, "cobol");
    module.functions.push(main);
    module.entry_point = Some("main".to_string());
    Ok(module)
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

/// A WORKING-STORAGE elementary item: its receiver [`Picture`] and its current
/// stored image (the exact digit/character string the oracle would hold).
struct Item {
    picture: Picture,
    /// IIR `str` register backing this item.
    reg: String,
    /// The item's initial stored image (from `VALUE`, or the picture default).
    initial: String,
}

#[derive(Default)]
struct Compiler {
    instrs: Vec<IIRInstr>,
    /// Elementary items by COBOL data-name, in declaration order (for a stable
    /// init prologue) plus a name index for `MOVE` / `DISPLAY` lookup.
    items: Vec<Item>,
    by_name: HashMap<String, usize>,
    /// Unique-suffix counter for the throwaway `str_const` registers a literal
    /// `DISPLAY` operand needs.
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

        // 2. Prologue: initialise every item register to its stored image, in
        //    declaration order. Every item is thus defined before any use — the
        //    same discipline the FLOW-MATIC frontend uses to zero its fields.
        for idx in 0..self.items.len() {
            let reg = self.items[idx].reg.clone();
            let init = self.items[idx].initial.clone();
            self.emit("str_const", Some(&reg), vec![Operand::Str(init)], "str");
        }

        // 3. The PROCEDURE DIVISION's statements, in document order. With no
        //    PERFORM / GO TO on this rung, paragraph boundaries carry no runtime
        //    meaning — control simply falls through top to bottom.
        let pd = child_node(program, "procedure_division")
            .ok_or_else(|| CompileError::Malformed("program without a PROCEDURE DIVISION".into()))?;
        for para in child_nodes(pd, "paragraph") {
            for sentence in child_nodes(para, "sentence") {
                for stmt in child_nodes(sentence, "statement") {
                    self.emit_statement(stmt)?;
                }
            }
        }

        // 4. A trailing `ret 0` guarantees `main` returns even if the program
        //    falls off the end without a STOP RUN.
        self.emit_ret_zero();
        Ok(())
    }

    /// Populate [`Self::items`] from the DATA DIVISION's WORKING-STORAGE. Only
    /// elementary items (those with a PICTURE) are modelled on this rung; a group
    /// item (no PICTURE) is skipped and any later reference to it errors cleanly.
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
            // A group item — deferred to a later rung. Leaving it unregistered
            // means a DISPLAY/MOVE of it errors honestly rather than misbehaving.
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

        // The initial stored image: a VALUE clause applied as an initialising
        // MOVE (exactly the oracle's rule), else the picture default (zeros for
        // numeric, spaces for character).
        let initial = match find_clause(entry, "value_clause") {
            Some(vc) => {
                let lit = child_node(vc, "literal")
                    .ok_or_else(|| CompileError::Malformed("VALUE without a literal".into()))?;
                let src = read_literal(lit)?;
                format_into_picture(&src, &picture).map_err(|m| {
                    CompileError::Unsupported(format!("VALUE {name}: {m}"))
                })?
            }
            None => default_image(&picture),
        };

        let reg = format!("itm_{}", sanitise(&name));
        let idx = self.items.len();
        self.items.push(Item { picture, reg, initial });
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
            other => Err(CompileError::Unsupported(format!(
                "the {} statement is a later rung",
                verb_name(other)
            ))),
        }
    }

    /// `DISPLAY op…` — print each operand's image with no separator, then a
    /// newline. A literal prints its source text; a data-name prints its item
    /// register's stored image.
    fn emit_display(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        for op in child_nodes(verb, "operand") {
            match read_operand(op)? {
                Operandy::Name(name) => {
                    let reg = self.item_reg(&name)?;
                    self.emit("print_str", None, vec![Operand::Var(reg)], "void");
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

    /// `MOVE src TO recv…` — on this rung the source must be a literal (an
    /// item-to-item move needs runtime picture reshaping). Each receiver is
    /// re-initialised to the literal formatted into *its* picture, computed at
    /// compile time and emitted as a fresh `str_const`.
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
        // Receivers are the direct NAME tokens after TO (the source, if a name,
        // sits inside the `operand` node, so it is not among these).
        let dsts: Vec<String> = child_tokens(verb)
            .into_iter()
            .filter(|(k, _)| k == "NAME")
            .map(|(_, v)| v)
            .collect();
        if dsts.is_empty() {
            return Err(CompileError::Malformed("MOVE without a receiver".into()));
        }
        for dst in dsts {
            let idx = *self
                .by_name
                .get(&dst)
                .ok_or_else(|| CompileError::Unsupported(format!(
                    "MOVE into {dst} (a group item or undeclared name — a later rung)"
                )))?;
            let picture = self.items[idx].picture.clone();
            let image = format_into_picture(&src, &picture)
                .map_err(|m| CompileError::Unsupported(format!("MOVE into {dst}: {m}")))?;
            let reg = self.items[idx].reg.clone();
            self.emit("str_const", Some(&reg), vec![Operand::Str(image)], "str");
        }
        Ok(())
    }

    /// `STOP RUN` → `ret 0`. `STOP <literal>` (an operator pause) is not modelled.
    fn emit_stop(&mut self, verb: &GrammarASTNode) -> Result<(), CompileError> {
        let has_run = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "RUN");
        if has_run {
            self.emit_ret_zero();
            Ok(())
        } else {
            Err(CompileError::Unsupported("STOP <literal> (only STOP RUN is modelled)".into()))
        }
    }

    /// The `str` register for a data-name, or a clean error if it is not a
    /// declared elementary item (a group item or an undeclared name).
    fn item_reg(&self, name: &str) -> Result<String, CompileError> {
        self.by_name
            .get(name)
            .map(|&i| self.items[i].reg.clone())
            .ok_or_else(|| CompileError::Unsupported(format!(
                "DISPLAY of {name} (a group item or undeclared name — a later rung)"
            )))
    }

    /// `putchar('\n')` — a `const` then the host `putchar` builtin, the record
    /// terminator every `DISPLAY` appends.
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
}

// ---------------------------------------------------------------------------
// Literals & picture formatting (the reuse boundary with cobol-runtime)
// ---------------------------------------------------------------------------

/// A source value in flight: a literal (this rung's only `MOVE`/`VALUE` source)
/// or a data-name reference (a `DISPLAY` operand).
enum Operandy {
    Literal(Src),
    Name(String),
}

/// A literal source, mirroring the runtime's `Src` for the values this rung
/// handles. The figurative constants collapse to the two the v0.1 runtime knows.
enum Src {
    /// A numeric literal, kept as its source text so a *literal* `DISPLAY` shows
    /// it verbatim; parsed to a [`Decimal`] only when it lands in a numeric item.
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

/// The default initial image of an item with no VALUE: zeros for a numeric
/// picture, spaces for a character picture (the oracle's `build_items` rule).
fn default_image(picture: &Picture) -> String {
    if picture.is_numeric() {
        "0".repeat(picture.size())
    } else {
        " ".repeat(picture.size())
    }
}

/// Format a literal source into a receiver picture's stored image — the exact
/// transform the oracle performs in `move_into`, reusing cobol-runtime's own
/// picture/value logic so the result is byte-identical. Returns a message for
/// the category errors the runtime also rejects (alphanumeric → numeric,
/// `SPACES` → numeric).
fn format_into_picture(src: &Src, picture: &Picture) -> Result<String, String> {
    match picture {
        Picture::Numeric { int_digits, dec_digits, .. } => {
            let d = match src {
                Src::Num(s) => Decimal::parse_literal(s)
                    .ok_or_else(|| format!("numeric literal {s}"))?,
                Src::Zero => Decimal::zero(),
                Src::Space => return Err("MOVE SPACES to a numeric item".into()),
                Src::Str(_) => return Err("MOVE of an alphanumeric value to a numeric item".into()),
            };
            Ok(move_into_numeric(&d, *int_digits, *dec_digits))
        }
        Picture::Alphanumeric { size } | Picture::Alphabetic { size } => {
            let chars = match src {
                Src::Str(s) => s.clone(),
                // A numeric literal moved into a character field contributes its
                // digits (no point), matching the runtime's `Decimal::digits`.
                Src::Num(s) => Decimal::parse_literal(s)
                    .ok_or_else(|| format!("numeric literal {s}"))?
                    .digits(),
                Src::Zero => "0".repeat(*size),
                Src::Space => " ".repeat(*size),
            };
            Ok(move_into_char(&chars, *size))
        }
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
/// COBOL user words draw from letters, digits and hyphens, so this is injective
/// over any single program's names.
fn sanitise(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// A human-readable verb name for the "not yet lowered" error.
fn verb_name(rule: &str) -> &str {
    match rule {
        "add_stmt" => "ADD",
        "subtract_stmt" => "SUBTRACT",
        "multiply_stmt" => "MULTIPLY",
        "divide_stmt" => "DIVIDE",
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

    /// Assemble a program from code lines, carding each (6 sequence columns + a
    /// space indicator, then code from column 8) — the fixed 80-column format.
    fn program(lines: &[&str]) -> String {
        lines.iter().map(|l| format!("000000 {l}")).collect::<Vec<_>>().join("\n")
    }

    /// A minimal well-formed program wrapping the given DATA and PROCEDURE lines.
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

    /// Every `str_const`'s emitted string literal, in order.
    fn str_consts(module: &IIRModule) -> Vec<String> {
        module.functions[0]
            .instructions
            .iter()
            .filter(|i| i.op == "str_const")
            .filter_map(|i| match i.srcs.first() {
                Some(Operand::Str(s)) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn hello_world_compiles_and_validates() {
        let module = compile_source(
            &wrap(&[], &["DISPLAY \"HELLO, WORLD\".", "STOP RUN."]),
            "hello",
        )
        .unwrap();
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.entry_point.as_deref(), Some("main"));
        assert!(module.validate().is_empty(), "validate: {:?}", module.validate());
        // The literal is a str_const, printed, then a newline putchar, then ret.
        let os = ops(&module);
        assert!(os.contains(&"str_const".to_string()));
        assert!(os.contains(&"print_str".to_string()));
        assert!(os.contains(&"call_builtin".to_string())); // putchar
        assert!(os.contains(&"ret".to_string()));
        assert_eq!(str_consts(&module), vec!["HELLO, WORLD".to_string()]);
    }

    #[test]
    fn numeric_value_is_zero_filled_at_compile_time() {
        // 01 N PIC 9(5) VALUE 42 → the register initialises to "00042".
        let module = compile_source(
            &wrap(&["01  N  PIC 9(5) VALUE 42."], &["DISPLAY N.", "STOP RUN."]),
            "n",
        )
        .unwrap();
        assert!(module.validate().is_empty());
        // The item's init str_const carries the zero-filled digits.
        assert!(str_consts(&module).contains(&"00042".to_string()));
    }

    #[test]
    fn move_literal_reshapes_to_receiver_picture() {
        // MOVE "HI" TO PIC X(5) → the receiver register is re-set to "HI   ".
        let module = compile_source(
            &wrap(&["01  W  PIC X(5)."], &["MOVE \"HI\" TO W.", "DISPLAY W.", "STOP RUN."]),
            "m",
        )
        .unwrap();
        assert!(module.validate().is_empty());
        let cs = str_consts(&module);
        // The default (spaces) init and then the MOVE image "HI   " both appear;
        // the MOVE image is the one displayed.
        assert!(cs.contains(&"HI   ".to_string()), "{cs:?}");
    }

    #[test]
    fn numeric_move_truncates_with_implied_point() {
        // MOVE 123.456 TO PIC 9(2)V9 → "234" (integer keeps "23", fraction "4").
        let module = compile_source(
            &wrap(&["01  N  PIC 9(2)V9."], &["MOVE 123.456 TO N.", "STOP RUN."]),
            "t",
        )
        .unwrap();
        assert!(str_consts(&module).contains(&"234".to_string()));
    }

    #[test]
    fn display_literal_shows_source_text_not_picture() {
        // A numeric *literal* displays verbatim — "42", never zero-filled.
        let module = compile_source(&wrap(&[], &["DISPLAY 42.", "STOP RUN."]), "l").unwrap();
        assert_eq!(str_consts(&module), vec!["42".to_string()]);
    }

    #[test]
    fn figurative_value_initialises_the_field() {
        // VALUE ZERO on 9(3) → "000"; VALUE SPACES on X(4) → "    ".
        let module = compile_source(
            &wrap(
                &["01  N  PIC 9(3) VALUE ZERO.", "01  S  PIC X(4) VALUE SPACES."],
                &["STOP RUN."],
            ),
            "f",
        )
        .unwrap();
        let cs = str_consts(&module);
        assert!(cs.contains(&"000".to_string()), "{cs:?}");
        assert!(cs.contains(&"    ".to_string()), "{cs:?}");
    }

    #[test]
    fn multi_receiver_move_sets_every_target() {
        let module = compile_source(
            &wrap(
                &["01  A  PIC 9(3).", "01  B  PIC 9(4)."],
                &["MOVE 7 TO A B.", "STOP RUN."],
            ),
            "mm",
        )
        .unwrap();
        let cs = str_consts(&module);
        assert!(cs.contains(&"007".to_string()), "{cs:?}");
        assert!(cs.contains(&"0007".to_string()), "{cs:?}");
    }

    #[test]
    fn arithmetic_is_a_clean_unsupported_error() {
        let err = compile_source(
            &wrap(&["01  R  PIC 9(3) VALUE 0."], &["ADD 1 TO R.", "STOP RUN."]),
            "a",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
        assert!(err.to_string().contains("ADD"), "{err}");
    }

    #[test]
    fn item_to_item_move_is_a_clean_unsupported_error() {
        let err = compile_source(
            &wrap(
                &["01  A  PIC 9(3) VALUE 5.", "01  B  PIC 9(3)."],
                &["MOVE A TO B.", "STOP RUN."],
            ),
            "i",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn signed_item_is_a_clean_unsupported_error() {
        let err = compile_source(
            &wrap(&["01  N  PIC S9(3) VALUE -1."], &["STOP RUN."]),
            "s",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn alphanumeric_into_numeric_is_rejected_like_the_oracle() {
        let err = compile_source(
            &wrap(&["01  N  PIC 9(3)."], &["MOVE \"AB\" TO N.", "STOP RUN."]),
            "an",
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn parse_error_surfaces() {
        // No PROCEDURE DIVISION → a parse error.
        let err = compile_source(&program(&["IDENTIFICATION DIVISION.", "PROGRAM-ID. P."]), "p")
            .unwrap_err();
        assert!(matches!(err, CompileError::Parse(_)), "got {err:?}");
    }
}

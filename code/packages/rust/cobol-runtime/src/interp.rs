//! The interpreter: build the PICTURE-typed data model from WORKING-STORAGE and
//! execute the PROCEDURE DIVISION, capturing everything `DISPLAY`ed.

use crate::error::RuntimeError;
use crate::picture::Picture;
use crate::program::{
    ArithOp, Cond, ConvertOperand, Expr, Fig, Lit, Operand, Paragraph, PerformMode, Program,
    RefIndex, Region, RegionKind, RelOp, ReplaceMultiLeadingItem, Stmt, TallyCounterGroup,
    TallyMultiLeadingItem, ValueSpec, WhenValue,
};
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
///
/// Public so the `cobol-iir-compiler` frontend can reproduce the **exact same**
/// intermediate scale when it lowers a nested COMPUTE division — keeping compiled
/// output byte-identical to this oracle.
pub const COMPUTE_DIV_SCALE: usize = 12;

/// Widest alphanumeric source an alphanumeric → numeric MOVE folds into an `i64`.
/// An all-digit source of at most this many characters has value `< 10^18`, which
/// fits an `i64` (`< ~9.22 * 10^18`), so the per-character fold never overflows;
/// a wider source is a clean later rung. Mirrors the compiler's constant of the
/// same name so both engines defer the same source widths.
const NUMERIC_MAX_DIGITS: usize = 18;

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

/// The control-flow signal an executed statement (or block of statements)
/// produces. Most statements are [`Flow::Normal`]; the two transfers unwind out
/// of any enclosing `IF`/`PERFORM`/handler up to the top-level program-counter
/// loop in [`Machine::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// Continue with the next statement (and, at a paragraph's end, fall through
    /// to the following paragraph).
    Normal,
    /// `STOP RUN` — end the whole program.
    Stop,
    /// `GO TO` — transfer control to the paragraph at this index. Because it
    /// unwinds to the top-level loop, a `GO TO` inside a performed paragraph
    /// transfers control there (abandoning the `PERFORM`'s return) — the honest
    /// reading of "GO TO out of a range", and enough for structured top-level
    /// flow. (`GO TO … DEPENDING`/`ALTER` and range-return niceties are later.)
    GoTo(usize),
}

/// Turn an arithmetic result into a `Result`, reporting `i128` overflow (a value
/// beyond ~38 digits — larger than any real COBOL numeric field) as an error
/// rather than panicking or wrapping.
fn checked(r: Option<Decimal>) -> Result<Decimal, RuntimeError> {
    r.ok_or_else(|| RuntimeError::Unsupported("arithmetic overflow (result exceeds ~38 digits)".into()))
}

/// The numeric value of a level-88 `VALUE` literal: a numeric literal or `ZERO`.
/// A non-numeric value (string / other figurative) against a numeric variable is
/// a later rung.
fn num_value(lit: &Lit) -> Result<Decimal, RuntimeError> {
    match lit {
        Lit::Num(s) => {
            Decimal::parse_literal(s).ok_or_else(|| RuntimeError::Unsupported(format!("VALUE {s}")))
        }
        Lit::Fig(Fig::Zero) => Ok(Decimal::zero()),
        _ => Err(RuntimeError::Unsupported(
            "a level-88 VALUE that is not numeric on a numeric item is a later rung".into(),
        )),
    }
}

/// Whether every level-88 `VALUE` item is a STRING value: a discrete string
/// literal (`Single(Lit::Str)`) OR an inclusive `THRU` range BOTH of whose bounds
/// are string literals (`Range(Lit::Str, Lit::Str)`). This is the accept predicate
/// for a condition-name on an ALPHANUMERIC (`PIC X`) conditional variable:
///
///   * a discrete string VALUE reads (equality) and SETs (store) exactly like
///     `MOVE "…" TO item`;
///   * a string `THRU` range reads as an inclusive `lo ≤ var ≤ hi` alphanumeric
///     comparison and SETs to its low bound `lo` — both through the SAME
///     space-padded byte compare / store an `IF`/`MOVE` uses.
///
/// A range with a NON-string bound (`"A" THRU 5`), a numeric/figurative VALUE, or
/// a mixed string/numeric list still fails → still a later rung, so this returns
/// `false` and the caller rejects. The predicate is logically IDENTICAL to the
/// compiler's (`Src::Str` there, `Lit::Str` here), so both engines accept and
/// reject the very same programs.
fn all_str_values(values: &[ValueSpec]) -> bool {
    values.iter().all(|v| match v {
        ValueSpec::Single(lit) => matches!(lit, Lit::Str(_)),
        ValueSpec::Range(lo, hi) => matches!(lo, Lit::Str(_)) && matches!(hi, Lit::Str(_)),
    })
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
    /// Level-88 condition-names → the item they qualify and the value that makes
    /// them true. `IF IS-OK` tests `items[var] == value`.
    conditions: HashMap<String, ConditionName>,
    paragraphs: Vec<Paragraph>,
    para_index: HashMap<String, usize>,
    /// Current `PERFORM` nesting depth, bounded by [`MAX_PERFORM_DEPTH`].
    perform_depth: usize,
    output: String,
}

/// A level-88 condition-name: the index of the item it qualifies (its
/// "conditional variable") and the value-set that makes it true. The name holds
/// when the variable equals any single value or falls within any `THRU` range.
struct ConditionName {
    var: usize,
    /// The conditional variable's data-name. Used to form an `Operand::Ident` when
    /// an ALPHANUMERIC level-88 read compares the variable through
    /// [`Machine::compare_operands`] — reusing the exact space-padded byte compare
    /// an `IF var = "…"` relation runs, so the read is byte-identical to the
    /// compiler. Empty for the degenerate case of an unnamed conditional variable
    /// (which cannot be compared and errors cleanly at read time).
    var_name: String,
    values: Vec<ValueSpec>,
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
            conditions: HashMap::new(),
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
        // The data-name of the most recently defined item (levels 01–49/77). A
        // level-88 entry qualifies exactly that item, so this carries its name into
        // the condition-name registration below (an unnamed `FILLER` item leaves it
        // empty). Level-88 entries `continue` without touching it, so it always
        // names the item the next 88 refers to.
        let mut last_item_name = String::new();

        for def in &program.data {
            // A level-88 entry is not an item — it declares a boolean
            // condition-name that qualifies the most recently defined item (its
            // "conditional variable"). Register it and move on: it takes no
            // storage and never joins the item tree.
            if def.level == 88 {
                let name = def.name.clone().ok_or_else(|| {
                    RuntimeError::Unsupported("a level-88 entry needs a condition-name".into())
                })?;
                if def.values.is_empty() {
                    return Err(RuntimeError::Unsupported("a level-88 entry needs a VALUE".into()));
                }
                let values = def.values.clone();
                let var = self.items.len().checked_sub(1).ok_or_else(|| {
                    RuntimeError::Unsupported("a level-88 entry must follow an item".into())
                })?;
                let var_name = last_item_name.clone();
                // A level-88 whose conditional variable is an UNNAMED (`FILLER`)
                // item — an empty `var_name` — is a later rung. Reject it BEFORE
                // registering, co-totally with the compiler (whose FILLER-88 would
                // otherwise bind to the wrong item, since it does not model FILLERs).
                if var_name.is_empty() {
                    return Err(RuntimeError::Unsupported(
                        "a level-88 condition-name on an unnamed (FILLER) conditional variable is a later rung"
                            .into(),
                    ));
                }
                if self
                    .conditions
                    .insert(name.clone(), ConditionName { var, var_name, values })
                    .is_some()
                {
                    return Err(RuntimeError::DuplicateName(name));
                }
                continue;
            }

            // Only the hierarchy levels 01–49 and the standalone 77 are modelled
            // in v0.1. Rejecting anything else is faithful COBOL (66 is a
            // deferred feature; 50+ are invalid) and bounds the item-tree depth
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
            // Remember it as the conditional variable a following level-88 qualifies.
            if let Some(name) = &def.name {
                if self.by_name.insert(name.clone(), idx).is_some() {
                    return Err(RuntimeError::DuplicateName(name.clone()));
                }
                last_item_name = name.clone();
            } else {
                last_item_name = String::new();
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

            // Apply a VALUE clause as an initialising MOVE. A plain item's VALUE
            // is a single literal; multiple values and `THRU` ranges are only
            // meaningful on a level-88 condition-name.
            match def.values.as_slice() {
                [] => {}
                [ValueSpec::Single(lit)] => {
                    let src = self.src_from_lit(lit)?;
                    self.move_into(idx, src)?;
                }
                _ => {
                    return Err(RuntimeError::Unsupported(
                        "a multi-value or THRU-range VALUE is only allowed on a level-88 entry".into(),
                    ))
                }
            }
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Execution
    // ----------------------------------------------------------------------

    /// Run the procedure division and return the captured console output.
    ///
    /// Execution is a **program counter** over paragraphs: after a paragraph's
    /// statements run, control falls through to the next paragraph, unless a
    /// `GO TO` jumped the counter or a `STOP RUN` ended the program. The loop is
    /// iterative, so a `GO TO` back-edge (a COBOL loop) never grows the stack.
    /// Each paragraph's statements are cloned to run them, since executing
    /// borrows `self` mutably while the paragraphs live in `self`.
    pub fn run(mut self, _program: &Program) -> Result<String, RuntimeError> {
        let count = self.paragraphs.len();
        let mut pc = 0;
        while pc < count {
            let stmts = self.paragraphs[pc].stmts.clone();
            match self.run_stmts(&stmts)? {
                Flow::Normal => pc += 1,        // fall through
                Flow::Stop => break,            // STOP RUN
                Flow::GoTo(target) => pc = target, // jump
            }
        }
        // Falling off the end of the procedure division ends the run too.
        Ok(self.output)
    }

    /// Execute one statement, returning its control-flow [`Flow`] signal.
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, RuntimeError> {
        match stmt {
            Stmt::StopRun => return Ok(Flow::Stop),
            Stmt::Display(ops) => self.exec_display(ops)?,
            Stmt::Move { src, dsts } => self.exec_move(src, dsts)?,
            Stmt::Add { operands, to, giving, rounded, on_size_error } => {
                return self.exec_add(operands, to, giving, *rounded, on_size_error)
            }
            Stmt::Subtract { operands, from, giving, rounded, on_size_error } => {
                return self.exec_subtract(operands, from, giving, *rounded, on_size_error)
            }
            Stmt::Multiply { a, by, giving, rounded, on_size_error } => {
                return self.exec_multiply(a, by, giving, *rounded, on_size_error)
            }
            Stmt::Divide { divisor, dividend, giving, rounded, on_size_error } => {
                return self.exec_divide(divisor, dividend, giving, *rounded, on_size_error)
            }
            Stmt::Compute { target, rounded, expr, on_size_error } => {
                return self.exec_compute(target, *rounded, expr, on_size_error);
            }
            Stmt::Perform { target, thru, mode } => {
                return self.exec_perform(target, thru, mode)
            }
            Stmt::GoTo { target } => {
                let idx = *self
                    .para_index
                    .get(target)
                    .ok_or_else(|| RuntimeError::UndefinedName(target.clone()))?;
                return Ok(Flow::GoTo(idx));
            }
            Stmt::If { cond, then_branch, else_branch } => {
                let branch = if self.eval_cond(cond)? { then_branch } else { else_branch };
                return self.run_stmts(branch);
            }
            Stmt::SetTrue { cond_name } => self.exec_set_true(cond_name)?,
            Stmt::Evaluate { subject, branches } => return self.exec_evaluate(subject, branches),
            Stmt::String { sources, target, delim, pointer, on_overflow, not_on_overflow } => {
                return self.exec_string(
                    sources,
                    target,
                    delim.as_ref(),
                    pointer.as_deref(),
                    on_overflow,
                    not_on_overflow,
                );
            }
            Stmt::Unstring { source, delim, targets, pointer, on_overflow, not_on_overflow } => {
                return self.exec_unstring(
                    source,
                    delim,
                    targets,
                    pointer.as_deref(),
                    on_overflow,
                    not_on_overflow,
                );
            }
            Stmt::Inspect { source, counter, delim, leading, characters, region } => {
                return self.exec_inspect(
                    source,
                    counter,
                    delim,
                    *leading,
                    *characters,
                    region.as_ref(),
                )
            }
            Stmt::InspectReplacing { source, search, replace, leading, region } => {
                self.exec_inspect_replacing(source, search, replace, *leading, region.as_ref())?
            }
            Stmt::InspectReplacingMulti { source, items } => {
                self.exec_inspect_replacing_multi(source, items)?
            }
            Stmt::InspectReplacingCharacters { source, replace } => {
                self.exec_inspect_replacing_characters(source, replace)?
            }
            Stmt::InspectTallyMulti { source, counter, items } => {
                return self.exec_inspect_tally_multi(source, counter, items)
            }
            Stmt::InspectTallyCounters { source, groups } => {
                return self.exec_inspect_tally_counters(source, groups)
            }
            Stmt::InspectTallyReplace {
                source,
                counter,
                delim,
                tally_leading,
                tally_region,
                search,
                replace,
                replace_leading,
                replace_region,
            } => {
                return self.exec_inspect_tally_replace(
                    source,
                    counter,
                    delim,
                    *tally_leading,
                    tally_region.as_ref(),
                    search,
                    replace,
                    *replace_leading,
                    replace_region.as_ref(),
                )
            }
            Stmt::InspectConverting { source, from, to, region } => {
                self.exec_inspect_converting(source, from, to, region.as_ref())?
            }
        }
        Ok(Flow::Normal)
    }

    /// `STRING s… DELIMITED BY {SIZE | delim} INTO t` — concatenate every sending
    /// field, then overlay the result onto the receiver `t` from the left. COBOL's
    /// STRING is unusual: it writes only as many characters as it produced and
    /// **leaves the rest of `t` unchanged** — no space-fill (unlike `MOVE`). So a
    /// result longer than `t` is truncated at `t`'s width, and a shorter one leaves
    /// `t`'s trailing bytes exactly as they were. This is the ANSI-85 rule,
    /// implemented identically in the `cobol-iir-compiler` so the compiled program
    /// matches this oracle byte-for-byte.
    ///
    /// The `delim` argument selects how much of each field is taken:
    ///
    ///   * `None` (`DELIMITED BY SIZE`) — each field is taken in FULL.
    ///   * `Some(d)` — each field contributes only the run of characters BEFORE the
    ///     first `d` in that field (`"ab,cd"` with `d=','` → `"ab"`); a field with
    ///     no `d` contributes its whole image, and a field starting with `d`
    ///     contributes `""`. ONE delimiter applies to all fields.
    ///
    /// **Why the delimiter and any delimited literal field must be ASCII.** The
    /// prefix boundary is "up to the first delimiter char". The oracle finds it by
    /// scanning CHARACTERS; the compiler lowers it to byte-based `str_index` /
    /// `str_slice`. The two agree only when one char == one byte, i.e. ASCII. So a
    /// non-ASCII delimiter, and a non-ASCII string-LITERAL sending field WHEN a
    /// delimiter is active, are clean later rungs rejected identically on BOTH
    /// engines. (Under `DELIMITED BY SIZE` no per-char boundary is computed, so
    /// sending fields are unrestricted — the guard is delimiter-only.)
    fn exec_string(
        &mut self,
        sources: &[Operand],
        target: &str,
        delim: Option<&Operand>,
        pointer: Option<&str>,
        on_overflow: &[Stmt],
        not_on_overflow: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        // Resolve the single delimiter character once (it applies to every field).
        // A multi-char / numeric / figurative / reference-modified / wider-item
        // delimiter is rejected by the SAME `single_delim_char` UNSTRING uses.
        //
        // The non-ASCII guard is scoped to a LITERAL delimiter. A non-ASCII single
        // char makes the char-based oracle and the byte-based compiler diverge, so
        // it must be rejected on BOTH engines to stay co-total — but the compiler
        // can only SEE the delimiter's bytes at build time when it is a literal (a
        // multi-byte literal like `"é"` fails its byte-length test). A non-ASCII
        // PIC X(1) delimiter ITEM has no build-time byte on the compiler (its byte
        // is a run-time `str_index`), so — exactly as UNSTRING does — we do NOT add
        // a one-sided reject for it; it stays the shared byte-vs-char chip (both
        // engines accept, and for ASCII sending fields still agree).
        let delim_ch = match delim {
            Some(d) => {
                let ch = self.single_delim_char(d, "STRING")?;
                if matches!(d, Operand::Lit(Lit::Str(_))) && !ch.is_ascii() {
                    return Err(RuntimeError::Unsupported(
                        "STRING with a non-ASCII delimiter is a later rung".into(),
                    ));
                }
                Some(ch)
            }
            None => None,
        };
        // Concatenate the sending fields left-to-right. With a delimiter each field
        // is truncated at its first delimiter char; without one it is taken in full.
        let mut concat = String::new();
        for op in sources {
            // A non-ASCII string-LITERAL field under an active delimiter is a later
            // rung (its prefix boundary differs byte-vs-char). A non-ASCII IDENTIFIER
            // field is the pre-existing byte-vs-char chip and is not guarded here.
            if delim_ch.is_some() {
                if let Operand::Lit(Lit::Str(s)) = op {
                    if !s.is_ascii() {
                        return Err(RuntimeError::Unsupported(
                            "STRING with a non-ASCII sending field under DELIMITED BY is a later rung"
                                .into(),
                        ));
                    }
                }
            }
            let image = self.string_source_chars(op)?;
            match delim_ch {
                // Prefix up to (not including) the first delimiter char.
                Some(d) => concat.extend(image.chars().take_while(|c| *c != d)),
                None => concat.push_str(&image),
            }
        }
        let idx = *self
            .by_name
            .get(target)
            .ok_or_else(|| RuntimeError::UndefinedName(target.to_string()))?;
        let size = match &self.items[idx].picture {
            Some(Picture::Alphanumeric { size }) | Some(Picture::Alphabetic { size }) => *size,
            Some(_) => {
                return Err(RuntimeError::Unsupported(
                    "STRING into a numeric receiver is a later rung".into(),
                ))
            }
            None => {
                return Err(RuntimeError::Unsupported(
                    "STRING into a group receiver is a later rung".into(),
                ))
            }
        };
        // Overlay the concatenation, preserving the receiver bytes STRING did not
        // fill. `src` is the built image; `dst` is the receiver's current storage,
        // normalised to exactly `size` chars (elementary alphanumeric storage is
        // already `size` wide, but keep the overlay robust against any drift).
        let src: Vec<char> = concat.chars().collect();
        let mut dst: Vec<char> = self.items[idx].storage.chars().collect();
        if dst.len() < size {
            dst.resize(size, ' ');
        } else {
            dst.truncate(size);
        }

        // # `WITH POINTER p`
        //
        // `p` is an unsigned-integer item (`PIC 9(n)`) holding the **1-based**
        // character position in the RECEIVER at which the first transferred
        // character is placed. Without the phrase (`None`) the overlay starts at
        // position 0 and nothing is written back — today's behaviour, unchanged.
        //
        // With the phrase two things change:
        //
        //   * **Overlay offset.** Characters go to receiver positions `p-1, p, …`
        //     (0-based `start = p-1`). Only what fits from `start` to the end of the
        //     `size`-wide receiver is placed: `chars_placed = min(concat.len(),
        //     size - start)`. Positions BEFORE `start` and AFTER `start +
        //     chars_placed` keep their prior bytes (STRING overwrites only the run
        //     it fills). `p = 1` (start 0) is byte-identical to the no-pointer
        //     overlay above — the correctness anchor.
        //
        //   * **Write-back.** `p` becomes `p + chars_placed`, the 1-based position
        //     one past the last character stored. When the content does not all fit
        //     (`concat.len() > size - start`) the excess is dropped — this IS ISO's
        //     overflow, and `chars_placed = size - start`, so `p` becomes `size + 1`.
        //
        // ## Out-of-range initial pointer
        //
        // `p` is `PIC 9(n)` so `p ≥ 0`. When the initial value is OUTSIDE `[1,
        // size]` — either `p == 0` (a 0-based start of −1) or `p > size` (start past
        // the receiver end) — this is ISO's overflow condition. We apply the ISO
        // "overflow ⇒ no data movement" rule DETERMINISTICALLY: NO character is
        // transferred (receiver UNCHANGED) and `p` is left UNCHANGED. Because `p` is
        // a run-time value neither engine can range-check it at build time; the
        // compiler emits the identical guard so the accept/skip decision is
        // byte-identical.
        //
        // # The `overflow` condition (drives ON / NOT ON OVERFLOW)
        //
        // ISO: overflow occurs when the receiver fills before every sending character
        // is transferred (characters dropped), OR the initial pointer is out of
        // range. Concretely, with `avail` = characters reachable from the start:
        //
        //   | case                       | start | avail       | overflow           |
        //   | -------------------------- | ----- | ----------- | ------------------ |
        //   | no WITH POINTER            | 0     | size        | concat.len > size  |
        //   | POINTER p, p∈[1,size]      | p-1   | size-(p-1)  | concat.len > avail |
        //   | POINTER p, p==0 || p>size  | —     | —           | true (no movement) |
        //
        // Both engines MUST compute this identical comparison; a mismatch diverges.
        // After the (unchanged) data movement we run `on_overflow` when `overflow` is
        // set, else `not_on_overflow` — either list may be empty (clause absent),
        // in which case `run_stmts` returns `Flow::Normal`.
        let overflow: bool;
        match pointer {
            None => {
                overflow = src.len() > size;
                let n = src.len().min(size);
                dst[..n].copy_from_slice(&src[..n]);
                self.items[idx].storage = dst.into_iter().collect();
            }
            Some(pname) => {
                // Validate the pointer item: an UNSIGNED INTEGER `PIC 9(n)`, n ≤ 18
                // (so the value fits the `i64` the compiler stores it in) — the same
                // class INSPECT's counter demands. A signed, fractional, non-numeric,
                // or group `p` is a clean later rung, rejected identically on the
                // compiler (which checks the picture at build time).
                let pidx = *self
                    .by_name
                    .get(pname)
                    .ok_or_else(|| RuntimeError::UndefinedName(pname.to_string()))?;
                match &self.items[pidx].picture {
                    Some(Picture::Numeric { signed: true, .. }) => {
                        return Err(RuntimeError::Unsupported(format!(
                            "STRING … WITH POINTER: a signed pointer {pname} is a later rung"
                        )))
                    }
                    Some(Picture::Numeric { dec_digits, .. }) if *dec_digits != 0 => {
                        return Err(RuntimeError::Unsupported(format!(
                            "STRING … WITH POINTER: a non-integer pointer {pname} is a later rung"
                        )))
                    }
                    Some(Picture::Numeric { int_digits, .. }) if *int_digits > 18 => {
                        return Err(RuntimeError::Unsupported(format!(
                            "STRING … WITH POINTER: a pointer {pname} wider than 18 digits is a later rung"
                        )))
                    }
                    Some(Picture::Numeric { .. }) => {}
                    _ => {
                        return Err(RuntimeError::Unsupported(format!(
                            "STRING … WITH POINTER: a non-numeric pointer {pname} is a later rung"
                        )))
                    }
                }
                // The unsigned integer value of `p` (leading zeros stripped; an
                // all-zero image parses to 0). Comparing at u128 width cannot
                // overflow for ≤18 digits and keeps a huge `pv` safely greater than
                // any `size`.
                let pv_dec = self.named_decimal(pname)?;
                let pv: u128 = pv_dec.int.trim_start_matches('0').parse().unwrap_or(0);
                if pv == 0 || pv > size as u128 {
                    // Out of range IS overflow: no data movement, pointer unchanged,
                    // but the ON OVERFLOW imperative still runs (a behaviour change
                    // from the pre-overflow rung, which returned here with no
                    // imperative). Fall through to the shared imperative dispatch.
                    overflow = true;
                } else {
                    let start = (pv - 1) as usize; // 0-based overlay offset
                    let avail = size - start; // ≥ 1 here (pv ≤ size)
                    overflow = src.len() > avail; // some sending chars would be dropped
                    let chars_placed = src.len().min(avail);
                    dst[start..start + chars_placed].copy_from_slice(&src[..chars_placed]);
                    self.items[idx].storage = dst.into_iter().collect();
                    // Write `p` back to `p + chars_placed`, reshaped into its `PIC
                    // 9(n)` picture through the same numeric store ADD/INSPECT use.
                    let resume = pv + chars_placed as u128;
                    let value =
                        Decimal { neg: false, int: resume.to_string(), frac: String::new() };
                    self.store_result(pname, value, false, &[])?;
                }
            }
        }
        // Run the conditional imperative. An empty list → `run_stmts` = `Flow::Normal`
        // (mirrors how `store_result` dispatches `ON SIZE ERROR`). A `STOP RUN` / `GO
        // TO` inside the chosen list propagates its `Flow` up to unwind the enclosing
        // paragraph, exactly like an IF branch.
        if overflow {
            self.run_stmts(on_overflow)
        } else {
            self.run_stmts(not_on_overflow)
        }
    }

    /// The character image a sending field contributes to a `STRING` (taken in
    /// full — `DELIMITED BY SIZE`). An alphanumeric item gives its whole storage
    /// (trailing spaces and all); a string literal gives its text; a numeric
    /// literal gives its source digits verbatim (matching the compiler, which
    /// concatenates the literal's lexed text). A figurative constant SPACE/ZERO is
    /// accepted as its single-character image — SPACE→`" "`, ZERO→`"0"` — reducing
    /// to the string-literal path. A numeric item and a group item stay later rungs.
    ///
    /// A reference-modification sending field — `WS(start:len)` — contributes the
    /// sliced substring, produced by the shared [`Self::refmod_string`] evaluator
    /// (the exact same slice DISPLAY / comparison / MOVE-source take). Only
    /// **constant (literal) indices** are accepted: with a `PIC X` base and literal
    /// `start`/`len` the substring is a compile-time-known char image, so it drops
    /// into the concat like any alphanumeric field. A **computed (data-name) index**
    /// has a run-time length the compiler's compile-time STRING image contract
    /// cannot carry, so it stays a later rung — rejected here identically to the
    /// compiler so both engines refuse the same programs.
    fn string_source_chars(&self, op: &Operand) -> Result<String, RuntimeError> {
        match op {
            Operand::Lit(Lit::Str(s)) => Ok(s.clone()),
            Operand::Lit(Lit::Num(s)) => Ok(s.clone()),
            // A figurative constant SPACE/ZERO reduces to its single-character
            // image, dropping into the concat like a 1-char string literal. Both
            // are ASCII, so the non-ASCII sending-field guard passes unchanged.
            // `Fig` is exactly {Space, Zero}, so these two arms are exhaustive for
            // the figurative case; a numeric/group item stays a later rung below.
            Operand::Lit(Lit::Fig(Fig::Space)) => Ok(" ".into()),
            Operand::Lit(Lit::Fig(Fig::Zero)) => Ok("0".into()),
            Operand::RefMod { base, start, len } => {
                // Constant (literal) indices only: a computed data-name index gives a
                // run-time length the compiler's compile-time STRING image contract
                // cannot take, so it stays a later rung, rejected on BOTH engines.
                let const_ix = matches!(start, RefIndex::Lit(_))
                    && len.as_ref().is_none_or(|l| matches!(l, RefIndex::Lit(_)));
                if !const_ix {
                    return Err(RuntimeError::Unsupported(
                        "a computed reference modification as a STRING sending field is a later rung"
                            .into(),
                    ));
                }
                self.refmod_string(base, start, len)
            }
            Operand::Ident(name) => {
                let idx = *self
                    .by_name
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                match &self.items[idx].picture {
                    Some(p) if p.is_numeric() => Err(RuntimeError::Unsupported(
                        "a numeric item as a STRING sending field is a later rung".into(),
                    )),
                    Some(_) => Ok(self.items[idx].storage.clone()),
                    None => Err(RuntimeError::Unsupported(
                        "a group item as a STRING sending field is a later rung".into(),
                    )),
                }
            }
        }
    }

    /// `UNSTRING source DELIMITED BY delim INTO r1 [r2 …]` — the inverse of
    /// STRING. Scan the alphanumeric `source` left-to-right and split it into
    /// delimited fields on each occurrence of the single delimiter character,
    /// moving successive fields into successive receivers `r1..rn`.
    ///
    /// The scan holds a cursor `p` over the source characters. For each receiver
    /// in turn (while the source is not yet exhausted), it finds the next
    /// delimiter at or after `p` — call its index `q` (or end-of-source if none
    /// remains) — takes the field `source[p..q]`, moves it into the receiver as an
    /// ordinary alphanumeric MOVE (so padding/truncation match `move_into`), and
    /// advances `p` to `q + 1` (past the delimiter). Worked, with delimiter `,`:
    ///
    /// ```text
    ///   "A,B,C"  INTO R1 R2 R3   →  R1="A  " R2="B  " R3="C  "
    ///   "A,B,C,D" INTO R1 R2 R3  →  R1="A  " R2="B  " R3="C  "  (D dropped)
    ///   "A,B"    INTO R1 R2 R3   →  R1="A  " R2="B  " R3 UNCHANGED
    ///   "A,,C"   INTO R1 R2 R3   →  R1="A  " R2="   " R3="C  "  (empty field)
    ///   ",X"     INTO R1 R2      →  R1="   " R2="X  "
    /// ```
    ///
    /// The cursor tells "exhausted" (a field ran to end-of-source with no trailing
    /// delimiter, leaving `p` one past the end → remaining receivers UNCHANGED)
    /// apart from "a trailing delimiter" (`p` lands exactly at the end → one more
    /// empty field is still produced). Each receiver INCLUDING the last takes only
    /// its field up to the next delimiter; fields beyond the receiver count are
    /// dropped (that would be `ON OVERFLOW`, a later rung). The
    /// `cobol-iir-compiler` emits a run-time scan loop with these exact semantics,
    /// so a compiled program matches this oracle byte-for-byte.
    ///
    /// # `WITH POINTER p`
    ///
    /// `p` is an unsigned-integer item (`PIC 9(n)`) holding a **1-based** character
    /// position in the source. Two things change; the field extraction and receiver
    /// reshape are otherwise IDENTICAL:
    ///
    ///   * **Start offset.** Scanning begins at 0-based index `p_value - 1` instead
    ///     of 0. So `p = 1` is exactly today's no-pointer behaviour (start at 0),
    ///     which is the correctness anchor: `… INTO r… WITH POINTER p` with `p = 1`
    ///     must fill the SAME receivers as the same statement WITHOUT the phrase.
    ///
    ///   * **Write-back.** After the scan, `p` is set to the 1-based position of the
    ///     character immediately following the last one examined. The existing scan
    ///     leaves its 0-based cursor at `q + 1` past the terminating delimiter; when
    ///     the last field instead ran to end-of-source (no delimiter) that step is a
    ///     phantom one past the end. Clamping to `len` removes the phantom, so the
    ///     write-back value is `min(final_cursor, len) + 1` (the `+ 1` restores
    ///     1-basing). Worked: source `"a,b,c"` (len 5), `p = 3` → start at index 2
    ///     ("b,c"), r1="b", r2="c", final cursor 6 → `min(6,5)+1 = 6`.
    ///
    /// # Out-of-range initial pointer
    ///
    /// `p` is `PIC 9(n)` so `p ≥ 0`. When the initial value is OUTSIDE the valid
    /// range `[1, len]` — either `p == 0` (a 0-based start of −1) or `p > len` (past
    /// the source) — this is ISO's overflow condition. Since `ON OVERFLOW` is still
    /// deferred, we apply the ISO "overflow ⇒ no data movement" rule DETERMINISTIC-
    /// ally: NO receiver is modified and `p` is left UNCHANGED. Because `p` is a
    /// run-time value, neither engine can range-check it at build time; the compiler
    /// emits the identical guard so the accept/skip decision is byte-identical.
    fn exec_unstring(
        &mut self,
        source: &Operand,
        delim: &Operand,
        targets: &[String],
        pointer: Option<&str>,
        on_overflow: &[Stmt],
        not_on_overflow: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        // The field characters come from ONE of THREE providers; everything after
        // `src` is obtained (the delimiter scan and per-receiver reshape) is
        // shared, so only this match differs between an item source, a literal
        // source, and a reference-modified item slice.
        let src: Vec<char> = match source {
            // Identifier source: the characters are an alphanumeric item's
            // STORAGE (a numeric or group item is a later rung).
            Operand::Ident(name) => {
                let sidx = *self
                    .by_name
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                match &self.items[sidx].picture {
                    Some(p) if p.is_numeric() => {
                        return Err(RuntimeError::Unsupported(
                            "UNSTRING of a numeric source is a later rung".into(),
                        ))
                    }
                    Some(_) => {}
                    None => {
                        return Err(RuntimeError::Unsupported(
                            "UNSTRING of a group source is a later rung".into(),
                        ))
                    }
                }
                self.items[sidx].storage.chars().collect()
            }
            // Literal source: the characters are the string literal's OWN bytes.
            // A string literal is inherently alphanumeric, so there is no item to
            // look up and no picture to check.
            Operand::Lit(Lit::Str(s)) => s.chars().collect(),
            // Reference-modified source: the characters are the ref-mod slice
            // `base(start:len)` of the base item. `refmod_string` returns EXACTLY
            // the char range the compiler emits as a `str_slice` (so DISPLAY of the
            // same slice already agrees byte-for-byte) and already rejects a
            // numeric base and out-of-range indices. Only obtaining `src` differs;
            // the delimiter scan and receiver fill below are UNCHANGED.
            Operand::RefMod { base, start, len } => {
                self.refmod_string(base, start, len)?.chars().collect()
            }
            // The reader rejected every remaining source variant (numeric or
            // figurative literal), so this is unreachable in practice; guard
            // defensively rather than panic.
            Operand::Lit(_) => {
                return Err(RuntimeError::Unsupported(
                    "UNSTRING with this source kind is a later rung".into(),
                ))
            }
        };

        // The single delimiter character.
        let delim_ch = self.single_delim_char(delim, "UNSTRING")?;

        // Every receiver must be an alphanumeric item (validated up front so a
        // numeric/group receiver is a clean error even if the scan never reaches
        // it — matching the compiler, which lowers every receiver at build time).
        let mut tidx: Vec<usize> = Vec::with_capacity(targets.len());
        for t in targets {
            let idx = *self
                .by_name
                .get(t)
                .ok_or_else(|| RuntimeError::UndefinedName(t.to_string()))?;
            match &self.items[idx].picture {
                Some(p) if p.is_numeric() => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING into a numeric receiver is a later rung".into(),
                    ))
                }
                Some(_) => {}
                None => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING into a group receiver is a later rung".into(),
                    ))
                }
            }
            tidx.push(idx);
        }

        // Resolve `WITH POINTER p` (if present) into the scan's initial 0-based
        // cursor. The pointer item must be an UNSIGNED INTEGER `PIC 9(n)` — the
        // same class INSPECT's counter demands — so a signed, fractional, non-
        // numeric, or group `p` is a clean later rung, rejected identically on the
        // compiler (which checks the picture at build time). We also bound it to 18
        // digits so the value fits the `i64` the compiler stores it in.
        //
        // With a pointer we read its value `pv`, then apply the out-of-range rule:
        // `pv == 0` (0-based start would underflow to −1) or `pv > len` (start past
        // the source) is ISO's overflow ⇒ leave every receiver and `p` UNCHANGED.
        // Otherwise the 0-based start is `pv − 1`. The guard runs BEFORE computing
        // `pv − 1`, so the `usize` never underflows. Without a pointer the start is 0
        // and nothing is written back — today's behaviour, unchanged.
        //
        // # The `overflow` condition (drives ON / NOT ON OVERFLOW)
        //
        // ISO: UNSTRING overflow occurs when all receivers are filled but the source
        // is NOT exhausted (more delimited fields remain), OR the initial `WITH
        // POINTER` value is out of range. `overflow` starts `false` and is SET in the
        // two places the condition can arise, so BOTH engines compute the identical
        // boolean (a mismatch would diverge):
        //
        //   * out-of-range pointer → `overflow = true` (no data movement below);
        //   * after the scan loop  → `overflow = (p <= src.len())`, where `p` is the
        //     scan's final 0-based cursor. This SINGLE comparison is correct for every
        //     case:
        //       - loop broke early (`p > len`, fewer fields than receivers, source
        //         exhausted first) → `p > len` → `false`. ✓
        //       - all receivers filled, last field ended AT a delimiter (`q < len`, so
        //         `p = q+1 ≤ len`, more source remains) → `true`. ✓
        //       - all receivers filled, last field ran to end-of-source (`q == len`,
        //         `p = len+1 > len`, exhausted) → `false`. ✓
        //       - trailing delimiter as the last consumed char (`p == len`, an empty
        //         field remains) → `true`. ✓
        let mut overflow = false;
        let start: usize = if let Some(pname) = pointer {
            let pidx = *self
                .by_name
                .get(pname)
                .ok_or_else(|| RuntimeError::UndefinedName(pname.to_string()))?;
            match &self.items[pidx].picture {
                Some(Picture::Numeric { signed: true, .. }) => {
                    return Err(RuntimeError::Unsupported(format!(
                        "UNSTRING … WITH POINTER: a signed pointer {pname} is a later rung"
                    )))
                }
                Some(Picture::Numeric { dec_digits, .. }) if *dec_digits != 0 => {
                    return Err(RuntimeError::Unsupported(format!(
                        "UNSTRING … WITH POINTER: a non-integer pointer {pname} is a later rung"
                    )))
                }
                Some(Picture::Numeric { int_digits, .. }) if *int_digits > 18 => {
                    return Err(RuntimeError::Unsupported(format!(
                        "UNSTRING … WITH POINTER: a pointer {pname} wider than 18 digits is a later rung"
                    )))
                }
                Some(Picture::Numeric { .. }) => {}
                _ => {
                    return Err(RuntimeError::Unsupported(format!(
                        "UNSTRING … WITH POINTER: a non-numeric pointer {pname} is a later rung"
                    )))
                }
            }
            // The unsigned integer value of `p`. A `PIC 9(n)` item is non-negative
            // with no fraction, so the magnitude is its integer digits (leading
            // zeros stripped; an all-zero image parses to 0). Comparing at u128
            // width cannot overflow for ≤18 digits and keeps a huge `pv` safely
            // greater than any `len`.
            let pv_dec = self.named_decimal(pname)?;
            let pv: u128 = pv_dec.int.trim_start_matches('0').parse().unwrap_or(0);
            if pv == 0 || pv > src.len() as u128 {
                // Out of range IS overflow: no data movement, pointer unchanged, but
                // the ON OVERFLOW imperative still runs (a behaviour change from the
                // pre-overflow rung, which returned here with no imperative). Set the
                // flag and skip the scan + write-back with the `!overflow` guard
                // below; `start` is a never-read sentinel on this path.
                overflow = true;
                0
            } else {
                (pv - 1) as usize
            }
        } else {
            0
        };

        // The scan and write-back are skipped entirely on the out-of-range-pointer
        // path (no data movement, pointer unchanged), matching the oracle's former
        // early return. On every other path this runs and then RE-derives `overflow`
        // from the final cursor (see the truth table above), replacing its `false`
        // start.
        if !overflow {
            // Scan: cursor `p` over `src`; for each receiver take the field up to the
            // next delimiter (or end-of-source), then step past the delimiter.
            let mut p: usize = start;
            for &idx in &tidx {
                // `p > len` means the previous field ran off the end WITHOUT a
                // trailing delimiter — the source is exhausted, so leave this and
                // every later receiver UNCHANGED. (`p == len` still yields one empty
                // field, the trailing-delimiter case.)
                if p > src.len() {
                    break;
                }
                let mut q = p;
                while q < src.len() && src[q] != delim_ch {
                    q += 1;
                }
                let field: String = src[p..q].iter().collect();
                self.move_into(idx, Src::Chars(field))?;
                p = q + 1;
            }

            // Write the pointer back to the 1-based resume position. `p` is the scan's
            // final 0-based cursor, which sits one past the terminating delimiter —
            // but for a field that ran to end-of-source that step is a phantom one
            // past the end, so clamp to `len` before restoring 1-basing:
            // `min(p, len) + 1`. This is exactly "one after the last character
            // examined" (see the doc comment). Stored through the same numeric path
            // ADD/INSPECT use, so it reshapes into the pointer's `PIC 9(n)` picture
            // byte-for-byte as the compiler does. The write-back happens BEFORE the
            // imperative dispatch, unchanged.
            if let Some(pname) = pointer {
                let resume = p.min(src.len()) + 1;
                let value = Decimal { neg: false, int: resume.to_string(), frac: String::new() };
                self.store_result(pname, value, false, &[])?;
            }

            // The one comparison the compiler mirrors: source not yet exhausted after
            // filling every receiver ⇒ overflow. See the worked cases above.
            overflow = p <= src.len();
        }

        // Run the conditional imperative. An empty list → `run_stmts` = `Flow::Normal`
        // (mirrors STRING's `exec_string`). A `STOP RUN` / `GO TO` inside the chosen
        // list propagates its `Flow` up to unwind the enclosing paragraph, exactly
        // like an IF branch.
        if overflow {
            self.run_stmts(on_overflow)
        } else {
            self.run_stmts(not_on_overflow)
        }
    }

    /// The single delimiter character of a scan delimiter. It is either a
    /// 1-character string literal (`","`, `" "`) or a `PIC X(1)` item. A
    /// multi-character delimiter, `ALL`/`OR` delimiters, a numeric/figurative
    /// delimiter, and a numeric/group/wider delimiter item are later rungs.
    ///
    /// Shared by `UNSTRING … DELIMITED BY delim` and `INSPECT … FOR ALL delim`;
    /// `verb` names the caller so the later-rung message reads naturally.
    fn single_delim_char(&self, delim: &Operand, verb: &str) -> Result<char, RuntimeError> {
        match delim {
            Operand::Lit(Lit::Str(s)) => {
                let chars: Vec<char> = s.chars().collect();
                match chars.as_slice() {
                    [c] => Ok(*c),
                    _ => Err(RuntimeError::Unsupported(format!(
                        "{verb} with a multi-character delimiter is a later rung"
                    ))),
                }
            }
            Operand::Lit(Lit::Num(_)) => Err(RuntimeError::Unsupported(format!(
                "{verb} with a numeric-literal delimiter is a later rung"
            ))),
            Operand::Lit(Lit::Fig(_)) => Err(RuntimeError::Unsupported(format!(
                "{verb} with a figurative-constant delimiter is a later rung"
            ))),
            Operand::RefMod { .. } => Err(RuntimeError::Unsupported(format!(
                "{verb} with a reference-modified delimiter is a later rung"
            ))),
            Operand::Ident(name) => {
                let idx = *self
                    .by_name
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                match &self.items[idx].picture {
                    Some(p) if p.is_numeric() => Err(RuntimeError::Unsupported(format!(
                        "{verb} with a numeric delimiter item is a later rung"
                    ))),
                    Some(_) => {
                        let chars: Vec<char> = self.items[idx].storage.chars().collect();
                        match chars.as_slice() {
                            [c] => Ok(*c),
                            _ => Err(RuntimeError::Unsupported(format!(
                                "{verb} with a delimiter item wider than one character is a later rung"
                            ))),
                        }
                    }
                    None => Err(RuntimeError::Unsupported(format!(
                        "{verb} with a group delimiter item is a later rung"
                    ))),
                }
            }
        }
    }

    /// `INSPECT source TALLYING counter FOR ALL delim` — count the (non-
    /// overlapping, left-to-right) occurrences of the SINGLE-character `delim` in
    /// the alphanumeric `source`, then **ADD** that count to the unsigned-integer
    /// `counter`. INSPECT adds to the counter; it does NOT clear it first, so the
    /// effect is `counter := counter + occurrences`.
    ///
    /// The count folds into the counter through the SAME `store_result` path the
    /// arithmetic verbs use (COBOL's silent high-order truncation on overflow), so
    /// the compiled `cobol-iir-compiler` scan loop matches this reference output
    /// byte-for-byte. A numeric/group source, or a non-integer/non-numeric/signed
    /// counter, is a clean later-rung error.
    fn exec_inspect(
        &mut self,
        source: &str,
        counter: &str,
        delim: &Operand,
        leading: bool,
        characters: bool,
        region: Option<&Region>,
    ) -> Result<Flow, RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;
        self.inspect_tally(sidx, counter, delim, leading, characters, region)
    }

    /// The source of any INSPECT must resolve to an alphanumeric item. Returns its
    /// item index (a numeric or group source is a clean later-rung error). Shared
    /// by the lone TALLYING, lone REPLACING, and combined execs so all three
    /// diagnose an unsupported source identically.
    fn inspect_alnum_source(&self, source: &str) -> Result<usize, RuntimeError> {
        let sidx = *self
            .by_name
            .get(source)
            .ok_or_else(|| RuntimeError::UndefinedName(source.to_string()))?;
        match &self.items[sidx].picture {
            Some(p) if p.is_numeric() => Err(RuntimeError::Unsupported(
                "INSPECT of a numeric source is a later rung".into(),
            )),
            Some(_) => Ok(sidx),
            None => Err(RuntimeError::Unsupported(
                "INSPECT of a group source is a later rung".into(),
            )),
        }
    }

    /// The character window `[start, end)` an `INSPECT … {BEFORE|AFTER} x` region
    /// selects over `chars` (the source's current storage as a `char` vector). With
    /// no region the window is the WHOLE source (`(0, len)`); a region narrows it
    /// around the FIRST (leftmost) occurrence of the single region delimiter `x`,
    /// applying the ISO not-found asymmetry:
    ///   * `BEFORE x` → `(0, first_index_of(x))`; if `x` is ABSENT → `(0, len)`
    ///     (the ENTIRE source); and
    ///   * `AFTER x`  → `(first_index_of(x)+1, len)`; if `x` is ABSENT → `(len, len)`
    ///     (an EMPTY window).
    ///
    /// This is the ONE place both INSPECT operations derive their window: the count
    /// ([`Self::inspect_tally`]) and the `ALL` replacement ([`Self::inspect_replace`])
    /// call it with the SAME `chars`, so they narrow to byte-identical slices and the
    /// BEFORE→whole / AFTER→empty rule can never drift between them. A multi-character
    /// region delimiter is rejected here by `single_delim_char`, exactly like the
    /// scan/search delimiters.
    fn region_window(
        &self,
        chars: &[char],
        region: Option<&Region>,
    ) -> Result<(usize, usize), RuntimeError> {
        let len = chars.len();
        match region {
            None => Ok((0, len)),
            Some(r) => {
                let region_ch = self.single_delim_char(&r.delim, "INSPECT")?;
                let first = chars.iter().position(|&c| c == region_ch);
                Ok(match r.kind {
                    // BEFORE: everything left of the first `x`; if `x` is absent the
                    // region is the whole source (`end = len`).
                    RegionKind::Before => (0, first.unwrap_or(len)),
                    // AFTER: everything right of the first `x`; if `x` is absent the
                    // region is EMPTY (`start = end = len`).
                    RegionKind::After => match first {
                        Some(i) => (i + 1, len),
                        None => (len, len),
                    },
                })
            }
        }
    }

    /// The TALLYING half: count occurrences of the single-character `delim` in the
    /// source's CURRENT storage and ADD them to `counter`. When `leading` is false
    /// (`FOR ALL`) this counts EVERY occurrence; when true (`FOR LEADING`) it counts
    /// only the run of CONSECUTIVE occurrences at the START of the source, stopping
    /// at the first non-`delim` character. Factored out of [`Self::exec_inspect`] so
    /// the combined tally-then-replace exec can run it FIRST (on the pre-replacement
    /// bytes) and share the counter validation and store path; the combined path
    /// passes through its own `FOR ALL`/`FOR LEADING` selection here.
    /// Does not mutate the source.
    ///
    /// An optional `region` (`{BEFORE|AFTER} x`) narrows the count to a sub-slice of
    /// the source, bounded by the FIRST (leftmost) occurrence of the single region
    /// delimiter `x` — computed over the source's CURRENT storage:
    ///   * `BEFORE x` → count only within `source[0 .. first_index_of(x)]`; if `x`
    ///     is absent the region is the ENTIRE source (`end = len`); and
    ///   * `AFTER x`  → count only within `source[first_index_of(x)+1 .. len]`; if
    ///     `x` is absent the region is EMPTY (`start = end = len` → count 0).
    ///
    /// This not-found asymmetry (BEFORE→whole, AFTER→empty) is the ISO rule and MUST
    /// match the compiler byte-for-byte. With no region the window is the whole
    /// source (`start = 0`, `end = len`), so behaviour is unchanged. The combined
    /// tally-then-replace path never carries a region (rejected at read time), so it
    /// always passes `None`.
    fn inspect_tally(
        &mut self,
        sidx: usize,
        counter: &str,
        delim: &Operand,
        leading: bool,
        characters: bool,
        region: Option<&Region>,
    ) -> Result<Flow, RuntimeError> {
        // The counter must be an UNSIGNED INTEGER numeric item (`PIC 9(n)`): a
        // fractional (`V`) or signed (`S`) counter is a later rung.
        let cidx = *self
            .by_name
            .get(counter)
            .ok_or_else(|| RuntimeError::UndefinedName(counter.to_string()))?;
        match &self.items[cidx].picture {
            Some(Picture::Numeric { dec_digits: 0, signed: false, .. }) => {}
            Some(Picture::Numeric { .. }) => {
                return Err(RuntimeError::Unsupported(format!(
                    "INSPECT TALLYING into a non-integer or signed counter {counter} is a later rung"
                )))
            }
            _ => {
                return Err(RuntimeError::Unsupported(format!(
                    "INSPECT TALLYING into a non-numeric counter {counter} is a later rung"
                )))
            }
        }

        // The single delimiter character to count — ONLY the ALL/LEADING forms match a
        // delimiter, so we resolve it only when `characters == false`. The CHARACTERS
        // form counts POSITIONS, not delimiter matches, so it never reads `delim` (which
        // carries a never-read placeholder on that path) and never runs `single_delim_char`.
        let delim_ch = if characters {
            None
        } else {
            Some(self.single_delim_char(delim, "INSPECT")?)
        };

        // The character window `[start, end)` the count runs over. With no region
        // this is the WHOLE source; a `{BEFORE|AFTER} x` region narrows it around the
        // first occurrence of the single region delimiter `x`, applying the ISO
        // not-found asymmetry (BEFORE→whole, AFTER→empty). We work on `chars()` (not
        // bytes) so the window and the count agree on positions. The window is derived
        // by the SHARED [`Self::region_window`] helper — the SAME code the REPLACING
        // half uses, so the two INSPECT operations narrow to byte-identical slices.
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();
        let (start, end) = self.region_window(&chars, region)?;
        let window = &chars[start..end];

        // The occurrence count over the window. `FOR ALL` counts every match;
        // `FOR LEADING` counts only the leading run (stop at the first non-match) —
        // the ONLY difference between the two forms. Crucially the leading run is
        // anchored at the WINDOW START, not source position 0: because `window` is
        // already the `[start, end)` slice, `take_while` begins at `start` and stops
        // at the first non-`delim` character INSIDE the window (or the window end).
        // So a standalone `FOR LEADING … AFTER x` counts the run beginning at
        // `first+1`, not at 0 — e.g. "aaXaab" AFTER "X" narrows to "aab" and counts
        // the two leading a's, ignoring the "aa" before the X entirely. With `AFTER x`
        // and `x` absent the window is empty, so the count is 0 (the ISO not-found
        // asymmetry). (The combined `TALLYING … REPLACING` form now composes this SAME
        // routine for its LEADING tally half carrying a region — it reaches here with
        // `leading = true` and a `region`, byte-identical to the standalone form.)
        // `FOR CHARACTERS` is the "count every position" form: it adds the NUMBER OF
        // CHARACTER POSITIONS in the window, regardless of content. With no region that
        // is the whole source; with a `{BEFORE|AFTER} x` region it is the `[start, end)`
        // slice ALL/LEADING use, so it inherits the identical BEFORE→whole / AFTER→empty
        // not-found asymmetry (a `BEFORE x` with `x` absent counts the whole string; an
        // `AFTER x` with `x` absent counts 0, because the window is empty). No delimiter
        // is matched.
        //
        // COUNT IN BYTES, NOT `char`s. COBOL `PIC X` positions are BYTES, and the
        // `cobol-iir-compiler` measures the window with `str_len`/byte indices, so the
        // count MUST be the window's BYTE length to stay byte-identical. For ASCII every
        // char is one byte, so `sum(len_utf8) == window.len()` and nothing changes; a
        // non-ASCII source (a degenerate input for single-byte COBOL) would otherwise
        // diverge — the oracle would count code points while the compiler counts bytes.
        // Summing `len_utf8()` over the SAME `[start, end)` window equals the compiler's
        // `end - start` byte span because the two engines' region windows cover the same
        // substring (a non-ASCII region *delimiter* stays the pre-existing byte-vs-char
        // chip; here the delimiter, when present, is matched in char space and never
        // reaches this branch).
        let count = match delim_ch {
            None => window.iter().map(|c| c.len_utf8()).sum::<usize>(),
            Some(dch) if leading => window.iter().take_while(|&&c| c == dch).count(),
            Some(dch) => window.iter().filter(|&&c| c == dch).count(),
        };

        // counter := counter_value + count, reshaped into the counter's picture —
        // the same store path ADD uses (INSPECT adds; it does not clear first).
        let addend = Decimal { neg: false, int: count.to_string(), frac: String::new() };
        let acc = checked(add(&self.named_decimal(counter)?, &addend))?;
        self.store_result(counter, acc, false, &[])
    }

    /// `INSPECT source REPLACING ALL search BY replace` — replace EVERY
    /// occurrence of the SINGLE character `search` in the alphanumeric `source`
    /// with the SINGLE character `replace`, in place. Both are single characters
    /// so the source's width is unchanged: this is a straight per-position map
    /// (`c == search ? replace : c`), left to right. The rebuilt string is fed
    /// back through the SAME alphanumeric char-store path a `MOVE` uses
    /// (`move_into` with `Src::Chars`), which is a no-op on width here (the map
    /// preserves length), so the compiled `cobol-iir-compiler` lowering matches
    /// this reference output byte-for-byte. A numeric/group source is a clean
    /// later-rung error; a multi-character/figurative/wider search or
    /// replacement is rejected by `single_delim_char`.
    ///
    /// An optional `region` (`{BEFORE|AFTER} x`) narrows the `ALL` replacement to a
    /// sub-slice of the source, using the SAME window [`Self::inspect_tally`] uses
    /// for the count (see [`Self::region_window`]): positions OUTSIDE the window keep
    /// their original character. The window is computed over the ORIGINAL source
    /// bytes (before any substitution). A region is only reached on the lone
    /// `REPLACING ALL` path; `REPLACING LEADING` and the combined form always pass
    /// `None` (rejected at read time).
    fn exec_inspect_replacing(
        &mut self,
        source: &str,
        search: &Operand,
        replace: &Operand,
        leading: bool,
        region: Option<&Region>,
    ) -> Result<(), RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;
        self.inspect_replace(sidx, search, replace, leading, region)
    }

    /// `INSPECT source REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` — one
    /// INSPECT with TWO OR MORE `ALL` replace items, applied in a SINGLE
    /// left-to-right pass with FIRST-MATCH-WINS and NO RE-CHAINING.
    ///
    /// The whole subtlety is that this is NOT the same as running N single-item
    /// replaces in sequence. At each source position we consult the items IN WRITTEN
    /// ORDER and stop at the FIRST whose search matches the ORIGINAL character:
    ///
    /// ```text
    ///   for i in 0..width {
    ///       out[i] = src[i]                       // default: unchanged
    ///       for (search, replace) in items {      // written order
    ///           if src[i] == search { out[i] = replace; break }   // first wins
    ///       }
    ///   }
    /// ```
    ///
    /// Two properties fall out of that inner `break`, both pinned by tests:
    ///
    ///   * FIRST-MATCH-WINS — if two items could match the same position, only the
    ///     earlier-written one fires (`"a" BY "x"` before `"a" BY "y"` → `x`).
    ///   * NO RE-CHAINING — the byte a replacement PRODUCES is never fed back to a
    ///     later item, because the whole scan reads `src` (the original), never the
    ///     output. So `ALL "a" BY "b" ALL "b" BY "z"` over `"ab"` gives `"bz"`, not
    ///     `"zz"`: position 0 turns the original `a` into `b` and stops (the produced
    ///     `b` is not re-inspected), and position 1 turns the ORIGINAL `b` into `z`.
    ///
    /// Width is preserved (each position emits exactly one char), so — exactly like
    /// [`Self::inspect_replace`] — the rebuilt string feeds the SAME alphanumeric
    /// char-store path a `MOVE` uses, and the compiled per-position lowering in
    /// `cobol-iir-compiler` matches this reference output byte-for-byte.
    ///
    /// Every (search, replace) is validated with the SAME `single_delim_char` check
    /// the single-item path uses (a multi-character/figurative/wider/numeric operand
    /// is a later rung); ALL of them — AND every item's region window — are resolved
    /// BEFORE any mutation so an invalid item leaves the source untouched. A
    /// numeric/group source is rejected by [`Self::inspect_alnum_source`]. The
    /// read-time reader (`read_inspect_replacing_multi`) has already ruled out
    /// CHARACTERS/FIRST items, so here every item is a single-char `{ALL|LEADING}`
    /// search-BY-replace pair with an OPTIONAL `{BEFORE|AFTER} x` region.
    ///
    /// # Per-item regions
    ///
    /// Each item may carry its OWN optional `{BEFORE|AFTER} x` window, computed over
    /// the ORIGINAL source via the SAME [`Self::region_window`] helper the lone/
    /// single-item forms use (BEFORE→`[0, first_x)`; AFTER→`(first_x, len]`; not-found
    /// asymmetry BEFORE→whole, AFTER→empty). An item with NO region has the whole
    /// source as its window (so a region-less `LEADING` item anchors its run at source
    /// position 0).
    ///
    /// # The `LEADING` active-flag machine (this rung — twin of the tally side)
    ///
    /// This is the byte-producing analogue of [`Self::exec_inspect_tally_multi`]: the
    /// ONLY difference from that count-side machine is that the decision loop, instead
    /// of `count += 1`, EMITS the item's replacement char at position `i` (and on no
    /// match keeps the ORIGINAL char). The run-update loop is IDENTICAL. ONE
    /// left-to-right pass carries a per-item `active` flag (only consulted for `LEADING`
    /// items, all init `true`):
    ///
    /// ```text
    ///   active = [true; N]                              // one per item; LEADING-only
    ///   for i in 0..width {
    ///       c = chars[i]
    ///       out[i] = c                                   // default: unchanged
    ///       for (search, replace, leading, start, end) in items {   // written order
    ///           in_win = start <= i < end
    ///           if in_win && c == search && (!leading || active[k]) {
    ///               out[i] = replace; break              // first eligible item wins
    ///           }
    ///       }
    ///       for (search, _, leading, start, end) in items {   // then update EVERY run
    ///           if leading && start <= i < end && c != search { active[k] = false }
    ///       }
    ///   }
    /// ```
    ///
    /// So a `LEADING` item replaces only its CONSECUTIVE run of `search` anchored at its
    /// window start; a `search` after a break is NOT replaced. The run-update loop runs
    /// INDEPENDENTLY of which item won the decision — a run breaks at the FIRST in-window
    /// position whose char is NOT its search (a matching char keeps the run alive even if
    /// a higher-priority item claimed that position; positions outside the window neither
    /// begin nor break the run). First-match-wins and no-re-chaining are unchanged: the
    /// scan reads `chars` (the original) and never the output, and each window is computed
    /// over that same original, so both engines agree byte-for-byte (the match-based
    /// replacement only fires on a single-char ASCII search, so multi-byte source chars
    /// pass through untouched and the rebuilt string stays valid UTF-8 — the same
    /// byte-safety the single-item region form relies on).
    fn exec_inspect_replacing_multi(
        &mut self,
        source: &str,
        items: &[ReplaceMultiLeadingItem],
    ) -> Result<(), RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;
        // ONE pass over the ORIGINAL characters — resolve them FIRST so the window
        // (like the single-item path) sees the pre-replacement bytes, and so an
        // invalid operand aborts with the source untouched.
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();
        // Resolve every (search char, replace char, leading flag, [start, end) window)
        // FIRST — reading all items (and computing their windows over the original
        // `chars`) before touching storage means an invalid operand aborts cleanly,
        // exactly like the single-item path reads both chars and the window first.
        let resolved: Vec<(char, char, bool, usize, usize)> = items
            .iter()
            .map(|(search, replace, leading, region)| {
                let s = self.single_delim_char(search, "INSPECT REPLACING")?;
                let r = self.single_delim_char(replace, "INSPECT REPLACING")?;
                let (start, end) = self.region_window(&chars, region.as_ref())?;
                Ok((s, r, *leading, start, end))
            })
            .collect::<Result<_, RuntimeError>>()?;

        // The `LEADING` active-flag machine — the byte-producing twin of
        // `exec_inspect_tally_multi` (see the doc comment). A per-item `active` run flag
        // (only consulted for `LEADING` items, all init `true`) rides one left-to-right
        // pass. At each position the FIRST ELIGIBLE item in WRITTEN ORDER emits its
        // replacement and the rest are skipped; on no eligible item the ORIGINAL char is
        // kept. Because the scan reads `chars` (the original) and never the output, a
        // produced character is never re-examined — the no-re-chaining property — and
        // each window was computed over that same original.
        let mut active = vec![true; resolved.len()];
        let mut rebuilt = String::with_capacity(self.items[sidx].storage.len());
        for (i, &c) in chars.iter().enumerate() {
            // Decision: first eligible item wins. An `ALL` item is eligible iff its
            // window contains the position AND its search matches; a `LEADING` item ALSO
            // requires its run still active (every prior in-window position equalled its
            // search).
            let mut out = c; // default: unchanged
            for (k, &(search_ch, replace_ch, leading, start, end)) in resolved.iter().enumerate() {
                let in_win = start <= i && i < end;
                if in_win && c == search_ch && (!leading || active[k]) {
                    out = replace_ch;
                    break;
                }
            }
            rebuilt.push(out);
            // Then update EVERY `LEADING` item's run flag, INDEPENDENTLY of which item
            // won: a run breaks at the FIRST in-window position whose char is NOT its
            // search (a matching char keeps the run alive even if a higher-priority item
            // claimed the position; positions outside the window neither begin nor break
            // the run — anchoring the run at the window start).
            for (k, &(search_ch, _, leading, start, end)) in resolved.iter().enumerate() {
                if leading && start <= i && i < end && c != search_ch {
                    active[k] = false;
                }
            }
        }
        self.move_into(sidx, Src::Chars(rebuilt))
    }

    /// `INSPECT source REPLACING CHARACTERS BY x` — overwrite EVERY position of the
    /// alphanumeric `source` with the single replacement character `x`. With no
    /// region the WHOLE field becomes `x`s; its width is unchanged.
    ///
    /// # Byte-basis fill (co-total with the byte-based compiler)
    ///
    /// The compiler fills `str_len(S)` (BYTE-length) positions; to agree with it for
    /// ANY source we compute the fill on the BYTE basis too: `n = storage.len()` is
    /// the field's BYTE length, and we build `n` copies of `x`. Because `x` is a
    /// single ASCII byte (guard 2), the `n`-copy image is `n` ASCII bytes — a valid
    /// `String`. We then store it through the SAME `move_into` path `inspect_replace`
    /// uses, which re-pads/truncates the image to the picture's fixed CHAR size. So
    /// for a non-ASCII source whose byte length exceeds its char size (e.g.
    /// `PIC X(5) VALUE "café"` stores `"café "` = 5 chars / 6 bytes), the `n = 6`
    /// copies cap to the picture's 5 chars — exactly the compiler's `width = 5` fill.
    ///
    /// # Guards (IDENTICAL to the compiler)
    ///
    ///   1. `x` is a SINGLE character — the shared `single_delim_char` check (a
    ///      multi-character/figurative/wider/numeric operand is a later rung).
    ///   2. A single-char but NON-ASCII **literal** `x` is a later rung, mirroring the
    ///      compiler's byte-based single-char validator. Applied to LITERALS only: a
    ///      `PIC X(1)` *item* replacement is naturally co-total under the byte-fill
    ///      (both engines emit `width` copies of the item's char), so it is not gated.
    ///   4. A numeric/group source is rejected by `inspect_alnum_source` (guard 4).
    ///
    /// (Guard 3 — a `{BEFORE|AFTER}` region — is rejected earlier, at read time.)
    fn exec_inspect_replacing_characters(
        &mut self,
        source: &str,
        replace: &Operand,
    ) -> Result<(), RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;
        // Guard 2 — a single-char but non-ASCII LITERAL replacement is deferred so the
        // char-based oracle stays co-total with the byte-based compiler (whose
        // single-char validator rejects a multi-byte literal). Items are not gated.
        if let Operand::Lit(Lit::Str(s)) = replace {
            if s.chars().count() == 1 && !s.is_ascii() {
                return Err(RuntimeError::Unsupported(
                    "INSPECT REPLACING CHARACTERS with a non-ASCII replacement is a later rung"
                        .into(),
                ));
            }
        }
        // Guard 1 — resolve the single replacement char (also validates a PIC X(1)
        // item), reusing the SAME check REPLACING ALL uses.
        let ch = self.single_delim_char(replace, "INSPECT REPLACING")?;
        // Byte-basis fill: n = storage BYTE length copies of `ch`. `move_into`
        // re-pads/truncates to the picture's char size (see the doc comment).
        let n = self.items[sidx].storage.len();
        let rebuilt: String = std::iter::repeat_n(ch, n).collect();
        self.move_into(sidx, Src::Chars(rebuilt))
    }

    /// `INSPECT source TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|
    /// AFTER} q] …` — one INSPECT whose SINGLE counter carries TWO OR MORE `FOR ALL`
    /// items, each with its OWN optional `{BEFORE|AFTER} x` window, counted in a SINGLE
    /// left-to-right pass with FIRST-MATCH-PER-POSITION into the shared counter.
    ///
    /// This is the count-side analogue of [`Self::exec_inspect_replacing_multi`]. The
    /// items form an ordered priority list, each `ALL` OR `LEADING`, each carrying its OWN
    /// window over the source. ONE left-to-right pass with a per-item `active` flag (only
    /// consulted for `LEADING` items, all init `true`). At each position the FIRST
    /// ELIGIBLE item in WRITTEN ORDER increments the shared count by 1 and the scan
    /// advances; an `ALL` item is eligible iff its window contains the position AND its
    /// delimiter matches, a `LEADING` item ALSO requires its `active` flag still `true`.
    /// AFTER the tally decision, EVERY `LEADING` item's `active` flag is updated
    /// INDEPENDENTLY of which item tallied — its run breaks at the FIRST in-window
    /// position whose char is NOT its delimiter (a matching char keeps the run alive even
    /// if a higher-priority item claimed that position; positions outside the window
    /// neither begin nor break the run, so a `LEADING` run is anchored at its window start):
    ///
    /// ```text
    ///   count = 0;  active = [true; N]                   // one per item; LEADING-only
    ///   for i in 0..len {
    ///       c = chars[i]
    ///       for (d, leading, start, end) in items {      // written order — tally decision
    ///           in_win = start <= i < end
    ///           if in_win && c == d && (!leading || active[k]) { count += 1; break }
    ///       }
    ///       for (d, leading, start, end) in items {      // then update EVERY leading run
    ///           if leading && start <= i < end && c != d { active[k] = false }
    ///       }
    ///   }
    ///   counter := counter + count                       // INSPECT ADDS; never clears
    /// ```
    ///
    /// PER-ITEM WINDOWS: each item's optional region defines a window over the source
    /// via the SAME [`Self::region_window`] helper the lone/single-item forms use
    /// (BEFORE→`[0, first_x)`; AFTER→`(first_x, len]`; not-found asymmetry BEFORE→whole,
    /// AFTER→empty). An item with NO region has the whole source as its window (so a
    /// region-less `LEADING` item anchors its run at source position 0). The first-match
    /// `break` is why DUPLICATE items do NOT double-count a position: `FOR ALL "a" ALL
    /// "a"` over `"aa"` adds 2 — each `a` position is counted ONCE by the first item, the
    /// second item never sees it. So the count collapses to "the number of source
    /// positions counted by the FIRST eligible in-window item, each counted exactly once".
    /// INSPECT adds to the counter; it does not clear it first, and the fold uses the
    /// SAME `store_result` path [`Self::inspect_tally`] uses (COBOL's silent high-order
    /// truncation on overflow), so the compiled `cobol-iir-compiler` scan loop matches
    /// this reference output byte-for-byte.
    ///
    /// # Non-ASCII-clean (no `str_slice`)
    ///
    /// TALLYING only COUNTS — it never reconstructs the source — so there is no
    /// UTF-8-boundary trap. Match-based counting of ASCII delimiters is byte-robust (a
    /// multi-byte source char's continuation bytes never equal an ASCII delimiter), and
    /// each window is content-defined (bounded by the first occurrence of the ASCII
    /// region delimiter), so this char-based oracle and the byte-based compiler scan the
    /// SAME substring and count the SAME matches even on a non-ASCII source. (A non-ASCII
    /// item/region delimiter *operand* stays the pre-existing `single_delim_char`
    /// vs `single_delim_code` chip, identical across single- and multi-item tallying.)
    ///
    /// Every delimiter (and every region delimiter) is validated with the SAME
    /// `single_delim_char` check the single-item tally uses (a multi-character/figurative/
    /// wider/numeric operand is a later rung); ALL of them — and every window — are
    /// resolved BEFORE counting so an invalid operand aborts without touching the
    /// counter. A numeric/group source is rejected by [`Self::inspect_alnum_source`], and
    /// a non-integer/signed/non-numeric counter by the validation below. The read-time
    /// reader (`read_inspect_tally_multi`) has already ruled out CHARACTERS items, so here
    /// every item is a single-char `ALL` OR `LEADING` delimiter with an optional region.
    fn exec_inspect_tally_multi(
        &mut self,
        source: &str,
        counter: &str,
        items: &[TallyMultiLeadingItem],
    ) -> Result<Flow, RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;

        // The counter must be an UNSIGNED INTEGER numeric item (`PIC 9(n)`): a
        // fractional (`V`) or signed (`S`) counter is a later rung — the SAME
        // validation `inspect_tally` performs for the single-item form.
        let cidx = *self
            .by_name
            .get(counter)
            .ok_or_else(|| RuntimeError::UndefinedName(counter.to_string()))?;
        match &self.items[cidx].picture {
            Some(Picture::Numeric { dec_digits: 0, signed: false, .. }) => {}
            Some(Picture::Numeric { .. }) => {
                return Err(RuntimeError::Unsupported(format!(
                    "INSPECT TALLYING into a non-integer or signed counter {counter} is a later rung"
                )))
            }
            _ => {
                return Err(RuntimeError::Unsupported(format!(
                    "INSPECT TALLYING into a non-numeric counter {counter} is a later rung"
                )))
            }
        }

        // ONE pass over the source characters — resolved FIRST so every window (like
        // the single-item path) is computed over these same chars, and so an invalid
        // operand aborts with the counter untouched.
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();

        // Resolve every (delimiter char, leading flag, [start, end) window) FIRST. Reading
        // all items (and computing their windows over `chars`) before touching the counter
        // means an invalid operand aborts cleanly, exactly like the single-item path
        // resolves both its delimiter and its window first. A region-less item's window is
        // the whole source (`region_window(None) == (0, len)`).
        let resolved: Vec<(char, bool, usize, usize)> = items
            .iter()
            .map(|(delim, leading, region)| {
                let d = self.single_delim_char(delim, "INSPECT")?;
                let (start, end) = self.region_window(&chars, region.as_ref())?;
                Ok((d, *leading, start, end))
            })
            .collect::<Result<_, RuntimeError>>()?;

        // ONE left-to-right pass with a per-item `active` run flag (only consulted for
        // `LEADING` items, all init `true`). At each position the FIRST ELIGIBLE item in
        // WRITTEN ORDER contributes 1: an `ALL` item is eligible iff its window contains
        // the position AND its delimiter matches; a `LEADING` item ALSO requires its run
        // still active (every prior IN-WINDOW position equalled its delimiter). This
        // realises first-match-per-position for a pure count (a position matched by
        // several/duplicate items is still counted once).
        let mut active = vec![true; resolved.len()];
        let mut count: usize = 0;
        for (i, &c) in chars.iter().enumerate() {
            // Tally decision: first eligible item wins, count once, stop.
            for (k, &(d, leading, start, end)) in resolved.iter().enumerate() {
                let in_win = start <= i && i < end;
                if in_win && c == d && (!leading || active[k]) {
                    count += 1;
                    break;
                }
            }
            // Then update EVERY `LEADING` item's run flag, INDEPENDENTLY of which item
            // tallied: a run breaks at the FIRST in-window position whose char is NOT its
            // delimiter (a matching char keeps the run alive even if a higher-priority item
            // claimed the position; positions outside the window neither begin nor break
            // the run — anchoring the run at the window start).
            for (k, &(d, leading, start, end)) in resolved.iter().enumerate() {
                if leading && start <= i && i < end && c != d {
                    active[k] = false;
                }
            }
        }

        // counter := counter_value + count, reshaped into the counter's picture — the
        // same store path ADD (and the single-item tally) uses. INSPECT adds; it does
        // not clear the counter first.
        let addend = Decimal { neg: false, int: count.to_string(), frac: String::new() };
        let acc = checked(add(&self.named_decimal(counter)?, &addend))?;
        self.store_result(counter, acc, false, &[])
    }

    /// `INSPECT src TALLYING c1 FOR ALL a [{BEFORE|AFTER} p] [ALL b …] c2 FOR ALL d …` —
    /// several counters, each with its own delimiter list, and each delimiter item now
    /// carrying its OWN optional `{BEFORE|AFTER}` region window, folded through ONE
    /// combined priority list in a SINGLE left-to-right pass. This generalises
    /// [`Self::exec_inspect_tally_multi`] from one counter to a list of `(counter,
    /// delimiter, window)` entries where the matched entry's OWN counter is bumped.
    ///
    /// ISO COMBINED-PRIORITY-LIST-ACROSS-COUNTERS semantics (the crux): all delimiters
    /// of all groups, flattened in WRITTEN ORDER (group 1's items first, then group 2's,
    /// …), form ONE ordered priority list, each entry carrying its item's `[start, end)`
    /// window. At each source position that list is walked in order and the FIRST entry
    /// whose window contains the position AND whose delimiter matches bumps ITS OWN
    /// group's counter, then the scan advances (single-char ⇒ a normal one-position
    /// step). The `break` means an earlier group's (in-window) delimiter CONSUMES the
    /// position — a character it claims NEVER reaches a later group's delimiter, so
    /// `"aa" TALLYING C1 FOR ALL "a" C2 FOR ALL "a"` gives C1 += 2, C2 += 0 (group 1
    /// wins both positions). A position matching no in-window delimiter advances with no
    /// increment. Each item's window is derived by the SHARED [`Self::region_window`]
    /// helper over the source chars (a region-less item = the whole source), applying the
    /// ISO not-found asymmetry (BEFORE→whole, AFTER→empty). Every delimiter and every
    /// region delimiter is resolved BEFORE the scan, so an invalid operand aborts with
    /// every counter untouched.
    fn exec_inspect_tally_counters(
        &mut self,
        source: &str,
        groups: &[TallyCounterGroup],
    ) -> Result<Flow, RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;

        // Validate EVERY counter FIRST — each must be an UNSIGNED INTEGER (`PIC 9(n)`),
        // the SAME check the single-item tally applies. Doing all of them (and all the
        // delimiter resolution below) before touching any counter means an invalid group
        // aborts with every counter untouched. The same counter name may legally appear
        // in two groups; validating it twice is harmless.
        for (counter, _) in groups {
            let cidx = *self
                .by_name
                .get(counter)
                .ok_or_else(|| RuntimeError::UndefinedName(counter.to_string()))?;
            match &self.items[cidx].picture {
                Some(Picture::Numeric { dec_digits: 0, signed: false, .. }) => {}
                Some(Picture::Numeric { .. }) => {
                    return Err(RuntimeError::Unsupported(format!(
                        "INSPECT TALLYING into a non-integer or signed counter {counter} is a later rung"
                    )))
                }
                _ => {
                    return Err(RuntimeError::Unsupported(format!(
                        "INSPECT TALLYING into a non-numeric counter {counter} is a later rung"
                    )))
                }
            }
        }

        // The source characters — resolved FIRST so every item's window is computed over
        // these same chars (and so an invalid operand aborts with the counters untouched).
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();

        // Flatten every delimiter to `(group_index, char, start, end)` in WRITTEN ORDER,
        // resolving all delimiters AND all windows (via the SAME `single_delim_char` /
        // `region_window` the single-item path uses, so an invalid operand rejects
        // identically) BEFORE the scan. `group_index` remembers which counter a match
        // belongs to; `[start, end)` is the item's window (a region-less item =
        // `region_window(None) == (0, len)`, the whole source).
        let mut flat: Vec<(usize, char, usize, usize)> = Vec::new();
        for (gi, (_counter, items)) in groups.iter().enumerate() {
            for (delim, region) in items {
                let dch = self.single_delim_char(delim, "INSPECT")?;
                let (start, end) = self.region_window(&chars, region.as_ref())?;
                flat.push((gi, dch, start, end));
            }
        }

        // ONE pass over the source. Per position `i` walk the flattened list in order;
        // the FIRST entry whose window contains `i` AND whose delimiter matches bumps that
        // group's accumulator and breaks (first-match-wins across counters). A per-GROUP
        // accumulator (not per counter NAME) keeps the counts separate even when two
        // groups share a counter name — they are summed into that one item below.
        let mut accs = vec![0usize; groups.len()];
        for (i, ch) in chars.iter().enumerate() {
            for (gi, dch, start, end) in &flat {
                if *start <= i && i < *end && *ch == *dch {
                    accs[*gi] += 1;
                    break;
                }
            }
        }

        // Add each group's accumulator to its counter, via the SAME store path ADD (and
        // the single-item tally) uses. `named_decimal` re-reads the counter each time, so
        // if the SAME counter name appears in two groups both groups' shares accumulate
        // into that one item correctly. INSPECT adds; it never clears a counter.
        let mut flow = Flow::Normal;
        for (gi, (counter, _)) in groups.iter().enumerate() {
            let addend = Decimal { neg: false, int: accs[gi].to_string(), frac: String::new() };
            let acc = checked(add(&self.named_decimal(counter)?, &addend))?;
            flow = self.store_result(counter, acc, false, &[])?;
        }
        Ok(flow)
    }

    /// `INSPECT source TALLYING counter FOR {ALL|LEADING} delim REPLACING
    /// {ALL|LEADING} search BY replace` — one INSPECT carrying BOTH phrases. Per
    /// ISO this runs "as though an INSPECT TALLYING were specified, followed by an
    /// INSPECT REPLACING", so the order is fixed: FIRST [`Self::inspect_tally`]
    /// counts `delim` in the ORIGINAL (pre-replacement) storage and adds to
    /// `counter`, THEN [`Self::inspect_replace`] maps `search`→`replace` in the
    /// source. Running tally before replace is what makes `delim == search`
    /// correct — the count sees the bytes as they were, and only afterwards are
    /// they overwritten. The TALLYING half honours `tally_leading` (`FOR LEADING`
    /// counts only the leading run of `delim`, `FOR ALL` counts every occurrence);
    /// the REPLACING half independently honours `replace_leading` (`LEADING`
    /// rewrites only the leading run of `search`, `ALL` rewrites every
    /// occurrence). The `cobol-iir-compiler` composes the same two lowerings in
    /// the same order with the same two flags, so the compiled program matches
    /// this reference byte-for-byte.
    ///
    /// Each half independently carries an optional `{BEFORE|AFTER} x` region
    /// (`tally_region` / `replace_region`). Because the tally does NOT mutate the
    /// source, BOTH windows are derived (via [`Self::region_window`], the same
    /// helper the lone forms use) over the SAME original storage — the count's
    /// window and the replacement's window each see the pre-replacement bytes. The
    /// two regions are otherwise fully independent (different kind, different
    /// delimiter, either/both/neither present).
    // Ten parameters: the source/counter/delim/search/replace operands, the two
    // INDEPENDENT leading flags (tally half, replace half), and the two INDEPENDENT
    // optional regions. Grouping them into a struct would only re-spell the same
    // fields the caller already destructured from `Stmt::InspectTallyReplace`, so the
    // flat signature is clearer here.
    #[allow(clippy::too_many_arguments)]
    fn exec_inspect_tally_replace(
        &mut self,
        source: &str,
        counter: &str,
        delim: &Operand,
        tally_leading: bool,
        tally_region: Option<&Region>,
        search: &Operand,
        replace: &Operand,
        replace_leading: bool,
        replace_region: Option<&Region>,
    ) -> Result<Flow, RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;
        // Tally FIRST, on the current (original) storage — it does not mutate the
        // source, so the subsequent replace still sees the original bytes too. The
        // TALLYING half may be FOR ALL or FOR LEADING (`tally_leading`) and may carry
        // its OWN `{BEFORE|AFTER}` region (`tally_region`), whose window is computed
        // over the original storage.
        // The combined form never carries a CHARACTERS tally (rejected at read time),
        // so `characters` is always `false` here.
        self.inspect_tally(sidx, counter, delim, tally_leading, false, tally_region)?;
        // THEN replace, overwriting the source in place. The REPLACING half may be
        // ALL or LEADING (`replace_leading`) — the same leading-run map used by a
        // lone `INSPECT REPLACING LEADING` — and carries its OWN INDEPENDENT
        // `{BEFORE|AFTER}` region (`replace_region`). Since the tally left the source
        // untouched, this half's window is also over the SAME original storage.
        self.inspect_replace(sidx, search, replace, replace_leading, replace_region)?;
        Ok(Flow::Normal)
    }

    /// The REPLACING half: substitute `search` characters with `replace` in the
    /// source's storage, in place (same width). Factored out of
    /// [`Self::exec_inspect_replacing`] so the combined exec can run it AFTER the
    /// tally. When `leading` is false (`REPLACING ALL`) EVERY occurrence of
    /// `search` becomes `replace`; when true (`REPLACING LEADING`) only the run of
    /// CONSECUTIVE `search` characters at the START of the source is replaced,
    /// stopping at the first character that is not `search` — positions after that
    /// first gap are left unchanged even if they equal `search`. The combined
    /// tally-then-replace path passes whichever `leading` the statement's REPLACING
    /// half selected — this map is shared verbatim by the lone `REPLACING LEADING`
    /// and the combined `TALLYING … REPLACING LEADING`. A numeric/group source is
    /// rejected by the caller via [`Self::inspect_alnum_source`].
    ///
    /// An optional `region` (`{BEFORE|AFTER} x`) narrows BOTH maps to the window
    /// `[start, end)` [`Self::region_window`] derives over the ORIGINAL source. For
    /// `ALL` a position is rewritten only when it is BOTH inside the window AND equal
    /// to `search`; a position outside the window keeps its original character even if
    /// it equals `search`. For `LEADING` the run is anchored at the WINDOW START, not
    /// source position 0: characters before `start` are copied through UNCHANGED and
    /// do NOT begin or break the run, the run begins at `start`, and it stops at the
    /// first non-`search` character INSIDE the window (or the window end). So a
    /// standalone `REPLACING LEADING … AFTER x` rewrites the leading run beginning at
    /// `first+1` — e.g. "aaXaab" AFTER "X" narrows to "aab" and rewrites its leading
    /// a's, leaving the "aa" before the X untouched. With `AFTER x` and `x` absent the
    /// window is empty, so nothing is replaced (the ISO not-found asymmetry). With no
    /// region the window is the whole source, so both maps are unchanged. The combined
    /// `TALLYING … REPLACING` form now composes this SAME routine for its LEADING
    /// replace half carrying a region — so LEADING and a narrowed window combine on the
    /// COMBINED path too, byte-identical to the standalone `REPLACING LEADING …
    /// {BEFORE|AFTER}` form.
    fn inspect_replace(
        &mut self,
        sidx: usize,
        search: &Operand,
        replace: &Operand,
        leading: bool,
        region: Option<&Region>,
    ) -> Result<(), RuntimeError> {
        // The single search and replacement characters (shared validation with
        // UNSTRING/TALLYING: a multi-character/figurative/wider/numeric operand is
        // a later rung). Read both BEFORE mutating so an invalid replacement does
        // not leave the source half-changed.
        let search_ch = self.single_delim_char(search, "INSPECT REPLACING")?;
        let replace_ch = self.single_delim_char(replace, "INSPECT REPLACING")?;

        // The ORIGINAL source characters and the region window over them. We compute
        // the window BEFORE rebuilding so — exactly like the count — it sees the
        // pre-replacement bytes, and we reuse the SAME helper the tally uses so both
        // narrow to the identical slice.
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();
        let (start, end) = self.region_window(&chars, region)?;

        // Rebuild each character in place (same width), then store through the
        // alphanumeric char path. `REPLACING ALL` maps every in-window match;
        // `REPLACING LEADING` replaces only while still in the leading run — a
        // stateful map that flips `in_run` off at the first non-`search` character
        // and never replaces again, even for a later `search`. The run is anchored at
        // the WINDOW START: a position OUTSIDE `[start, end)` is copied through
        // unchanged and leaves `in_run` untouched (characters before `start` neither
        // begin nor break the run), so the run genuinely starts at `start`. With no
        // region `start = 0` and `end = len`, so every position is "inside" and the
        // map reduces to the plain leading-run rewrite. With `AFTER x` and `x` absent
        // the window is empty (`start = end = len`), so no position is inside and
        // nothing is replaced.
        let rebuilt: String = if leading {
            let mut in_run = true;
            chars
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    if i < start || i >= end {
                        // Outside the region window: keep the original character and
                        // leave the run state untouched (anchor the run at `start`).
                        c
                    } else if in_run && c == search_ch {
                        replace_ch
                    } else {
                        in_run = false;
                        c
                    }
                })
                .collect()
        } else {
            // ALL: rewrite a matching character only when its position `i` lies inside
            // the region window `[start, end)`; outside the window the original char
            // is kept. (No region ⇒ `start = 0`, `end = len` ⇒ every match rewritten.)
            chars
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    if i >= start && i < end && c == search_ch {
                        replace_ch
                    } else {
                        c
                    }
                })
                .collect()
        };
        self.move_into(sidx, Src::Chars(rebuilt))
    }

    /// `INSPECT source CONVERTING from TO to` — translate each character of the
    /// alphanumeric `source` through a per-character **translation table** built
    /// from the two EQUAL-length string literals `from` and `to`: a source
    /// character equal to `from[k]` becomes `to[k]`, where `k` is the FIRST index
    /// at which it appears in `from` (so if `from` repeats a character, the
    /// LEFTMOST entry wins); a character in no table entry is left unchanged. The
    /// source's width is unchanged (each character maps to exactly one), so — like
    /// [`Self::inspect_replace`] — the rebuilt string feeds the SAME alphanumeric
    /// char-store path a `MOVE` uses, and the compiled `cobol-iir-compiler`
    /// per-position lowering matches this reference output byte-for-byte.
    ///
    /// This rung: `from`/`to` are string LITERALS of equal length. An unequal-
    /// length pair is a clean later-rung error; a `PIC X` item / figurative /
    /// reference-modified `from`/`to` and a numeric/group source are rejected by the
    /// reader and [`Self::inspect_alnum_source`].
    ///
    /// An optional `region` (`{BEFORE|AFTER} x`) narrows the translation to the
    /// window `[start, end)` [`Self::region_window`] derives over the ORIGINAL
    /// source — the SAME helper the count and `ALL` replacement use, so all three
    /// INSPECT operations narrow to byte-identical slices. A position INSIDE the
    /// window is translated through the table; a position OUTSIDE keeps its original
    /// character even if it appears in the `from` set. With no region the window is
    /// the whole source, so every character is translated exactly as before.
    fn exec_inspect_converting(
        &mut self,
        source: &str,
        from: &ConvertOperand,
        to: &ConvertOperand,
        region: Option<&Region>,
    ) -> Result<(), RuntimeError> {
        let sidx = self.inspect_alnum_source(source)?;

        // Resolve each `from`/`to` operand to its translation-set string FIRST — a
        // literal is its own characters; a data-name reads the item's CURRENT
        // storage. This read is loop-invariant (the `from`/`to` item does not change
        // during the translate) and happens BEFORE the source is rewritten below, so
        // even a `from`/`to` that ALIASES the source sees the ORIGINAL bytes — the
        // same up-front table build the compiler mirrors.
        let from = self.converting_operand_str(from)?;
        let to = self.converting_operand_str(to)?;

        // The table pairs `from[k]` with `to[k]`, so the two must be equal length.
        let from_chars: Vec<char> = from.chars().collect();
        let to_chars: Vec<char> = to.chars().collect();
        if from_chars.len() != to_chars.len() {
            return Err(RuntimeError::Unsupported(
                "INSPECT CONVERTING with unequal-length FROM/TO operands is a later rung".into(),
            ));
        }

        // Build the char→char map, FIRST occurrence wins (so a duplicated `from`
        // character keeps its leftmost `to` partner — `or_insert` never overwrites).
        let mut table: HashMap<char, char> = HashMap::new();
        for (f, t) in from_chars.iter().zip(to_chars.iter()) {
            table.entry(*f).or_insert(*t);
        }

        // The ORIGINAL source characters and the region window over them, derived by
        // the SHARED helper (BEFORE→whole / AFTER→empty when `x` is absent). No
        // region ⇒ `(0, len)` ⇒ every position translated, exactly as before.
        let chars: Vec<char> = self.items[sidx].storage.chars().collect();
        let (start, end) = self.region_window(&chars, region)?;

        // Map each source character through the table only when its position lies
        // inside the window; a position outside the window keeps its original
        // character (unmapped in-window characters also pass through unchanged).
        let rebuilt: String = chars
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                if i >= start && i < end {
                    *table.get(&c).unwrap_or(&c)
                } else {
                    c
                }
            })
            .collect();
        self.move_into(sidx, Src::Chars(rebuilt))
    }

    /// Resolve a CONVERTING `from`/`to` operand to its translation-set string. A
    /// [`ConvertOperand::Literal`] is its own characters. A [`ConvertOperand::Item`]
    /// reads the data-name's CURRENT storage — but only when it names an ALPHANUMERIC
    /// (`PIC X`) item; a numeric or group item as `from`/`to` is a clean later rung,
    /// rejected here exactly as the compiler rejects it at build time (so both engines
    /// accept and reject the very same programs). The length used by the equal-length
    /// check is therefore the item's declared width — its stored image is always that
    /// many characters wide.
    fn converting_operand_str(&self, op: &ConvertOperand) -> Result<String, RuntimeError> {
        match op {
            ConvertOperand::Literal(s) => Ok(s.clone()),
            ConvertOperand::Item(name) => {
                let idx = *self
                    .by_name
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                match &self.items[idx].picture {
                    Some(p) if p.is_numeric() => Err(RuntimeError::Unsupported(
                        "INSPECT CONVERTING with a numeric FROM/TO item is a later rung".into(),
                    )),
                    Some(_) => Ok(self.items[idx].storage.clone()),
                    None => Err(RuntimeError::Unsupported(
                        "INSPECT CONVERTING with a group FROM/TO item is a later rung".into(),
                    )),
                }
            }
            // A CONSTANT refmod set: its characters are the slice `base(start:len)`,
            // produced by the SAME [`Self::refmod_string`] evaluator the MOVE-source,
            // STRING-sending-field, and DISPLAY paths use — resolved up front (before
            // any per-position write-back), so a refmod whose base ALIASES the source
            // slices the ORIGINAL bytes. `refmod_string` rejects a numeric base
            // (`later rung`) exactly as the compiler's `ref_mod_slice` does, keeping
            // accept/reject identical on both engines. The slice's length is the const
            // `len` (or `base_width - start + 1` when omitted) — a static width, so the
            // equal-length check stays fixed just like a data-name's declared width.
            ConvertOperand::RefMod { base, start, len } => self.refmod_string(base, start, len),
        }
    }

    /// `EVALUATE subject WHEN … END-EVALUATE` — run the first branch whose value
    /// equals the subject (a `WHEN OTHER` matches unconditionally once reached),
    /// then stop (no fall-through). Numeric comparison this rung; an alphanumeric
    /// subject/value is a later rung. Branches are tested by **iteration**, so many
    /// `WHEN`s cannot overflow the stack. The branch's `Flow` propagates (a
    /// `STOP RUN` or `GO TO` inside a `WHEN` unwinds), like an `IF` branch.
    fn exec_evaluate(
        &mut self,
        subject: &Operand,
        branches: &[(Option<Vec<WhenValue>>, Vec<Stmt>)],
    ) -> Result<Flow, RuntimeError> {
        for (when, stmts) in branches {
            let matches = match when {
                None => true, // WHEN OTHER
                Some(values) => self.subject_in_when(subject, values)?,
            };
            if matches {
                return self.run_stmts(stmts);
            }
        }
        Ok(Flow::Normal)
    }

    /// Whether the subject equals any single `WHEN` value or falls within any
    /// inclusive `THRU` range in the list. Comparisons go through
    /// [`Self::compare_operands`], so a numeric *or* alphanumeric subject works.
    fn subject_in_when(&self, subject: &Operand, values: &[WhenValue]) -> Result<bool, RuntimeError> {
        use std::cmp::Ordering;
        for wv in values {
            let hit = match wv {
                WhenValue::Single(v) => self.compare_operands(subject, v)? == Ordering::Equal,
                WhenValue::Range(lo, hi) => {
                    self.compare_operands(subject, lo)? != Ordering::Less
                        && self.compare_operands(subject, hi)? != Ordering::Greater
                }
            };
            if hit {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `SET cond-name TO TRUE` — assign the condition-name's conditional variable
    /// the value that makes it hold: the **first** of its `VALUE` items (the low
    /// bound of a leading range).
    ///
    /// Two accepted variable kinds, decided by the picture:
    ///
    /// A **numeric** variable takes the first value's numeric image formatted into
    /// its slot (`src_from_lit` → `move_into`) — the same store `MOVE 9 TO N` does.
    /// A leading `THRU` range contributes its low bound.
    ///
    /// An **alphanumeric** (`PIC X`) variable is supported for STRING values: when
    /// every VALUE item is a discrete string literal or a string `THRU` range
    /// ([`all_str_values`]), SET stores the FIRST value's string into the slot
    /// exactly as `MOVE "…" TO item` (`src_from_lit` yields `Src::Chars`, which
    /// `move_into` fits to the receiver width) — a leading discrete string `s`
    /// stores `s`; a leading range `lo THRU _` stores its LOW bound `lo`, mirroring
    /// the numeric arm. A range with a NON-string bound, a numeric/figurative VALUE,
    /// or a mixed list on an alphanumeric variable stays a later rung — rejected
    /// identically to the compiler. A group conditional variable (no picture)
    /// likewise stays a later rung.
    fn exec_set_true(&mut self, cond_name: &str) -> Result<(), RuntimeError> {
        let cn = self
            .conditions
            .get(cond_name)
            .ok_or_else(|| RuntimeError::UndefinedName(cond_name.to_string()))?;
        let var = cn.var;
        let is_numeric = self.items[var].picture.as_ref().is_some_and(|p| p.is_numeric());
        if is_numeric {
            // Numeric slot: the first value item — a single value, or a range's low
            // bound — formatted into the numeric picture (unchanged behaviour).
            let lit = match cn.values.first() {
                Some(ValueSpec::Single(lit)) | Some(ValueSpec::Range(lit, _)) => lit.clone(),
                None => {
                    return Err(RuntimeError::Unsupported(format!(
                        "condition-name {cond_name} has no VALUE"
                    )))
                }
            };
            let src = self.src_from_lit(&lit)?;
            return self.move_into(var, src);
        }
        // Alphanumeric (`PIC X`) slot: accept when every VALUE is a string
        // (discrete or a string range); store the FIRST value's string into the slot
        // exactly as `MOVE "…" TO item` — a discrete string, or a range's LOW bound.
        if self.items[var].picture.is_some() && all_str_values(&cn.values) {
            let lit = match cn.values.first() {
                Some(ValueSpec::Single(lit)) | Some(ValueSpec::Range(lit, _)) => lit.clone(),
                None => {
                    return Err(RuntimeError::Unsupported(format!(
                        "condition-name {cond_name} has no VALUE"
                    )))
                }
            };
            let src = self.src_from_lit(&lit)?;
            return self.move_into(var, src);
        }
        Err(RuntimeError::Unsupported(
            "SET … TO TRUE needs a numeric condition-name, or an alphanumeric one with \
             string VALUEs (a discrete string or a string THRU range; a range with a non-string \
             bound, a numeric/figurative VALUE, or a group conditional variable is a later rung)"
                .into(),
        ))
    }

    /// Execute a sequence of statements, short-circuiting on the first non-normal
    /// [`Flow`] (a `STOP RUN` or `GO TO`), which propagates up to unwind any
    /// enclosing `IF`/`PERFORM`/handler. Shared by `IF` branches,
    /// `COMPUTE … ON SIZE ERROR` handlers, and performed paragraphs.
    fn run_stmts(&mut self, stmts: &[Stmt]) -> Result<Flow, RuntimeError> {
        for s in stmts {
            match self.exec_stmt(s)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    /// `PERFORM target [n TIMES | UNTIL cond]` — run one paragraph out of line,
    /// then return. Bare form: once. `n TIMES`: a fixed count (`≤ 0` runs zero
    /// times, a fractional count truncates). `UNTIL cond`: repeat while the
    /// condition is false, testing it **before** each iteration (so a
    /// initially-true condition runs zero times).
    ///
    /// The repeat loop is iterative, so even a never-satisfied `UNTIL` (an
    /// infinite loop — the programmer's bug, valid COBOL) does not grow the
    /// stack. Nesting of *distinct* performs is bounded by [`MAX_PERFORM_DEPTH`]
    /// so a self-performing paragraph fails cleanly instead of overflowing. A
    /// `STOP RUN` or `GO TO` inside stops the repetition and propagates as its
    /// [`Flow`].
    fn exec_perform(
        &mut self,
        target: &str,
        thru: &Option<String>,
        mode: &PerformMode,
    ) -> Result<Flow, RuntimeError> {
        let start = *self
            .para_index
            .get(target)
            .ok_or_else(|| RuntimeError::UndefinedName(target.into()))?;
        // The range end: `target` itself without THRU, else the THRU paragraph
        // (which must not precede `target` in source order).
        let end = match thru {
            None => start,
            Some(t) => {
                let e = *self
                    .para_index
                    .get(t)
                    .ok_or_else(|| RuntimeError::UndefinedName(t.clone()))?;
                if e < start {
                    return Err(RuntimeError::Unsupported(
                        "PERFORM … THRU range runs backwards".into(),
                    ));
                }
                e
            }
        };

        self.perform_depth += 1;
        if self.perform_depth > MAX_PERFORM_DEPTH {
            self.perform_depth -= 1;
            return Err(RuntimeError::Unsupported(
                "PERFORM nesting too deep (a paragraph performing itself?)".into(),
            ));
        }
        // One body iteration runs the whole paragraph range `start..=end` in
        // source order (falling through between them); it returns
        // Some(flow-to-propagate) to stop repeating, or None to continue.
        // Outcomes are captured (never `?`-ed out) so the depth counter is
        // restored on every path.
        let run_body = |m: &mut Self| {
            for i in start..=end {
                let stmts = m.paragraphs[i].stmts.clone();
                match m.run_stmts(&stmts) {
                    Ok(Flow::Normal) => {}          // fall through to the next
                    other => return Some(other),    // Stop / GoTo / Err propagates
                }
            }
            None
        };

        let outcome = match mode {
            PerformMode::Once => run_body(self).unwrap_or(Ok(Flow::Normal)),
            PerformMode::Times(op) => match self.perform_count(op) {
                Ok(n) => {
                    let mut o = Ok(Flow::Normal);
                    for _ in 0..n {
                        if let Some(flow) = run_body(self) {
                            o = flow;
                            break;
                        }
                    }
                    o
                }
                Err(e) => Err(e),
            },
            PerformMode::Until(cond) => self.perform_until(cond, &run_body),
            PerformMode::Varying { var, from, by, until } => {
                // id := from, then the same TEST-BEFORE loop, stepping id by `by`
                // after each body run.
                match self
                    .operand_decimal(from)
                    .and_then(|start| self.store_number(var, start))
                {
                    Err(e) => Err(e),
                    Ok(()) => self.perform_until(until, &|m: &mut Self| {
                        // A body Stop/GoTo/Err wins; otherwise step id (a step
                        // error — e.g. overflow — stops the loop and propagates).
                        if let Some(f) = run_body(m) {
                            return Some(f);
                        }
                        match m.step_var(var, by) {
                            Ok(()) => None,
                            Err(e) => Some(Err(e)),
                        }
                    }),
                }
            }
        };
        self.perform_depth -= 1;
        outcome
    }

    /// Resolve a `PERFORM … TIMES` count: a non-negative integer; `≤ 0` → 0.
    fn perform_count(&self, op: &Operand) -> Result<usize, RuntimeError> {
        let d = self.operand_decimal(op)?;
        if d.neg {
            return Ok(0);
        }
        let int = d.int.trim_start_matches('0');
        if int.is_empty() {
            return Ok(0);
        }
        int.parse::<usize>()
            .map_err(|_| RuntimeError::Unsupported("PERFORM … TIMES count is too large".into()))
    }

    /// The shared TEST-BEFORE loop: while `cond` is false, run `body` (which
    /// returns `Some(flow)` to stop and propagate, `None` to continue). Iterative,
    /// so a never-satisfied condition hangs but never grows the stack.
    fn perform_until(
        &mut self,
        cond: &Cond,
        body: &dyn Fn(&mut Self) -> Option<Result<Flow, RuntimeError>>,
    ) -> Result<Flow, RuntimeError> {
        loop {
            match self.eval_cond(cond) {
                Ok(true) => return Ok(Flow::Normal),
                Ok(false) => {}
                Err(e) => return Err(e),
            }
            if let Some(flow) = body(self) {
                return flow;
            }
        }
    }

    /// Step a `PERFORM VARYING` induction variable: `var := var + by`.
    fn step_var(&mut self, var: &str, by: &Operand) -> Result<(), RuntimeError> {
        let cur = self.named_decimal(var)?;
        let step = self.operand_decimal(by)?;
        let next = checked(add(&cur, &step))?;
        self.store_number(var, next)
    }

    /// Evaluate a relational condition. Numeric when both sides are numeric;
    /// otherwise an alphanumeric (space-padded) character comparison — COBOL's
    /// rule. Figurative constants take the category/length of the other operand.
    fn eval_cond(&self, cond: &Cond) -> Result<bool, RuntimeError> {
        match cond {
            Cond::Relation { left, op, negated, right } => self.eval_relation(left, *op, *negated, right),
            Cond::ConditionName(name) => self.eval_condition_name(name),
            // Iterate the flat parts (short-circuiting) rather than recursing on
            // the chain length: a long `AND`/`OR` recurses only into nested
            // parenthesised groups, whose depth the parser already caps.
            Cond::And(parts) => {
                for c in parts {
                    if !self.eval_cond(c)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Cond::Or(parts) => {
                for c in parts {
                    if self.eval_cond(c)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Cond::Not(inner) => Ok(!self.eval_cond(inner)?),
        }
    }

    /// Evaluate a level-88 condition-name: does its conditional variable equal any
    /// single value, or fall within any inclusive `THRU` range?
    ///
    /// A **numeric** variable compares its decimal value against each numeric VALUE
    /// / `THRU` range (unchanged).
    ///
    /// An **alphanumeric** (`PIC X`) variable is supported for STRING values: when
    /// every VALUE item is a discrete string literal or a string `THRU` range
    /// ([`all_str_values`]), the name holds when the variable matches ANY of them
    /// under COBOL's alphanumeric comparison. Every comparison is the SAME
    /// space-padded byte compare an `IF var = "…"` / `IF var >= "…"` relation runs,
    /// routed through [`Self::compare_operands`] (which pads both sides to a common
    /// width and byte-compares):
    ///
    ///   * a discrete string `s` holds when `var == s` (ordering `Equal`);
    ///   * an inclusive range `lo THRU hi` holds when `var >= lo` (ordering not
    ///     `Less`) AND `var <= hi` (ordering not `Greater`).
    ///
    /// The per-value results OR-fold (any hit → true), exactly like the numeric arm.
    /// Reusing `compare_operands` is what makes the read byte-identical to the
    /// compiler's `str_cmp`. A `THRU` range with a NON-string bound, a
    /// numeric/figurative VALUE, or a mixed list on an alphanumeric variable stays a
    /// later rung; a group conditional variable (no picture) likewise.
    fn eval_condition_name(&self, name: &str) -> Result<bool, RuntimeError> {
        let cn = self
            .conditions
            .get(name)
            .ok_or_else(|| RuntimeError::UndefinedName(name.to_string()))?;
        let item = &self.items[cn.var];
        let is_numeric = item.picture.as_ref().is_some_and(|p| p.is_numeric());
        if is_numeric {
            let lhs = self.item_as_decimal(cn.var);
            for spec in &cn.values {
                let hit = match spec {
                    ValueSpec::Single(lit) => {
                        lhs.cmp_value(&num_value(lit)?) == std::cmp::Ordering::Equal
                    }
                    ValueSpec::Range(lo, hi) => {
                        lhs.cmp_value(&num_value(lo)?) != std::cmp::Ordering::Less
                            && lhs.cmp_value(&num_value(hi)?) != std::cmp::Ordering::Greater
                    }
                };
                if hit {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        // Alphanumeric (`PIC X`) variable: accept string VALUEs (discrete or a
        // string `THRU` range), then hold when the variable matches ANY of them
        // under the alphanumeric byte compare — the identical `compare_operands`
        // path an `IF var = "…"` / `IF var >= "…"` relation uses.
        if item.picture.is_some() && all_str_values(&cn.values) {
            use std::cmp::Ordering;
            let var_op = Operand::Ident(cn.var_name.clone());
            for spec in &cn.values {
                let hit = match spec {
                    // Discrete string: var == s.
                    ValueSpec::Single(lit) => {
                        self.compare_operands(&var_op, &Operand::Lit(lit.clone()))? == Ordering::Equal
                    }
                    // Inclusive range: var >= lo (not Less) AND var <= hi (not Greater).
                    ValueSpec::Range(lo, hi) => {
                        self.compare_operands(&var_op, &Operand::Lit(lo.clone()))? != Ordering::Less
                            && self.compare_operands(&var_op, &Operand::Lit(hi.clone()))?
                                != Ordering::Greater
                    }
                };
                if hit {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        Err(RuntimeError::Unsupported(
            "a level-88 condition-name needs a numeric variable, or an alphanumeric one with \
             string VALUEs (a discrete string or a string THRU range; a range with a non-string \
             bound, a numeric/figurative VALUE, or a group conditional variable is a later rung)"
                .into(),
        ))
    }

    fn eval_relation(
        &self,
        left: &Operand,
        op: RelOp,
        negated: bool,
        right: &Operand,
    ) -> Result<bool, RuntimeError> {
        use std::cmp::Ordering;
        let ordering = self.compare_operands(left, right)?;
        let base = match op {
            RelOp::Greater => ordering == Ordering::Greater,
            RelOp::Less => ordering == Ordering::Less,
            RelOp::Equal => ordering == Ordering::Equal,
        };
        Ok(base ^ negated)
    }

    /// Order two operands the way COBOL's relational and `EVALUATE` comparisons
    /// do: a **numeric** comparison when both are numeric (or one is the `ZERO`
    /// figurative), else an **alphanumeric** comparison — each side's characters
    /// (a numeric's are its digit image), a figurative expanded to the other's
    /// length, then both space-padded to a common length and byte-compared.
    ///
    /// A **mixed** numeric ↔ alphanumeric comparison (one side a numeric item, the
    /// other alphanumeric) falls into that alphanumeric arm: COBOL treats the
    /// numeric operand as though moved to an alphanumeric field — its digit image,
    /// which `Decimal::digits()` yields as the item's fixed-width zero-padded
    /// storage (`PIC 9(3) = 42` → `"042"`; a scaled `PIC 9(2)V9 = 4.2` → `"042"`,
    /// its `(int + frac)` digits with no point) — and compares by the byte rule. An
    /// **unsigned** numeric item, integer OR scaled (`PIC 9(i)V9(d)`), has that plain
    /// magnitude image. A **signed** (`PIC S9…`) numeric item is also supported: its
    /// comparison image is the same magnitude with the operational sign folded into a
    /// TRAILING OVERPUNCH on the units (last) digit (`overpunch_trailing`) — exactly
    /// the bytes the signed numeric→alphanumeric MOVE produces. So `PIC S9(3) = -123`
    /// compares equal to `"12L"`, `= +123` equal to `"12C"`, and a scaled
    /// `PIC S9V9 = -4.2` equal to `"4K"`. A value that truncates to a zero magnitude
    /// stores `neg = false` (COBOL has no negative zero), so its image is `"00{"`.
    /// A **group** item in a mixed comparison is still a clean later rung — rejected
    /// here so the oracle matches the compiler, which rejects it at compile time. A
    /// numeric *literal* vs an alphanumeric operand is a **different pairing** than a
    /// numeric ITEM vs alphanumeric (outside this rung's scope) and is rejected here
    /// too, so the oracle matches the compiler rather than silently answering it.
    fn compare_operands(&self, left: &Operand, right: &Operand) -> Result<std::cmp::Ordering, RuntimeError> {
        let l = self.src_from_operand(left)?;
        let r = self.src_from_operand(right)?;
        // A mixed comparison surfaces as one numeric `Src` and one alphanumeric —
        // either a character string (`Src::Chars`) or a figurative such as `SPACE`
        // (`Src::Fig`, but NOT `ZERO`, which is numeric and handled by the value
        // arms below). A signed numeric operand IS supported (its overpunched image
        // is used in the byte arm below); only a group item participating in a mixed
        // comparison is still deferred, so this engine errors precisely where the
        // compiler does — including for a numeric-vs-figurative pairing.
        let alnum_fig = |s: &Src| matches!(s, Src::Fig(f) if !matches!(f, Fig::Zero));
        let mixed = matches!(&l, Src::Num(_)) && (matches!(&r, Src::Chars(_)) || alnum_fig(&r))
            || matches!(&r, Src::Num(_)) && (matches!(&l, Src::Chars(_)) || alnum_fig(&l));
        if mixed {
            for op in [left, right] {
                // A numeric LITERAL against an alphanumeric operand is a *different*
                // pairing than a numeric ITEM vs alphanumeric — out of this rung's
                // scope, and rejected by the compiler (`num_digit_str_operand`), so
                // the oracle rejects it too rather than silently answering (which
                // would be a stricter-compiler asymmetry).
                if matches!(op, Operand::Lit(Lit::Num(_))) {
                    return Err(RuntimeError::Unsupported(
                        "a numeric literal compared with an alphanumeric operand is a later rung \
                         (a different pairing)"
                            .into(),
                    ));
                }
                if self.operand_is_group(op) {
                    return Err(RuntimeError::Unsupported(
                        "a group item compared with a numeric operand is a later rung".into(),
                    ));
                }
            }
        }
        Ok(match (&l, &r) {
            (Src::Num(a), Src::Num(b)) => a.cmp_value(b),
            (Src::Num(a), Src::Fig(Fig::Zero)) => a.cmp_value(&Decimal::zero()),
            (Src::Fig(Fig::Zero), Src::Num(b)) => Decimal::zero().cmp_value(b),
            _ => {
                // A SIGNED numeric operand compares by its overpunched magnitude image
                // (`overpunch_trailing` — the same bytes the signed→alphanumeric MOVE
                // produces), not its plain digit string; an unsigned numeric, a
                // figurative, or a literal keeps its ordinary `src_chars` image, so
                // this changes the compared string ONLY for a signed numeric operand.
                let mut ls = self.signed_overpunch_image(left).unwrap_or_else(|| src_chars(&l));
                let mut rs = self.signed_overpunch_image(right).unwrap_or_else(|| src_chars(&r));
                if let Src::Fig(f) = &l {
                    ls = fill_fig(f, rs.len().max(1));
                }
                if let Src::Fig(f) = &r {
                    rs = fill_fig(f, ls.len().max(1));
                }
                let width = ls.len().max(rs.len());
                format!("{ls:<width$}").cmp(&format!("{rs:<width$}"))
            }
        })
    }

    /// The overpunched comparison image of a **signed** numeric data-name operand:
    /// its stored magnitude with the operational sign folded into a trailing
    /// overpunch on the units digit (`overpunch_trailing`), the exact bytes a signed
    /// numeric→alphanumeric MOVE of the same item emits — so a mixed comparison
    /// against an alphanumeric operand byte-matches the compiler's image. Returns
    /// `None` for any other operand (unsigned numeric, literal, group, ref-mod),
    /// leaving the caller's ordinary `src_chars` image in place.
    fn signed_overpunch_image(&self, op: &Operand) -> Option<String> {
        if let Operand::Ident(name) = op {
            if let Some(&idx) = self.by_name.get(name) {
                if let Some(Picture::Numeric { signed: true, .. }) = &self.items[idx].picture {
                    return Some(overpunch_trailing(&self.items[idx].storage, self.items[idx].neg));
                }
            }
        }
        None
    }

    /// Whether `op` is a data-name referring to a **group** item (no picture — its
    /// value is the concatenation of its children). A group item in a mixed
    /// numeric↔alphanumeric comparison is a later rung.
    fn operand_is_group(&self, op: &Operand) -> bool {
        if let Operand::Ident(name) = op {
            if let Some(&idx) = self.by_name.get(name) {
                return self.items[idx].picture.is_none();
            }
        }
        false
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
            let idx = *self.by_name.get(dst).ok_or_else(|| RuntimeError::UndefinedName(dst.clone()))?;
            // Reference-modification SOURCE `base(start:len)` moved into an
            // ALPHANUMERIC receiver. The slice is obtained from `refmod_string`
            // (the SAME char range the compiler emits as a `str_slice`, so DISPLAY
            // of that slice already agrees byte-for-byte), then char-moved into the
            // receiver by the ordinary alphanumeric rule (`Src::Chars` →
            // `move_into_char`): LEFT-justified, space-padded on the right when the
            // receiver is wider, truncated on the right when narrower — exactly the
            // path a plain alphanumeric ident source takes. `refmod_string` handles
            // constant AND computed (data-name) indices and an omitted length, and
            // already traps an out-of-range slice identically to the compiled
            // `str_slice`. A NUMERIC receiver (de-editing a slice into a numeric
            // field) stays a later rung, rejected on both engines. The slice is
            // char-based here and byte-based in the compiler; on the ASCII-prefix
            // windows this rung targets they coincide, so output is byte-identical
            // (a non-ASCII char inside/after the window is the pre-existing refmod
            // char-vs-byte chip, shared with DISPLAY/comparison — not introduced
            // here).
            if let Operand::RefMod { base, start, len } = src {
                let alpha_recv = matches!(
                    self.items[idx].picture,
                    Some(Picture::Alphanumeric { .. }) | Some(Picture::Alphabetic { .. })
                );
                if !alpha_recv {
                    return Err(RuntimeError::Unsupported(format!(
                        "MOVE of a reference-modification source into the non-alphanumeric \
                         receiver {dst} is a later rung (an alphanumeric receiver is supported)"
                    )));
                }
                let slice = self.refmod_string(base, start, len)?;
                self.move_into(idx, Src::Chars(slice))?;
                continue;
            }
            // Cross-category numeric → alphanumeric MOVE: COBOL treats a numeric
            // sending item as though it were an alphanumeric item holding its digit
            // characters, then moves it by the alphanumeric rules (via
            // `move_into_char`). The digit image is the full `(int + frac)`-digit
            // magnitude — integer part then fractional part, NO decimal point —
            // which `Decimal::digits()` (equivalently the item's `storage`) already
            // yields (`PIC 9(2)V9 = 4.2` → `"042"`; an integer, `d = 0`, is the
            // special case). An UNSIGNED source (integer or scaled) flows through the
            // generic `move_into` path below (`Src::Num(d) => d.digits()`), giving
            // that plain magnitude image.
            //
            // A SIGNED source (`PIC S9…`) is handled HERE: its image additionally
            // carries the operational sign as a TRAILING OVERPUNCH on the units
            // (last) digit — the same zoned-decimal encoding `item_image`/`DISPLAY`
            // produce for a signed field (`overpunch_trailing`):
            //
            //   | units u  | 0 1 2 3 4 5 6 7 8 9 |
            //   | positive | { A B C D E F G H I |
            //   | negative | } J K L M N O P Q R |
            //
            // So `S9(3) = +123` → `"12C"`, `= −123` → `"12L"`, `S9V9 = −4.2` → `"4K"`.
            // The overpunch is driven by the item being signed, not by the value's
            // sign: a signed POSITIVE value takes the positive `{…I` row (which is
            // exactly why an unsigned `"123"` and a signed positive `"12C"` differ).
            // The overpunched image is then char-moved into the receiver by the same
            // alphanumeric rule (`Src::Chars` → `move_into_char`), so the oracle and
            // the compiler (which builds the identical image via the same table) emit
            // byte-identical bytes. A group source has no `Picture::Numeric`, so it
            // never reaches this arm.
            if let Operand::Ident(name) = src {
                let alpha_recv = matches!(
                    self.items[idx].picture,
                    Some(Picture::Alphanumeric { .. }) | Some(Picture::Alphabetic { .. })
                );
                if alpha_recv {
                    if let Some(&sidx) = self.by_name.get(name) {
                        if let Some(Picture::Numeric { signed: true, .. }) =
                            &self.items[sidx].picture
                        {
                            // Magnitude image (`storage`) with the trailing sign
                            // overpunch, then the ordinary alphanumeric char-move.
                            let image =
                                overpunch_trailing(&self.items[sidx].storage, self.items[sidx].neg);
                            self.move_into(idx, Src::Chars(image))?;
                            continue;
                        }
                    }
                }
            }
            // Cross-category alphanumeric → numeric MOVE (the reverse direction):
            // an alphanumeric source item (`PIC X(m)`) moved into an UNSIGNED
            // numeric receiver `PIC 9(i)V9(d)` (no `S`; `d` may be 0 — an INTEGER —
            // or > 0 — a SCALED receiver). COBOL reads the source's `m` characters
            // as an unsigned integer `V` (fold `V = V*10 + (byte - '0')`), and that
            // folded integer IS the receiver's scaled-slot magnitude directly: it
            // fills the receiver's `(i + d)` digit positions RIGHT-justified, with
            // the implied point `d` places from the right, so the slot is
            // `V mod 10^(i+d)` (left-zero-padded when the source is shorter,
            // high-order-truncated when longer). This is NOT the arithmetic
            // decimal-align rule — `V` is not multiplied by `10^d`; the fold already
            // lands at scale `d`.
            //
            //   MOVE "042"   TO 9(2)V9  → V=42    → slot 042 → reads 4.2
            //   MOVE "12345" TO 9(2)V9  → V=12345 → slot 345 → reads 34.5
            //
            // We fold the `m` bytes into an `i64`, then build a `Decimal` that places
            // the folded magnitude at scale `d` — the point inserted `d` places from
            // the right: `int` = the magnitude's digits above the last `d`
            // (empty → "0"), `frac` = its last `d` digits, left-zero-padded to `d`.
            // `move_into` → `move_into_numeric(int_digits=i, dec_digits=d)` then keeps
            // the low-order `i` integer digits and the high-order `d` fractional
            // digits, i.e. exactly `V mod 10^(i+d)` with the point at `d`. This is
            // byte-identical to the compiler, which folds the identical per-character
            // arithmetic and hands `store_scaled` the SAME scale `d` (a no-op shift,
            // then `mag mod 10^(i+d)`). For `d = 0` the split is `int = V_str`,
            // `frac = ""` — reproducing the old integer-receiver behaviour exactly.
            // ANY numeric receiver (SIGNED or unsigned) and a genuine alphanumeric
            // SOURCE ITEM (not a group, not a literal) are handled here; every other
            // shape falls through to `move_into` below, which rejects a `Src::Chars`
            // → numeric MOVE.
            //
            // A SIGNED receiver (`PIC S9…`) is now handled (guard relaxed from
            // `recv_unsigned` to `recv_numeric`). The WHY: an alphanumeric source
            // carries NO operational sign — COBOL does not read an overpunch from a
            // plain `PIC X` source — so we store the folded MAGNITUDE and its sign is
            // ALWAYS POSITIVE. The body below builds `Decimal { neg: false, .. }` and
            // hands it to `move_into`, which computes the stored sign as
            // `neg = signed && d.neg && …` = `false` (since `d.neg` is false), so a
            // signed receiver correctly stores a POSITIVE value. DISPLAY of that
            // signed field then overpunches the units digit on its POSITIVE row
            // (`{A…I`) via the same `overpunch_trailing`/`item_image` path signed
            // DISPLAY already uses — untouched here. The fold/scale rule is UNCHANGED
            // from the unsigned path.
            if let Operand::Ident(name) = src {
                if let Some(&sidx) = self.by_name.get(name) {
                    let src_is_char = matches!(
                        self.items[sidx].picture,
                        Some(Picture::Alphanumeric { .. }) | Some(Picture::Alphabetic { .. })
                    );
                    let recv_numeric = matches!(
                        self.items[idx].picture,
                        Some(Picture::Numeric { .. })
                    );
                    let recv_dec = match &self.items[idx].picture {
                        Some(Picture::Numeric { dec_digits, .. }) => *dec_digits,
                        _ => 0,
                    };
                    if src_is_char && recv_numeric {
                        let chars = self.items[sidx].storage.clone();
                        // Guard the `i64` fold: an all-digit source of ≤ 18
                        // characters stays below `10^18 < i64::MAX`; a wider source
                        // is a clean later rung (the compiler rejects it identically).
                        if chars.len() > NUMERIC_MAX_DIGITS {
                            return Err(RuntimeError::Unsupported(format!(
                                "alphanumeric → numeric MOVE from {name} into {dst}: a source \
                                 wider than {NUMERIC_MAX_DIGITS} characters is a later rung"
                            )));
                        }
                        // Fold the bytes as decimal digits. `wrapping_*` matches the
                        // compiler's i64 arithmetic and never panics; for the
                        // in-scope all-digit ≤ 18-char source it never wraps.
                        let mut value: i64 = 0;
                        for b in chars.bytes() {
                            value = value.wrapping_mul(10).wrapping_add((b as i64) - (b'0' as i64));
                        }
                        // Take the MAGNITUDE, exactly as the compiler's `store_scaled`
                        // does (it `abs`es the value before `mod 10^(i+d)`). This
                        // matters for a source byte below `'0'` — most commonly a SPACE
                        // (an uninitialised `PIC X` is spaces): `(b - '0')` is then
                        // negative and the fold goes negative, but the source has NO
                        // operational sign, so both engines keep the magnitude and
                        // store it POSITIVE (`Decimal { neg: false }` below) — never a
                        // stray `'-'` — for a signed OR unsigned receiver alike.
                        // A non-digit source is defined-but-unspecified,
                        // identical on both engines by construction. (`unsigned_abs`
                        // is total — no panic on `i64::MIN`, unreachable here anyway.)
                        let mag = value.unsigned_abs().to_string();
                        // Split the magnitude at scale `d`: the point sits `d` places
                        // from the right. Left-pad to at least `d` digits first so the
                        // fractional slice is always exactly `d` chars and the integer
                        // slice is whatever remains ("0" when the magnitude has ≤ `d`
                        // digits). For `d = 0` this is `int = mag`, `frac = ""`.
                        let padded = if mag.len() < recv_dec {
                            format!("{mag:0>recv_dec$}")
                        } else {
                            mag
                        };
                        let split = padded.len() - recv_dec;
                        let int = if split == 0 { "0".to_string() } else { padded[..split].to_string() };
                        let frac = padded[split..].to_string();
                        let decimal = Decimal { neg: false, int, frac };
                        self.move_into(idx, Src::Num(decimal))?;
                        continue;
                    }
                }
            }
            // Resolve the source afresh per receiver (its category can differ).
            let value = self.src_from_operand(src)?;
            self.move_into(idx, value)?;
        }
        Ok(())
    }

    // ----------------------------------------------------------------------
    // Arithmetic (fixed-point decimal, truncating; unsigned receivers)
    // ----------------------------------------------------------------------

    /// `ADD op… TO name [GIVING g] [ROUNDED] [ON SIZE ERROR …]`.
    fn exec_add(
        &mut self,
        operands: &[Operand],
        to: &str,
        giving: &Option<String>,
        rounded: bool,
        on_size_error: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        let mut acc = self.named_decimal(to)?;
        for op in operands {
            acc = checked(add(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_result(giving.as_deref().unwrap_or(to), acc, rounded, on_size_error)
    }

    /// `SUBTRACT op… FROM name [GIVING g] [ROUNDED] [ON SIZE ERROR …]`.
    fn exec_subtract(
        &mut self,
        operands: &[Operand],
        from: &str,
        giving: &Option<String>,
        rounded: bool,
        on_size_error: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        let mut acc = self.named_decimal(from)?;
        for op in operands {
            acc = checked(sub(&acc, &self.operand_decimal(op)?))?;
        }
        self.store_result(giving.as_deref().unwrap_or(from), acc, rounded, on_size_error)
    }

    /// `MULTIPLY a BY b [GIVING g] [ROUNDED] [ON SIZE ERROR …]`.
    fn exec_multiply(
        &mut self,
        a: &Operand,
        by: &Operand,
        giving: &Option<String>,
        rounded: bool,
        on_size_error: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
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
        self.store_result(&target, product, rounded, on_size_error)
    }

    /// `DIVIDE a INTO b [GIVING g] [ROUNDED] [ON SIZE ERROR …]` → b ÷ a.
    fn exec_divide(
        &mut self,
        divisor: &Operand,
        dividend: &Operand,
        giving: &Option<String>,
        rounded: bool,
        on_size_error: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        let d = self.operand_decimal(divisor)?;
        if d.is_zero() {
            // Division by zero is a size-error condition (as in COMPUTE): the
            // handler catches it; without one it is a hard error.
            if on_size_error.is_empty() {
                return Err(RuntimeError::DivideByZero);
            }
            return self.run_stmts(on_size_error);
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
        // Compute at the shared intermediate precision; store_result then
        // rounds or truncates into the receiver's decimal places.
        let quotient = checked(div(&n, &d, COMPUTE_DIV_SCALE))?;
        self.store_result(&target, quotient, rounded, on_size_error)
    }

    /// Store a computed value into a numeric receiver, applying `ROUNDED`
    /// (half away from zero, else truncate toward zero at the receiver's decimal
    /// places) and `ON SIZE ERROR` (when the result's integer part overflows the
    /// receiver, run the handler and leave the receiver unchanged; without a
    /// handler, COBOL truncates the high-order digits silently, as `MOVE` does).
    /// Shared by the arithmetic verbs and `COMPUTE`.
    fn store_result(
        &mut self,
        target: &str,
        value: Decimal,
        rounded: bool,
        on_size_error: &[Stmt],
    ) -> Result<Flow, RuntimeError> {
        let (int_digits, dec_digits) = self.numeric_dims(target)?;
        let final_value = if rounded {
            checked(round(&value, dec_digits))?
        } else {
            value
        };
        // Size error = the integer part does not fit (fractional truncation is
        // never a size error).
        if final_value.int.trim_start_matches('0').len() > int_digits {
            if on_size_error.is_empty() {
                self.store_number(target, final_value)?;
                return Ok(Flow::Normal);
            }
            return self.run_stmts(on_size_error);
        }
        self.store_number(target, final_value)?;
        Ok(Flow::Normal)
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
    ) -> Result<Flow, RuntimeError> {
        let value = match self.eval_expr(expr) {
            Ok(v) => v,
            // Division by zero is a size-error condition: the handler catches it;
            // without one it stays a hard DivideByZero (as bare DIVIDE does).
            Err(RuntimeError::DivideByZero) if !on_size_error.is_empty() => {
                return self.run_stmts(on_size_error);
            }
            Err(e) => return Err(e),
        };
        self.store_result(target, value, rounded, on_size_error)
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
                // A signed field keeps the sign, EXCEPT on zero, which is
                // unsigned (COBOL has no negative zero); an unsigned field drops
                // it to magnitude. Test the STORED magnitude, not the source `d`:
                // a nonzero value can high-order-truncate to an all-zero slot
                // (e.g. `-1000` into `PIC S9(3)` → `000`), and that stored zero
                // must be positive — matching the compiler, whose single-i64 slot
                // collapses such a value to a plain `0`.
                let stored = move_into_numeric(&d, int_digits, dec_digits);
                let neg = signed && d.neg && stored.bytes().any(|b| b != b'0');
                (stored, neg)
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
            // A reference modification always yields the selected characters — an
            // alphanumeric value. In a numeric context this `Src::Chars` is
            // rejected downstream (`operand_decimal`), matching the compiler.
            Operand::RefMod { base, start, len } => {
                Ok(Src::Chars(self.refmod_string(base, start, len)?))
            }
        }
    }

    /// Slice an alphanumeric item for a reference modification `base(start:len)`.
    ///
    /// COBOL reference modification is 1-based: `base(start:len)` selects the
    /// characters at 1-based positions `start .. start+len-1`, i.e. the 0-based
    /// half-open range `[start-1, start-1+len)`. An omitted `len` runs to the end
    /// of the item, so `end = width`. `start` and `len` may each be an integer
    /// literal *or* a data-name read at run time (a **computed** reference
    /// modification) — [`Self::refmod_index_value`] resolves both to `i64`.
    ///
    /// This slices the *same* character range the compiler emits as a `str_slice`
    /// (constant-index for a literal:literal refmod, register-computed for a
    /// computed one), so DISPLAY output and comparisons agree byte-for-byte. The
    /// base must be an alphanumeric item (a numeric item is a later rung).
    ///
    /// **Out-of-range rule.** Let `start0 = start - 1` and
    /// `end = start0 + len` (or `end = width` when the length is omitted). The
    /// slice traps — [`RuntimeError::RefModOutOfRange`] — exactly when
    /// `start0 < 0 || end < start0 || end > width`. This is the *identical*
    /// predicate the compiled `str_slice` enforces in the VM/wasm backends
    /// (`start < 0 || end < start || end > s.len()`), so an in-range program
    /// slices identically on both engines and an out-of-range one errors on both.
    fn refmod_string(
        &self,
        base: &str,
        start: &RefIndex,
        len: &Option<RefIndex>,
    ) -> Result<String, RuntimeError> {
        let idx = *self.by_name.get(base).ok_or_else(|| RuntimeError::UndefinedName(base.to_string()))?;
        let item = &self.items[idx];
        let content = match &item.picture {
            Some(p) if p.is_numeric() => {
                return Err(RuntimeError::Unsupported(
                    "reference modification of a numeric item is a later rung".into(),
                ));
            }
            Some(_) => item.storage.clone(),
            None => self.group_image(idx),
        };
        let chars: Vec<char> = content.chars().collect();
        let width = chars.len() as i128;
        // Compute the half-open [start0, end) bounds in i128 so an adversarial
        // index item (e.g. `PIC 9(18)` holding a value near i64::MAX) can never
        // overflow the `start0 + len` add into a debug-build panic — it is caught
        // by the bounds check below and reported as a clean RefModOutOfRange. The
        // predicate `start0 < 0 || end < start0 || end > width` is the identical
        // rule the emitted run-time `str_slice` enforces on the compiler side.
        let start_v = self.refmod_index_value(start)?;
        let start0 = start_v as i128 - 1;
        let end: i128 = match len {
            Some(l) => start0 + self.refmod_index_value(l)? as i128,
            None => width,
        };
        if start0 < 0 || end < start0 || end > width {
            return Err(RuntimeError::RefModOutOfRange(format!(
                "{base}({start_v}:{}) does not fit the {width}-character item",
                len.as_ref().map(|_| (end - start0).to_string()).unwrap_or_default()
            )));
        }
        // Bounds passed: 0 ≤ start0 ≤ end ≤ width, all within the small char count.
        Ok(chars[start0 as usize..end as usize].iter().collect())
    }

    /// Resolve a reference-modification index ([`RefIndex`]) to an `i64`. A
    /// literal is its own value; a data-name must be an **unsigned integer** item
    /// (`PIC 9…`, no `S`, no `V`) — its stored digits parsed as an integer. A
    /// signed, fractional, or non-numeric index item is a later rung.
    fn refmod_index_value(&self, ix: &RefIndex) -> Result<i64, RuntimeError> {
        match ix {
            RefIndex::Lit(v) => Ok(*v as i64),
            RefIndex::Name(name) => {
                let idx = *self
                    .by_name
                    .get(name)
                    .ok_or_else(|| RuntimeError::UndefinedName(name.clone()))?;
                match &self.items[idx].picture {
                    Some(Picture::Numeric { dec_digits: 0, signed: false, .. }) => {
                        self.items[idx].storage.parse::<i64>().map_err(|_| {
                            RuntimeError::RefModOutOfRange(format!(
                                "index {name} value {} is out of range",
                                self.items[idx].storage
                            ))
                        })
                    }
                    Some(Picture::Numeric { .. }) => Err(RuntimeError::Unsupported(format!(
                        "a signed or fractional reference-modification index ({name}) is a later rung"
                    ))),
                    _ => Err(RuntimeError::Unsupported(format!(
                        "a non-numeric reference-modification index ({name}) is a later rung"
                    ))),
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
            Operand::RefMod { base, start, len } => self.refmod_string(base, start, len),
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

//! Lowering the generic parse tree into a typed program model.
//!
//! The `cobol-parser` CST is a uniform [`GrammarASTNode`] tree (see the parser's
//! probe output). This module walks it once and produces the small typed model
//! the interpreter runs — data definitions and procedure statements — returning
//! a descriptive [`RuntimeError`] for anything v0.1 does not yet handle.

use crate::error::RuntimeError;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

// ---------------------------------------------------------------------------
// Typed model
// ---------------------------------------------------------------------------

/// A whole program: its WORKING-STORAGE definitions and its paragraphs.
#[derive(Debug, Clone)]
pub struct Program {
    pub data: Vec<DataDef>,
    pub paragraphs: Vec<Paragraph>,
}

/// One WORKING-STORAGE entry, as written (the interpreter turns these into the
/// item tree).
#[derive(Debug, Clone)]
pub struct DataDef {
    pub level: u32,
    /// The data-name, or `None` for `FILLER`.
    pub name: Option<String>,
    /// The raw picture string (`"9(3)V99"`), if the entry has a PICTURE clause.
    pub picture: Option<String>,
    /// The `VALUE` clause's items, empty if there is no `VALUE`. A plain item has
    /// exactly one [`ValueSpec::Single`]; a level-88 condition-name may list
    /// several values and `THRU` ranges (`88 OK VALUE 1 THRU 5 9`).
    pub values: Vec<ValueSpec>,
}

/// One item of a `VALUE` clause: a single literal or an inclusive `lo THRU hi`
/// range.
#[derive(Debug, Clone)]
pub enum ValueSpec {
    Single(Lit),
    Range(Lit, Lit),
}

/// A literal or figurative constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit {
    Num(String),
    Str(String),
    Fig(Fig),
}

/// A figurative constant (v0.1 subset).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fig {
    Zero,
    Space,
}

/// A named paragraph of statements.
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// The paragraph name — a `PERFORM` (and, later, `GO TO`) target.
    pub name: String,
    pub stmts: Vec<Stmt>,
}

/// An executable statement (v0.1 subset).
#[derive(Debug, Clone)]
pub enum Stmt {
    Display(Vec<Operand>),
    Move { src: Operand, dsts: Vec<String> },
    /// `ADD op… TO name [GIVING g] [ROUNDED] [ON SIZE ERROR …]` — op1+…+name.
    Add {
        operands: Vec<Operand>,
        to: String,
        giving: Option<String>,
        rounded: bool,
        on_size_error: Vec<Stmt>,
    },
    /// `SUBTRACT op… FROM name [GIVING g] [ROUNDED] [ON SIZE ERROR …]`.
    Subtract {
        operands: Vec<Operand>,
        from: String,
        giving: Option<String>,
        rounded: bool,
        on_size_error: Vec<Stmt>,
    },
    /// `MULTIPLY a BY b [GIVING g] [ROUNDED] [ON SIZE ERROR …]` — a*b.
    Multiply {
        a: Operand,
        by: Operand,
        giving: Option<String>,
        rounded: bool,
        on_size_error: Vec<Stmt>,
    },
    /// `DIVIDE a INTO b [GIVING g] [ROUNDED] [ON SIZE ERROR …]` — b/a.
    Divide {
        divisor: Operand,
        dividend: Operand,
        giving: Option<String>,
        rounded: bool,
        on_size_error: Vec<Stmt>,
    },
    /// `COMPUTE target [ROUNDED] = expr [ON SIZE ERROR stmts…]` — evaluate an
    /// arithmetic expression and store it in `target`, rounding instead of
    /// truncating when `rounded`, running `on_size_error` when the result
    /// overflows the receiver (or a division by zero occurs).
    Compute {
        target: String,
        rounded: bool,
        expr: Expr,
        on_size_error: Vec<Stmt>,
    },
    /// `PERFORM para [THRU para2] <mode>` — run a paragraph (or the range
    /// `para`…`para2` in source order) out of line, then return. The
    /// [`PerformMode`] is the repeat form.
    Perform { target: String, thru: Option<String>, mode: PerformMode },
    /// `GO TO para` — transfer control unconditionally to a paragraph (no return).
    GoTo { target: String },
    /// `IF cond then… [ELSE else…]`.
    If { cond: Cond, then_branch: Vec<Stmt>, else_branch: Vec<Stmt> },
    /// `SET cond-name TO TRUE` — assign the condition-name's conditional variable
    /// the value that makes it hold (the first of its `VALUE` items).
    SetTrue { cond_name: String },
    /// `EVALUATE subject WHEN v… … WHEN OTHER … END-EVALUATE` — COBOL's case
    /// statement. Each branch's `when` is its value-list (`None` = `WHEN OTHER`);
    /// the first branch whose list contains the subject (or the reached `OTHER`)
    /// runs its statements, with no fall-through. A value-list entry is a single
    /// value or an inclusive `THRU` range.
    Evaluate { subject: Operand, branches: Vec<(Option<Vec<WhenValue>>, Vec<Stmt>)> },
    /// `STRING s… DELIMITED BY {SIZE | delim} INTO t` — concatenate each source
    /// left-to-right and store the result into the alphanumeric receiver `t`,
    /// LEFT-JUSTIFIED and truncated at `t`'s width, **without** space-filling the
    /// untouched tail (bytes past what STRING wrote keep their prior content — the
    /// ANSI-85 STRING rule).
    ///
    /// `delim` selects HOW MUCH of each sending field is taken:
    ///
    ///   * `delim = None` (`DELIMITED BY SIZE`) — every field is taken in FULL.
    ///   * `delim = Some(d)` — each field contributes only its PREFIX up to (but
    ///     NOT including) the FIRST occurrence of the single character `d` in that
    ///     field's image; a field with no `d` contributes its whole image, and a
    ///     field that STARTS with `d` contributes the empty string. ONE delimiter
    ///     applies to every field. The delimiter is reduced by the SAME
    ///     `single_delim_char` helper UNSTRING uses (so a multi-char / numeric /
    ///     figurative / reference-modified / wider-item delimiter rejects
    ///     identically), and must be ASCII: the compiler's prefix scan is byte-
    ///     based while this oracle scans by char, so a multi-byte delimiter would
    ///     diverge (a clean later rung on both engines). For the same byte-vs-char
    ///     reason a non-ASCII string-LITERAL sending field is a later rung WHEN a
    ///     delimiter is active (its prefix boundary differs byte-vs-char).
    ///
    /// `pointer` is the optional `WITH POINTER p` phrase: `p` is an unsigned
    /// integer item (`PIC 9(n)`) giving the 1-BASED character position in the
    /// RECEIVER at which the first transferred character is placed, and it is
    /// UPDATED after the operation to `p + chars_placed` (one past the last
    /// character stored). `None` when the phrase is absent (overlay at position 0,
    /// no write-back).
    ///
    /// `ON OVERFLOW` / `NOT ON OVERFLOW` are now MODELLED: `on_overflow` holds the
    /// imperative statement list run when the STRING overflows (the receiver fills
    /// before every sending character is transferred, OR the initial `WITH POINTER`
    /// value is out of range), and `not_on_overflow` holds the list run when it does
    /// NOT. Either list may be empty (clause absent). A multi-character delimiter and
    /// per-field different delimiters remain later rungs (rejected at build time).
    String {
        sources: Vec<Operand>,
        target: String,
        delim: Option<Operand>,
        pointer: Option<String>,
        on_overflow: Vec<Stmt>,
        not_on_overflow: Vec<Stmt>,
    },
    /// `UNSTRING source DELIMITED BY delim INTO r1 [r2 …]` — the inverse of
    /// STRING. Scan the alphanumeric `source` left-to-right, splitting it into
    /// delimited fields on each occurrence of the SINGLE-character `delim` (a
    /// 1-char literal or a `PIC X(1)` item), and move successive fields into
    /// successive receivers `r1..rn` as ordinary alphanumeric MOVEs (left-
    /// justified, space-padded, truncated). Each receiver — INCLUDING the last —
    /// takes the field up to the NEXT delimiter; fields beyond the receiver count
    /// are dropped (that would be `ON OVERFLOW`, a later rung), and once the
    /// source is exhausted the remaining receivers are left UNCHANGED (not
    /// space-filled). `WITH POINTER`, `ON OVERFLOW`, a multi-character or `ALL`/
    /// `OR` delimiter, and a numeric/group source or receiver are later rungs.
    ///
    /// The `source` is an [`Operand`] so the field text can come from either of
    /// two providers with IDENTICAL downstream scanning: an `Operand::Ident`
    /// reads an alphanumeric item's *storage* (as before), while an
    /// `Operand::Lit(Lit::Str(_))` scans the string literal's OWN bytes directly
    /// (`UNSTRING "a,b,c" DELIMITED BY "," INTO w1 w2 w3` → w1="a", w2="b",
    /// w3="c"). Only the source of the characters differs; the delimiter scan and
    /// per-receiver reshape are shared. A NUMERIC or FIGURATIVE literal source and
    /// a reference-modified source remain later rungs (rejected at read time).
    ///
    /// `pointer` is the optional `WITH POINTER p` phrase: `p` is an unsigned
    /// integer item (`PIC 9(n)`) giving the 1-BASED character position in the
    /// source at which the scan STARTS, and it is UPDATED after the operation to
    /// the 1-based position of the character immediately following the last
    /// character examined. `None` when the phrase is absent (start at position 1,
    /// no write-back).
    ///
    /// `ON OVERFLOW` / `NOT ON OVERFLOW` are MODELLED (the DIRECT sibling of the
    /// STRING clauses): `on_overflow` holds the imperative statement list run when
    /// the UNSTRING overflows — all receivers are filled but the source is NOT
    /// exhausted (more delimited fields remain), OR the initial `WITH POINTER` value
    /// is out of range — and `not_on_overflow` holds the list run when it does NOT.
    /// Either list may be empty (clause absent).
    Unstring {
        source: Operand,
        delim: Operand,
        targets: Vec<String>,
        pointer: Option<String>,
        on_overflow: Vec<Stmt>,
        not_on_overflow: Vec<Stmt>,
    },
    /// `INSPECT source TALLYING counter FOR ALL delim` (or `FOR LEADING delim`)
    /// — count occurrences of the SINGLE-character `delim` (a 1-char literal or a
    /// `PIC X(1)` item) in the alphanumeric `source` and **ADD** that count to the
    /// unsigned-integer `counter` (`PIC 9(n)`). `FOR ALL` counts EVERY (non-
    /// overlapping, left-to-right) occurrence; `FOR LEADING` counts only the run of
    /// CONSECUTIVE occurrences at the START of the source, stopping at the first
    /// character that is not `delim` (`leading == true` selects this). INSPECT adds
    /// to the counter; it does NOT clear it first. `CHARACTERS` tallies,
    /// `BEFORE`/`AFTER` phrases, several counters or `FOR` phrases, and a multi-
    /// character/figurative/wider delimiter or a numeric/group source or a
    /// non-integer/non-numeric counter are later rungs. (A `REPLACING` phrase in
    /// the SAME statement is the combined form below, not this lone `Inspect`.)
    ///
    /// An optional `region` restricts the count to a sub-slice of the source — the
    /// `{BEFORE|AFTER} x` phrase (see [`Region`]). This rung wires the region up for
    /// `FOR ALL` only, with a SINGLE-character region delimiter `x`; `FOR LEADING`
    /// with a region, a multi-character region delimiter, and a region on the
    /// combined/`REPLACING`/`CONVERTING` forms are later rungs.
    ///
    /// `FOR CHARACTERS` (`characters == true`) is the "count every position" form:
    /// instead of matching a delimiter, it tallies the NUMBER OF CHARACTER POSITIONS
    /// in the region window. With no region that is `length(source)`; with a region
    /// it is the window length `end - start` (the SAME window `FOR ALL` uses, so it
    /// inherits the BEFORE→whole / AFTER→empty not-found asymmetry). When
    /// `characters == true` neither `delim` nor `leading` is ever consumed, so `delim`
    /// carries a never-read placeholder and `leading` is `false`. Multi-item and
    /// multi-counter `CHARACTERS` remain later rungs (see the multi variants below).
    Inspect {
        source: String,
        counter: String,
        /// The delimiter to match for `FOR ALL`/`FOR LEADING`. UNUSED (and set to a
        /// placeholder) when `characters == true`, because CHARACTERS counts
        /// positions rather than matching a delimiter.
        delim: Operand,
        leading: bool,
        /// `true` for the `FOR CHARACTERS` form — count every position in the window
        /// rather than matching `delim`. Mutually exclusive with `leading`.
        characters: bool,
        region: Option<Region>,
    },
    /// `INSPECT source REPLACING ALL search BY replace` (or `REPLACING LEADING
    /// search BY replace`) — substitute the SINGLE character `search` in the
    /// alphanumeric `source` with the SINGLE character `replace`, in place.
    /// Because both are single characters the source's width is unchanged, so
    /// this is a per-position map. `ALL` replaces EVERY occurrence
    /// (`source := source with each search→replace`); `LEADING` (`leading ==
    /// true`) replaces only the run of CONSECUTIVE `search` characters at the
    /// START of the source, stopping at the first character that is not `search`
    /// — positions after that first gap are left unchanged even if they equal
    /// `search`. Each of `search` and `replace` is a 1-char literal or a `PIC
    /// X(1)` item. `REPLACING CHARACTERS`/`FIRST`, several replace items, and a
    /// multi-character/figurative/wider search or replacement or a numeric/group
    /// source are later rungs. (A `LEADING` replacement inside the combined
    /// `TALLYING … REPLACING` form is now supported — see the combined statement
    /// below.)
    ///
    /// An optional `region` restricts the `ALL` replacement to a sub-slice of the
    /// source — the `{BEFORE|AFTER} x` phrase (see [`Region`], shared with the
    /// TALLYING form). This rung wires the region up for `REPLACING ALL` only, with
    /// a SINGLE-character region delimiter `x`: positions OUTSIDE the window keep
    /// their original character; the window is computed over the ORIGINAL source
    /// with the same leftmost-first-index and BEFORE→whole / AFTER→empty not-found
    /// asymmetry the count window uses. `REPLACING LEADING` with a region, a multi-
    /// character region delimiter, and a region on the combined `TALLYING …
    /// REPLACING` form are later rungs.
    InspectReplacing {
        source: String,
        search: Operand,
        replace: Operand,
        leading: bool,
        region: Option<Region>,
    },
    /// `INSPECT source REPLACING CHARACTERS BY x` — overwrite EVERY character
    /// position of the alphanumeric `source` with the single replacement character
    /// `x`. With NO region this is the "fill the whole field with `x`" form: a field
    /// of byte-length N becomes N copies of `x`, its width UNCHANGED.
    ///
    /// # Byte-basis (the crux — why this variant carries NO region)
    ///
    /// The compiled `cobol-iir-compiler` models storage as a **byte** buffer (COBOL
    /// `PIC X` positions ARE bytes; its `str_len` is a BYTE length). The oracle
    /// models storage as a Rust `String`. "Replace EVERY position" must therefore be
    /// computed on a common basis so both engines agree for ANY source — ASCII and
    /// non-ASCII alike. We pick the BYTE basis: the exec fills
    /// `n = storage.len()` (the field's BYTE length) copies of `x`, then stores the
    /// image through the SAME `move_into` path a `MOVE`/`inspect_replace` uses, which
    /// re-pads/truncates to the picture's fixed CHAR size. Because `x` is a single
    /// ASCII byte, the rebuilt image is `n` ASCII bytes (a valid `String`), and after
    /// `move_into` caps it to the picture's `size` characters the stored image is
    /// exactly `size` copies of `x` — identical to the compiler's `width`-many fill.
    ///
    /// Worked example (non-ASCII source): `PIC X(5) VALUE "café"` stores `"café "`
    /// (padded to 5 CHARS = 6 BYTES). `REPLACING CHARACTERS BY "Z"` fills
    /// `n = 6` copies → `"ZZZZZZ"`, which `move_into` caps to the picture's 5 chars →
    /// `"ZZZZZ"` (5 `Z`s). The compiler builds `width = 5` copies → also `"ZZZZZ"`.
    /// Both engines land on the SAME image byte-for-byte.
    ///
    /// # Deferred: a `{BEFORE|AFTER}` region
    ///
    /// A region window is computed as a BYTE span on the compiler but a CHAR span on
    /// the oracle, and a byte window can split a multi-byte character mid-position —
    /// a state the oracle's `String` storage cannot represent while the compiler's
    /// byte buffer can. Including a region would be unsound (the two engines could
    /// diverge on a non-ASCII source), so `REPLACING CHARACTERS … {BEFORE|AFTER}` is
    /// rejected at read time on BOTH engines and this variant carries no region.
    ///
    /// # Guards (applied IDENTICALLY on both engines — co-total)
    ///
    ///   1. `x` must be a SINGLE character (the shared `single_delim_char` check).
    ///   2. A single-char but NON-ASCII **literal** `x` (e.g. `"é"`, one char / two
    ///      bytes) is a later rung — mirroring the compiler, whose byte-based
    ///      single-char validator rejects it. (A `PIC X(1)` *item* replacement is not
    ///      ASCII-gated: the byte-fill above is co-total for a multi-byte item too.)
    ///   3. A `{BEFORE|AFTER}` region is deferred (see above).
    ///   4. A numeric/group/reference-modified/literal source is a later rung, exactly
    ///      as for every other INSPECT form (`inspect_alnum_source`).
    ///
    /// `REPLACING CHARACTERS` inside a MULTI-item REPLACING list, or inside the
    /// COMBINED `TALLYING … REPLACING` form, remain later rungs — only the SINGLE-item
    /// lone-REPLACING path produces this variant.
    InspectReplacingCharacters { source: String, replace: Operand },
    /// `INSPECT source REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` — one
    /// INSPECT carrying TWO OR MORE replace items in a single REPLACING clause.
    ///
    /// Semantics (ISO): ONE left-to-right pass over the source. At each character
    /// position the items are considered IN WRITTEN ORDER, and the FIRST item whose
    /// (single-char) search matches the current character is applied — the position
    /// then ADVANCES. Two consequences make this more than N independent replaces:
    ///
    ///   * FIRST-MATCH-WINS: only the earliest-written matching item fires at a
    ///     position; later items with the same-position match never see it.
    ///   * NO RE-CHAINING: the character a match produces is NOT re-examined by any
    ///     later item — the output byte is final. So `ALL a BY b ALL b BY z` over
    ///     "ab" yields "bz", NOT "zz": position 0's `a`→`b` is done (the produced
    ///     `b` is not then turned into `z`), and position 1's `b`→`z` fires on the
    ///     ORIGINAL `b`.
    ///
    /// Width is unchanged (each position maps to exactly one output char). Each item
    /// is a SINGLE-char search BY single-char replacement and may be `ALL` OR `LEADING`
    /// (THIS rung lifts the multi-item `LEADING` reject — the count-side twin, a
    /// multi-item TALLYING list with a LEADING item, was lifted in the sibling rung),
    /// each with its OWN optional `{BEFORE|AFTER} x` region. `CHARACTERS`/`FIRST` items
    /// in a multi-item list, and the combined `TALLYING … REPLACING` form with several
    /// items, remain later rungs (see `read_statement`). `items` are in written order —
    /// the exec walks them in that order at every position, which is what realises
    /// first-match-wins.
    ///
    /// PER-ITEM WINDOWS: each item's optional region defines a window over the
    /// ORIGINAL source (via the SAME `region_window` helper the lone/single-item forms
    /// use — BEFORE x → `[0, first_index_of_x)`; AFTER x → `(first_index_of_x, len]`;
    /// not-found asymmetry BEFORE→whole, AFTER→empty). An item with NO region has a
    /// whole-source window (every position inside). At each position the items are
    /// tried IN WRITTEN ORDER and the FIRST item that BOTH (i) has the position inside
    /// its OWN window AND (ii) whose search equals the current ORIGINAL char AND (iii)
    /// — for a `LEADING` item — whose run is still `active` wins; the rest are skipped
    /// for that position. This is the exact composition of multi-item first-match-wins
    /// with the per-item region window and the `LEADING` run machine, always compared
    /// against the ORIGINAL char (no re-chaining).
    ///
    /// LEADING RUN FLAG: the single left-to-right pass carries a per-item `active` flag
    /// (only consulted for `LEADING` items, all init `true`) — IDENTICAL to the tally
    /// side's [`Stmt::InspectTallyMulti`] machine. A `LEADING` item may replace at
    /// position `i` only while `active` is still `true` (every prior IN-WINDOW position
    /// equalled its search). AFTER the replace decision at each position, EVERY
    /// `LEADING` item's run flag is updated INDEPENDENTLY of which item won — its run
    /// breaks at the FIRST in-window position whose char is NOT its search (a matching
    /// char keeps the run alive even if a higher-priority item claimed that position;
    /// positions outside the window neither begin nor break the run). See
    /// [`ReplaceMultiLeadingItem`].
    InspectReplacingMulti { source: String, items: Vec<ReplaceMultiLeadingItem> },
    /// `INSPECT source TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|
    /// AFTER} q] …` — one INSPECT whose SINGLE counter carries TWO OR MORE `FOR ALL`
    /// items, each a single-char delimiter that MAY now carry its OWN optional
    /// `{BEFORE|AFTER} x` window, all folding into the SAME `counter`.
    ///
    /// Semantics (ISO priority-list — the exact count-side analogue of
    /// `InspectReplacingMulti`): ONE left-to-right pass over the source. At each
    /// character position the items are tried IN WRITTEN ORDER and the FIRST item that
    /// BOTH (i) has the position inside its OWN window AND (ii) whose single-char
    /// delimiter equals the current char increments the shared count by 1, then the
    /// scan advances past the match. A position matched by NO item advances with no
    /// increment:
    ///
    /// ```text
    ///   for (i, ch) in source {
    ///       for (delim, start, end) in items {          // written order
    ///           if start <= i < end && ch == delim { count += 1; break }  // first wins
    ///       }
    ///   }
    ///   counter := counter + count                       // INSPECT ADDS; never clears
    /// ```
    ///
    /// PER-ITEM WINDOWS: each item's optional region defines a window over the source
    /// via the SAME `region_window` helper the lone/single-item forms use — BEFORE x →
    /// `[0, first_index_of_x)`; AFTER x → `(first_index_of_x, len]`; the not-found
    /// asymmetry BEFORE→whole, AFTER→empty. A region-less item's window = the whole
    /// source (every position inside). The first-match-per-position `break` is what
    /// makes DUPLICATE items NOT double-count a position: `FOR ALL "a" ALL "a"` over
    /// `"aa"` adds 2 (each `a` counted once by the FIRST item), not 4. Net: `count` is
    /// the number of source positions matched by SOME in-window item, each counted
    /// exactly once. INSPECT adds to the counter; it does not clear it first.
    ///
    /// Each item may be `ALL` OR `LEADING` (each with its own optional region — THIS
    /// rung lifts the multi-item `LEADING` reject); a `CHARACTERS` item in a multi-item
    /// list stays a later rung. SEVERAL counters (more than one `FOR` phrase group) and
    /// the combined `TALLYING … REPLACING` form with several tally items remain later
    /// rungs (see `read_statement`). `items` are in written order — the exec walks them
    /// in that order at every position, which is what realises first-match-per-position
    /// (and thus duplicate-safe) counting.
    ///
    /// A `LEADING` item counts only the CONSECUTIVE run of its delimiter anchored at the
    /// START of its window (a region-less item's window is the whole source, so anchored
    /// at source position 0). The single left-to-right pass carries a per-item `active`
    /// flag (only consulted for `LEADING` items, all init `true`): a `LEADING` item is
    /// eligible to tally at position `i` only while `active` is still `true` (every prior
    /// IN-WINDOW position equalled its delimiter). AFTER the tally decision at each
    /// position, EVERY `LEADING` item's run flag is updated INDEPENDENTLY of which item
    /// tallied — its run breaks at the FIRST in-window position whose char is NOT its
    /// delimiter (a matching char keeps the run alive even if a higher-priority item
    /// claimed that position; positions outside the window neither begin nor break the
    /// run). See [`TallyMultiLeadingItem`].
    InspectTallyMulti { source: String, counter: String, items: Vec<TallyMultiLeadingItem> },
    /// `INSPECT source TALLYING c1 FOR ALL a [{BEFORE|AFTER} p] [ALL b …] c2 FOR ALL d
    /// [{BEFORE|AFTER} q] …` — one INSPECT carrying TWO OR MORE `tally_for` groups, each
    /// with its OWN counter and one-or-more single-char `FOR ALL` delimiters, and each
    /// delimiter item now carrying its OWN optional `{BEFORE|AFTER}` region window.
    /// `groups` holds [`TallyCounterGroup`] pairs `(counter_name, items)` in WRITTEN
    /// ORDER (group 1 first, then group 2, …), and within each group the items — each a
    /// `(delim, Option<Region>)` — are also in written order.
    ///
    /// Semantics (ISO COMBINED priority list ACROSS counters — the crux of this rung):
    /// ALL the delimiters of ALL groups form ONE combined ordered priority list, and
    /// the source is scanned in a SINGLE left-to-right pass. At each character position
    /// the delimiters are tried in WRITTEN ORDER — group 1's items first, then group
    /// 2's, and so on — and the FIRST delimiter that is BOTH inside its own item's
    /// window AND matches the character increments ITS OWN group's counter by 1; the
    /// scan then advances past the match (single-char ⇒ a normal one-position step). A
    /// position matching NO in-window delimiter advances with no increment. Each item's
    /// window is `[start, end)` derived by `region_window` over the source (a region-less
    /// item = the whole source `(0, len)`), applying the ISO not-found asymmetry
    /// (BEFORE→whole, AFTER→empty).
    ///
    /// The decisive consequence of the SINGLE shared pass with first-match-wins: a
    /// character CLAIMED by an earlier group's (in-window) delimiter NEVER reaches a
    /// later group's delimiter. So the groups are NOT independent counts — an earlier
    /// group can "starve" a later one of positions:
    ///
    /// ```text
    ///   "aa"  TALLYING C1 FOR ALL "a"  C2 FOR ALL "a"          -> C1 += 2, C2 += 0
    ///   "ab"  TALLYING C1 FOR ALL "a"  C2 FOR ALL "b"          -> C1 += 1, C2 += 1
    ///   "aba" TALLYING C1 FOR ALL "a" ALL "b"  C2 FOR ALL "a"  -> C1 += 3, C2 += 0
    /// ```
    ///
    /// Each counter ADDS its own share; INSPECT does not clear any counter first. The
    /// SAME counter name may legitimately appear in two groups — then BOTH groups'
    /// matches add to that one item (resolve the counter by name at each add, do not
    /// assume the counters are distinct).
    ///
    /// This rung supports ONLY plain `FOR ALL` single-char delimiters, EACH with an
    /// OPTIONAL `{BEFORE|AFTER}` region (the region reject is LIFTED this rung): NO
    /// `LEADING`/`CHARACTERS` on ANY item of ANY group. Every counter must be an unsigned
    /// integer (`PIC 9(n)`). A group carrying a LEADING/CHARACTERS item is a clean
    /// later-rung `Unsupported`. This variant fires ONLY when there are TWO OR MORE
    /// `tally_for` groups; exactly one group keeps the single-counter paths (`Inspect` /
    /// `InspectTallyMulti`) UNCHANGED, and the combined `TALLYING … REPLACING` form with
    /// several counters stays a later rung.
    InspectTallyCounters { source: String, groups: Vec<TallyCounterGroup> },
    /// `INSPECT source TALLYING counter FOR {ALL|LEADING} delim REPLACING
    /// {ALL|LEADING} search BY replace` — one INSPECT carrying BOTH phrases. Per
    /// ISO this executes "as though an INSPECT TALLYING were specified, followed by an
    /// INSPECT REPLACING": FIRST count occurrences of `delim` in the ORIGINAL
    /// source and **ADD** them to `counter`, THEN replace every `search` with
    /// `replace` in the source. The tally-first ordering matters when
    /// `delim == search`: the count must see the pre-replacement bytes. The
    /// TALLYING half may be `FOR ALL` (count every occurrence) or `FOR LEADING`
    /// (count only the consecutive run of `delim` at the start of the source),
    /// selected by `tally_leading`. The REPLACING half is likewise selected by
    /// `replace_leading`: `ALL` (substitute every `search`) or `LEADING`
    /// (substitute only the consecutive run of `search` at the start of the
    /// source, stopping at the first byte that is not `search`). The two flags
    /// are independent — either, both, or neither half may be LEADING. Each of
    /// `delim`/`search`/`replace` is a single character; the remaining single-form
    /// restrictions as the lone phrases apply (CHARACTERS/FIRST, multiple
    /// counters/FOR/replace items, multi-char/figurative/wider operands, and a
    /// numeric/group source or non-integer counter are later rungs).
    ///
    /// Each half independently carries an optional `{BEFORE|AFTER} x` region (see
    /// [`Region`], shared with the lone TALLYING/REPLACING forms): `tally_region`
    /// narrows the count, `replace_region` narrows the substitution. The two
    /// regions are INDEPENDENT (different kinds, different delimiters, either/both/
    /// neither present). Because tallying does not mutate the source, BOTH windows
    /// are computed over the SAME original source — exactly as the lone forms do.
    /// A single-character region delimiter is supported this rung on `FOR ALL` /
    /// `REPLACING ALL` only; `FOR LEADING`/`REPLACING LEADING` with a region and a
    /// multi-character region delimiter remain later rungs (rejected at read/exec
    /// time by the shared readers, identically to the lone forms).
    InspectTallyReplace {
        source: String,
        counter: String,
        delim: Operand,
        /// `true` for `FOR LEADING` (count only the leading run of `delim`),
        /// `false` for `FOR ALL` (count every occurrence). Applies to the
        /// TALLYING half only.
        tally_leading: bool,
        /// Optional `{BEFORE|AFTER} x` region narrowing the TALLYING half's count,
        /// computed over the original source (see [`Region`]). `None` = whole source.
        tally_region: Option<Region>,
        search: Operand,
        replace: Operand,
        /// `true` for `REPLACING LEADING` (substitute only the leading run of
        /// `search`), `false` for `REPLACING ALL` (substitute every `search`).
        /// Applies to the REPLACING half only.
        replace_leading: bool,
        /// Optional `{BEFORE|AFTER} x` region narrowing the REPLACING half's
        /// substitution, computed over the same original source (see [`Region`]).
        /// `None` = whole source. Independent of `tally_region`.
        replace_region: Option<Region>,
    },
    /// `INSPECT source CONVERTING from TO to` — translate each character of the
    /// alphanumeric `source` through a per-character **translation table** built
    /// from the two EQUAL-length string literals `from` and `to`: a character equal
    /// to `from[k]` becomes `to[k]` (the FIRST such `k` wins if `from` repeats a
    /// character), and a character in no table entry is left unchanged. In place,
    /// same width. `from`/`to` are each a string LITERAL **or** a data-name (`PIC X`
    /// item) — either or both may be an item, mixing freely with a literal on the
    /// other side (carried as a [`ConvertOperand`], resolved at exec time). A
    /// figurative / reference-modified `from`/`to`, an unequal-length pair, and a
    /// numeric/group source (or a numeric/group item AS `from`/`to`) are later rungs.
    ///
    /// A data-name's translation set is the item's CURRENT storage (its declared
    /// width in characters). The equal-length requirement stays: a data-name's length
    /// is its DECLARED WIDTH — known at compile time — so `from`-width must equal
    /// `to`-width, whatever mix of literal/item the two sides are.
    ///
    /// An optional `region` restricts the translation to a sub-slice of the source —
    /// the `{BEFORE|AFTER} x` phrase (see [`Region`], shared with the TALLYING and
    /// REPLACING forms). A SINGLE-character region delimiter `x` is supported this
    /// rung: only positions inside the window are translated; positions OUTSIDE keep
    /// their original character. The window is computed over the ORIGINAL source with
    /// the same leftmost-first-index and BEFORE→whole / AFTER→empty not-found
    /// asymmetry the count/replace windows use. A multi-character region delimiter is
    /// a later rung (rejected at exec time by `single_delim_char`).
    InspectConverting {
        source: String,
        from: ConvertOperand,
        to: ConvertOperand,
        region: Option<Region>,
    },
    StopRun,
}

/// A CONVERTING `from`/`to` operand, carried UNRESOLVED so a data-name reads its
/// translation set from the item's CURRENT storage at exec time.
///
/// A string literal is fixed at parse time, so it is carried already resolved. A
/// data-name (`PIC X` item) is carried by NAME — its set is the item's storage,
/// which the interpreter reads when the CONVERTING executes (loop-invariant: the
/// `from`/`to` item does not change during the translation, so reading it once up
/// front is exactly the oracle's char→char table build). Whether the item is a
/// valid alphanumeric operand (a numeric/group item is a later rung) is checked at
/// exec time, mirroring the compiler's compile-time check.
#[derive(Debug, Clone)]
pub enum ConvertOperand {
    /// A string literal — its own characters ARE the set.
    Literal(String),
    /// A data-name (`PIC X` item) whose CURRENT storage is the set.
    Item(String),
}

/// One item of a `WHEN` value-list: a single value or an inclusive `lo THRU hi`
/// range. Each side is an [`Operand`] (a literal or a data-name), evaluated at
/// match time.
#[derive(Debug, Clone)]
pub enum WhenValue {
    Single(Operand),
    Range(Operand, Operand),
}

/// How a [`Stmt::Perform`] repeats its paragraph.
#[derive(Debug, Clone)]
pub enum PerformMode {
    /// Bare `PERFORM para` — run it once.
    Once,
    /// `PERFORM para n TIMES` — run it a fixed number of times.
    Times(Operand),
    /// `PERFORM para UNTIL cond` — run it while `cond` is false (test before).
    Until(Cond),
    /// `PERFORM para VARYING id FROM start BY step UNTIL cond` — set `id` to
    /// `start`, then run while `cond` is false, stepping `id` by `step` after
    /// each iteration (test before).
    Varying {
        var: String,
        from: Operand,
        by: Operand,
        until: Cond,
    },
}

/// An arithmetic expression tree (the operand of `COMPUTE`). Operator precedence
/// and grouping are already resolved by the grammar's rule cascade, so this is a
/// plain binary tree — no precedence logic lives here.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A numeric literal (its source text, parsed to a value at evaluation).
    Num(String),
    /// A data-name reference (must resolve to a numeric item).
    Var(String),
    /// A unary minus (`neg == true`); unary plus is folded away by the reader.
    Unary { neg: bool, operand: Box<Expr> },
    /// A binary operation `left <op> right`.
    Binary { op: ArithOp, left: Box<Expr>, right: Box<Expr> },
}

/// The binary arithmetic operators COMPUTE understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    /// Exponentiation (`**`), right-associative.
    Pow,
}

/// A statement operand: a data-name, a literal, or a reference modification.
#[derive(Debug, Clone)]
pub enum Operand {
    Ident(String),
    Lit(Lit),
    /// `base(start:len)` / `base(start:)` — a reference modification selecting
    /// `len` characters of alphanumeric item `base` from 1-based position
    /// `start`; an omitted `len` runs to the end of the item. `start` and `len`
    /// are each a [`RefIndex`]: an integer literal *or* a data-name whose value
    /// is read at run time (a **computed** reference modification).
    RefMod { base: String, start: RefIndex, len: Option<RefIndex> },
}

/// Which side of a delimiter an `INSPECT … BEFORE`/`AFTER` region selects.
/// `BEFORE x` restricts work to the source text to the LEFT of the first `x`;
/// `AFTER x` restricts it to the text to the RIGHT of the first `x`.
#[derive(Debug, Clone)]
pub enum RegionKind {
    Before,
    After,
}

/// An `INSPECT … {BEFORE|AFTER} x` region: the phrase that narrows a TALLYING /
/// REPLACING / CONVERTING operation to a sub-slice of the source, bounded by the
/// FIRST occurrence of the single-character delimiter `delim`. This rung wires it
/// up for the lone `TALLYING FOR ALL` form only (see [`Stmt::Inspect`]); the
/// window it implies is computed at exec time over the ORIGINAL source (leftmost
/// occurrence of `delim`), with the ISO not-found asymmetry:
///   * `BEFORE x`, `x` absent → the region is the ENTIRE source; and
///   * `AFTER x`, `x` absent → the region is EMPTY (nothing counted).
#[derive(Debug, Clone)]
pub struct Region {
    pub kind: RegionKind,
    pub delim: Operand,
}

/// One `ALL delim [{BEFORE|AFTER} x]` item of a multi-COUNTER `TALLYING` list: the
/// single-char delimiter operand plus its OWN optional `{BEFORE|AFTER}` region window.
/// The count-side analogue of the `(search, replace, region)` triple a multi-item
/// `REPLACING` item carries; named so [`read_inspect_tally_counters`]'s return type stays
/// legible (and below clippy's type-complexity threshold). The several-COUNTERS path
/// stays `ALL`-only, so this item carries no `leading` flag; the single-counter multi-item
/// path uses [`TallyMultiLeadingItem`], which adds one.
pub type TallyMultiItem = (Operand, Option<Region>);

/// One `{ALL|LEADING} delim [{BEFORE|AFTER} x]` item of a SINGLE-counter multi-item
/// `TALLYING` list: the single-char delimiter operand, a `leading` flag (`true` for a
/// `LEADING` item — count only its run anchored at the window start; `false` for `ALL`),
/// and its OWN optional `{BEFORE|AFTER}` region window. Extends [`TallyMultiItem`] with the
/// `leading` flag this rung lifts into the multi-item list; named so
/// [`read_inspect_tally_multi`]'s return type stays legible (and below clippy's
/// type-complexity threshold).
pub type TallyMultiLeadingItem = (Operand, bool, Option<Region>);

/// One `{ALL|LEADING} search BY replace [{BEFORE|AFTER} x]` item of a multi-item
/// `REPLACING` list: the single-char search operand, the single-char replacement
/// operand, a `leading` flag (`true` for a `LEADING` item — replace only its run
/// anchored at the window start; `false` for `ALL`), and its OWN optional
/// `{BEFORE|AFTER} x` region window. The replace-side twin of
/// [`TallyMultiLeadingItem`]: it extends the older `(search, replace, region)` triple
/// with the `leading` flag THIS rung lifts into the multi-item list, mirroring how the
/// tally side gained its flag. Named so [`read_inspect_replacing_multi`]'s return type
/// stays legible (and below clippy's type-complexity threshold).
pub type ReplaceMultiLeadingItem = (Operand, Operand, bool, Option<Region>);

/// One `counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|AFTER} q] …` group of a
/// MULTI-counter `TALLYING` list: the counter name plus its written-order items, each a
/// [`TallyMultiItem`] (single-char delimiter + its OWN optional region window). The
/// several-counters analogue of a single group's item list; named so
/// [`read_inspect_tally_counters`]'s return type stays legible (and below clippy's
/// type-complexity threshold).
pub type TallyCounterGroup = (String, Vec<TallyMultiItem>);

/// One index (start or length) of a reference modification: a compile-time
/// integer literal, or a data-name whose integer value is the index at run time.
#[derive(Debug, Clone)]
pub enum RefIndex {
    /// A plain integer NUMBER literal — the `2`/`3` in `WS(2:3)`.
    Lit(usize),
    /// A data-name whose unsigned-integer value is the index — the `J`/`K` in
    /// `WS(J:K)`.
    Name(String),
}

/// A condition tested by `IF` (and `PERFORM … UNTIL`). Either a relation between
/// two operands, or a **level-88 condition-name** — a boolean shorthand a data
/// entry declares for "does my parent item hold one of these values?".
#[derive(Debug, Clone)]
pub enum Cond {
    /// `left <relop> right`, optionally negated.
    Relation {
        left: Operand,
        op: RelOp,
        negated: bool,
        right: Operand,
    },
    /// A bare condition-name (`IF IS-OK`). The interpreter resolves it against the
    /// level-88 entries it collected while building the data model.
    ConditionName(String),
    /// `c1 AND c2 AND …` — true when *every* part holds. Held as a flat list (not
    /// a nested tree) so a long `AND` chain evaluates by iteration, never by
    /// recursion — a crafted `A AND A AND … (thousands)` cannot overflow the stack.
    And(Vec<Cond>),
    /// `c1 OR c2 OR …` — true when *any* part holds. Flat, as [`Cond::And`].
    Or(Vec<Cond>),
    /// `NOT c` — true when `c` does not hold.
    Not(Box<Cond>),
}

/// The relational operator of a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelOp {
    Greater,
    Less,
    Equal,
}

// ---------------------------------------------------------------------------
// CST navigation helpers
// ---------------------------------------------------------------------------

/// Direct child nodes with the given rule name.
fn child_nodes<'a>(n: &'a GrammarASTNode, rule: &str) -> Vec<&'a GrammarASTNode> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(x) if x.rule_name == rule => Some(x),
            _ => None,
        })
        .collect()
}

/// First direct child node with the given rule name.
fn child_node<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(n, rule).into_iter().next()
}

/// First descendant node with the given rule name (depth-first) — for locating
/// divisions/sections anywhere under the program root.
fn find<'a>(n: &'a GrammarASTNode, rule: &str) -> Option<&'a GrammarASTNode> {
    if n.rule_name == rule {
        return Some(n);
    }
    for c in &n.children {
        if let ASTNodeOrToken::Node(x) = c {
            if let Some(f) = find(x, rule) {
                return Some(f);
            }
        }
    }
    None
}

/// Direct child tokens' (effective type name, value) pairs.
fn child_tokens(n: &GrammarASTNode) -> Vec<(String, String)> {
    n.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some((t.effective_type_name().to_string(), t.value.clone())),
            _ => None,
        })
        .collect()
}

/// The value of the first direct child token of the given effective type.
fn first_token(n: &GrammarASTNode, type_name: &str) -> Option<String> {
    child_tokens(n).into_iter().find(|(k, _)| k == type_name).map(|(_, v)| v)
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Lower a parsed program CST into the typed [`Program`] model.
pub fn read_program(root: &GrammarASTNode) -> Result<Program, RuntimeError> {
    let data = match find(root, "data_division") {
        Some(dd) => read_working_storage(dd)?,
        None => Vec::new(),
    };
    let paragraphs = match find(root, "procedure_division") {
        Some(pd) => read_procedure(pd)?,
        None => Vec::new(),
    };
    Ok(Program { data, paragraphs })
}

fn read_working_storage(dd: &GrammarASTNode) -> Result<Vec<DataDef>, RuntimeError> {
    let ws = match find(dd, "working_storage_section") {
        Some(ws) => ws,
        None => return Ok(Vec::new()),
    };
    child_nodes(ws, "data_entry").into_iter().map(read_data_entry).collect()
}

fn read_data_entry(e: &GrammarASTNode) -> Result<DataDef, RuntimeError> {
    let level = first_token(e, "NUMBER")
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| RuntimeError::Unsupported("data entry without a level number".into()))?;

    // Name: the NAME token, or `None` for FILLER (unnamed) entries.
    let name = first_token(e, "NAME");

    // Clauses: each `data_clause` wraps a `picture_clause` or `value_clause`.
    let mut picture = None;
    let mut values = Vec::new();
    for clause in child_nodes(e, "data_clause") {
        if let Some(pc) = child_node(clause, "picture_clause") {
            picture = first_token(pc, "PIC_STRING");
        } else if let Some(vc) = child_node(clause, "value_clause") {
            for item in child_nodes(vc, "value_item") {
                values.push(read_value_item(item)?);
            }
        }
    }

    Ok(DataDef { level, name, picture, values })
}

/// Read one `value_item` (`literal [ (THRU|THROUGH) literal ]`) into a
/// [`ValueSpec`]: two literals form an inclusive range, one a single value.
fn read_value_item(item: &GrammarASTNode) -> Result<ValueSpec, RuntimeError> {
    let lits = child_nodes(item, "literal");
    match lits.as_slice() {
        [one] => Ok(ValueSpec::Single(read_literal(one)?)),
        [lo, hi] => Ok(ValueSpec::Range(read_literal(lo)?, read_literal(hi)?)),
        _ => Err(RuntimeError::Unsupported("a VALUE item must be `literal` or `literal THRU literal`".into())),
    }
}

fn read_literal(lit: &GrammarASTNode) -> Result<Lit, RuntimeError> {
    if let Some(fig) = child_node(lit, "figurative") {
        // The figurative's token value is the uppercased word.
        let word = child_tokens(fig).into_iter().map(|(_, v)| v).next().unwrap_or_default();
        return match word.as_str() {
            "ZERO" | "ZEROS" | "ZEROES" => Ok(Lit::Fig(Fig::Zero)),
            "SPACE" | "SPACES" => Ok(Lit::Fig(Fig::Space)),
            other => Err(RuntimeError::Unsupported(format!("figurative constant {other}"))),
        };
    }
    for (kind, val) in child_tokens(lit) {
        match kind.as_str() {
            "NUMBER" => return Ok(Lit::Num(val)),
            "STRING" => return Ok(Lit::Str(val)),
            _ => {}
        }
    }
    Err(RuntimeError::Unsupported("unrecognised literal".into()))
}

fn read_operand(op: &GrammarASTNode) -> Result<Operand, RuntimeError> {
    if let Some(lit) = child_node(op, "literal") {
        return Ok(Operand::Lit(read_literal(lit)?));
    }
    if let Some(name) = first_token(op, "NAME") {
        // A reference-modification suffix appears as nested `operand` child nodes
        // (the start, and optionally the length). A bare NAME has none.
        let inner = child_nodes(op, "operand");
        if !inner.is_empty() {
            let start = read_refmod_index(inner[0])?;
            let len = match inner.get(1) {
                Some(l) => Some(read_refmod_index(l)?),
                None => None,
            };
            return Ok(Operand::RefMod { base: name, start, len });
        }
        return Ok(Operand::Ident(name));
    }
    Err(RuntimeError::Unsupported("unrecognised operand".into()))
}

/// Read a reference-modification start or length subnode into a [`RefIndex`]:
/// a plain integer NUMBER literal becomes [`RefIndex::Lit`]; a bare data-name
/// becomes [`RefIndex::Name`] (a *computed* index resolved at run time). Any
/// other form — a signed/fractional literal, a figurative, or a nested
/// reference modification as the index — is a later rung.
fn read_refmod_index(op: &GrammarASTNode) -> Result<RefIndex, RuntimeError> {
    let unsupported = |m: &str| RuntimeError::Unsupported(m.into());
    if child_node(op, "literal").is_none() {
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
        Lit::Num(s) => s
            .parse::<usize>()
            .map(RefIndex::Lit)
            .map_err(|_| unsupported("a signed or fractional reference-modification index is a later rung")),
        _ => Err(unsupported(
            "a non-integer reference-modification index is a later rung",
        )),
    }
}

/// Read one `when_value` (`operand [ (THRU|THROUGH) operand ]`) into a
/// [`WhenValue`]: two operands form an inclusive range, one a single value.
fn read_when_value(wv: &GrammarASTNode) -> Result<WhenValue, RuntimeError> {
    let ops = child_nodes(wv, "operand");
    match ops.as_slice() {
        [one] => Ok(WhenValue::Single(read_operand(one)?)),
        [lo, hi] => Ok(WhenValue::Range(read_operand(lo)?, read_operand(hi)?)),
        _ => Err(RuntimeError::Unsupported("a WHEN value must be `operand` or `operand THRU operand`".into())),
    }
}

fn read_procedure(pd: &GrammarASTNode) -> Result<Vec<Paragraph>, RuntimeError> {
    let mut paragraphs = Vec::new();
    for para in child_nodes(pd, "paragraph") {
        let name = first_token(para, "NAME").unwrap_or_default();
        let mut stmts = Vec::new();
        for sentence in child_nodes(para, "sentence") {
            for stmt in child_nodes(sentence, "statement") {
                stmts.push(read_statement(stmt)?);
            }
        }
        paragraphs.push(Paragraph { name, stmts });
    }
    Ok(paragraphs)
}

fn read_statement(stmt: &GrammarASTNode) -> Result<Stmt, RuntimeError> {
    // A `statement` wraps exactly one verb node.
    let verb = stmt
        .children
        .iter()
        .find_map(|c| match c {
            ASTNodeOrToken::Node(x) => Some(x),
            _ => None,
        })
        .ok_or_else(|| RuntimeError::Unsupported("empty statement".into()))?;

    match verb.rule_name.as_str() {
        "display_stmt" => {
            let ops = child_nodes(verb, "operand")
                .into_iter()
                .map(read_operand)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Stmt::Display(ops))
        }
        "move_stmt" => {
            let src_node = child_node(verb, "operand")
                .ok_or_else(|| RuntimeError::Unsupported("MOVE without a source".into()))?;
            let src = read_operand(src_node)?;
            // Receiving fields are the NAME tokens after TO.
            let dsts: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            if dsts.is_empty() {
                return Err(RuntimeError::Unsupported("MOVE without a receiver".into()));
            }
            Ok(Stmt::Move { src, dsts })
        }
        "stop_stmt" => {
            // STOP RUN vs STOP <literal>.
            let has_run = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "RUN");
            if has_run {
                Ok(Stmt::StopRun)
            } else {
                Err(RuntimeError::Unsupported("STOP <literal> (only STOP RUN in v0.1)".into()))
            }
        }
        "add_stmt" => {
            // ADD op… TO name [GIVING g]: operands are `operand` nodes; the
            // direct NAME tokens are [to] or [to, giving].
            let operands = read_operands(verb)?;
            let (to, giving) = read_target_and_giving(verb)?;
            let (rounded, on_size_error) = read_rounded_and_size_error(verb)?;
            Ok(Stmt::Add { operands, to, giving, rounded, on_size_error })
        }
        "subtract_stmt" => {
            let operands = read_operands(verb)?;
            let (from, giving) = read_target_and_giving(verb)?;
            let (rounded, on_size_error) = read_rounded_and_size_error(verb)?;
            Ok(Stmt::Subtract { operands, from, giving, rounded, on_size_error })
        }
        "multiply_stmt" => {
            // MULTIPLY a BY b [GIVING g]: two operand nodes; a direct NAME token
            // only when GIVING is present.
            let ops = read_operands(verb)?;
            if ops.len() != 2 {
                return Err(RuntimeError::Unsupported("MULTIPLY needs exactly two operands".into()));
            }
            let has_giving = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "GIVING");
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let giving = if has_giving { names.into_iter().next() } else { None };
            let (rounded, on_size_error) = read_rounded_and_size_error(verb)?;
            let mut it = ops.into_iter();
            Ok(Stmt::Multiply {
                a: it.next().unwrap(),
                by: it.next().unwrap(),
                giving,
                rounded,
                on_size_error,
            })
        }
        "divide_stmt" => {
            // DIVIDE a INTO b [GIVING g]: first operand is the divisor, second
            // the dividend; result = b / a.
            let ops = read_operands(verb)?;
            if ops.len() != 2 {
                return Err(RuntimeError::Unsupported("DIVIDE needs exactly two operands".into()));
            }
            let has_giving = child_tokens(verb).iter().any(|(k, v)| k == "KEYWORD" && v == "GIVING");
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let giving = if has_giving { names.into_iter().next() } else { None };
            let (rounded, on_size_error) = read_rounded_and_size_error(verb)?;
            let mut it = ops.into_iter();
            Ok(Stmt::Divide {
                divisor: it.next().unwrap(),
                dividend: it.next().unwrap(),
                giving,
                rounded,
                on_size_error,
            })
        }
        "compute_stmt" => {
            // COMPUTE target [ROUNDED] = <expr> [ON SIZE ERROR stmts…].
            // The one direct NAME token is the receiver; expression names live
            // deeper, inside the arith_* nodes.
            let target = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("COMPUTE without a receiver".into()))?;
            let expr_node = child_node(verb, "arith_expr")
                .ok_or_else(|| RuntimeError::Unsupported("COMPUTE without an expression".into()))?;
            let expr = read_arith_expr_bounded(expr_node)?;
            let (rounded, on_size_error) = read_rounded_and_size_error(verb)?;
            Ok(Stmt::Compute { target, rounded, expr, on_size_error })
        }
        "perform_stmt" => {
            // PERFORM target [THROUGH/THRU target2] [ operand TIMES | UNTIL … |
            // VARYING … ]. The direct NAME tokens are [target] or, with THRU,
            // [target, target2]; the induction/TIMES/UNTIL operands live inside
            // their own child nodes.
            let names: Vec<String> = child_tokens(verb)
                .into_iter()
                .filter(|(k, _)| k == "NAME")
                .map(|(_, v)| v)
                .collect();
            let target = names
                .first()
                .cloned()
                .ok_or_else(|| RuntimeError::Unsupported("PERFORM without a target paragraph".into()))?;
            let has_thru = child_tokens(verb)
                .iter()
                .any(|(k, v)| k == "KEYWORD" && (v == "THRU" || v == "THROUGH"));
            let thru = if has_thru { names.get(1).cloned() } else { None };
            // The repeat mode: VARYING (its own node), else TIMES (a direct
            // operand), else UNTIL (a direct condition), else bare/once.
            let mode = if let Some(v) = child_node(verb, "perform_varying") {
                read_perform_varying(v)?
            } else if let Some(op) = child_node(verb, "operand") {
                PerformMode::Times(read_operand(op)?)
            } else if let Some(cond) = child_node(verb, "condition") {
                PerformMode::Until(read_condition(cond)?)
            } else {
                PerformMode::Once
            };
            Ok(Stmt::Perform { target, thru, mode })
        }
        "goto_stmt" => {
            // GO [TO] target. The DEPENDING ON form is not in the grammar yet.
            let target = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("GO TO without a target paragraph".into()))?;
            Ok(Stmt::GoTo { target })
        }
        "set_stmt" => {
            // SET cond-name TO TRUE.
            let cond_name = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("SET without a condition-name".into()))?;
            Ok(Stmt::SetTrue { cond_name })
        }
        "evaluate_stmt" => {
            // EVALUATE subject { WHEN (OTHER|value) stmt… } END-EVALUATE. The
            // subject operand is a direct child; each WHEN value is nested under a
            // `when_branch` node, so they don't collide.
            let subject_node = child_node(verb, "operand")
                .ok_or_else(|| RuntimeError::Unsupported("EVALUATE without a subject".into()))?;
            let subject = read_operand(subject_node)?;
            let mut branches = Vec::new();
            for wb in child_nodes(verb, "when_branch") {
                let is_other = child_tokens(wb).iter().any(|(k, v)| k == "KEYWORD" && v == "OTHER");
                let when = if is_other {
                    None
                } else {
                    let mut values = Vec::new();
                    for wv in child_nodes(wb, "when_value") {
                        values.push(read_when_value(wv)?);
                    }
                    Some(values)
                };
                let stmts = child_nodes(wb, "statement")
                    .into_iter()
                    .map(read_statement)
                    .collect::<Result<Vec<_>, _>>()?;
                branches.push((when, stmts));
            }
            Ok(Stmt::Evaluate { subject, branches })
        }
        "if_stmt" => {
            // Children in order: IF, condition, then-statements…, [ELSE,
            // else-statements…]. Split the statement nodes at the ELSE keyword.
            let cond_node = child_node(verb, "condition")
                .ok_or_else(|| RuntimeError::Unsupported("IF without a condition".into()))?;
            let cond = read_condition(cond_node)?;
            let mut then_branch = Vec::new();
            let mut else_branch = Vec::new();
            let mut seen_else = false;
            for child in &verb.children {
                match child {
                    ASTNodeOrToken::Token(t) if t.value == "ELSE" && t.effective_type_name() == "KEYWORD" => {
                        seen_else = true;
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                        let stmt = read_statement(n)?;
                        if seen_else {
                            else_branch.push(stmt);
                        } else {
                            then_branch.push(stmt);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Stmt::If { cond, then_branch, else_branch })
        }
        "string_stmt" => {
            // STRING s… DELIMITED BY {SIZE | delim} INTO t [WITH POINTER p]
            //        [ON OVERFLOW imp…] [NOT ON OVERFLOW imp…]. Both `WITH POINTER`
            // and the two OVERFLOW imperatives are now MODELLED (see the extraction
            // below), so nothing is rejected here.
            let toks = child_tokens(verb);
            // The delimiter is `SIZE` or a general operand. `DELIMITED BY SIZE`
            // takes each field in full (`delim = None`); a real single-character
            // delimiter (`delim = Some(op)`) truncates each field at its first
            // occurrence. The `string_delim` grammar node parses both, so a real
            // delimiter is NOT rejected here any more — it is read and its ASCII /
            // single-character legality is enforced at exec time (via the same
            // `single_delim_char` helper UNSTRING uses), keeping the oracle and the
            // byte-based compiler co-total.
            let delim_node = child_node(verb, "string_delim")
                .ok_or_else(|| RuntimeError::Unsupported("STRING without DELIMITED BY".into()))?;
            let is_size =
                child_tokens(delim_node).iter().any(|(k, v)| k == "KEYWORD" && v == "SIZE");
            let delim = if is_size {
                None
            } else {
                // The delimiter operand is nested UNDER `string_delim` (so it never
                // collides with the sending-field `operand` children of `verb`).
                let dop = child_node(delim_node, "operand").ok_or_else(|| {
                    RuntimeError::Unsupported("STRING DELIMITED BY without a delimiter".into())
                })?;
                Some(read_operand(dop)?)
            };
            // The sending fields are the DIRECT `operand` children (the delimiter
            // operand is a grandchild under `string_delim`, so it does not collide).
            let sources = child_nodes(verb, "operand")
                .into_iter()
                .map(read_operand)
                .collect::<Result<Vec<_>, _>>()?;
            if sources.is_empty() {
                return Err(RuntimeError::Unsupported("STRING without a sending field".into()));
            }
            // The receiver is the first NAME token: `INTO t` always precedes a
            // `WITH POINTER p` phrase, so the first direct NAME is the receiver and
            // the pointer NAME (if any) is the first NAME AFTER the `POINTER`
            // keyword. (Sending-field identifiers are nested under `operand` nodes,
            // not direct NAME tokens, so they never appear in this flat run.)
            let target = first_token(verb, "NAME")
                .ok_or_else(|| RuntimeError::Unsupported("STRING without an INTO receiver".into()))?;
            let ptr_pos = toks.iter().position(|(k, v)| k == "KEYWORD" && v == "POINTER");
            let pointer: Option<String> = ptr_pos.and_then(|pp| {
                toks[pp + 1..].iter().find(|(k, _)| k == "NAME").map(|(_, v)| v.clone())
            });
            // The optional imperatives are direct `statement` child nodes of the
            // `string_stmt`, appearing ONLY after the `ON OVERFLOW` / `NOT ON
            // OVERFLOW` keywords (the grammar emits the keywords as leaf tokens and
            // the imperatives as `statement` siblings — mirror the `if_stmt` reader
            // above, which splits its then/else statement children at `ELSE`).
            //
            //   STRING … ON OVERFLOW  MOVE 1 TO F   NOT ON OVERFLOW  MOVE 0 TO F
            //                         └─ on_overflow ┘  ▲NOT flips   └ not_on_overflow ┘
            //
            // A nested statement's OWN `NOT` (e.g. `IF A NOT = B …`) is buried inside
            // that `statement` node, never a direct token child of `string_stmt`, so
            // the split cannot be fooled. Once `seen_not` flips, every subsequent
            // `statement` belongs to NOT ON OVERFLOW.
            let mut on_overflow = Vec::new();
            let mut not_on_overflow = Vec::new();
            let mut seen_not = false;
            for child in &verb.children {
                match child {
                    ASTNodeOrToken::Token(t)
                        if t.value == "NOT" && t.effective_type_name() == "KEYWORD" =>
                    {
                        seen_not = true;
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                        let stmt = read_statement(n)?;
                        if seen_not {
                            not_on_overflow.push(stmt);
                        } else {
                            on_overflow.push(stmt);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Stmt::String { sources, target, delim, pointer, on_overflow, not_on_overflow })
        }
        "unstring_stmt" => {
            // UNSTRING source DELIMITED BY delim INTO r1 [r2 …] [WITH POINTER p]
            //          [ON OVERFLOW imp…] [NOT ON OVERFLOW imp…]. Both `WITH POINTER`
            // and the two OVERFLOW imperatives are now MODELLED (see the extraction
            // below), so nothing is rejected here — the DIRECT sibling of the STRING
            // arm above.
            let toks = child_tokens(verb);
            // The two direct `operand` children are the source and the delimiter,
            // in order (a reference-modification suffix nests *under* an operand,
            // so it never appears as a third top-level operand).
            let ops = child_nodes(verb, "operand");
            let (source_op, delim_op) = match ops.as_slice() {
                [s, d] => (s, d),
                _ => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING needs a source and a DELIMITED BY delimiter".into(),
                    ))
                }
            };
            // The source is a plain data-name (a PIC X item, checked at exec
            // time), an alphanumeric STRING literal (its own bytes supply the field
            // text), OR a reference-modified item slice `base(start:len)` (its
            // sliced characters supply the field text — the numeric-base and
            // out-of-range checks live in the shared `refmod_string` at exec time).
            // A NUMERIC or FIGURATIVE literal source is still a later rung. We keep
            // the whole `Operand` so exec time can pick the provider.
            let source = match read_operand(source_op)? {
                src @ Operand::Ident(_) => src,
                // Reference-modified source: accepted here; the slice bounds and
                // numeric-base reject are enforced by `refmod_string` at exec time,
                // identically to the compiler's `ref_mod_slice`.
                src @ Operand::RefMod { .. } => src,
                // A string-literal source is scanned by CHARACTER here but the
                // compiler lowers it to BYTE-based IIR string ops; the two agree
                // only for ASCII (one byte per char). A non-ASCII literal source
                // is therefore a clean later rung on BOTH engines (kept co-total).
                Operand::Lit(Lit::Str(s)) if !s.is_ascii() => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING of a non-ASCII literal source is a later rung".into(),
                    ))
                }
                src @ Operand::Lit(Lit::Str(_)) => src,
                Operand::Lit(Lit::Num(_)) => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING of a numeric-literal source is a later rung".into(),
                    ))
                }
                Operand::Lit(Lit::Fig(_)) => {
                    return Err(RuntimeError::Unsupported(
                        "UNSTRING of a figurative-constant source is a later rung".into(),
                    ))
                }
            };
            let delim = read_operand(delim_op)?;
            // The grammar is flat — `INTO NAME { NAME } [ WITH POINTER NAME ]` —
            // so the child tokens appear in source order: every receiver NAME,
            // then (optionally) the `POINTER` keyword, then the pointer NAME. We
            // therefore split the NAME tokens at the `POINTER` keyword's position:
            // NAMEs BEFORE it are the INTO receivers; the first NAME AFTER it is
            // the pointer. (Taking "the last NAME" blindly would misread a
            // single-receiver `INTO r WITH POINTER p` as two receivers.)
            let ptr_pos = toks.iter().position(|(k, v)| k == "KEYWORD" && v == "POINTER");
            let pointer: Option<String> = ptr_pos.and_then(|pp| {
                toks[pp + 1..].iter().find(|(k, _)| k == "NAME").map(|(_, v)| v.clone())
            });
            let targets: Vec<String> = toks
                .iter()
                .enumerate()
                .filter(|(i, (k, _))| k == "NAME" && ptr_pos.is_none_or(|pp| *i < pp))
                .map(|(_, (_, v))| v.clone())
                .collect();
            if targets.is_empty() {
                return Err(RuntimeError::Unsupported(
                    "UNSTRING without an INTO receiver".into(),
                ));
            }
            // The optional imperatives are direct `statement` child nodes of the
            // `unstring_stmt`, appearing ONLY after the `ON OVERFLOW` / `NOT ON
            // OVERFLOW` keywords (the receiver/pointer NAMEs are direct token
            // children — never `statement` nodes — so the split below never sees a
            // receiver). Mirror the `string_stmt` reader above and the `if_stmt`
            // then/else split at `ELSE`:
            //
            //   UNSTRING … ON OVERFLOW  MOVE 1 TO F   NOT ON OVERFLOW  MOVE 0 TO F
            //                           └ on_overflow ┘  ▲NOT flips    └ not_on_overflow ┘
            //
            // A nested statement's OWN `NOT` (e.g. `IF A NOT = B …`) is buried inside
            // that `statement` node, never a direct token child of `unstring_stmt`,
            // so the split cannot be fooled. Once `seen_not` flips, every subsequent
            // `statement` belongs to NOT ON OVERFLOW.
            let mut on_overflow = Vec::new();
            let mut not_on_overflow = Vec::new();
            let mut seen_not = false;
            for child in &verb.children {
                match child {
                    ASTNodeOrToken::Token(t)
                        if t.value == "NOT" && t.effective_type_name() == "KEYWORD" =>
                    {
                        seen_not = true;
                    }
                    ASTNodeOrToken::Node(n) if n.rule_name == "statement" => {
                        let stmt = read_statement(n)?;
                        if seen_not {
                            not_on_overflow.push(stmt);
                        } else {
                            on_overflow.push(stmt);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Stmt::Unstring { source, delim, targets, pointer, on_overflow, not_on_overflow })
        }
        "inspect_stmt" => {
            // The grammar accepts the full INSPECT surface — a TALLYING clause, a
            // REPLACING clause, or both together (LEADING/CHARACTERS, BEFORE/AFTER
            // regions, several counters/replace items, …) — so the forms this rung
            // does not model reject as a friendly Unsupported here, not a parse
            // error. This rung supports a LONE `TALLYING … FOR ALL`, a LONE
            // `REPLACING ALL … BY …`, or the COMBINED `TALLYING … REPLACING` in one
            // INSPECT (which the standard executes as tally-then-replace).
            let has_tally = child_node(verb, "inspect_tallying").is_some();
            let has_repl = child_node(verb, "inspect_replacing").is_some();
            let has_conv = child_node(verb, "inspect_converting").is_some();
            // The source is the first (and only top-level) `operand`; a literal or
            // reference-modified source is a later rung (its category is checked at
            // exec time). Shared by both the TALLYING and REPLACING forms.
            let source_node = child_node(verb, "operand")
                .ok_or_else(|| RuntimeError::Unsupported("INSPECT without a source".into()))?;
            let source = match read_operand(source_node)? {
                Operand::Ident(name) => name,
                Operand::Lit(_) => {
                    return Err(RuntimeError::Unsupported(
                        "INSPECT of a literal source is a later rung".into(),
                    ))
                }
                Operand::RefMod { .. } => {
                    return Err(RuntimeError::Unsupported(
                        "INSPECT of a reference-modified source is a later rung".into(),
                    ))
                }
            };
            // CONVERTING is a STANDALONE alternative — the grammar never lets it
            // appear beside TALLYING/REPLACING — so it is handled on its own before
            // the tally/replace composition.
            if has_conv {
                let (from, to, region) = read_inspect_converting(verb)?;
                return Ok(Stmt::InspectConverting { source, from, to, region });
            }
            match (has_tally, has_repl) {
                // Combined: tally-then-replace in one statement. Both phrases are
                // extracted here (each rejecting its own later-rung forms) and the
                // ordering is enforced at exec time.
                (true, true) => {
                    // Each half independently parses its OWN optional `{BEFORE|AFTER}
                    // x` region from its own phrase child (the TALLYING half's region
                    // rides on `inspect_tallying`, the REPLACING half's on
                    // `inspect_replacing`). The shared readers now ACCEPT a LEADING half
                    // carrying a region (the STANDALONE `FOR LEADING …`/`REPLACING
                    // LEADING … BEFORE/AFTER` forms are supported this rung), so the
                    // combined form re-imposes the deferral itself just below: a
                    // combined LEADING half PLUS a region is still a later rung. (A
                    // combined `FOR ALL`/`ALL` half WITH a region, and a LEADING half
                    // WITHOUT one, both remain supported.)
                    let (counter, delim, leading, tally_characters, tally_region) =
                        read_inspect_tally_all(verb)?;
                    // The shared reader now ACCEPTS `FOR CHARACTERS` (standalone), but the
                    // COMBINED `TALLYING … REPLACING` form does not support a CHARACTERS
                    // tally this rung. Re-impose the deferral here, mirroring the LEADING+
                    // region deferral below, so the combined form stays a later rung on
                    // both engines identically.
                    if tally_characters {
                        return Err(RuntimeError::Unsupported(
                            "INSPECT TALLYING … FOR CHARACTERS in a combined TALLYING/REPLACING is a later rung"
                                .into(),
                        ));
                    }
                    // The combined form's TALLYING half supports BOTH `FOR ALL`
                    // and `FOR LEADING`: `leading` selects the count semantics
                    // (LEADING counts only the consecutive run of `delim` at the
                    // start of the source). It rides along into the statement.
                    let (search, replace, repl_leading, replace_region) =
                        read_inspect_replacing_all(verb)?;
                    // The combined form's REPLACING half supports BOTH `ALL`
                    // and `LEADING`: `repl_leading` selects the substitution
                    // semantics (LEADING rewrites only the consecutive run of
                    // `search` at the start of the source). It rides along into
                    // the statement, independent of the TALLYING half's `leading`.
                    //
                    // Re-impose the combined-form deferral the shared readers no longer
                    // enforce: a LEADING half carrying a `{BEFORE|AFTER}` region is a
                    // later rung ONLY in the combined form. The exact messages match
                    // the standalone rejects these readers used to raise, so both
                    // engines and both forms diagnose it identically.
                    if leading && tally_region.is_some() {
                        return Err(RuntimeError::Unsupported(
                            "INSPECT TALLYING … FOR LEADING with a BEFORE/AFTER region is a later rung"
                                .into(),
                        ));
                    }
                    if repl_leading && replace_region.is_some() {
                        return Err(RuntimeError::Unsupported(
                            "INSPECT REPLACING LEADING with a BEFORE/AFTER region is a later rung"
                                .into(),
                        ));
                    }
                    Ok(Stmt::InspectTallyReplace {
                        source,
                        counter,
                        delim,
                        tally_leading: leading,
                        tally_region,
                        search,
                        replace,
                        replace_leading: repl_leading,
                        replace_region,
                    })
                }
                (false, true) => {
                    // Dispatch on the number of replace items. Exactly ONE item
                    // keeps the full single-item path (LEADING, region, …)
                    // UNCHANGED via `read_inspect_replacing_all`; TWO OR MORE items
                    // take the new multi-item path (`ALL`-only, single-char, no
                    // region — enforced by `read_inspect_replacing_multi`). Reading
                    // the phrase child once here keeps the compiler's CST-side
                    // dispatch (which counts the same `replace_item` children)
                    // co-total with this reader.
                    let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
                        RuntimeError::Unsupported(
                            "INSPECT without a REPLACING clause is a later rung".into(),
                        )
                    })?;
                    let items = child_nodes(replacing, "replace_item");
                    // Detect a lone `REPLACING CHARACTERS BY x` FIRST — a SINGLE
                    // replace item whose tokens carry the CHARACTERS keyword — before
                    // the ALL/LEADING operand logic. (A MULTI-item list containing a
                    // CHARACTERS item stays a later rung, rejected by
                    // `read_inspect_replacing_multi` below; the CHARACTERS reject in
                    // `read_inspect_replacing_all` still guards the COMBINED form.)
                    if let [ri] = items.as_slice() {
                        let toks = child_tokens(ri);
                        if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
                            // Guard 3 — a `{BEFORE|AFTER}` region on the CHARACTERS item
                            // is a later rung (a byte window can split a multi-byte char
                            // mid-position, which the oracle's `String` storage cannot
                            // represent; the same reject fires on the compiler).
                            if child_node(ri, "inspect_region").is_some() {
                                return Err(RuntimeError::Unsupported(
                                    "INSPECT REPLACING CHARACTERS with a BEFORE/AFTER region is a later rung"
                                        .into(),
                                ));
                            }
                            // The lone `operand` child is the replacement `x`
                            // (guards 1/2/4 are applied at exec time / by the caller's
                            // source-category check, identically to the compiler).
                            let replace_node = child_node(ri, "operand").ok_or_else(|| {
                                RuntimeError::Unsupported(
                                    "INSPECT REPLACING CHARACTERS without a BY replacement".into(),
                                )
                            })?;
                            return Ok(Stmt::InspectReplacingCharacters {
                                source,
                                replace: read_operand(replace_node)?,
                            });
                        }
                    }
                    if items.len() >= 2 {
                        let items = read_inspect_replacing_multi(verb)?;
                        Ok(Stmt::InspectReplacingMulti { source, items })
                    } else {
                        let (search, replace, leading, region) =
                            read_inspect_replacing_all(verb)?;
                        Ok(Stmt::InspectReplacing { source, search, replace, leading, region })
                    }
                }
                // A lone TALLYING (or neither phrase, which `read_inspect_tally_all`
                // rejects as a missing TALLYING clause). Dispatch on the number of
                // `FOR` items UNDER THE SOLE counter, mirroring the multi-REPLACING
                // dispatch above: exactly ONE `tally_item` keeps the full single-item
                // path (LEADING, region, …) UNCHANGED via `read_inspect_tally_all`; TWO
                // OR MORE `tally_item`s under one `tally_for` take the new multi-item
                // path (`ALL`-only, single-char, each with its OWN optional region — one
                // first-match-per-position pass into the shared counter, enforced by
                // `read_inspect_tally_multi`).
                // The multi path fires ONLY when there is EXACTLY ONE `tally_for`: SEVERAL
                // counters (more than one `tally_for`) stays a later rung, rejected
                // unchanged by `read_inspect_tally_all`. Counting the same `tally_item`
                // children the compiler counts keeps the two engines' CST-side dispatch
                // co-total.
                _ => {
                    if let Some(tallying) = child_node(verb, "inspect_tallying") {
                        let fors = child_nodes(tallying, "tally_for");
                        // TWO OR MORE `tally_for` groups take the NEW multi-COUNTER path:
                        // each group has its own counter, and ALL groups' items form ONE
                        // combined priority list scanned in a single left-to-right pass
                        // (see `Stmt::InspectTallyCounters`). This dispatch is PURELY on
                        // `fors.len() >= 2`, so it precedes the single-`tally_for`
                        // branches below — several counters is no longer rejected by
                        // `read_inspect_tally_all` (that reject still guards the COMBINED
                        // `TALLYING … REPLACING` form, which routes through it directly).
                        if fors.len() >= 2 {
                            let groups = read_inspect_tally_counters(verb)?;
                            return Ok(Stmt::InspectTallyCounters { source, groups });
                        }
                        if let [tf] = fors.as_slice() {
                            if child_nodes(tf, "tally_item").len() >= 2 {
                                let (counter, items) = read_inspect_tally_multi(verb)?;
                                return Ok(Stmt::InspectTallyMulti { source, counter, items });
                            }
                        }
                    }
                    let (counter, delim, leading, characters, region) =
                        read_inspect_tally_all(verb)?;
                    Ok(Stmt::Inspect { source, counter, delim, leading, characters, region })
                }
            }
        }
        other => Err(RuntimeError::Unsupported(format!("the {} verb", verb_name(other)))),
    }
}

/// Extract the supported `TALLYING counter FOR ALL delim [{BEFORE|AFTER} x]` /
/// `FOR LEADING delim` (or `FOR CHARACTERS`) phrase from an `inspect_stmt`,
/// returning `(counter_name, delim_operand, leading, characters, region)` where
/// `leading` is `true` for `FOR LEADING` and `false` for `FOR ALL`, `characters` is
/// `true` for the `FOR CHARACTERS` form (in which case `delim` is a never-read
/// placeholder and `leading` is `false`), and `region` carries an optional
/// `{BEFORE|AFTER} x` window (see [`Region`]). Rejects every later-rung form the
/// grammar also accepts: several counters and several `FOR` phrases. `CHARACTERS`
/// is now ACCEPTED for the single-item single-counter case (count = window length);
/// only the multi-item / multi-counter `CHARACTERS` forms remain later rungs, guarded
/// in `read_inspect_tally_multi` / the several-counters dispatch. A
/// `FOR LEADING` phrase carrying a region is now ACCEPTED here — the STANDALONE
/// `FOR LEADING … BEFORE/AFTER` form is supported this rung (the count anchors the
/// leading run at the window start). The COMBINED `TALLYING … REPLACING` form still
/// defers a LEADING half with a region; that gate lives in the caller
/// (`read_statement`), not here. (`REPLACING` (lone or combined, `ALL` or `LEADING`
/// on either half) and a non-alphanumeric source/counter are handled by the caller
/// and exec; a multi-character region delimiter is rejected at exec by
/// `single_delim_char`, exactly like the tally delimiter itself, so both engines
/// diagnose it identically.)
fn read_inspect_tally_all(
    verb: &GrammarASTNode,
) -> Result<(String, Operand, bool, bool, Option<Region>), RuntimeError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    let fors = child_nodes(tallying, "tally_for");
    let tf = match fors.as_slice() {
        [one] => *one,
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT TALLYING with several counters is a later rung".into(),
            ))
        }
    };
    let counter = first_token(tf, "NAME")
        .ok_or_else(|| RuntimeError::Unsupported("INSPECT TALLYING without a counter".into()))?;
    let items = child_nodes(tf, "tally_item");
    let ti = match items.as_slice() {
        [one] => *one,
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT TALLYING with several FOR phrases is a later rung".into(),
            ))
        }
    };
    let toks = child_tokens(ti);
    // `FOR CHARACTERS` is the "count every position" form — it tallies the NUMBER OF
    // CHARACTER POSITIONS in the region window rather than matching a delimiter. The
    // grammar's `tally_item` CHARACTERS branch is `CHARACTERS { inspect_region }`: it
    // carries NO delimiter operand, so on this path we must NOT read an `operand`
    // child (there is none). We read the optional region EXACTLY as the ALL/LEADING
    // path does, set `characters = true`, `leading = false`, and hand back a never-
    // read placeholder for `delim` (the exec skips `single_delim_char` and every other
    // delimiter use when `characters == true`). The count then becomes the window
    // length — `length(source)` with no region, or `end - start` with one — inheriting
    // the SAME BEFORE→whole / AFTER→empty not-found asymmetry `FOR ALL` uses.
    let characters = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS");
    // A `{BEFORE|AFTER} x` region now PARSES into an `Option<Region>` (it used to
    // be rejected wholesale here) REGARDLESS of `leading`/`characters`: the STANDALONE
    // `FOR LEADING … BEFORE/AFTER` and `FOR CHARACTERS … BEFORE/AFTER` forms are
    // supported this rung. The COMBINED `TALLYING … REPLACING` form still rejects a
    // LEADING half carrying a region (and any CHARACTERS half); those gates live in
    // the combined caller (`read_statement`), not here, so relaxing this shared reader
    // does not leak the combination into the combined form.
    let region = match child_node(ti, "inspect_region") {
        None => None,
        Some(region_node) => Some(read_inspect_region(region_node)?),
    };
    if characters {
        // No delimiter to read on the CHARACTERS path. Stash a placeholder in `delim`
        // that is NEVER consumed (guaranteed by `characters == true` at every use).
        return Ok((
            counter,
            Operand::Lit(Lit::Str(" ".to_string())),
            false,
            true,
            region,
        ));
    }
    // `FOR LEADING` is supported (leading-run count); `FOR ALL` is the default.
    // The keyword picks the count semantics threaded through to `inspect_tally`.
    let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
    let delim_node = child_node(ti, "operand").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT TALLYING FOR ALL/LEADING without a delimiter".into())
    })?;
    Ok((counter, read_operand(delim_node)?, leading, false, region))
}

/// Extract the `TALLYING counter FOR ALL a [{BEFORE|AFTER} p] ALL b [{BEFORE|AFTER}
/// q] …` phrase from an `inspect_stmt` whose SOLE counter carries TWO OR MORE `FOR`
/// items, returning `(counter_name, items)` where `items` are
/// `(delimiter_operand, Option<Region>)` pairs in WRITTEN ORDER (the order the exec
/// walks them at each position to realise first-match-per-position). Only called when
/// the caller has already confirmed EXACTLY ONE `tally_for` with `>= 2` `tally_item`
/// children — the single-item case keeps [`read_inspect_tally_all`] and all its
/// capabilities (LEADING, region), and SEVERAL counters (more than one `tally_for`)
/// stays a later rung rejected there.
///
/// Scope bound for the multi-item path (this rung): EVERY item must be `ALL` or
/// `LEADING` (NO `CHARACTERS`). Each item carries a `leading` flag (`true` for a
/// `LEADING` item) AND its OWN optional `{BEFORE|AFTER} x` region (the third tuple
/// slot), read with the SAME `read_inspect_region` the single-item reader uses — the
/// per-item region reject and the multi-item `LEADING` reject are BOTH LIFTED this rung.
/// Only `CHARACTERS` in a multi-item list remains a later rung. Any item violating the
/// remaining scope is a clean later-rung `Unsupported`, with the SAME messages the
/// compiler-side reader raises, so both engines accept exactly the same multi-item
/// statements and reject the same ones identically. (A multi-character/figurative/
/// wider/numeric/reference-modified delimiter is NOT rejected here — it falls to the
/// SAME `single_delim_char` check the single-item exec uses, so that rejection is
/// identical across single and multi.)
fn read_inspect_tally_multi(
    verb: &GrammarASTNode,
) -> Result<(String, Vec<TallyMultiLeadingItem>), RuntimeError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    // Exactly one counter (`tally_for`): several counters is a later rung, diagnosed
    // with the SAME message `read_inspect_tally_all` raises so the reject is uniform.
    let fors = child_nodes(tallying, "tally_for");
    let tf = match fors.as_slice() {
        [one] => *one,
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT TALLYING with several counters is a later rung".into(),
            ))
        }
    };
    let counter = first_token(tf, "NAME")
        .ok_or_else(|| RuntimeError::Unsupported("INSPECT TALLYING without a counter".into()))?;
    let mut items = Vec::new();
    for ti in child_nodes(tf, "tally_item") {
        let toks = child_tokens(ti);
        // `CHARACTERS` is not supported for a multi-item list this rung (it is not
        // even supported for a single item). Reuse the single-item message.
        if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
            return Err(RuntimeError::Unsupported(
                "INSPECT TALLYING … FOR CHARACTERS is a later rung".into(),
            ));
        }
        // A `LEADING` item in a multi-item list is now ACCEPTED (this rung): the multi
        // path supports a MIX of `ALL` and `LEADING` items. The keyword picks per-item
        // count semantics threaded to `exec_inspect_tally_multi` (a `LEADING` item counts
        // only its run anchored at its window start). (A LONE `FOR LEADING` is still
        // supported via the single-item path, not here.)
        let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
        // A `{BEFORE|AFTER}` region on an item is now ACCEPTED (this rung): read it
        // into an `Option<Region>` with the SAME `read_inspect_region` the single-item
        // reader uses. The region contributes its OWN nested `operand` (the region
        // delimiter) under the `inspect_region` child, so the item's DIRECT `operand`
        // child below is still exactly the tally delimiter — the region delimiter is
        // not among the item's direct operands.
        let region = match child_node(ti, "inspect_region") {
            None => None,
            Some(region_node) => Some(read_inspect_region(region_node)?),
        };
        let delim_node = child_node(ti, "operand").ok_or_else(|| {
            RuntimeError::Unsupported("INSPECT TALLYING FOR ALL/LEADING without a delimiter".into())
        })?;
        items.push((read_operand(delim_node)?, leading, region));
    }
    Ok((counter, items))
}

/// Extract the `TALLYING c1 FOR ALL a [ALL b …] c2 FOR ALL d …` phrase from an
/// `inspect_stmt` carrying TWO OR MORE `tally_for` groups, returning the
/// `(counter_name, delims)` groups in WRITTEN ORDER (and, within each group, the
/// single-char delimiter operands in written order). Only called when the caller has
/// already confirmed `>= 2` `tally_for` groups — exactly ONE group keeps the single-
/// counter readers (`read_inspect_tally_all` / `read_inspect_tally_multi`) UNCHANGED.
///
/// Scope bound for the multi-counter path (this rung): EVERY item of EVERY group must
/// be a plain `FOR ALL` item with NO `LEADING`/`CHARACTERS`; each item MAY now carry its
/// OWN optional `{BEFORE|AFTER}` region (the region reject is LIFTED this rung), read
/// with the SAME `read_inspect_region` the single-item reader uses. Any item violating
/// the remaining scope is a clean later-rung `Unsupported`, with the SAME messages the
/// compiler-side `inspect_tally_counters` reader raises, so both engines accept exactly
/// the same multi-counter statements and reject the same ones identically. (A
/// multi-character/figurative/wider/numeric/reference-modified delimiter is NOT rejected
/// here — it falls to the SAME `single_delim_char` check the single-item exec uses, so
/// that rejection is identical across every tally path.) The counters themselves are
/// validated (unsigned-integer `PIC 9(n)`) at exec time by `exec_inspect_tally_counters`,
/// exactly as the single-item tally validates its lone counter at exec time.
fn read_inspect_tally_counters(
    verb: &GrammarASTNode,
) -> Result<Vec<TallyCounterGroup>, RuntimeError> {
    let tallying = child_node(verb, "inspect_tallying").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a TALLYING clause is a later rung".into())
    })?;
    let mut groups = Vec::new();
    for tf in child_nodes(tallying, "tally_for") {
        let counter = first_token(tf, "NAME").ok_or_else(|| {
            RuntimeError::Unsupported("INSPECT TALLYING without a counter".into())
        })?;
        let mut items = Vec::new();
        for ti in child_nodes(tf, "tally_item") {
            let toks = child_tokens(ti);
            // `CHARACTERS` is not supported in the multi-counter path (nor anywhere yet).
            if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
                return Err(RuntimeError::Unsupported(
                    "INSPECT TALLYING … FOR CHARACTERS is a later rung".into(),
                ));
            }
            // A `LEADING` item in the multi-counter path is a later rung: the path is
            // `ALL`-only. (A LONE `FOR LEADING` is still supported via the single path.)
            if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING") {
                return Err(RuntimeError::Unsupported(
                    "INSPECT TALLYING with several counters and a LEADING item is a later rung"
                        .into(),
                ));
            }
            // A `{BEFORE|AFTER}` region on an item is now ACCEPTED (this rung): read it
            // into an `Option<Region>` with the SAME `read_inspect_region` the single-item
            // reader uses. The region contributes its OWN nested `operand` (the region
            // delimiter) under the `inspect_region` child, so the item's DIRECT `operand`
            // child below is still exactly the tally delimiter.
            let region = match child_node(ti, "inspect_region") {
                None => None,
                Some(region_node) => Some(read_inspect_region(region_node)?),
            };
            let delim_node = child_node(ti, "operand").ok_or_else(|| {
                RuntimeError::Unsupported("INSPECT TALLYING FOR ALL without a delimiter".into())
            })?;
            items.push((read_operand(delim_node)?, region));
        }
        groups.push((counter, items));
    }
    Ok(groups)
}

/// Read an `inspect_region` CST node — the `{BEFORE|AFTER} x` phrase — into a
/// [`Region`]. The grammar's region rule is `(BEFORE | AFTER) operand` (the
/// `INITIAL` keyword is optional/absent), so the leading keyword picks the side
/// and the lone `operand` child is the delimiter `x`. The delimiter is NOT
/// width-checked here: exactly like the tally delimiter, a multi-character region
/// delimiter is a clean later-rung error raised by `single_delim_char` at exec
/// time, keeping both engines' rejection identical.
fn read_inspect_region(region_node: &GrammarASTNode) -> Result<Region, RuntimeError> {
    let toks = child_tokens(region_node);
    let kind = if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "BEFORE") {
        RegionKind::Before
    } else if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "AFTER") {
        RegionKind::After
    } else {
        return Err(RuntimeError::Unsupported(
            "INSPECT region without a BEFORE or AFTER keyword".into(),
        ));
    };
    let delim_node = child_node(region_node, "operand").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT BEFORE/AFTER region without a delimiter".into())
    })?;
    Ok(Region { kind, delim: read_operand(delim_node)? })
}

/// Extract the supported `REPLACING ALL search BY replace [{BEFORE|AFTER} x]` /
/// `REPLACING LEADING search BY replace` phrase from an `inspect_stmt`, returning
/// `(search_operand, replace_operand, leading, region)` where `leading` is `true`
/// for `REPLACING LEADING` (replace only the leading run) and `false` for
/// `REPLACING ALL` (replace every occurrence), and `region` carries an optional
/// `{BEFORE|AFTER} x` window (see [`Region`], shared with the TALLYING reader).
/// Rejects every later-rung form the grammar also accepts: several replace items,
/// and a `CHARACTERS` or `FIRST` replacement. A `REPLACING LEADING` phrase carrying
/// a region is now ACCEPTED here — the STANDALONE `REPLACING LEADING … BEFORE/AFTER`
/// form is supported this rung (the substitution anchors the leading run at the
/// window start), exactly mirroring the TALLYING side. Both `ALL` and `LEADING` are
/// accepted here, whether the phrase is lone or combined with `TALLYING`; for the
/// combined form the caller (`read_statement`) separately defers a LEADING half that
/// carries a region. (A non-alphanumeric source is rejected by the caller; a
/// multi-character/wider/figurative search, replacement, or region delimiter is
/// rejected by `single_delim_char` at exec time.)
fn read_inspect_replacing_all(
    verb: &GrammarASTNode,
) -> Result<(Operand, Operand, bool, Option<Region>), RuntimeError> {
    let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a REPLACING clause is a later rung".into())
    })?;
    let items = child_nodes(replacing, "replace_item");
    let ri = match items.as_slice() {
        [one] => *one,
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT REPLACING with several replace items is a later rung".into(),
            ))
        }
    };
    let toks = child_tokens(ri);
    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
        return Err(RuntimeError::Unsupported(
            "INSPECT REPLACING CHARACTERS is a later rung".into(),
        ));
    }
    if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "FIRST") {
        return Err(RuntimeError::Unsupported(
            "INSPECT REPLACING FIRST is a later rung".into(),
        ));
    }
    // `REPLACING LEADING` is now supported (leading-run replace); `REPLACING ALL`
    // is the default. The keyword selects the stop-at-first-mismatch behaviour
    // threaded through to `inspect_replace`.
    let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
    // A `{BEFORE|AFTER} x` region now PARSES into an `Option<Region>` (it used to be
    // rejected wholesale here) REGARDLESS of `leading`, reusing the SAME
    // `read_inspect_region` the TALLYING reader uses: the STANDALONE
    // `REPLACING LEADING … BEFORE/AFTER` form is supported this rung (the substitution
    // anchors the leading run at the window start — see `inspect_replace`), the exact
    // analogue of the count side. The COMBINED `TALLYING … REPLACING` form still
    // rejects a LEADING half carrying a region; that gate lives in the combined caller
    // (`read_statement`), so relaxing this shared reader does not leak the combination
    // into the combined form.
    let region = match child_node(ri, "inspect_region") {
        None => None,
        Some(region_node) => Some(read_inspect_region(region_node)?),
    };
    // `ALL`/`LEADING search BY replace` — the two `operand` children are the
    // search (first) and the replacement (second), in order. (A `BEFORE`/`AFTER`
    // region contributes its OWN nested `operand`, so we select the two operands
    // that belong to the `replace_item` itself — the region's delimiter lives on
    // the `inspect_region` child, not here.)
    let repl_ops: Vec<&GrammarASTNode> = child_nodes(ri, "operand");
    let (search_node, replace_node) = match repl_ops.as_slice() {
        [s, r] => (*s, *r),
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT REPLACING ALL/LEADING without a search and a BY replacement".into(),
            ))
        }
    };
    Ok((read_operand(search_node)?, read_operand(replace_node)?, leading, region))
}

/// Extract the `REPLACING ALL a BY x ALL b BY y [ALL c BY z …]` phrase from an
/// `inspect_stmt` that carries TWO OR MORE replace items, returning the items as a
/// `Vec<(search, replace)>` in WRITTEN ORDER (the order the exec walks them at each
/// position to realise first-match-wins). Only called when the caller has already
/// counted `>= 2` `replace_item` children — the single-item case keeps
/// [`read_inspect_replacing_all`] and all its capabilities.
///
/// Scope bound for the multi-item path (this rung): EVERY item must be a single-char
/// `{ALL|LEADING} search BY replace` pair with NO `CHARACTERS`/`FIRST`. Each item MAY
/// be `ALL` OR `LEADING` (THIS rung lifts the multi-item `LEADING` reject, mirroring
/// how the tally side's `read_inspect_tally_multi` reads a per-item leading flag), and
/// MAY carry its OWN optional `{BEFORE|AFTER} x` region, read with the SAME
/// `read_inspect_region` the single-item reader uses. Any item violating the remaining
/// scope is a clean later-rung `Unsupported`, with the SAME messages the compiler-side
/// reader raises, so both engines accept exactly the same multi-item statements and
/// reject the same ones identically. (A multi-character/figurative/wider/numeric/
/// reference-modified search, replacement, or region delimiter is not rejected here —
/// it falls to the SAME `single_delim_char` check the single-item exec uses, so that
/// rejection is identical across single and multi.)
fn read_inspect_replacing_multi(
    verb: &GrammarASTNode,
) -> Result<Vec<ReplaceMultiLeadingItem>, RuntimeError> {
    let replacing = child_node(verb, "inspect_replacing").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a REPLACING clause is a later rung".into())
    })?;
    let mut items = Vec::new();
    for ri in child_nodes(replacing, "replace_item") {
        let toks = child_tokens(ri);
        // `CHARACTERS`/`FIRST` are not supported for a multi-item list this rung
        // (they are not even supported for a single item). Reuse the single-item
        // messages so the diagnostic is uniform.
        if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "CHARACTERS") {
            return Err(RuntimeError::Unsupported(
                "INSPECT REPLACING CHARACTERS is a later rung".into(),
            ));
        }
        if toks.iter().any(|(k, v)| k == "KEYWORD" && v == "FIRST") {
            return Err(RuntimeError::Unsupported(
                "INSPECT REPLACING FIRST is a later rung".into(),
            ));
        }
        // A `LEADING` item in a multi-item list is now ACCEPTED (THIS rung): the multi
        // path supports a MIX of `ALL` and `LEADING` items — the replace-side twin of
        // the tally side's `read_inspect_tally_multi`, which already reads a per-item
        // leading flag. The keyword picks per-item substitution semantics threaded to
        // `exec_inspect_replacing_multi` (a `LEADING` item replaces only its run anchored
        // at the window start). (A LONE `REPLACING LEADING` is still supported via the
        // single-item path, not here.)
        let leading = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "LEADING");
        // A `{BEFORE|AFTER}` region on an item is ACCEPTED: read it into an
        // `Option<Region>` with the SAME `read_inspect_region` the single-item reader
        // uses. The region contributes its OWN nested `operand` (the delimiter) under the
        // `inspect_region` child, so the item's two DIRECT `operand` children are still
        // exactly the search/replacement (see below) — the region delimiter is not among
        // them.
        let region = match child_node(ri, "inspect_region") {
            None => None,
            Some(region_node) => Some(read_inspect_region(region_node)?),
        };
        // `{ALL|LEADING} search BY replace` — the two DIRECT `operand` children are the
        // search (first) and the replacement (second), in written order. A region's
        // delimiter rides on the `inspect_region` child, not as a direct child of
        // `replace_item`, so exactly two direct operands are expected whether or not a
        // region is present.
        let ops: Vec<&GrammarASTNode> = child_nodes(ri, "operand");
        let (search_node, replace_node) = match ops.as_slice() {
            [s, r] => (*s, *r),
            _ => {
                return Err(RuntimeError::Unsupported(
                    "INSPECT REPLACING ALL/LEADING without a search and a BY replacement".into(),
                ))
            }
        };
        items.push((read_operand(search_node)?, read_operand(replace_node)?, leading, region));
    }
    Ok(items)
}

/// Extract the `CONVERTING from TO to [{BEFORE|AFTER} x]` phrase from an
/// `inspect_stmt`, returning the two string-literal operands and the optional region
/// window `(from, to, region)`. This rung only supports STRING-LITERAL translation
/// tables and rejects the later-rung forms the grammar also accepts: a data-name /
/// figurative / numeric-literal / reference-modified `from`/`to`. A `{BEFORE|AFTER}
/// x` region now PARSES into an `Option<Region>` (it used to be rejected wholesale
/// here), reusing the SAME `read_inspect_region` the TALLYING/REPLACING readers use;
/// a multi-character region delimiter stays a later rung, rejected at exec time by
/// `single_delim_char`. (The equal-length requirement is checked at exec time so it
/// can share the same diagnostic as any other CONVERTING error.)
fn read_inspect_converting(
    verb: &GrammarASTNode,
) -> Result<(ConvertOperand, ConvertOperand, Option<Region>), RuntimeError> {
    let converting = child_node(verb, "inspect_converting").ok_or_else(|| {
        RuntimeError::Unsupported("INSPECT without a CONVERTING clause is a later rung".into())
    })?;
    let region = match child_node(converting, "inspect_region") {
        None => None,
        Some(region_node) => Some(read_inspect_region(region_node)?),
    };
    // `from TO to` — the two `operand` children are the FROM (first) and the TO
    // (second), in order. (A `{BEFORE|AFTER}` region contributes its OWN nested
    // `operand` under the `inspect_region` child, not a direct `operand` here, so
    // these two direct children are exactly the FROM and TO.)
    let ops = child_nodes(converting, "operand");
    let (from_node, to_node) = match ops.as_slice() {
        [f, t] => (*f, *t),
        _ => {
            return Err(RuntimeError::Unsupported(
                "INSPECT CONVERTING without a FROM and a TO operand".into(),
            ))
        }
    };
    Ok((
        read_converting_operand(from_node, "from")?,
        read_converting_operand(to_node, "to")?,
        region,
    ))
}

/// Read a CONVERTING `from`/`to` operand into a [`ConvertOperand`]. A string
/// literal becomes [`ConvertOperand::Literal`] (its set is fixed now); a data-name
/// becomes [`ConvertOperand::Item`] (its set is the item's CURRENT storage, read at
/// exec time — this rung LIFTS the old data-name reject). A figurative constant,
/// numeric literal, or reference modification stays a later rung. `which` names the
/// position (`"from"`/`"to"`) for the diagnostic.
fn read_converting_operand(op: &GrammarASTNode, which: &str) -> Result<ConvertOperand, RuntimeError> {
    match read_operand(op)? {
        Operand::Lit(Lit::Str(s)) => Ok(ConvertOperand::Literal(s)),
        Operand::Ident(name) => Ok(ConvertOperand::Item(name)),
        Operand::Lit(Lit::Num(_)) => Err(RuntimeError::Unsupported(format!(
            "INSPECT CONVERTING with a numeric-literal {which} operand is a later rung"
        ))),
        Operand::Lit(Lit::Fig(_)) => Err(RuntimeError::Unsupported(format!(
            "INSPECT CONVERTING with a figurative-constant {which} operand is a later rung"
        ))),
        Operand::RefMod { .. } => Err(RuntimeError::Unsupported(format!(
            "INSPECT CONVERTING with a reference-modified {which} operand is a later rung"
        ))),
    }
}

/// Read a `condition` node: a `disjunction` of `AND`-joined simple conditions,
/// combined left-associatively. `AND` binds tighter than `OR`.
fn read_condition(cond: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    let disjunction = child_node(cond, "disjunction")
        .ok_or_else(|| RuntimeError::Unsupported("empty condition".into()))?;
    read_disjunction(disjunction)
}

/// `disjunction = conjunction { "OR" conjunction }` — collect into [`Cond::Or`].
fn read_disjunction(node: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    read_group(node, "conjunction", read_conjunction, Cond::Or)
}

/// `conjunction = negation { "AND" negation }` — collect into [`Cond::And`].
fn read_conjunction(node: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    read_group(node, "negation", read_negation, Cond::And)
}

/// `negation = [ "NOT" ] simple_condition` — wrap in [`Cond::Not`] when the `NOT`
/// keyword is present.
fn read_negation(node: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    let simple = child_node(node, "simple_condition")
        .ok_or_else(|| RuntimeError::Unsupported("negation without a condition".into()))?;
    let inner = read_simple_condition(simple)?;
    let negated = child_tokens(node).iter().any(|(k, v)| k == "KEYWORD" && v == "NOT");
    Ok(if negated { Cond::Not(Box::new(inner)) } else { inner })
}

/// Collect a rule's same-named operand children into a **flat** `AND`/`OR` list.
/// A lone child needs no wrapper (so a plain relation stays a `Relation`); two or
/// more become one `combine(Vec<Cond>)` node — never a nested tree, so evaluation
/// iterates rather than recursing on the chain length.
fn read_group(
    node: &GrammarASTNode,
    child_rule: &str,
    read_child: impl Fn(&GrammarASTNode) -> Result<Cond, RuntimeError>,
    combine: impl Fn(Vec<Cond>) -> Cond,
) -> Result<Cond, RuntimeError> {
    let mut parts = Vec::new();
    for child in child_nodes(node, child_rule) {
        parts.push(read_child(child)?);
    }
    match parts.len() {
        0 => Err(RuntimeError::Unsupported("empty condition group".into())),
        1 => Ok(parts.remove(0)),
        _ => Ok(combine(parts)),
    }
}

/// `simple_condition = relation | condition_name | "(" condition ")"`.
fn read_simple_condition(node: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    if let Some(relation) = child_node(node, "relation") {
        return read_relation(relation);
    }
    if let Some(cn) = child_node(node, "condition_name") {
        let name = first_token(cn, "NAME")
            .ok_or_else(|| RuntimeError::Unsupported("condition-name without a NAME".into()))?;
        return Ok(Cond::ConditionName(name));
    }
    if let Some(inner) = child_node(node, "condition") {
        return read_condition(inner);
    }
    Err(RuntimeError::Unsupported("condition must be a relation, condition-name, or parenthesised".into()))
}

/// Read a `relation` node (`operand relop operand`).
fn read_relation(relation: &GrammarASTNode) -> Result<Cond, RuntimeError> {
    let operands = child_nodes(relation, "operand");
    if operands.len() != 2 {
        return Err(RuntimeError::Unsupported("relation must be `operand relop operand`".into()));
    }
    let left = read_operand(operands[0])?;
    let right = read_operand(operands[1])?;
    let relop = child_node(relation, "relop")
        .ok_or_else(|| RuntimeError::Unsupported("relation without a relational operator".into()))?;
    let toks = child_tokens(relop);
    let explicit_not = toks.iter().any(|(k, v)| k == "KEYWORD" && v == "NOT");
    // Each operator resolves to a base relation plus a *baseline* negation: the
    // symbols `>=`/`<=`/`<>` already mean "not <", "not >", "not =". A written
    // `NOT` composes with that baseline by XOR.
    let (op, baseline_neg) = toks
        .iter()
        .find_map(|(k, v)| relop_meaning(k, v))
        .ok_or_else(|| RuntimeError::Unsupported("unrecognised relational operator".into()))?;
    Ok(Cond::Relation { left, op, negated: explicit_not ^ baseline_neg, right })
}

/// Map a relop token to `(base relation, baseline negation)`. Word forms
/// (`GREATER`/`LESS`/`EQUAL`) and the symbols `>`/`<`/`=` are un-negated; `>=`,
/// `<=`, `<>` carry a baseline negation (`>=` ≡ `NOT <`, etc.).
fn relop_meaning(kind: &str, value: &str) -> Option<(RelOp, bool)> {
    match (kind, value) {
        ("KEYWORD", "GREATER") | ("GT", _) => Some((RelOp::Greater, false)),
        ("KEYWORD", "LESS") | ("LT", _) => Some((RelOp::Less, false)),
        ("KEYWORD", "EQUAL") | ("EQ", _) => Some((RelOp::Equal, false)),
        ("GE", _) => Some((RelOp::Less, true)),
        ("LE", _) => Some((RelOp::Greater, true)),
        ("NE", _) => Some((RelOp::Equal, true)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Arithmetic expressions (COMPUTE)
// ---------------------------------------------------------------------------
//
// The grammar's rule cascade already encodes precedence, so each reader here
// just folds one level's operands into a binary tree. `+ - * /` fold
// left-to-right (left-associative); `**` folds right-to-left (COBOL's
// right-associative exponentiation). A single operand with no operator collapses
// to the operand itself, so `COMPUTE X = A` carries no spurious tree nodes.
//
// DoS bound: the grammar's `{ … }` repetition is *flat* (no rule recursion), so
// the parser's recursion-depth cap does NOT limit how *wide* a chain can be —
// `A + A + … + A` with N terms is one node with 2N−1 children at bounded parse
// depth. Folding that yields an N-deep `Expr` tree, and both `eval_expr` and the
// recursive `Drop` of `Box<Expr>` would then overflow the native stack. So a
// single [`MAX_EXPR_OPERANDS`] budget is threaded through the whole expression
// (counting every primary, across parenthesised levels too); exhausting it is a
// clean `RuntimeError`, which keeps the folded tree — hence the eval/Drop
// recursion depth — bounded.

/// Largest number of primaries (`NUMBER`/`NAME`/parenthesised group) a single
/// `COMPUTE` expression may contain. Real expressions have a handful; the cap is
/// only a native-stack backstop against a hostile flat chain.
const MAX_EXPR_OPERANDS: usize = 1024;

/// Read a `COMPUTE` expression, bounding its size against a stack-overflow DoS.
fn read_arith_expr_bounded(node: &GrammarASTNode) -> Result<Expr, RuntimeError> {
    let mut budget = MAX_EXPR_OPERANDS;
    read_arith_expr(node, &mut budget)
}

/// `arith_expr = arith_term { ( "+" | "-" ) arith_term }` — additive, left-assoc.
fn read_arith_expr(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    read_binary_chain(node, read_arith_term, |t| match t {
        "PLUS" => Some(ArithOp::Add),
        "MINUS" => Some(ArithOp::Sub),
        _ => None,
    }, budget)
}

/// `arith_term = arith_factor { ( "*" | "/" ) arith_factor }` — multiplicative.
fn read_arith_term(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    read_binary_chain(node, read_arith_factor, |t| match t {
        "STAR" => Some(ArithOp::Mul),
        "SLASH" => Some(ArithOp::Div),
        _ => None,
    }, budget)
}

/// `arith_factor = arith_unary { "**" arith_unary }` — exponentiation, folded
/// right-associatively so `A ** B ** C` = `A ** (B ** C)`.
fn read_arith_factor(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    let units = child_nodes(node, "arith_unary");
    let mut rev = units.iter().rev();
    let last = rev
        .next()
        .ok_or_else(|| RuntimeError::Unsupported("empty arithmetic factor".into()))?;
    let mut expr = read_arith_unary(last, budget)?;
    for u in rev {
        expr = Expr::Binary {
            op: ArithOp::Pow,
            left: Box::new(read_arith_unary(u, budget)?),
            right: Box::new(expr),
        };
    }
    Ok(expr)
}

/// `arith_unary = [ "+" | "-" ] arith_primary` — a leading minus negates; a
/// leading plus is a no-op.
fn read_arith_unary(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    let neg = child_tokens(node).iter().any(|(k, _)| k == "MINUS");
    let prim = child_node(node, "arith_primary")
        .ok_or_else(|| RuntimeError::Unsupported("unary operator without an operand".into()))?;
    let e = read_arith_primary(prim, budget)?;
    Ok(if neg { Expr::Unary { neg: true, operand: Box::new(e) } } else { e })
}

/// `arith_primary = NUMBER | NAME | "(" arith_expr ")"`. Charges one unit of the
/// expression's operand budget.
fn read_arith_primary(node: &GrammarASTNode, budget: &mut usize) -> Result<Expr, RuntimeError> {
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| RuntimeError::Unsupported("COMPUTE expression too large".into()))?;
    // A parenthesised sub-expression recurses back to the top of the cascade.
    if let Some(inner) = child_node(node, "arith_expr") {
        return read_arith_expr(inner, budget);
    }
    for (k, v) in child_tokens(node) {
        match k.as_str() {
            "NUMBER" => return Ok(Expr::Num(v)),
            "NAME" => return Ok(Expr::Var(v)),
            _ => {}
        }
    }
    Err(RuntimeError::Unsupported("empty arithmetic primary".into()))
}

/// Fold a `head { op tail }` node into a left-associative binary tree. `sub`
/// reads each operand node; `map_op` maps an operator token's type name to an
/// [`ArithOp`] (returning `None` for tokens that are not operators). `budget`
/// bounds the total operand count (see [`MAX_EXPR_OPERANDS`]).
fn read_binary_chain(
    node: &GrammarASTNode,
    sub: fn(&GrammarASTNode, &mut usize) -> Result<Expr, RuntimeError>,
    map_op: fn(&str) -> Option<ArithOp>,
    budget: &mut usize,
) -> Result<Expr, RuntimeError> {
    let mut expr: Option<Expr> = None;
    let mut pending: Option<ArithOp> = None;
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(n) => {
                let operand = sub(n, budget)?;
                expr = Some(match (expr.take(), pending.take()) {
                    (Some(left), Some(op)) => Expr::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(operand),
                    },
                    // First operand (or a malformed chain missing its operator):
                    // take the operand as the running expression.
                    (_, _) => operand,
                });
            }
            ASTNodeOrToken::Token(t) => {
                if let Some(op) = map_op(t.effective_type_name()) {
                    pending = Some(op);
                }
            }
        }
    }
    expr.ok_or_else(|| RuntimeError::Unsupported("empty arithmetic expression".into()))
}

/// Read a `perform_varying` node
/// (`VARYING NAME FROM operand BY operand UNTIL condition`).
fn read_perform_varying(v: &GrammarASTNode) -> Result<PerformMode, RuntimeError> {
    let var = first_token(v, "NAME")
        .ok_or_else(|| RuntimeError::Unsupported("PERFORM VARYING without a variable".into()))?;
    let operands = child_nodes(v, "operand");
    if operands.len() != 2 {
        return Err(RuntimeError::Unsupported(
            "PERFORM VARYING needs FROM and BY operands".into(),
        ));
    }
    let from = read_operand(operands[0])?;
    let by = read_operand(operands[1])?;
    let cond = child_node(v, "condition")
        .ok_or_else(|| RuntimeError::Unsupported("PERFORM VARYING without an UNTIL".into()))?;
    let until = read_condition(cond)?;
    Ok(PerformMode::Varying { var, from, by, until })
}

/// Read the trailing `[ROUNDED] [ON SIZE ERROR statements…]` clauses shared by
/// the arithmetic verbs (`ADD`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`) and `COMPUTE`.
fn read_rounded_and_size_error(
    verb: &GrammarASTNode,
) -> Result<(bool, Vec<Stmt>), RuntimeError> {
    let rounded = child_tokens(verb)
        .iter()
        .any(|(k, v)| k == "KEYWORD" && v == "ROUNDED");
    let on_size_error = match child_node(verb, "size_error") {
        Some(se) => child_nodes(se, "statement")
            .into_iter()
            .map(read_statement)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    Ok((rounded, on_size_error))
}

/// All `operand` child nodes of a verb, read to typed [`Operand`]s.
fn read_operands(verb: &GrammarASTNode) -> Result<Vec<Operand>, RuntimeError> {
    child_nodes(verb, "operand").into_iter().map(read_operand).collect()
}

/// The target NAME and optional GIVING NAME of an `ADD … TO`/`SUBTRACT … FROM`.
/// The direct NAME tokens are `[target]` or `[target, giving]`.
fn read_target_and_giving(verb: &GrammarASTNode) -> Result<(String, Option<String>), RuntimeError> {
    let names: Vec<String> = child_tokens(verb)
        .into_iter()
        .filter(|(k, _)| k == "NAME")
        .map(|(_, v)| v)
        .collect();
    let mut it = names.into_iter();
    let target = it
        .next()
        .ok_or_else(|| RuntimeError::Unsupported("arithmetic statement without a target".into()))?;
    Ok((target, it.next()))
}

/// Human-friendly verb name from a grammar rule name (`move_stmt` → `MOVE`).
fn verb_name(rule: &str) -> String {
    rule.trim_end_matches("_stmt").replace('_', " ").to_uppercase()
}

//! # Compiler + Pike VM — the linear-time matching core
//!
//! The parsed [`Ast`](crate::ast::Ast) is compiled into a flat list of
//! instructions (a tiny bytecode), which the **Pike VM** then executes. The Pike
//! VM is a Thompson-NFA simulation: instead of backtracking (which can take
//! exponential time on adversarial patterns — a denial-of-service risk), it
//! advances *all* possible matches through the input in lockstep, one character
//! at a time, so matching is always **O(pattern × input)**. It resolves
//! ambiguity with *leftmost-first* priority (greedy quantifiers prefer to match
//! more), matching the behaviour of the `regex` crate for the patterns this
//! engine targets.
//!
//! This module implements `is_match` (boolean matching), which needs no capture
//! tracking — threads are bare program counters. Reporting match *extents*
//! (with capture slots) is a later addition.

use crate::ast::{Assertion, Ast, Class, Flags, ParseError};

/// One VM instruction.
#[derive(Debug, Clone)]
enum Inst {
    /// Consume one input character equal to `char` (case-insensitively if the
    /// program's flag is set).
    Char(char),
    /// Consume any one character (`.`); matches `\n` only if `dot_all`.
    Any { dot_all: bool },
    /// Consume one character that is a member of the class.
    Class(Class),
    /// Zero-width assertion.
    Assert(Assertion),
    /// Continue at both targets, `a` before `b` in priority order.
    Split(usize, usize),
    /// Continue at `target`.
    Jmp(usize),
    /// A successful match.
    Match,
}

/// A compiled regular expression program.
#[derive(Debug, Clone)]
pub struct Program {
    insts: Vec<Inst>,
    case_insensitive: bool,
    /// Unicode mode — whether `\b` word boundaries use the Unicode word set.
    /// (`\d\w\s` classes are already baked into instructions at parse time.)
    unicode: bool,
    /// True when every match must begin at the start of the input (`^…`), so the
    /// unanchored scan can stop seeding start threads past position 0.
    anchored_start: bool,
}

/// Compile an AST into a [`Program`]. `group_count` is the number of capturing
/// groups; `flags` carries case-insensitivity and dot-all. `max_insts` caps the
/// program size (a pattern that would exceed it is rejected).
pub fn compile(
    ast: &Ast,
    group_count: usize,
    flags: Flags,
    max_insts: usize,
) -> Result<Program, ParseError> {
    // `group_count` is unused for `is_match` (capture boundaries are not tracked);
    // it is part of the stable compile signature for the future extent path.
    let _ = group_count;
    let mut c = Compiler {
        insts: Vec::new(),
        dot_all: flags.dot_matches_new_line,
        cap: max_insts,
    };
    let anchored_start = starts_with_start_anchor(ast);
    c.emit(ast);
    c.insts.push(Inst::Match);
    if c.insts.len() > c.cap {
        return Err(ParseError(format!(
            "compiled program too large (> {max_insts} instructions)"
        )));
    }
    Ok(Program {
        insts: c.insts,
        case_insensitive: flags.case_insensitive,
        unicode: flags.unicode,
        anchored_start,
    })
}

/// Whether `ast` can match the empty string. Used to pick the correct star
/// compilation: a star whose body is nullable needs the "optional-plus" shape so
/// an empty iteration routes to the exit (matching the `regex` crate's extent),
/// whereas a star with a non-nullable body uses the simpler single-split loop.
fn is_nullable(ast: &Ast) -> bool {
    match ast {
        Ast::Empty | Ast::Assert(_) => true,
        Ast::Literal(_) | Ast::AnyChar | Ast::Class(_) => false,
        Ast::Concat(items) => items.iter().all(is_nullable),
        Ast::Alternate(branches) => branches.iter().any(is_nullable),
        Ast::Group { inner, .. } => is_nullable(inner),
        Ast::Repeat { inner, min, .. } => *min == 0 || is_nullable(inner),
    }
}

fn starts_with_start_anchor(ast: &Ast) -> bool {
    match ast {
        Ast::Assert(Assertion::StartText) => true,
        Ast::Concat(items) => items.first().is_some_and(starts_with_start_anchor),
        Ast::Group { inner, .. } => starts_with_start_anchor(inner),
        // An alternation is only anchored if *every* branch is.
        Ast::Alternate(branches) => branches.iter().all(starts_with_start_anchor),
        _ => false,
    }
}

struct Compiler {
    insts: Vec<Inst>,
    dot_all: bool,
    cap: usize,
}

impl Compiler {
    fn emit(&mut self, ast: &Ast) {
        // Stop growing once over the cap; `compile` turns this into an error.
        // This bounds memory even for `{0,huge}` expansions.
        if self.insts.len() > self.cap {
            return;
        }
        match ast {
            Ast::Empty => {}
            Ast::Literal(c) => self.insts.push(Inst::Char(*c)),
            Ast::AnyChar => self.insts.push(Inst::Any {
                dot_all: self.dot_all,
            }),
            Ast::Class(class) => self.insts.push(Inst::Class(class.clone())),
            Ast::Assert(a) => self.insts.push(Inst::Assert(*a)),
            Ast::Concat(items) => {
                for item in items {
                    self.emit(item);
                }
            }
            // Capturing and non-capturing groups compile identically for
            // `is_match` — only the sub-expression structure matters, not the
            // capture positions.
            Ast::Group { inner, .. } => self.emit(inner),
            Ast::Alternate(branches) => self.emit_alternation(branches),
            Ast::Repeat {
                inner,
                min,
                max,
                greedy,
            } => self.emit_repeat(inner, *min, *max, *greedy),
        }
    }

    fn emit_alternation(&mut self, branches: &[Ast]) {
        // a|b|c  =>  split a, (split b, c). Jumps route each branch to the end.
        if branches.len() == 1 {
            self.emit(&branches[0]);
            return;
        }
        let mut jmp_fixups = Vec::new();
        for (i, branch) in branches.iter().enumerate() {
            if i + 1 < branches.len() {
                let split_at = self.insts.len();
                self.insts.push(Inst::Split(0, 0)); // patched below
                let branch_start = self.insts.len();
                self.emit(branch);
                let jmp_at = self.insts.len();
                self.insts.push(Inst::Jmp(0)); // to end, patched later
                jmp_fixups.push(jmp_at);
                let next = self.insts.len();
                self.insts[split_at] = Inst::Split(branch_start, next);
            } else {
                self.emit(branch); // last branch: no split/jmp
            }
        }
        let end = self.insts.len();
        for j in jmp_fixups {
            self.insts[j] = Inst::Jmp(end);
        }
    }

    fn emit_repeat(&mut self, inner: &Ast, min: u32, max: Option<u32>, greedy: bool) {
        match max {
            None if min == 0 && is_nullable(inner) => {
                // `e*` with a **nullable body** — compile as the optional plus
                // `(e+)?`: an *entry* split and an *after-body* split, both choosing
                // between the body start and the exit. The after-body split loops
                // back to the body start (the `+` loop-back), so an empty iteration
                // re-enters the already-`seen` body start and dead-ends into the exit
                // at the correct priority — `(a??)*` on "aa" ⇒ the empty `0..0`, not
                // `0..2`. The textbook single-split loop would instead rank the body's
                // own consume alternative above the exit and over-match here. (Both
                // forms accept the same language, so `is_match` is unaffected; only
                // the reported extent differs.)
                let entry = self.insts.len();
                self.insts.push(Inst::Split(0, 0)); // patched below
                let body = self.insts.len();
                self.emit(inner);
                if self.insts.len() > self.cap {
                    return;
                }
                let after = self.insts.len();
                self.insts.push(Inst::Split(0, 0)); // patched below
                let exit = self.insts.len();
                let arms = if greedy { (body, exit) } else { (exit, body) };
                self.insts[entry] = Inst::Split(arms.0, arms.1);
                self.insts[after] = Inst::Split(arms.0, arms.1);
            }
            None if min == 0 => {
                // `e*` with a **non-nullable body** — the simple single-split loop
                // `U: split(body, exit); body; jmp U` (also the `regex` crate's
                // shape). The body always consumes at least one character per
                // iteration, so there is no empty-iteration priority subtlety, and an
                // inner lazy star (e.g. `(..)*?`) keeps its minimal-consumption
                // preference rather than being forced wider by the optional-plus form.
                let u = self.insts.len();
                self.insts.push(Inst::Split(0, 0)); // patched below
                let body = self.insts.len();
                self.emit(inner);
                if self.insts.len() > self.cap {
                    return;
                }
                self.insts.push(Inst::Jmp(u));
                let exit = self.insts.len();
                self.insts[u] = if greedy {
                    Inst::Split(body, exit)
                } else {
                    Inst::Split(exit, body)
                };
            }
            None => {
                // `e{min,}` with min ≥ 1 — emit `min - 1` mandatory copies, then a
                // final body whose loop-back split targets *that body's start* (not a
                // separate entry split): `…copies…; body; U: split(body, exit)`. This
                // is the `regex` crate's `+`/`{n,}` shape; looping to the body start
                // is what makes the extent match for a nullable body — `(a??)+` on
                // "aa" ⇒ `0..0`. (Looping to a separate entry split instead would
                // rank the body's consume alternative above the exit and over-match.)
                for _ in 0..(min - 1) {
                    if self.insts.len() > self.cap {
                        return;
                    }
                    self.emit(inner);
                }
                if self.insts.len() > self.cap {
                    return;
                }
                let body = self.insts.len();
                self.emit(inner);
                if self.insts.len() > self.cap {
                    return;
                }
                let u = self.insts.len();
                self.insts.push(Inst::Split(0, 0)); // patched below
                let exit = self.insts.len();
                self.insts[u] = if greedy {
                    Inst::Split(body, exit)
                } else {
                    Inst::Split(exit, body)
                };
            }
            Some(max) => {
                // Emit `min` mandatory copies, then the optional ones. The cap check
                // bails early so a large `min` cannot spin past the program-size cap.
                for _ in 0..min {
                    if self.insts.len() > self.cap {
                        return;
                    }
                    self.emit(inner);
                }
                // `{min,max}` — (max - min) optional copies. The cap check must be
                // *inside* the loop and before the `Split` push, otherwise a large
                // `max` (e.g. `a{0,4000000000}`) would push billions of `Split`
                // instructions before `compile` ever sees the cap — an OOM DoS.
                let mut split_positions = Vec::new();
                for _ in min..max {
                    if self.insts.len() > self.cap {
                        break;
                    }
                    let split_at = self.insts.len();
                    self.insts.push(Inst::Split(0, 0));
                    split_positions.push(split_at);
                    let body = self.insts.len();
                    self.emit(inner);
                    // Patch: on entry, try body (greedy) or skip to end.
                    self.insts[split_at] = if greedy {
                        Inst::Split(body, 0) // second target patched to end below
                    } else {
                        Inst::Split(0, body)
                    };
                }
                let end = self.insts.len();
                for split_at in split_positions {
                    self.insts[split_at] = match &self.insts[split_at] {
                        Inst::Split(a, _) if greedy => Inst::Split(*a, end),
                        Inst::Split(_, b) => Inst::Split(end, *b),
                        _ => unreachable!(),
                    };
                }
            }
        }
    }
}

/// A priority-ordered set of program counters with O(1) "already added" dedup.
///
/// For `is_match` a thread is just a program counter — no capture slots are
/// tracked. That keeps memory at O(instructions) regardless of how many capture
/// groups the pattern has (a pattern with thousands of groups would otherwise
/// clone a large slot vector per thread per step — an accidental DoS). Capture
/// tracking is reintroduced separately when match *extents* are added.
struct PcList {
    dense: Vec<usize>,
    seen: Vec<u32>,
    generation: u32,
}

impl PcList {
    fn new(n: usize) -> Self {
        PcList {
            dense: Vec::with_capacity(n),
            seen: vec![0; n],
            generation: 0,
        }
    }
    fn clear(&mut self) {
        self.dense.clear();
        self.generation += 1;
    }
    fn contains(&self, pc: usize) -> bool {
        self.seen[pc] == self.generation
    }
    fn mark(&mut self, pc: usize) {
        self.seen[pc] = self.generation;
    }
}

/// A priority-ordered thread list for `find`, where each thread additionally
/// remembers the input position at which its match *started*. Reporting the
/// overall match extent needs the start (the end is simply the position at which
/// a thread reaches `Match`), so a thread carries exactly one extra `usize` — no
/// per-group capture vector, so this stays O(instructions) per step and cannot
/// blow up on a pattern with many groups (that is the [`PcList`] DoS argument,
/// preserved here). Dedup is still by program counter: the first thread to reach
/// a `pc` at a given step has the highest priority, so a later thread reaching
/// the same `pc` (necessarily lower priority, hence a less-preferred start) is
/// correctly dropped.
struct StartThread {
    pc: usize,
    start: usize,
}

struct StartList {
    dense: Vec<StartThread>,
    seen: Vec<u32>,
    generation: u32,
}

impl StartList {
    fn new(n: usize) -> Self {
        StartList {
            dense: Vec::with_capacity(n),
            seen: vec![0; n],
            generation: 0,
        }
    }
    fn clear(&mut self) {
        self.dense.clear();
        self.generation += 1;
    }
    fn contains(&self, pc: usize) -> bool {
        self.seen[pc] == self.generation
    }
    fn mark(&mut self, pc: usize) {
        self.seen[pc] = self.generation;
    }
}

impl Program {
    /// Whether the pattern matches at or after char index `from`. `chars`/
    /// `offsets` describe the input (`offsets[i]` is the byte offset of
    /// `chars[i]`, `offsets[chars.len()]` the byte length — kept for the future
    /// extent-returning path; unused here but cheap).
    fn matches(&self, chars: &[char], from: usize) -> bool {
        let n = self.insts.len();
        let mut clist = PcList::new(n);
        let mut nlist = PcList::new(n);
        let mut matched = false;

        clist.clear();
        let mut pos = from;
        loop {
            // Seed a start thread at this position (unless already matched, or the
            // pattern is start-anchored and we're past the very start).
            if !(matched || (self.anchored_start && pos > 0)) {
                self.add_thread(&mut clist, 0, chars, pos);
            }
            if clist.dense.is_empty() && matched {
                break;
            }

            let cur_char = chars.get(pos).copied();
            nlist.clear();
            let mut i = 0;
            while i < clist.dense.len() {
                let pc = clist.dense[i];
                match &self.insts[pc] {
                    Inst::Char(c) => {
                        if cur_char.is_some_and(|ch| self.char_eq(ch, *c)) {
                            self.add_thread(&mut nlist, pc + 1, chars, pos + 1);
                        }
                    }
                    Inst::Any { dot_all } => {
                        if cur_char.is_some_and(|ch| *dot_all || ch != '\n') {
                            self.add_thread(&mut nlist, pc + 1, chars, pos + 1);
                        }
                    }
                    Inst::Class(class) => {
                        if cur_char.is_some_and(|ch| self.class_matches(class, ch)) {
                            self.add_thread(&mut nlist, pc + 1, chars, pos + 1);
                        }
                    }
                    Inst::Match => {
                        // Highest-priority thread to reach Match wins; lower-priority
                        // threads in this list can't beat it, so stop scanning it.
                        matched = true;
                        break;
                    }
                    // Epsilon instructions were already expanded by add_thread.
                    Inst::Assert(_) | Inst::Split(_, _) | Inst::Jmp(_) => {}
                }
                i += 1;
            }

            std::mem::swap(&mut clist, &mut nlist);
            if pos >= chars.len() {
                break;
            }
            pos += 1;
        }
        matched
    }

    /// Find the leftmost match at or after char index `from`, returning its
    /// `(start, end)` in **char indices** (map to byte offsets via `Input.offsets`).
    ///
    /// Same lockstep Pike-VM scan as [`matches`](Self::matches), but each thread
    /// carries the position at which it started, and instead of a boolean we keep
    /// the best match's `(start, end)`. The leftmost-first priority the existing
    /// scan already implements — process threads in priority order, and on `Match`
    /// stop scanning *lower*-priority threads at this step while letting the
    /// already-advanced *higher*-priority threads run on — is exactly what yields
    /// the correct greedy extent: a greedy quantifier keeps a higher-priority
    /// "match more" thread alive, so when it reaches `Match` at a later position it
    /// overwrites the shorter match. This holds for nullable loops too (e.g.
    /// `(a?)*` on `"aaa"` ⇒ `0..3`): the per-position `seen` dedup stops an
    /// empty-body iteration from re-entering the loop, so the scan terminates while
    /// the longest greedy path still wins.
    fn find(&self, chars: &[char], from: usize) -> Option<(usize, usize)> {
        let n = self.insts.len();
        let mut clist = StartList::new(n);
        let mut nlist = StartList::new(n);
        let mut best: Option<(usize, usize)> = None;

        clist.clear();
        let mut pos = from;
        loop {
            // Seed a start thread at this position, unless we already have a match
            // (a later start is lower priority — leftmost wins) or the pattern is
            // start-anchored and we are past position 0.
            if best.is_none() && !(self.anchored_start && pos > 0) {
                self.add_start_thread(&mut clist, 0, pos, chars, pos);
            }
            if clist.dense.is_empty() && best.is_some() {
                break;
            }

            let cur_char = chars.get(pos).copied();
            nlist.clear();
            let mut i = 0;
            while i < clist.dense.len() {
                let StartThread { pc, start } = clist.dense[i];
                match &self.insts[pc] {
                    Inst::Char(c) => {
                        if cur_char.is_some_and(|ch| self.char_eq(ch, *c)) {
                            self.add_start_thread(&mut nlist, pc + 1, start, chars, pos + 1);
                        }
                    }
                    Inst::Any { dot_all } => {
                        if cur_char.is_some_and(|ch| *dot_all || ch != '\n') {
                            self.add_start_thread(&mut nlist, pc + 1, start, chars, pos + 1);
                        }
                    }
                    Inst::Class(class) => {
                        if cur_char.is_some_and(|ch| self.class_matches(class, ch)) {
                            self.add_start_thread(&mut nlist, pc + 1, start, chars, pos + 1);
                        }
                    }
                    Inst::Match => {
                        // This thread is higher-priority than everything after it in
                        // `clist`, so its match is preferred over theirs: record it
                        // and cut the rest of this step. Higher-priority threads have
                        // already advanced into `nlist` and may still overwrite this.
                        best = Some((start, pos));
                        break;
                    }
                    Inst::Assert(_) | Inst::Split(_, _) | Inst::Jmp(_) => {}
                }
                i += 1;
            }

            std::mem::swap(&mut clist, &mut nlist);
            if pos >= chars.len() {
                break;
            }
            pos += 1;
        }
        best
    }

    /// Epsilon-closure for [`find`](Self::find): identical walk to
    /// [`add_thread`](Self::add_thread) but propagating each thread's `start`
    /// position and parking `StartThread`s.
    fn add_start_thread(
        &self,
        list: &mut StartList,
        pc_start: usize,
        start: usize,
        chars: &[char],
        pos: usize,
    ) {
        let mut stack = vec![pc_start];
        while let Some(pc) = stack.pop() {
            if list.contains(pc) {
                continue;
            }
            list.mark(pc);
            match &self.insts[pc] {
                Inst::Jmp(t) => stack.push(*t),
                Inst::Split(a, b) => {
                    stack.push(*b); // push `b` first so `a` is popped/processed first
                    stack.push(*a);
                }
                Inst::Assert(a) => {
                    if self.assertion_holds(*a, chars, pos) {
                        stack.push(pc + 1);
                    }
                }
                Inst::Char(_) | Inst::Any { .. } | Inst::Class(_) | Inst::Match => {
                    list.dense.push(StartThread { pc, start });
                }
            }
        }
    }

    /// Follow epsilon transitions (Save/Split/Jmp/Assert) from `pc`, adding the
    /// reachable Char/Any/Class/Match instructions to `list` in priority order.
    ///
    /// Implemented with an explicit stack rather than recursion so a pathological
    /// pattern with a long chain of epsilon transitions cannot overflow the call
    /// stack (a DoS on arbitrary `re:` input). To preserve depth-first priority
    /// order (try a Split's first target before its second), the second target is
    /// pushed before the first, so the first is popped and processed earlier.
    fn add_thread(&self, list: &mut PcList, start: usize, chars: &[char], pos: usize) {
        let mut stack = vec![start];
        while let Some(pc) = stack.pop() {
            if list.contains(pc) {
                continue;
            }
            list.mark(pc);
            match &self.insts[pc] {
                Inst::Jmp(t) => stack.push(*t),
                Inst::Split(a, b) => {
                    stack.push(*b); // push `b` first so `a` is popped first
                    stack.push(*a);
                }
                Inst::Assert(a) => {
                    if self.assertion_holds(*a, chars, pos) {
                        stack.push(pc + 1);
                    }
                }
                // A consuming instruction or Match: park it in the list.
                Inst::Char(_) | Inst::Any { .. } | Inst::Class(_) | Inst::Match => {
                    list.dense.push(pc);
                }
            }
        }
    }

    fn assertion_holds(&self, a: Assertion, chars: &[char], pos: usize) -> bool {
        match a {
            Assertion::StartText => pos == 0,
            Assertion::EndText => pos == chars.len(),
            Assertion::WordBoundary => self.word_boundary(chars, pos),
            Assertion::NotWordBoundary => !self.word_boundary(chars, pos),
        }
    }

    fn word_boundary(&self, chars: &[char], pos: usize) -> bool {
        let before = pos.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let after = chars.get(pos).copied();
        self.is_word(before) != self.is_word(after)
    }

    /// Whether `c` is a "word" character for `\b`: the Unicode word set in
    /// Unicode mode, else ASCII `[0-9A-Za-z_]`.
    fn is_word(&self, c: Option<char>) -> bool {
        match c {
            None => false,
            Some(ch) if self.unicode => in_table(crate::unicode_tables::WORD, ch),
            Some(ch) => ch.is_ascii_alphanumeric() || ch == '_',
        }
    }

    fn char_eq(&self, input: char, pat: char) -> bool {
        if input == pat {
            return true;
        }
        if !self.case_insensitive {
            return false;
        }
        // Case-insensitive: Unicode simple case folding in Unicode mode, else
        // ASCII case folding (matching the `regex` crate's `(?i)` vs `(?i-u)`).
        if self.unicode {
            crate::casefold::fold_eq(input, pat)
        } else {
            input.eq_ignore_ascii_case(&pat)
        }
    }

    fn class_matches(&self, class: &Class, input: char) -> bool {
        let mut hit = class_contains(class, input);
        if !hit && self.case_insensitive {
            if self.unicode {
                // The class matches if any case-fold mate of `input` is in it
                // (the `regex` crate folds the class under `(?i)`).
                if let Some(orbit) = crate::casefold::orbit(input) {
                    hit = orbit.iter().any(|&m| class_contains(class, m));
                }
            } else {
                hit = class_contains(class, swap_ascii_case(input));
            }
        }
        hit != class.negated
    }
}

/// Binary-search membership over a class's sorted, merged ranges (built by
/// `ast::make_class`), so Unicode classes with hundreds of ranges stay fast.
fn class_contains(class: &Class, c: char) -> bool {
    class
        .ranges
        .binary_search_by(|r| {
            if c < r.start {
                std::cmp::Ordering::Greater
            } else if c > r.end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Binary-search membership over a sorted `(u32, u32)` range table.
fn in_table(table: &[(u32, u32)], c: char) -> bool {
    let cp = c as u32;
    table
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                std::cmp::Ordering::Greater
            } else if cp > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn swap_ascii_case(c: char) -> char {
    if c.is_ascii_uppercase() {
        c.to_ascii_lowercase()
    } else if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

// --- Public surface used by `lib.rs` ----------------------------------------

/// A prepared input: the characters of the text plus, for each character, the
/// byte offset at which it starts. `offsets[i]` is the byte offset of `chars[i]`,
/// and `offsets[chars.len()]` is the total byte length — so a match spanning
/// character indices `s..e` maps to byte range `offsets[s]..offsets[e]`, which is
/// what callers (and the `regex` crate) report.
pub(crate) struct Input {
    pub chars: Vec<char>,
    pub offsets: Vec<usize>,
}

impl Input {
    pub fn new(text: &str) -> Self {
        let mut chars = Vec::new();
        let mut offsets = Vec::new();
        for (byte, ch) in text.char_indices() {
            chars.push(ch);
            offsets.push(byte);
        }
        offsets.push(text.len()); // sentinel: byte length, = end of the last char
        Input { chars, offsets }
    }
}

impl Program {
    /// Whether the pattern matches `input` at or after char index `from`.
    pub(crate) fn is_match_from(&self, input: &Input, from: usize) -> bool {
        self.matches(&input.chars, from)
    }

    /// The leftmost match at or after char index `from`, as a **byte** range
    /// `start..end` into the original text (via `input.offsets`), or `None`.
    pub(crate) fn find_from(&self, input: &Input, from: usize) -> Option<(usize, usize)> {
        self.find(&input.chars, from)
            .map(|(s, e)| (input.offsets[s], input.offsets[e]))
    }
}

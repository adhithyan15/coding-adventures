//! # Compile-time resource budgets (#18474)
//!
//! Twig source is untrusted wherever a user hands `twig-vm` a file, so the
//! compiler must bound the work a program can make it do, and must refuse
//! a program its target VM could never run *before* that program reaches a
//! backend.  Until #18474 this crate had neither: a 25 KB record definition
//! made it emit two million instructions, and a `main` with more registers
//! than a VM frame holds compiled cleanly and failed only at run time.
//!
//! Every budget here is a fixed constant.  A breach is an ordinary
//! [`TwigCompileError`] whose message starts with the budget's name, so it
//! reads like this:
//!
//! ```text
//! TwigCompileError at 65537:1: registers per function exceeds the limit of 65536 in function `main`
//! ```
//!
//! ## The budgets, and where each one is enforced
//!
//! ```text
//!   source ──► [source bytes] ──► parse_to_ast ──► CST
//!   CST ──► [CST items] ──► check_cst [depth 256] ──► lowering
//!   each finished IIRFunction ──► [registers per function]
//!                              ──► [IIR instructions per function]
//!                              ──► [IIR instructions per module] ──► IIRModule
//! ```
//!
//! | Budget                      | Limit       | Counted as                                         | Why this number |
//! |-----------------------------|-------------|----------------------------------------------------|-----------------|
//! | source bytes                | 1 MiB       | `source.len()`                                     | the trusted ceiling Python and Ruby use (LANG84 C5); the largest Twig file in the repo, `twig/compiler/cst-parser.tw`, is 29 KB |
//! | CST items                   | 2²⁰         | every node and token of the tree, the root included | ordinary Twig is about 0.6 items per byte (`cst-parser.tw`: 17,142 items), so a 1 MiB program of it fits; the 16,384 the JavaScript and Perl pilots use would refuse that one 29 KB file. Large enough that the register budget below can be reached, so that budget is a live limit, not a dead one |
//! | tree depth                  | 256         | (already enforced by [`crate::check_cst`] and [`crate::compiler::MAX_COMPILE_DEPTH`]) | unchanged |
//! | registers per function      | 2¹⁶ = 65,536 | parameters + distinct non-parameter `dest`s       | the frame limit of both VMs Twig runs on (see below) |
//! | IIR instructions per function | 2¹⁸ = 262,144 | `instructions.len()`                            | four instructions per register at the register limit, while a branchy `(if x x x)` spends six per register |
//! | IIR instructions per module | 2²⁰ = 1,048,576 | the sum over every function                    | twig-vm profiles at most 2²⁰ instruction slots (`MAX_PROFILED_INSTRUCTION_SLOTS`); the other frontends' cap is 1,000,000 (LANG84 C5) |
//! | IIR instructions per linked program | 2²⁰ (the same constant) | the sum over every module of a `twig-module-driver` graph | the linker merges the graph into the one module the VM runs, so it gets one module's budget ([`check_linked_instructions`]) |
//! | functions per module, and per linked program | 2¹⁶ | functions admitted | twig-vm profiles and caches at most 2¹⁶ distinct functions a run (`MAX_PROFILED_FUNCTIONS`, `MAX_IC_FUNCTIONS`) |
//! | name bytes | 256 | each `NAME` token: in `twig_parser::parse_to_ast`, and in [`check_cst_items`] for a hand-built tree | see `twig_parser::budgets` |
//! | generated name bytes | 1 MiB | record and union predicate/accessor names, by arithmetic before any is formatted: here in the pre-pass, in `twig_parser::extract_program` and in the type checker | a long record name × many fields was ≈ 30 GB of names before any other budget counted (#18474 security review) |
//! | source bytes per program | 4 MiB | `twig-module-driver`, summed as each file is read | [`MAX_PROGRAM_SOURCE_BYTES`] |
//! | imported names per program | 2²⁰ | `twig-module-driver`, summed over import edges before merging | [`MAX_IMPORTED_NAMES`] |
//! | imported name bytes per program | 16 MiB | the same edges, charged each exported name's length: every name is cloned into the importer | [`MAX_IMPORTED_NAME_BYTES`] |
//!
//! ## Registers: the stricter of two VM frames
//!
//! Twig modules run on two VMs, and each refuses an oversized frame:
//!
//! | VM      | Constant                                    | Counts                                          |
//! |---------|---------------------------------------------|-------------------------------------------------|
//! | twig-vm | `dispatch::MAX_REGISTERS_PER_FRAME` = 2¹⁶   | distinct register names written in the frame     |
//! | vm-core | `VMFrame::MAX_FRAME_REGISTERS` = 2¹⁶        | `params.len()` + distinct non-parameter `dest`s  |
//!
//! Neither VM is a dependency of this crate (twig-vm depends on *us*, and
//! vm-core is not in twig-vm's graph), so the number is written here once
//! and pinned by tests on both sides: `twig-vm`'s own test suite asserts
//! its frame constant equals [`MAX_REGISTERS_PER_FUNCTION`] and runs a
//! function of exactly that many registers, and this crate's
//! `tests/budgets.rs` (with vm-core as a dev-dependency) puts every
//! function of the at-limit module through `VMFrame::check_register_limit`.
//!
//! The count is vm-core's, because it is the stricter one: a parameter
//! is a *positional* slot, so `(define (f a a) …)` uses two registers even
//! if both had one name (lesson: *a register cap that counts distinct
//! names must also count positional parameter slots*).  This compiler
//! renames a repeated parameter anyway, so for its own output the two
//! counts agree.
//!
//! ## Why the function checks run on the finished function
//!
//! Lowering one function is linear in the CST it walks, and the CST is
//! already bounded, so checking a function once it is complete costs at
//! most one function's worth of wasted work.  What is *not* linear is
//! the number of functions: a `(record …)` of *n* fields emits *n*
//! accessors, the *i*-th walking *i* `cdr`s, so ≈ n²/2 instructions from
//! O(n) source.  The module total is therefore checked as each function
//! is admitted, which stops a hostile record after at most
//! [`MAX_IIR_INSTRUCTIONS_PER_MODULE`] + n instructions of work.
//!
//! ## Where an error points
//!
//! A refusal names the function and points at the source of the
//! instruction that went over: the instruction that wrote the
//! 65,537th register, or the 262,145th instruction.  The compiler
//! writes some instructions itself (`main`'s `nil` epilogue, record
//! accessors) and tags them `SourceLoc::SYNTHETIC`; for those the error
//! walks back to the nearest instruction with a real position, and failing
//! that uses the declaration's position (`(record …)`, `(define …)`).

use std::collections::HashSet;

use interpreter_ir::{IIRFunction, SourceLoc};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::errors::TwigCompileError;

/// Bytes of Twig source [`crate::compile_source`] and
/// [`crate::compile_typed_source`] accept: 1 MiB.
pub const MAX_SOURCE_BYTES: usize = 1 << 20;

/// Nodes and tokens, the root included, of a tree any `compile_ast*` entry
/// point or [`crate::Compiler::compile`] accepts: 2²⁰.
pub const MAX_CST_ITEMS: usize = 1 << 20;

/// Registers one lowered function may use: 2¹⁶, the frame limit of both
/// twig-vm (`MAX_REGISTERS_PER_FRAME`) and vm-core
/// (`VMFrame::MAX_FRAME_REGISTERS`), counted as vm-core counts them.
pub const MAX_REGISTERS_PER_FUNCTION: usize = 1 << 16;

/// Instructions one lowered function may hold: 2¹⁸.
pub const MAX_IIR_INSTRUCTIONS_PER_FUNCTION: usize = 1 << 18;

/// Instructions a whole module may hold, summed over its functions: 2²⁰.
pub const MAX_IIR_INSTRUCTIONS_PER_MODULE: usize = 1 << 20;

/// Functions one module may hold: 2¹⁶, twig-vm's `MAX_PROFILED_FUNCTIONS`
/// and `MAX_IC_FUNCTIONS`.  twig-vm refuses to profile or cache a
/// 65,537th distinct function in a run; a module (or a linked program)
/// with more is refused here instead, at compile time.
pub const MAX_FUNCTIONS_PER_MODULE: usize = 1 << 16;

/// Source bytes a whole `twig-module-driver` import graph may hold: 4 MiB.
///
/// Each file is held to [`MAX_SOURCE_BYTES`], but a graph may hold 1,000
/// files, and the driver reads, parses, extracts and type-checks all of
/// them before it compiles one.  The graph links into *one* module, held to
/// one module's [`MAX_IIR_INSTRUCTIONS_PER_MODULE`]; the densest Twig spends
/// one instruction per two source bytes (`1` on a line), so past about
/// 2 MiB of such code a graph could not link anyway.  4 MiB is that, with
/// room for comments and sparser code, and it bounds the driver's
/// pre-compile work at four maximum-size files instead of a thousand.
pub const MAX_PROGRAM_SOURCE_BYTES: usize = 4 << 20;

/// Imported names a `twig-module-driver` graph may merge, summed over every
/// import edge: 2²⁰.  Each edge copies the imported module's export list
/// into the importer's type-checker globals and its compiler externs, so a
/// graph where many modules import one module with many exports does
/// edges × exports work; this bounds it at one module's worth of
/// instructions.
pub const MAX_IMPORTED_NAMES: usize = 1 << 20;

/// Bytes of imported names a `twig-module-driver` graph may merge, summed
/// over every import edge: 16 MiB.  [`MAX_IMPORTED_NAMES`] counts names, but
/// each one is *cloned* into the importer's type-checker globals, so the
/// cost is names × name length, and a name may be 256 bytes.  16 MiB is the
/// names budget at a 16-byte average name (the repo's Twig averages about
/// 10), so ordinary graphs reach the name count first.
pub const MAX_IMPORTED_NAME_BYTES: usize = 16 << 20;

pub use twig_parser::{MAX_GENERATED_NAME_BYTES, MAX_NAME_BYTES, MAX_TOKENS};

/// A budget error.  `which` is the budget's name as the table above
/// spells it; `function` names the function for the per-function budgets.
fn breach(
    which: &str,
    limit: usize,
    function: Option<&str>,
    (line, column): (usize, usize),
) -> TwigCompileError {
    let in_function = function.map(|name| format!(" in function `{name}`")).unwrap_or_default();
    TwigCompileError {
        message: format!("{which} exceeds the limit of {limit}{in_function}"),
        line,
        column,
    }
}

/// Refuse source longer than [`MAX_SOURCE_BYTES`] before the lexer sees
/// it.  The error points at 1:1: the source as a whole is the problem.
///
/// Public for callers that parse Twig themselves before calling a
/// `compile_ast*` entry point, as `twig-module-driver` does for every file
/// of an import graph.
pub fn check_source(source: &str) -> Result<(), TwigCompileError> {
    check_source_len(source.len() as u64)
}

/// [`check_source`] for a length known before the text is read: a file's
/// `metadata().len()`, or the bytes a bounded reader returned.  A `u64`, so
/// a file larger than `usize::MAX` on a 32-bit host is still refused, not
/// truncated.
pub fn check_source_len(len: u64) -> Result<(), TwigCompileError> {
    if len > MAX_SOURCE_BYTES as u64 {
        return Err(breach("source bytes", MAX_SOURCE_BYTES, None, (1, 1)));
    }
    Ok(())
}

/// Refuse an import graph whose source, summed over every file read so far,
/// exceeds [`MAX_PROGRAM_SOURCE_BYTES`].
pub fn check_graph_source_bytes(total: u64) -> Result<(), TwigCompileError> {
    if total > MAX_PROGRAM_SOURCE_BYTES as u64 {
        return Err(breach("source bytes per program", MAX_PROGRAM_SOURCE_BYTES, None, (1, 1)));
    }
    Ok(())
}

/// Refuse an import graph whose import edges, summed, would merge more than
/// [`MAX_IMPORTED_NAMES`] exported names.
pub fn check_imported_names(total: usize) -> Result<(), TwigCompileError> {
    if total > MAX_IMPORTED_NAMES {
        return Err(breach("imported names per program", MAX_IMPORTED_NAMES, None, (1, 1)));
    }
    Ok(())
}

/// Refuse an import graph whose import edges, summed, would clone more than
/// [`MAX_IMPORTED_NAME_BYTES`] bytes of exported names.
pub fn check_imported_name_bytes(total: usize) -> Result<(), TwigCompileError> {
    if total > MAX_IMPORTED_NAME_BYTES {
        return Err(breach("imported name bytes per program", MAX_IMPORTED_NAME_BYTES, None, (1, 1)));
    }
    Ok(())
}

/// Refuse a program linked from several modules whose instructions,
/// summed over every module, exceed [`MAX_IIR_INSTRUCTIONS_PER_MODULE`].
///
/// Each module is already under that budget on its own, but a
/// `twig-module-driver` graph may hold up to 1,000 modules, and the linker
/// merges them into the *one* module the VM runs and profiles. So the
/// linked program gets the same budget as any single module: the limit
/// that matters downstream (twig-vm's 2²⁰ profile slots) is per module the
/// VM sees, not per source file. The driver calls this with the running
/// total as each module is compiled, so a graph is refused as soon as it
/// goes over, without compiling the rest.
pub fn check_linked_instructions(total: usize) -> Result<(), TwigCompileError> {
    if total > MAX_IIR_INSTRUCTIONS_PER_MODULE {
        return Err(breach(
            "IIR instructions per linked program",
            MAX_IIR_INSTRUCTIONS_PER_MODULE,
            None,
            (1, 1),
        ));
    }
    Ok(())
}

/// [`check_linked_instructions`]' twin for [`MAX_FUNCTIONS_PER_MODULE`]: a
/// linked program is one module to twig-vm, so it gets one module's
/// function budget.
pub fn check_linked_functions(total: usize) -> Result<(), TwigCompileError> {
    if total > MAX_FUNCTIONS_PER_MODULE {
        return Err(breach("functions per linked program", MAX_FUNCTIONS_PER_MODULE, None, (1, 1)));
    }
    Ok(())
}

/// Count the tree's items without recursion, and stop at the first item
/// past [`MAX_CST_ITEMS`].  The same walk refuses a `NAME` token longer than
/// [`MAX_NAME_BYTES`]: a tree from `twig_parser::parse_to_ast` had its
/// names checked there, but a hand-built one did not, and this walk visits
/// every token anyway.
///
/// The count is taken *before* a child node is queued, so a hostile tree
/// costs at most the budget's worth of work however wide or deep it is
/// (lesson: *depth limits do not bound breadth*).  The error points at the
/// node whose child went over.
///
/// Public so a caller that parses Twig itself can refuse an oversized tree
/// before doing anything else with it: `twig-module-driver` runs it right
/// after `parse_to_ast`, before extracting, type-checking or keeping the
/// tree.
pub fn check_cst_items(root: &GrammarASTNode) -> Result<(), TwigCompileError> {
    let mut pending = vec![root];
    let mut items = 1_usize;
    while let Some(node) = pending.pop() {
        for child in &node.children {
            if items == MAX_CST_ITEMS {
                return Err(breach("CST items", MAX_CST_ITEMS, None, crate::cst::pos(node)));
            }
            items += 1;
            match child {
                ASTNodeOrToken::Node(inner) => pending.push(inner),
                ASTNodeOrToken::Token(token) => twig_parser::check_name_token(token)?,
            }
        }
    }
    Ok(())
}

/// The running total for [`MAX_IIR_INSTRUCTIONS_PER_MODULE`]; one per
/// [`crate::Compiler`].
#[derive(Debug, Default)]
pub(crate) struct ModuleBudget {
    instructions: usize,
    functions: usize,
}

impl ModuleBudget {
    /// Check one finished function against the per-function budgets and
    /// add it to the module total.  `declared` is the position of the
    /// source form the function came from, used when none of its own
    /// instructions has one.
    pub(crate) fn admit(
        &mut self,
        function: &IIRFunction,
        declared: (usize, usize),
    ) -> Result<(), TwigCompileError> {
        let name = Some(function.name.as_str());

        // ── Registers, counted as vm-core counts them ─────────────────
        //
        //     registers = params.len() + |{ dest : dest ∉ params }|
        //
        // Parameters first: they are positional slots, filled before the
        // first instruction runs, so too many of them is the declaration's
        // fault.
        let limit = MAX_REGISTERS_PER_FUNCTION;
        if function.params.len() > limit {
            return Err(breach("registers per function", limit, name, declared));
        }
        let params: HashSet<&str> = function.params.iter().map(|(p, _)| p.as_str()).collect();
        let mut dests: HashSet<&str> = HashSet::new();
        for (index, instr) in function.instructions.iter().enumerate() {
            let Some(dest) = instr.dest.as_deref() else { continue };
            if !params.contains(dest)
                && dests.insert(dest)
                && function.params.len() + dests.len() > limit
            {
                let at = position(function, index, declared);
                return Err(breach("registers per function", limit, name, at));
            }
        }

        // ── Instructions in this function ─────────────────────────────
        let limit = MAX_IIR_INSTRUCTIONS_PER_FUNCTION;
        if function.instructions.len() > limit {
            let at = position(function, limit, declared);
            return Err(breach("IIR instructions per function", limit, name, at));
        }

        // ── Functions in the module so far ────────────────────────────
        //
        // Checked before the instruction total so a breach of both (a
        // union of 40,000 empty variants) names the budget that twig-vm
        // would have refused first.
        if self.functions == MAX_FUNCTIONS_PER_MODULE {
            return Err(breach("functions per module", MAX_FUNCTIONS_PER_MODULE, name, declared));
        }

        // ── Instructions in the module so far ─────────────────────────
        let limit = MAX_IIR_INSTRUCTIONS_PER_MODULE;
        match self.instructions.checked_add(function.instructions.len()) {
            Some(total) if total <= limit => {
                self.instructions = total;
                self.functions += 1;
                Ok(())
            }
            _ => Err(breach(
                "IIR instructions per module",
                limit,
                name,
                position(function, function.instructions.len().saturating_sub(1), declared),
            )),
        }
    }
}

/// The source position to blame for `function.instructions[index]`: its
/// own position, or the nearest earlier real one when the compiler wrote
/// it (`SourceLoc::SYNTHETIC`), or `declared` when there is none.
fn position(function: &IIRFunction, index: usize, declared: (usize, usize)) -> (usize, usize) {
    let end = index.saturating_add(1).min(function.source_map.len());
    function.source_map[..end]
        .iter()
        .rev()
        .find(|loc| **loc != SourceLoc::SYNTHETIC)
        .map(|loc| (loc.line as usize, loc.column as usize))
        .unwrap_or(declared)
}

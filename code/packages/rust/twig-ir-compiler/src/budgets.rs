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
    if source.len() > MAX_SOURCE_BYTES {
        return Err(breach("source bytes", MAX_SOURCE_BYTES, None, (1, 1)));
    }
    Ok(())
}

/// Count the tree's items without recursion, and stop at the first item
/// past [`MAX_CST_ITEMS`].
///
/// The count is taken *before* a child node is queued, so a hostile tree
/// costs at most the budget's worth of work however wide or deep it is
/// (lesson: *depth limits do not bound breadth*).  The error points at the
/// node whose child went over.
pub(crate) fn check_cst_items(root: &GrammarASTNode) -> Result<(), TwigCompileError> {
    let mut pending = vec![root];
    let mut items = 1_usize;
    while let Some(node) = pending.pop() {
        for child in &node.children {
            if items == MAX_CST_ITEMS {
                return Err(breach("CST items", MAX_CST_ITEMS, None, crate::cst::pos(node)));
            }
            items += 1;
            if let ASTNodeOrToken::Node(inner) = child {
                pending.push(inner);
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

        // ── Instructions in the module so far ─────────────────────────
        let limit = MAX_IIR_INSTRUCTIONS_PER_MODULE;
        match self.instructions.checked_add(function.instructions.len()) {
            Some(total) if total <= limit => {
                self.instructions = total;
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

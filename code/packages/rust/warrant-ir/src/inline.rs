//! # WIR call inlining (VER04h section 7)
//!
//! The symbolic executor refuses `CallFunction` ("Uninlined function call in
//! symbolic mode"), so a public export that delegates to a helper (`ct_eq` ->
//! `ct_eq_equal_length`) could previously only be proved by proving the helper.
//! That proves a different object than the one a caller links against.  This
//! pass splices every callee body into its caller so the *export itself* is
//! what the provers analyse.
//!
//! ## The transformation
//!
//! For a call `r = f(e1, e2)` in block `B` at instruction index `i`:
//!
//! ```text
//!   B:  pre...           B:  pre...
//!       r = f(e1, e2)        f$k_a = e1; f$k_b = e2       (parameter binding)
//!       post...        =>    jump f$k_entry
//!       terminator
//!                        f$k blocks (variables prefixed, ids offset):
//!                            ... return v      becomes   r = v; jump C
//!                        C:  post...; terminator
//! ```
//!
//! Callee variables are prefixed `inl<k>_` so two inlined copies of the same
//! function (or a callee variable that happens to share a name with a caller
//! variable) cannot collide.  Block ids of the callee are shifted past every
//! id already in use.  Back edges that targeted `B` still target `B`: they
//! re-run `pre` and the call, which is exactly what a loop around a call means.
//!
//! The pass terminates because the number of inlinings is capped; a call graph
//! that needs more (for example direct recursion) is reported as an error
//! rather than being partially expanded.  Calls to functions that are not in
//! the program are left in place (and the symbolic executor will then refuse
//! them, loudly).

use std::collections::HashMap;

use crate::ast::{
    BasicBlock, BlockId, Function, Label, Program, VarId, WirExpr, WirInst, WirTerminator,
};

/// Upper bound on spliced call sites for one function.
pub const MAX_INLINES: usize = 64;

/// Why inlining stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineError {
    /// The requested root function is not in the program.
    UnknownFunction(String),
    /// More than [`MAX_INLINES`] call sites would be expanded (recursion).
    TooManyInlines(String),
}

impl std::fmt::Display for InlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InlineError::UnknownFunction(n) => write!(f, "unknown function `{n}`"),
            InlineError::TooManyInlines(n) => {
                write!(f, "more than {MAX_INLINES} calls to expand in `{n}` (recursion?)")
            }
        }
    }
}

impl std::error::Error for InlineError {}

fn rename(v: &VarId, prefix: &str) -> VarId {
    VarId(format!("{prefix}{}", v.0))
}

fn rename_expr(e: &WirExpr, p: &str) -> WirExpr {
    let r = |x: &WirExpr| Box::new(rename_expr(x, p));
    match e {
        WirExpr::Var(v) => WirExpr::Var(rename(v, p)),
        WirExpr::Unary { op, expr } => WirExpr::Unary { op: *op, expr: r(expr) },
        WirExpr::Binary { op, lhs, rhs } => WirExpr::Binary {
            op: *op,
            lhs: r(lhs),
            rhs: r(rhs),
        },
        WirExpr::Trunc { expr, target_ty } => WirExpr::Trunc {
            expr: r(expr),
            target_ty: target_ty.clone(),
        },
        WirExpr::ZExt { expr, target_ty } => WirExpr::ZExt {
            expr: r(expr),
            target_ty: target_ty.clone(),
        },
        WirExpr::SExt { expr, target_ty } => WirExpr::SExt {
            expr: r(expr),
            target_ty: target_ty.clone(),
        },
        WirExpr::TupleMake(items) => {
            WirExpr::TupleMake(items.iter().map(|x| rename_expr(x, p)).collect())
        }
        WirExpr::TupleGet { expr, index } => WirExpr::TupleGet {
            expr: r(expr),
            index: *index,
        },
        WirExpr::PtrOffset { ptr, offset_bytes } => WirExpr::PtrOffset {
            ptr: r(ptr),
            offset_bytes: r(offset_bytes),
        },
        WirExpr::PtrLen(x) => WirExpr::PtrLen(r(x)),
        WirExpr::MakeDynamic { expr, tag } => WirExpr::MakeDynamic {
            expr: r(expr),
            tag: tag.clone(),
        },
        WirExpr::CheckDynamic { expr, expected_tag } => WirExpr::CheckDynamic {
            expr: r(expr),
            expected_tag: expected_tag.clone(),
        },
        WirExpr::ConstInt { .. }
        | WirExpr::ConstBool(_)
        | WirExpr::ConstBytes(_)
        | WirExpr::ConstUnit => e.clone(),
    }
}

fn rename_inst(i: &WirInst, p: &str) -> WirInst {
    let re = |x: &WirExpr| rename_expr(x, p);
    match i {
        WirInst::Let { var, ty, label, expr } => WirInst::Let {
            var: rename(var, p),
            ty: ty.clone(),
            label: *label,
            expr: re(expr),
        },
        WirInst::Alloc { var, size_expr } => WirInst::Alloc {
            var: rename(var, p),
            size_expr: re(size_expr),
        },
        WirInst::Load { var, ptr_expr, ty } => WirInst::Load {
            var: rename(var, p),
            ptr_expr: re(ptr_expr),
            ty: ty.clone(),
        },
        WirInst::Store { ptr_expr, val_expr } => WirInst::Store {
            ptr_expr: re(ptr_expr),
            val_expr: re(val_expr),
        },
        WirInst::Free { ptr_expr } => WirInst::Free { ptr_expr: re(ptr_expr) },
        WirInst::CallCapability { var, capability, args } => WirInst::CallCapability {
            var: var.as_ref().map(|v| rename(v, p)),
            capability: capability.clone(),
            args: args.iter().map(re).collect(),
        },
        WirInst::CallFunction { var, func_name, args } => WirInst::CallFunction {
            var: var.as_ref().map(|v| rename(v, p)),
            func_name: func_name.clone(),
            args: args.iter().map(re).collect(),
        },
    }
}

/// Return a copy of `root` with every call to a function of `program` expanded.
pub fn inline_calls(program: &Program, root: &str) -> Result<Function, InlineError> {
    let mut func = program
        .find_function(root)
        .ok_or_else(|| InlineError::UnknownFunction(root.to_string()))?
        .clone();
    let known: HashMap<&str, &Function> =
        program.functions.iter().map(|f| (f.name.as_str(), f)).collect();

    for k in 0..=MAX_INLINES {
        // Find the first expandable call site.
        let site = func.blocks.iter().enumerate().find_map(|(bi, b)| {
            b.instructions.iter().enumerate().find_map(|(ii, inst)| match inst {
                WirInst::CallFunction { func_name, .. } if known.contains_key(func_name.as_str()) => {
                    Some((bi, ii))
                }
                _ => None,
            })
        });
        let Some((bi, ii)) = site else {
            return Ok(func);
        };
        if k == MAX_INLINES {
            return Err(InlineError::TooManyInlines(root.to_string()));
        }
        let WirInst::CallFunction { var, func_name, args } = func.blocks[bi].instructions[ii].clone()
        else {
            unreachable!("site was selected as a CallFunction");
        };
        let callee = known[func_name.as_str()];
        let prefix = format!("inl{k}_");

        let max_id = func.blocks.iter().map(|b| b.id.0).max().unwrap_or(0);
        let base = max_id + 1;
        let remap = |id: BlockId| BlockId(base + id.0);
        let callee_max = callee.blocks.iter().map(|b| b.id.0).max().unwrap_or(0);
        let cont_id = BlockId(base + callee_max + 1);
        let entry = callee.blocks.first().map(|b| remap(b.id)).unwrap_or(cont_id);

        // Split the caller block.
        let block = func.blocks[bi].clone();
        let mut head = block.instructions[..ii].to_vec();
        for ((pvar, pty, _plabel), arg) in callee.params.iter().zip(args.iter()) {
            head.push(WirInst::Let {
                var: rename(pvar, &prefix),
                ty: pty.clone(),
                label: Label::PUBLIC,
                expr: arg.clone(),
            });
        }
        func.blocks[bi] = BasicBlock {
            id: block.id,
            instructions: head,
            terminator: WirTerminator::Jump(entry),
        };

        let mut spliced: Vec<BasicBlock> = Vec::new();
        for cb in &callee.blocks {
            let mut insts: Vec<WirInst> =
                cb.instructions.iter().map(|i| rename_inst(i, &prefix)).collect();
            let terminator = match &cb.terminator {
                WirTerminator::Return(value) => {
                    if let (Some(result), Some(expr)) = (&var, value) {
                        insts.push(WirInst::Let {
                            var: result.clone(),
                            ty: callee.return_type.clone(),
                            label: Label::PUBLIC,
                            expr: rename_expr(expr, &prefix),
                        });
                    }
                    WirTerminator::Jump(cont_id)
                }
                WirTerminator::Jump(t) => WirTerminator::Jump(remap(*t)),
                WirTerminator::Branch { cond, then_block, else_block } => WirTerminator::Branch {
                    cond: rename_expr(cond, &prefix),
                    then_block: remap(*then_block),
                    else_block: remap(*else_block),
                },
                other => other.clone(),
            };
            spliced.push(BasicBlock {
                id: remap(cb.id),
                instructions: insts,
                terminator,
            });
        }
        spliced.push(BasicBlock {
            id: cont_id,
            instructions: block.instructions[ii + 1..].to_vec(),
            terminator: block.terminator.clone(),
        });
        func.blocks.extend(spliced);
    }
    Err(InlineError::TooManyInlines(root.to_string()))
}

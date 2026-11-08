//! Static type checker and well-formedness validator for WIR programs.

use std::collections::{HashMap, HashSet};

use crate::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    DuplicateFunction(String),
    FunctionNotFound(String),
    EmptyFunction(String),
    DuplicateBlock(BlockId),
    BlockNotFound(BlockId),
    UndefinedVariable(VarId),
    VariableRedefined(VarId),
    TypeMismatch {
        expected: WirType,
        actual: WirType,
    },
    ExpectedInteger(WirType),
    ExpectedPointer(WirType),
    ExpectedTuple(WirType),
    TupleIndexOutOfBounds {
        index: usize,
        tuple_len: usize,
    },
    InvalidTruncation {
        from: WirType,
        to: WirType,
    },
    InvalidExtension {
        from: WirType,
        to: WirType,
    },
    BranchConditionNotBool(WirType),
    SecretDependentBranch(VarId),
    ArgumentCountMismatch {
        expected: usize,
        actual: usize,
    },
}

pub struct TypeChecker<'a> {
    program: &'a Program,
}

impl<'a> TypeChecker<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self { program }
    }

    pub fn check_program(&self) -> Result<(), TypeError> {
        let mut seen_fn_names = HashSet::new();
        for func in &self.program.functions {
            if !seen_fn_names.insert(&func.name) {
                return Err(TypeError::DuplicateFunction(func.name.clone()));
            }
        }

        for func in &self.program.functions {
            self.check_function(func)?;
        }
        Ok(())
    }

    pub fn check_function(&self, func: &Function) -> Result<(), TypeError> {
        if func.blocks.is_empty() {
            return Err(TypeError::EmptyFunction(func.name.clone()));
        }

        let mut block_ids = HashSet::new();
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                return Err(TypeError::DuplicateBlock(block.id));
            }
        }

        let mut env: HashMap<VarId, (WirType, Label)> = HashMap::new();
        for (param_var, param_ty, param_label) in &func.params {
            if env.contains_key(param_var) {
                return Err(TypeError::VariableRedefined(param_var.clone()));
            }
            env.insert(param_var.clone(), (param_ty.clone(), *param_label));
        }

        for block in &func.blocks {
            self.check_block(func, block, &mut env, &block_ids)?;
        }

        Ok(())
    }

    fn check_block(
        &self,
        func: &Function,
        block: &BasicBlock,
        env: &mut HashMap<VarId, (WirType, Label)>,
        block_ids: &HashSet<BlockId>,
    ) -> Result<(), TypeError> {
        for inst in &block.instructions {
            match inst {
                WirInst::Let { var, ty, label, expr } => {
                    if env.contains_key(var) {
                        return Err(TypeError::VariableRedefined(var.clone()));
                    }
                    let (expr_ty, expr_label) = self.check_expr(expr, env)?;
                    if &expr_ty != ty {
                        return Err(TypeError::TypeMismatch {
                            expected: ty.clone(),
                            actual: expr_ty,
                        });
                    }
                    let combined_label = label.join(expr_label);
                    env.insert(var.clone(), (ty.clone(), combined_label));
                }
                WirInst::Alloc { var, size_expr } => {
                    if env.contains_key(var) {
                        return Err(TypeError::VariableRedefined(var.clone()));
                    }
                    let (size_ty, size_label) = self.check_expr(size_expr, env)?;
                    if !size_ty.is_integer() {
                        return Err(TypeError::ExpectedInteger(size_ty));
                    }
                    let ptr_ty = WirType::Ptr(Box::new(WirType::U8));
                    env.insert(var.clone(), (ptr_ty, size_label));
                }
                WirInst::Load { var, ptr_expr, ty } => {
                    if env.contains_key(var) {
                        return Err(TypeError::VariableRedefined(var.clone()));
                    }
                    let (ptr_ty, ptr_label) = self.check_expr(ptr_expr, env)?;
                    match ptr_ty {
                        WirType::Ptr(pointee) => {
                            if &*pointee != ty && *pointee != WirType::U8 {
                                return Err(TypeError::TypeMismatch {
                                    expected: *pointee,
                                    actual: ty.clone(),
                                });
                            }
                        }
                        other => return Err(TypeError::ExpectedPointer(other)),
                    }
                    env.insert(var.clone(), (ty.clone(), ptr_label));
                }
                WirInst::Store { ptr_expr, val_expr } => {
                    let (ptr_ty, _) = self.check_expr(ptr_expr, env)?;
                    let (val_ty, _) = self.check_expr(val_expr, env)?;
                    match ptr_ty {
                        WirType::Ptr(pointee) => {
                            if *pointee != val_ty && *pointee != WirType::U8 {
                                return Err(TypeError::TypeMismatch {
                                    expected: *pointee,
                                    actual: val_ty,
                                });
                            }
                        }
                        other => return Err(TypeError::ExpectedPointer(other)),
                    }
                }
                WirInst::Free { ptr_expr } => {
                    let (ptr_ty, _) = self.check_expr(ptr_expr, env)?;
                    if !matches!(ptr_ty, WirType::Ptr(_)) {
                        return Err(TypeError::ExpectedPointer(ptr_ty));
                    }
                }
                WirInst::CallCapability { var, capability: _, args } => {
                    for arg in args {
                        self.check_expr(arg, env)?;
                    }
                    if let Some(v) = var {
                        if env.contains_key(v) {
                            return Err(TypeError::VariableRedefined(v.clone()));
                        }
                        // Dynamic result for generic capability call
                        env.insert(v.clone(), (WirType::Dynamic, Label::PUBLIC));
                    }
                }
                WirInst::CallFunction { var, func_name, args } => {
                    let target_fn = self
                        .program
                        .find_function(func_name)
                        .ok_or_else(|| TypeError::FunctionNotFound(func_name.clone()))?;
                    if target_fn.params.len() != args.len() {
                        return Err(TypeError::ArgumentCountMismatch {
                            expected: target_fn.params.len(),
                            actual: args.len(),
                        });
                    }
                    let mut joint_label = Label::PUBLIC;
                    for (arg, (_pvar, pty, _plabel)) in args.iter().zip(&target_fn.params) {
                        let (aty, alabel) = self.check_expr(arg, env)?;
                        if &aty != pty {
                            return Err(TypeError::TypeMismatch {
                                expected: pty.clone(),
                                actual: aty,
                            });
                        }
                        joint_label = joint_label.join(alabel);
                    }
                    if let Some(v) = var {
                        if env.contains_key(v) {
                            return Err(TypeError::VariableRedefined(v.clone()));
                        }
                        env.insert(v.clone(), (target_fn.return_type.clone(), joint_label));
                    }
                }
            }
        }

        match &block.terminator {
            WirTerminator::Return(maybe_expr) => {
                let actual_ty = if let Some(e) = maybe_expr {
                    let (t, _) = self.check_expr(e, env)?;
                    t
                } else {
                    WirType::Unit
                };
                if actual_ty != func.return_type {
                    return Err(TypeError::TypeMismatch {
                        expected: func.return_type.clone(),
                        actual: actual_ty,
                    });
                }
            }
            WirTerminator::Jump(target) => {
                if !block_ids.contains(target) {
                    return Err(TypeError::BlockNotFound(*target));
                }
            }
            WirTerminator::Branch { cond, then_block, else_block } => {
                let (cond_ty, cond_label) = self.check_expr(cond, env)?;
                if cond_ty != WirType::Bool {
                    return Err(TypeError::BranchConditionNotBool(cond_ty));
                }
                if cond_label.is_secret {
                    let dummy_var = VarId("secret_cond".to_string());
                    return Err(TypeError::SecretDependentBranch(dummy_var));
                }
                if !block_ids.contains(then_block) {
                    return Err(TypeError::BlockNotFound(*then_block));
                }
                if !block_ids.contains(else_block) {
                    return Err(TypeError::BlockNotFound(*else_block));
                }
            }
            WirTerminator::Trap(_) | WirTerminator::Unreachable => {}
        }

        Ok(())
    }

    pub fn check_expr(
        &self,
        expr: &WirExpr,
        env: &HashMap<VarId, (WirType, Label)>,
    ) -> Result<(WirType, Label), TypeError> {
        match expr {
            WirExpr::ConstInt { val, ty } => {
                if !ty.is_integer() {
                    return Err(TypeError::ExpectedInteger(ty.clone()));
                }
                let min = ty.min_value().unwrap_or(0);
                let max = ty.max_value().unwrap_or(0);
                if *val < min || *val > max {
                    return Err(TypeError::TypeMismatch {
                        expected: ty.clone(),
                        actual: WirType::Dynamic,
                    });
                }
                Ok((ty.clone(), Label::PUBLIC))
            }
            WirExpr::ConstBool(_) => Ok((WirType::Bool, Label::PUBLIC)),
            WirExpr::ConstBytes(_) => Ok((WirType::ByteSlice, Label::PUBLIC)),
            WirExpr::ConstUnit => Ok((WirType::Unit, Label::PUBLIC)),
            WirExpr::Var(var) => env
                .get(var)
                .cloned()
                .ok_or_else(|| TypeError::UndefinedVariable(var.clone())),
            WirExpr::Unary { op, expr } => {
                let (sub_ty, label) = self.check_expr(expr, env)?;
                match op {
                    UnaryOp::Not => {
                        if sub_ty == WirType::Bool || sub_ty.is_integer() {
                            Ok((sub_ty, label))
                        } else {
                            Err(TypeError::TypeMismatch {
                                expected: WirType::Bool,
                                actual: sub_ty,
                            })
                        }
                    }
                    UnaryOp::Neg(_) => {
                        if sub_ty.is_integer() {
                            Ok((sub_ty, label))
                        } else {
                            Err(TypeError::ExpectedInteger(sub_ty))
                        }
                    }
                }
            }
            WirExpr::Binary { op, lhs, rhs } => {
                let (lhs_ty, lhs_label) = self.check_expr(lhs, env)?;
                let (rhs_ty, rhs_label) = self.check_expr(rhs, env)?;
                let joint_label = lhs_label.join(rhs_label);

                match op {
                    BinaryOp::Add(_)
                    | BinaryOp::Sub(_)
                    | BinaryOp::Mul(_)
                    | BinaryOp::Div(_)
                    | BinaryOp::Rem(_)
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor => {
                        if !lhs_ty.is_integer() {
                            return Err(TypeError::ExpectedInteger(lhs_ty));
                        }
                        if lhs_ty != rhs_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: lhs_ty,
                                actual: rhs_ty,
                            });
                        }
                        Ok((lhs_ty, joint_label))
                    }
                    BinaryOp::Shl | BinaryOp::Shr => {
                        if !lhs_ty.is_integer() {
                            return Err(TypeError::ExpectedInteger(lhs_ty));
                        }
                        if !rhs_ty.is_integer() {
                            return Err(TypeError::ExpectedInteger(rhs_ty));
                        }
                        Ok((lhs_ty, joint_label))
                    }
                    BinaryOp::Eq | BinaryOp::Ne => {
                        if lhs_ty != rhs_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: lhs_ty,
                                actual: rhs_ty,
                            });
                        }
                        Ok((WirType::Bool, joint_label))
                    }
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                        if !lhs_ty.is_integer() {
                            return Err(TypeError::ExpectedInteger(lhs_ty));
                        }
                        if lhs_ty != rhs_ty {
                            return Err(TypeError::TypeMismatch {
                                expected: lhs_ty,
                                actual: rhs_ty,
                            });
                        }
                        Ok((WirType::Bool, joint_label))
                    }
                }
            }
            WirExpr::Trunc { expr, target_ty } => {
                let (sub_ty, label) = self.check_expr(expr, env)?;
                if !sub_ty.is_integer() || !target_ty.is_integer() {
                    return Err(TypeError::ExpectedInteger(sub_ty));
                }
                let from_w = sub_ty.bit_width().unwrap_or(0);
                let to_w = target_ty.bit_width().unwrap_or(0);
                if to_w >= from_w {
                    return Err(TypeError::InvalidTruncation {
                        from: sub_ty,
                        to: target_ty.clone(),
                    });
                }
                Ok((target_ty.clone(), label))
            }
            WirExpr::ZExt { expr, target_ty } | WirExpr::SExt { expr, target_ty } => {
                let (sub_ty, label) = self.check_expr(expr, env)?;
                if !sub_ty.is_integer() || !target_ty.is_integer() {
                    return Err(TypeError::ExpectedInteger(sub_ty));
                }
                let from_w = sub_ty.bit_width().unwrap_or(0);
                let to_w = target_ty.bit_width().unwrap_or(0);
                if to_w <= from_w {
                    return Err(TypeError::InvalidExtension {
                        from: sub_ty,
                        to: target_ty.clone(),
                    });
                }
                Ok((target_ty.clone(), label))
            }
            WirExpr::TupleMake(elems) => {
                let mut elem_tys = Vec::new();
                let mut joint_label = Label::PUBLIC;
                for e in elems {
                    let (ety, elabel) = self.check_expr(e, env)?;
                    elem_tys.push(ety);
                    joint_label = joint_label.join(elabel);
                }
                Ok((WirType::Tuple(elem_tys), joint_label))
            }
            WirExpr::TupleGet { expr, index } => {
                let (sub_ty, label) = self.check_expr(expr, env)?;
                match sub_ty {
                    WirType::Tuple(elems) => {
                        if *index >= elems.len() {
                            return Err(TypeError::TupleIndexOutOfBounds {
                                index: *index,
                                tuple_len: elems.len(),
                            });
                        }
                        Ok((elems[*index].clone(), label))
                    }
                    other => Err(TypeError::ExpectedTuple(other)),
                }
            }
            WirExpr::PtrOffset { ptr, offset_bytes } => {
                let (ptr_ty, ptr_label) = self.check_expr(ptr, env)?;
                let (offset_ty, offset_label) = self.check_expr(offset_bytes, env)?;
                if !matches!(ptr_ty, WirType::Ptr(_)) {
                    return Err(TypeError::ExpectedPointer(ptr_ty));
                }
                if !offset_ty.is_integer() {
                    return Err(TypeError::ExpectedInteger(offset_ty));
                }
                Ok((ptr_ty, ptr_label.join(offset_label)))
            }
            WirExpr::MakeDynamic { expr, tag: _ } => {
                let (_, label) = self.check_expr(expr, env)?;
                Ok((WirType::Dynamic, label))
            }
            WirExpr::CheckDynamic { expr, expected_tag: _ } => {
                let (ty, label) = self.check_expr(expr, env)?;
                if ty != WirType::Dynamic {
                    return Err(TypeError::TypeMismatch {
                        expected: WirType::Dynamic,
                        actual: ty,
                    });
                }
                Ok((WirType::Dynamic, label))
            }
        }
    }
}

//! Static type checker and well-formedness validator for WIR programs.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    DuplicateFunction(String),
    FunctionNotFound(String),
    EmptyFunction(String),
    DuplicateBlock(BlockId),
    BlockNotFound(BlockId),
    UndefinedVariable(VarId),
    /// A read of `var` in `block` of `function` that some path from the entry
    /// block reaches with `var` unbound (KRN-006b).
    UseBeforeDefinition {
        function: String,
        block: BlockId,
        var: VarId,
    },
    /// The function has no block with id 0, where execution starts (KRN-006b).
    NoEntryBlock(String),
    /// Block ids are not exactly `0..count`; `block` is the smallest id that
    /// is `>= count` (KRN-006b).
    BlockIdsNotDense {
        function: String,
        block: BlockId,
        count: usize,
    },
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

    // ------------------------------------------------------------------
    // Control-flow-aware definite assignment (KRN-006b, VER25 section 8.5)
    // ------------------------------------------------------------------
    //
    // A function's blocks sit in a `Vec`, but the vector order is only a
    // storage order.  Execution starts at the block with id 0 and follows
    // `Jump` and `Branch` edges, so "the variable was bound by an earlier
    // block in the vector" says nothing about whether it is bound on the path
    // that actually reaches a read.  Example (the KRN-006b counterexample):
    //
    //     block 0:  jump 2
    //     block 1:  let x = 1; jump 2        (never reached from block 0)
    //     block 2:  return x                 (reached with x unbound)
    //
    // The rule is: a variable may be read at a program point only if it is
    // bound on EVERY path from the entry block to that point.
    //
    // Why a dataflow fixpoint and not dominators?  Both decide the same
    // question here.  "x is bound on every path to P" is the same as "some
    // binding of x dominates P", provided bindings are the only way a name
    // becomes bound, and that holds in WIR.  Dominance wants the dominator
    // tree (an extra algorithm and data structure); the fixpoint wants only a
    // set per block and an intersection at joins, and it is the same loop that
    // carries the types and labels of the bound variables, which a dominator
    // tree would not.  And the fixpoint also accepts what dominance rejects
    // by construction: a diamond whose two arms each bind `x` (neither arm
    // dominates the join, but `x` is bound on both paths).
    //
    //   IN(entry)  = the parameters
    //   IN(b)      = meet of OUT(p) over every processed predecessor p
    //   OUT(b)     = IN(b) plus whatever the instructions of b bind
    //   meet       = keep a variable only if every side binds it with the same
    //                type; join the labels (secret/hostile are sticky)
    //
    // Each IN set only ever shrinks and each label only ever grows, and both
    // are finite, so the worklist terminates.  Loops are handled for free: a
    // back edge contributes the loop body's OUT, which lacks anything bound
    // only inside the body, so such a name drops out of the header's IN.
    //
    // A variable bound with different types on different paths has no single
    // type at the join, so it is treated as not definitely bound there.
    //
    // The fixpoint is computed leniently: an instruction that fails to type
    // still binds its target (with the declared type), so one error does not
    // cascade.  Errors are reported only by the strict pass that follows,
    // once every block's IN is final.  (Reporting during the iteration would
    // be wrong: an early IN is a superset of the final one, so a redefinition
    // that looks real early may vanish once the other predecessor arrives.)
    //
    // Blocks unreachable from the entry have no IN set.  The strict pass
    // checks them against the parameters alone, so dead code is still held to
    // the type rules but may not read a local; the read-only
    // `check_definite_assignment` skips them, because nothing reaches them.

    /// Structural checks shared by every entry point: at least one block, no
    /// duplicate ids, an entry block, and ids exactly `0..n`.
    fn check_structure(&self, func: &Function) -> Result<HashSet<BlockId>, TypeError> {
        if func.blocks.is_empty() {
            return Err(TypeError::EmptyFunction(func.name.clone()));
        }

        let mut block_ids = HashSet::new();
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                return Err(TypeError::DuplicateBlock(block.id));
            }
        }

        // `BasicBlock::id` is a field separate from the block's position in
        // `Function::blocks`, so ids can disagree with positions; the
        // interpreter looks blocks up by id and starts at `BlockId(0)`.
        if !block_ids.contains(&BlockId(0)) {
            return Err(TypeError::NoEntryBlock(func.name.clone()));
        }
        // Ids are distinct (checked above), so "exactly 0..n" is the same as
        // "every id is below n".  Report the smallest offender.
        let count = func.blocks.len();
        if let Some(bad) = block_ids.iter().filter(|id| id.0 >= count).min_by_key(|id| id.0) {
            return Err(TypeError::BlockIdsNotDense {
                function: func.name.clone(),
                block: *bad,
                count,
            });
        }
        Ok(block_ids)
    }

    fn params_env(func: &Function) -> Result<TypedEnv, TypeError> {
        let mut env: TypedEnv = HashMap::new();
        for (param_var, param_ty, param_label) in &func.params {
            if env.contains_key(param_var) {
                return Err(TypeError::VariableRedefined(param_var.clone()));
            }
            env.insert(param_var.clone(), (param_ty.clone(), *param_label));
        }
        Ok(env)
    }

    /// Run the fixpoint.  Result is indexed by position in `func.blocks`;
    /// `None` means the block is unreachable from the entry block.
    fn definite_envs(&self, func: &Function) -> Result<Vec<Option<TypedEnv>>, TypeError> {
        let index: HashMap<BlockId, usize> =
            func.blocks.iter().enumerate().map(|(i, b)| (b.id, i)).collect();
        let mut ins: Vec<Option<TypedEnv>> = vec![None; func.blocks.len()];
        let entry = index[&BlockId(0)];
        ins[entry] = Some(Self::params_env(func)?);

        let mut work: VecDeque<usize> = VecDeque::from([entry]);
        while let Some(i) = work.pop_front() {
            let block = &func.blocks[i];
            let mut out = ins[i].clone().expect("queued blocks have an IN set");
            self.bind_leniently(block, &mut out);
            for succ in successors(&block.terminator) {
                // A target that is not a block is reported by the strict pass.
                let Some(&j) = index.get(&succ) else { continue };
                match &mut ins[j] {
                    slot @ None => {
                        *slot = Some(out.clone());
                        work.push_back(j);
                    }
                    Some(cur) => {
                        if meet_into(cur, &out) {
                            work.push_back(j);
                        }
                    }
                }
            }
        }
        Ok(ins)
    }

    /// The effect of a block on the environment, ignoring type errors.
    fn bind_leniently(&self, block: &BasicBlock, env: &mut TypedEnv) {
        for inst in &block.instructions {
            match inst {
                WirInst::Let { var, ty, label, expr } => {
                    let l = self.check_expr(expr, env).map(|(_, l)| l).unwrap_or(Label::PUBLIC);
                    env.insert(var.clone(), (ty.clone(), label.join(l)));
                }
                WirInst::Alloc { var, size_expr } => {
                    let l = self.check_expr(size_expr, env).map(|(_, l)| l).unwrap_or(Label::PUBLIC);
                    env.insert(var.clone(), (WirType::Ptr(Box::new(WirType::U8)), l));
                }
                WirInst::Load { var, ptr_expr, ty } => {
                    let l = self.check_expr(ptr_expr, env).map(|(_, l)| l).unwrap_or(Label::PUBLIC);
                    env.insert(var.clone(), (ty.clone(), l));
                }
                WirInst::CallCapability { var: Some(v), .. } => {
                    env.insert(v.clone(), (WirType::Dynamic, Label::PUBLIC));
                }
                WirInst::CallFunction { var: Some(v), func_name, args } => {
                    if let Some(target) = self.program.find_function(func_name) {
                        let mut joint = Label::PUBLIC;
                        for a in args {
                            if let Ok((_, l)) = self.check_expr(a, env) {
                                joint = joint.join(l);
                            }
                        }
                        env.insert(v.clone(), (target.return_type.clone(), joint));
                    }
                }
                WirInst::Store { .. }
                | WirInst::Free { .. }
                | WirInst::CallCapability { var: None, .. }
                | WirInst::CallFunction { var: None, .. } => {}
            }
        }
    }

    /// Every name the function binds anywhere (parameters included).  Used
    /// only to choose between `UseBeforeDefinition` (bound somewhere, but not
    /// on every path to the read) and `UndefinedVariable` (never bound).
    fn bound_anywhere(func: &Function) -> HashSet<VarId> {
        let mut all: HashSet<VarId> = func.params.iter().map(|(v, _, _)| v.clone()).collect();
        for inst in func.blocks.iter().flat_map(|b| &b.instructions) {
            match inst {
                WirInst::Let { var, .. } | WirInst::Alloc { var, .. } | WirInst::Load { var, .. } => {
                    all.insert(var.clone());
                }
                WirInst::CallCapability { var: Some(v), .. }
                | WirInst::CallFunction { var: Some(v), .. } => {
                    all.insert(v.clone());
                }
                _ => {}
            }
        }
        all
    }

    fn qualify(
        func: &Function,
        block: &BasicBlock,
        bound: &HashSet<VarId>,
        err: TypeError,
    ) -> TypeError {
        match err {
            TypeError::UndefinedVariable(var) if bound.contains(&var) => {
                TypeError::UseBeforeDefinition {
                    function: func.name.clone(),
                    block: block.id,
                    var,
                }
            }
            other => other,
        }
    }

    /// Only the definite-assignment rule (plus the structural checks): every
    /// read in a block reachable from the entry must name a variable bound on
    /// every path to it.  No type rules.  Lets a caller ask the KRN-006b
    /// question of a program that the type rules reject for other reasons.
    pub fn check_definite_assignment(&self, func: &Function) -> Result<(), TypeError> {
        self.check_structure(func)?;
        let ins = self.definite_envs(func)?;
        let bound = Self::bound_anywhere(func);
        for (block, maybe_in) in func.blocks.iter().zip(&ins) {
            let Some(in_env) = maybe_in else { continue };
            let mut have: HashSet<&VarId> = in_env.keys().collect();
            let check_reads = |exprs: &[&WirExpr], have: &HashSet<&VarId>| {
                let mut reads = Vec::new();
                for e in exprs {
                    vars_read(e, &mut reads);
                }
                for v in reads {
                    if !have.contains(v) {
                        let e = TypeError::UndefinedVariable(v.clone());
                        return Err(Self::qualify(func, block, &bound, e));
                    }
                }
                Ok(())
            };
            for inst in &block.instructions {
                let (reads, def): (Vec<&WirExpr>, Option<&VarId>) = match inst {
                    WirInst::Let { var, expr, .. } => (vec![expr], Some(var)),
                    WirInst::Alloc { var, size_expr } => (vec![size_expr], Some(var)),
                    WirInst::Load { var, ptr_expr, .. } => (vec![ptr_expr], Some(var)),
                    WirInst::Store { ptr_expr, val_expr } => (vec![ptr_expr, val_expr], None),
                    WirInst::Free { ptr_expr } => (vec![ptr_expr], None),
                    WirInst::CallCapability { var, args, .. }
                    | WirInst::CallFunction { var, args, .. } => {
                        (args.iter().collect(), var.as_ref())
                    }
                };
                check_reads(&reads, &have)?;
                if let Some(v) = def {
                    have.insert(v);
                }
            }
            match &block.terminator {
                WirTerminator::Return(Some(e)) => check_reads(&[e], &have)?,
                WirTerminator::Branch { cond, .. } => check_reads(&[cond], &have)?,
                _ => {}
            }
        }
        Ok(())
    }

    pub fn check_function(&self, func: &Function) -> Result<(), TypeError> {
        let block_ids = self.check_structure(func)?;
        let ins = self.definite_envs(func)?;
        let bound = Self::bound_anywhere(func);

        for (block, maybe_in) in func.blocks.iter().zip(ins) {
            let mut env = match maybe_in {
                Some(env) => env,
                None => Self::params_env(func)?,
            };
            self.check_block(func, block, &mut env, &block_ids)
                .map_err(|e| Self::qualify(func, block, &bound, e))?;
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
            WirExpr::PtrLen(ptr) => {
                let (sub_ty, _) = self.check_expr(ptr, env)?;
                match sub_ty {
                    WirType::Ptr(_) => Ok((WirType::U64, Label::PUBLIC)),
                    _ => Err(TypeError::ExpectedPointer(sub_ty)),
                }
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

/// Variables in scope with their type and label.
type TypedEnv = HashMap<VarId, (WirType, Label)>;

/// Control-flow successors of a terminator.
fn successors(t: &WirTerminator) -> Vec<BlockId> {
    match t {
        WirTerminator::Jump(b) => vec![*b],
        WirTerminator::Branch { then_block, else_block, .. } => vec![*then_block, *else_block],
        WirTerminator::Return(_) | WirTerminator::Trap(_) | WirTerminator::Unreachable => vec![],
    }
}

/// Intersect `other` into `cur` (the join of two paths).  A variable survives
/// only if both sides bind it with the same type; labels are joined.  Returns
/// whether `cur` changed, which is what drives the worklist.
fn meet_into(cur: &mut TypedEnv, other: &TypedEnv) -> bool {
    let mut changed = false;
    cur.retain(|var, (ty, label)| match other.get(var) {
        Some((oty, olabel)) if oty == ty => {
            let joined = label.join(*olabel);
            if joined != *label {
                *label = joined;
                changed = true;
            }
            true
        }
        _ => {
            changed = true;
            false
        }
    });
    changed
}

/// Every variable an expression reads.
fn vars_read<'e>(expr: &'e WirExpr, out: &mut Vec<&'e VarId>) {
    match expr {
        WirExpr::Var(v) => out.push(v),
        WirExpr::ConstInt { .. }
        | WirExpr::ConstBool(_)
        | WirExpr::ConstBytes(_)
        | WirExpr::ConstUnit => {}
        WirExpr::Unary { expr, .. }
        | WirExpr::Trunc { expr, .. }
        | WirExpr::ZExt { expr, .. }
        | WirExpr::SExt { expr, .. }
        | WirExpr::TupleGet { expr, .. }
        | WirExpr::PtrLen(expr)
        | WirExpr::MakeDynamic { expr, .. }
        | WirExpr::CheckDynamic { expr, .. } => vars_read(expr, out),
        WirExpr::Binary { lhs, rhs, .. } => {
            vars_read(lhs, out);
            vars_read(rhs, out);
        }
        WirExpr::PtrOffset { ptr, offset_bytes } => {
            vars_read(ptr, out);
            vars_read(offset_bytes, out);
        }
        WirExpr::TupleMake(elems) => elems.iter().for_each(|e| vars_read(e, out)),
    }
}

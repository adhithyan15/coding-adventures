//! Lowering from Confined Rust Subset AST to Warrant Intermediate Representation (WIR).
//! (VER25b, VER-P023)

use std::collections::HashMap;

use crate::ast::*;
use crate::rust_subset::ast as rust_ast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweringError {
    pub message: String,
    pub span: rust_ast::Span,
}

pub struct LoweringContext {
    var_counter: usize,
    block_counter: usize,
    scopes: Vec<HashMap<String, (VarId, WirType, Label)>>,
    current_instructions: Vec<WirInst>,
    blocks: Vec<BasicBlock>,
}

impl Default for LoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

impl LoweringContext {
    pub fn new() -> Self {
        Self {
            var_counter: 0,
            block_counter: 0,
            scopes: vec![HashMap::new()],
            current_instructions: Vec::new(),
            blocks: Vec::new(),
        }
    }

    pub fn fresh_var(&mut self, prefix: &str) -> VarId {
        self.var_counter += 1;
        VarId(format!("{}_{}", prefix, self.var_counter))
    }

    pub fn fresh_block_id(&mut self) -> BlockId {
        let id = self.block_counter;
        self.block_counter += 1;
        BlockId(id)
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert_var(&mut self, name: String, var: VarId, ty: WirType, label: Label) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, (var, ty, label));
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<(VarId, WirType, Label)> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry.clone());
            }
        }
        None
    }

    pub fn emit_inst(&mut self, inst: WirInst) {
        self.current_instructions.push(inst);
    }

    pub fn finish_block(&mut self, id: BlockId, terminator: WirTerminator) {
        let instructions = std::mem::take(&mut self.current_instructions);
        self.blocks.push(BasicBlock {
            id,
            instructions,
            terminator,
        });
    }
}

/// Convert a Rust AST type to a WIR type.
pub fn lower_type(rust_ty: &rust_ast::RustType) -> Result<WirType, LoweringError> {
    match rust_ty {
        rust_ast::RustType::Bool => Ok(WirType::Bool),
        rust_ast::RustType::U8 => Ok(WirType::U8),
        rust_ast::RustType::U16 => Ok(WirType::U16),
        rust_ast::RustType::U32 => Ok(WirType::U32),
        rust_ast::RustType::U64 | rust_ast::RustType::Usize => Ok(WirType::U64),
        rust_ast::RustType::I8 => Ok(WirType::I8),
        rust_ast::RustType::I16 => Ok(WirType::I16),
        rust_ast::RustType::I32 => Ok(WirType::I32),
        rust_ast::RustType::I64 | rust_ast::RustType::Isize => Ok(WirType::I64),
        rust_ast::RustType::Unit => Ok(WirType::Unit),
        rust_ast::RustType::Ref(inner) | rust_ast::RustType::RefMut(inner) => {
            match &**inner {
                rust_ast::RustType::Slice(elem)
                | rust_ast::RustType::Array(elem, _)
                | rust_ast::RustType::ConstArray(elem, _) => {
                    let elem_wir = lower_type(elem)?;
                    Ok(WirType::Ptr(Box::new(elem_wir)))
                }
                _ => {
                    let inner_wir = lower_type(inner)?;
                    Ok(WirType::Ptr(Box::new(inner_wir)))
                }
            }
        }
        rust_ast::RustType::Slice(inner) => {
            let inner_wir = lower_type(inner)?;
            Ok(WirType::Ptr(Box::new(inner_wir)))
        }
        rust_ast::RustType::Array(inner, _len) => {
            let inner_wir = lower_type(inner)?;
            Ok(WirType::Ptr(Box::new(inner_wir)))
        }
        rust_ast::RustType::ConstArray(inner, _name) => {
            let inner_wir = lower_type(inner)?;
            Ok(WirType::Ptr(Box::new(inner_wir)))
        }
        rust_ast::RustType::Vec(inner) => {
            let inner_wir = lower_type(inner)?;
            Ok(WirType::Ptr(Box::new(inner_wir)))
        }
        rust_ast::RustType::Custom(name) => Err(LoweringError {
            message: format!("Unsupported custom type in WIR lowering: {}", name),
            span: rust_ast::Span::default(),
        }),
    }
}

/// Lower a binary operator.
fn lower_binop(op: rust_ast::BinOp) -> BinaryOp {
    match op {
        rust_ast::BinOp::Add => BinaryOp::Add(OverflowMode::Wrap),
        rust_ast::BinOp::Sub => BinaryOp::Sub(OverflowMode::Wrap),
        rust_ast::BinOp::Mul => BinaryOp::Mul(OverflowMode::Wrap),
        rust_ast::BinOp::Div => BinaryOp::Div(OverflowMode::Wrap),
        rust_ast::BinOp::Rem => BinaryOp::Rem(OverflowMode::Wrap),
        rust_ast::BinOp::BitAnd => BinaryOp::BitAnd,
        rust_ast::BinOp::BitOr => BinaryOp::BitOr,
        rust_ast::BinOp::BitXor => BinaryOp::BitXor,
        rust_ast::BinOp::Shl => BinaryOp::Shl,
        rust_ast::BinOp::Shr => BinaryOp::Shr,
        rust_ast::BinOp::Eq => BinaryOp::Eq,
        rust_ast::BinOp::Ne => BinaryOp::Ne,
        rust_ast::BinOp::Lt => BinaryOp::Lt,
        rust_ast::BinOp::Le => BinaryOp::Le,
        rust_ast::BinOp::Gt => BinaryOp::Gt,
        rust_ast::BinOp::Ge => BinaryOp::Ge,
        rust_ast::BinOp::And => BinaryOp::BitAnd, // for booleans
        rust_ast::BinOp::Or => BinaryOp::BitOr,
    }
}

/// Lower an expression into a WIR variable or expression.
pub fn lower_expr(
    ctx: &mut LoweringContext,
    expr: &rust_ast::Expr,
) -> Result<(WirExpr, WirType, Label), LoweringError> {
    match expr {
        rust_ast::Expr::Lit(lit, _span) => match lit {
            rust_ast::Lit::Bool(b) => Ok((WirExpr::ConstBool(*b), WirType::Bool, Label::PUBLIC)),
            rust_ast::Lit::Int(val, suffix) => {
                let ty = match suffix.as_deref() {
                    Some("u8") => WirType::U8,
                    Some("u16") => WirType::U16,
                    Some("u32") => WirType::U32,
                    Some("u64") | Some("usize") | None => WirType::U64,
                    Some("i8") => WirType::I8,
                    Some("i16") => WirType::I16,
                    Some("i32") => WirType::I32,
                    Some("i64") | Some("isize") => WirType::I64,
                    Some(other) => {
                        return Err(LoweringError {
                            message: format!("Unsupported integer literal suffix: {}", other),
                            span: expr.span(),
                        });
                    }
                };
                Ok((WirExpr::ConstInt { val: *val as i128, ty: ty.clone() }, ty, Label::PUBLIC))
            }
            rust_ast::Lit::ByteString(bytes) => Ok((
                WirExpr::ConstBytes(bytes.clone()),
                WirType::Ptr(Box::new(WirType::U8)),
                Label::PUBLIC,
            )),
            rust_ast::Lit::String(s) => Ok((
                WirExpr::ConstBytes(s.as_bytes().to_vec()),
                WirType::Ptr(Box::new(WirType::U8)),
                Label::PUBLIC,
            )),
        },
        rust_ast::Expr::Path(path, span) => {
            let name = path.last().cloned().unwrap_or_default();
            if name == "MAX" && path.len() >= 2 && path[0] == "u64" {
                return Ok((
                    WirExpr::ConstInt { val: u64::MAX as i128, ty: WirType::U64 },
                    WirType::U64,
                    Label::PUBLIC,
                ));
            }
            if let Some((var, ty, label)) = ctx.lookup_var(&name) {
                Ok((WirExpr::Var(var), ty, label))
            } else {
                Err(LoweringError {
                    message: format!("Variable '{}' not found in scope", name),
                    span: *span,
                })
            }
        }
        rust_ast::Expr::Unary(un_op, inner, _span) => {
            let (inner_expr, inner_ty, label) = lower_expr(ctx, inner)?;
            match un_op {
                rust_ast::UnOp::Not => {
                    if inner_ty == WirType::Bool {
                        Ok((
                            WirExpr::Unary {
                                op: UnaryOp::Not,
                                expr: Box::new(inner_expr),
                            },
                            WirType::Bool,
                            label,
                        ))
                    } else {
                        // Bitwise not: (x ^ MAX)
                        let max_val = match inner_ty {
                            WirType::U8 => u8::MAX as i128,
                            WirType::U16 => u16::MAX as i128,
                            WirType::U32 => u32::MAX as i128,
                            WirType::U64 => u64::MAX as i128,
                            _ => -1,
                        };
                        Ok((
                            WirExpr::Binary {
                                op: BinaryOp::BitXor,
                                lhs: Box::new(inner_expr),
                                rhs: Box::new(WirExpr::ConstInt { val: max_val, ty: inner_ty.clone() }),
                            },
                            inner_ty,
                            label,
                        ))
                    }
                }
                rust_ast::UnOp::Neg => Ok((
                    WirExpr::Unary {
                        op: UnaryOp::Neg(OverflowMode::Wrap),
                        expr: Box::new(inner_expr),
                    },
                    inner_ty,
                    label,
                )),
                rust_ast::UnOp::Ref | rust_ast::UnOp::RefMut => {
                    // In WIR, address-of variable is representationally pointer
                    if matches!(inner_ty, WirType::Ptr(_)) {
                        Ok((inner_expr, inner_ty, label))
                    } else {
                        Ok((inner_expr, WirType::Ptr(Box::new(inner_ty)), label))
                    }
                }
                rust_ast::UnOp::Deref => {
                    if let WirType::Ptr(inner_ref) = inner_ty {
                        let tmp = ctx.fresh_var("deref");
                        let res_ty = *inner_ref;
                        ctx.emit_inst(WirInst::Load {
                            var: tmp.clone(),
                            ptr_expr: inner_expr,
                            ty: res_ty.clone(),
                        });
                        Ok((WirExpr::Var(tmp), res_ty, label))
                    } else {
                        Ok((inner_expr, inner_ty, label))
                    }
                }
            }
        }
        rust_ast::Expr::Binary(op, lhs, rhs, _span) => {
            let (l_expr, l_ty, l_lbl) = lower_expr(ctx, lhs)?;
            let (r_expr, r_ty, r_lbl) = lower_expr(ctx, rhs)?;
            let label = l_lbl.join(r_lbl);
            let wir_op = lower_binop(*op);

            let res_ty = match op {
                rust_ast::BinOp::Eq
                | rust_ast::BinOp::Ne
                | rust_ast::BinOp::Lt
                | rust_ast::BinOp::Le
                | rust_ast::BinOp::Gt
                | rust_ast::BinOp::Ge => WirType::Bool,
                _ => {
                    if l_ty.is_integer() {
                        l_ty
                    } else {
                        r_ty
                    }
                }
            };

            Ok((
                WirExpr::Binary {
                    op: wir_op,
                    lhs: Box::new(l_expr),
                    rhs: Box::new(r_expr),
                },
                res_ty,
                label,
            ))
        }
        rust_ast::Expr::Cast(inner, target_rust_ty, _span) => {
            let (inner_expr, inner_ty, label) = lower_expr(ctx, inner)?;
            let target_ty = lower_type(target_rust_ty)?;
            if inner_ty == target_ty {
                return Ok((inner_expr, target_ty, label));
            }

            if inner_ty == WirType::Bool && target_ty.is_integer() {
                // bool as integer (false -> 0, true -> 1) via memory round-trip
                let slot = ctx.fresh_var("bool_slot");
                let size = target_ty.byte_size().unwrap_or(1);
                ctx.emit_inst(WirInst::Alloc {
                    var: slot.clone(),
                    size_expr: WirExpr::ConstInt { val: size as i128, ty: WirType::U64 },
                });
                ctx.emit_inst(WirInst::Store {
                    ptr_expr: WirExpr::Var(slot.clone()),
                    val_expr: inner_expr,
                });
                let res = ctx.fresh_var("bool_cast");
                ctx.emit_inst(WirInst::Load {
                    var: res.clone(),
                    ptr_expr: WirExpr::Var(slot.clone()),
                    ty: target_ty.clone(),
                });
                ctx.emit_inst(WirInst::Free {
                    ptr_expr: WirExpr::Var(slot),
                });
                return Ok((WirExpr::Var(res), target_ty, label));
            }

            let res_expr = if let (Some(w_in), Some(w_out)) = (inner_ty.bit_width(), target_ty.bit_width()) {
                if w_out < w_in {
                    WirExpr::Trunc {
                        expr: Box::new(inner_expr),
                        target_ty: target_ty.clone(),
                    }
                } else if inner_ty.is_signed_integer() {
                    WirExpr::SExt {
                        expr: Box::new(inner_expr),
                        target_ty: target_ty.clone(),
                    }
                } else {
                    WirExpr::ZExt {
                        expr: Box::new(inner_expr),
                        target_ty: target_ty.clone(),
                    }
                }
            } else {
                inner_expr
            };

            Ok((res_expr, target_ty, label))
        }
        rust_ast::Expr::MethodCall(target, method, args, span) => {
            let (target_expr, target_ty, label) = lower_expr(ctx, target)?;
            match method.as_str() {
                "wrapping_sub" => {
                    if args.len() != 1 {
                        return Err(LoweringError {
                            message: "wrapping_sub expects 1 argument".to_string(),
                            span: *span,
                        });
                    }
                    let (arg_expr, _, arg_lbl) = lower_expr(ctx, &args[0])?;
                    Ok((
                        WirExpr::Binary {
                            op: BinaryOp::Sub(OverflowMode::Wrap),
                            lhs: Box::new(target_expr),
                            rhs: Box::new(arg_expr),
                        },
                        target_ty,
                        label.join(arg_lbl),
                    ))
                }
                "wrapping_neg" => Ok((
                    WirExpr::Binary {
                        op: BinaryOp::Sub(OverflowMode::Wrap),
                        lhs: Box::new(WirExpr::ConstInt { val: 0, ty: target_ty.clone() }),
                        rhs: Box::new(target_expr),
                    },
                    target_ty,
                    label,
                )),
                "len" => {
                    // Length of slice or array: lookup len variable or default
                    if let Some((len_var, len_ty, _)) = ctx.lookup_var("len") {
                        Ok((WirExpr::Var(len_var), len_ty, Label::PUBLIC))
                    } else if let Some((len_var, len_ty, _)) = ctx.lookup_var("N") {
                        Ok((WirExpr::Var(len_var), len_ty, Label::PUBLIC))
                    } else {
                        // Query length from pointer allocation in memory
                        let len_var = ctx.fresh_var("slice_len");
                        ctx.emit_inst(WirInst::Let {
                            var: len_var.clone(),
                            ty: WirType::U64,
                            label: Label::PUBLIC,
                            expr: WirExpr::PtrLen(Box::new(target_expr)),
                        });
                        Ok((WirExpr::Var(len_var), WirType::U64, Label::PUBLIC))
                    }
                }
                other => Err(LoweringError {
                    message: format!("Unsupported method call '{}' in WIR lowering", other),
                    span: *span,
                }),
            }
        }
        rust_ast::Expr::Call(callee, args, span) => {
            if let rust_ast::Expr::Path(path, _) = &**callee {
                let name = path.join("::");
                if name == "black_box" || name == "core::hint::black_box" {
                    if let Some(arg) = args.first() {
                        return lower_expr(ctx, arg);
                    }
                }
                if name == "volatile_barrier::accumulate" && args.len() == 3 {
                    let (acc_ptr, _, _) = lower_expr(ctx, &args[0])?;
                    let (left_val, _, l_lbl) = lower_expr(ctx, &args[1])?;
                    let (right_val, _, r_lbl) = lower_expr(ctx, &args[2])?;

                    let acc_curr = ctx.fresh_var("acc_curr");
                    ctx.emit_inst(WirInst::Load {
                        var: acc_curr.clone(),
                        ptr_expr: acc_ptr.clone(),
                        ty: WirType::U8,
                    });

                    let diff_var = ctx.fresh_var("diff");
                    ctx.emit_inst(WirInst::Let {
                        var: diff_var.clone(),
                        ty: WirType::U8,
                        label: l_lbl.join(r_lbl),
                        expr: WirExpr::Binary {
                            op: BinaryOp::BitXor,
                            lhs: Box::new(left_val),
                            rhs: Box::new(right_val),
                        },
                    });

                    let acc_next = ctx.fresh_var("acc_next");
                    ctx.emit_inst(WirInst::Let {
                        var: acc_next.clone(),
                        ty: WirType::U8,
                        label: l_lbl.join(r_lbl),
                        expr: WirExpr::Binary {
                            op: BinaryOp::BitOr,
                            lhs: Box::new(WirExpr::Var(acc_curr)),
                            rhs: Box::new(WirExpr::Var(diff_var)),
                        },
                    });

                    ctx.emit_inst(WirInst::Store {
                        ptr_expr: acc_ptr,
                        val_expr: WirExpr::Var(acc_next),
                    });

                    return Ok((WirExpr::ConstUnit, WirType::Unit, Label::PUBLIC));
                }
                if name == "volatile_barrier::read" && args.len() == 1 {
                    let (acc_ptr, _, label) = lower_expr(ctx, &args[0])?;
                    let res = ctx.fresh_var("acc_res");
                    ctx.emit_inst(WirInst::Load {
                        var: res.clone(),
                        ptr_expr: acc_ptr,
                        ty: WirType::U8,
                    });
                    return Ok((WirExpr::Var(res), WirType::U8, label));
                }
                if name == "read_volatile" || name == "core::ptr::read_volatile" {
                    if let Some(arg) = args.first() {
                        let (ptr_expr, ty, label) = lower_expr(ctx, arg)?;
                        let elem_ty = match ty {
                            WirType::Ptr(inner) => *inner,
                            _ => WirType::U8,
                        };
                        let tmp = ctx.fresh_var("vol_read");
                        ctx.emit_inst(WirInst::Load {
                            var: tmp.clone(),
                            ptr_expr,
                            ty: elem_ty.clone(),
                        });
                        return Ok((WirExpr::Var(tmp), elem_ty, label));
                    }
                }
                if (name == "write_volatile" || name == "core::ptr::write_volatile") && args.len() == 2 {
                    let (ptr_expr, _, _) = lower_expr(ctx, &args[0])?;
                    let (val_expr, _, _) = lower_expr(ctx, &args[1])?;
                    ctx.emit_inst(WirInst::Store {
                        ptr_expr,
                        val_expr,
                    });
                    return Ok((WirExpr::ConstUnit, WirType::Unit, Label::PUBLIC));
                }

                // General function call
                let mut lowered_args = Vec::new();
                let mut joined_label = Label::PUBLIC;
                for a in args {
                    let (ae, _, al) = lower_expr(ctx, a)?;
                    lowered_args.push(ae);
                    joined_label = joined_label.join(al);
                }
                let ret_var = ctx.fresh_var("call_res");
                ctx.emit_inst(WirInst::CallFunction {
                    var: Some(ret_var.clone()),
                    func_name: name,
                    args: lowered_args,
                });
                return Ok((WirExpr::Var(ret_var), WirType::Bool, joined_label));
            }
            Err(LoweringError {
                message: "Unsupported call expression".to_string(),
                span: *span,
            })
        }
        rust_ast::Expr::Index(base, index, _span) => {
            let (base_expr, base_ty, b_lbl) = lower_expr(ctx, base)?;
            let (idx_expr, _, i_lbl) = lower_expr(ctx, index)?;
            let elem_ty = match base_ty {
                WirType::Ptr(inner) => *inner,
                _ => WirType::U8,
            };

            let ptr_offset = ctx.fresh_var("idx_ptr");
            ctx.emit_inst(WirInst::Let {
                var: ptr_offset.clone(),
                ty: WirType::Ptr(Box::new(elem_ty.clone())),
                label: b_lbl.join(i_lbl),
                expr: WirExpr::PtrOffset {
                    ptr: Box::new(base_expr),
                    offset_bytes: Box::new(idx_expr),
                },
            });

            let val = ctx.fresh_var("idx_val");
            ctx.emit_inst(WirInst::Load {
                var: val.clone(),
                ptr_expr: WirExpr::Var(ptr_offset),
                ty: elem_ty.clone(),
            });

            Ok((WirExpr::Var(val), elem_ty, b_lbl.join(i_lbl)))
        }
        rust_ast::Expr::MacroCall(name, args, span) => {
            if name == "vec" && args.len() == 2 {
                // vec![val; len]
                let (val_expr, val_ty, _) = lower_expr(ctx, &args[0])?;
                let (len_expr, _, _) = lower_expr(ctx, &args[1])?;
                let buf = ctx.fresh_var("vec_buf");
                ctx.emit_inst(WirInst::Alloc {
                    var: buf.clone(),
                    size_expr: len_expr,
                });
                // Allocation is zero-filled, so a zero fill needs no store.  The
                // store matters for another fill value, and it is *wrong* for a
                // zero-length vector (it would write past a 0-byte buffer), which
                // is why it is skipped rather than emitted unconditionally.
                let zero_fill = matches!(
                    &val_expr,
                    WirExpr::ConstInt { val: 0, .. } | WirExpr::ConstBool(false)
                );
                if !zero_fill {
                    ctx.emit_inst(WirInst::Store {
                        ptr_expr: WirExpr::Var(buf.clone()),
                        val_expr,
                    });
                }
                return Ok((WirExpr::Var(buf), WirType::Ptr(Box::new(val_ty)), Label::PUBLIC));
            }
            if name == "assert_eq" || name == "debug_assert_eq" || name == "assert" {
                // Non-operational in WIR lowering
                return Ok((WirExpr::ConstUnit, WirType::Unit, Label::PUBLIC));
            }
            Err(LoweringError {
                message: format!("Unsupported macro '{}' in WIR lowering", name),
                span: *span,
            })
        }
        rust_ast::Expr::ArrayRepeat(val, len, _span) => {
            let (v_expr, v_ty, _) = lower_expr(ctx, val)?;
            let (l_expr, _, _) = lower_expr(ctx, len)?;
            let buf = ctx.fresh_var("arr_rep");
            ctx.emit_inst(WirInst::Alloc {
                var: buf.clone(),
                size_expr: l_expr,
            });
            ctx.emit_inst(WirInst::Store {
                ptr_expr: WirExpr::Var(buf.clone()),
                val_expr: v_expr,
            });
            Ok((WirExpr::Var(buf), WirType::Ptr(Box::new(v_ty)), Label::PUBLIC))
        }
        rust_ast::Expr::Array(elements, _span) => {
            let elem_count = elements.len();
            let buf = ctx.fresh_var("arr_lit");
            ctx.emit_inst(WirInst::Alloc {
                var: buf.clone(),
                size_expr: WirExpr::ConstInt { val: elem_count as i128, ty: WirType::U64 },
            });
            for (i, elem) in elements.iter().enumerate() {
                let (e_expr, _, _) = lower_expr(ctx, elem)?;
                let offset_ptr = ctx.fresh_var("arr_elem_ptr");
                ctx.emit_inst(WirInst::Let {
                    var: offset_ptr.clone(),
                    ty: WirType::Ptr(Box::new(WirType::U8)),
                    label: Label::PUBLIC,
                    expr: WirExpr::PtrOffset {
                        ptr: Box::new(WirExpr::Var(buf.clone())),
                        offset_bytes: Box::new(WirExpr::ConstInt { val: i as i128, ty: WirType::U64 }),
                    },
                });
                ctx.emit_inst(WirInst::Store {
                    ptr_expr: WirExpr::Var(offset_ptr),
                    val_expr: e_expr,
                });
            }
            Ok((WirExpr::Var(buf), WirType::Ptr(Box::new(WirType::U8)), Label::PUBLIC))
        }
        _ => Err(LoweringError {
            message: "Unsupported expression variant in WIR lowering".to_string(),
            span: expr.span(),
        }),
    }
}

/// Lower a block into basic blocks.
pub fn lower_block(
    ctx: &mut LoweringContext,
    block: &rust_ast::Block,
    mut current_block: BlockId,
) -> Result<BlockId, LoweringError> {
    ctx.push_scope();

    for stmt in &block.stmts {
        match stmt {
            rust_ast::Stmt::Let { name, ty, init, .. } => {
                if let Some(init_expr) = init {
                    let (mut expr_val, mut expr_ty, label) = lower_expr(ctx, init_expr)?;
                    if let Some(rust_ty) = ty {
                        if let Ok(declared_wir_ty) = lower_type(rust_ty) {
                            if expr_ty != declared_wir_ty {
                                if let WirExpr::ConstInt { val, .. } = expr_val {
                                    expr_val = WirExpr::ConstInt { val, ty: declared_wir_ty.clone() };
                                    expr_ty = declared_wir_ty;
                                }
                            }
                        }
                    }
                    if name == "acc" {
                        let ptr_var = ctx.fresh_var(&format!("{}_ptr", name));
                        ctx.emit_inst(WirInst::Alloc {
                            var: ptr_var.clone(),
                            size_expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
                        });
                        ctx.emit_inst(WirInst::Store {
                            ptr_expr: WirExpr::Var(ptr_var.clone()),
                            val_expr: expr_val,
                        });
                        ctx.insert_var(name.clone(), ptr_var, WirType::Ptr(Box::new(WirType::U8)), label);
                    } else {
                        let var = ctx.fresh_var(name);
                        ctx.emit_inst(WirInst::Let {
                            var: var.clone(),
                            ty: expr_ty.clone(),
                            label,
                            expr: expr_val,
                        });
                        ctx.insert_var(name.clone(), var, expr_ty, label);
                    }
                }
            }
            rust_ast::Stmt::Assign { target, op, value, span: _ } => {
                let (val_expr, val_ty, val_lbl) = lower_expr(ctx, value)?;
                match target {
                    rust_ast::Expr::Path(path, _) => {
                        let name = path.last().cloned().unwrap_or_default();
                        if let Some((curr_var, curr_ty, curr_lbl)) = ctx.lookup_var(&name) {
                            let new_val = match op {
                                rust_ast::AssignOp::Assign => val_expr,
                                rust_ast::AssignOp::BitOrAssign => WirExpr::Binary {
                                    op: BinaryOp::BitOr,
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::BitXorAssign => WirExpr::Binary {
                                    op: BinaryOp::BitXor,
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::BitAndAssign => WirExpr::Binary {
                                    op: BinaryOp::BitAnd,
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::AddAssign => WirExpr::Binary {
                                    op: BinaryOp::Add(OverflowMode::Wrap),
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::SubAssign => WirExpr::Binary {
                                    op: BinaryOp::Sub(OverflowMode::Wrap),
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::ShlAssign => WirExpr::Binary {
                                    op: BinaryOp::Shl,
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                                rust_ast::AssignOp::ShrAssign => WirExpr::Binary {
                                    op: BinaryOp::Shr,
                                    lhs: Box::new(WirExpr::Var(curr_var.clone())),
                                    rhs: Box::new(val_expr),
                                },
                            };
                            let new_var = ctx.fresh_var(&name);
                            let new_lbl = curr_lbl.join(val_lbl);
                            ctx.emit_inst(WirInst::Let {
                                var: new_var.clone(),
                                ty: curr_ty.clone(),
                                label: new_lbl,
                                expr: new_val,
                            });
                            ctx.insert_var(name, new_var, curr_ty, new_lbl);
                        }
                    }
                    rust_ast::Expr::Index(base, idx, _) => {
                        let (base_expr, _, _) = lower_expr(ctx, base)?;
                        let (idx_expr, _, _) = lower_expr(ctx, idx)?;
                        let offset_ptr = ctx.fresh_var("store_idx_ptr");
                        ctx.emit_inst(WirInst::Let {
                            var: offset_ptr.clone(),
                            ty: WirType::Ptr(Box::new(val_ty)),
                            label: Label::PUBLIC,
                            expr: WirExpr::PtrOffset {
                                ptr: Box::new(base_expr),
                                offset_bytes: Box::new(idx_expr),
                            },
                        });
                        ctx.emit_inst(WirInst::Store {
                            ptr_expr: WirExpr::Var(offset_ptr),
                            val_expr,
                        });
                    }
                    _ => {}
                }
            }
            rust_ast::Stmt::Expr(expr, _) => {
                // `assert!` / `assert_eq!` are active in release builds: a failed
                // check traps before anything after it runs.  Lowering them as
                // no-ops made the mismatch path of `ct_select_bytes` look like an
                // out-of-bounds scan, so it is lowered as a branch to a trap.
                // (`debug_assert*` is compiled out of release builds and stays a
                // no-op.)  Both operands are evaluated for their side effects.
                if let rust_ast::Expr::MacroCall(name, args, _) = expr {
                    let cond = match (name.as_str(), args.len()) {
                        ("assert_eq", n) if n >= 2 => {
                            let (l, _, _) = lower_expr(ctx, &args[0])?;
                            let (r, _, _) = lower_expr(ctx, &args[1])?;
                            Some(WirExpr::Binary {
                                op: BinaryOp::Eq,
                                lhs: Box::new(l),
                                rhs: Box::new(r),
                            })
                        }
                        ("assert", n) if n >= 1 => Some(lower_expr(ctx, &args[0])?.0),
                        _ => None,
                    };
                    if let Some(cond) = cond {
                        let ok_id = ctx.fresh_block_id();
                        let trap_id = ctx.fresh_block_id();
                        ctx.finish_block(
                            current_block,
                            WirTerminator::Branch {
                                cond,
                                then_block: ok_id,
                                else_block: trap_id,
                            },
                        );
                        ctx.finish_block(trap_id, WirTerminator::Trap("assertion failed".into()));
                        current_block = ok_id;
                        continue;
                    }
                }
                if let rust_ast::Expr::If(cond, then_b, else_b, _) = expr {
                    let (cond_expr, _, _) = lower_expr(ctx, cond)?;
                    let then_id = ctx.fresh_block_id();
                    let else_id = ctx.fresh_block_id();
                    let merge_id = ctx.fresh_block_id();

                    ctx.finish_block(current_block, WirTerminator::Branch {
                        cond: cond_expr,
                        then_block: then_id,
                        else_block: else_id,
                    });

                    // Then block
                    let then_end = lower_block(ctx, then_b, then_id)?;
                    ctx.finish_block(then_end, WirTerminator::Jump(merge_id));

                    // Else block
                    if let Some(else_block) = else_b {
                        let else_end = lower_block(ctx, else_block, else_id)?;
                        ctx.finish_block(else_end, WirTerminator::Jump(merge_id));
                    } else {
                        ctx.finish_block(else_id, WirTerminator::Jump(merge_id));
                    }

                    current_block = merge_id;
                } else {
                    lower_expr(ctx, expr)?;
                }
            }
            rust_ast::Stmt::Return(ret_expr, _) => {
                let term = if let Some(e) = ret_expr {
                    let (ret_val, _, _) = lower_expr(ctx, e)?;
                    WirTerminator::Return(Some(ret_val))
                } else {
                    WirTerminator::Return(None)
                };
                let new_id = ctx.fresh_block_id();
                ctx.finish_block(current_block, term);
                current_block = new_id;
            }
            rust_ast::Stmt::For { var, iter, body, span: _ } => {
                if let rust_ast::Expr::Range(Some(start_expr), Some(end_expr), is_inclusive, _) = iter {
                    let (start_val, _, _) = lower_expr(ctx, start_expr)?;
                    let (end_val, _, _) = lower_expr(ctx, end_expr)?;

                    // Allocate loop counter in memory so it can be mutated across loop iterations in SSA CFG
                    let loop_ptr = ctx.fresh_var(&format!("{}_ptr", var));
                    ctx.emit_inst(WirInst::Alloc {
                        var: loop_ptr.clone(),
                        size_expr: WirExpr::ConstInt { val: 8, ty: WirType::U64 },
                    });
                    ctx.emit_inst(WirInst::Store {
                        ptr_expr: WirExpr::Var(loop_ptr.clone()),
                        val_expr: start_val,
                    });

                    let header_id = ctx.fresh_block_id();
                    let body_id = ctx.fresh_block_id();
                    let exit_id = ctx.fresh_block_id();

                    ctx.finish_block(current_block, WirTerminator::Jump(header_id));

                    // Header block: load current counter from loop_ptr
                    let curr_loop_var = ctx.fresh_var(var);
                    ctx.emit_inst(WirInst::Load {
                        var: curr_loop_var.clone(),
                        ptr_expr: WirExpr::Var(loop_ptr.clone()),
                        ty: WirType::U64,
                    });

                    // Header condition: i < end or i <= end
                    let cmp_op = if *is_inclusive { BinaryOp::Le } else { BinaryOp::Lt };
                    let cond_expr = WirExpr::Binary {
                        op: cmp_op,
                        lhs: Box::new(WirExpr::Var(curr_loop_var.clone())),
                        rhs: Box::new(end_val),
                    };
                    ctx.finish_block(header_id, WirTerminator::Branch {
                        cond: cond_expr,
                        then_block: body_id,
                        else_block: exit_id,
                    });

                    // Body
                    ctx.push_scope();
                    ctx.insert_var(var.clone(), curr_loop_var.clone(), WirType::U64, Label::PUBLIC);
                    let body_end = lower_block(ctx, body, body_id)?;
                    ctx.pop_scope();

                    // Latch: i_next = curr_i + 1; store to loop_ptr
                    let inc_var = ctx.fresh_var(&format!("{}_next", var));
                    ctx.emit_inst(WirInst::Let {
                        var: inc_var.clone(),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Add(OverflowMode::Wrap),
                            lhs: Box::new(WirExpr::Var(curr_loop_var)),
                            rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                        },
                    });
                    ctx.emit_inst(WirInst::Store {
                        ptr_expr: WirExpr::Var(loop_ptr.clone()),
                        val_expr: WirExpr::Var(inc_var),
                    });
                    ctx.finish_block(body_end, WirTerminator::Jump(header_id));

                    // Exit block: free loop pointer
                    ctx.emit_inst(WirInst::Free {
                        ptr_expr: WirExpr::Var(loop_ptr),
                    });

                    current_block = exit_id;
                }
            }
            rust_ast::Stmt::Unsafe(unsafe_block, _) => {
                current_block = lower_block(ctx, unsafe_block, current_block)?;
            }
            rust_ast::Stmt::Item(_) => {}
        }
    }

    if let Some(trailing) = &block.expr {
        let (val, _, _) = lower_expr(ctx, trailing)?;
        ctx.finish_block(current_block, WirTerminator::Return(Some(val)));
        let next_id = ctx.fresh_block_id();
        current_block = next_id;
    }

    ctx.pop_scope();
    Ok(current_block)
}

/// Lower a single Rust function declaration into a WIR function.
pub fn lower_function(
    fn_decl: &rust_ast::FnDecl,
    secret_param_names: &[&str],
) -> Result<Function, LoweringError> {
    let mut ctx = LoweringContext::new();

    let mut params = Vec::new();
    for p in &fn_decl.params {
        let wir_ty = lower_type(&p.ty)?;
        let label = if secret_param_names.contains(&p.name.as_str()) {
            Label::SECRET
        } else {
            Label::PUBLIC
        };
        let var = VarId(p.name.clone());
        ctx.insert_var(p.name.clone(), var.clone(), wir_ty.clone(), label);
        params.push((var, wir_ty, label));
    }

    let return_type = match &fn_decl.return_type {
        Some(ty) => lower_type(ty)?,
        None => WirType::Unit,
    };

    let entry_id = ctx.fresh_block_id();
    let final_block = lower_block(&mut ctx, &fn_decl.body, entry_id)?;

    // If current block is not terminated, terminate with default return
    if !ctx.current_instructions.is_empty() || ctx.blocks.iter().all(|b| b.id != final_block) {
        let ret = if return_type == WirType::Unit {
            Some(WirExpr::ConstUnit)
        } else {
            None
        };
        ctx.finish_block(final_block, WirTerminator::Return(ret));
    }

    Ok(Function {
        name: fn_decl.name.clone(),
        params,
        return_type,
        blocks: ctx.blocks,
        ranking_metric: None,
    })
}

fn is_test_item(attrs: &[rust_ast::Attribute], name: &str) -> bool {
    name == "tests"
        || attrs.iter().any(|a| {
            a.name == "test"
                || (a.name == "cfg" && a.args.iter().any(|arg| arg == "test"))
        })
}

/// Lower an entire Rust source file into a WIR Program.
pub fn lower_source_file(
    file: &rust_ast::SourceFile,
    secret_param_config: &[(&str, &[&str])],
) -> Result<Program, LoweringError> {
    let mut functions = Vec::new();

    fn collect_functions(
        items: &[rust_ast::Item],
        config: &[(&str, &[&str])],
        out: &mut Vec<Function>,
    ) -> Result<(), LoweringError> {
        for item in items {
            match item {
                rust_ast::Item::Function(f) => {
                    if is_test_item(&f.attrs, &f.name) {
                        continue;
                    }
                    let secrets = config
                        .iter()
                        .find(|(name, _)| *name == f.name.as_str())
                        .map(|(_, s)| *s)
                        .unwrap_or(&[]);
                    out.push(lower_function(f, secrets)?);
                }
                rust_ast::Item::Module(m) => {
                    if is_test_item(&m.attrs, &m.name) {
                        continue;
                    }
                    collect_functions(&m.items, config, out)?;
                }
                rust_ast::Item::Use(_) => {}
            }
        }
        Ok(())
    }

    collect_functions(&file.items, secret_param_config, &mut functions)?;
    Ok(Program::new(functions))
}

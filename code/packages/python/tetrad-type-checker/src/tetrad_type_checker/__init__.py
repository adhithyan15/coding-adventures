"""Tetrad type checker: bottom-up type inference over the Tetrad AST.

The type checker sits between the parser (TET02) and the bytecode compiler
(TET03).  It is the mechanism by which optional type annotations accelerate
both the VM and the JIT:

  Untyped:  fn add(a, b) { return a + b; }
    → compiler emits ADD r0, slot=N  (3 bytes, feedback slot allocated)
    → VM records u8×u8 on every call
    → JIT waits 100 calls before compiling

  Typed:    fn add(a: u8, b: u8) -> u8 { return a + b; }
    → compiler emits ADD r0  (2 bytes, NO slot)
    → VM skips feedback vector allocation
    → JIT compiles BEFORE the first call (immediate_jit_eligible = True)

The checker runs in four phases:

  Phase 1 — Collect function signatures (forward declarations).
             Every fn is visible to every other fn regardless of order.

  Phase 2 — Check global variables.
             Infer their types and bind them in the top-level environment.

  Phase 3 — Check each function body.
             Walk expressions bottom-up, infer types, record in type_map.

  Phase 4 — Classify each function (FULLY_TYPED / PARTIALLY_TYPED / UNTYPED).
             Emit warnings for untyped functions.

The checker never raises — all errors go into TypeCheckResult.errors.

Public API
----------
``check(program) -> TypeCheckResult``
    Type-check a parsed Program.

``check_source(source: str) -> TypeCheckResult``
    Lex + parse + type-check in one call.

``TypeCheckResult``, ``TypeError``, ``TypeWarning`` — result containers.

``TypeInfo``, ``FunctionTypeStatus``, ``TypeEnvironment`` — in types.py.
"""

from __future__ import annotations

from tetrad_parser import parse
from tetrad_parser.ast import (
    AssignStmt,
    BinaryExpr,
    Block,
    CallExpr,
    ExprStmt,
    FnDecl,
    GlobalDecl,
    GroupExpr,
    IfStmt,
    InExpr,
    IntLiteral,
    LetStmt,
    NameExpr,
    OutExpr,
    Program,
    ReturnStmt,
    UnaryExpr,
    WhileStmt,
)

from tetrad_type_checker.types import (
    FunctionType,
    FunctionTypeStatus,
    TypeCheckResult,
    TypeEnvironment,
    TypeError,
    TypeInfo,
    TypeWarning,
)

__all__ = [
    "check",
    "check_source",
    "TypeCheckResult",
    "TypeError",
    "TypeWarning",
    "TypeInfo",
    "FunctionTypeStatus",
    "TypeEnvironment",
]

# ---------------------------------------------------------------------------
# Type arithmetic
# ---------------------------------------------------------------------------

# In Tetrad v1, u8 is closed under all operations: u8 OP u8 → u8.
# Any Unknown operand propagates Unknown upward (conservative).
#
# Comparison operators (==, !=, <, <=, >, >=) always produce u8 (0 or 1)
# regardless of operand types — the result is always a boolean encoded as u8.
_COMPARISON_OPS = frozenset(["==", "!=", "<", "<=", ">", ">="])

# Logical operators (&&, ||, !) always produce u8 (0 or 1).
_LOGICAL_OPS = frozenset(["&&", "||", "!"])


def _binary_result_type(op: str, left_ty: str, right_ty: str) -> str:
    """Infer the result type of a binary operation.

    Comparison and logical operators always produce u8 — the result is 0 or 1
    regardless of operand types.  For arithmetic and bitwise ops, u8 × u8 → u8;
    any Unknown propagates Unknown.
    """
    if op in _COMPARISON_OPS or op in _LOGICAL_OPS:
        return "u8"
    if left_ty == "u8" and right_ty == "u8":
        return "u8"
    return "Unknown"


# ---------------------------------------------------------------------------
# Expression type inference
# ---------------------------------------------------------------------------


def _check_expr(
    expr: object,
    env: TypeEnvironment,
    type_map: dict[int, TypeInfo],
    errors: list[TypeError],
) -> TypeInfo:
    """Walk an expression bottom-up, infer its type, record in ``type_map``.

    Returns the TypeInfo for ``expr``.  Side effect: type_map[id(expr)] is set.

    This is a pure bottom-up pass — it does not need the return type of the
    enclosing function.  Return-type mismatch checking happens in check_block().
    """
    info: TypeInfo

    if isinstance(expr, IntLiteral):
        # Integer literals are always u8.  Range is checked by the compiler.
        info = TypeInfo(ty="u8", source="inferred", line=expr.line, column=expr.column)

    elif isinstance(expr, NameExpr):
        found = env.lookup_var(expr.name)
        if found is not None:
            info = TypeInfo(
                ty=found.ty, source=found.source, line=expr.line, column=expr.column
            )
        else:
            info = TypeInfo(
                ty="Unknown", source="unknown", line=expr.line, column=expr.column
            )

    elif isinstance(expr, BinaryExpr):
        left_info = _check_expr(expr.left, env, type_map, errors)
        right_info = _check_expr(expr.right, env, type_map, errors)
        result_ty = _binary_result_type(expr.op, left_info.ty, right_info.ty)
        info = TypeInfo(
            ty=result_ty,
            source="inferred",
            line=expr.line,
            column=expr.column,
        )

    elif isinstance(expr, UnaryExpr):
        # ! and ~ always produce u8.  Unary - preserves operand type.
        operand_info = _check_expr(expr.operand, env, type_map, errors)
        result_ty = "u8" if expr.op in ("!", "~") else operand_info.ty
        info = TypeInfo(
            ty=result_ty, source="inferred", line=expr.line, column=expr.column
        )

    elif isinstance(expr, CallExpr):
        # Check arguments (for side-effect type recording)
        for arg in expr.args:
            _check_expr(arg, env, type_map, errors)
        fn_type = env.functions.get(expr.name)
        if fn_type is not None and fn_type.return_type is not None:
            result_ty = fn_type.return_type
            source = "inferred"
        else:
            result_ty = "Unknown"
            source = "unknown"
        info = TypeInfo(
            ty=result_ty, source=source, line=expr.line, column=expr.column
        )

    elif isinstance(expr, InExpr):
        # in() reads from hardware I/O — the value type is not statically known.
        info = TypeInfo(
            ty="Unknown", source="unknown", line=expr.line, column=expr.column
        )

    elif isinstance(expr, OutExpr):
        _check_expr(expr.value, env, type_map, errors)
        info = TypeInfo(
            ty="Void", source="inferred", line=expr.line, column=expr.column
        )

    elif isinstance(expr, GroupExpr):
        inner_info = _check_expr(expr.expr, env, type_map, errors)
        info = TypeInfo(
            ty=inner_info.ty,
            source=inner_info.source,
            line=expr.line,
            column=expr.column,
        )

    else:
        info = TypeInfo(ty="Unknown", source="unknown", line=0, column=0)

    type_map[id(expr)] = info
    return info


# ---------------------------------------------------------------------------
# Statement checking
# ---------------------------------------------------------------------------


def _check_block(
    block: Block,
    env: TypeEnvironment,
    return_type: str | None,
    type_map: dict[int, TypeInfo],
    errors: list[TypeError],
    warnings: list[TypeWarning],
) -> None:
    """Check all statements in a block, threading the environment through."""
    for stmt in block.stmts:
        _check_stmt(stmt, env, return_type, type_map, errors, warnings)


def _check_stmt(
    stmt: object,
    env: TypeEnvironment,
    return_type: str | None,
    type_map: dict[int, TypeInfo],
    errors: list[TypeError],
    warnings: list[TypeWarning],
) -> None:
    """Check one statement, updating ``env`` with any new variable bindings."""
    if isinstance(stmt, LetStmt):
        inferred = _check_expr(stmt.value, env, type_map, errors)
        annotated = stmt.declared_type
        if (
            annotated is not None
            and inferred.ty != "Unknown"
            and annotated != inferred.ty
        ):
            errors.append(
                TypeError(
                    f"'{stmt.name}' declared {annotated} but got {inferred.ty}",
                    stmt.line,
                    stmt.column,
                )
            )
        # Check for annotation mismatch with Unknown (I/O assignment)
        if annotated is not None and inferred.ty == "Unknown":
            errors.append(
                TypeError(
                    f"'{stmt.name}' declared {annotated} but assigned Unknown "
                    f"(value has no static type)",
                    stmt.line,
                    stmt.column,
                )
            )
            actual_ty = annotated
        elif annotated is not None:
            actual_ty = annotated
        else:
            actual_ty = inferred.ty
        env.bind_var(
            stmt.name,
            TypeInfo(
                ty=actual_ty,
                source="annotation" if annotated else inferred.source,
                line=stmt.line,
                column=stmt.column,
            ),
        )

    elif isinstance(stmt, AssignStmt):
        inferred = _check_expr(stmt.value, env, type_map, errors)
        existing = env.lookup_var(stmt.name)
        if (
            existing is not None
            and existing.ty != "Unknown"
            and inferred.ty != "Unknown"
            and existing.ty != inferred.ty
        ):
            errors.append(
                TypeError(
                    f"'{stmt.name}' has type {existing.ty} but assigned "
                    f"{inferred.ty}",
                    stmt.line,
                    stmt.column,
                )
            )
        if existing is not None:
            env.bind_var(
                stmt.name,
                TypeInfo(
                    ty=existing.ty,
                    source=existing.source,
                    line=stmt.line,
                    column=stmt.column,
                ),
            )

    elif isinstance(stmt, ReturnStmt):
        if stmt.value is not None:
            val_info = _check_expr(stmt.value, env, type_map, errors)
            if (
                return_type is not None
                and val_info.ty == "Unknown"
                and return_type != "Unknown"
            ):
                errors.append(
                    TypeError(
                        f"declared -> {return_type} but return expression has "
                        f"unknown type",
                        stmt.line,
                        stmt.column,
                    )
                )

    elif isinstance(stmt, IfStmt):
        _check_expr(stmt.condition, env, type_map, errors)
        then_env = env.child_scope()
        _check_block(stmt.then_block, then_env, return_type, type_map, errors, warnings)
        if stmt.else_block is not None:
            else_env = env.child_scope()
            _check_block(
                stmt.else_block, else_env, return_type, type_map, errors, warnings
            )

    elif isinstance(stmt, WhileStmt):
        _check_expr(stmt.condition, env, type_map, errors)
        body_env = env.child_scope()
        _check_block(stmt.body, body_env, return_type, type_map, errors, warnings)

    elif isinstance(stmt, ExprStmt):
        _check_expr(stmt.expr, env, type_map, errors)

    elif isinstance(stmt, Block):
        child = env.child_scope()
        _check_block(stmt, child, return_type, type_map, errors, warnings)


# ---------------------------------------------------------------------------
# Function checking
# ---------------------------------------------------------------------------


def _check_fn(
    fn: FnDecl,
    env: TypeEnvironment,
    type_map: dict[int, TypeInfo],
    errors: list[TypeError],
    warnings: list[TypeWarning],
) -> None:
    """Check a function body within a child scope that has params bound."""
    local_env = env.child_scope()
    for name, ann_type in zip(fn.params, fn.param_types, strict=True):
        ty = ann_type if ann_type is not None else "Unknown"
        local_env.bind_var(
            name,
            TypeInfo(
                ty=ty,
                source="annotation" if ann_type is not None else "unknown",
                line=fn.line,
                column=fn.column,
            ),
        )
    _check_block(fn.body, local_env, fn.return_type, type_map, errors, warnings)


# ---------------------------------------------------------------------------
# Function classification
# ---------------------------------------------------------------------------


def _all_exprs_in_block(block: Block) -> list[object]:
    """Collect all expression nodes reachable from ``block``, depth-first."""
    result: list[object] = []
    for stmt in block.stmts:
        result.extend(_exprs_in_stmt(stmt))
    return result


def _exprs_in_stmt(stmt: object) -> list[object]:
    """Collect expression nodes in a single statement."""
    if isinstance(stmt, (LetStmt, AssignStmt)):
        return _exprs_in_expr(stmt.value)
    if isinstance(stmt, ReturnStmt):
        return _exprs_in_expr(stmt.value) if stmt.value is not None else []
    if isinstance(stmt, ExprStmt):
        return _exprs_in_expr(stmt.expr)
    if isinstance(stmt, IfStmt):
        result = _exprs_in_expr(stmt.condition)
        result += _all_exprs_in_block(stmt.then_block)
        if stmt.else_block is not None:
            result += _all_exprs_in_block(stmt.else_block)
        return result
    if isinstance(stmt, WhileStmt):
        return _exprs_in_expr(stmt.condition) + _all_exprs_in_block(stmt.body)
    if isinstance(stmt, Block):
        return _all_exprs_in_block(stmt)
    return []


def _exprs_in_expr(expr: object) -> list[object]:
    """Collect all expression nodes in an expression tree (including root)."""
    if expr is None:
        return []
    result = [expr]
    if isinstance(expr, BinaryExpr):
        result += _exprs_in_expr(expr.left)
        result += _exprs_in_expr(expr.right)
    elif isinstance(expr, UnaryExpr):
        result += _exprs_in_expr(expr.operand)
    elif isinstance(expr, CallExpr):
        for arg in expr.args:
            result += _exprs_in_expr(arg)
    elif isinstance(expr, OutExpr):
        result += _exprs_in_expr(expr.value)
    elif isinstance(expr, GroupExpr):
        result += _exprs_in_expr(expr.expr)
    return result


def _classify_function(
    fn: FnDecl,
    type_map: dict[int, TypeInfo],
) -> FunctionTypeStatus:
    """Classify a function into the three-tier system.

    FULLY_TYPED  — all params annotated, return annotated, all binary ops
                   and calls in the body inferred to u8.
    PARTIALLY_TYPED — some annotations present but not all, or an op yields Unknown.
    UNTYPED      — no annotations whatsoever.

    The classification drives the JIT compilation threshold:
      FULLY_TYPED → compile before first call
      PARTIALLY_TYPED → compile after 10 calls
      UNTYPED → compile after 100 calls
    """
    has_any_annotation = (
        any(t is not None for t in fn.param_types) or fn.return_type is not None
    )
    if not has_any_annotation:
        return FunctionTypeStatus.UNTYPED

    all_params_typed = all(t is not None for t in fn.param_types)
    return_typed = fn.return_type is not None
    if not (all_params_typed and return_typed):
        return FunctionTypeStatus.PARTIALLY_TYPED

    # All params and return are annotated.  Check that every binary op and call
    # in the body also inferred to a known (non-Unknown) type.
    for expr_node in _all_exprs_in_block(fn.body):
        if isinstance(expr_node, (BinaryExpr, CallExpr)):
            info = type_map.get(id(expr_node))
            if info is not None and info.ty == "Unknown":
                return FunctionTypeStatus.PARTIALLY_TYPED

    return FunctionTypeStatus.FULLY_TYPED


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------


def check(program: Program) -> TypeCheckResult:
    """Type-check a parsed Tetrad program.

    Never raises — all errors are collected in TypeCheckResult.errors.
    The caller should abort compilation if errors is non-empty.

    Algorithm:
      Phase 1 — build function signature table (enables mutual recursion)
      Phase 2 — check global variable initializers
      Phase 3 — check each function body
      Phase 4 — classify each function; emit warnings for untyped functions
    """
    type_map: dict[int, TypeInfo] = {}
    errors: list[TypeError] = []
    warnings: list[TypeWarning] = []

    # Phase 1: Collect all function signatures.
    # This must happen before checking bodies so that mutual recursion and
    # forward calls work correctly.
    env = TypeEnvironment(functions={}, variables={}, function_status={})
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            env.functions[decl.name] = FunctionType(
                param_types=decl.param_types,
                return_type=decl.return_type,
            )

    # Phase 2: Check global variable initializers.
    for decl in program.decls:
        if isinstance(decl, GlobalDecl):
            inferred = _check_expr(decl.value, env, type_map, errors)
            annotated = decl.declared_type
            if annotated is not None and inferred.ty == "Unknown":
                errors.append(
                    TypeError(
                        f"global '{decl.name}' declared {annotated} but "
                        f"assigned Unknown (value has no static type)",
                        decl.line,
                        decl.column,
                    )
                )
                actual_ty = annotated
            elif annotated is not None:
                if inferred.ty != "Unknown" and annotated != inferred.ty:
                    errors.append(
                        TypeError(
                            f"global '{decl.name}': declared {annotated}, "
                            f"got {inferred.ty}",
                            decl.line,
                            decl.column,
                        )
                    )
                actual_ty = annotated
            else:
                actual_ty = inferred.ty
            env.bind_var(
                decl.name,
                TypeInfo(
                    ty=actual_ty,
                    source="annotation" if annotated else inferred.source,
                    line=decl.line,
                    column=decl.column,
                ),
            )

    # Phase 3: Check each function body.
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            _check_fn(decl, env, type_map, errors, warnings)

    # Phase 4: Classify each function; warn about untyped ones.
    for decl in program.decls:
        if isinstance(decl, FnDecl):
            status = _classify_function(decl, type_map)
            env.function_status[decl.name] = status
            if status is FunctionTypeStatus.UNTYPED:
                warnings.append(
                    TypeWarning(
                        message=(
                            f"'{decl.name}' has no type annotations — "
                            f"JIT warmup required"
                        ),
                        line=decl.line,
                        column=decl.column,
                        hint=(
                            "add param types and -> return type to enable "
                            "immediate JIT compilation"
                        ),
                    )
                )
            elif status is FunctionTypeStatus.PARTIALLY_TYPED:
                # Warn if the function calls an untyped function, which is the
                # most common reason a typed function gets downgraded.
                _warn_if_calls_untyped(decl, env, type_map, warnings)

    return TypeCheckResult(
        program=program,
        type_map=type_map,
        env=env,
        errors=errors,
        warnings=warnings,
    )


def _warn_if_calls_untyped(
    fn: FnDecl,
    env: TypeEnvironment,
    type_map: dict[int, TypeInfo],
    warnings: list[TypeWarning],
) -> None:
    """Emit a warning for each call to an untyped callee from a typed context."""
    for expr_node in _all_exprs_in_block(fn.body):
        if isinstance(expr_node, CallExpr):
            callee_status = env.function_status.get(expr_node.name)
            if callee_status is FunctionTypeStatus.UNTYPED:
                warnings.append(
                    TypeWarning(
                        message=(
                            f"call to untyped '{expr_node.name}' in typed context "
                            f"'{fn.name}' — '{fn.name}' downgraded to PARTIALLY_TYPED"
                        ),
                        line=expr_node.line,
                        column=expr_node.column,
                        hint=(
                            f"add type annotations to '{expr_node.name}' "
                            f"to restore FULLY_TYPED"
                        ),
                    )
                )


def check_source(source: str) -> TypeCheckResult:
    """Lex + parse + type-check a Tetrad source string in one call.

    Raises ``LexError`` or ``ParseError`` if the source is syntactically invalid.
    Type errors are returned in ``TypeCheckResult.errors``.
    """
    program = parse(source)
    return check(program)

"""Lower the first ALGOL 60 compiler subset into compiler IR."""

from __future__ import annotations

from dataclasses import dataclass

from algol_type_checker import TypeCheckResult, check_algol
from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from lang_parser import ASTNode
from lexer import Token


@dataclass(frozen=True)
class CompileResult:
    """The IR compiler output and useful allocation metadata."""

    program: IrProgram
    variable_registers: dict[str, int]
    max_register: int


class CompileError(Exception):
    """Raised when checked ALGOL cannot be lowered to the current IR subset."""


@dataclass
class _RegisterScope:
    parent: _RegisterScope | None = None
    registers: dict[str, int] | None = None

    def __post_init__(self) -> None:
        if self.registers is None:
            self.registers = {}

    def declare(self, name: str, reg: int) -> None:
        if self.registers is None:
            self.registers = {}
        self.registers[name] = reg

    def resolve(self, name: str) -> int | None:
        scope: _RegisterScope | None = self
        while scope is not None:
            if scope.registers is not None and name in scope.registers:
                return scope.registers[name]
            scope = scope.parent
        return None


class AlgolIrCompiler:
    """Compile a typed ALGOL AST into the repository's register IR.

    The compiler keeps the first milestone intentionally plain. Declarations
    allocate stable virtual registers, expressions allocate throwaway
    temporaries, and structured control flow emits the label shapes that the
    existing IR-to-WASM lowerer already recognizes.
    """

    def __init__(self) -> None:
        self.ids = IDGenerator()
        self.program = IrProgram(entry_label="_start")
        self.next_reg = 2
        self.if_count = 0
        self.loop_count = 0
        self.global_registers: dict[str, int] = {}

    def compile(self, typed: TypeCheckResult | ASTNode) -> CompileResult:
        type_result = (
            typed if isinstance(typed, TypeCheckResult) else check_algol(typed)
        )
        if not type_result.ok:
            details = "\n".join(
                f"Line {diag.line}, Col {diag.column}: {diag.message}"
                for diag in type_result.diagnostics
            )
            raise CompileError(details)

        self.ids = IDGenerator()
        self.program = IrProgram(entry_label="_start")
        self.next_reg = 2
        self.if_count = 0
        self.loop_count = 0
        self.global_registers = {}

        self._label("_start")
        self._emit(IrOp.LOAD_IMM, IrRegister(0), IrImmediate(0))

        block = _first_node(type_result.ast, "block")
        if block is None:
            raise CompileError("ALGOL program must contain a block")
        root_scope = _RegisterScope()
        self._compile_block(block, root_scope, is_root=True)

        result_reg = root_scope.resolve("result")
        if result_reg is None:
            raise CompileError(
                "compiled ALGOL programs must declare integer variable 'result'"
            )
        self._emit(IrOp.ADD_IMM, IrRegister(1), IrRegister(result_reg), IrImmediate(0))
        self._emit(IrOp.HALT)

        return CompileResult(
            program=self.program,
            variable_registers=dict(self.global_registers),
            max_register=max(1, self.next_reg - 1),
        )

    def _compile_block(
        self, block: ASTNode, parent: _RegisterScope, *, is_root: bool = False
    ) -> None:
        scope = parent if is_root else _RegisterScope(parent=parent)
        for declaration in _direct_nodes(block, "declaration"):
            self._compile_declaration(declaration, scope)
        for statement in _direct_nodes(block, "statement"):
            self._compile_statement(statement, scope)

    def _compile_declaration(self, declaration: ASTNode, scope: _RegisterScope) -> None:
        type_decl = _first_node(declaration, "type_decl")
        if type_decl is None:
            raise CompileError("only integer scalar declarations can be compiled")
        ident_list = _first_node(type_decl, "ident_list")
        for token in _tokens(ident_list):
            if token.type_name != "NAME":
                continue
            reg = self._fresh_reg()
            scope.declare(token.value, reg)
            self.global_registers.setdefault(token.value, reg)
            self._emit(IrOp.LOAD_IMM, IrRegister(reg), IrImmediate(0))

    def _compile_statement(self, statement: ASTNode, scope: _RegisterScope) -> None:
        inner = _first_ast_child(statement)
        if inner is None:
            return
        if inner.rule_name == "unlabeled_stmt":
            self._compile_unlabeled(inner, scope)
        elif inner.rule_name == "cond_stmt":
            self._compile_if(inner, scope)
        else:
            raise CompileError(
                f"{inner.rule_name} is not supported by algol-ir-compiler"
            )

    def _compile_unlabeled(self, node: ASTNode, scope: _RegisterScope) -> None:
        inner = _first_ast_child(node)
        if inner is None:
            return
        if inner.rule_name == "assign_stmt":
            self._compile_assignment(inner, scope)
        elif inner.rule_name == "for_stmt":
            self._compile_for(inner, scope)
        elif inner.rule_name == "compound_stmt":
            for statement in _direct_nodes(inner, "statement"):
                self._compile_statement(statement, scope)
        elif inner.rule_name == "block":
            self._compile_block(inner, scope)
        else:
            raise CompileError(
                f"{inner.rule_name} is not supported by algol-ir-compiler"
            )

    def _compile_assignment(self, assign: ASTNode, scope: _RegisterScope) -> None:
        left_part = _first_direct_node(assign, "left_part")
        expr = _first_direct_node(assign, "expression")
        name = _variable_name(left_part)
        if name is None or expr is None:
            raise CompileError("only scalar assignments are supported")
        target = scope.resolve(name.value)
        if target is None:
            raise CompileError(f"{name.value!r} is not declared")
        value = self._compile_expr(expr, scope)
        self._emit(IrOp.ADD_IMM, IrRegister(target), IrRegister(value), IrImmediate(0))

    def _compile_if(self, cond: ASTNode, scope: _RegisterScope) -> None:
        index = self.if_count
        self.if_count += 1
        else_label = f"if_{index}_else"
        end_label = f"if_{index}_end"

        bool_expr = _first_direct_node(cond, "bool_expr")
        if bool_expr is None:
            raise CompileError("if statement is missing a condition")
        condition = self._compile_expr(bool_expr, scope)
        self._emit(IrOp.BRANCH_Z, IrRegister(condition), IrLabel(else_label))

        seen_then = False
        for child in cond.children:
            if isinstance(child, Token) and child.value == "then":
                seen_then = True
            elif (
                isinstance(child, ASTNode)
                and child.rule_name == "unlabeled_stmt"
                and seen_then
            ):
                self._compile_unlabeled(child, scope)

        self._emit(IrOp.JUMP, IrLabel(end_label))
        self._label(else_label)
        for child in cond.children:
            if isinstance(child, ASTNode) and child.rule_name == "statement":
                self._compile_statement(child, scope)
        self._label(end_label)

    def _compile_for(self, node: ASTNode, scope: _RegisterScope) -> None:
        loop_token = next(
            (tok for tok in _direct_tokens(node) if tok.type_name == "NAME"), None
        )
        if loop_token is None:
            raise CompileError("for loop is missing its control variable")
        loop_reg = scope.resolve(loop_token.value)
        if loop_reg is None:
            raise CompileError(f"{loop_token.value!r} is not declared")

        elem = _first_direct_node(_first_direct_node(node, "for_list"), "for_elem")
        pieces = _direct_nodes(elem, "arith_expr")
        if elem is None or len(pieces) != 3:
            raise CompileError("only step/until for-elements are supported")

        start = self._compile_expr(pieces[0], scope)
        self._emit(
            IrOp.ADD_IMM, IrRegister(loop_reg), IrRegister(start), IrImmediate(0)
        )

        index = self.loop_count
        self.loop_count += 1
        start_label = f"loop_{index}_start"
        end_label = f"loop_{index}_end"

        self._label(start_label)
        limit = self._compile_expr(pieces[2], scope)
        should_stop = self._fresh_reg()
        self._emit(
            IrOp.CMP_GT,
            IrRegister(should_stop),
            IrRegister(loop_reg),
            IrRegister(limit),
        )
        self._emit(IrOp.BRANCH_NZ, IrRegister(should_stop), IrLabel(end_label))

        body = _first_direct_node(node, "statement")
        if body is not None:
            self._compile_statement(body, scope)

        step = self._compile_expr(pieces[1], scope)
        self._emit(
            IrOp.ADD, IrRegister(loop_reg), IrRegister(loop_reg), IrRegister(step)
        )
        self._emit(IrOp.JUMP, IrLabel(start_label))
        self._label(end_label)

    def _compile_expr(self, expr: ASTNode | Token, scope: _RegisterScope) -> int:
        if isinstance(expr, Token):
            return self._compile_token(expr, scope)
        if expr.rule_name == "variable":
            name = _variable_name(expr)
            if name is None:
                raise CompileError("array subscripts are not supported")
            reg = scope.resolve(name.value)
            if reg is None:
                raise CompileError(f"{name.value!r} is not declared")
            return reg

        meaningful = _meaningful_children(expr)
        if not meaningful:
            raise CompileError(f"empty expression node {expr.rule_name}")
        if len(meaningful) == 1:
            return self._compile_expr(meaningful[0], scope)

        if expr.rule_name in {"expr_not", "bool_factor", "bool_secondary"}:
            first = meaningful[0]
            if isinstance(first, Token) and first.value == "not":
                return self._invert_bool(self._compile_expr(meaningful[1], scope))

        if expr.rule_name in {"expr_add", "simple_arith"} and isinstance(
            meaningful[0], Token
        ):
            operator = meaningful[0].value
            value = self._compile_expr(meaningful[1], scope)
            if operator == "+":
                return value
            if operator == "-":
                dst = self._fresh_reg()
                self._emit(IrOp.SUB, IrRegister(dst), IrRegister(0), IrRegister(value))
                return dst

        if expr.rule_name in {"expr_cmp", "relation"} and _has_comparison(meaningful):
            return self._compile_comparison(meaningful, scope)

        if expr.rule_name in {"expr_add", "simple_arith", "expr_mul", "term"}:
            return self._compile_numeric_chain(meaningful, scope)

        if expr.rule_name in {
            "expr_eqv",
            "expr_impl",
            "expr_or",
            "expr_and",
            "simple_bool",
            "implication",
            "bool_term",
        }:
            return self._compile_bool_chain(expr.rule_name, meaningful, scope)

        if expr.rule_name in {"expr_atom", "primary", "bool_primary"}:
            token = next(
                (child for child in meaningful if isinstance(child, Token)), None
            )
            if token is not None and token.value not in {"(", ")"}:
                return self._compile_token(token, scope)
            nested = next(
                (child for child in meaningful if isinstance(child, ASTNode)), None
            )
            if nested is not None:
                return self._compile_expr(nested, scope)

        if expr.rule_name in {
            "expression",
            "arith_expr",
            "bool_expr",
            "expr_pow",
            "factor",
        }:
            if any(
                isinstance(child, Token) and child.value in {"**", "^"}
                for child in meaningful
            ):
                raise CompileError("exponentiation is not supported yet")
            return self._compile_expr(meaningful[0], scope)

        return self._compile_expr(meaningful[0], scope)

    def _compile_token(self, token: Token, scope: _RegisterScope) -> int:
        if token.type_name == "INTEGER_LIT":
            dst = self._fresh_reg()
            self._emit(IrOp.LOAD_IMM, IrRegister(dst), IrImmediate(int(token.value)))
            return dst
        if token.value in {"true", "false"}:
            dst = self._fresh_reg()
            self._emit(
                IrOp.LOAD_IMM,
                IrRegister(dst),
                IrImmediate(1 if token.value == "true" else 0),
            )
            return dst
        if token.type_name == "NAME":
            reg = scope.resolve(token.value)
            if reg is None:
                raise CompileError(f"{token.value!r} is not declared")
            return reg
        raise CompileError(f"unsupported expression token {token.value!r}")

    def _compile_numeric_chain(
        self, children: list[ASTNode | Token], scope: _RegisterScope
    ) -> int:
        current = self._compile_expr(children[0], scope)
        index = 1
        while index < len(children):
            operator = children[index]
            right = self._compile_expr(children[index + 1], scope)
            if not isinstance(operator, Token):
                raise CompileError("expected numeric operator")
            current = self._emit_numeric(operator.value, current, right)
            index += 2
        return current

    def _compile_bool_chain(
        self,
        rule_name: str,
        children: list[ASTNode | Token],
        scope: _RegisterScope,
    ) -> int:
        current = self._compile_expr(children[0], scope)
        index = 1
        while index < len(children):
            operator = children[index]
            right = self._compile_expr(children[index + 1], scope)
            if not isinstance(operator, Token):
                raise CompileError("expected boolean operator")
            value = operator.value
            if value == "and" or rule_name in {"expr_and", "bool_term"}:
                dst = self._fresh_reg()
                self._emit(
                    IrOp.AND, IrRegister(dst), IrRegister(current), IrRegister(right)
                )
                current = dst
            elif value == "or" or rule_name in {"expr_or", "simple_bool"}:
                summed = self._fresh_reg()
                dst = self._fresh_reg()
                self._emit(
                    IrOp.ADD, IrRegister(summed), IrRegister(current), IrRegister(right)
                )
                self._emit(
                    IrOp.CMP_NE, IrRegister(dst), IrRegister(summed), IrRegister(0)
                )
                current = dst
            else:
                raise CompileError(f"boolean operator {value!r} is not supported yet")
            index += 2
        return current

    def _compile_comparison(
        self, children: list[ASTNode | Token], scope: _RegisterScope
    ) -> int:
        left = self._compile_expr(children[0], scope)
        operator = children[1]
        right = self._compile_expr(children[2], scope)
        if not isinstance(operator, Token):
            raise CompileError("expected comparison operator")
        dst = self._fresh_reg()
        if operator.value == "=":
            self._emit(
                IrOp.CMP_EQ, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
        elif operator.value == "!=":
            self._emit(
                IrOp.CMP_NE, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
        elif operator.value == "<":
            self._emit(
                IrOp.CMP_LT, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
        elif operator.value == ">":
            self._emit(
                IrOp.CMP_GT, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
        elif operator.value == "<=":
            self._emit(
                IrOp.CMP_GT, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
            dst = self._invert_bool(dst)
        elif operator.value == ">=":
            self._emit(
                IrOp.CMP_LT, IrRegister(dst), IrRegister(left), IrRegister(right)
            )
            dst = self._invert_bool(dst)
        else:
            raise CompileError(
                f"comparison operator {operator.value!r} is not supported"
            )
        return dst

    def _emit_numeric(self, operator: str, left: int, right: int) -> int:
        dst = self._fresh_reg()
        if operator == "+":
            self._emit(IrOp.ADD, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "-":
            self._emit(IrOp.SUB, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "*":
            self._emit(IrOp.MUL, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "div":
            self._emit(IrOp.DIV, IrRegister(dst), IrRegister(left), IrRegister(right))
        elif operator == "mod":
            quotient = self._fresh_reg()
            product = self._fresh_reg()
            self._emit(
                IrOp.DIV, IrRegister(quotient), IrRegister(left), IrRegister(right)
            )
            self._emit(
                IrOp.MUL, IrRegister(product), IrRegister(quotient), IrRegister(right)
            )
            self._emit(IrOp.SUB, IrRegister(dst), IrRegister(left), IrRegister(product))
        else:
            raise CompileError(f"numeric operator {operator!r} is not supported")
        return dst

    def _invert_bool(self, source: int) -> int:
        dst = self._fresh_reg()
        self._emit(IrOp.ADD_IMM, IrRegister(dst), IrRegister(source), IrImmediate(-1))
        self._emit(IrOp.AND_IMM, IrRegister(dst), IrRegister(dst), IrImmediate(1))
        return dst

    def _fresh_reg(self) -> int:
        reg = self.next_reg
        self.next_reg += 1
        return reg

    def _label(self, name: str) -> None:
        self.program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel(name)], id=-1))

    def _emit(
        self, opcode: IrOp, *operands: IrRegister | IrImmediate | IrLabel
    ) -> None:
        self.program.add_instruction(
            IrInstruction(opcode, list(operands), id=self.ids.next())
        )


def compile_algol(typed: TypeCheckResult | ASTNode) -> CompileResult:
    return AlgolIrCompiler().compile(typed)


def _node_children(node: ASTNode | None) -> list[ASTNode]:
    if node is None:
        return []
    return [child for child in node.children if isinstance(child, ASTNode)]


def _direct_nodes(node: ASTNode | None, rule_name: str) -> list[ASTNode]:
    return [child for child in _node_children(node) if child.rule_name == rule_name]


def _first_direct_node(node: ASTNode | None, rule_name: str) -> ASTNode | None:
    return next(iter(_direct_nodes(node, rule_name)), None)


def _first_ast_child(node: ASTNode) -> ASTNode | None:
    return next((child for child in node.children if isinstance(child, ASTNode)), None)


def _direct_tokens(node: ASTNode | None) -> list[Token]:
    if node is None:
        return []
    return [child for child in node.children if isinstance(child, Token)]


def _tokens(node: ASTNode | None) -> list[Token]:
    if node is None:
        return []
    found: list[Token] = []
    for child in node.children:
        if isinstance(child, Token):
            found.append(child)
        else:
            found.extend(_tokens(child))
    return found


def _meaningful_children(node: ASTNode) -> list[ASTNode | Token]:
    return [
        child
        for child in node.children
        if not (isinstance(child, Token) and child.value in {"(", ")"})
    ]


def _first_node(node: ASTNode, rule_name: str) -> ASTNode | None:
    if node.rule_name == rule_name:
        return node
    for child in _node_children(node):
        found = _first_node(child, rule_name)
        if found is not None:
            return found
    return None


def _variable_name(node: ASTNode | None) -> Token | None:
    if node is None:
        return None
    variable = node if node.rule_name == "variable" else _first_node(node, "variable")
    if variable is None:
        return None
    names = [token for token in _tokens(variable) if token.type_name == "NAME"]
    return names[0] if len(names) == 1 else None


def _has_comparison(children: list[ASTNode | Token]) -> bool:
    return any(
        isinstance(child, Token) and child.value in {"=", "!=", "<", "<=", ">", ">="}
        for child in children
    )

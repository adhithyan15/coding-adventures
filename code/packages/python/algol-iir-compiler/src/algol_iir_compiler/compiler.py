"""ALGOL 60 to InterpreterIR compiler.

This is the first generic-backend ALGOL slice: scalar declarations,
assignments, arithmetic/comparison expressions, structured conditionals,
local labels/goto, and one-element ``for step until`` loops.  The output is a
single fully typed ``IIRFunction`` named ``main``.  A source variable named
``result`` becomes the function return value; otherwise ``main`` returns void.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Never

from algol_parser import parse_algol
from algol_type_checker import TypeCheckResult, assert_algol_typed
from interpreter_ir import FunctionTypeStatus, IIRFunction, IIRInstr, IIRModule
from lang_parser import ASTNode
from lexer import Token

from algol_iir_compiler.errors import (
    AlgolIIRCompileError,
    AlgolIIRUnsupportedError,
)

_ALGOL_TO_IIR_TYPE = {
    "integer": "i32",
    "real": "f64",
    "boolean": "bool",
}

_DEFAULT_VALUES: dict[str, int | float | bool] = {
    "i32": 0,
    "f64": 0.0,
    "bool": False,
}

_ARITHMETIC_OPS = {
    "+": "add",
    "-": "sub",
    "*": "mul",
    "/": "div",
    "div": "div",
}

_COMPARISON_OPS = {
    "=": "cmp_eq",
    "<>": "cmp_ne",
    "!=": "cmp_ne",
    "≠": "cmp_ne",
    "<": "cmp_lt",
    "<=": "cmp_le",
    "≤": "cmp_le",
    ">": "cmp_gt",
    ">=": "cmp_ge",
    "≥": "cmp_ge",
}

_EXPR_WRAPPERS = {
    "expression",
    "expr_eqv",
    "expr_impl",
    "expr_or",
    "expr_and",
    "expr_not",
    "expr_cmp",
    "expr_add",
    "expr_mul",
    "expr_pow",
    "expr_atom",
    "bool_expr",
    "simple_bool",
    "implication",
    "bool_term",
    "bool_factor",
    "bool_secondary",
    "bool_primary",
    "relation",
    "arith_expr",
    "simple_arith",
    "term",
    "factor",
    "primary",
}


def compile_source(
    source: str,
    *,
    module_name: str = "algol60",
) -> IIRModule:
    """Parse, type-check, and lower ALGOL source into ``IIRModule``."""
    return compile_to_iir(parse_algol(source), module_name=module_name)


def compile_to_iir(
    typed: TypeCheckResult | ASTNode,
    *,
    module_name: str = "algol60",
) -> IIRModule:
    """Lower a typed ALGOL program or parsed AST into ``IIRModule``."""
    type_result = assert_algol_typed(typed) if isinstance(typed, ASTNode) else typed
    if not type_result.ok:
        raise AlgolIIRCompileError("cannot lower ALGOL program with diagnostics")
    compiler = _Compiler(type_result)
    fn = compiler.compile()
    return IIRModule(
        name=module_name,
        functions=[fn],
        entry_point="main",
        language="algol60",
    )


class _Compiler:
    def __init__(self, typed: TypeCheckResult) -> None:
        self.typed = typed
        self.ast = typed.ast
        self.instructions: list[IIRInstr] = []
        self.variables: dict[str, str] = {}
        self.register_types: dict[str, str] = {}
        self.register_names: set[str] = set()
        self.temp_index = 0
        self.label_index = 0

    def compile(self) -> IIRFunction:
        program_block = _first_direct_node(self.ast, "block")
        if program_block is None:
            raise AlgolIIRCompileError("ALGOL program has no block")

        self._emit_block(program_block, allow_declarations=True)

        result_type = self.variables.get("result")
        if result_type is not None:
            self._emit("ret", None, ["result"], result_type)
            return_type = result_type
        else:
            self._emit("ret_void", None, [], "void")
            return_type = "void"

        return IIRFunction(
            name="main",
            params=[],
            return_type=return_type,
            instructions=self.instructions,
            register_count=max(len(self.register_names) + 4, 8),
            type_status=FunctionTypeStatus.FULLY_TYPED,
        )

    # ------------------------------------------------------------------
    # Blocks and statements
    # ------------------------------------------------------------------

    def _emit_block(self, block: ASTNode, *, allow_declarations: bool) -> None:
        declarations = _direct_nodes(block, "declaration")
        if declarations and not allow_declarations:
            self._unsupported(
                block,
                "nested block declarations need ALGOL frame-layout lowering",
            )
        for declaration in declarations:
            self._emit_declaration(declaration)
        for statement in _direct_nodes(block, "statement"):
            self._emit_statement(statement)

    def _emit_declaration(self, declaration: ASTNode) -> None:
        inner = _first_ast_child(declaration)
        if inner is None:
            return
        if inner.rule_name != "type_decl":
            self._unsupported(
                inner,
                f"{inner.rule_name} is not part of the scalar IIR slice",
            )

        source_type = _first_token_value(_first_direct_node(inner, "type"))
        if source_type is None:
            raise AlgolIIRCompileError(_at(inner, "type declaration has no type"))
        iir_type = _ALGOL_TO_IIR_TYPE.get(source_type.lower())
        if iir_type is None:
            self._unsupported(
                inner,
                f"{source_type} declarations need runtime lowering",
            )

        ident_list = _first_direct_node(inner, "ident_list")
        for token in _tokens(ident_list):
            if token.type_name != "NAME":
                continue
            name = token.value
            if name in self.variables:
                raise AlgolIIRCompileError(_at(token, f"duplicate scalar {name!r}"))
            self.variables[name] = iir_type
            self.register_types[name] = iir_type
            self.register_names.add(name)
            self._emit("const", name, [_DEFAULT_VALUES[iir_type]], iir_type)

    def _emit_statement(self, statement: ASTNode) -> None:
        for label in _direct_nodes(statement, "label"):
            self._emit("label", None, [_label_name(label)], "void")
        body = next(
            (
                child
                for child in _direct_ast_children(statement)
                if child.rule_name != "label"
            ),
            None,
        )
        if body is not None:
            self._emit_statement_body(body)

    def _emit_statement_body(self, node: ASTNode) -> None:
        if node.rule_name in {"statement", "unlabeled_stmt"}:
            child = _first_ast_child(node)
            if child is not None:
                self._emit_statement_body(child)
            return
        if node.rule_name == "assign_stmt":
            self._emit_assignment(node)
        elif node.rule_name == "cond_stmt":
            self._emit_if_statement(node)
        elif node.rule_name == "compound_stmt":
            for statement in _direct_nodes(node, "statement"):
                self._emit_statement(statement)
        elif node.rule_name == "block":
            self._emit_block(node, allow_declarations=False)
        elif node.rule_name == "goto_stmt":
            self._emit_goto(node)
        elif node.rule_name == "for_stmt":
            self._emit_for(node)
        elif node.rule_name == "dummy_stmt":
            return
        else:
            self._unsupported(node, f"{node.rule_name} is not part of this IIR slice")

    def _emit_assignment(self, node: ASTNode) -> None:
        left_parts = _direct_nodes(node, "left_part")
        expression = _first_direct_node(node, "expression")
        if expression is None:
            raise AlgolIIRCompileError(_at(node, "assignment has no expression"))

        value_name, value_type = self._emit_expr(expression)
        for left in left_parts:
            variable = _first_direct_node(left, "variable")
            target = self._simple_variable_name(variable)
            target_type = self.variables[target]
            if target_type != value_type:
                raise AlgolIIRCompileError(
                    _at(left, f"cannot assign {value_type} to {target_type} {target!r}")
                )
            if target != value_name:
                self._emit("mov", target, [value_name], target_type)

    def _emit_if_statement(self, node: ASTNode) -> None:
        children = list(node.children)
        then_index = _keyword_index(children, "then")
        if then_index is None:
            raise AlgolIIRCompileError(_at(node, "if statement has no then"))
        else_index = _keyword_index(children, "else")
        end_index = else_index if else_index is not None else len(children)

        condition = _first_ast_between(children, 0, then_index, {"bool_expr"})
        then_branch = _first_ast_between(children, then_index + 1, end_index)
        else_branch = (
            _first_ast_between(children, else_index + 1, len(children))
            if else_index is not None
            else None
        )
        if condition is None or then_branch is None:
            raise AlgolIIRCompileError(_at(node, "malformed if statement"))

        cond_name, cond_type = self._emit_expr(condition)
        if cond_type != "bool":
            raise AlgolIIRCompileError(_at(condition, "if condition must be boolean"))

        false_label = self._fresh_label("if_false")
        end_label = self._fresh_label("if_end")
        self._emit("jmp_if_false", None, [cond_name, false_label], "void")
        self._emit_statement_body(then_branch)
        if else_branch is not None:
            self._emit("jmp", None, [end_label], "void")
        self._emit("label", None, [false_label], "void")
        if else_branch is not None:
            self._emit_statement_body(else_branch)
            self._emit("label", None, [end_label], "void")

    def _emit_goto(self, node: ASTNode) -> None:
        target = _first_direct_node(node, "desig_expr")
        if target is None:
            raise AlgolIIRCompileError(_at(node, "goto has no designational target"))
        if _keyword_index(list(target.children), "if") is not None:
            self._unsupported(target, "conditional designational goto needs lowering")
        self._emit("jmp", None, [_label_name(target)], "void")

    def _emit_for(self, node: ASTNode) -> None:
        variable = _first_direct_node(node, "variable")
        target = self._simple_variable_name(variable)
        target_type = self.variables[target]
        for_list = _first_direct_node(node, "for_list")
        elements = _direct_nodes(for_list, "for_elem")
        if len(elements) != 1:
            self._unsupported(node, "multi-element for lists need dispatch lowering")
        element = elements[0]
        arith_exprs = _direct_nodes(element, "arith_expr")
        if (
            len(arith_exprs) != 3
            or _first_direct_node(element, "bool_expr") is not None
        ):
            self._unsupported(element, "only step-until for elements are supported")

        start_name, start_type = self._emit_expr(arith_exprs[0])
        step_name, step_type = self._emit_expr(arith_exprs[1])
        until_name, until_type = self._emit_expr(arith_exprs[2])
        if {start_type, step_type, until_type, target_type} - {"i32", "f64"}:
            self._unsupported(element, "for control values must be numeric scalars")
        if (
            start_type != target_type
            or step_type != target_type
            or until_type != target_type
        ):
            self._unsupported(element, "mixed-type for controls need coercion lowering")

        step_value = _constant_number(arith_exprs[1])
        if step_value is None:
            self._unsupported(
                element,
                "dynamic step sign needs runtime branch lowering",
            )
        compare_op = "cmp_le" if step_value >= 0 else "cmp_ge"

        self._emit("mov", target, [start_name], target_type)
        start_label = self._fresh_label("for_start")
        end_label = self._fresh_label("for_end")
        cond_name = self._fresh_temp("bool")
        self._emit("label", None, [start_label], "void")
        self._emit(compare_op, cond_name, [target, until_name], "bool")
        self._emit("jmp_if_false", None, [cond_name, end_label], "void")

        body = self._for_body(node)
        self._emit_statement_body(body)
        next_name = self._fresh_temp(target_type)
        self._emit("add", next_name, [target, step_name], target_type)
        self._emit("mov", target, [next_name], target_type)
        self._emit("jmp", None, [start_label], "void")
        self._emit("label", None, [end_label], "void")

    # ------------------------------------------------------------------
    # Expressions
    # ------------------------------------------------------------------

    def _emit_expr(self, node: ASTNode) -> tuple[str, str]:
        conditional = self._conditional_expression_parts(node)
        if conditional is not None:
            return self._emit_conditional_expression(node, conditional)

        if node.rule_name == "variable":
            name = self._simple_variable_name(node)
            return name, self.variables[name]

        token = _single_token(node)
        if token is not None:
            return self._emit_token_expr(token)

        if node.rule_name == "bool_secondary" and _starts_with_keyword(node, "not"):
            children = _direct_ast_children(node)
            if not children:
                raise AlgolIIRCompileError(_at(node, "not expression has no operand"))
            src_name, src_type = self._emit_expr(children[-1])
            if src_type != "bool":
                raise AlgolIIRCompileError(_at(node, "not operand must be boolean"))
            false_name = self._fresh_temp("bool")
            dest = self._fresh_temp("bool")
            self._emit("const", false_name, [False], "bool")
            self._emit("cmp_eq", dest, [src_name, false_name], "bool")
            return dest, "bool"

        unary = self._emit_unary_numeric(node)
        if unary is not None:
            return unary

        if node.rule_name in _EXPR_WRAPPERS:
            binary = self._emit_binary_sequence(node)
            if binary is not None:
                return binary
            children = _direct_ast_children(node)
            if len(children) == 1:
                return self._emit_expr(children[0])

        self._unsupported(node, f"expression rule {node.rule_name} needs lowering")

    def _emit_binary_sequence(self, node: ASTNode) -> tuple[str, str] | None:
        parts = [
            child
            for child in node.children
            if isinstance(child, ASTNode) or _is_operator_token(child)
        ]
        if not any(isinstance(child, Token) for child in parts):
            return None

        rest = parts
        if isinstance(rest[0], Token) and rest[0].value in {"+", "-"}:
            if len(rest) < 2 or not isinstance(rest[1], ASTNode):
                raise AlgolIIRCompileError(_at(node, "malformed unary expression"))
            left_name, left_type = self._emit_expr(rest[1])
            if rest[0].value == "-":
                left_name, left_type = self._emit_negated(rest[0], left_name, left_type)
            rest = rest[2:]
        elif isinstance(rest[0], ASTNode):
            left_name, left_type = self._emit_expr(rest[0])
            rest = rest[1:]
        else:
            raise AlgolIIRCompileError(_at(node, "malformed binary expression"))

        if len(rest) % 2 != 0:
            raise AlgolIIRCompileError(_at(node, "malformed binary expression"))

        for op_token, right_node in zip(rest[0::2], rest[1::2], strict=True):
            if not isinstance(op_token, Token) or not isinstance(right_node, ASTNode):
                raise AlgolIIRCompileError(_at(node, "malformed binary expression"))
            op_value = op_token.value.lower()
            right_name, right_type = self._emit_expr(right_node)
            if op_value in {"and", "or", "⊃", "≡"}:
                self._unsupported(
                    op_token,
                    "logical operators need common-backend boolean lowering",
                )
            if op_value in _COMPARISON_OPS:
                dest = self._fresh_temp("bool")
                self._emit(
                    _COMPARISON_OPS[op_value],
                    dest,
                    [left_name, right_name],
                    "bool",
                )
                left_name, left_type = dest, "bool"
                continue
            if op_value not in _ARITHMETIC_OPS:
                self._unsupported(
                    op_token,
                    f"operator {op_token.value!r} is unsupported",
                )
            if left_type == "bool" or right_type == "bool":
                raise AlgolIIRCompileError(
                    _at(op_token, "boolean arithmetic is invalid")
                )
            result_type = "f64" if "f64" in {left_type, right_type} else "i32"
            if _ARITHMETIC_OPS[op_value] == "div" and result_type == "f64":
                self._unsupported(op_token, "real division needs VM/runtime lowering")
            dest = self._fresh_temp(result_type)
            self._emit(
                _ARITHMETIC_OPS[op_value],
                dest,
                [left_name, right_name],
                result_type,
            )
            left_name, left_type = dest, result_type
        return left_name, left_type

    def _emit_unary_numeric(self, node: ASTNode) -> tuple[str, str] | None:
        children = _direct_ast_children(node)
        tokens = [
            token
            for token in _direct_tokens(node)
            if token.value in {"+", "-"}
        ]
        if len(tokens) != 1 or len(children) != 1:
            return None
        src_name, src_type = self._emit_expr(children[0])
        if tokens[0].value == "+":
            return src_name, src_type
        return self._emit_negated(tokens[0], src_name, src_type)

    def _emit_negated(
        self,
        token: Token,
        src_name: str,
        src_type: str,
    ) -> tuple[str, str]:
        if src_type not in {"i32", "f64"}:
            raise AlgolIIRCompileError(_at(token, "unary minus needs numeric input"))
        dest = self._fresh_temp(src_type)
        self._emit("neg", dest, [src_name], src_type)
        return dest, src_type

    def _emit_conditional_expression(
        self,
        node: ASTNode,
        parts: tuple[ASTNode, ASTNode, ASTNode],
    ) -> tuple[str, str]:
        condition, then_expr, else_expr = parts
        result_type = self._iir_type_for_node(node)
        dest = self._fresh_temp(result_type)
        else_label = self._fresh_label("expr_else")
        end_label = self._fresh_label("expr_end")

        cond_name, cond_type = self._emit_expr(condition)
        if cond_type != "bool":
            raise AlgolIIRCompileError(_at(condition, "condition must be boolean"))
        self._emit("jmp_if_false", None, [cond_name, else_label], "void")
        then_name, then_type = self._emit_expr(then_expr)
        if then_type != result_type:
            self._unsupported(then_expr, "conditional expression coercion")
        self._emit("mov", dest, [then_name], result_type)
        self._emit("jmp", None, [end_label], "void")
        self._emit("label", None, [else_label], "void")
        else_name, else_type = self._emit_expr(else_expr)
        if else_type != result_type:
            self._unsupported(else_expr, "conditional expression coercion")
        self._emit("mov", dest, [else_name], result_type)
        self._emit("label", None, [end_label], "void")
        return dest, result_type

    def _emit_token_expr(self, token: Token) -> tuple[str, str]:
        if token.type_name == "INTEGER_LIT":
            dest = self._fresh_temp("i32")
            self._emit("const", dest, [int(token.value)], "i32")
            return dest, "i32"
        if token.type_name == "REAL_LIT":
            dest = self._fresh_temp("f64")
            self._emit("const", dest, [float(token.value)], "f64")
            return dest, "f64"
        if token.type_name == "NAME":
            name = token.value
            if name not in self.variables:
                raise AlgolIIRCompileError(_at(token, f"name {name!r} is not a scalar"))
            return name, self.variables[name]
        if token.type_name == "KEYWORD" and token.value.lower() in {"true", "false"}:
            dest = self._fresh_temp("bool")
            self._emit("const", dest, [token.value.lower() == "true"], "bool")
            return dest, "bool"
        self._unsupported(token, f"token {token.value!r} needs expression lowering")

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _emit(
        self,
        op: str,
        dest: str | None,
        srcs: list[str | int | float | bool],
        type_hint: str,
    ) -> None:
        self.instructions.append(
            IIRInstr(op=op, dest=dest, srcs=srcs, type_hint=type_hint)
        )
        if dest is not None:
            self.register_names.add(dest)
            self.register_types[dest] = type_hint

    def _fresh_temp(self, type_hint: str) -> str:
        name = f"__algol_t{self.temp_index}"
        self.temp_index += 1
        self.register_names.add(name)
        self.register_types[name] = type_hint
        return name

    def _fresh_label(self, stem: str) -> str:
        name = f"algol_{stem}_{self.label_index}"
        self.label_index += 1
        return name

    def _simple_variable_name(self, node: ASTNode | None) -> str:
        if node is None:
            raise AlgolIIRCompileError("expected variable")
        direct_names = [
            token for token in _direct_tokens(node) if token.type_name == "NAME"
        ]
        if (
            len(direct_names) != 1
            or _first_direct_node(node, "subscript_list") is not None
        ):
            self._unsupported(node, "array designators need storage lowering")
        name = direct_names[0].value
        if name not in self.variables:
            raise AlgolIIRCompileError(_at(node, f"name {name!r} is not a scalar"))
        return name

    def _for_body(self, node: ASTNode) -> ASTNode:
        children = list(node.children)
        do_index = _keyword_index(children, "do")
        if do_index is None:
            raise AlgolIIRCompileError(_at(node, "for statement has no do"))
        body = _first_ast_between(children, do_index + 1, len(children))
        if body is None:
            raise AlgolIIRCompileError(_at(node, "for statement has no body"))
        return body

    def _conditional_expression_parts(
        self,
        node: ASTNode,
    ) -> tuple[ASTNode, ASTNode, ASTNode] | None:
        if node.rule_name != "expression" or not _starts_with_keyword(node, "if"):
            return None
        children = list(node.children)
        then_index = _keyword_index(children, "then")
        else_index = _keyword_index(children, "else")
        if then_index is None or else_index is None or then_index >= else_index:
            raise AlgolIIRCompileError(_at(node, "malformed conditional expression"))
        condition = _first_ast_between(children, 1, then_index)
        then_expr = _first_ast_between(children, then_index + 1, else_index)
        else_expr = _first_ast_between(children, else_index + 1, len(children))
        if condition is None or then_expr is None or else_expr is None:
            raise AlgolIIRCompileError(_at(node, "malformed conditional expression"))
        return condition, then_expr, else_expr

    def _iir_type_for_node(self, node: ASTNode) -> str:
        algol_type = self.typed.expression_types.get(id(node))
        iir_type = _ALGOL_TO_IIR_TYPE.get(algol_type or "")
        if iir_type is not None:
            return iir_type
        token = _single_token(node)
        if token is not None:
            token_type = self._token_expr_type(token)
            if token_type is not None:
                return token_type
        children = _direct_ast_children(node)
        if len(children) == 1:
            return self._iir_type_for_node(children[0])
        self._unsupported(node, "cannot infer IIR type for expression")

    def _token_expr_type(self, token: Token) -> str | None:
        if token.type_name == "INTEGER_LIT":
            return "i32"
        if token.type_name == "REAL_LIT":
            return "f64"
        if token.type_name == "NAME":
            return self.variables.get(token.value)
        if token.type_name == "KEYWORD" and token.value.lower() in {"true", "false"}:
            return "bool"
        return None

    def _unsupported(self, obj: ASTNode | Token, message: str) -> Never:
        raise AlgolIIRUnsupportedError(_at(obj, message))


def _direct_ast_children(node: ASTNode | None) -> list[ASTNode]:
    if node is None:
        return []
    return [child for child in node.children if isinstance(child, ASTNode)]


def _direct_nodes(node: ASTNode | None, rule_name: str) -> list[ASTNode]:
    return [
        child for child in _direct_ast_children(node) if child.rule_name == rule_name
    ]


def _first_direct_node(node: ASTNode | None, rule_name: str) -> ASTNode | None:
    return next(iter(_direct_nodes(node, rule_name)), None)


def _first_ast_child(node: ASTNode | None) -> ASTNode | None:
    return next(iter(_direct_ast_children(node)), None)


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


def _single_token(node: ASTNode) -> Token | None:
    tokens = _tokens(node)
    meaningful = [
        token
        for token in tokens
        if token.value not in {"(", ")"} and token.type_name not in {"SEMICOLON"}
    ]
    return meaningful[0] if len(meaningful) == 1 else None


def _first_token_value(node: ASTNode | None) -> str | None:
    token = next(iter(_tokens(node)), None)
    return token.value if token is not None else None


def _is_operator_token(child: ASTNode | Token) -> bool:
    if not isinstance(child, Token):
        return False
    value = child.value.lower()
    return value in _ARITHMETIC_OPS or value in _COMPARISON_OPS or value in {
        "and",
        "or",
        "⊃",
        "≡",
    }


def _starts_with_keyword(node: ASTNode, keyword: str) -> bool:
    first = next(iter(node.children), None)
    return (
        isinstance(first, Token)
        and first.type_name == "KEYWORD"
        and first.value.lower() == keyword
    )


def _keyword_index(children: Sequence[ASTNode | Token], keyword: str) -> int | None:
    for index, child in enumerate(children):
        if (
            isinstance(child, Token)
            and child.type_name == "KEYWORD"
            and child.value.lower() == keyword
        ):
            return index
    return None


def _first_ast_between(
    children: Sequence[ASTNode | Token],
    start: int,
    end: int,
    rule_names: set[str] | None = None,
) -> ASTNode | None:
    for child in children[start:end]:
        if isinstance(child, ASTNode) and (
            rule_names is None or child.rule_name in rule_names
        ):
            return child
    return None


def _label_name(node: ASTNode) -> str:
    for token in _tokens(node):
        if token.type_name in {"NAME", "INTEGER_LIT"}:
            return token.value
    raise AlgolIIRCompileError(_at(node, "label has no name"))


def _constant_number(node: ASTNode) -> float | None:
    tokens = [
        token
        for token in _tokens(node)
        if token.value not in {"(", "+"} and token.type_name not in {"SEMICOLON"}
    ]
    if not tokens:
        return None
    sign = 1.0
    if len(tokens) == 2 and tokens[0].value == "-":
        sign = -1.0
        tokens = tokens[1:]
    if len(tokens) != 1:
        return None
    token = tokens[0]
    if token.type_name == "INTEGER_LIT":
        return sign * int(token.value)
    if token.type_name == "REAL_LIT":
        return sign * float(token.value)
    return None


def _at(obj: ASTNode | Token, message: str) -> str:
    line = getattr(obj, "start_line", None) or getattr(obj, "line", None)
    column = getattr(obj, "start_column", None) or getattr(obj, "column", None)
    if line is None or column is None:
        return message
    return f"Line {line}, Col {column}: {message}"

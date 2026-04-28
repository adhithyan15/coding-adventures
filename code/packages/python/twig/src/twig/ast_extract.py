"""Convert the generic ``ASTNode`` tree into typed Twig nodes.

The grammar-driven parser emits one ASTNode per non-terminal (e.g.
``program``, ``form``, ``expr``, ``compound``, ``apply``).  These
wrapping non-terminals exist to make the grammar readable but they
have no semantic content of their own — the *meaningful* shapes
(``define``, ``if_form``, ``apply``, …) are nested inside.

This module walks the generic tree and lifts each meaningful
subtree into a typed node from :mod:`twig.ast_nodes`.  Compiler and
analyser code downstream then deals with a small, exhaustive set of
classes rather than a sea of ``rule_name`` checks.

The conversion is a straightforward recursive descent.  Each
extractor function consumes one ``ASTNode`` and returns one typed
node, or raises :class:`TwigParseError` if the shape is unexpected
(which only happens if the grammar file has drifted out of sync
with this module — both are kept aligned by tests).
"""

from __future__ import annotations

from typing import Any

from lang_parser import ASTNode

from twig.ast_nodes import (
    Apply,
    Begin,
    BoolLit,
    Define,
    Expr,
    Form,
    If,
    IntLit,
    Lambda,
    Let,
    NilLit,
    Program,
    SymLit,
    VarRef,
)
from twig.errors import TwigParseError

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _is_token(node: Any, type_name: str | None = None) -> bool:
    """True iff *node* is a Token (not an ASTNode), optionally of
    a given ``type``.

    The lang_parser stream includes ``Token`` objects whose ``type``
    is either a ``TokenType`` enum member (``LPAREN``, ``RPAREN``,
    ``KEYWORD``, ``NAME``) or a custom string emitted by the lexer
    (``"INTEGER"``, ``"BOOL_TRUE"``, ``"BOOL_FALSE"``).  The grammar
    lexer normalises these uniformly so we just compare the string
    representation.
    """
    if isinstance(node, ASTNode):
        return False
    if type_name is None:
        return True
    actual = getattr(node, "type", None)
    if actual is None:
        return False
    name = getattr(actual, "name", None) or str(actual)
    return name == type_name


def _ast_children(node: ASTNode) -> list[ASTNode]:
    """Return only ``ASTNode`` children, skipping bare tokens.

    Many grammar rules include literal punctuation like ``LPAREN`` /
    ``RPAREN`` in the children list; downstream code is rarely
    interested in those.  This helper filters them out so we can
    focus on the structural sub-rules.
    """
    return [c for c in node.children if isinstance(c, ASTNode)]


def _expect_rule(node: ASTNode, *names: str) -> None:
    if node.rule_name not in names:
        raise TwigParseError(
            f"expected {' | '.join(names)} but got {node.rule_name!r}"
        )


def _pos(node: ASTNode | Any) -> tuple[int | None, int | None]:
    """Best-effort source-position extraction."""
    line = getattr(node, "start_line", None) or getattr(node, "line", None)
    col = getattr(node, "start_column", None) or getattr(node, "column", None)
    return line, col


# ---------------------------------------------------------------------------
# Public entry
# ---------------------------------------------------------------------------


def extract_program(root: ASTNode) -> Program:
    """Convert a parsed ``program`` ASTNode into a :class:`Program`."""
    _expect_rule(root, "program")
    forms: list[Form] = []
    for child in _ast_children(root):
        _expect_rule(child, "form")
        forms.append(_extract_form(child))
    return Program(forms=forms)


# ---------------------------------------------------------------------------
# Forms (define | expr)
# ---------------------------------------------------------------------------


def _extract_form(node: ASTNode) -> Form:
    # ``form = define | expr ;`` — the AST has exactly one ASTNode child.
    inner = _ast_children(node)[0]
    if inner.rule_name == "define":
        return _extract_define(inner)
    if inner.rule_name == "expr":
        return _extract_expr(inner)
    raise TwigParseError(f"unexpected form child: {inner.rule_name!r}")


def _extract_define(node: ASTNode) -> Define:
    # define = LPAREN "define" name_or_signature expr { expr } RPAREN
    line, col = _pos(node)
    children = _ast_children(node)
    sig_node = children[0]
    body_exprs = [_extract_expr(c) for c in children[1:]]
    if not body_exprs:
        raise TwigParseError("(define …) must have a body expression",
                             line=line, column=col)

    # name_or_signature = NAME | LPAREN NAME { NAME } RPAREN
    sig_children = sig_node.children
    name_tokens = [c for c in sig_children if _is_token(c, "NAME")]
    if not name_tokens:
        raise TwigParseError("(define …) missing a name", line=line, column=col)

    if len(name_tokens) == 1 and not _has_paren(sig_children):
        # Plain (define name expr) — single body expression expected.
        if len(body_exprs) != 1:
            raise TwigParseError(
                "(define name expr) takes exactly one body expression — "
                "use (define (name args...) body+) for multi-expression bodies",
                line=line, column=col,
            )
        return Define(
            name=str(name_tokens[0].value),
            expr=body_exprs[0],
            line=line, column=col,
        )

    # Function-sugar form: (define (name args...) body+)
    fn_name = str(name_tokens[0].value)
    params = [str(t.value) for t in name_tokens[1:]]
    lam = Lambda(params=params, body=body_exprs, line=line, column=col)
    return Define(name=fn_name, expr=lam, line=line, column=col)


def _has_paren(children: list[Any]) -> bool:
    return any(_is_token(c, "LPAREN") for c in children)


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------


def _extract_expr(node: ASTNode) -> Expr:
    _expect_rule(node, "expr")
    inner = _ast_children(node)[0]
    if inner.rule_name == "atom":
        return _extract_atom(inner)
    if inner.rule_name == "quoted":
        return _extract_quoted(inner)
    if inner.rule_name == "compound":
        return _extract_compound(inner)
    raise TwigParseError(f"unexpected expr child: {inner.rule_name!r}")


def _extract_atom(node: ASTNode) -> Expr:
    # atom = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME
    line, col = _pos(node)
    # Atom children are bare tokens, not ASTNodes.
    tok = next((c for c in node.children if not isinstance(c, ASTNode)), None)
    if tok is None:
        raise TwigParseError("empty atom", line=line, column=col)
    type_name = getattr(getattr(tok, "type", None), "name", None) or str(tok.type)

    if type_name == "INTEGER":
        return IntLit(value=int(tok.value), line=line, column=col)
    if type_name == "BOOL_TRUE":
        return BoolLit(value=True, line=line, column=col)
    if type_name == "BOOL_FALSE":
        return BoolLit(value=False, line=line, column=col)
    if type_name == "KEYWORD" and tok.value == "nil":
        return NilLit(line=line, column=col)
    if type_name == "NAME":
        return VarRef(name=str(tok.value), line=line, column=col)
    raise TwigParseError(
        f"unexpected atom token: type={type_name!r} value={tok.value!r}",
        line=line, column=col,
    )


def _extract_quoted(node: ASTNode) -> SymLit:
    # quoted = QUOTE NAME — bare tokens, no nested ASTNodes
    line, col = _pos(node)
    name_tok = next(
        (c for c in node.children if _is_token(c, "NAME")), None
    )
    if name_tok is None:
        raise TwigParseError("expected NAME after '", line=line, column=col)
    return SymLit(name=str(name_tok.value), line=line, column=col)


def _extract_compound(node: ASTNode) -> Expr:
    inner = _ast_children(node)[0]
    line, col = _pos(node)
    rule = inner.rule_name
    if rule == "if_form":
        return _extract_if(inner)
    if rule == "let_form":
        return _extract_let(inner)
    if rule == "begin_form":
        return _extract_begin(inner)
    if rule == "lambda_form":
        return _extract_lambda(inner)
    if rule == "quote_form":
        return _extract_quote_form(inner)
    if rule == "apply":
        return _extract_apply(inner)
    raise TwigParseError(
        f"unexpected compound child: {rule!r}", line=line, column=col,
    )


def _extract_if(node: ASTNode) -> If:
    # if_form = LPAREN "if" expr expr expr RPAREN
    line, col = _pos(node)
    exprs = [_extract_expr(c) for c in _ast_children(node) if c.rule_name == "expr"]
    if len(exprs) != 3:
        raise TwigParseError(
            "(if …) takes exactly 3 expressions",
            line=line, column=col,
        )
    return If(
        cond=exprs[0],
        then_branch=exprs[1],
        else_branch=exprs[2],
        line=line, column=col,
    )


def _extract_let(node: ASTNode) -> Let:
    # let_form = LPAREN "let" LPAREN { binding } RPAREN expr { expr } RPAREN
    line, col = _pos(node)
    bindings: list[tuple[str, Expr]] = []
    body: list[Expr] = []
    for child in _ast_children(node):
        if child.rule_name == "binding":
            bindings.append(_extract_binding(child))
        elif child.rule_name == "expr":
            body.append(_extract_expr(child))
    if not body:
        raise TwigParseError(
            "(let (...) ...) needs at least one body expression",
            line=line, column=col,
        )
    return Let(bindings=bindings, body=body, line=line, column=col)


def _extract_binding(node: ASTNode) -> tuple[str, Expr]:
    # binding = LPAREN NAME expr RPAREN
    name_tok = next((c for c in node.children if _is_token(c, "NAME")), None)
    expr_node = next(
        (c for c in node.children if isinstance(c, ASTNode) and c.rule_name == "expr"),
        None,
    )
    if name_tok is None or expr_node is None:
        line, col = _pos(node)
        raise TwigParseError(
            "malformed binding — expected (name expr)",
            line=line, column=col,
        )
    return str(name_tok.value), _extract_expr(expr_node)


def _extract_begin(node: ASTNode) -> Begin:
    # begin_form = LPAREN "begin" expr { expr } RPAREN
    line, col = _pos(node)
    exprs = [_extract_expr(c) for c in _ast_children(node) if c.rule_name == "expr"]
    if not exprs:
        raise TwigParseError(
            "(begin …) needs at least one expression",
            line=line, column=col,
        )
    return Begin(exprs=exprs, line=line, column=col)


def _extract_lambda(node: ASTNode) -> Lambda:
    # lambda_form = LPAREN "lambda" LPAREN { NAME } RPAREN expr { expr } RPAREN
    line, col = _pos(node)
    # Walk children: collect the NAMEs (params) before the first expr.
    params: list[str] = []
    body: list[Expr] = []
    seen_first_expr = False
    for child in node.children:
        if isinstance(child, ASTNode) and child.rule_name == "expr":
            body.append(_extract_expr(child))
            seen_first_expr = True
        elif _is_token(child, "NAME") and not seen_first_expr:
            params.append(str(child.value))
    if not body:
        raise TwigParseError(
            "(lambda (...) …) needs at least one body expression",
            line=line, column=col,
        )
    return Lambda(params=params, body=body, line=line, column=col)


def _extract_quote_form(node: ASTNode) -> SymLit:
    # quote_form = LPAREN "quote" NAME RPAREN
    line, col = _pos(node)
    name_tok = next((c for c in node.children if _is_token(c, "NAME")), None)
    if name_tok is None:
        raise TwigParseError(
            "(quote …) needs a name",
            line=line, column=col,
        )
    return SymLit(name=str(name_tok.value), line=line, column=col)


def _extract_apply(node: ASTNode) -> Apply:
    # apply = LPAREN expr { expr } RPAREN
    line, col = _pos(node)
    exprs = [_extract_expr(c) for c in _ast_children(node) if c.rule_name == "expr"]
    if not exprs:
        raise TwigParseError(
            "empty application '()' — use 'nil' for the empty list",
            line=line, column=col,
        )
    return Apply(fn=exprs[0], args=exprs[1:], line=line, column=col)

"""Tetrad parser: Pratt expression parser + recursive-descent statement parser.

The parser consumes the token stream from ``tetrad_lexer.tokenize()`` and
produces an Abstract Syntax Tree rooted at ``Program``.

Architecture
------------
Expression parsing uses a **Pratt parser** (top-down operator precedence,
TDOP).  The idea: every token type has an optional *null denotation* (NUD,
used when the token starts an expression) and an optional *left denotation*
(LED, used when the token appears in the middle with something already parsed
to its left).  Each LED carries a **binding power** — the precedence number.

``parse_expr(min_bp)`` drives the core loop:

    1. Call NUD of the current token → get ``left``
    2. Peek at the next token.  If its binding power ≤ ``min_bp``, stop.
    3. Otherwise consume the operator, build ``BinaryExpr(op, left, right)``
       where ``right = parse_expr(op_bp)`` (same bp → left-associative).
    4. Repeat from step 2 with the new ``left``.

Binding powers (higher = tighter binding):

    ||  →  10      &&  →  20
    == !=  →  30   < > <= >=  →  40
    |  →  50       ^  →  60    &  →  70
    << >>  →  80   + -  →  90  * / %  →  100
    unary prefix  →  110  (always wins)

Statement and declaration parsing use plain recursive descent — they dispatch
on the leading token without any precedence games.

Public API
----------
``parse(source: str) -> Program``
    Tokenize and parse.  Raises ``LexError`` or ``ParseError``.

``ParseError``
    Carries ``.message``, ``.line``, ``.column``.

All AST types live in ``tetrad_parser.ast`` and are re-exported here for
convenience.
"""

from __future__ import annotations

from tetrad_lexer import Token, TokenType, tokenize

from tetrad_parser.ast import (
    AssignStmt,
    BinaryExpr,
    Block,
    CallExpr,
    Expr,
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
    Stmt,
    UnaryExpr,
    WhileStmt,
)

__all__ = [
    "parse",
    "ParseError",
    "Program",
    "FnDecl",
    "GlobalDecl",
    "Block",
    "LetStmt",
    "AssignStmt",
    "IfStmt",
    "WhileStmt",
    "ReturnStmt",
    "ExprStmt",
    "IntLiteral",
    "NameExpr",
    "BinaryExpr",
    "UnaryExpr",
    "CallExpr",
    "InExpr",
    "OutExpr",
    "GroupExpr",
]


# ---------------------------------------------------------------------------
# Error type
# ---------------------------------------------------------------------------


class ParseError(Exception):
    """Raised on any syntax error.  Carries source position."""

    def __init__(self, message: str, line: int, column: int) -> None:
        super().__init__(f"{message} at line {line} col {column}")
        self.message = message
        self.line = line
        self.column = column


# ---------------------------------------------------------------------------
# Binding power table (Pratt precedence)
# ---------------------------------------------------------------------------

# Maps infix token type → left binding power.
# All Tetrad binary operators are left-associative, so right_bp = left_bp.
_INFIX_BP: dict[TokenType, int] = {
    TokenType.PIPE_PIPE: 10,
    TokenType.AMP_AMP: 20,
    TokenType.EQ_EQ: 30,
    TokenType.BANG_EQ: 30,
    TokenType.LT: 40,
    TokenType.GT: 40,
    TokenType.LT_EQ: 40,
    TokenType.GT_EQ: 40,
    TokenType.PIPE: 50,
    TokenType.CARET: 60,
    TokenType.AMP: 70,
    TokenType.SHL: 80,
    TokenType.SHR: 80,
    TokenType.PLUS: 90,
    TokenType.MINUS: 90,
    TokenType.STAR: 100,
    TokenType.SLASH: 100,
    TokenType.PERCENT: 100,
}

# Maps infix token type → operator string stored in BinaryExpr.op.
_OP_STR: dict[TokenType, str] = {
    TokenType.PLUS: "+",
    TokenType.MINUS: "-",
    TokenType.STAR: "*",
    TokenType.SLASH: "/",
    TokenType.PERCENT: "%",
    TokenType.AMP: "&",
    TokenType.PIPE: "|",
    TokenType.CARET: "^",
    TokenType.SHL: "<<",
    TokenType.SHR: ">>",
    TokenType.EQ_EQ: "==",
    TokenType.BANG_EQ: "!=",
    TokenType.LT: "<",
    TokenType.LT_EQ: "<=",
    TokenType.GT: ">",
    TokenType.GT_EQ: ">=",
    TokenType.AMP_AMP: "&&",
    TokenType.PIPE_PIPE: "||",
}

# Unary prefix operators bind at bp=110, which is above every binary op.
# This ensures ``-a * b`` parses as ``(-a) * b``, not ``-(a * b)``.
_UNARY_BP = 110


# ---------------------------------------------------------------------------
# Parser
# ---------------------------------------------------------------------------


class _Parser:
    """Stateful Pratt + recursive-descent parser over a token list."""

    def __init__(self, tokens: list[Token]) -> None:
        self._tokens = tokens
        self._pos = 0

    # ------------------------------------------------------------------
    # Low-level token navigation
    # ------------------------------------------------------------------

    def _peek(self) -> Token:
        """Return the token at the current position without consuming it."""
        return self._tokens[self._pos]

    def _peek_next(self) -> Token:
        """Return the token one position ahead (used for 2-token lookahead)."""
        pos = self._pos + 1
        if pos < len(self._tokens):
            return self._tokens[pos]
        return self._tokens[-1]  # EOF

    def _advance(self) -> Token:
        """Consume and return the current token."""
        tok = self._tokens[self._pos]
        if self._pos < len(self._tokens) - 1:
            self._pos += 1
        return tok

    def _expect(self, tt: TokenType) -> Token:
        """Consume the current token, raising ParseError if it is not ``tt``."""
        tok = self._peek()
        if tok.type is not tt:
            raise ParseError(
                f"expected {tt.value}, got {tok.type.value}",
                tok.line,
                tok.column,
            )
        return self._advance()

    # ------------------------------------------------------------------
    # Top-level entry
    # ------------------------------------------------------------------

    def parse_program(self) -> Program:
        """Parse a complete Tetrad source file into a ``Program`` node."""
        decls: list[FnDecl | GlobalDecl] = []
        while self._peek().type is not TokenType.EOF:
            decls.append(self._parse_top_decl())
        return Program(decls=decls)

    # ------------------------------------------------------------------
    # Declarations
    # ------------------------------------------------------------------

    def _parse_top_decl(self) -> FnDecl | GlobalDecl:
        tok = self._peek()
        if tok.type is TokenType.KW_FN:
            return self._parse_fn_decl()
        if tok.type is TokenType.KW_LET:
            return self._parse_global_decl()
        raise ParseError(
            f"expected fn or let at top level, got {tok.type.value}",
            tok.line,
            tok.column,
        )

    def _parse_fn_decl(self) -> FnDecl:
        """Parse: ``fn NAME ( params? ) [ -> type ] block``"""
        fn_tok = self._expect(TokenType.KW_FN)
        name_tok = self._expect(TokenType.IDENT)
        name: str = name_tok.value  # type: ignore[assignment]
        self._expect(TokenType.LPAREN)

        params: list[str] = []
        param_types: list[str | None] = []
        if self._peek().type is not TokenType.RPAREN:
            pname, ptype = self._parse_param()
            params.append(pname)
            param_types.append(ptype)
            while self._peek().type is TokenType.COMMA:
                self._advance()
                pname, ptype = self._parse_param()
                params.append(pname)
                param_types.append(ptype)

        self._expect(TokenType.RPAREN)

        return_type: str | None = None
        if self._peek().type is TokenType.ARROW:
            self._advance()
            return_type = self._parse_type("->")

        body = self._parse_block()
        return FnDecl(
            name=name,
            params=params,
            param_types=param_types,
            return_type=return_type,
            body=body,
            line=fn_tok.line,
            column=fn_tok.column,
        )

    def _parse_param(self) -> tuple[str, str | None]:
        """Parse a function parameter: ``NAME`` or ``NAME : type``."""
        tok = self._expect(TokenType.IDENT)
        name: str = tok.value  # type: ignore[assignment]
        if self._peek().type is TokenType.COLON:
            self._advance()
            return name, self._parse_type(":")
        return name, None

    def _parse_type(self, context: str = "") -> str:
        """Parse a type annotation.  Only ``u8`` is valid in Tetrad v1.

        ``context`` is a hint for error messages (e.g. ``"->"`` or ``":"``).
        """
        tok = self._peek()
        if tok.type is TokenType.KW_U8:
            self._advance()
            return "u8"
        if tok.type is TokenType.IDENT:
            bad: str = tok.value  # type: ignore[assignment]
            self._advance()
            raise ParseError(
                f"unknown type '{bad}'; only 'u8' is valid",
                tok.line,
                tok.column,
            )
        if tok.type is TokenType.EOF:
            raise ParseError(
                f"expected type after '{context}', got EOF",
                tok.line,
                tok.column,
            )
        raise ParseError(
            f"expected type name, got {tok.type.value}",
            tok.line,
            tok.column,
        )

    def _parse_global_decl(self) -> GlobalDecl:
        """Parse a top-level: ``let NAME [ : type ] = expr ;``"""
        let_tok = self._expect(TokenType.KW_LET)
        name_tok = self._expect(TokenType.IDENT)
        name: str = name_tok.value  # type: ignore[assignment]
        declared_type: str | None = None
        if self._peek().type is TokenType.COLON:
            self._advance()
            declared_type = self._parse_type(":")
        self._expect(TokenType.EQ)
        value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return GlobalDecl(
            name=name,
            declared_type=declared_type,
            value=value,
            line=let_tok.line,
            column=let_tok.column,
        )

    # ------------------------------------------------------------------
    # Statements
    # ------------------------------------------------------------------

    def _parse_block(self) -> Block:
        """Parse ``{ stmt* }``."""
        lbrace = self._expect(TokenType.LBRACE)
        stmts: list[Stmt] = []
        while self._peek().type not in (TokenType.RBRACE, TokenType.EOF):
            stmts.append(self._parse_stmt())
        self._expect(TokenType.RBRACE)
        return Block(stmts=stmts, line=lbrace.line, column=lbrace.column)

    def _parse_stmt(self) -> Stmt:
        """Dispatch to the correct statement parser based on the leading token."""
        tok = self._peek()
        if tok.type is TokenType.KW_LET:
            return self._parse_let_stmt()
        if tok.type is TokenType.KW_IF:
            return self._parse_if_stmt()
        if tok.type is TokenType.KW_WHILE:
            return self._parse_while_stmt()
        if tok.type is TokenType.KW_RETURN:
            return self._parse_return_stmt()
        # Two-token lookahead: IDENT followed by = (not ==) is assignment.
        if tok.type is TokenType.IDENT and self._peek_next().type is TokenType.EQ:
            return self._parse_assign_stmt()
        return self._parse_expr_stmt()

    def _parse_let_stmt(self) -> LetStmt:
        """Parse ``let NAME [ : type ] = expr ;``"""
        let_tok = self._expect(TokenType.KW_LET)
        name_tok = self._expect(TokenType.IDENT)
        name: str = name_tok.value  # type: ignore[assignment]
        declared_type: str | None = None
        if self._peek().type is TokenType.COLON:
            self._advance()
            declared_type = self._parse_type(":")
        self._expect(TokenType.EQ)
        value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return LetStmt(
            name=name,
            declared_type=declared_type,
            value=value,
            line=let_tok.line,
            column=let_tok.column,
        )

    def _parse_assign_stmt(self) -> AssignStmt:
        """Parse ``NAME = expr ;``  (only reached when next-next token is ``=``)."""
        name_tok = self._expect(TokenType.IDENT)
        name: str = name_tok.value  # type: ignore[assignment]
        self._expect(TokenType.EQ)
        value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return AssignStmt(
            name=name,
            value=value,
            line=name_tok.line,
            column=name_tok.column,
        )

    def _parse_if_stmt(self) -> IfStmt:
        """Parse ``if expr block [ else block ]``."""
        if_tok = self._expect(TokenType.KW_IF)
        condition = self._parse_expr()
        then_block = self._parse_block()
        else_block: Block | None = None
        if self._peek().type is TokenType.KW_ELSE:
            self._advance()
            else_block = self._parse_block()
        return IfStmt(
            condition=condition,
            then_block=then_block,
            else_block=else_block,
            line=if_tok.line,
            column=if_tok.column,
        )

    def _parse_while_stmt(self) -> WhileStmt:
        """Parse ``while expr block``."""
        while_tok = self._expect(TokenType.KW_WHILE)
        condition = self._parse_expr()
        body = self._parse_block()
        return WhileStmt(
            condition=condition,
            body=body,
            line=while_tok.line,
            column=while_tok.column,
        )

    def _parse_return_stmt(self) -> ReturnStmt:
        """Parse ``return [ expr ] ;``."""
        ret_tok = self._expect(TokenType.KW_RETURN)
        value: Expr | None = None
        if self._peek().type is not TokenType.SEMI:
            value = self._parse_expr()
        self._expect(TokenType.SEMI)
        return ReturnStmt(value=value, line=ret_tok.line, column=ret_tok.column)

    def _parse_expr_stmt(self) -> ExprStmt:
        """Parse ``expr ;``."""
        expr = self._parse_expr()
        self._expect(TokenType.SEMI)
        return ExprStmt(expr=expr, line=expr.line, column=expr.column)

    # ------------------------------------------------------------------
    # Expressions — Pratt parser
    # ------------------------------------------------------------------

    def _parse_expr(self, min_bp: int = 0) -> Expr:
        """Core Pratt expression parser.

        Conceptually:
          1. Parse the left-hand side via NUD (null denotation).
          2. While the next token is a binary operator with bp > min_bp,
             consume it and parse the right side with parse_expr(same_bp).
          3. Wrap left and right in BinaryExpr and continue.

        Because all Tetrad binary operators are left-associative, the
        right side always re-enters at the same binding power.  This means
        ``1 + 2 + 3`` builds ``((1 + 2) + 3)`` — the ``+`` on the right
        does NOT absorb the third ``+`` into its own subtree.
        """
        tok = self._advance()
        left = self._nud(tok)

        while True:
            op_tok = self._peek()
            bp = _INFIX_BP.get(op_tok.type)
            if bp is None or bp <= min_bp:
                break
            self._advance()  # consume operator
            right = self._parse_expr(bp)  # left-associative: right_bp = left_bp
            left = BinaryExpr(
                op=_OP_STR[op_tok.type],
                left=left,
                right=right,
                line=op_tok.line,
                column=op_tok.column,
            )

        return left

    def _nud(self, tok: Token) -> Expr:
        """Null denotation: parse ``tok`` as the START of an expression."""

        # --- Integer literals (decimal and hex both carry int value) ---
        if tok.type in (TokenType.INT, TokenType.HEX):
            return IntLiteral(
                value=tok.value,  # type: ignore[arg-type]
                line=tok.line,
                column=tok.column,
            )

        # --- Identifier: variable reference or function call ---
        if tok.type is TokenType.IDENT:
            name: str = tok.value  # type: ignore[assignment]
            if self._peek().type is TokenType.LPAREN:
                return self._parse_call(name, tok)
            return NameExpr(name=name, line=tok.line, column=tok.column)

        # --- I/O read: in() ---
        if tok.type is TokenType.KW_IN:
            if self._peek().type is not TokenType.LPAREN:
                raise ParseError(
                    "in must be called as in(), not as a bare name",
                    tok.line,
                    tok.column,
                )
            self._advance()  # consume (
            self._expect(TokenType.RPAREN)
            return InExpr(line=tok.line, column=tok.column)

        # --- I/O write: out(expr) ---
        if tok.type is TokenType.KW_OUT:
            self._expect(TokenType.LPAREN)
            value = self._parse_expr()
            self._expect(TokenType.RPAREN)
            return OutExpr(value=value, line=tok.line, column=tok.column)

        # --- Unary operators ---
        if tok.type is TokenType.MINUS:
            operand = self._parse_expr(_UNARY_BP)
            return UnaryExpr(op="-", operand=operand, line=tok.line, column=tok.column)

        if tok.type is TokenType.BANG:
            operand = self._parse_expr(_UNARY_BP)
            return UnaryExpr(op="!", operand=operand, line=tok.line, column=tok.column)

        if tok.type is TokenType.TILDE:
            operand = self._parse_expr(_UNARY_BP)
            return UnaryExpr(op="~", operand=operand, line=tok.line, column=tok.column)

        # --- Grouped expression: (expr) ---
        if tok.type is TokenType.LPAREN:
            inner = self._parse_expr(0)
            self._expect(TokenType.RPAREN)
            return GroupExpr(expr=inner, line=tok.line, column=tok.column)

        raise ParseError(
            f"unexpected token {tok.type.value} in expression",
            tok.line,
            tok.column,
        )

    def _parse_call(self, name: str, name_tok: Token) -> CallExpr:
        """Parse the argument list of a function call: ``( [arg, ...] )``."""
        self._advance()  # consume (
        args: list[Expr] = []
        if self._peek().type is not TokenType.RPAREN:
            args.append(self._parse_expr())
            while self._peek().type is TokenType.COMMA:
                self._advance()
                args.append(self._parse_expr())
        if self._peek().type is not TokenType.RPAREN:
            raise ParseError(
                f"unclosed argument list for call to '{name}'",
                self._peek().line,
                self._peek().column,
            )
        self._advance()  # consume )
        return CallExpr(
            name=name, args=args, line=name_tok.line, column=name_tok.column
        )


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def parse(source: str) -> Program:
    """Tokenize and parse a Tetrad source string.

    Calls ``tetrad_lexer.tokenize()`` internally, then runs the Pratt parser.

    Raises
    ------
    LexError
        On illegal characters or malformed literals.
    ParseError
        On syntax errors in the token stream.
    """
    tokens = tokenize(source)
    return _Parser(tokens).parse_program()

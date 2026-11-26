"""Recursive-descent parser for ``.warrant`` documents (the second half of the reader).

The grammar is VER04 section 5 plus the v1 additions of VER04j (tuples,
records, variants, ``match`` patterns, fixed-width literals and static type
arguments) and the VER04k WX-G ``inout`` parameters and ``ensures`` blocks.

Precedence, highest first (VER04 section 4)::

    call / static application / projection
    unary ``not`` and minus
    ``*``
    ``+``  ``-``
    comparisons and equality         (never chained)
    ``and``
    ``or``
    ``=>``                           (right associative)
    quantification, ``if``, ``match`` (extend as far right as possible)

The parser is *fail-closed*: an unknown declaration, a v1 construct in a v0
document or a chained comparison is a ``parse`` error, never skipped.  Nesting is
capped at 64 levels (VER04 section 2) so a hostile document cannot exhaust the
Python stack, and ``RecursionError`` is converted into a ``limit`` error as a
second line of defence.
"""

from __future__ import annotations

import sys

from . import ast as A
from .errors import OracleError
from .lexer import MAX_SOURCE_BYTES, Token, decode_source, split_int, tokenize

MAX_NESTING = 64
MAX_DECLARATIONS = 4096
MAX_TUPLE_ARITY = 8

#: Built-ins that take ``<...>`` static arguments (VER04j section 2.3 and 3.3).
STATIC_BUILTINS = frozenset(
    ["widen", "wrap", "try", "max_value", "min_value", "from_be", "from_le", "fixed"]
)
#: Words that can never be a name in expression position.  VER04 section 2
#: lists the declaration keywords (``op``, ``claim`` ...) as reserved too, but the
#: shipped ``arithmetic.warrant`` has a parameter called ``op``, so the Rust parser
#: evidently treats them as contextual; the oracle follows the behaviour the
#: adopters need and rejects only the words that *start or join expressions*.
RESERVED = frozenset(
    ["if", "then", "else", "match", "with", "forall", "exists", "not", "and", "or", "true", "false"]
)
_COMPARE = ("==", "!=", "<", "<=", ">", ">=")
_V1_ONLY = frozenset({"record", "variant", "ensures"})


class _Parser:
    def __init__(self, tokens: list[Token]) -> None:
        self.toks = tokens
        self.i = 0
        self.depth = 0
        self.version = "v0"
        self.static_names: set[str] = set(STATIC_BUILTINS)
        self._collect_static_ops()

    # -- token helpers -------------------------------------------------------

    def _collect_static_ops(self) -> None:
        """Pre-scan ``op NAME <`` so ``f<3>(x)`` is a static application.

        The ambiguity of ``f<T>(x)`` against a comparison is resolved exactly as
        VER04j section 2.4 says: only a callable declared with static
        parameters (or a static built-in) treats ``<`` as an application.
        """
        t = self.toks
        for k in range(len(t) - 2):
            if (
                t[k].kind == "NAME"
                and t[k].text == "op"
                and t[k + 1].kind == "NAME"
                and t[k + 2].kind == "OP"
                and t[k + 2].text == "<"
            ):
                self.static_names.add(t[k + 1].text)

    def peek(self, k: int = 0) -> Token:
        j = self.i + k
        return self.toks[j] if j < len(self.toks) else self.toks[-1]

    def next(self) -> Token:
        tok = self.toks[self.i]
        if tok.kind != "EOF":
            self.i += 1
        return tok

    def fail(self, message: str) -> OracleError:
        tok = self.peek()
        return OracleError("parse", f"line {tok.line}: {message}")

    def is_op(self, text: str, k: int = 0) -> bool:
        tok = self.peek(k)
        return tok.kind == "OP" and tok.text == text

    def is_word(self, text: str, k: int = 0) -> bool:
        tok = self.peek(k)
        return tok.kind == "NAME" and tok.text == text

    def eat_op(self, text: str) -> Token:
        if not self.is_op(text):
            raise self.fail(f"expected `{text}`")
        return self.next()

    def eat_word(self, text: str) -> Token:
        if not self.is_word(text):
            raise self.fail(f"expected `{text}`")
        return self.next()

    def eat_kind(self, kind: str) -> Token:
        if self.peek().kind != kind:
            raise self.fail(f"expected {kind}")
        return self.next()

    def ident(self) -> str:
        tok = self.peek()
        if tok.kind != "NAME" or tok.text in RESERVED:
            raise self.fail("expected an identifier")
        self.next()
        return tok.text

    def descend(self) -> None:
        self.depth += 1
        if self.depth > MAX_NESTING:
            raise OracleError("limit", "nesting exceeds 64 levels")

    def ascend(self) -> None:
        self.depth -= 1

    # -- document ------------------------------------------------------------

    def document(self) -> A.Document:
        self.eat_word("warrant")
        name = self.ident()
        version = "v0"
        if self.peek().kind == "NAME":
            if self.peek().text != "v1":
                raise self.fail("the only version marker is `v1`")
            self.next()
            version = "v1"
        self.version = version
        if self.is_word("profile"):
            # VER33 4.9: ``warrant NAME v1 profile algorithm``.  A profile needs a
            # ``v1`` document and ``algorithm`` is the only one (the Rust reader's
            # W0801 rules).  The oracle verifies *corpus* subjects; an algorithm
            # document is a model file with no ops, so it is named and refused,
            # never read as a pure document and never skipped.
            if version != "v1":
                raise self.fail("a profile needs a `v1` document")
            self.next()
            if not self.is_word("algorithm"):
                raise self.fail("the only profile is `algorithm`")
            self.next()
            if self.peek().kind == "NAME":
                raise self.fail("a header has at most one profile clause")
            raise OracleError(
                "level",
                "a `profile algorithm` document is a model file, not a corpus subject",
                level="profile.algorithm",
            )
        self.eat_kind("NEWLINE")
        self.eat_kind("INDENT")
        decls: list = []
        while self.peek().kind not in ("DEDENT", "EOF"):
            if len(decls) >= MAX_DECLARATIONS:
                raise OracleError("limit", "more than 4,096 declarations")
            decls.append(self.declaration())
        self.eat_kind("DEDENT")
        self.eat_kind("EOF")
        return A.Document(name, version, tuple(decls))

    def declaration(self):
        tok = self.peek()
        if tok.kind != "NAME":
            raise self.fail("expected a declaration")
        word = tok.text
        if word in _V1_ONLY and self.version != "v1":
            raise self.fail(f"`{word}` is a v1 construct in a v0 document")
        handler = {
            "error": self.error_decl,
            "model": self.model_decl,
            "record": self.record_decl,
            "variant": self.variant_decl,
            "op": self.op_decl,
            "example": self.example_decl,
            "claim": self.claim_decl,
            "assume": self.assume_decl,
            "labels": self.labels_decl,
            "input": self.input_decl,
            "bound": self.bound_decl,
            "ensures": self.ensures_decl,
        }.get(word)
        if handler is None:
            raise self.fail("unknown declaration")
        return handler()

    # -- types ---------------------------------------------------------------

    def type_(self, depth: int = 0) -> A.Ty:
        if depth > MAX_NESTING:
            raise OracleError("limit", "type nesting exceeds 64 levels")
        if self.is_op("("):
            self.next()
            items = [self.type_(depth + 1)]
            while self.is_op(","):
                self.next()
                items.append(self.type_(depth + 1))
            self.eat_op(")")
            if not 2 <= len(items) <= MAX_TUPLE_ARITY:
                raise self.fail("a tuple type has 2 to 8 components")
            return A.Ty("()", tuple(items))
        head = self.ident_type_name()
        if not self.is_op("<"):
            return A.Ty(head)
        self.next()
        args: list = []
        while True:
            if self.peek().kind == "INT":
                value, suffix = split_int(self.next().text)
                if suffix is not None:
                    raise self.fail("a width is an unsuffixed integer")
                args.append(value)
            else:
                args.append(self.type_(depth + 1))
            if self.is_op(","):
                self.next()
                continue
            break
        self.eat_op(">")
        if head == "Bytes":
            if len(args) != 1:
                raise self.fail("Bytes takes one width")
            only = args[0]
            if isinstance(only, A.Ty):
                if only.args:
                    raise self.fail("a width is a number or a static name")
                args = [only.head]
        return A.Ty(head, tuple(args))

    def ident_type_name(self) -> str:
        tok = self.peek()
        if tok.kind != "NAME" or (tok.text in RESERVED):
            raise self.fail("expected a type")
        self.next()
        return tok.text

    # -- declarations --------------------------------------------------------

    def end_line(self) -> None:
        self.eat_kind("NEWLINE")

    def error_decl(self) -> A.ErrorDecl:
        line = self.next().line
        name = self.ident()
        self.eat_op("=")
        cases = [self.ident()]
        while self.is_op("|"):
            self.next()
            cases.append(self.ident())
        self.end_line()
        return A.ErrorDecl(name, tuple(cases), line)

    def params(self, closer: str = ")") -> tuple:
        out = []
        if self.is_op(closer):
            return ()
        while True:
            out.append(self.binder())
            if self.is_op(","):
                self.next()
                continue
            break
        return tuple(out)

    def binder(self) -> A.Binder:
        name = self.ident()
        self.eat_op(":")
        inout = False
        if self.is_word("inout") and (self.peek(1).kind == "NAME" or self.is_op("(", 1)):
            self.next()
            inout = True
        ty = self.type_()
        refinement = None
        if self.is_op("|"):
            self.next()
            refinement = self.expr()
        return A.Binder(name, ty, refinement, inout)

    def model_decl(self):
        line = self.next().line
        name = self.ident()
        if self.is_op("="):
            self.next()
            ty = self.type_()
            self.end_line()
            return A.AliasDecl(name, ty, line)
        self.eat_op("(")
        params = self.params()
        self.eat_op(")")
        self.eat_op("->")
        ret = self.type_()
        self.eat_op("=")
        body = self.expr()
        self.end_line()
        return A.ModelDecl(name, params, ret, body, line)

    def record_decl(self) -> A.RecordDecl:
        line = self.next().line
        name = self.ident()
        self.eat_op("(")
        fields = self.fields()
        self.eat_op(")")
        self.end_line()
        return A.RecordDecl(name, fields, line)

    def fields(self) -> tuple:
        out = []
        while True:
            fname = self.ident()
            self.eat_op(":")
            out.append((fname, self.type_()))
            if self.is_op(","):
                self.next()
                continue
            return tuple(out)

    def variant_decl(self) -> A.VariantDecl:
        line = self.next().line
        name = self.ident()
        self.eat_op("=")
        ctors = []
        while True:
            cname = self.ident()
            fields: tuple = ()
            if self.is_op("("):
                self.next()
                fields = self.fields()
                self.eat_op(")")
            ctors.append((cname, fields))
            if self.is_op("|"):
                self.next()
                continue
            break
        self.end_line()
        return A.VariantDecl(name, tuple(ctors), line)

    def op_decl(self) -> A.OpDecl:
        line = self.next().line
        name = self.ident()
        statics: list[str] = []
        if self.is_op("<"):
            self.next()
            while True:
                statics.append(self.ident())
                self.eat_op(":")
                self.eat_word("Nat")
                if self.is_op(","):
                    self.next()
                    continue
                break
            self.eat_op(">")
        self.eat_op("(")
        params = self.params()
        self.eat_op(")")
        self.eat_op("->")
        ret = self.type_()
        fresh = False
        if self.is_word("fresh"):
            self.next()
            fresh = True
        self.end_line()
        return A.OpDecl(name, tuple(statics), params, ret, fresh, line)

    def _block_expr(self) -> object:
        """``NEWLINE INDENT expr NEWLINE DEDENT`` or an inline expression."""
        if self.peek().kind == "NEWLINE":
            self.next()
            self.eat_kind("INDENT")
            body = self.expr()
            self.end_line()
            self.eat_kind("DEDENT")
            return body
        body = self.expr()
        self.end_line()
        return body

    def example_decl(self) -> A.ExampleDecl:
        line = self.next().line
        name = self.ident()
        self.eat_op(":")
        return A.ExampleDecl(name, self._block_expr(), line)

    def claim_decl(self) -> A.ClaimDecl:
        line = self.next().line
        name = self.ident()
        self.eat_op(":")
        return A.ClaimDecl(name, self._block_expr(), line)

    def _rows_open(self) -> None:
        self.eat_op(":")
        self.eat_kind("NEWLINE")
        self.eat_kind("INDENT")

    def assume_decl(self) -> A.AssumeDecl:
        self.next()
        tok = self.next()
        if tok.kind != "NAME" or tok.text not in ("platform", "environment"):
            raise self.fail("expected `platform` or `environment`")
        kind = tok.text
        self._rows_open()
        rows: list = []
        if self.is_word("none"):
            self.next()
            self.end_line()
        else:
            while self.peek().kind != "DEDENT":
                if kind == "platform":
                    parts = [self.ident()]
                    while self.peek().kind == "DOT":
                        self.next()
                        parts.append(self.ident())
                    rows.append(".".join(parts))
                else:
                    rname = self.ident()
                    self.eat_op(":")
                    rows.append((rname, self.expr()))
                self.end_line()
            if not rows:
                raise self.fail("an assumption block is empty")
        self.eat_kind("DEDENT")
        return A.AssumeDecl(kind, tuple(rows))

    def labels_decl(self) -> A.LabelsDecl:
        self.next()
        op = self.ident()
        self._rows_open()
        rows = []
        while self.peek().kind != "DEDENT":
            path = [self.ident()]
            while self.peek().kind == "DOT":
                self.next()
                path.append(self.ident())
            tok = self.next()
            if tok.kind != "NAME" or tok.text not in ("public", "secret"):
                raise self.fail("expected `public` or `secret`")
            rows.append((tuple(path), tok.text))
            self.end_line()
        if not rows:
            raise self.fail("a labels block is empty")
        self.eat_kind("DEDENT")
        return A.LabelsDecl(op, tuple(rows))

    def input_decl(self) -> A.InputDecl:
        self.next()
        op = self.ident()
        if self.peek().kind != "DOT":
            raise self.fail("expected `op.parameter`")
        self.next()
        param = self.ident()
        self._rows_open()
        self.eat_word("hostile")
        self.end_line()
        rejects = []
        while self.is_word("rejects"):
            self.next()
            predicate = self.expr()
            self.eat_word("with")
            err = self.ident()
            if self.peek().kind != "DOT":
                raise self.fail("expected a qualified error case")
            self.next()
            case = self.ident()
            rejects.append((predicate, (err, case)))
            self.end_line()
        self.eat_word("ensures")
        flags = [self.ident()]
        while self.is_op(","):
            self.next()
            flags.append(self.ident())
        self.end_line()
        self.eat_kind("DEDENT")
        return A.InputDecl(op, param, tuple(rejects), tuple(flags))

    def bound_decl(self) -> A.BoundDecl:
        self.next()
        op = self.ident()
        self._rows_open()
        rows = []
        while self.peek().kind != "DEDENT":
            metric = self.ident()
            self.eat_op("<=")
            rows.append((metric, self.expr()))
            self.end_line()
        if not rows:
            raise self.fail("a bound block is empty")
        self.eat_kind("DEDENT")
        return A.BoundDecl(op, tuple(rows))

    def ensures_decl(self) -> A.EnsuresDecl:
        self.next()
        op = self.ident()
        self._rows_open()
        rows = []
        while self.peek().kind != "DEDENT":
            rows.append(self.expr())
            self.end_line()
        if not rows:
            raise self.fail("an ensures block is empty")
        self.eat_kind("DEDENT")
        return A.EnsuresDecl(op, tuple(rows))

    # -- expressions ---------------------------------------------------------

    def expr(self):
        self.descend()
        try:
            if self.is_word("forall") or self.is_word("exists"):
                return self.quantifier()
            return self.implies()
        finally:
            self.ascend()

    def quantifier(self) -> A.Quant:
        kind = self.next().text
        binders = [self.binder()]
        while self.is_op(","):
            self.next()
            binders.append(self.binder())
        self.eat_kind("DOTSEP")
        return A.Quant(kind, tuple(binders), self.expr())

    def implies(self):
        left = self.or_()
        if self.is_op("=>"):
            self.next()
            return A.Binary("=>", left, self.implies_rhs())
        return left

    def implies_rhs(self):
        # Right associative: ``a => b => c`` is ``a => (b => c)``.
        self.descend()
        try:
            return self.implies()
        finally:
            self.ascend()

    def or_(self):
        left = self.and_()
        while self.is_word("or"):
            self.next()
            left = A.Binary("or", left, self.and_())
        return left

    def and_(self):
        left = self.compare()
        while self.is_word("and"):
            self.next()
            left = A.Binary("and", left, self.compare())
        return left

    def compare(self):
        left = self.add()
        tok = self.peek()
        if tok.kind == "OP" and tok.text in _COMPARE:
            self.next()
            right = self.add()
            nxt = self.peek()
            if nxt.kind == "OP" and nxt.text in _COMPARE:
                raise self.fail("comparisons cannot be chained")
            return A.Binary(tok.text, left, right)
        return left

    def add(self):
        left = self.mul()
        while self.peek().kind == "OP" and self.peek().text in ("+", "-"):
            op = self.next().text
            left = A.Binary(op, left, self.mul())
        return left

    def mul(self):
        left = self.unary()
        while self.is_op("*"):
            self.next()
            left = A.Binary("*", left, self.unary())
        return left

    def unary(self):
        if self.is_word("not"):
            self.next()
            self.descend()
            try:
                return A.Unary("not", self.unary())
            finally:
                self.ascend()
        if self.is_op("-"):
            self.next()
            self.descend()
            try:
                return A.Unary("neg", self.unary())
            finally:
                self.ascend()
        return self.postfix()

    def postfix(self):
        base = self.primary()
        while self.peek().kind == "DOT" and self.peek(1).kind in ("NAME", "INT"):
            self.next()  # the DOT
            base = A.Proj(base, self.next().text)
        return base

    def primary(self):
        tok = self.peek()
        if tok.kind == "INT":
            self.next()
            value, suffix = split_int(tok.text)
            return A.IntLit(value, suffix)
        if tok.kind == "BYTES":
            self.next()
            return A.BytesLit(tok.text)
        if tok.kind == "OP" and tok.text == "(":
            self.next()
            first = self.expr()
            if self.is_op(","):
                items = [first]
                while self.is_op(","):
                    self.next()
                    items.append(self.expr())
                self.eat_op(")")
                if len(items) > MAX_TUPLE_ARITY:
                    raise self.fail("a tuple has at most 8 components")
                return A.TupleLit(tuple(items))
            self.eat_op(")")
            return first
        if tok.kind == "OP" and tok.text == "[":
            self.next()
            items = []
            if not self.is_op("]"):
                items.append(self.expr())
                while self.is_op(","):
                    self.next()
                    items.append(self.expr())
            self.eat_op("]")
            return A.ListLit(tuple(items))
        if tok.kind == "NAME":
            if tok.text in ("true", "false"):
                self.next()
                return A.BoolLit(tok.text == "true")
            if tok.text == "if":
                return self.conditional()
            if tok.text == "match":
                return self.match_()
            if tok.text in RESERVED:
                raise self.fail("a reserved word is not an expression")
            return self.reference()
        raise self.fail("expected an expression")

    def conditional(self) -> A.If:
        self.next()
        cond = self.expr()
        self.eat_word("then")
        then = self.expr()
        self.eat_word("else")
        return A.If(cond, then, self.expr())

    def reference(self):
        parts = [self.next().text]
        while self.peek().kind == "DOT" and self.peek(1).kind in ("NAME", "INT"):
            self.next()  # the DOT
            parts.append(self.next().text)
        statics: tuple = ()
        if parts[0] in self.static_names and len(parts) == 1 and self.is_op("<"):
            self.next()
            items: list = []
            while True:
                tok = self.next()
                if tok.kind == "INT":
                    value, suffix = split_int(tok.text)
                    if suffix is not None:
                        raise self.fail("a static width is an unsuffixed integer")
                    items.append(value)
                elif tok.kind == "NAME":
                    items.append(tok.text)
                else:
                    raise self.fail("expected a static argument")
                if self.is_op(","):
                    self.next()
                    continue
                break
            self.eat_op(">")
            statics = tuple(items)
        args = None
        if self.is_op("("):
            self.next()
            collected = []
            if not self.is_op(")"):
                collected.append(self.expr())
                while self.is_op(","):
                    self.next()
                    collected.append(self.expr())
            self.eat_op(")")
            args = tuple(collected)
        if args is None and not statics:
            return A.Name(tuple(parts))
        return A.Call(tuple(parts), statics, args)

    # -- match ---------------------------------------------------------------

    def match_(self) -> A.Match:
        self.next()
        scrutinee = self.expr()
        self.eat_word("with")
        arms = []
        while True:
            pattern = self.pattern(0)
            self.eat_op("->")
            arms.append((pattern, self.expr()))
            if self.is_op("|"):
                self.next()
                continue
            break
        if len(arms) > 256:
            raise OracleError("limit", "a match has more than 256 arms")
        return A.Match(scrutinee, tuple(arms))

    def pattern(self, depth: int) -> A.Pattern:
        if depth > 8:
            raise OracleError("limit", "pattern nesting exceeds 8 levels")
        if self.is_op("("):
            self.next()
            items = [self.pattern(depth + 1)]
            while self.is_op(","):
                self.next()
                items.append(self.pattern(depth + 1))
            self.eat_op(")")
            if len(items) < 2:
                raise self.fail("a tuple pattern has at least two components")
            return A.Pattern("tuple", items=tuple(items))
        tok = self.peek()
        if tok.kind != "NAME":
            raise self.fail("expected a pattern")
        if tok.text in ("true", "false"):
            self.next()
            return A.Pattern("bool", value=tok.text == "true")
        if tok.text == "_":
            self.next()
            return A.Pattern("wild")
        if tok.text in RESERVED:
            raise self.fail("a reserved word is not a pattern")
        parts = [self.next().text]
        while self.peek().kind == "DOT" and self.peek(1).kind == "NAME":
            self.next()  # the DOT
            parts.append(self.next().text)
        items: list = []
        has_args = False
        if self.is_op("("):
            self.next()
            has_args = True
            items.append(self.pattern(depth + 1))
            while self.is_op(","):
                self.next()
                items.append(self.pattern(depth + 1))
            self.eat_op(")")
        if len(parts) == 1 and not has_args:
            return A.Pattern("bind", name=tuple(parts))
        return A.Pattern("ctor", name=tuple(parts), items=tuple(items))


def parse_tokens(tokens: list[Token]) -> A.Document:
    """Parse a token list; converts stack exhaustion into a ``limit`` error."""
    previous = sys.getrecursionlimit()
    sys.setrecursionlimit(max(previous, 6000))
    try:
        return _Parser(tokens).document()
    except RecursionError as exc:
        raise OracleError("limit", "document nests too deeply") from exc
    finally:
        sys.setrecursionlimit(previous)


def parse_source(raw: bytes | str) -> A.Document:
    """Admit, lex and parse ``.warrant`` source (bytes or already-decoded text)."""
    if isinstance(raw, str):
        if len(raw.encode("utf-8")) > MAX_SOURCE_BYTES:
            raise OracleError("source", "source exceeds the 1 MiB admission limit")
        raw = raw.encode("utf-8")
    return parse_tokens(tokenize(decode_source(raw)))

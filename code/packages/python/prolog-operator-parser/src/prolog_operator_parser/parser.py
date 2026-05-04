"""Operator-aware token-level Prolog parser."""

from __future__ import annotations

from dataclasses import dataclass

from lexer import Token
from logic_engine import (
    Atom,
    Clause,
    Compound,
    GoalExpr,
    LogicVar,
    Program,
    RelationCall,
    Term,
    atom,
    fact,
    goal_from_term,
    logic_list,
    num,
    program,
    relation,
    rule,
    string,
    term,
    var,
)
from prolog_core import (
    OperatorSpec,
    OperatorTable,
    PredicateRegistry,
    PrologDirective,
    apply_op_directive,
    apply_predicate_directive,
    directive,
    empty_predicate_registry,
    expand_dcg_clause,
)
from prolog_parser import ParsedQuery, PrologParseError

__version__ = "0.1.0"

_INFIX_ASSOC = frozenset({"xfx", "xfy", "yfx"})
_PREFIX_ASSOC = frozenset({"fx", "fy"})
_POSTFIX_ASSOC = frozenset({"xf", "yf"})
_NON_ATOM_TERMINALS = frozenset(
    {
        "VARIABLE",
        "ANON_VAR",
        "INTEGER",
        "FLOAT",
        "STRING",
        "LPAREN",
        "RPAREN",
        "LBRACKET",
        "RBRACKET",
        "LCURLY",
        "RCURLY",
        "BAR",
        "DOT",
        "DCG",
    }
)


@dataclass(frozen=True, slots=True)
class ParsedOperatorSource:
    """A token-level parsed Prolog source."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]
    directives: tuple[PrologDirective, ...]
    operator_table: OperatorTable
    predicate_registry: PredicateRegistry


@dataclass(frozen=True, slots=True)
class ParsedOperatorTerm:
    """A parsed term plus its named variable scope."""

    term: Term
    variables: dict[str, LogicVar]


@dataclass(slots=True)
class _Scope:
    """Per-clause or per-query variable scope."""

    variables: dict[str, LogicVar]
    anonymous_count: int = 0

    def variable(self, name: str) -> LogicVar:
        existing = self.variables.get(name)
        if existing is not None:
            return existing
        created = var(name)
        self.variables[name] = created
        return created

    def anonymous(self) -> LogicVar:
        self.anonymous_count += 1
        return var(f"_{self.anonymous_count}")


@dataclass(frozen=True, slots=True)
class _ParsedTerm:
    """A parsed term plus the outer precedence that formed it."""

    value: Term
    precedence: int = 0


class _OperatorParser:
    """A precedence-aware parser over Prolog token streams."""

    def __init__(
        self,
        tokens: list[Token],
        operator_table: OperatorTable,
        *,
        allow_directives: bool,
    ) -> None:
        self._tokens = tokens
        self._operator_table = operator_table
        self._predicate_registry = empty_predicate_registry()
        self._allow_directives = allow_directives
        self._pos = 0

    def parse_source(self) -> ParsedOperatorSource:
        clauses: list[Clause] = []
        queries: list[ParsedQuery] = []
        directives: list[PrologDirective] = []

        while not self._at_end():
            if self._match_type("QUERY"):
                queries.append(self._parse_query_statement())
                continue
            if self._allow_directives and self._match_type("RULE"):
                directive_value = self._parse_directive_statement()
                directives.append(directive_value)
                try:
                    self._operator_table = apply_op_directive(
                        self._operator_table,
                        directive_value.term,
                    )
                    self._predicate_registry = apply_predicate_directive(
                        self._predicate_registry,
                        directive_value,
                    )
                except (TypeError, ValueError) as error:
                    raise self._error(self._current_or_eof(), str(error)) from error
                continue
            clauses.append(self._parse_clause_statement())

        return ParsedOperatorSource(
            program=program(
                *clauses,
                dynamic_relations=self._predicate_registry.dynamic_relations(),
            ),
            clauses=tuple(clauses),
            queries=tuple(queries),
            directives=tuple(directives),
            operator_table=self._operator_table,
            predicate_registry=self._predicate_registry,
        )

    def parse_query(self) -> ParsedQuery:
        if not self._match_type("QUERY"):
            raise self._error(self._current_or_eof(), "expected top-level query")
        query = self._parse_query_statement()
        if not self._at_end():
            raise self._error(self._current_or_eof(), "unexpected tokens after query")
        return query

    def parse_program(self) -> Program:
        parsed = self.parse_source()
        if parsed.queries:
            raise self._error(
                self._tokens[0] if self._tokens else self._eof_token(),
                "expected only clauses"
                f", but found {len(parsed.queries)} query statement(s)",
            )
        return parsed.program

    def parse_goal(self) -> ParsedQuery:
        scope = _Scope(variables={})
        goal = self._parse_goal_expression(scope, stop_types={"EOF"})
        if not self._at_end():
            raise self._error(self._current_or_eof(), "unexpected tokens after goal")
        return ParsedQuery(goal=goal, variables=dict(scope.variables))

    def parse_term(self) -> Term:
        return self.parse_named_term().term

    def parse_named_term(self) -> ParsedOperatorTerm:
        scope = _Scope(variables={})
        parsed = self._parse_term_expression(scope, 1200, stop_types={"EOF"})
        if not self._at_end():
            raise self._error(self._current_or_eof(), "unexpected tokens after term")
        return ParsedOperatorTerm(term=parsed.value, variables=dict(scope.variables))

    def _parse_query_statement(self) -> ParsedQuery:
        scope = _Scope(variables={})
        goal = self._parse_goal_expression(scope, stop_types={"DOT"})
        self._expect_type("DOT")
        return ParsedQuery(goal=goal, variables=dict(scope.variables))

    def _parse_directive_statement(self) -> PrologDirective:
        scope = _Scope(variables={})
        goal = self._parse_goal_expression(scope, stop_types={"DOT"})
        self._expect_type("DOT")
        return directive(goal, scope.variables)

    def _parse_clause_statement(self) -> Clause:
        scope = _Scope(variables={})
        head = self._parse_term_expression(
            scope,
            1200,
            stop_types={"RULE", "DCG", "DOT"},
        )
        if self._match_type("DCG"):
            body_term = self._parse_term_expression(scope, 1200, stop_types={"DOT"})
            self._expect_type("DOT")
            try:
                return expand_dcg_clause(head.value, body_term.value)
            except TypeError as error:
                raise self._error(self._current_or_eof(), str(error)) from error

        try:
            relation_head = _term_as_relation_call(
                head.value,
                "clause head must be callable",
            )
        except TypeError as error:
            raise self._error(self._current_or_eof(), str(error)) from error
        if self._match_type("RULE"):
            body = self._parse_goal_expression(scope, stop_types={"DOT"})
            self._expect_type("DOT")
            return rule(relation_head, body)

        self._expect_type("DOT")
        return fact(relation_head)

    def _parse_goal_expression(
        self,
        scope: _Scope,
        *,
        stop_types: set[str],
    ) -> GoalExpr:
        term_value = self._parse_term_expression(
            scope,
            1200,
            stop_types=stop_types,
        ).value
        try:
            return goal_from_term(term_value)
        except TypeError as error:
            raise self._error(
                self._current_or_eof(),
                str(error),
            ) from error

    def _parse_term_expression(
        self,
        scope: _Scope,
        max_precedence: int,
        *,
        stop_types: set[str],
    ) -> _ParsedTerm:
        left = self._parse_prefix_or_primary(
            scope,
            max_precedence,
            stop_types=stop_types,
        )

        while True:
            token = self._current()
            if token is None or token.type_name in stop_types:
                return left

            postfix = self._matching_postfix(token, max_precedence, left.precedence)
            if postfix is not None:
                self._advance()
                left = _ParsedTerm(
                    value=term(postfix.symbol, left.value),
                    precedence=postfix.precedence,
                )
                continue

            infix = self._matching_infix(token, max_precedence, left.precedence)
            if infix is None:
                return left

            self._advance()
            right_precedence = (
                infix.precedence
                if infix.associativity == "xfy"
                else infix.precedence - 1
            )
            right = self._parse_term_expression(
                scope,
                right_precedence,
                stop_types=stop_types,
            )
            left = _ParsedTerm(
                value=term(infix.symbol, left.value, right.value),
                precedence=infix.precedence,
            )

    def _parse_prefix_or_primary(
        self,
        scope: _Scope,
        max_precedence: int,
        *,
        stop_types: set[str],
    ) -> _ParsedTerm:
        token = self._current_or_eof()
        if token.type_name in stop_types:
            raise self._error(token, "expected term")

        if self._is_compound_functor(token):
            return self._parse_compound_term(scope)

        prefix = self._matching_prefix(token, max_precedence)
        if prefix is not None:
            self._advance()
            right_precedence = (
                prefix.precedence
                if prefix.associativity == "fy"
                else prefix.precedence - 1
            )
            operand = self._parse_term_expression(
                scope,
                right_precedence,
                stop_types=stop_types,
            )
            return _ParsedTerm(
                value=term(prefix.symbol, operand.value),
                precedence=prefix.precedence,
            )

        if self._match_type("LPAREN"):
            inner = self._parse_term_expression(scope, 1200, stop_types={"RPAREN"})
            self._expect_type("RPAREN")
            return _ParsedTerm(inner.value)

        if self._match_type("LBRACKET"):
            return _ParsedTerm(self._parse_list(scope))
        if self._match_type("LCURLY"):
            inner = self._parse_term_expression(scope, 1200, stop_types={"RCURLY"})
            self._expect_type("RCURLY")
            return _ParsedTerm(term("{}", inner.value))

        if token.type_name == "ATOM":
            self._advance()
            return _ParsedTerm(atom(_atom_name(token.value)))
        if token.type_name == "CUT":
            self._advance()
            return _ParsedTerm(atom("!"))
        if token.type_name == "VARIABLE":
            self._advance()
            return _ParsedTerm(scope.variable(token.value))
        if token.type_name == "ANON_VAR":
            self._advance()
            return _ParsedTerm(scope.anonymous())
        if token.type_name == "INTEGER":
            self._advance()
            return _ParsedTerm(num(int(token.value)))
        if token.type_name == "FLOAT":
            self._advance()
            return _ParsedTerm(num(float(token.value)))
        if token.type_name == "STRING":
            self._advance()
            return _ParsedTerm(string(token.value))

        raise self._error(token, "expected term")

    def _parse_compound_term(self, scope: _Scope) -> _ParsedTerm:
        token = self._advance()
        self._expect_type("LPAREN")
        arguments: list[Term] = []
        if not self._check_type("RPAREN"):
            while True:
                arguments.append(
                    self._parse_term_expression(
                        scope,
                        1200,
                        stop_types={"COMMA", "RPAREN"},
                    ).value,
                )
                if not self._match_type("COMMA"):
                    break
        self._expect_type("RPAREN")
        return _ParsedTerm(term(_functor_name(token), *arguments))

    def _parse_list(self, scope: _Scope) -> Term:
        if self._match_type("RBRACKET"):
            return logic_list([])

        items: list[Term] = []
        tail: Term | None = None
        while True:
            items.append(
                self._parse_term_expression(
                    scope,
                    1200,
                    stop_types={"COMMA", "BAR", "RBRACKET"},
                ).value,
            )
            if self._match_type("COMMA"):
                continue
            if self._match_type("BAR"):
                tail = self._parse_term_expression(
                    scope,
                    1200,
                    stop_types={"RBRACKET"},
                ).value
            break

        self._expect_type("RBRACKET")
        return logic_list(items, tail=tail)

    def _matching_prefix(
        self,
        token: Token,
        max_precedence: int,
    ) -> OperatorSpec | None:
        specs = [
            spec
            for spec in self._operator_table.named(token.value)
            if spec.associativity in _PREFIX_ASSOC and spec.precedence <= max_precedence
        ]
        if not specs:
            return None
        return min(specs, key=lambda spec: spec.precedence)

    def _matching_postfix(
        self,
        token: Token,
        max_precedence: int,
        left_precedence: int,
    ) -> OperatorSpec | None:
        specs = []
        for spec in self._operator_table.named(token.value):
            if spec.associativity not in _POSTFIX_ASSOC:
                continue
            if spec.precedence > max_precedence:
                continue
            if spec.associativity == "xf" and left_precedence >= spec.precedence:
                continue
            if spec.associativity == "yf" and left_precedence > spec.precedence:
                continue
            specs.append(spec)
        if not specs:
            return None
        return min(specs, key=lambda spec: spec.precedence)

    def _matching_infix(
        self,
        token: Token,
        max_precedence: int,
        left_precedence: int,
    ) -> OperatorSpec | None:
        specs = []
        for spec in self._operator_table.named(token.value):
            if spec.associativity not in _INFIX_ASSOC:
                continue
            if spec.precedence > max_precedence:
                continue
            if (
                spec.associativity in {"xfx", "xfy"}
                and left_precedence >= spec.precedence
            ):
                continue
            if spec.associativity == "yfx" and left_precedence > spec.precedence:
                continue
            specs.append(spec)
        if not specs:
            return None
        return min(specs, key=lambda spec: spec.precedence)

    def _is_compound_functor(self, token: Token) -> bool:
        next_token = self._peek()
        if next_token is None or next_token.type_name != "LPAREN":
            return False
        return token.type_name not in _NON_ATOM_TERMINALS

    def _check_type(self, type_name: str) -> bool:
        token = self._current()
        return token is not None and token.type_name == type_name

    def _match_type(self, type_name: str) -> bool:
        if self._check_type(type_name):
            self._advance()
            return True
        return False

    def _expect_type(self, type_name: str) -> Token:
        token = self._current_or_eof()
        if token.type_name != type_name:
            raise self._error(token, f"expected {type_name}")
        return self._advance()

    def _advance(self) -> Token:
        token = self._current_or_eof()
        self._pos += 1
        return token

    def _peek(self) -> Token | None:
        index = self._pos + 1
        if index >= len(self._tokens):
            return None
        return self._tokens[index]

    def _current(self) -> Token | None:
        if self._pos >= len(self._tokens):
            return None
        return self._tokens[self._pos]

    def _current_or_eof(self) -> Token:
        current = self._current()
        if current is not None:
            return current
        return self._eof_token()

    def _at_end(self) -> bool:
        return self._pos >= len(self._tokens)

    def _eof_token(self) -> Token:
        if self._tokens:
            last = self._tokens[-1]
            return Token("EOF", "", last.line, last.column + len(last.value))
        return Token("EOF", "", 1, 1)

    def _error(self, token: Token, message: str) -> PrologParseError:
        return PrologParseError(token, message)


def _atom_name(value: str) -> str:
    if len(value) >= 2 and value[0] == "'" and value[-1] == "'":
        return value[1:-1].replace("\\'", "'").replace("\\\\", "\\")
    return value


def _functor_name(token: Token) -> str:
    if token.type_name == "ATOM":
        return _atom_name(token.value)
    return token.value


def _term_as_relation_call(term_value: Term, message: str) -> RelationCall:
    if isinstance(term_value, RelationCall):
        return term_value
    if isinstance(term_value, Atom):
        return relation(term_value.symbol, 0)()
    if isinstance(term_value, Compound):
        return relation(term_value.functor, len(term_value.args))(*term_value.args)
    raise TypeError(message)


def _strip_eof(tokens: list[Token]) -> list[Token]:
    return [token for token in tokens if token.type_name != "EOF"]


def parse_operator_source_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
    *,
    allow_directives: bool = False,
) -> ParsedOperatorSource:
    """Parse a token stream into clauses, queries, and optional directives."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=allow_directives,
    ).parse_source()


def parse_operator_program_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
    *,
    allow_directives: bool = False,
) -> Program:
    """Parse a token stream containing only clauses and optional directives."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=allow_directives,
    ).parse_program()


def parse_operator_query_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
) -> ParsedQuery:
    """Parse a token stream containing exactly one top-level query."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=False,
    ).parse_query()


def parse_operator_goal_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
) -> ParsedQuery:
    """Parse a bare goal token stream using the supplied operator table."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=False,
    ).parse_goal()


def parse_operator_term_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
) -> Term:
    """Parse a bare term token stream using the supplied operator table."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=False,
    ).parse_term()


def parse_operator_named_term_tokens(
    tokens: list[Token],
    operator_table: OperatorTable,
) -> ParsedOperatorTerm:
    """Parse a bare term and retain its named variable scope."""

    return _OperatorParser(
        _strip_eof(tokens),
        operator_table,
        allow_directives=False,
    ).parse_named_term()

"""Recursive-descent parser that lowers Prolog syntax to ``logic-engine``."""

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
    conj,
    cut,
    disj,
    eq,
    fact,
    fail,
    logic_list,
    neq,
    num,
    program,
    relation,
    rule,
    string,
    succeed,
    term,
    var,
)
from prolog_lexer import tokenize_prolog


@dataclass(frozen=True, slots=True)
class ParsedQuery:
    """A parsed top-level query plus variables visible in that query."""

    goal: GoalExpr
    variables: dict[str, LogicVar]


@dataclass(frozen=True, slots=True)
class ParsedSource:
    """A parsed Prolog source file lowered to executable engine objects."""

    program: Program
    clauses: tuple[Clause, ...]
    queries: tuple[ParsedQuery, ...]


class PrologParseError(SyntaxError):
    """Raised when Prolog source cannot be parsed by this subset parser."""

    def __init__(self, token: Token, message: str) -> None:
        super().__init__(
            f"{message} at line {token.line}, column {token.column}: {token!r}",
        )
        self.token = token


@dataclass(slots=True)
class _Scope:
    """Per-clause or per-query variable scope."""

    variables: dict[str, LogicVar]
    anonymous_count: int = 0

    def variable(self, name: str) -> LogicVar:
        """Return the scoped variable for ``name``."""

        existing = self.variables.get(name)
        if existing is not None:
            return existing
        created = var(name)
        self.variables[name] = created
        return created

    def anonymous(self) -> LogicVar:
        """Return a fresh variable for one anonymous ``_`` occurrence."""

        self.anonymous_count += 1
        return var(f"_{self.anonymous_count}")


class _Parser:
    """Small parser for facts, rules, queries, terms, lists, and core goals."""

    def __init__(self, tokens: list[Token]) -> None:
        self.tokens = tokens
        self.index = 0

    def parse_source(self) -> ParsedSource:
        """Parse every statement in the token stream."""

        clauses: list[Clause] = []
        queries: list[ParsedQuery] = []
        while not self._check("EOF"):
            if self._check("QUERY"):
                queries.append(self._parse_query_statement())
            else:
                clauses.append(self._parse_clause_statement())
        return ParsedSource(
            program=program(*clauses),
            clauses=tuple(clauses),
            queries=tuple(queries),
        )

    def _parse_query_statement(self) -> ParsedQuery:
        self._expect("QUERY", "expected query introducer")
        scope = _Scope(variables={})
        goal = self._parse_goal(scope)
        self._expect("DOT", "expected '.' after query")
        return ParsedQuery(goal=goal, variables=dict(scope.variables))

    def _parse_clause_statement(self) -> Clause:
        scope = _Scope(variables={})
        head = self._term_as_relation_call(
            self._parse_term(scope),
            "clause head must be an atom or compound term",
        )
        if self._match("RULE"):
            body = self._parse_goal(scope)
            self._expect("DOT", "expected '.' after rule")
            return rule(head, body)
        if self._check("DCG"):
            raise PrologParseError(
                self._peek(),
                "DCG rules are recognized by the lexer but not parsed yet",
            )
        self._expect("DOT", "expected '.' after fact")
        return fact(head)

    def _parse_goal(self, scope: _Scope) -> GoalExpr:
        return self._parse_disjunction(scope)

    def _parse_disjunction(self, scope: _Scope) -> GoalExpr:
        parts = [self._parse_conjunction(scope)]
        while self._match("SEMICOLON"):
            parts.append(self._parse_conjunction(scope))
        return disj(*parts) if len(parts) > 1 else parts[0]

    def _parse_conjunction(self, scope: _Scope) -> GoalExpr:
        parts = [self._parse_goal_primary(scope)]
        while self._match("COMMA"):
            parts.append(self._parse_goal_primary(scope))
        return conj(*parts) if len(parts) > 1 else parts[0]

    def _parse_goal_primary(self, scope: _Scope) -> GoalExpr:
        if self._match("CUT"):
            return cut()
        if self._match("LPAREN"):
            goal = self._parse_goal(scope)
            self._expect("RPAREN", "expected ')' after grouped goal")
            return goal

        left = self._parse_term(scope)
        if self._check("ATOM") and self._peek().value in {"=", "\\="}:
            operator = self._advance().value
            right = self._parse_term(scope)
            return eq(left, right) if operator == "=" else neq(left, right)
        return self._term_as_goal(left)

    def _parse_term(self, scope: _Scope) -> Term:
        if self._match("VARIABLE"):
            return scope.variable(self._previous().value)
        if self._match("ANON_VAR"):
            return scope.anonymous()
        if self._match("INTEGER"):
            return num(int(self._previous().value))
        if self._match("FLOAT"):
            return num(float(self._previous().value))
        if self._match("STRING"):
            return string(self._previous().value)
        if self._match("LBRACKET"):
            return self._parse_list(scope)
        if self._match("ATOM"):
            name = _atom_name(self._previous().value)
            if self._match("LPAREN"):
                args = self._parse_term_arguments(scope)
                return term(name, *args)
            return atom(name)

        raise PrologParseError(self._peek(), "expected term")

    def _parse_term_arguments(self, scope: _Scope) -> tuple[Term, ...]:
        if self._match("RPAREN"):
            return ()

        args = [self._parse_term(scope)]
        while self._match("COMMA"):
            args.append(self._parse_term(scope))
        self._expect("RPAREN", "expected ')' after term arguments")
        return tuple(args)

    def _parse_list(self, scope: _Scope) -> Term:
        if self._match("RBRACKET"):
            return logic_list([])

        items = [self._parse_term(scope)]
        while self._match("COMMA"):
            items.append(self._parse_term(scope))

        tail: Term | None = None
        if self._match("BAR"):
            tail = self._parse_term(scope)
        self._expect("RBRACKET", "expected ']' after list")
        return logic_list(items, tail=tail)

    def _term_as_goal(self, term_value: Term) -> GoalExpr:
        if isinstance(term_value, Atom):
            name = term_value.symbol.name
            if term_value.symbol.namespace is None and name == "true":
                return succeed()
            if term_value.symbol.namespace is None and name == "fail":
                return fail()
            return relation(term_value.symbol, 0)()
        if isinstance(term_value, Compound):
            return relation(term_value.functor, len(term_value.args))(*term_value.args)
        raise PrologParseError(self._peek(), "term is not callable as a goal")

    def _term_as_relation_call(
        self,
        term_value: Term,
        message: str,
    ) -> RelationCall:
        if isinstance(term_value, Atom):
            return relation(term_value.symbol, 0)()
        if isinstance(term_value, Compound):
            return relation(term_value.functor, len(term_value.args))(*term_value.args)
        raise PrologParseError(self._peek(), message)

    def _match(self, token_type: str) -> bool:
        if not self._check(token_type):
            return False
        self.index += 1
        return True

    def _expect(self, token_type: str, message: str) -> Token:
        if self._check(token_type):
            return self._advance()
        raise PrologParseError(self._peek(), message)

    def _check(self, token_type: str) -> bool:
        return self._peek().type_name == token_type

    def _advance(self) -> Token:
        token = self._peek()
        self.index += 1
        return token

    def _peek(self) -> Token:
        return self.tokens[self.index]

    def _previous(self) -> Token:
        return self.tokens[self.index - 1]


def _atom_name(value: str) -> str:
    """Normalize lexer atom text into the engine's atom symbol name."""

    if len(value) >= 2 and value[0] == "'" and value[-1] == "'":
        return value[1:-1].replace("\\'", "'").replace("\\\\", "\\")
    return value


def parse_source(source: str) -> ParsedSource:
    """Parse clauses and queries from Prolog source text."""

    return _Parser(tokenize_prolog(source)).parse_source()


def parse_program(source: str) -> Program:
    """Parse a Prolog source containing only facts and rules."""

    parsed = parse_source(source)
    if parsed.queries:
        raise PrologParseError(
            tokenize_prolog(source)[0],
            "expected only clauses, but found "
            f"{len(parsed.queries)} query statement(s)",
        )
    return parsed.program


def parse_query(source: str) -> ParsedQuery:
    """Parse one top-level query statement."""

    parsed = parse_source(source)
    if parsed.clauses:
        raise PrologParseError(
            tokenize_prolog(source)[0],
            f"expected only a query, but found {len(parsed.clauses)} clause(s)",
        )
    if len(parsed.queries) != 1:
        raise PrologParseError(
            tokenize_prolog(source)[0],
            f"expected exactly one query, but found {len(parsed.queries)}",
        )
    return parsed.queries[0]

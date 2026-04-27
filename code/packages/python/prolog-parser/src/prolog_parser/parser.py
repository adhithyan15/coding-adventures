"""Grammar-driven parser that lowers Prolog syntax to ``logic-engine``."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cache
from pathlib import Path

from grammar_tools import ParserGrammar, parse_parser_grammar
from lang_parser import ASTNode, GrammarParseError, GrammarParser
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

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
PROLOG_GRAMMAR_PATH = GRAMMAR_DIR / "prolog.grammar"


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


@cache
def _prolog_grammar() -> ParserGrammar:
    """Load and cache the declarative Prolog grammar."""

    return parse_parser_grammar(PROLOG_GRAMMAR_PATH.read_text())


def create_prolog_parser(source: str) -> GrammarParser:
    """Create a generic ``GrammarParser`` configured for Prolog source."""

    return GrammarParser(tokenize_prolog(source), _prolog_grammar())


def parse_ast(source: str) -> ASTNode:
    """Parse Prolog source with ``prolog.grammar`` and return the syntax tree."""

    tokens = tokenize_prolog(source)
    for token in tokens:
        if token.type_name == "DCG":
            raise PrologParseError(
                token,
                "DCG rules are recognized by the lexer but not parsed yet",
            )
    try:
        return GrammarParser(tokens, _prolog_grammar()).parse()
    except GrammarParseError as error:
        token = error.token if error.token is not None else tokens[-1]
        raise PrologParseError(token, str(error)) from error


class _Lowerer:
    """Lower the grammar AST into executable ``logic-engine`` values."""

    def lower_source(self, ast: ASTNode) -> ParsedSource:
        """Lower a ``program`` AST node into a parsed source object."""

        clauses: list[Clause] = []
        queries: list[ParsedQuery] = []
        for statement in _node_children(ast, "statement"):
            body = _single_node_child(statement)
            if body.rule_name == "query_statement":
                queries.append(self._lower_query_statement(body))
            elif body.rule_name == "rule_statement":
                clauses.append(self._lower_rule_statement(body))
            elif body.rule_name == "fact_statement":
                clauses.append(self._lower_fact_statement(body))
            else:  # pragma: no cover - grammar validation keeps this unreachable.
                raise _unexpected_node(body)

        return ParsedSource(
            program=program(*clauses),
            clauses=tuple(clauses),
            queries=tuple(queries),
        )

    def _lower_query_statement(self, node: ASTNode) -> ParsedQuery:
        scope = _Scope(variables={})
        goal = self._lower_goal(_single_node_child(node, "goal"), scope)
        return ParsedQuery(goal=goal, variables=dict(scope.variables))

    def _lower_rule_statement(self, node: ASTNode) -> Clause:
        scope = _Scope(variables={})
        head = self._term_as_relation_call(
            self._lower_callable_term(_single_node_child(node, "callable_term"), scope),
            "clause head must be an atom or compound term",
        )
        body = self._lower_goal(_single_node_child(node, "goal"), scope)
        return rule(head, body)

    def _lower_fact_statement(self, node: ASTNode) -> Clause:
        scope = _Scope(variables={})
        head = self._term_as_relation_call(
            self._lower_callable_term(_single_node_child(node, "callable_term"), scope),
            "clause head must be an atom or compound term",
        )
        return fact(head)

    def _lower_goal(self, node: ASTNode, scope: _Scope) -> GoalExpr:
        if node.rule_name == "goal":
            return self._lower_goal(_single_node_child(node), scope)
        if node.rule_name == "disjunction":
            parts = [
                self._lower_goal(child, scope)
                for child in _node_children(node, "conjunction")
            ]
            return disj(*parts) if len(parts) > 1 else parts[0]
        if node.rule_name == "conjunction":
            parts = [
                self._lower_goal(child, scope)
                for child in _node_children(node, "goal_primary")
            ]
            return conj(*parts) if len(parts) > 1 else parts[0]
        if node.rule_name != "goal_primary":
            raise _unexpected_node(node)

        child = node.children[0]
        if isinstance(child, Token):
            if child.type_name == "CUT":
                return cut()
            raise PrologParseError(child, "unexpected token in goal")

        if child.rule_name == "grouped_goal":
            return self._lower_goal(_single_node_child(child, "goal"), scope)
        if child.rule_name == "equality_goal":
            return self._lower_equality_goal(child, scope)
        if child.rule_name == "callable_goal":
            return self._term_as_goal(
                self._lower_callable_term(
                    _single_node_child(child, "callable_term"),
                    scope,
                ),
                child,
            )
        raise _unexpected_node(child)

    def _lower_equality_goal(self, node: ASTNode, scope: _Scope) -> GoalExpr:
        terms = [
            self._lower_term(child, scope) for child in _node_children(node, "term")
        ]
        operator = _single_token_child(
            _single_node_child(node, "equality_operator"),
        ).value
        return eq(terms[0], terms[1]) if operator == "=" else neq(terms[0], terms[1])

    def _lower_callable_term(self, node: ASTNode, scope: _Scope) -> Term:
        return self._lower_term(_single_node_child(node), scope)

    def _lower_term(self, node: ASTNode, scope: _Scope) -> Term:
        if node.rule_name == "term":
            return self._lower_term(_single_node_child(node), scope)
        if node.rule_name == "compound_term":
            name = self._lower_atom_token(_single_node_child(node, "atom_token"))
            arguments_node = _optional_node_child(node, "term_arguments")
            arguments = (
                ()
                if arguments_node is None
                else tuple(
                    self._lower_term(child, scope)
                    for child in _node_children(arguments_node, "term")
                )
            )
            return term(name, *arguments)
        if node.rule_name == "atom_term":
            return atom(self._lower_atom_token(_single_node_child(node, "atom_token")))
        if node.rule_name == "variable_term":
            return scope.variable(_single_token_child(node).value)
        if node.rule_name == "anonymous_term":
            return scope.anonymous()
        if node.rule_name == "number_term":
            token = _single_token_child(node)
            if token.type_name == "FLOAT":
                return num(float(token.value))
            return num(int(token.value))
        if node.rule_name == "string_term":
            return string(_single_token_child(node).value)
        if node.rule_name == "list_term":
            return self._lower_list(node, scope)
        raise _unexpected_node(node)

    def _lower_list(self, node: ASTNode, scope: _Scope) -> Term:
        body = _optional_node_child(node, "list_body")
        if body is None:
            return logic_list([])

        item_nodes: list[ASTNode] = []
        tail: Term | None = None
        after_bar = False
        for child in body.children:
            if isinstance(child, Token):
                after_bar = after_bar or child.type_name == "BAR"
                continue
            if child.rule_name != "term":
                continue
            if after_bar:
                tail = self._lower_term(child, scope)
            else:
                item_nodes.append(child)

        items = [self._lower_term(child, scope) for child in item_nodes]
        return logic_list(items, tail=tail)

    def _lower_atom_token(self, node: ASTNode) -> str:
        return _atom_name(_single_token_child(node).value)

    def _term_as_goal(self, term_value: Term, node: ASTNode) -> GoalExpr:
        if isinstance(term_value, Atom):
            name = term_value.symbol.name
            if term_value.symbol.namespace is None and name == "true":
                return succeed()
            if term_value.symbol.namespace is None and name == "fail":
                return fail()
            return relation(term_value.symbol, 0)()
        if isinstance(term_value, Compound):
            return relation(term_value.functor, len(term_value.args))(*term_value.args)
        raise _node_error(node, "term is not callable as a goal")

    def _term_as_relation_call(
        self,
        term_value: Term,
        message: str,
    ) -> RelationCall:
        if isinstance(term_value, Atom):
            return relation(term_value.symbol, 0)()
        if isinstance(term_value, Compound):
            return relation(term_value.functor, len(term_value.args))(*term_value.args)
        token = Token("EOF", "", 1, 1)
        raise PrologParseError(token, message)


def _node_children(node: ASTNode, rule_name: str | None = None) -> list[ASTNode]:
    """Return AST children, optionally filtered by grammar rule name."""

    children = [child for child in node.children if isinstance(child, ASTNode)]
    if rule_name is None:
        return children
    return [child for child in children if child.rule_name == rule_name]


def _single_node_child(node: ASTNode, rule_name: str | None = None) -> ASTNode:
    """Return exactly one AST child from ``node``."""

    children = _node_children(node, rule_name)
    if len(children) != 1:
        raise _node_error(
            node,
            f"expected one {rule_name or 'AST'} child, found {len(children)}",
        )
    return children[0]


def _optional_node_child(node: ASTNode, rule_name: str) -> ASTNode | None:
    """Return zero or one AST child by rule name."""

    children = _node_children(node, rule_name)
    if len(children) > 1:
        raise _node_error(node, f"expected at most one {rule_name} child")
    return children[0] if children else None


def _single_token_child(node: ASTNode) -> Token:
    """Return exactly one direct token child from ``node``."""

    children = [child for child in node.children if isinstance(child, Token)]
    if len(children) != 1:
        raise _node_error(node, f"expected one token child, found {len(children)}")
    return children[0]


def _node_error(node: ASTNode, message: str) -> PrologParseError:
    token = _first_token(node)
    if token is None:
        token = Token("EOF", "", node.start_line or 1, node.start_column or 1)
    return PrologParseError(token, message)


def _unexpected_node(node: ASTNode) -> PrologParseError:
    return _node_error(node, f"unexpected {node.rule_name} node")


def _first_token(node: ASTNode) -> Token | None:
    for child in node.children:
        if isinstance(child, Token):
            return child
        token = _first_token(child)
        if token is not None:
            return token
    return None


def _atom_name(value: str) -> str:
    """Normalize lexer atom text into the engine's atom symbol name."""

    if len(value) >= 2 and value[0] == "'" and value[-1] == "'":
        return value[1:-1].replace("\\'", "'").replace("\\\\", "\\")
    return value


def parse_source(source: str) -> ParsedSource:
    """Parse clauses and queries from Prolog source text."""

    return lower_ast(parse_ast(source))


def lower_goal_ast(ast: ASTNode) -> ParsedQuery:
    """Lower a ``goal`` AST node into an executable query-shaped object."""

    scope = _Scope(variables={})
    goal = _Lowerer()._lower_goal(ast, scope)
    return ParsedQuery(goal=goal, variables=dict(scope.variables))


def lower_ast(ast: ASTNode) -> ParsedSource:
    """Lower a Prolog grammar AST into executable engine objects."""

    return _Lowerer().lower_source(ast)


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

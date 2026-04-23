"""Shared runtime model objects for Prolog dialect frontends."""

from __future__ import annotations

from dataclasses import dataclass, field
from functools import cache
from typing import Literal

from logic_engine import Atom, Compound, GoalExpr, LogicVar, Number, Relation, RelationCall, Term, goal_as_term
from symbol_core import Symbol, sym

__version__ = "0.1.0"

type OperatorAssociativity = Literal["xfx", "xfy", "yfx", "fx", "fy", "xf", "yf"]

_VALID_ASSOCIATIVITIES = frozenset({"xfx", "xfy", "yfx", "fx", "fy", "xf", "yf"})
_ASSOCIATIVITY_ARITY: dict[OperatorAssociativity, int] = {
    "xfx": 2,
    "xfy": 2,
    "yfx": 2,
    "fx": 1,
    "fy": 1,
    "xf": 1,
    "yf": 1,
}


def _validate_operator_associativity(value: str) -> OperatorAssociativity:
    if value not in _VALID_ASSOCIATIVITIES:
        msg = f"invalid operator associativity: {value!r}"
        raise ValueError(msg)
    return value  # type: ignore[return-value]


def _validate_precedence(value: int) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        msg = "operator precedence must be an integer"
        raise TypeError(msg)
    if not 0 <= value <= 1200:
        msg = "operator precedence must be between 0 and 1200"
        raise ValueError(msg)
    return value


@dataclass(frozen=True, slots=True)
class OperatorSpec:
    """A single operator declaration such as `:-/2 xfx 1200`."""

    symbol: Symbol
    precedence: int
    associativity: OperatorAssociativity

    def __post_init__(self) -> None:
        object.__setattr__(self, "precedence", _validate_precedence(self.precedence))
        object.__setattr__(
            self,
            "associativity",
            _validate_operator_associativity(self.associativity),
        )

    @property
    def arity(self) -> int:
        """Return the operator arity implied by its associativity."""

        return _ASSOCIATIVITY_ARITY[self.associativity]

    def key(self) -> tuple[Symbol, OperatorAssociativity]:
        """Return the immutable key used for table indexing."""

        return (self.symbol, self.associativity)


def operator(
    name: str | Symbol,
    precedence: int,
    associativity: OperatorAssociativity,
) -> OperatorSpec:
    """Construct one operator specification."""

    symbol = name if isinstance(name, Symbol) else sym(name)
    return OperatorSpec(
        symbol=symbol,
        precedence=precedence,
        associativity=associativity,
    )


@dataclass(frozen=True, slots=True)
class OperatorTable:
    """An immutable collection of operator declarations."""

    operators: tuple[OperatorSpec, ...] = ()
    _by_key: dict[tuple[Symbol, OperatorAssociativity], OperatorSpec] = field(
        init=False,
        repr=False,
        compare=False,
    )
    _by_symbol: dict[Symbol, tuple[OperatorSpec, ...]] = field(
        init=False,
        repr=False,
        compare=False,
    )

    def __post_init__(self) -> None:
        by_key: dict[tuple[Symbol, OperatorAssociativity], OperatorSpec] = {}
        by_symbol: dict[Symbol, list[OperatorSpec]] = {}
        for spec in self.operators:
            if not isinstance(spec, OperatorSpec):
                msg = "operator tables may only contain OperatorSpec values"
                raise TypeError(msg)
            key = spec.key()
            if key in by_key:
                msg = (
                    "duplicate operator declaration for "
                    f"{spec.symbol}/{spec.associativity}"
                )
                raise ValueError(msg)
            by_key[key] = spec
            by_symbol.setdefault(spec.symbol, []).append(spec)

        object.__setattr__(self, "_by_key", by_key)
        object.__setattr__(
            self,
            "_by_symbol",
            {symbol: tuple(specs) for symbol, specs in by_symbol.items()},
        )

    def get(
        self,
        name: str | Symbol,
        associativity: OperatorAssociativity,
    ) -> OperatorSpec | None:
        """Return one declaration by name and associativity."""

        symbol = name if isinstance(name, Symbol) else sym(name)
        return self._by_key.get((symbol, associativity))

    def named(self, name: str | Symbol) -> tuple[OperatorSpec, ...]:
        """Return every declaration for the given operator name."""

        symbol = name if isinstance(name, Symbol) else sym(name)
        return self._by_symbol.get(symbol, ())

    def define(
        self,
        precedence: int,
        associativity: OperatorAssociativity,
        *names: str | Symbol,
    ) -> OperatorTable:
        """Return a new table with operator declarations added or replaced.

        Prolog uses precedence `0` as the removal form of `op/3`, so this
        method follows that same convention.
        """

        validated_precedence = _validate_precedence(precedence)
        validated_associativity = _validate_operator_associativity(associativity)
        if not names:
            return self

        next_specs = [
            spec
            for spec in self.operators
            if not (
                spec.associativity == validated_associativity
                and spec.symbol
                in {name if isinstance(name, Symbol) else sym(name) for name in names}
            )
        ]
        if validated_precedence == 0:
            return OperatorTable(tuple(next_specs))

        for name in names:
            next_specs.append(
                operator(name, validated_precedence, validated_associativity),
            )
        return OperatorTable(tuple(next_specs))


def empty_operator_table() -> OperatorTable:
    """Return an empty immutable operator table."""

    return OperatorTable()


@dataclass(frozen=True, slots=True)
class PrologDirective:
    """A parsed top-level `:- Goal.` directive."""

    goal: GoalExpr
    term: Term
    variables: dict[str, LogicVar]

    @property
    def relation(self) -> Relation | None:
        """Return the directive's top-level relation when it is a simple call."""

        if isinstance(self.goal, RelationCall):
            return self.goal.relation
        return None


def directive(
    goal: GoalExpr,
    variables: dict[str, LogicVar] | None = None,
) -> PrologDirective:
    """Construct a directive value from a lowered goal expression."""

    return PrologDirective(
        goal=goal,
        term=goal_as_term(goal),
        variables={} if variables is None else dict(variables),
    )


_EMPTY_LIST = sym("[]")
_LIST_CONS = sym(".")
_OP_DIRECTIVE = sym("op")


def apply_op_directive(
    operator_table: OperatorTable,
    directive_term: Term,
) -> OperatorTable:
    """Apply one ``op/3`` directive term to an operator table.

    Non-``op/3`` directive terms leave the table unchanged so callers can
    safely pass every parsed directive through this helper while only
    operator declarations take effect.
    """

    if not isinstance(directive_term, Compound) or directive_term.functor != _OP_DIRECTIVE:
        return operator_table

    if len(directive_term.args) != 3:
        msg = "op/3 directives require exactly three arguments"
        raise ValueError(msg)

    precedence_term, associativity_term, names_term = directive_term.args
    precedence = _directive_precedence(precedence_term)
    associativity = _directive_associativity(associativity_term)
    names = _directive_operator_names(names_term)
    return operator_table.define(precedence, associativity, *names)


def _directive_precedence(term_value: Term) -> int:
    if not isinstance(term_value, Number) or not isinstance(term_value.value, int):
        msg = "op/3 precedence must be an integer number term"
        raise TypeError(msg)
    return _validate_precedence(term_value.value)


def _directive_associativity(term_value: Term) -> OperatorAssociativity:
    if not isinstance(term_value, Atom):
        msg = "op/3 associativity must be an atom"
        raise TypeError(msg)
    return _validate_operator_associativity(term_value.symbol.name)


def _directive_operator_names(term_value: Term) -> tuple[str | Symbol, ...]:
    if isinstance(term_value, Atom):
        return (term_value.symbol,)

    items = _logic_list_items(term_value)
    if items is None:
        msg = "op/3 operator names must be an atom or proper list of atoms"
        raise TypeError(msg)

    names: list[str | Symbol] = []
    for item in items:
        if not isinstance(item, Atom):
            msg = "op/3 operator name lists may only contain atoms"
            raise TypeError(msg)
        names.append(item.symbol)
    return tuple(names)


def _logic_list_items(term_value: Term) -> list[Term] | None:
    items: list[Term] = []
    current = term_value
    while True:
        if isinstance(current, Atom) and current.symbol == _EMPTY_LIST:
            return items
        if (
            isinstance(current, Compound)
            and current.functor == _LIST_CONS
            and len(current.args) == 2
        ):
            items.append(current.args[0])
            current = current.args[1]
            continue
        return None


@cache
def iso_operator_table() -> OperatorTable:
    """Return the first shared ISO/Core operator defaults."""

    table = empty_operator_table()
    definitions: tuple[tuple[int, OperatorAssociativity, tuple[str, ...]], ...] = (
        (1200, "xfx", (":-", "-->")),
        (1100, "xfy", (";",)),
        (1050, "xfy", ("->",)),
        (1000, "xfy", (",",)),
        (900, "fy", ("\\+",)),
        (700, "xfx", ("=", "\\=", "is", "=:=", "=\\=", "<", "=<", ">", ">=")),
        (500, "yfx", ("+", "-")),
        (400, "yfx", ("*", "/", "//", "mod")),
        (200, "xfy", ("^",)),
        (200, "xfx", ("**",)),
        (200, "fy", ("+", "-")),
    )
    for precedence, associativity, names in definitions:
        table = table.define(precedence, associativity, *names)
    return table


@cache
def swi_operator_table() -> OperatorTable:
    """Return the first shared SWI-Prolog operator defaults."""

    return iso_operator_table().define(600, "xfy", ":")

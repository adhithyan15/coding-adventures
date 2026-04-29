"""Shared runtime model objects for Prolog dialect frontends."""

from __future__ import annotations

from dataclasses import dataclass, field
from functools import cache
from typing import Literal

from logic_engine import (
    Atom,
    Clause,
    Compound,
    GoalExpr,
    LogicVar,
    Number,
    Relation,
    RelationCall,
    Term,
    atom,
    goal_as_term,
    goal_from_term,
    logic_list,
    relation,
    rule,
    term,
    var,
)
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


@dataclass(frozen=True, slots=True)
class PrologTermExpansion:
    """One structured ``term_expansion/2`` declaration."""

    pattern: Term
    expansion: Term
    variables: dict[str, LogicVar]


@dataclass(frozen=True, slots=True)
class PrologGoalExpansion:
    """One structured ``goal_expansion/2`` declaration."""

    pattern: Term
    expansion: Term
    variables: dict[str, LogicVar]


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


@dataclass(frozen=True, slots=True)
class PrologModule:
    """One parsed module declaration plus its exported surface."""

    name: Symbol
    exports: tuple[Relation, ...] = ()
    exported_operators: tuple[OperatorSpec, ...] = ()


@dataclass(frozen=True, slots=True)
class PrologModuleImport:
    """One parsed `use_module/1` or `use_module/2` directive."""

    module_name: Symbol
    imports: tuple[Relation, ...] = ()
    import_all: bool = False


@dataclass(frozen=True, slots=True)
class PredicateSpec:
    """Metadata collected for one predicate indicator such as ``parent/2``."""

    relation: Relation
    dynamic: bool = False
    discontiguous: bool = False
    multifile: bool = False

    @property
    def key(self) -> tuple[Symbol, int]:
        """Return the immutable predicate indicator used for indexing."""

        return self.relation.key()


@dataclass(frozen=True, slots=True)
class PredicateRegistry:
    """An immutable registry of frontend predicate properties and directives."""

    predicates: tuple[PredicateSpec, ...] = ()
    initialization_directives: tuple[PrologDirective, ...] = ()
    term_expansions: tuple[PrologTermExpansion, ...] = ()
    goal_expansions: tuple[PrologGoalExpansion, ...] = ()
    _by_key: dict[tuple[Symbol, int], PredicateSpec] = field(
        init=False,
        repr=False,
        compare=False,
    )

    def __post_init__(self) -> None:
        by_key: dict[tuple[Symbol, int], PredicateSpec] = {}
        for spec in self.predicates:
            if not isinstance(spec, PredicateSpec):
                msg = "predicate registries may only contain PredicateSpec values"
                raise TypeError(msg)
            if spec.key in by_key:
                msg = (
                    "duplicate predicate specification for "
                    f"{spec.relation.symbol}/{spec.relation.arity}"
                )
                raise ValueError(msg)
            by_key[spec.key] = spec

        for directive_value in self.initialization_directives:
            if not isinstance(directive_value, PrologDirective):
                msg = (
                    "predicate registry initialization directives must be "
                    "PrologDirective values"
                )
                raise TypeError(msg)

        for expansion_value in self.term_expansions:
            if not isinstance(expansion_value, PrologTermExpansion):
                msg = (
                    "predicate registry term expansions must be "
                    "PrologTermExpansion values"
                )
                raise TypeError(msg)

        for expansion_value in self.goal_expansions:
            if not isinstance(expansion_value, PrologGoalExpansion):
                msg = (
                    "predicate registry goal expansions must be "
                    "PrologGoalExpansion values"
                )
                raise TypeError(msg)

        object.__setattr__(self, "_by_key", by_key)

    def get(
        self,
        name: str | Symbol | Relation,
        arity: int | None = None,
    ) -> PredicateSpec | None:
        """Return predicate metadata by relation or predicate indicator."""

        relation_value = _coerce_relation(name, arity)
        return self._by_key.get(relation_value.key())

    def define(
        self,
        *relations: Relation,
        dynamic: bool = False,
        discontiguous: bool = False,
        multifile: bool = False,
    ) -> PredicateRegistry:
        """Return a registry with metadata added or merged for each relation."""

        if not relations:
            return self

        updates = dict(self._by_key)
        for relation_value in relations:
            checked = _coerce_relation(relation_value)
            existing = updates.get(checked.key())
            updates[checked.key()] = PredicateSpec(
                relation=checked,
                dynamic=dynamic if existing is None else existing.dynamic or dynamic,
                discontiguous=(
                    discontiguous
                    if existing is None
                    else existing.discontiguous or discontiguous
                ),
                multifile=(
                    multifile if existing is None else existing.multifile or multifile
                ),
            )

        return PredicateRegistry(
            predicates=tuple(updates.values()),
            initialization_directives=self.initialization_directives,
            term_expansions=self.term_expansions,
            goal_expansions=self.goal_expansions,
        )

    def add_initialization(self, directive_value: PrologDirective) -> PredicateRegistry:
        """Return a registry that appends one structured initialization directive."""

        if not isinstance(directive_value, PrologDirective):
            msg = "initialization directives must be PrologDirective values"
            raise TypeError(msg)
        return PredicateRegistry(
            predicates=self.predicates,
            initialization_directives=(
                *self.initialization_directives,
                directive_value,
            ),
            term_expansions=self.term_expansions,
            goal_expansions=self.goal_expansions,
        )

    def add_term_expansion(
        self,
        expansion_value: PrologTermExpansion,
    ) -> PredicateRegistry:
        """Return a registry that appends one structured term expansion."""

        if not isinstance(expansion_value, PrologTermExpansion):
            msg = "term expansions must be PrologTermExpansion values"
            raise TypeError(msg)
        return PredicateRegistry(
            predicates=self.predicates,
            initialization_directives=self.initialization_directives,
            term_expansions=(*self.term_expansions, expansion_value),
            goal_expansions=self.goal_expansions,
        )

    def add_goal_expansion(
        self,
        expansion_value: PrologGoalExpansion,
    ) -> PredicateRegistry:
        """Return a registry that appends one structured goal expansion."""

        if not isinstance(expansion_value, PrologGoalExpansion):
            msg = "goal expansions must be PrologGoalExpansion values"
            raise TypeError(msg)
        return PredicateRegistry(
            predicates=self.predicates,
            initialization_directives=self.initialization_directives,
            term_expansions=self.term_expansions,
            goal_expansions=(*self.goal_expansions, expansion_value),
        )

    def dynamic_relations(self) -> tuple[Relation, ...]:
        """Return every predicate currently marked as dynamic."""

        return tuple(spec.relation for spec in self.predicates if spec.dynamic)


def empty_predicate_registry() -> PredicateRegistry:
    """Return an empty immutable predicate registry."""

    return PredicateRegistry()


_EMPTY_LIST = sym("[]")
_LIST_CONS = sym(".")
_OP_DIRECTIVE = sym("op")
_DYNAMIC_DIRECTIVE = sym("dynamic")
_DISCONTIGUOUS_DIRECTIVE = sym("discontiguous")
_MULTIFILE_DIRECTIVE = sym("multifile")
_INITIALIZATION_DIRECTIVE = sym("initialization")
_TERM_EXPANSION_DIRECTIVE = sym("term_expansion")
_GOAL_EXPANSION_DIRECTIVE = sym("goal_expansion")
_PREDICATE_INDICATOR = sym("/")
_MODULE_DIRECTIVE = sym("module")
_USE_MODULE_DIRECTIVE = sym("use_module")
_DCG_CONJUNCTION = sym(",")
_DCG_DISJUNCTION = sym(";")
_DCG_BRACED_GOAL = sym("{}")
_DCG_CUT = sym("!")


def apply_op_directive(
    operator_table: OperatorTable,
    directive_term: Term,
) -> OperatorTable:
    """Apply one ``op/3`` directive term to an operator table.

    Non-``op/3`` directive terms leave the table unchanged so callers can
    safely pass every parsed directive through this helper while only
    operator declarations take effect.
    """

    if (
        not isinstance(directive_term, Compound)
        or directive_term.functor != _OP_DIRECTIVE
    ):
        return operator_table

    if len(directive_term.args) != 3:
        msg = "op/3 directives require exactly three arguments"
        raise ValueError(msg)

    precedence_term, associativity_term, names_term = directive_term.args
    precedence = _directive_precedence(precedence_term)
    associativity = _directive_associativity(associativity_term)
    names = _directive_operator_names(names_term)
    return operator_table.define(precedence, associativity, *names)


def apply_predicate_directive(
    predicate_registry: PredicateRegistry,
    directive_value: PrologDirective,
) -> PredicateRegistry:
    """Apply one recognized predicate-property directive to a registry.

    Non-property directives leave the registry unchanged so callers can feed
    every parsed directive through this helper while only the currently
    supported frontend directives take effect.
    """

    if not isinstance(directive_value, PrologDirective):
        msg = "predicate directives must be PrologDirective values"
        raise TypeError(msg)

    term_value = directive_value.term
    if not isinstance(term_value, Compound):
        return predicate_registry

    if term_value.functor == _INITIALIZATION_DIRECTIVE:
        if len(term_value.args) != 1:
            msg = "initialization/1 directives require exactly one argument"
            raise ValueError(msg)
        return predicate_registry.add_initialization(directive_value)

    parsed_term_expansion = term_expansion_from_directive(directive_value)
    if parsed_term_expansion is not None:
        return predicate_registry.add_term_expansion(parsed_term_expansion)

    parsed_goal_expansion = goal_expansion_from_directive(directive_value)
    if parsed_goal_expansion is not None:
        return predicate_registry.add_goal_expansion(parsed_goal_expansion)

    if len(term_value.args) != 1:
        return predicate_registry

    predicate_property = _directive_predicate_property(term_value.functor)
    if predicate_property is None:
        return predicate_registry

    relations = _directive_relations(
        term_value.args[0],
        directive_name=term_value.functor.name,
    )
    if predicate_property == "dynamic":
        return predicate_registry.define(*relations, dynamic=True)
    if predicate_property == "discontiguous":
        return predicate_registry.define(*relations, discontiguous=True)
    return predicate_registry.define(*relations, multifile=True)


def module_spec_from_directive(
    directive_value: PrologDirective,
) -> PrologModule | None:
    """Parse a `module/2` directive into shared module metadata."""

    if not isinstance(directive_value, PrologDirective):
        msg = "module directives must be PrologDirective values"
        raise TypeError(msg)

    term_value = directive_value.term
    if not isinstance(term_value, Compound) or term_value.functor != _MODULE_DIRECTIVE:
        return None
    if len(term_value.args) != 2:
        msg = "module/2 directives require exactly two arguments"
        raise ValueError(msg)

    name_term, export_list_term = term_value.args
    if not isinstance(name_term, Atom):
        msg = "module/2 module names must be atoms"
        raise TypeError(msg)

    exports, exported_operators = _module_exports(export_list_term)
    return PrologModule(
        name=name_term.symbol,
        exports=exports,
        exported_operators=exported_operators,
    )


def module_import_from_directive(
    directive_value: PrologDirective,
) -> PrologModuleImport | None:
    """Parse one `use_module/1` or `use_module/2` directive."""

    if not isinstance(directive_value, PrologDirective):
        msg = "module import directives must be PrologDirective values"
        raise TypeError(msg)

    term_value = directive_value.term
    if (
        not isinstance(term_value, Compound)
        or term_value.functor != _USE_MODULE_DIRECTIVE
    ):
        return None
    if len(term_value.args) not in {1, 2}:
        msg = "use_module directives require one or two arguments"
        raise ValueError(msg)

    module_term = term_value.args[0]
    if not isinstance(module_term, Atom):
        msg = "use_module/1 and use_module/2 currently expect an atom module name"
        raise TypeError(msg)

    if len(term_value.args) == 1:
        return PrologModuleImport(module_name=module_term.symbol, import_all=True)

    imports = _directive_relations(term_value.args[1], directive_name="use_module")
    return PrologModuleImport(
        module_name=module_term.symbol,
        imports=imports,
        import_all=False,
    )


def term_expansion_from_directive(
    directive_value: PrologDirective,
) -> PrologTermExpansion | None:
    """Parse one ``term_expansion/2`` declaration into shared metadata."""

    parsed = _expansion_from_directive(
        directive_value,
        directive_name=_TERM_EXPANSION_DIRECTIVE,
    )
    if parsed is None:
        return None
    return PrologTermExpansion(
        pattern=parsed[0],
        expansion=parsed[1],
        variables=dict(directive_value.variables),
    )


def goal_expansion_from_directive(
    directive_value: PrologDirective,
) -> PrologGoalExpansion | None:
    """Parse one ``goal_expansion/2`` declaration into shared metadata."""

    parsed = _expansion_from_directive(
        directive_value,
        directive_name=_GOAL_EXPANSION_DIRECTIVE,
    )
    if parsed is None:
        return None
    return PrologGoalExpansion(
        pattern=parsed[0],
        expansion=parsed[1],
        variables=dict(directive_value.variables),
    )


def expand_dcg_clause(head_term: Term, body_term: Term) -> Clause:
    """Expand one DCG rule into an ordinary executable clause."""

    dcg_input = var("__DcgInput")
    dcg_output = var("__DcgOutput")
    expanded_head = _append_dcg_state(
        head_term,
        dcg_input,
        dcg_output,
        context="DCG head must be callable",
    )
    relation_head = _term_as_relation_call(
        expanded_head,
        "DCG head must be callable",
    )
    expanded_body = expand_dcg_body(body_term, dcg_input, dcg_output)
    return rule(relation_head, goal_from_term(expanded_body))


def expand_dcg_body(
    body_term: Term,
    dcg_input: Term,
    dcg_output: Term,
) -> Term:
    """Expand one DCG body term into an ordinary Prolog goal term."""

    terminals = _dcg_terminal_pattern(body_term)
    if terminals is not None:
        items, tail = terminals
        if isinstance(tail, Atom) and tail.symbol == _EMPTY_LIST:
            return term("=", dcg_input, logic_list(items, tail=dcg_output))
        return term(
            ",",
            term("=", dcg_input, logic_list(items, tail=tail)),
            term("=", tail, dcg_output),
        )

    if isinstance(body_term, Atom) and body_term.symbol == _DCG_CUT:
        return term(",", body_term, term("=", dcg_input, dcg_output))

    if isinstance(body_term, Compound):
        if body_term.functor == _DCG_CONJUNCTION and len(body_term.args) == 2:
            dcg_middle = var("__DcgMiddle")
            return term(
                ",",
                expand_dcg_body(body_term.args[0], dcg_input, dcg_middle),
                expand_dcg_body(body_term.args[1], dcg_middle, dcg_output),
            )
        if body_term.functor == _DCG_DISJUNCTION and len(body_term.args) == 2:
            return term(
                ";",
                expand_dcg_body(body_term.args[0], dcg_input, dcg_output),
                expand_dcg_body(body_term.args[1], dcg_input, dcg_output),
            )
        if body_term.functor == _DCG_BRACED_GOAL and len(body_term.args) == 1:
            return term(
                ",",
                body_term.args[0],
                term("=", dcg_input, dcg_output),
            )

    return _append_dcg_state(
        body_term,
        dcg_input,
        dcg_output,
        context="unsupported DCG body item",
    )


def expand_dcg_phrase(
    goal_term: Term,
    dcg_input: Term,
    dcg_output: Term | None = None,
) -> Term:
    """Expand one ``phrase/2`` or ``phrase/3`` call into an ordinary goal term."""

    return _append_dcg_state(
        goal_term,
        dcg_input,
        atom("[]") if dcg_output is None else dcg_output,
        context="phrase/2 and phrase/3 expect a callable grammar goal",
    )


def _directive_predicate_property(
    directive_name: Symbol,
) -> Literal["dynamic", "discontiguous", "multifile"] | None:
    if directive_name == _DYNAMIC_DIRECTIVE:
        return "dynamic"
    if directive_name == _DISCONTIGUOUS_DIRECTIVE:
        return "discontiguous"
    if directive_name == _MULTIFILE_DIRECTIVE:
        return "multifile"
    return None


def _module_exports(
    export_list_term: Term,
) -> tuple[tuple[Relation, ...], tuple[OperatorSpec, ...]]:
    items = _logic_list_items(export_list_term)
    if items is None:
        msg = "module/2 export lists must be proper lists"
        raise TypeError(msg)

    exports: list[Relation] = []
    exported_operators: list[OperatorSpec] = []
    for item in items:
        relation_value = _indicator_relation(item)
        if relation_value is not None:
            exports.append(relation_value)
            continue

        if isinstance(item, Compound) and item.functor == _OP_DIRECTIVE:
            exported_operators.extend(_module_operator_exports(item))
            continue

        msg = (
            "module/2 export lists may only contain predicate indicators "
            "or op(Precedence, Type, Name) declarations"
        )
        raise TypeError(msg)

    return (tuple(exports), tuple(exported_operators))


def _module_operator_exports(export_term: Compound) -> tuple[OperatorSpec, ...]:
    if len(export_term.args) != 3:
        msg = "module/2 operator exports must use op/3 terms"
        raise ValueError(msg)
    precedence_term, associativity_term, names_term = export_term.args
    precedence = _directive_precedence(precedence_term)
    associativity = _directive_associativity(associativity_term)
    names = _directive_operator_names(names_term)
    if precedence == 0:
        msg = "module/2 operator exports may not remove operators"
        raise ValueError(msg)
    return tuple(operator(name, precedence, associativity) for name in names)


def _expansion_from_directive(
    directive_value: PrologDirective,
    *,
    directive_name: Symbol,
) -> tuple[Term, Term] | None:
    if not isinstance(directive_value, PrologDirective):
        msg = "expansion directives must be PrologDirective values"
        raise TypeError(msg)

    term_value = directive_value.term
    if not isinstance(term_value, Compound) or term_value.functor != directive_name:
        return None
    if len(term_value.args) != 2:
        msg = f"{directive_name.name}/2 directives require exactly two arguments"
        raise ValueError(msg)
    return (term_value.args[0], term_value.args[1])


def _append_dcg_state(
    callable_term: Term,
    dcg_input: Term,
    dcg_output: Term,
    *,
    context: str,
) -> Term:
    if isinstance(callable_term, Atom):
        return term(callable_term.symbol, dcg_input, dcg_output)
    if isinstance(callable_term, Compound):
        return term(
            callable_term.functor,
            *callable_term.args,
            dcg_input,
            dcg_output,
        )
    raise TypeError(context)


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


def _directive_relations(
    term_value: Term,
    *,
    directive_name: str,
) -> tuple[Relation, ...]:
    relation_value = _indicator_relation(term_value)
    if relation_value is not None:
        return (relation_value,)

    items = _logic_list_items(term_value)
    if items is None:
        msg = (
            f"{directive_name}/1 expects a predicate indicator or proper list "
            "of predicate indicators"
        )
        raise TypeError(msg)

    relations: list[Relation] = []
    for item in items:
        parsed = _indicator_relation(item)
        if parsed is None:
            msg = (
                f"{directive_name}/1 lists may only contain predicate "
                "indicators of the form name/arity"
            )
            raise TypeError(msg)
        relations.append(parsed)
    return tuple(relations)


def _indicator_relation(term_value: Term) -> Relation | None:
    if (
        not isinstance(term_value, Compound)
        or term_value.functor != _PREDICATE_INDICATOR
        or len(term_value.args) != 2
    ):
        return None

    name_term, arity_term = term_value.args
    if not isinstance(name_term, Atom):
        return None
    if not isinstance(arity_term, Number) or not isinstance(arity_term.value, int):
        return None
    if arity_term.value < 0:
        msg = "predicate indicator arity must be non-negative"
        raise ValueError(msg)
    return relation(name_term.symbol, arity_term.value)


def _term_as_relation_call(term_value: Term, message: str) -> RelationCall:
    if isinstance(term_value, RelationCall):
        return term_value
    if isinstance(term_value, Atom):
        return relation(term_value.symbol, 0)()
    if isinstance(term_value, Compound):
        return relation(term_value.functor, len(term_value.args))(*term_value.args)
    raise TypeError(message)


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


def _dcg_terminal_pattern(term_value: Term) -> tuple[list[Term], Term] | None:
    items: list[Term] = []
    current = term_value
    while True:
        if isinstance(current, Atom) and current.symbol == _EMPTY_LIST:
            return (items, current)
        if (
            isinstance(current, Compound)
            and current.functor == _LIST_CONS
            and len(current.args) == 2
        ):
            items.append(current.args[0])
            current = current.args[1]
            continue
        if items:
            return (items, current)
        return None


def _coerce_relation(
    name: str | Symbol | Relation,
    arity: int | None = None,
) -> Relation:
    if isinstance(name, Relation):
        if arity is not None and arity != name.arity:
            msg = "arity must match the provided relation"
            raise ValueError(msg)
        return name
    if arity is None:
        msg = "predicate registry lookups by name require an arity"
        raise TypeError(msg)
    return relation(name, arity)


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

    return (
        iso_operator_table()
        .define(700, "xfx", "#=", "#\\=", "#<", "#=<", "#>", "#>=", "in", "ins")
        .define(600, "xfy", ":")
        .define(450, "xfx", "..")
    )

"""Fault wrappers (VER04r section 8) over the levels this lane implements.

A *negative control* is how a runner proves it can fail: break exactly one
operation of the library -- not by editing the library, but by wrapping the
callable the runner holds -- and require the suites to notice.  v1 kept a
per-lane table of named faults; v2 moves the table into the corpus, so **a fault
is data**::

    {"fault": "generator-shift", "op": "multiply", "kind": "post",
     "expr": <pure IR over the op's params and `result`>, "requires": ["ir.core"]}

and the lane implements the three wrappers below **once** and deletes its table.

=========  ===================================================================
kind       effect
=========  ===================================================================
``pre``    rewrites parameters *before* the call.  The row has ``rewrites``, a
           list of ``{"param": p, "expr": e}``; every ``e`` reads **all** the
           parameters as they were passed in, and all rewrites are applied
           **simultaneously** (one rewrite never sees another's output), so
           swapping two parameters works.
``post``   rewrites the result: ``expr`` over the parameters and ``result``.
``replace``  supplies the result without trusting the call's value (the call is
           still made, and counted); ``expr`` is over the same names.
=========  ===================================================================

``result`` is the one reserved name for the call's result.  A fault expression
uses only the tags of its declared level and **never** a ``subject_op``: a fault
that called the library would not be a fault of one operation.

**Typing a fault row** (sections 16.17 R67 and 16.18 R94, 16.20 R96 to R98, slice
S10e).  A fault changes values on purpose, so its expression need only have a
*common type* with what it replaces, not flow to it: ``Bytes`` against
``Bytes<32>``, ``Int`` against ``Nat``, and -- for the ``off-by-one-consumed``
template -- a tuple ``(Bytes,Int)`` against ``(Bytes,Nat)``.  The declared type is
handed down to every branch of an ``if``, every arm of a ``match``, the payload of
an ``ok`` and each component of a ``tuple`` (the last under a narrower relation),
and a bytes literal where a plain ``Bytes`` is expected has the type ``Bytes``.

Why one op at a time?  Take ``mul(a, b) == mul(b, a)``.  Break *every* op by
XOR-ing the low bit of its result and the claim still holds (``mul(a,b)^1 ==
mul(b,a)^1``): the suite looks insensitive when it is not.  Scoping a fault to
exactly one op removes that ambiguity.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from .errors import KitError, Refusal
from .ir import (
    IMPLEMENTED_LEVELS,
    Compiled,
    Program,
    check_requires,
    evaluate,
    is_node,
    scan_tags,
    tag_level,
    walk,
)
from .semantics import STANDARD, Semantics
from .values import decode_value, value_to_json
from .wtypes import BYTES, Ty, check_types_table, fault_unify

KINDS = ("pre", "post", "replace")


@dataclass
class CompiledFault:
    """A fault row checked against its op, ready to wrap calls.

    ``params`` are the op's parameter names and types in order; ``ret`` its
    result type; ``rewrites`` the ``(param, compiled expression)`` pairs of a
    ``pre`` fault; ``expr`` the compiled expression of a ``post`` or ``replace``.
    """

    kind: str
    params: list[tuple[str, Ty]]
    ret: Ty
    types: dict
    sem: Semantics
    rewrites: list[tuple[str, Compiled]]
    expr: Compiled | None

    def rewrite_args(self, args: list) -> list:
        """The arguments a ``pre`` fault passes on (rewrites read the *original*)."""
        env = dict(zip([p for p, _t in self.params], args, strict=True))
        pairs = [
            (name, lambda e, c=compiled: evaluate(c, e))
            for name, compiled in self.rewrites
        ]
        new = self.sem.apply_pre(pairs, env)  # type: ignore[arg-type]
        return [new.get(name, env[name]) for name, _t in self.params]

    def rewrite_result(self, args: list, result: object) -> object:
        """What a ``post``/``replace`` fault returns, from args and real result."""
        assert self.expr is not None
        env = dict(zip([p for p, _t in self.params], args, strict=True))
        env["result"] = result
        return evaluate(self.expr, env)

    def wrap(self, subject: Callable[..., object]) -> Callable[..., object]:
        """The faulted callable: the fault applied around ``subject(*args)``."""

        def faulted(*args: object) -> object:
            call_args = (
                self.rewrite_args(list(args)) if self.kind == "pre" else list(args)
            )
            result = subject(*call_args)
            if self.kind == "pre":
                return result
            # `replace` still made the call above, so it is counted; it then
            # distrusts the value.  Both kinds read the ORIGINAL arguments.
            return self.rewrite_result(list(args), result)

        return faulted


_KEYS_OF_A_FAULT = {
    "pre": frozenset({"fault", "op", "kind", "rewrites", "requires"}),
    "post": frozenset({"fault", "op", "kind", "expr", "requires"}),
    "replace": frozenset({"fault", "op", "kind", "expr", "requires"}),
}


def _ill_formed(why: str) -> KitError:
    """A malformed fault row breaks the case's own schema: a kit error (R126, R45 f).

    It is never an outcome: ``ill-typed`` covers the types table and the typing
    of the row's expressions, not the shape of the row itself.
    """
    return KitError(f"a fault row is malformed: {why}")


def _check_fault_shape(fault: object) -> dict:
    """The fault row, if it is a closed object of one of the three kinds (8.1).

    Every defect is a :class:`KitError` (R126), the undefined level name of
    ``requires`` included (section 4.2 rule 7 makes it a corpus error in a
    corpus, but a schema break in a kit case).  This is read before the table.
    """
    if not isinstance(fault, dict) or fault.get("kind") not in KINDS:
        raise _ill_formed("its kind is pre, post or replace")
    if set(fault) != _KEYS_OF_A_FAULT[fault["kind"]]:
        raise _ill_formed(f"the keys of a {fault['kind']} fault are fixed")
    if type(fault["op"]) is not str or type(fault["fault"]) is not str:
        raise _ill_formed("fault and op are names")
    requires = fault["requires"]
    if type(requires) is not list or not all(type(level) is str for level in requires):
        raise _ill_formed("requires is a list of level names")
    # An undefined level name is a corpus error, never `level-missing` (4.2 rule 7).
    try:
        check_requires(requires, "corpus", "a fault's requires")
    except Refusal as refusal:
        raise _ill_formed(str(refusal)) from refusal
    if fault["kind"] == "pre":
        _check_rewrite_rows(fault["rewrites"], None)
    return fault


def _check_rewrite_rows(
    rows: object, scope: dict[str, Ty] | None
) -> list[tuple[str, object]]:
    """The ``(parameter, expression)`` pairs of a ``pre`` fault, each row checked.

    A rewrite names a parameter that exists, at most one rewrite names it, and
    the row has exactly the two keys ``param`` and ``expr``.  With ``scope``
    ``None`` only the shape is read (before the table); the parameter's
    existence needs the op row.  Both are kit errors (R126).
    """
    if type(rows) is not list or not rows:
        raise _ill_formed("a pre fault must rewrite something")
    seen: set[str] = set()
    pairs: list[tuple[str, object]] = []
    for row in rows:
        if type(row) is not dict or set(row) != {"param", "expr"}:
            raise _ill_formed("a rewrite is {param, expr}")
        param = row["param"]
        if type(param) is not str or (scope is not None and param not in scope):
            raise _ill_formed("a rewrite targets a parameter missing from the op")
        if param in seen:
            raise _ill_formed("a parameter is rewritten twice")
        seen.add(param)
        pairs.append((param, row["expr"]))
    return pairs


def _common(produced: Ty, declared: Ty, narrow: bool) -> bool:
    """Do ``produced`` and ``declared`` have a common type? (R94, R96, R97)

    The relation of a fault row, :func:`~.wtypes.fault_unify`: at the top
    (``narrow`` false) R51's ``common_type`` (equal types; ``Nat`` and ``Int``;
    two ``Bytes`` of different or forgotten width, because a fault may change a
    length by design; ``Option`` and ``Result`` of types that relate, with one
    error type); an undetermined ``?`` (a bare ``ok`` or ``err``) relates to
    anything (R97); and below a **tuple** the narrower relation of R96.
    """
    return fault_unify(produced, declared, narrow) is not None


def _positions(expr: object, declared: Ty) -> list[tuple[object, Ty, bool]]:
    """Every ``(node, expected type, narrow)`` the declared type is handed down to.

    R98 / R67: the declared type goes to the expression itself, to each branch of
    an ``if`` and each arm of a ``match``, to the payload of an ``ok`` and to
    component ``i`` of a ``tuple`` (the last one with the narrow relation, and so
    does everything below it).  Only the *shape* of the declared type is read, so
    this runs before the expression is typed.  A malformed node simply hands
    nothing down: the compile that follows refuses it.
    """
    found: list[tuple[object, Ty, bool]] = []
    pending: list[tuple[object, Ty, bool]] = [(expr, declared, False)]
    while pending:
        node, expected, narrow = pending.pop()
        found.append((node, expected, narrow))
        if not is_node(node):
            continue
        tag = node[0]  # type: ignore[index]
        if tag == "if" and len(node) == 4:  # type: ignore[arg-type]
            pending += [(node[2], expected, narrow), (node[3], expected, narrow)]  # type: ignore[index]
        elif tag == "match" and len(node) == 3 and type(node[2]) is list:  # type: ignore[arg-type]
            pending += [
                (arm[1], expected, narrow)
                for arm in node[2]  # type: ignore[index]
                if type(arm) is list and len(arm) == 2
            ]
        elif tag == "ok" and len(node) == 2 and expected.head == "Result":  # type: ignore[arg-type]
            pending.append((node[1], expected.args[0], narrow))  # type: ignore[index]
        elif (
            tag == "tuple"
            and len(node) == 2  # type: ignore[arg-type]
            and type(node[1]) is list  # type: ignore[index]
            and expected.head == "()"
            and len(node[1]) == len(expected.args)  # type: ignore[index]
        ):
            pending += [
                (part, wanted, True)
                for part, wanted in zip(node[1], expected.args, strict=True)  # type: ignore[index]
            ]
    return found


def _mark_plain_bytes(
    program: Program, positions: list[tuple[object, Ty, bool]]
) -> None:
    """R67: a bytes literal where a plain ``Bytes`` is expected has the type ``Bytes``.

    Also records which ``if`` and ``match`` nodes sit at a position the declared
    type is handed to: their branches are typed with the fault relation (R98),
    and every other ``if`` or ``match`` by R51 alone.
    """
    for node, expected, narrow in positions:
        if is_node(node) and node[0] in ("if", "match"):  # type: ignore[index]
            program.handed[id(node)] = narrow
        if (
            expected == BYTES
            and is_node(node)
            and node[0] == "lit"  # type: ignore[index]
            and len(node) == 2  # type: ignore[arg-type]
            and type(node[1]) is dict  # type: ignore[index]
            and node[1].get("type") == "Bytes"  # type: ignore[index]
        ):
            program.plain_bytes.add(id(node))


def _check_positions(
    program: Program, positions: list[tuple[object, Ty, bool]]
) -> None:
    """Every position of a compiled fault expression has a common declared type.

    The positions that only *pass the declared type down* (an ``if``, a ``match``,
    an ``ok`` against a ``Result``, a ``tuple`` against a tuple of the same arity)
    are compared through their branches, arms, payload and components, not by
    their own type: the type the typer builds for them may take one branch's
    shape (R98).  Every other position is compared by its own static type.
    """
    assert program.recorded is not None
    for node, expected, narrow in positions:
        if not is_node(node):  # pragma: no cover - compile refused it already
            continue
        tag = node[0]  # type: ignore[index]
        if (
            tag in ("if", "match")
            or (tag == "ok" and expected.head == "Result")
            or (
                tag == "tuple"
                and expected.head == "()"
                and len(node[1]) == len(expected.args)  # type: ignore[index]
            )
        ):
            continue
        if not _common(program.recorded[id(node)], expected, narrow):
            raise Refusal(
                "ill-typed",
                "a fault expression has no common type with what it replaces",
            )


def _static_order(exprs: list[object], available: frozenset[str]) -> None:
    """Orders 2 to 4 of R119 (c), over **every** expression of the row.

    ``level-missing`` for a tag whose level the runner lacks (a tag of no level
    is not a level question), then ``depth``, ``unknown-tag`` and ``model-order``
    (a fault case has no models, so every ``model_call`` is one), then
    ``subject-forbidden``.  Each step reads all the expressions before the next
    begins, so one rewrite's ``subject_op`` never hides another rewrite's static
    code.  The row's ``requires`` is checked by the caller, before this.
    """
    for expr in exprs:
        for node, _depth in walk(expr):
            level = tag_level(node[0])
            if level is not None and level not in available:
                raise Refusal("level-missing", str(node[0]))
    for expr in exprs:
        scan_tags(expr, "body", models=(), subjects=False)
    for expr in exprs:
        for node, _depth in walk(expr):
            if node[0] == "subject_op":
                raise Refusal(
                    "subject-forbidden", "a fault expression calls no subject"
                )


def compile_fault(
    fault: object,
    op_row: object,
    types: dict,
    levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
    sem: Semantics = STANDARD,
) -> CompiledFault:
    """Check a fault row against its op's section 6 row and compile its expressions.

    A row that is not a well-formed fault is a :class:`KitError` (R126), read
    before anything else.  Refusals (the kit's ``v2-fault`` codes, in this order:
    R110 (b), R119 (c), R121): ``ill-typed`` for a ``types`` table that breaks
    section 2.2 or an op row whose parameter or result type is refused (order
    1); ``level-missing`` when the row's ``requires`` (or a tag in its
    expressions) is a level this runner lacks (2) -- a lane records such a
    control ``not-live``, it is not an error of the lane; ``unknown-tag``,
    ``depth`` and ``model-order`` (3, no order among them: a fault case has no
    models, so every ``model_call`` is one); ``subject-forbidden`` for a
    ``subject_op`` in a fault expression (4); and ``ill-typed`` when the
    expressions do not produce the type of what they replace (5, F6).
    """
    row = _check_fault_shape(fault)
    # Order 1 (R110 (b), R121): the table, then the op row's parameter and
    # result types, both `ill-typed`; [D74], and `corpus` is not in the closed
    # set of `v2-fault`.  The Program reads the op row, so it is built here,
    # before the `requires` check.
    check_types_table(types, "ill-typed")
    program = Program(types=types, ops={row["op"]: op_row}, levels=levels, sem=sem)
    declared = program.op(row["op"])
    # Order 2: `level-missing`, from the row's `requires`, before any expression.
    available = frozenset(levels) & IMPLEMENTED_LEVELS
    if not set(row["requires"]) <= available:
        raise Refusal("level-missing", "the fault needs a level this runner lacks")
    params, ret = declared.unbound()
    scope = dict(params)
    if "result" in scope:
        raise Refusal("ill-typed", "an op with a parameter named result")
    program.recorded = {}
    if row["kind"] == "pre":
        rows = _check_rewrite_rows(row["rewrites"], scope)
        # Orders 2 to 4 for every rewrite before any rewrite is compiled, so a
        # typing defect in one never hides a static code in another.
        _static_order([expr for _param, expr in rows], available)
        placed = [(_positions(expr, scope[param])) for param, expr in rows]
        for positions in placed:
            _mark_plain_bytes(program, positions)
        rewrites = [
            (param, program.compile(expr, scope, role="fault")) for param, expr in rows
        ]
        # The typing of the row is the last step (R119 c, 5): every rewrite is
        # scanned and compiled before any rewrite's type is compared.
        for positions in placed:
            _check_positions(program, positions)
        return CompiledFault("pre", params, ret, types, sem, rewrites, None)
    scope["result"] = ret
    _static_order([row["expr"]], available)
    positions = _positions(row["expr"], ret)
    _mark_plain_bytes(program, positions)
    expr = program.compile(row["expr"], scope, role="fault")
    _check_positions(program, positions)
    return CompiledFault(row["kind"], params, ret, types, sem, [], expr)


def apply_fault(
    fault: object,
    op_row: object,
    types: dict,
    levels: frozenset[str] | set[str],
    args: list,
    result: object,
    sem: Semantics = STANDARD,
) -> dict:
    """The ``v2-fault`` outcome of a fault over one call (JSON ``args`` and ``result``).

    The call is played by a stand-in that records the arguments it received and
    returns ``result``.  For ``pre`` the outcome is ``{"args": [...]}`` -- **every**
    argument in parameter order after the rewrite, unchanged ones included; for
    ``post`` and ``replace`` it is ``{"result": V}``, typed as the op's
    *declared* result type.
    """
    compiled = compile_fault(fault, op_row, types, levels, sem)
    values = [
        decode_value(raw, ty, types, sem)
        for raw, (_name, ty) in zip(args, compiled.params, strict=True)
    ]
    real = decode_value(result, compiled.ret, types, sem)
    seen: list[list] = []

    def stand_in(*call_args: object) -> object:
        seen.append(list(call_args))
        return real

    out = compiled.wrap(stand_in)(*values)
    if compiled.kind == "pre":
        return {
            "args": [
                value_to_json(v, ty, types)
                for v, (_n, ty) in zip(seen[0], compiled.params, strict=True)
            ]
        }
    return {"result": value_to_json(out, compiled.ret, types)}

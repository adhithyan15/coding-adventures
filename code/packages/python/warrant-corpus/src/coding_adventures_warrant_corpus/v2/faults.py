"""Fault wrappers at ``ir.core`` (VER04r section 8).

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

Why one op at a time?  Take ``mul(a, b) == mul(b, a)``.  Break *every* op by
XOR-ing the low bit of its result and the claim still holds (``mul(a,b)^1 ==
mul(b,a)^1``): the suite looks insensitive when it is not.  Scoping a fault to
exactly one op removes that ambiguity.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from .errors import Refusal
from .ir import IMPLEMENTED_LEVELS, Compiled, Program, check_requires, evaluate
from .semantics import STANDARD, Semantics
from .values import decode_value, value_to_json
from .wtypes import Ty, check_types_table, unify

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


def _ill_formed(why: str) -> Refusal:
    return Refusal("ill-typed", f"a fault row is malformed: {why}")


def _check_fault_shape(fault: object) -> dict:
    """The fault row, if it is a closed object of one of the three kinds (8.1)."""
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
    check_requires(requires, "corpus", "a fault's requires")
    return fault


def _check_rewrite_rows(rows: object, scope: dict[str, Ty]) -> list[tuple[str, object]]:
    """The ``(parameter, expression)`` pairs of a ``pre`` fault, each row checked.

    A rewrite names a parameter that exists, at most one rewrite names it, and
    the row has exactly the two keys ``param`` and ``expr``.
    """
    if type(rows) is not list or not rows:
        raise _ill_formed("a pre fault must rewrite something")
    seen: set[str] = set()
    pairs: list[tuple[str, object]] = []
    for row in rows:
        if type(row) is not dict or set(row) != {"param", "expr"}:
            raise _ill_formed("a rewrite is {param, expr}")
        param = row["param"]
        if type(param) is not str or param not in scope:
            raise _ill_formed("a rewrite targets a parameter missing from the op")
        if param in seen:
            raise _ill_formed("a parameter is rewritten twice")
        seen.add(param)
        pairs.append((param, row["expr"]))
    return pairs


def _may_replace(produced: Ty, wanted: Ty) -> bool:
    """May an expression of type ``produced`` stand where a ``wanted`` value goes?

    A fault changes values on purpose, and sometimes their length: dropping the
    last byte of a ``Bytes<32>`` result yields a plain ``Bytes``, and ``Nat + 1``
    is an ``Int`` (section 3.2).  So the test is the weaker one the checker uses
    for two branches, that the two types have a *common* type (R51), not that one
    flows into the other.  The value is held to the declared type when it is
    written, so an out-of-range or wrong-length one is refused there, never
    truncated.
    """
    return unify(produced, wanted) is not None


def compile_fault(
    fault: object,
    op_row: object,
    types: dict,
    levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
    sem: Semantics = STANDARD,
) -> CompiledFault:
    """Check a fault row against its op's section 6 row and compile its expressions.

    Refusals (the kit's ``v2-fault`` codes, in this order: R110 (b)): ``ill-typed``
    for a ``types`` table that breaks section 2.2 (read whole, before any
    expression), then ``level-missing`` when the row's
    ``requires`` (or a tag in its expressions) is a level this runner lacks -- a
    lane records such a control ``not-live``, it is not an error of the lane --
    and ``subject-forbidden`` for a ``subject_op`` in a fault expression.  A row
    that is not a well-formed fault, or whose expressions do not produce the type
    of what they replace, is ``ill-typed`` (F6).
    """
    row = _check_fault_shape(fault)
    # Section 2.2, again by every consumer: the table is read whole, and first
    # after the row's own shape.  Its code in a fault case is `ill-typed`
    # ([D74], R110 (b)); `corpus` is not in the closed set of `v2-fault`.
    check_types_table(types, "ill-typed")
    available = frozenset(levels) & IMPLEMENTED_LEVELS
    if not set(row["requires"]) <= available:
        raise Refusal("level-missing", "the fault needs a level this runner lacks")
    program = Program(types=types, ops={row["op"]: op_row}, levels=levels, sem=sem)
    declared = program.op(row["op"])
    params, ret = declared.unbound()
    scope = dict(params)
    if "result" in scope:
        raise Refusal("ill-typed", "an op with a parameter named result")
    if row["kind"] == "pre":
        rewrites = []
        for param, expr in _check_rewrite_rows(row["rewrites"], scope):
            compiled = program.compile(expr, scope, role="fault")
            if not _may_replace(compiled.ty, scope[param]):
                raise Refusal("ill-typed", f"the rewrite of {param} has the wrong type")
            rewrites.append((param, compiled))
        return CompiledFault("pre", params, ret, types, sem, rewrites, None)
    scope["result"] = ret
    expr = program.compile(row["expr"], scope, role="fault")
    if not _may_replace(expr.ty, ret):
        raise Refusal(
            "ill-typed", "the fault expression is not of the op's result type"
        )
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

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
from .ir import IMPLEMENTED_LEVELS, Compiled, Program, evaluate
from .semantics import STANDARD, Semantics
from .values import decode_value, value_to_json
from .wtypes import Ty

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


def compile_fault(
    fault: object,
    op_row: object,
    types: dict,
    levels: frozenset[str] | set[str] = IMPLEMENTED_LEVELS,
    sem: Semantics = STANDARD,
) -> CompiledFault:
    """Check a fault row against its op's section 6 row and compile its expressions.

    Refusals (the kit's ``v2-fault`` codes): ``level-missing`` when the row's
    ``requires`` (or a tag in its expressions) is a level this runner lacks -- a
    lane records such a control ``not-live``, it is not an error of the lane --
    and ``subject-forbidden`` for a ``subject_op`` in a fault expression.  A row
    that is not a well-formed fault is ``ill-typed``.
    """
    if not isinstance(fault, dict) or fault.get("kind") not in KINDS:
        raise Refusal("ill-typed", "a fault is pre, post or replace")
    available = frozenset(levels) & IMPLEMENTED_LEVELS
    if not set(fault.get("requires", ())) <= available:
        raise Refusal("level-missing", "the fault needs a level this runner lacks")
    assert isinstance(op_row, dict)
    program = Program(types=types, ops={fault["op"]: op_row}, levels=levels, sem=sem)
    sig = program.ops[fault["op"]]
    scope = {name: ty for name, ty in sig.params}
    if "result" in scope:
        raise Refusal("ill-typed", "an op with a parameter named result")
    if fault["kind"] == "pre":
        rewrites = [
            (row["param"], program.compile(row["expr"], scope, role="fault"))
            for row in fault["rewrites"]
        ]
        return CompiledFault("pre", sig.params, sig.ret, types, sem, rewrites, None)
    scope["result"] = sig.ret
    expr = program.compile(fault["expr"], scope, role="fault")
    return CompiledFault(fault["kind"], sig.params, sig.ret, types, sem, [], expr)


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

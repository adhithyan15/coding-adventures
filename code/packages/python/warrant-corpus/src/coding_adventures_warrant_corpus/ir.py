"""The model IR: compile a language-neutral expression tree to a closure.

Every claim body in the corpus is a tiny expression tree, for example::

    ["eq", ["var", "a"], ["var", "b"]]          a == b
    ["ok", ["if", ["var","choice"], ["var","a"], ["var","b"]]]
    ["subject_op", "eq", [["var","a"], ["var","b"]]]     call the real export

There are exactly two kinds of call, and keeping them apart is what keeps the
oracle outside the library (VER04i 2.5):

``subject_op``  dispatches through the binding to the **real** export.  It is
                allowed only in a claim's subject side, never in a model.
``model_call``  invokes a named, pure model (``same``, ``same_u64``).

The tree is *compiled once* into nested Python closures before any case runs.
That has two benefits: an unknown or malformed node is a corpus error
immediately (never a skipped case), and evaluating 140,000 cases costs a
function call per node instead of re-interpreting a list of lists.

Arithmetic (``add sub mul``) is checked, not wrapped: this profile has no
``U64`` arithmetic in any model, and a silent wrap would hide a corpus that
needed one.  A result outside ``0 .. 2**64-1`` is an error.
"""

from __future__ import annotations

from collections.abc import Callable

from .errors import CorpusError, need
from .values import U64_MAX

Env = dict[str, object]
Compiled = Callable[[Env], object]
# (op name, evaluated arguments, static width or None) -> normalised result
SubjectCall = Callable[[str, list[object], int | None], object]

MAX_DEPTH = 64

# Static upper bound on the node evaluations one expression may cost per case,
# counting every model call at the full cost of its body.  A chain of models
# that each call the next one twice costs 2**n; it is refused at compile time,
# before any case runs, instead of burning CPU for hours.
MAX_IR_COST = 2_000

_BINARY = {
    "eq": lambda a, b: a == b,
    "ne": lambda a, b: a != b,
    "lt": lambda a, b: a < b,
    "le": lambda a, b: a <= b,
    "gt": lambda a, b: a > b,
    "ge": lambda a, b: a >= b,
    "and": lambda a, b: a and b,
    "or": lambda a, b: a or b,
    "implies": lambda a, b: (not a) or b,
}
_ARITHMETIC = {
    "add": lambda a, b: a + b,
    "sub": lambda a, b: a - b,
    "mul": lambda a, b: a * b,
}


def _operand(value: object) -> int:
    """An arithmetic operand must be a real int in ``0 .. 2**64-1``.

    Checked *before* the operation, so ``mul`` can never build a huge
    intermediate (a 10**6-bit operand squared) just to be rejected afterwards,
    and ``True`` is not silently the number 1.
    """
    if type(value) is not int or not 0 <= value <= U64_MAX:
        raise CorpusError("ir-overflow", "arithmetic operand is outside U64")
    return value


def _checked(result: int) -> int:
    if not 0 <= result <= U64_MAX:
        raise CorpusError("ir-overflow", "model arithmetic left the U64 range")
    return result


class Compiler:
    """Compiles IR nodes against a model table and an optional subject hook."""

    def __init__(
        self,
        models: dict[str, object],
        subject: SubjectCall | None,
        ops: dict[str, object],
    ) -> None:
        self._models = models
        self._subject = subject
        self._ops = ops
        self._compiled_models: dict[str, tuple[list[str], Compiled]] = {}
        self._in_progress: set[str] = set()
        self._model_costs: dict[str, int] = {}

    # -- models ---------------------------------------------------------
    def _model(self, name: str) -> tuple[list[str], Compiled]:
        if name in self._compiled_models:
            return self._compiled_models[name]
        need(name in self._models, "unsupported-ir", "unknown model call")
        need(name not in self._in_progress, "unsupported-ir", "recursive model")
        model = self._models[name]
        need(
            type(model) is dict and set(model) == {"params", "body"},
            "unsupported-ir",
            "malformed model",
        )
        assert isinstance(model, dict)
        params = model["params"]
        need(
            type(params) is list and all(type(p) is str for p in params),
            "unsupported-ir",
            "malformed model parameters",
        )
        self._in_progress.add(name)
        # A model body may only call other pure models: no subject hook.
        body = Compiler(self._models, None, self._ops)
        body._compiled_models = self._compiled_models
        body._in_progress = self._in_progress
        body._model_costs = self._model_costs
        compiled = body.compile(model["body"], set(params), 0)
        self._in_progress.discard(name)
        self._compiled_models[name] = (list(params), compiled)
        return self._compiled_models[name]

    # -- expressions ----------------------------------------------------
    def compile(self, node: object, scope: set[str], depth: int = 0) -> Compiled:
        """Compile ``node``; ``scope`` is the set of variable names in scope.

        A top-level call (``depth == 0``) also enforces :data:`MAX_IR_COST`
        and turns a Python ``RecursionError`` (an absurdly long model chain)
        into a corpus error.
        """
        if depth != 0:
            return self._compile(node, scope, depth)
        try:
            compiled = self._compile(node, scope, 0)
            self._cost(node)
        except RecursionError as error:
            raise CorpusError("ir-limit", "model IR nests too deeply") from error
        return compiled

    def _cost(self, node: list[object]) -> int:
        """Static upper bound on node evaluations (see :data:`MAX_IR_COST`)."""
        tag = node[0]
        if tag in ("len", "ok", "not"):
            children = [node[1]]
        elif tag == "if":
            children = node[1:4]
        elif tag in _BINARY or tag in _ARITHMETIC:
            children = node[1:3]
        elif tag in ("model_call", "subject_op"):
            children = node[2]  # type: ignore[assignment]
        else:
            children = []
        total = 1 + sum(self._cost(child) for child in children)  # type: ignore[arg-type]
        if tag == "model_call":
            total += self._model_cost(str(node[1]))
        if total > MAX_IR_COST:
            raise CorpusError("ir-limit", "model IR cost exceeds the work limit")
        return total

    def _model_cost(self, name: str) -> int:
        if name not in self._model_costs:
            body = self._models[name]["body"]  # type: ignore[index]
            self._model_costs[name] = self._cost(body)  # type: ignore[arg-type]
        return self._model_costs[name]

    def _compile(self, node: object, scope: set[str], depth: int) -> Compiled:
        need(depth <= MAX_DEPTH, "ir-limit", "model IR nests too deeply")
        need(
            type(node) is list and bool(node) and type(node[0]) is str,
            "unsupported-ir",
            "malformed or unknown model IR node",
        )
        assert isinstance(node, list)
        tag = node[0]
        sub = lambda child: self._compile(child, scope, depth + 1)  # noqa: E731

        if tag == "lit":
            need(len(node) == 2, "unsupported-ir", "malformed literal")
            value = self._literal(node[1])
            return lambda _env: value
        if tag == "var":
            need(
                len(node) == 2 and type(node[1]) is str and node[1] in scope,
                "unsupported-ir",
                "unknown variable",
            )
            name = node[1]
            return lambda env: env[name]
        if tag == "len":
            need(len(node) == 2, "unsupported-ir", "malformed len")
            inner = sub(node[1])
            return lambda env: len(inner(env))  # type: ignore[arg-type]
        if tag == "ok":
            need(len(node) == 2, "unsupported-ir", "malformed ok")
            inner = sub(node[1])
            return lambda env: ("ok", inner(env))
        if tag in ("err", "name"):
            need(
                len(node) == 2 and type(node[1]) is str,
                "unsupported-ir",
                "malformed err/name",
            )
            variant = node[1]
            return lambda _env: (tag, variant)
        if tag == "not":
            need(len(node) == 2, "unsupported-ir", "malformed not")
            inner = sub(node[1])
            return lambda env: not inner(env)
        if tag == "if":
            need(len(node) == 4, "unsupported-ir", "malformed if")
            cond, then, other = sub(node[1]), sub(node[2]), sub(node[3])
            return lambda env: then(env) if cond(env) else other(env)
        if tag in _BINARY:
            need(len(node) == 3, "unsupported-ir", "malformed binary node")
            left, right, function = sub(node[1]), sub(node[2]), _BINARY[tag]
            return lambda env: function(left(env), right(env))
        if tag in _ARITHMETIC:
            need(len(node) == 3, "unsupported-ir", "malformed arithmetic node")
            left, right, function = sub(node[1]), sub(node[2]), _ARITHMETIC[tag]
            return lambda env: _checked(
                function(_operand(left(env)), _operand(right(env)))
            )
        if tag == "model_call":
            return self._model_call(node, sub)
        if tag == "subject_op":
            return self._subject_op(node, sub)
        raise CorpusError("unsupported-ir", "unknown model IR tag")

    def _literal(self, value: object) -> object:
        if type(value) is bool:
            return value
        if type(value) is int and not isinstance(value, bool):
            return value
        raise CorpusError("unsupported-ir", "unsupported literal")

    def _model_call(
        self, node: list[object], sub: Callable[[object], Compiled]
    ) -> Compiled:
        need(
            len(node) == 3 and type(node[1]) is str and type(node[2]) is list,
            "unsupported-ir",
            "malformed model_call",
        )
        params, body = self._model(str(node[1]))
        args = [sub(argument) for argument in node[2]]  # type: ignore[union-attr]
        need(len(args) == len(params), "unsupported-ir", "model arity mismatch")
        return lambda env: body(
            {name: argument(env) for name, argument in zip(params, args, strict=True)}
        )

    def _subject_op(
        self, node: list[object], sub: Callable[[object], Compiled]
    ) -> Compiled:
        need(
            self._subject is not None,
            "unsupported-ir",
            "a subject call is forbidden in a model or admission expression",
        )
        need(
            len(node) in (3, 4) and type(node[1]) is str and type(node[2]) is list,
            "unsupported-ir",
            "malformed subject_op",
        )
        name = str(node[1])
        need(name in self._ops, "unsupported-ir", "subject call to an unknown op")
        width: int | None = None
        if len(node) == 4:
            widths = node[3]
            need(
                type(widths) is list
                and len(widths) == 1
                and type(widths[0]) is int
                and widths[0] >= 0,
                "unsupported-ir",
                "malformed static width",
            )
            width = int(widths[0])  # type: ignore[index]
        signature = self._ops[name]
        needs_width = bool(getattr(signature, "width_params", ()))
        need(
            needs_width == (width is not None),
            "unsupported-ir",
            "a fixed-width op needs its width, and any other op must not state one",
        )
        args = [sub(argument) for argument in node[2]]  # type: ignore[union-attr]
        call = self._subject
        assert call is not None
        return lambda env: call(name, [argument(env) for argument in args], width)

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
        compiled = body.compile(model["body"], set(params), 0)
        self._in_progress.discard(name)
        self._compiled_models[name] = (list(params), compiled)
        return self._compiled_models[name]

    # -- expressions ----------------------------------------------------
    def compile(self, node: object, scope: set[str], depth: int = 0) -> Compiled:
        """Compile ``node``; ``scope`` is the set of variable names in scope."""
        need(depth <= MAX_DEPTH, "ir-limit", "model IR nests too deeply")
        need(
            type(node) is list and bool(node) and type(node[0]) is str,
            "unsupported-ir",
            "malformed or unknown model IR node",
        )
        assert isinstance(node, list)
        tag = node[0]
        sub = lambda child: self.compile(child, scope, depth + 1)  # noqa: E731

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
            return lambda env: _checked(function(left(env), right(env)))  # type: ignore[arg-type]
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
        args = [sub(argument) for argument in node[2]]  # type: ignore[union-attr]
        call = self._subject
        assert call is not None
        return lambda env: call(name, [argument(env) for argument in args], width)

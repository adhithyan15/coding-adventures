"""The binding table: Warrant operations -> this lane's callables.

``evidence.bindings`` is *data*.  It says which public export implements each
Warrant operation and how an exception becomes a declared ``Err`` -- and
nothing else.  It cannot hold logic (a computed answer, a retry, a fallback):
that is precisely why it is a flat file of directives and not Python.  See
VER04i section 5.1 for the grammar::

    lane python
    package python/ct-compare
    module ct_compare
    import coding_adventures_ct_compare
    source src/coding_adventures_ct_compare
    seed 0x5741_5252_414e_5421
    values Bool=bool Bytes=bytes Nat=int U64=int
    op eq export=ct_eq
    op select_bytes export=ct_select_bytes errors=CT_SELECT_LENGTH_MISMATCH:ValueError

Two guards make the binding trustworthy at the boundary:

* **Resolution at start-up.**  Every export must exist and be callable *before*
  a single case runs (a lazily-missing export would otherwise surface as a
  pass on whichever suites never reached it).
* **Source binding.**  The imported module's file must live under the declared
  ``source`` root.  The manifest's ``source_sha256`` digests that root; if the
  interpreter had instead picked up a stale installed copy, the digest would
  describe code that never ran.

Error normalisation is closed (VER04i 5, rule 2): a *mapped* exception class
becomes ``Err(variant)``; any other exception is ``unexpected-error``.  Exception
names resolve only among the interpreter's built-in exception classes, never to
``BaseException``/``Exception`` themselves, so a binding cannot swallow
everything.
"""

from __future__ import annotations

import builtins
import importlib
import re
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from .errors import UNEXPECTED_ERROR, BindingError, CorpusError
from .faults import Fault
from .values import OpSig

_NAME = re.compile(r"[a-z][a-z0-9_]*")
_DOTTED = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*")
_VARIANT = re.compile(r"[A-Z][A-Z0-9_]*")
_SEED = re.compile(r"(?:0x[0-9a-fA-F_]+|[0-9][0-9_]*)")
_PYTHON_VALUES = {"Bool": "bool", "Bytes": "bytes", "U64": "int", "Nat": "int"}
_FORBIDDEN_EXCEPTIONS = frozenset({"BaseException", "Exception"})
MAX_BINDING_BYTES = 64 * 1024


@dataclass(frozen=True)
class OpBinding:
    """How one Warrant op maps to an export."""

    name: str
    export: str
    width: str  # "none" | "implicit" | "runtime"
    errors: tuple[tuple[str, type[Exception]], ...] = ()


@dataclass
class Binding:
    """A parsed ``evidence.bindings``."""

    lane: str
    package: str
    module: str
    import_name: str
    source: str
    seed: int
    values: dict[str, str]
    ops: dict[str, OpBinding]
    raw: bytes = field(repr=False, default=b"")


def _exception_class(name: str) -> type[Exception]:
    candidate = getattr(builtins, name, None)
    if (
        name in _FORBIDDEN_EXCEPTIONS
        or not isinstance(candidate, type)
        or not issubclass(candidate, Exception)
    ):
        raise BindingError(
            "bad-exception", f"`{name}` is not an allowed exception class"
        )
    return candidate


def _parse_op(parts: list[str]) -> OpBinding:
    if len(parts) < 2 or _NAME.fullmatch(parts[0]) is None:
        raise BindingError("syntax", "malformed op directive")
    options: dict[str, str] = {}
    for item in parts[1:]:
        key, separator, value = item.partition("=")
        if not separator or not value or key in options:
            raise BindingError("syntax", f"malformed or duplicate op option `{key}`")
        options[key] = value
    unknown = set(options) - {"export", "width", "errors"}
    if unknown or "export" not in options:
        raise BindingError(
            "syntax", f"op `{parts[0]}` needs export= and only known options"
        )
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", options["export"]) is None:
        raise BindingError("syntax", "export must be an identifier")
    width = options.get("width", "none")
    if width not in ("none", "implicit", "runtime"):
        raise BindingError("syntax", "width must be implicit or runtime")
    errors: list[tuple[str, type[Exception]]] = []
    for row in filter(None, options.get("errors", "").split(",")):
        variant, separator, exception = row.partition(":")
        if not separator or _VARIANT.fullmatch(variant) is None:
            raise BindingError("syntax", "errors rows are VARIANT:ExceptionName")
        errors.append((variant, _exception_class(exception)))
    return OpBinding(parts[0], options["export"], width, tuple(errors))


def parse_binding(data: bytes) -> Binding:
    """Parse the directive file; any irregularity is a :class:`BindingError`."""
    if len(data) > MAX_BINDING_BYTES:
        raise BindingError("input-limit", "binding file is too large")
    try:
        text = data.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise BindingError("syntax", "binding is not UTF-8") from error
    single: dict[str, str] = {}
    values: dict[str, str] = {}
    ops: dict[str, OpBinding] = {}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        directive, *parts = line.split()
        if directive == "op":
            op = _parse_op(parts)
            if op.name in ops:
                raise BindingError("syntax", f"duplicate op `{op.name}`")
            ops[op.name] = op
        elif directive == "values":
            for item in parts:
                key, separator, value = item.partition("=")
                if not separator or key in values:
                    raise BindingError("syntax", "malformed or duplicate values row")
                values[key] = value
        elif directive in ("lane", "package", "module", "import", "source", "seed"):
            if len(parts) != 1 or directive in single:
                raise BindingError("syntax", f"`{directive}` takes one value, once")
            single[directive] = parts[0]
        else:
            raise BindingError("syntax", f"unknown directive `{directive}`")
    missing = {"lane", "package", "module", "import", "source", "seed"} - set(single)
    if missing or not ops:
        raise BindingError("incomplete", f"binding lacks {sorted(missing) or 'ops'}")
    if _DOTTED.fullmatch(single["import"]) is None:
        raise BindingError("syntax", "import must be a dotted module name")
    if _SEED.fullmatch(single["seed"]) is None:
        raise BindingError("syntax", "seed must be a decimal or 0x hex u64")
    seed = int(single["seed"].replace("_", ""), 0)
    if not 0 <= seed < 1 << 64:
        raise BindingError("syntax", "seed is outside u64")
    # Rule 3: width representations are stated, and forbidden ones are refused.
    for scalar, python_type in values.items():
        if _PYTHON_VALUES.get(scalar) != python_type:
            raise BindingError(
                "values",
                f"`{scalar}={python_type}` is not an allowed Python representation",
            )
    return Binding(
        lane=single["lane"],
        package=single["package"],
        module=single["module"],
        import_name=single["import"],
        source=single["source"],
        seed=seed,
        values=values,
        ops=ops,
        raw=data,
    )


# ---------------------------------------------------------------------------
# Resolution and the subject
# ---------------------------------------------------------------------------
class SubjectFailure(Exception):
    """The library misbehaved for one case: a verdict, not a lane error."""

    def __init__(self, failure_id: str) -> None:
        super().__init__(failure_id)
        self.failure_id = failure_id


def resolve_exports(
    binding: Binding, package_dir: Path
) -> dict[str, Callable[..., object]]:
    """Import the library and resolve every export, or refuse to start."""
    try:
        module = importlib.import_module(binding.import_name)
    except ImportError as error:
        raise BindingError(
            "import", f"cannot import `{binding.import_name}`"
        ) from error
    root = (package_dir / binding.source).resolve()
    origin = getattr(module, "__file__", None)
    if origin is None or not Path(origin).resolve().is_relative_to(root):
        raise BindingError(
            "source-mismatch",
            f"`{binding.import_name}` was not imported from the declared source root",
        )
    exports: dict[str, Callable[..., object]] = {}
    for name, op in binding.ops.items():
        target = getattr(module, op.export, None)
        if not callable(target):
            raise BindingError(
                "export", f"op `{name}`: `{op.export}` is missing or not callable"
            )
        exports[name] = target
    return exports


class Subject:
    """The real library, seen through the binding, with a call counter.

    ``call`` is the *only* path from a corpus case to the library.  It counts
    invocations (the runner compares the count with the corpus's
    ``expected_calls``, so a runner that never calls the binding is caught),
    applies an optional fault, and normalises the outcome to the model's value
    shape: a value, or ``("err", VARIANT)``.
    """

    def __init__(
        self,
        binding: Binding,
        exports: dict[str, Callable[..., object]],
        signatures: dict[str, OpSig],
        fault: Fault | None = None,
        fault_op: str | None = None,
    ) -> None:
        self._binding = binding
        self._exports = exports
        self._signatures = signatures
        self._fault = fault
        self._fault_op = fault_op
        self.calls: dict[str, int] = {name: 0 for name in binding.ops}

    def total_calls(self, ops: list[str]) -> int:
        """Calls made so far to any op in ``ops``."""
        return sum(self.calls[name] for name in ops)

    def _faulted(self, op: str) -> Fault | None:
        fault = self._fault
        if fault is None or (self._fault_op is not None and op != self._fault_op):
            return None
        return fault if fault.applies(self._signatures[op]) else None

    def call(self, op: str, args: list[object], width: int | None) -> object:
        """Invoke ``op`` on ``args``; return the normalised result."""
        signature = self._signatures[op]
        binding = self._binding.ops[op]
        self.calls[op] += 1
        if width is not None:
            for value in args:
                if type(value) is bytes and len(value) != width:
                    raise CorpusError(
                        "width", "a Bytes<N> argument does not have length N"
                    )
        # Wherever the signature itself states a length (`Bytes<4>`), assert it.
        for value, (_name, ty) in zip(args, signature.params, strict=False):
            if (
                ty.kind == "Bytes"
                and isinstance(ty.width, int)
                and (type(value) is not bytes or len(value) != ty.width)
            ):
                raise CorpusError(
                    "width", "an argument does not have its declared length"
                )
        fault = self._faulted(op)
        passed = fault.on_input(list(args), signature) if fault else args
        if binding.width == "runtime":
            passed = [*passed, width]
        try:
            returned = self._exports[op](*passed)
            # A library signals success by returning the value itself; the
            # model's Result wraps it.  Wrapping here (never trusting a
            # returned tuple) is what makes a stray ("err", ...) from the
            # library fail the type check instead of passing as an error.
            outcome: object = (
                ("ok", returned) if signature.ret.kind == "Result" else returned
            )
        except Exception as error:  # noqa: BLE001 - classified just below
            for variant, exception_class in binding.errors:
                # Exactly the mapped class: a subclass (an unrelated, more
                # specific ValueError) must not satisfy a rejection claim.
                if type(error) is exception_class:
                    outcome = ("err", variant)
                    break
            else:
                raise SubjectFailure(UNEXPECTED_ERROR) from None
        return fault.on_output(outcome, list(args), signature) if fault else outcome

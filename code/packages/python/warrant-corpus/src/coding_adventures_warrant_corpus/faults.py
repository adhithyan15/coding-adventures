"""The ten portable fault wrappers (VER04i section 4.2).

A *negative control* is how a runner proves it can fail.  We deliberately
break exactly one operation of the library -- not by editing the library, but
by wrapping the callable the runner holds -- and require the suites to notice.

Why wrap one op at a time
-------------------------
Take the claim ``mul(a, b) == mul(b, a)``.  Break *every* op by XOR-ing the low
bit of its result and the claim still holds: ``mul(a,b)^1 == mul(b,a)^1``.
The suite passes under the fault and looks "insensitive".  Per-op scoping
removes the ambiguity: a control is the triple (suite kind, op, fault) and
perturbs exactly one op.

Each fault has two halves, applied around the call:

* ``on_input``  rewrites the arguments *before* the library sees them;
* ``on_output`` rewrites the (already error-normalised) result afterwards.

======================  ==========================  =========================
fault                   applies when                behaviour
======================  ==========================  =========================
``invert``              op returns ``Bool``         negate the result
``ignore-last-byte``    op has ``Bytes`` params     zero each input's last byte
``ignore-high-bit``     op has ``U64`` params       clear bit 63 of each input
``swap-choice``         ``Bool`` param ``choice``   negate it
``missing-error``       op returns ``Result``       ``Err`` -> ``Ok(first Bytes arg)``
``xor-lsb``             returns U8/U64/Nat          ``result ^ 1``
``off-by-one``          returns U8/U64/Nat          ``result + 1`` (masked)
``return-first-arg``    two params of return type   return the first argument
``truncate-last``       op returns ``Bytes``        drop the final byte
``drop-to-none``        op returns ``Option``       ``None``
======================  ==========================  =========================

``Option`` is outside this profile's value model, so ``drop-to-none`` never
applies to a checked corpus today; it is kept so the registry matches the spec.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from .errors import UsageError
from .values import U64_MAX, OpSig

Args = list[object]
_INT_KINDS = ("U8", "U64", "Nat")
_WIDTH_MASK = {"U8": 0xFF, "U64": U64_MAX, "Nat": U64_MAX}


@dataclass(frozen=True)
class Fault:
    """A named transformer with a signature-based applicability test."""

    name: str
    applies: Callable[[OpSig], bool]
    on_input: Callable[[Args, OpSig], Args]
    on_output: Callable[[object, Args, OpSig], object]


def _same_input(args: Args, _sig: OpSig) -> Args:
    return args


def _same_output(result: object, _args: Args, _sig: OpSig) -> object:
    return result


def _has_param(sig: OpSig, kind: str) -> bool:
    return any(ty.kind == kind for _name, ty in sig.params)


def _map_params(
    args: Args, sig: OpSig, kind: str, change: Callable[[object], object]
) -> Args:
    return [
        change(value) if ty.kind == kind else value
        for value, (_name, ty) in zip(args, sig.params, strict=True)
    ]


def _zero_last(value: object) -> object:
    assert type(value) is bytes
    return value[:-1] + b"\x00" if value else value


def _swap_choice(args: Args, sig: OpSig) -> Args:
    return [
        (not value) if name == "choice" and ty.kind == "Bool" else value
        for value, (name, ty) in zip(args, sig.params, strict=True)
    ]


def _missing_error(result: object, args: Args, sig: OpSig) -> object:
    if type(result) is tuple and result and result[0] == "err":
        for value, (_name, ty) in zip(args, sig.params, strict=True):
            if ty.kind == "Bytes":
                return ("ok", value)
    return result


def _first_bytes_arg_exists(sig: OpSig) -> bool:
    return _has_param(sig, "Bytes")


def _off_by_one(result: object, _args: Args, sig: OpSig) -> object:
    assert type(result) is int
    return (result + 1) & _WIDTH_MASK[sig.ret.kind]


def _return_first(_result: object, args: Args, _sig: OpSig) -> object:
    return args[0]


def _two_params_of_return_type(sig: OpSig) -> bool:
    same = [ty.kind for _name, ty in sig.params if ty.kind == sig.ret.kind]
    return len(sig.params) == 2 and len(same) == 2


FAULTS: dict[str, Fault] = {
    fault.name: fault
    for fault in (
        Fault(
            "invert",
            lambda sig: sig.ret.kind == "Bool",
            _same_input,
            lambda result, _a, _s: not result,
        ),
        Fault(
            "ignore-last-byte",
            lambda sig: _has_param(sig, "Bytes"),
            lambda args, sig: _map_params(args, sig, "Bytes", _zero_last),
            _same_output,
        ),
        Fault(
            "ignore-high-bit",
            lambda sig: _has_param(sig, "U64"),
            lambda args, sig: _map_params(
                args,
                sig,
                "U64",
                lambda v: v & (U64_MAX >> 1),  # type: ignore[operator]
            ),
            _same_output,
        ),
        Fault(
            "swap-choice",
            lambda sig: any(n == "choice" and t.kind == "Bool" for n, t in sig.params),
            _swap_choice,
            _same_output,
        ),
        Fault(
            "missing-error",
            lambda sig: sig.ret.kind == "Result" and _first_bytes_arg_exists(sig),
            _same_input,
            _missing_error,
        ),
        Fault(
            "xor-lsb",
            lambda sig: sig.ret.kind in _INT_KINDS,
            _same_input,
            lambda result, _a, _s: result ^ 1,  # type: ignore[operator]
        ),
        Fault(
            "off-by-one",
            lambda sig: sig.ret.kind in _INT_KINDS,
            _same_input,
            _off_by_one,
        ),
        Fault(
            "return-first-arg",
            _two_params_of_return_type,
            _same_input,
            _return_first,
        ),
        Fault(
            "truncate-last",
            lambda sig: sig.ret.kind == "Bytes",
            _same_input,
            lambda result, _a, _s: result[:-1],  # type: ignore[index]
        ),
        Fault(
            "drop-to-none",
            lambda sig: sig.ret.text.startswith("Option<"),
            _same_input,
            lambda _r, _a, _s: ("none",),
        ),
    )
}


def get_fault(name: str) -> Fault:
    """Look a fault up by its portable name, or refuse."""
    if name not in FAULTS:
        raise UsageError("unknown-fault", f"unknown fault `{name}`")
    return FAULTS[name]


def fault_applies(fault: str, sig: OpSig) -> bool:
    """Does the registered ``fault`` apply to an op with signature ``sig``?"""
    return get_fault(fault).applies(sig)

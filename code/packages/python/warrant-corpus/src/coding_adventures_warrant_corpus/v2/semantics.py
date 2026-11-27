"""The places where an implementation of the v2 rules can go wrong.

Every rule of VER04r that an evaluator, a codec or an enumerator could
implement incorrectly is a method of :class:`Semantics`, and the library calls
that method instead of writing the rule inline.  :data:`STANDARD` is the
specification: it is what the lane runs.

Why a class and not just functions?  Because of the first acceptance item of
slice S6a ([D42]): *the conformance harness must be able to fail*.  A test that
cannot be shown to fail proves nothing, so the harness takes a ``Semantics``
object and the falsification tests (``tests/v2_mutants.py``) hand it a
deliberately wrong subclass -- one that does not wrap, one that evaluates both
sides of an ``and``, one that enumerates the first binder fastest -- and require
the kit to report the failing case ids.  The methods are exactly the seams the
section 16.13 mutant table names.

The class holds no state; the standard behaviour of each method is a few lines
and is written next to the rule it implements.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Iterator

from . import arith, fixed
from .errors import Refusal
from .wtypes import FIXED, Ty, Value, fixed_range

_DECIMAL = re.compile(r"-?(0|[1-9][0-9]*)")
MAX_DIGITS = 20  # section 2.3: at most 20 digits, the sign not counted


class Semantics:
    """The standard semantics of VER04r; subclass and override to build a mutant."""

    # -- section 2.3: decimal text -------------------------------------------
    def decode_int(self, text: object, ty: Ty) -> int:
        """Decode the decimal text of an integer value of type ``ty``.

        Digits only, with one leading ``-`` allowed for a *signed* type and only
        for a non-zero value (``"-0"`` is refused); no ``+``, no leading zero
        except ``"0"``; at most 20 digits; then range-checked against the type
        (``Nat`` is ``0..2^64-1``, ``Int`` only ever appears in an IR literal).
        A value that does not fit is refused, never truncated.
        """
        if not isinstance(text, str) or len(text) > MAX_DIGITS + 1:
            # Text longer than a sign and twenty digits is never a legal integer;
            # it is refused before any pattern looks at it.
            raise Refusal("value", "an integer is decimal text of at most 20 digits")
        if _DECIMAL.fullmatch(text) is None:
            raise Refusal("value", "an integer is canonical decimal text")
        if len(text.lstrip("-")) > MAX_DIGITS:
            raise Refusal("value", "more than 20 digits")
        number = int(text)
        if text.startswith("-") and (number == 0 or not self._may_be_negative(ty)):
            raise Refusal(
                "value", "a minus sign needs a signed type and a non-zero value"
            )
        low, high = self.value_range(ty)
        if not low <= number <= high:
            raise Refusal("value", "the integer is out of range for its type")
        return number

    @staticmethod
    def _may_be_negative(ty: Ty) -> bool:
        return ty.head == "Int" or (ty.head in FIXED and ty.signed)

    @staticmethod
    def value_range(ty: Ty) -> tuple[int, int]:
        """Inclusive range of the values a decoded integer of type ``ty`` may have."""
        if ty.head == "Nat":
            return 0, (1 << 64) - 1
        if ty.head == "Int":
            return -(10**20) + 1, 10**20 - 1  # [D58]: at most 20 digits
        return fixed_range(ty)

    # -- section 2.4: stream bytes -------------------------------------------
    def u32(self, number: int) -> bytes:
        """A ``u32`` length or count: four bytes, **little-endian**."""
        return number.to_bytes(4, "little")

    def encode_signed(self, value: int) -> bytes:
        """The eight bytes of a signed value: two's complement, sign-extended to 64."""
        return (value & ((1 << 64) - 1)).to_bytes(8, "little")

    # -- section 3: arithmetic -------------------------------------------------
    def wrap(self, value: int, ty: Ty) -> int:
        """Reduce into fixed-width ``ty`` (modulo ``2^w``, two's complement)."""
        return arith.wrap(value, ty)

    def fixed_binary(self, op: str, left: int, right: int, ty: Ty) -> int:
        """``add``, ``sub`` or ``mul`` on a fixed-width type: exact, then wrap."""
        return self.wrap(arith.raw_binary(op, left, right), ty)

    def fixed_neg(self, value: int, ty: Ty) -> int:
        """Unary ``-`` on a fixed-width type: ``-(-128i8)`` wraps to ``-128i8``."""
        return self.wrap(-value, ty)

    def exact(self, value: int) -> int:
        """Admit an exact ``Nat``/``Int`` result only inside the window (3.2)."""
        return arith.check_window(value)

    def order_key(self, value: Value, ty: Ty) -> Value:
        """The key a comparison orders by: the *mathematical* value of the operand.

        An unsigned type orders by its non-negative representative and a signed
        one by its signed value (section 3.2).  Python ints already are these.
        """
        return value

    # -- Appendix A.1: the ir.fixed built-ins ------------------------------------
    def shift(self, op: str, value: int, count: int, ty: Ty) -> int:
        """``shl``, ``shr`` or ``sar``: a count of ``w`` or more is decided *first*."""
        return fixed.shift(op, value, count, ty)

    def rotate(self, op: str, value: int, count: int, ty: Ty) -> int:
        """``rotl`` or ``rotr``: the count is reduced modulo ``w`` first."""
        return fixed.rotate(op, value, count, ty)

    def count_zeros(self, op: str, value: int, ty: Ty) -> int:
        """``clz`` or ``ctz``; both are ``w`` for the pattern 0."""
        return fixed.count_zeros(op, value, ty)

    def checked(self, op: str, left: int, right: int, ty: Ty) -> tuple:
        """``checked_add``/``sub``/``mul``: ``None`` iff the result leaves ``ty``."""
        return fixed.checked(op, left, right, ty)

    def divide(self, op: str, left: int, right: int, ty: Ty | None) -> tuple:
        """``div``/``rem`` (floor) and ``div_trunc``/``rem_trunc`` (toward zero)."""
        return fixed.divide(op, left, right, ty)

    # -- section 4.2 rule 2: `match` evaluates exactly one arm ----------------------
    def choose_arm(self, matching: list[int]) -> int:
        """The arm a ``match`` runs: the **first** whose pattern matches."""
        return matching[0]

    # -- section 5.1: a tuple descriptor's product ----------------------------------
    def component_digits(self, index: int, sizes: list[int]) -> list[int]:
        """Component indices of tuple value ``index``: the **last** varies fastest."""
        digits = [0] * len(sizes)
        for position in range(len(sizes) - 1, -1, -1):
            index, digits[position] = divmod(index, sizes[position])
        return digits

    # -- section 4: Bytes operators --------------------------------------------
    def take(self, data: bytes, count: int) -> bytes:
        """The first ``count`` bytes; a negative count is 0, a long one clamps."""
        return data[: max(count, 0)]

    def drop(self, data: bytes, count: int) -> bytes:
        """All but the first ``count`` bytes; total, the mirror of :meth:`take`."""
        return data[max(count, 0) :]

    # -- section 4.2 rule 2: short circuit --------------------------------------
    # ``left`` and ``right`` are zero-argument callables: calling one *evaluates*
    # that operand, which is what makes laziness observable (an untaken operand
    # that is never called contributes no subject call).
    def and_(self, left: Callable[[], Value], right: Callable[[], Value]) -> bool:
        return left() and right()

    def or_(self, left: Callable[[], Value], right: Callable[[], Value]) -> bool:
        return left() or right()

    def implies(self, left: Callable[[], Value], right: Callable[[], Value]) -> bool:
        return (not left()) or right()

    def if_(
        self,
        cond: Callable[[], Value],
        then: Callable[[], Value],
        other: Callable[[], Value],
    ) -> Value:
        return then() if cond() else other()

    # -- section 5.1: enumeration -------------------------------------------------
    def fx_edge_values(self, ty: Ty) -> list[int]:
        """The ``fx_edge`` domain of a 16-, 32- or 64-bit fixed-width type.

        Unsigned width ``w``: ``0..=255``; then ``1<<i`` for ``i = 8..=w-1``;
        then ``MAX``; then ``MAX ^ (1<<i)`` for ``i = 0..=w-1``.  A signed type
        [D21] reinterprets those bit patterns in two's complement, adds the small
        negatives ``-255..=-1``, removes duplicates and lists the result in
        ascending *numeric* order.
        """
        bits = ty.width
        top = (1 << bits) - 1
        unsigned = [
            *range(256),
            *(1 << i for i in range(8, bits)),
            top,
            *(top ^ (1 << i) for i in range(bits)),
        ]
        if not ty.signed:
            return unsigned
        signed = {self.wrap(v, ty) for v in unsigned} | set(range(-255, 0))
        return sorted(signed)

    def enumerate_indices(
        self, sizes: list[int], start: int, stop: int
    ) -> Iterator[list[int]]:
        """Yield the binder index vectors of product ordinals ``start .. stop-1``.

        Section 5.2: the product of binders ``b1..bk`` is **row-major, last
        binder fastest**, and ordinal ``p`` unranks to indices by mixed-radix
        division.  Unranking is done once, at ``start``; after that the vector is
        advanced like an odometer (the last digit turns first), so a chunk can be
        enumerated alone without touching an earlier chunk (section 5.4).
        """
        digits = self.unrank(start, sizes)
        last = len(sizes) - 1
        for _ in range(start, stop):
            yield digits
            position = last
            while position >= 0:
                digits[position] += 1
                if digits[position] < sizes[position]:
                    break
                digits[position] = 0
                position -= 1

    def unrank(self, ordinal: int, sizes: list[int]) -> list[int]:
        """Mixed-radix digits of ``ordinal``; the last binder is least significant."""
        digits = [0] * len(sizes)
        for position in range(len(sizes) - 1, -1, -1):
            ordinal, digits[position] = divmod(ordinal, sizes[position])
        return digits

    # -- section 5.4: chunks -------------------------------------------------------
    def advance_admitted(self, running: int, line_cases: int, chunk_size: int) -> int:
        """The **admitted** count after one more chunk (the next ``first_admitted``).

        Chunk boundaries are on *product* ordinals, but ``first_admitted`` counts
        *admitted* tuples, so the running sum adds the chunk's ``cases``, never
        ``chunk_size``.
        """
        return running + line_cases

    def chunk_prefix(self, context: bytes, chunk: int) -> bytes:
        """A chunk's stream prefix: the suite context, then the u32-LE chunk index."""
        return context + self.u32(chunk)

    # -- section 8.1: fault wrappers --------------------------------------------------
    def apply_pre(
        self, rewrites: list[tuple[str, Callable[[dict], object]]], env: dict
    ) -> dict:
        """The new parameter values of a ``pre`` fault: **all rewrites simultaneously**.

        Every rewrite expression reads the parameters *as they were passed in*;
        one rewrite never sees another's output.
        """
        return {param: expr(env) for param, expr in rewrites}


STANDARD = Semantics()

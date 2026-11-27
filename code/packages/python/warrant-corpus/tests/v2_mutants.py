"""Deliberately wrong implementations of the v2 rules (slice S6a, [D42]).

Each class below overrides exactly one :class:`Semantics` method with the
mistake a real implementation makes.  The falsification tests hand one to the
conformance harness and require it to *fail*, naming case ids: a harness that
cannot be made to fail proves nothing.

The first three are the ones the spec names (VER04r section 13, row S6):

* :class:`NoWrap`            fixed-width arithmetic does not wrap
* :class:`EagerAnd`          ``and`` evaluates both operands (and counts the calls)
* :class:`FirstBinderFastest`  the product is enumerated with the first binder fastest

The rest come from the section 16.13 mutant table, where a lane has the seam.

Slice S10e adds the mistakes the two new levels invite (``ir.fixed`` and
``ir.tuple``): a shift count masked instead of compared, a logical ``shr`` that
sign-fills, a rotation by the width or more that returns zero, ``clz`` of zero
that is zero, ``checked_add`` that wraps, a floor division where the spec says
truncate (and the reverse), ``MIN / -1`` that wraps, a ``match`` whose last
matching arm wins, and a tuple descriptor with the first component varying
fastest.
"""

from __future__ import annotations

from collections.abc import Callable, Iterator

from coding_adventures_warrant_corpus.v2 import fixed
from coding_adventures_warrant_corpus.v2.arith import raw_binary
from coding_adventures_warrant_corpus.v2.semantics import Semantics
from coding_adventures_warrant_corpus.v2.wtypes import Ty, fixed_range


class NoWrap(Semantics):
    """Fixed-width ``+ - *`` and unary ``-`` keep the exact result."""

    def wrap(self, value: int, ty: Ty) -> int:
        return value


class Saturate(Semantics):
    """Fixed-width ``+ - *`` clamp to the type's range instead of wrapping."""

    def fixed_binary(self, op: str, left: int, right: int, ty: Ty) -> int:
        low, high = fixed_range(ty)
        return min(max(raw_binary(op, left, right), low), high)


class EagerAnd(Semantics):
    """``and`` evaluates its right operand even when the left is false."""

    def and_(self, left: Callable[[], bool], right: Callable[[], bool]) -> bool:
        a, b = left(), right()
        return a and b


class EagerIf(Semantics):
    """``if`` evaluates both branches and then picks one."""

    def if_(
        self,
        cond: Callable[[], bool],
        then: Callable[[], object],
        other: Callable[[], object],
    ) -> object:
        t, o = then(), other()
        return t if cond() else o


class FirstBinderFastest(Semantics):
    """The product is row-major with the **first** binder varying fastest."""

    def unrank(self, ordinal: int, sizes: list[int]) -> list[int]:
        digits = [0] * len(sizes)
        for position in range(len(sizes)):
            ordinal, digits[position] = divmod(ordinal, sizes[position])
        return digits

    def enumerate_indices(
        self, sizes: list[int], start: int, stop: int
    ) -> Iterator[list[int]]:
        for ordinal in range(start, stop):
            yield self.unrank(ordinal, sizes)


class SignedCompareOnBitPattern(Semantics):
    """A signed comparison made on the unsigned bit pattern."""

    def order_key(self, value: int, ty: Ty) -> int:
        return value % (1 << ty.width) if ty.is_fixed and ty.signed else value


class NegOfMinimumRaises(Semantics):
    """``-(-128i8)`` raises instead of wrapping to ``-128i8``."""

    def fixed_neg(self, value: int, ty: Ty) -> int:
        if ty.signed and value == fixed_range(ty)[0]:
            raise ArithmeticError("negation overflow")
        return super().fixed_neg(value, ty)


class NatProductUnchecked(Semantics):
    """An exact ``Nat``/``Int`` result is never checked against the 128-bit window."""

    def exact(self, value: int) -> int:
        return value


class TakeDoesNotClamp(Semantics):
    """``take`` and ``drop`` use raw slicing: a negative count slices from the end."""

    def take(self, data: bytes, count: int) -> bytes:
        return data[:count]

    def drop(self, data: bytes, count: int) -> bytes:
        return data[count:]


class SignedNotSignExtended(Semantics):
    """A signed stream value is zero-extended from one byte instead of sign-extended."""

    def encode_signed(self, value: int) -> bytes:
        return (value & 0xFF).to_bytes(8, "little")


class U32BigEndian(Semantics):
    """A ``u32`` length is written big-endian."""

    def u32(self, number: int) -> bytes:
        return number.to_bytes(4, "big")


class DecimalAcceptsLeadingZero(Semantics):
    """Integer text with a leading zero (``"01"``) is accepted."""

    def decode_int(self, text: object, ty: Ty) -> int:
        if (
            isinstance(text, str)
            and text.isdigit()
            and len(text) > 1
            and text[0] == "0"
        ):
            return super().decode_int(text.lstrip("0") or "0", ty)
        return super().decode_int(text, ty)


class SignedFxEdgeInUnsignedOrder(Semantics):
    """The withdrawn [D21] signed ``fx_edge`` set: unsigned bit patterns, unsorted."""

    def fx_edge_values(self, ty: Ty) -> list[int]:
        bits, top = ty.width, (1 << ty.width) - 1
        unsigned = [
            *range(256),
            *(1 << i for i in range(8, bits)),
            top,
            *(top ^ (1 << i) for i in range(bits)),
        ]
        return [self.wrap(v, ty) for v in unsigned]


class FirstAdmittedOnProductOrdinals(Semantics):
    """``first_admitted`` advances by ``chunk_size`` instead of by the chunk's ``cases``."""

    def advance_admitted(self, running: int, line_cases: int, chunk_size: int) -> int:
        return running + chunk_size


class ChunkIndexMissingFromPrefix(Semantics):
    """The u32 chunk index is left out of the chunk's stream prefix."""

    def chunk_prefix(self, context: bytes, chunk: int) -> bytes:
        return context


class PreRewritesSequential(Semantics):
    """A ``pre`` fault applies its rewrites one after another, each seeing the last."""

    def apply_pre(self, rewrites: list, env: dict) -> dict:
        seen, new = dict(env), {}
        for param, expr in rewrites:
            seen[param] = new[param] = expr(seen)
        return new


class ShiftCountMasked(Semantics):
    """A shift count is masked to ``w - 1`` bits (the hardware's rule) instead of compared with ``w``."""

    def shift(self, op: str, value: int, count: int, ty: Ty) -> int:
        return fixed.shift(op, value, count & (ty.width - 1), ty)


class ShrFillsTheSign(Semantics):
    """``shr`` is the arithmetic shift: a negative value keeps its sign."""

    def shift(self, op: str, value: int, count: int, ty: Ty) -> int:
        return fixed.shift("sar" if op == "shr" and ty.signed else op, value, count, ty)


class RotateByTheWidthIsZero(Semantics):
    """A rotation by ``w`` or more is treated like a shift: the count is not reduced modulo ``w``."""

    def rotate(self, op: str, value: int, count: int, ty: Ty) -> int:
        if count >= ty.width:
            return 0
        return fixed.rotate(op, value, count, ty)


class ClzOfZeroIsZero(Semantics):
    """``clz`` and ``ctz`` of the pattern 0 are 0 instead of ``w``."""

    def count_zeros(self, op: str, value: int, ty: Ty) -> int:
        return 0 if fixed.pattern(value, ty) == 0 else fixed.count_zeros(op, value, ty)


class CheckedWraps(Semantics):
    """``checked_add``/``sub``/``mul`` wrap and answer ``Some`` instead of ``None`` on overflow."""

    def checked(self, op: str, left: int, right: int, ty: Ty) -> tuple:
        return ("some", self.wrap(raw_binary(op, left, right), ty))


class DivIsTruncating(Semantics):
    """``div`` and ``rem`` on ``Int`` round toward zero (C's rule) instead of flooring."""

    def divide(self, op: str, left: int, right: int, ty: Ty | None) -> tuple:
        if op in ("div", "rem") and right != 0:
            quotient = (
                abs(left) // abs(right) * (1 if (left < 0) == (right < 0) else -1)
            )
            return ("some", quotient if op == "div" else left - right * quotient)
        return fixed.divide(op, left, right, ty)


class DivTruncIsFloor(Semantics):
    """``div_trunc`` and ``rem_trunc`` floor, so ``-7 / 2`` is ``-4``."""

    def divide(self, op: str, left: int, right: int, ty: Ty | None) -> tuple:
        if op in ("div_trunc", "rem_trunc") and right != 0:
            return fixed.divide(
                "div" if op == "div_trunc" else "rem", left, right, None
            )
        return fixed.divide(op, left, right, ty)


class DivTruncMinByMinusOneWraps(Semantics):
    """``MIN / -1`` wraps to ``MIN`` instead of being ``None``."""

    def divide(self, op: str, left: int, right: int, ty: Ty | None) -> tuple:
        if op == "div_trunc" and ty is not None and right == -1:
            return ("some", self.wrap(-left, ty))
        return fixed.divide(op, left, right, ty)


class MatchLastArmWins(Semantics):
    """A ``match`` runs the **last** arm whose pattern matches, not the first."""

    def choose_arm(self, matching: list[int]) -> int:
        return matching[-1]


class TupleFirstComponentFastest(Semantics):
    """A tuple descriptor is a product with the **first** component varying fastest."""

    def component_digits(self, index: int, sizes: list[int]) -> list[int]:
        digits = []
        for size in sizes:
            index, digit = divmod(index, size)
            digits.append(digit)
        return digits


# name -> (mutant, categories to run, case ids that must be reported)
MUTANTS: dict[str, tuple[Semantics, tuple[str, ...], tuple[str, ...]]] = {
    "no-wrap": (
        NoWrap(),
        ("v2-arith", "v2-ir", "v2-fault"),
        (
            "v2-arith/u8-add-wrap",
            "v2-ir/add-u8-wraps",
            "v2-fault/off-by-one-u8-wraps",
            "v2-ir/wrap-nat-256-into-u8",  # S10e: the ir.fixed `wrap` node reads the same seam
        ),
    ),
    "eager-and": (
        EagerAnd(),
        ("v2-ir",),
        ("v2-ir/short-circuit-and-untaken", "v2-ir/and-untaken-operand-not-evaluated"),
    ),
    "first-binder-fastest": (
        FirstBinderFastest(),
        ("v2-chunk",),
        ("v2-chunk/big-first-chunk", "v2-chunk/two-bytes-first-chunk"),
    ),
    "saturate": (
        Saturate(),
        ("v2-arith", "v2-ir"),
        ("v2-arith/u8-add-wrap", "v2-ir/add-u8-wraps"),
    ),
    "eager-if": (
        EagerIf(),
        ("v2-ir",),
        (
            "v2-ir/short-circuit-if-then-untaken",
            "v2-ir/if-untaken-branch-not-evaluated",
        ),
    ),
    "signed-compare-on-pattern": (
        SignedCompareOnBitPattern(),
        ("v2-arith", "v2-ir"),
        ("v2-ir/lt-i8-signed-order",),
    ),
    "neg-of-minimum-raises": (
        NegOfMinimumRaises(),
        ("v2-arith", "v2-ir"),
        ("v2-ir/neg-i8-min",),
    ),
    "nat-product-unchecked": (
        NatProductUnchecked(),
        ("v2-arith", "v2-ir"),
        ("v2-ir/overflow-nat-product-corpus-class", "v2-ir/int-product-exactly-2-127"),
    ),
    "take-does-not-clamp": (
        TakeDoesNotClamp(),
        ("v2-ir",),
        ("v2-ir/take-negative-count-is-zero", "v2-ir/drop-negative-count-is-zero"),
    ),
    "signed-not-sign-extended": (
        SignedNotSignExtended(),
        ("v2-value", "v2-domain"),
        ("v2-value/i8-minus-one", "v2-domain/fx-full-i8-index-0"),
    ),
    "u32-big-endian": (
        U32BigEndian(),
        ("v2-value", "v2-domain"),
        ("v2-value/bytes-one", "v2-domain/bytes-d1-index-last"),
    ),
    "decimal-leading-zero": (
        DecimalAcceptsLeadingZero(),
        ("v2-value",),
        ("v2-value/decimal-leading-zero", "v2-value/decimal-double-zero"),
    ),
    "signed-fx-edge-unsigned-order": (
        SignedFxEdgeInUnsignedOrder(),
        ("v2-domain",),
        ("v2-domain/fx-edge-i64-index-0", "v2-domain/fx-edge-i16-index-last"),
    ),
    "first-admitted-on-product-ordinals": (
        FirstAdmittedOnProductOrdinals(),
        ("v2-chunk",),
        ("v2-chunk/gap-first-chunk-is-half-admitted", "v2-chunk/ragged-first-chunk"),
    ),
    "chunk-index-missing-from-prefix": (
        ChunkIndexMissingFromPrefix(),
        ("v2-chunk",),
        ("v2-chunk/big-first-chunk", "v2-chunk/grid-last-chunk-1024"),
    ),
    "pre-rewrites-sequential": (
        PreRewritesSequential(),
        ("v2-fault",),
        ("v2-fault/pre-rewrites-read-the-original-arguments",),
    ),
    # -- slice S10e: the mistakes ir.fixed and ir.tuple invite ------------------
    "shift-count-masked": (
        ShiftCountMasked(),
        ("v2-ir",),
        ("v2-ir/shl-u8-count-equal-to-width", "v2-ir/shl-count-is-the-largest-nat"),
    ),
    "shr-fills-the-sign": (
        ShrFillsTheSign(),
        ("v2-ir",),
        ("v2-ir/shr-i8-is-logical", "v2-ir/shr-i8-minimum-by-seven"),
    ),
    "rotate-by-the-width-is-zero": (
        RotateByTheWidthIsZero(),
        ("v2-ir",),
        ("v2-ir/rotl-u8-count-equal-to-width", "v2-ir/rotl-u8-count-reduced-mod-w"),
    ),
    "clz-of-zero-is-zero": (
        ClzOfZeroIsZero(),
        ("v2-ir",),
        ("v2-ir/clz-u8-zero", "v2-ir/ctz-u8-zero"),
    ),
    "checked-wraps": (
        CheckedWraps(),
        ("v2-ir",),
        ("v2-ir/checked-add-u8-overflows", "v2-ir/checked-add-i8-past-the-maximum"),
    ),
    "div-is-truncating": (
        DivIsTruncating(),
        ("v2-ir",),
        ("v2-ir/div-int-floors", "v2-ir/rem-int-has-the-divisors-sign-positive"),
    ),
    "div-trunc-is-floor": (
        DivTruncIsFloor(),
        ("v2-ir",),
        ("v2-ir/div-trunc-i8-toward-zero",),
    ),
    "div-trunc-min-by-minus-one-wraps": (
        DivTruncMinByMinusOneWraps(),
        ("v2-ir",),
        ("v2-ir/div-trunc-i8-minimum-by-minus-one",),
    ),
    "match-last-arm-wins": (
        MatchLastArmWins(),
        ("v2-ir",),
        ("v2-ir/match-first-matching-arm-wins",),
    ),
    "tuple-first-component-fastest": (
        TupleFirstComponentFastest(),
        ("v2-domain", "v2-chunk"),
        (
            "v2-domain/tuple-u8-bool-range-first-two",
            "v2-domain/tuple-bool-u8-across-the-first-component",
            "v2-chunk/tuple-pair-binder-chunk",
        ),
    ),
}

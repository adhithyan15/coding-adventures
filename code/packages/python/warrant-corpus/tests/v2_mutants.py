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
"""

from __future__ import annotations

from collections.abc import Callable, Iterator

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


# name -> (mutant, categories to run, case ids that must be reported)
MUTANTS: dict[str, tuple[Semantics, tuple[str, ...], tuple[str, ...]]] = {
    "no-wrap": (
        NoWrap(),
        ("v2-arith", "v2-ir", "v2-fault"),
        ("v2-arith/u8-add-wrap", "v2-ir/add-u8-wraps", "v2-fault/off-by-one-u8-wraps"),
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
}

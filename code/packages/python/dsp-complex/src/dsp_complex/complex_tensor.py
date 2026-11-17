"""Complex tensor representation and manipulation for the DSP layer.

Conforms to DSP00, DSP01 Phase 2, and VER15 security/weakness matrix invariants:
- CWE-400 resource exhaustion defense via MAX_SAMPLES pre-flight checks.
- CWE-20 fail-closed finite float validation rejecting NaN/±Inf.
- CWE-674 strictly iterative execution (zero recursion).
- CWE-1333 zero regular expressions.
"""

from __future__ import annotations

import math
from collections.abc import Iterator, Sequence

# Maximum number of complex samples in a ComplexTensor.
# 2^24 = 16,777,216 complex elements (128 MiB for 2 * f32 per sample).
MAX_SAMPLES: int = 1 << 24


class ComplexError(ValueError):
    """Base exception for complex tensor validation and operations."""


class InvalidInputError(ComplexError):
    """Raised when input parameters violate finiteness, dimensions, or bounds."""


class LengthMismatchError(ComplexError):
    """Raised when real and imaginary components differ in length."""

    def __init__(self, real_len: int, imag_len: int) -> None:
        super().__init__(
            f"real array has {real_len} elements, imag has {imag_len}; must match"
        )
        self.real_len = real_len
        self.imag_len = imag_len


class OddInterleavedLengthError(ComplexError):
    """Raised when an interleaved buffer has an odd length."""

    def __init__(self, length: int) -> None:
        super().__init__(
            f"interleaved buffer has odd length {length}; must be even"
        )
        self.length = length


class ComplexTensor:
    """Complex-valued signal or spectrum in interleaved [re, im] float format."""

    __slots__ = ("_interleaved",)

    def __init__(self, interleaved: Sequence[float] | None = None) -> None:
        """Construct a ComplexTensor from an interleaved [re_0, im_0, ...] buffer.

        Validates that the buffer length is even, within MAX_SAMPLES, and that
        every float is finite before storing.
        """
        if interleaved is None:
            self._interleaved: list[float] = []
            return

        raw_len = len(interleaved)
        if raw_len > 2 * MAX_SAMPLES:
            msg = (
                f"interleaved element count {raw_len // 2} exceeds "
                f"MAX_SAMPLES ({MAX_SAMPLES})"
            )
            raise InvalidInputError(msg)
        if raw_len % 2 != 0:
            raise OddInterleavedLengthError(raw_len)

        # Fail-closed finite float validation before retaining data
        buf: list[float] = []
        for x in interleaved:
            val = float(x)
            if not math.isfinite(val):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )
            buf.append(val)

        self._interleaved = buf

    @classmethod
    def try_from_real(cls, real: Sequence[float]) -> ComplexTensor:
        """Construct from a real-only sequence with zero-filled imaginary parts."""
        n = len(real)
        if n > MAX_SAMPLES:
            raise InvalidInputError(
                f"real slice length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
            )

        buf: list[float] = []
        for x in real:
            val = float(x)
            if not math.isfinite(val):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )
            buf.append(val)
            buf.append(0.0)

        tensor = cls.__new__(cls)
        tensor._interleaved = buf
        return tensor

    @classmethod
    def from_real(cls, real: Sequence[float]) -> ComplexTensor:
        """Construct from a real-only sequence. Alias for try_from_real."""
        return cls.try_from_real(real)

    @classmethod
    def from_real_imag(
        cls, real: Sequence[float], imag: Sequence[float]
    ) -> ComplexTensor:
        """Construct from separate real and imaginary sequences."""
        if len(real) != len(imag):
            raise LengthMismatchError(len(real), len(imag))

        n = len(real)
        if n > MAX_SAMPLES:
            raise InvalidInputError(
                f"array length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
            )

        buf: list[float] = []
        for r, i in zip(real, imag, strict=True):
            r_val = float(r)
            i_val = float(i)
            if not math.isfinite(r_val) or not math.isfinite(i_val):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )
            buf.append(r_val)
            buf.append(i_val)

        tensor = cls.__new__(cls)
        tensor._interleaved = buf
        return tensor

    @classmethod
    def from_interleaved(cls, data: Sequence[float]) -> ComplexTensor:
        """Construct directly from an interleaved buffer."""
        return cls(data)

    @classmethod
    def from_complex_list(cls, complexes: Sequence[complex]) -> ComplexTensor:
        """Construct from a sequence of native Python complex numbers."""
        n = len(complexes)
        if n > MAX_SAMPLES:
            raise InvalidInputError(
                f"complex sequence length {n} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
            )

        buf: list[float] = []
        for c in complexes:
            r_val = float(c.real)
            i_val = float(c.imag)
            if not math.isfinite(r_val) or not math.isfinite(i_val):
                raise InvalidInputError(
                    "input contains non-finite float (NaN or Inf)"
                )
            buf.append(r_val)
            buf.append(i_val)

        tensor = cls.__new__(cls)
        tensor._interleaved = buf
        return tensor

    def __len__(self) -> int:
        """Number of complex elements (half the interleaved length)."""
        return len(self._interleaved) // 2

    def is_empty(self) -> bool:
        """Whether the tensor has zero complex elements."""
        return len(self._interleaved) == 0

    def as_interleaved(self) -> list[float]:
        """Return a copy of the interleaved buffer."""
        return list(self._interleaved)

    def into_interleaved(self) -> list[float]:
        """Return the underlying interleaved list."""
        return list(self._interleaved)

    def real(self) -> list[float]:
        """Real components [re_0, re_1, ..., re_{N-1}]."""
        n = len(self)
        return [self._interleaved[2 * i] for i in range(n)]

    def imag(self) -> list[float]:
        """Imaginary components [im_0, im_1, ..., im_{N-1}]."""
        n = len(self)
        return [self._interleaved[2 * i + 1] for i in range(n)]

    def magnitude(self) -> list[float]:
        """Element-wise magnitude sqrt(re^2 + im^2)."""
        n = len(self)
        return [
            math.hypot(self._interleaved[2 * i], self._interleaved[2 * i + 1])
            for i in range(n)
        ]

    def phase(self) -> list[float]:
        """Element-wise phase angle in radians atan2(im, re)."""
        n = len(self)
        return [
            math.atan2(self._interleaved[2 * i + 1], self._interleaved[2 * i])
            for i in range(n)
        ]

    def conjugate(self) -> ComplexTensor:
        """Element-wise complex conjugate: (re, im) -> (re, -im)."""
        out = list(self._interleaved)
        for i in range(1, len(out), 2):
            out[i] = -out[i]
        tensor = ComplexTensor.__new__(ComplexTensor)
        tensor._interleaved = out
        return tensor

    def to_complex_list(self) -> list[complex]:
        """Convert to a list of standard Python complex numbers."""
        n = len(self)
        return [
            complex(self._interleaved[2 * i], self._interleaved[2 * i + 1])
            for i in range(n)
        ]

    @property
    def shape(self) -> tuple[int, int]:
        """Shape tuple matching [num_complex, 2]."""
        return (len(self), 2)

    def matrix_shape(self) -> tuple[int, int]:
        """Shape matching matrix-ir convention: (len, 2)."""
        return (len(self), 2)

    def matrix_dtype(self) -> str:
        """Dtype matching matrix-ir convention: 'F32'."""
        return "F32"

    def __getitem__(self, idx: int) -> tuple[float, float]:
        """Return (real, imag) tuple at index idx."""
        n = len(self)
        if idx < 0:
            idx += n
        if idx < 0 or idx >= n:
            raise IndexError("ComplexTensor index out of range")
        return (self._interleaved[2 * idx], self._interleaved[2 * idx + 1])

    def __iter__(self) -> Iterator[tuple[float, float]]:
        """Iterate over (real, imag) tuples."""
        n = len(self)
        for i in range(n):
            yield (self._interleaved[2 * i], self._interleaved[2 * i + 1])

    def __eq__(self, other: object) -> bool:
        """Equality comparison based on interleaved values."""
        if not isinstance(other, ComplexTensor):
            return False
        return self._interleaved == other._interleaved

    def __repr__(self) -> str:
        """Abbreviated debug representation."""
        n = len(self)
        head_items = [
            (self._interleaved[2 * i], self._interleaved[2 * i + 1])
            for i in range(min(n, 4))
        ]
        return f"ComplexTensor(len={n}, head={head_items})"

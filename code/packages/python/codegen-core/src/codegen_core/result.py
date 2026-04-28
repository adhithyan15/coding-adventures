"""CodegenResult — the output of one CodegenPipeline.compile_with_stats() call.

Every call to ``CodegenPipeline.compile_with_stats()`` returns a
``CodegenResult[IR]`` that bundles the native binary with enough metadata
to drive diagnostics, profiling, and cache-key decisions:

- ``binary`` — the opaque native bytes (or ``None`` if compilation failed).
- ``ir`` — the post-optimization IR snapshot, useful for IR dumps and tests.
- ``backend_name`` — identifies which backend produced the binary.
- ``compilation_time_ns`` — wall-clock nanoseconds from IR-in to binary-out.
- ``optimizer_applied`` — whether an optimizer pass ran.

Typical uses
------------
**JIT cache entry** — ``JITCacheEntry`` wraps a ``CodegenResult`` so the
cache stores both the binary and its provenance metadata.

**Diagnostics** — ``codegen_result.ir`` lets tooling print the
post-optimization IR without needing to re-run the optimizer.

**Testing** — ``assert result.success`` is a clear assertion that
compilation did not silently return ``None``.

>>> from dataclasses import asdict
>>> r = CodegenResult(  # doctest: +SKIP
...     binary=b"\\x01", ir=[1, 2], backend_name="test", compilation_time_ns=100
... )
>>> r.success
True
>>> r2 = CodegenResult(binary=None, ir=[], backend_name="test", compilation_time_ns=0)
>>> r2.success
False
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TypeVar

IR = TypeVar("IR")


@dataclass
class CodegenResult[IR]:
    """Result of one ``CodegenPipeline.compile_with_stats()`` call.

    Type parameters
    ---------------
    IR
        The IR type that was compiled.  For the JIT/AOT path this is
        ``list[CIRInstr]``; for the compiled-language path it is
        ``IrProgram``.

    Attributes
    ----------
    binary:
        Opaque native bytes produced by the backend, or ``None`` if the
        backend declined to compile the IR.
    ir:
        Post-optimization IR snapshot.  Set to the optimizer output so
        that callers can inspect or dump what went into the backend.
    backend_name:
        Short identifier of the backend that produced the binary
        (``Backend.name``).
    compilation_time_ns:
        Wall-clock nanoseconds from IR-in to binary-out.  Includes both
        the optimizer and the backend compile step.
    optimizer_applied:
        ``True`` when an ``Optimizer`` was present in the pipeline and
        ran at least one pass.  ``False`` when the pipeline was built
        without an optimizer.
    """

    binary: bytes | None
    ir: IR
    backend_name: str
    compilation_time_ns: int = 0
    optimizer_applied: bool = False

    # ------------------------------------------------------------------
    # Derived properties
    # ------------------------------------------------------------------

    @property
    def success(self) -> bool:
        """True when the backend produced a non-None binary."""
        return self.binary is not None

    @property
    def binary_size(self) -> int:
        """Byte length of the compiled binary, or 0 if compilation failed."""
        return len(self.binary) if self.binary is not None else 0

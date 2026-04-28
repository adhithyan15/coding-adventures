"""AOTStats — compilation statistics returned by AOTCore.stats().

After calling ``AOTCore.compile()``, the stats snapshot captures:

- How many functions were fully compiled to native code.
- How many functions remained untyped and were routed through the vm-runtime
  stub path (serialized into the IIR table section of the .aot binary).
- Total wall-clock compilation time in nanoseconds.
- The combined size of all native code bytes produced by the backend.
- The optimization level that was active during compilation.

The stats object is a snapshot: it reflects the state at the time
``AOTCore.stats()`` was called and is not updated retroactively.

Example
-------
>>> aot = AOTCore(backend=my_backend)
>>> aot.compile(module)
>>> s = aot.stats()
>>> print(s.functions_compiled, s.functions_untyped)
3 1
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class AOTStats:
    """Snapshot of AOT compilation statistics.

    Attributes
    ----------
    functions_compiled:
        Number of functions successfully compiled to native binary by the backend.
    functions_untyped:
        Number of functions that could not be fully specialized (remained "any"
        after static inference) and were routed to the vm-runtime stub path.
    compilation_time_ns:
        Total wall-clock time spent in the AOT pipeline (infer + specialise +
        optimize + backend.compile) across all functions, in nanoseconds.
    total_binary_size:
        Combined byte length of all native binaries produced by the backend.
    optimization_level:
        The optimization level that was active: 0=none, 1=basic (fold+DCE),
        2=full (fold+DCE + AOT-specific passes).
    """

    functions_compiled: int = field(default=0)
    functions_untyped: int = field(default=0)
    compilation_time_ns: int = field(default=0)
    total_binary_size: int = field(default=0)
    optimization_level: int = field(default=2)

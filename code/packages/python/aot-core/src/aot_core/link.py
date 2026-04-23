"""Linker — concatenate per-function native binaries into a single code section.

The linker takes a sequence of ``(fn_name, binary)`` pairs (one per compiled
function) and concatenates them.  It records the byte offset at which each
function's code begins, producing a *function offset table*.

Why a separate linker step?
---------------------------
The backend compiles one function at a time and produces independent byte
blobs.  Before writing the ``.aot`` snapshot the blobs must be joined into a
single code section, and the entry-point offset (the offset of ``"main"``) must
be determined.  The linker handles both concerns.

Cross-compilation note
----------------------
The linker is architecture-neutral: it performs no fixups or relocation.
The backend is responsible for producing position-independent code (or
relative-addressing code) so that concatenating blobs does not require
relocation table entries.  Simulators (Intel 4004, RISC-V, etc.) load the
entire code section at a fixed base address, so PI code is not needed for
simulation targets.

Example
-------
>>> binaries = [("main", b"\\x01\\x02"), ("helper", b"\\x03\\x04\\x05")]
>>> code, offsets = link(binaries)
>>> code
b'\\x01\\x02\\x03\\x04\\x05'
>>> offsets["main"]
0
>>> offsets["helper"]
2
"""

from __future__ import annotations


def link(fn_binaries: list[tuple[str, bytes]]) -> tuple[bytes, dict[str, int]]:
    """Concatenate per-function binaries into a single code section.

    Parameters
    ----------
    fn_binaries:
        Ordered list of ``(function_name, native_binary)`` pairs.  Order
        determines the layout of the code section.

    Returns
    -------
    tuple[bytes, dict[str, int]]
        ``(combined_code, offsets)`` where ``offsets`` maps each function name
        to its byte offset within ``combined_code``.
    """
    combined = b""
    offsets: dict[str, int] = {}
    for name, binary in fn_binaries:
        offsets[name] = len(combined)
        combined += binary
    return combined, offsets


def entry_point_offset(offsets: dict[str, int], entry: str = "main") -> int:
    """Return the byte offset of the entry-point function.

    Parameters
    ----------
    offsets:
        Offset table returned by ``link()``.
    entry:
        Name of the entry-point function (default: ``"main"``).

    Returns
    -------
    int
        Byte offset of the entry-point within the code section, or ``0`` if
        the function was not found (e.g., the module has no ``"main"``).
    """
    return offsets.get(entry, 0)

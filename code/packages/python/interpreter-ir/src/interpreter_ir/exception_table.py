"""ExceptionTableEntry — static exception handler record for VMCOND00 Layer 2.

A function's exception table is a flat list of ``ExceptionTableEntry`` objects
that the THROW handler walks at runtime to find a matching catch clause.  Each
entry describes one guarded range and the handler that covers it.

Design rationale
----------------
The exception table lives on ``IIRFunction`` (per-function), not on
``IIRModule`` (module-wide).  This matches the JVM (per-method attribute_info)
and CPython (per-code-object co_exceptiontable in 3.11+).  Per-function
placement means:

- The VM only loads the exception table when it pushes a frame for that
  function — cold functions pay no loading cost.
- The THROW handler only walks entries for the function it is currently
  searching; it never scans entries belonging to other functions.
- Frontends can easily attach exactly the entries they generate for a function
  in a single compilation step.

Entry format
------------
All IPs (``from_ip``, ``to_ip``, ``handler_ip``) are zero-based instruction
indices within the *same* function's instruction list — they are NOT absolute
module-wide byte offsets.  This is the same coordinate system used by
``IIRFunction.label_index()``.

  from_ip   — *inclusive* start of the guarded range.  A THROW whose
              IP equals ``from_ip`` is covered by this entry.

  to_ip     — *exclusive* end of the guarded range.  A THROW whose IP
              equals ``to_ip`` is NOT covered.  The half-open ``[from_ip,
              to_ip)`` convention matches Python ``range``, Java's
              exception table, and virtually every other VM — it avoids
              off-by-one errors when ranges are adjacent.

  handler_ip — the instruction index the VM should jump to when this entry
               fires.  Typically the index of a ``label`` instruction that
               marks the beginning of the catch block.

  type_id   — the condition type this entry handles.  The sentinel value
              ``"*"`` means *catch all* — equivalent to a bare ``except``
              in Python or ``catch (Throwable t)`` in Java.  In Phase 2,
              all matching is exact string equality (no subtype hierarchy).
              Phase 3 will introduce the condition type hierarchy and
              replace exact-string matching with ``is_subtype`` calls.

  val_reg   — the name of the register that receives the caught condition
              object.  The THROW handler writes the thrown value into this
              register before jumping to ``handler_ip``.  Choose a name
              that does not collide with any register written in the guarded
              range (this is a frontend responsibility; the VM does not verify).

Type matching (Phase 2 semantics)
----------------------------------
Two values are checked in order for each entry:

    1. ``entry.type_id == "*"``   → catch-all; always matches.
    2. ``str(type(condition).__name__) == entry.type_id`` — matches by the
       Python class name of the condition object.  This keeps Phase 2 simple
       while allowing ``type(x).__name__`` as the "type tag" for Python objects
       used as conditions (e.g. ``ValueError``, ``MyCustomCondition``).

Phase 3 will replace rule 2 with a walk of the condition type hierarchy
maintained in the condition-type table.

Thread safety
-------------
``ExceptionTableEntry`` is immutable after construction.  All fields are set
at compile time.  The THROW handler reads them without locking — safe because
vm-core is not thread-safe anyway (one VM per thread).
"""

from __future__ import annotations

from dataclasses import dataclass

#: Sentinel ``type_id`` value that matches any thrown condition.  Equivalent
#: to a bare ``except`` clause in Python or ``catch (Exception e)`` in Java.
CATCH_ALL: str = "*"


@dataclass(frozen=True)
class ExceptionTableEntry:
    """One entry in a function's static exception table.

    Parameters
    ----------
    from_ip:
        Inclusive start of the guarded range (IIR instruction index).
        The entry covers throws at IPs ``[from_ip, to_ip)``.
    to_ip:
        Exclusive end of the guarded range.  A throw at ``to_ip`` is
        NOT covered by this entry.
    handler_ip:
        Instruction index to jump to when this entry fires.  Normally the
        index of a ``label`` instruction at the start of the catch block.
    type_id:
        Condition type string to match.  Use ``"*"`` (``CATCH_ALL``) for a
        catch-all handler.  In Phase 2, non-wildcard entries match when
        ``type(condition).__name__ == type_id``.
    val_reg:
        Name of the register that receives the caught condition object.
        Written by the THROW handler before jumping to ``handler_ip``.

    Examples
    --------
    Catch everything thrown between instructions 2 and 5, jump to
    instruction 8, place the condition in register ``"ex"``::

        entry = ExceptionTableEntry(
            from_ip=2, to_ip=5, handler_ip=8,
            type_id=CATCH_ALL, val_reg="ex",
        )

    Catch only ``ValueError`` conditions::

        entry = ExceptionTableEntry(
            from_ip=0, to_ip=10, handler_ip=12,
            type_id="ValueError", val_reg="err",
        )
    """

    from_ip: int
    """Inclusive start of the guarded range (instruction index)."""

    to_ip: int
    """Exclusive end of the guarded range (instruction index)."""

    handler_ip: int
    """Instruction index of the catch-block entry point."""

    type_id: str
    """Condition type to catch; ``"*"`` matches any condition."""

    val_reg: str
    """Register name that receives the caught condition object."""

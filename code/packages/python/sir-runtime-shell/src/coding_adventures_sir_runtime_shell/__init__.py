"""coding-adventures-sir-runtime-shell — the SIR ``backtick`` builtin for Python.

The Ruby→SIR frontend lowers a backtick literal ``` `cmd` ``` to
``BuiltinCall("backtick", [StrLit cmd])``.  This package is the Python runtime
for that builtin: it runs the command through the system shell and returns the
command's captured standard output as a ``str`` — Ruby ``` `cmd` ``` semantics::

    from coding_adventures_sir_runtime_shell import backtick
    backtick('python -c "print(123)"')   # "123\\n"

**Shell semantics.**  Like Ruby's backtick, the command is handed to the system
shell (``/bin/sh -c`` on POSIX, ``cmd.exe /c`` on Windows), so shell features —
pipes, redirections, globbing, ``$VAR`` expansion — work.  The child's exit
status is ignored: a non-zero exit still returns whatever stdout was produced.

**Author-supplied command (SECURITY).**  ``command`` is the string literal the
programmer wrote inside the backticks of their *own* Ruby source, threaded
verbatim through the compiler into emitted Python.  It carries the same trust
level it had in Ruby — the author's own code — and this package interpolates no
external/untrusted runtime input.  The ``shell=True`` used internally is the
intentional, faithful implementation of Ruby backtick; see
:mod:`coding_adventures_sir_runtime_shell.shell` for the full SECURITY note.

See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from .shell import Val, backtick

__all__ = [
    "Val",
    "backtick",
]

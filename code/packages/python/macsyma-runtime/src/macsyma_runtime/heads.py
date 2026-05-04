"""IR head symbols introduced by macsyma-runtime.

These heads are Maxima-specific: they don't have meaningful analogues
in Mathematica or Maple's evaluation models, so they live in the
runtime layer rather than in :mod:`symbolic_ir`. The runtime registers
handlers for each one on :class:`MacsymaBackend`.
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

# Statement-terminator wrappers. The compiler emits one of these around
# each top-level statement based on whether the source ended with `;`
# (display) or `$` (suppress). The REPL inspects the wrapper type
# *before* eval to decide whether to print the result; the VM unwraps
# them in the handler so downstream code never has to think about them.
DISPLAY = IRSymbol("Display")
SUPPRESS = IRSymbol("Suppress")

# Bookkeeping operations.
KILL = IRSymbol("Kill")
EV = IRSymbol("Ev")
BLOCK = IRSymbol("Block")  # Phase G — handled by symbolic-vm's block_ handler.
ASSUME = IRSymbol("Assume")
FORGET = IRSymbol("Forget")
IS = IRSymbol("Is")

# A sentinel symbol that ``kill(all)`` matches.
ALL_SYMBOL = IRSymbol("all")

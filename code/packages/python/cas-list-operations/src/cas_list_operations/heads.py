"""IR head sentinels for list operations.

Backends (e.g., :class:`macsyma_runtime.MacsymaBackend`) install
handlers against these names. The pure-Python helpers in this package
work directly on raw lists without needing backend integration, which
is convenient for tests and for consumers that want to use list
operations programmatically.
"""

from __future__ import annotations

from symbolic_ir import IRSymbol

LENGTH = IRSymbol("Length")
FIRST = IRSymbol("First")
REST = IRSymbol("Rest")
LAST = IRSymbol("Last")
APPEND = IRSymbol("Append")
REVERSE = IRSymbol("Reverse")
RANGE = IRSymbol("Range")
MAP = IRSymbol("Map")
APPLY = IRSymbol("Apply")
SELECT = IRSymbol("Select")
SORT = IRSymbol("Sort")
PART = IRSymbol("Part")
FLATTEN = IRSymbol("Flatten")
JOIN = IRSymbol("Join")

# Re-exported convenience.
LIST = IRSymbol("List")

"""List manipulation primitives for the symbolic IR.

Quick start::

    from cas_list_operations import (
        length, first, rest, range_, map_, apply_,
    )
    from symbolic_ir import IRApply, IRInteger, ADD, IRSymbol
    from cas_list_operations import LIST

    lst = IRApply(LIST, (IRInteger(1), IRInteger(2), IRInteger(3)))
    length(lst)            # IRInteger(3)
    first(lst)             # IRInteger(1)
    range_(5)              # [1, 2, 3, 4, 5]
    apply_(ADD, lst)       # Add(1, 2, 3)  (un-simplified)
"""

from cas_list_operations.heads import (
    APPEND,
    APPLY,
    FIRST,
    FLATTEN,
    JOIN,
    LAST,
    LENGTH,
    LIST,
    MAP,
    PART,
    RANGE,
    REST,
    REVERSE,
    SELECT,
    SORT,
)
from cas_list_operations.operations import (
    ListOperationError,
    append,
    apply_,
    first,
    flatten,
    join,
    last,
    length,
    map_,
    part,
    range_,
    rest,
    reverse,
    select,
    sort_,
)

__all__ = [
    "APPEND",
    "APPLY",
    "FIRST",
    "FLATTEN",
    "JOIN",
    "LAST",
    "LENGTH",
    "LIST",
    "ListOperationError",
    "MAP",
    "PART",
    "RANGE",
    "REST",
    "REVERSE",
    "SELECT",
    "SORT",
    "append",
    "apply_",
    "first",
    "flatten",
    "join",
    "last",
    "length",
    "map_",
    "part",
    "range_",
    "rest",
    "reverse",
    "select",
    "sort_",
]

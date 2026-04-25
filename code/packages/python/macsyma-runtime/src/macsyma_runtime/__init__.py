"""MACSYMA-specific runtime layer.

Public API::

    from macsyma_runtime import (
        MacsymaBackend,
        History,
        DISPLAY,
        SUPPRESS,
        KILL,
        EV,
        MACSYMA_NAME_TABLE,
        extend_compiler_name_table,
    )

The thin shell that turns the language-neutral ``symbolic-vm`` into a
Maxima-flavored evaluator. See ``code/specs/macsyma-runtime.md``.
"""

from macsyma_runtime.backend import MacsymaBackend
from macsyma_runtime.heads import (
    ALL_SYMBOL,
    ASSUME,
    BLOCK,
    DISPLAY,
    EV,
    FORGET,
    IS,
    KILL,
    SUPPRESS,
)
from macsyma_runtime.history import History
from macsyma_runtime.name_table import (
    MACSYMA_NAME_TABLE,
    extend_compiler_name_table,
)

__all__ = [
    "ALL_SYMBOL",
    "ASSUME",
    "BLOCK",
    "DISPLAY",
    "EV",
    "FORGET",
    "History",
    "IS",
    "KILL",
    "MACSYMA_NAME_TABLE",
    "MacsymaBackend",
    "SUPPRESS",
    "extend_compiler_name_table",
]

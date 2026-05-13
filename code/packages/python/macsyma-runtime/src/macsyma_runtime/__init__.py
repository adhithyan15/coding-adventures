"""MACSYMA-specific runtime layer.

Public API::

    from macsyma_runtime import (
        MacsymaBackend,
        History,
        DISPLAY,
        SUPPRESS,
        KILL,
        EV,
        DECLARE,
        PROPERTIES,
        PROP_VARS,
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
    DECLARE,
    DISPLAY,
    EV,
    FORGET,
    IS,
    KILL,
    PROP_VARS,
    PROPERTIES,
    SUPPRESS,
)
from macsyma_runtime.history import History
from macsyma_runtime.help import help_text, parse_help_query
from macsyma_runtime.name_table import (
    MACSYMA_NAME_TABLE,
    extend_compiler_name_table,
)
from macsyma_runtime.presentation import has_ev_flag, output_text_for

__all__ = [
    "ALL_SYMBOL",
    "ASSUME",
    "BLOCK",
    "DECLARE",
    "DISPLAY",
    "EV",
    "FORGET",
    "History",
    "IS",
    "KILL",
    "MACSYMA_NAME_TABLE",
    "MacsymaBackend",
    "PROPERTIES",
    "PROP_VARS",
    "SUPPRESS",
    "extend_compiler_name_table",
    "has_ev_flag",
    "help_text",
    "output_text_for",
    "parse_help_query",
]

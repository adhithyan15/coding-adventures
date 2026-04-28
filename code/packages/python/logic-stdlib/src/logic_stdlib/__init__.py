"""Relational standard library helpers built on top of ``logic-engine``.

The core and engine layers are intentionally small. That keeps the execution
model crisp, but it means users quickly end up rewriting the same recursive
list relations in every example. This package starts factoring that vocabulary
out into reusable host-language helpers.
"""

from logic_stdlib.relations import (
    appendo,
    conso,
    emptyo,
    heado,
    lasto,
    lengtho,
    listo,
    membero,
    msorto,
    nth0o,
    nth1o,
    permuteo,
    reverseo,
    selecto,
    sorto,
    subsequenceo,
    tailo,
)

__all__ = [
    "__version__",
    "appendo",
    "conso",
    "emptyo",
    "heado",
    "lasto",
    "lengtho",
    "listo",
    "membero",
    "msorto",
    "nth0o",
    "nth1o",
    "permuteo",
    "reverseo",
    "selecto",
    "sorto",
    "subsequenceo",
    "tailo",
]

__version__ = "0.7.0"

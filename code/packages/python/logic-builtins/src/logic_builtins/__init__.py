"""logic-builtins — Prolog-inspired control and term predicates.

The functions in this package return ordinary `logic-engine` goals. They are
the first state-aware builtins layer for the library-first Prolog path.
"""

from logic_builtins.builtins import (
    argo,
    atomo,
    callo,
    compoundo,
    functoro,
    groundo,
    nonvaro,
    noto,
    numbero,
    onceo,
    stringo,
    varo,
)

__all__ = [
    "__version__",
    "argo",
    "atomo",
    "callo",
    "compoundo",
    "functoro",
    "groundo",
    "nonvaro",
    "noto",
    "numbero",
    "onceo",
    "stringo",
    "varo",
]

__version__ = "0.1.0"

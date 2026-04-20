"""logic-builtins — Prolog-inspired control and term predicates.

The functions in this package return ordinary `logic-engine` goals. They are
the first state-aware builtins layer for the library-first Prolog path.
"""

from logic_builtins.builtins import (
    add,
    argo,
    atomo,
    callo,
    compoundo,
    div,
    floordiv,
    functoro,
    geqo,
    groundo,
    gto,
    iso,
    leqo,
    lto,
    mod,
    mul,
    neg,
    nonvaro,
    noto,
    numbero,
    numeqo,
    numneqo,
    onceo,
    stringo,
    sub,
    varo,
)

__all__ = [
    "__version__",
    "add",
    "argo",
    "atomo",
    "callo",
    "compoundo",
    "div",
    "floordiv",
    "functoro",
    "geqo",
    "gto",
    "groundo",
    "iso",
    "leqo",
    "lto",
    "mod",
    "mul",
    "neg",
    "nonvaro",
    "noto",
    "numeqo",
    "numneqo",
    "numbero",
    "onceo",
    "stringo",
    "sub",
    "varo",
]

__version__ = "0.2.0"

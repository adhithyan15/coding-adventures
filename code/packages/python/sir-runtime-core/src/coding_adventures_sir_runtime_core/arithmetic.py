"""SIR arithmetic and comparison.

These differ from Python's native operators in two ways that justify a
runtime helper rather than a bare ``a + b``:

1. **Variadic** — ``add`` / ``sub`` / ``mul`` fold over any number of
   arguments (``add()`` is ``0``, ``mul()`` is ``1``), matching the SIR
   builtin contract shared with the Lisp/Twig frontends.
2. **Truncating integer division** — ``div`` truncates toward zero
   (``div(7, 2) == 3``, ``div(-7, 2) == -3``) to match SIR semantics,
   where Python's ``/`` would yield a float.

Comparisons (``lt`` / ``gt``) are thin wrappers kept here so the dispatch
table can expose them by SIR name.
"""

from __future__ import annotations

from typing import Any

from .values import to_display


def add(*args: Any) -> Any:
    """Variadic sum; ``add()`` is ``0``.

    Ruby's ``+`` is **polymorphic on the receiver type**, and because a SIR
    array is a plain Python ``list`` and a SIR string is a ``str``, Python's
    own ``+`` already gives us all three Ruby behaviours *for free* — no
    explicit ``isinstance`` dispatch is needed here:

    | Operands            | ``+`` result  | Ruby         |
    |---------------------|---------------|--------------|
    | ints/floats         | numeric sum   | ``1 + 2``    |
    | ``str`` + ``str``   | concatenation | ``"a"+"b"``  |
    | ``list`` + ``list`` | concatenation | ``[1]+[2]``  |

    The one subtlety: we **seed the fold with the first operand** rather than
    with the integer ``0``. Seeding from ``0`` (``total = 0; total += a``) only
    works for numbers — ``0 + "a"`` / ``0 + [1]`` raise ``TypeError`` — so the
    string/array cases would break. Seeding from ``args[0]`` starts the fold in
    the right type; for the numeric case the result is identical because
    addition is associative (``0 + a + b == a + b``). ``list`` concatenation
    with ``+`` produces a *fresh* list (no aliasing), matching Ruby's
    non-destructive ``Array#+``. The empty-sum identity ``add() == 0`` is kept.
    """
    if not args:
        return 0
    total: Any = args[0]
    for a in args[1:]:
        total = total + a
    return total


def sub(*args: Any) -> Any:
    """Variadic difference; ``sub(x)`` negates, ``sub()`` is ``0``."""
    if not args:
        return 0
    if len(args) == 1:
        return -args[0]
    acc = args[0]
    for a in args[1:]:
        acc -= a
    return acc


def mul(*args: Any) -> Any:
    """Variadic product; ``mul()`` is ``1``.

    Ruby's ``*`` is **polymorphic on the receiver type**, and unlike ``+`` the
    native Python ``*`` does *not* line up with Ruby for every case, so we
    dispatch explicitly on the runtime tag (``isinstance``, never reflection —
    see the dynamic-dispatch RCE lesson). Ruby's four arms:

    | Receiver × operand | Ruby result           | Example              |
    |--------------------|-----------------------|----------------------|
    | ``str`` × ``int``  | repeated string        | ``"ab"*3 → "ababab"``|
    | ``list`` × ``int`` | repeated-element list  | ``[0]*3 → [0,0,0]``  |
    | ``list`` × ``str`` | join elements → string | ``[1,2]*", " → "1, 2"``|
    | numbers            | numeric product        | ``2*3 → 6``          |

    ``*`` is **binary** in Ruby, so the string/array arms handle exactly the
    two-operand shape the frontend lowers. The pure-numeric case keeps the
    variadic fold (``mul() == 1``, ``mul(2, 3, 4) == 24``) that SIR's builtin
    contract shares with the Lisp/Twig frontends.

    Notes on the arms:

    * ``str`` × ``int`` with ``int <= 0`` → ``""`` — Python's own ``"ab" * 0``
      and ``"ab" * -1`` already yield ``""``, matching Ruby's ``String#*``.
    * ``list`` × ``int`` uses Python's ``list * int`` (fresh list, non-negative
      count clamps to empty), matching Ruby's ``Array#*`` with an Integer.
    * ``list`` × ``str`` joins with :func:`to_display` — the runtime's canonical
      element display — so ``[1, 2] * ", "`` renders as ``"1, 2"`` (element
      ``to_s``, not Python's ``repr``), matching Ruby's ``Array#*`` with a
      String separator.
    """
    # --- string/array arms (Ruby's binary `*`) -----------------------------
    # These only apply to the two-operand shape the frontend emits for `*`.
    if len(args) == 2:
        left, right = args
        # bool is an int subclass in Python but is NOT a SIR count/number
        # (see values.is_number); exclude it so `"ab" * True` is not treated
        # as a repeat with count 1.
        right_is_int = isinstance(right, int) and not isinstance(right, bool)

        if isinstance(left, str) and right_is_int:
            # "ab" * 3 -> "ababab"; count <= 0 -> "" (Python matches Ruby).
            return left * right
        if isinstance(left, list) and right_is_int:
            # [0] * 3 -> [0, 0, 0]; fresh list, count <= 0 -> [].
            return left * right
        if isinstance(left, list) and isinstance(right, str):
            # [1, 2] * ", " -> "1, 2" (element to_s, canonical SIR display).
            return right.join(to_display(el) for el in left)

    # --- numeric variadic fold (unchanged) ---------------------------------
    acc: Any = 1
    for a in args:
        acc *= a
    return acc


def div(*args: Any) -> Any:
    """Variadic quotient with **truncating integer** division (toward
    zero), matching SIR semantics rather than Python's float ``/``."""
    if not args:
        return 0
    acc = args[0]
    for a in args[1:]:
        acc = int(acc / a)
    return acc


def lt(a: Any, b: Any) -> bool:
    """Less-than."""
    return bool(a < b)


def gt(a: Any, b: Any) -> bool:
    """Greater-than."""
    return bool(a > b)

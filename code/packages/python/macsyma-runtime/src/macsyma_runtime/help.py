"""Small MACSYMA help catalog used by REPL/session frontends."""

from __future__ import annotations

_TOPICS: dict[str, str] = {
    "arithmetic": "Arithmetic: use +, -, *, /, and ^. Example: expand((x + 1)^2);",
    "calculus": "Calculus: diff(expr, var), integrate(expr, var), limit(expr, var, point), and taylor(expr, var, point, order).",
    "diff": "diff(expr, var) differentiates expr with respect to var. Example: diff(x^3, x);",
    "integrate": "integrate(expr, var) computes an antiderivative when supported. Example: integrate(x^2, x);",
    "solve": "solve(expr, var) solves equations or supported inequalities. Use linsolve([...], [...]) for linear systems and nsolve(poly, var) for numeric polynomial roots.",
    "matrix": "Matrix tools: matrix([...], ...), transpose, determinant, invert, dot, rank, rowreduce, ident, zeromatrix, and matrix_size.",
    "lists": "List tools: length, first, rest, last, append, reverse, range, map, apply, sublist, sort, part, flatten, join, and makelist.",
    "assumptions": "Assumptions: assume(x > 0), declare(x, positive), is(x > 0), forget(), properties(x), and propvars().",
    "properties": "properties(symbol) lists declared properties. propvars() lists symbols with declared properties.",
    "display": "Display: terminate with ; to show output and $ to suppress it. ev(expr, display2d) renders 2D output.",
    "history": "History: % is the last output; %iN and %oN refer to input and output number N.",
    "showtime": "showtime:true enables per-expression timing; showtime:false disables it.",
    "repl": "REPL commands: :quit exits. Use --file path.mac for batch execution.",
}

_ALIASES = {
    "d": "diff",
    "derivative": "diff",
    "integral": "integrate",
    "matrices": "matrix",
    "list": "lists",
    "assume": "assumptions",
    "declare": "assumptions",
    "propvars": "properties",
    "display2d": "display",
    "%": "history",
    "timing": "showtime",
    "quit": "repl",
}


def parse_help_query(source: str) -> str | None:
    """Return the requested help topic if ``source`` is a ``?`` query."""
    stripped = source.strip()
    if not stripped.startswith("?"):
        return None
    topic = stripped.lstrip("?").strip()
    if topic.endswith((";", "$")):
        topic = topic[:-1].strip()
    return topic


def help_text(topic: str | None = None) -> str:
    """Return user-facing MACSYMA help text for ``topic``."""
    key = (topic or "").strip().lower()
    if not key:
        topics = ", ".join(sorted(_TOPICS))
        return f"MACSYMA help topics: {topics}. Use ? topic for details."
    key = _ALIASES.get(key, key)
    if key in _TOPICS:
        return _TOPICS[key]
    topics = ", ".join(sorted(_TOPICS))
    return f"No MACSYMA help topic named {topic!r}. Available topics: {topics}."


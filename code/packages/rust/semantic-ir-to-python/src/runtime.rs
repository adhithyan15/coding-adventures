//! Inlined Python runtime helpers — pasted into every artifact.
//!
//! Provides the `Symbol` / `Pair` / `Closure` classes, builtin
//! implementations, interning, truthiness, formatting, globals, and
//! a builtin dispatch table.  All single-threaded; CPython's GIL
//! makes this safe in-process.

pub const RUNTIME: &str = r##"# ── inlined SIR runtime ──────────────────────────────────────
class Symbol:
    __slots__ = ("name",)
    def __init__(self, name: str) -> None:
        self.name = name
    def __eq__(self, other: object) -> bool:
        return isinstance(other, Symbol) and self.name == other.name
    def __hash__(self) -> int:
        return hash(("__SIR_SYM__", self.name))
    def __repr__(self) -> str:
        return self.name

class Pair:
    __slots__ = ("car", "cdr")
    def __init__(self, car, cdr) -> None:
        self.car = car
        self.cdr = cdr

class Closure:
    __slots__ = ("fn",)
    def __init__(self, fn) -> None:
        self.fn = fn

_sir_symbol_table: dict[str, Symbol] = {}
_globals: dict[str, object] = {}

def _sir_intern(name: str) -> Symbol:
    s = _sir_symbol_table.get(name)
    if s is None:
        s = Symbol(name)
        _sir_symbol_table[name] = s
    return s

def _sir_truthy(v) -> bool:
    return v is not False and v is not None

def _sir_apply(c, args):
    if not isinstance(c, Closure):
        raise TypeError("apply on non-closure")
    return c.fn(*args)

def _sir_make_closure(fn, captures):
    return Closure(lambda *args: fn(*captures, *args))

def _sir_global_set(name, value):
    key = name.name if isinstance(name, Symbol) else str(name)
    _globals[key] = value
    return value

def _sir_global_get(name):
    key = name.name if isinstance(name, Symbol) else str(name)
    if key not in _globals:
        raise NameError(f"undefined global: {key}")
    return _globals[key]

def _sir_global_get_static(name: str):
    if name not in _globals:
        raise NameError(f"undefined global: {name}")
    return _globals[name]

def _sir_plus(*args):
    total = 0
    for a in args:
        total += a
    return total

def _sir_minus(*args):
    if not args:
        return 0
    if len(args) == 1:
        return -args[0]
    acc = args[0]
    for a in args[1:]:
        acc -= a
    return acc

def _sir_times(*args):
    acc = 1
    for a in args:
        acc *= a
    return acc

def _sir_divide(*args):
    if not args:
        return 0
    acc = args[0]
    for a in args[1:]:
        # Truncating integer division to match Twig semantics.
        acc = int(acc / a)
    return acc

def _sir_eq(a, b):
    if isinstance(a, Symbol) and isinstance(b, Symbol):
        return a.name == b.name
    return a == b

def _sir_lt(a, b):
    return a < b

def _sir_gt(a, b):
    return a > b

def _sir_cons(a, b):
    return Pair(a, b)

def _sir_car(p):
    if not isinstance(p, Pair):
        raise TypeError("car on non-pair")
    return p.car

def _sir_cdr(p):
    if not isinstance(p, Pair):
        raise TypeError("cdr on non-pair")
    return p.cdr

def _sir_is_null(v):
    return v is None

def _sir_is_pair(v):
    return isinstance(v, Pair)

def _sir_is_number(v):
    return isinstance(v, int) and not isinstance(v, bool)

def _sir_is_symbol(v):
    return isinstance(v, Symbol)

def _sir_format(v) -> str:
    if v is None:
        return "nil"
    if isinstance(v, bool):
        return "#t" if v else "#f"
    if isinstance(v, Symbol):
        return v.name
    if isinstance(v, Pair):
        out = ["(", _sir_format(v.car)]
        rest = v.cdr
        while isinstance(rest, Pair):
            out.append(" ")
            out.append(_sir_format(rest.car))
            rest = rest.cdr
        if rest is not None:
            out.append(" . ")
            out.append(_sir_format(rest))
        out.append(")")
        return "".join(out)
    if isinstance(v, Closure):
        return "<closure>"
    return str(v)

def _sir_print(v):
    print(_sir_format(v))
    return None

def _sir_call_builtin(name: str, args):
    dispatch = _sir_builtins.get(name)
    if dispatch is None:
        raise NameError(f"unknown builtin: {name}")
    return dispatch(*args)

def _sir_builtin_closure(name: str) -> Closure:
    return Closure(lambda *args: _sir_call_builtin(name, args))

_sir_builtins = {
    "+": _sir_plus, "-": _sir_minus, "*": _sir_times, "/": _sir_divide,
    "=": _sir_eq, "<": _sir_lt, ">": _sir_gt,
    "cons": _sir_cons, "car": _sir_car, "cdr": _sir_cdr,
    "null?": _sir_is_null, "pair?": _sir_is_pair,
    "number?": _sir_is_number, "symbol?": _sir_is_symbol,
    "print": _sir_print,
    "global_set": _sir_global_set, "global_get": _sir_global_get,
}
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_non_empty_ends_newline() {
        assert!(!RUNTIME.is_empty());
        assert!(RUNTIME.ends_with('\n'));
    }

    #[test]
    fn runtime_declares_classes_and_helpers() {
        for s in &["class Symbol", "class Pair", "class Closure", "_sir_intern", "_sir_truthy", "_sir_apply", "_sir_make_closure"] {
            assert!(RUNTIME.contains(s), "missing: {}", s);
        }
    }

    #[test]
    fn runtime_includes_all_builtins() {
        for op in &[
            "_sir_plus", "_sir_minus", "_sir_times", "_sir_divide",
            "_sir_eq", "_sir_lt", "_sir_gt",
            "_sir_cons", "_sir_car", "_sir_cdr",
            "_sir_is_null", "_sir_is_pair", "_sir_is_number", "_sir_is_symbol",
            "_sir_print", "_sir_format",
            "_sir_global_set", "_sir_global_get",
        ] {
            assert!(RUNTIME.contains(op), "runtime missing `{}`", op);
        }
    }
}

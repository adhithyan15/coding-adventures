---
category: Compiler / VM / language pipeline
---

# An unbound Python name read is not always a NameError; builtins and module dunders resolve first

LANG79's module-level binding slice first modelled "read before any
assignment" as a runtime NameError. The CPython oracle disagreed for a whole
class of names: `print(float)` prints `<class 'float'>`, `print(__name__)`
prints `__main__`, and `print(len)` resolves the builtin. Module lookup falls
back to the builtins module, and a script's globals already hold several
dunders before its first statement. Lowering those reads to NameError would
have invented an error CPython never raises.

The native compiler now checks bound names first, then rejects unbound reads
of `print`, any Python 3.12 builtin name or any dunder as unsupported at
compile time; only other unbound names lower to the raising builtin. Binding a
builtin name such as `float` is ordinary module-global shadowing and works.

When a frontend adds name resolution, enumerate every scope the language
consults (module, builtins, implicit module attributes), test an unbound
builtin and an unbound dunder against the host oracle, and keep the builtin
table pinned to the versioned language target rather than the host version.

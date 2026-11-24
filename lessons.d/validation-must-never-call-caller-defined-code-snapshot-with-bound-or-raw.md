---
category: Security boundaries
---

# Validation must never call caller-defined code; snapshot with bound or raw primitives first

Hardening review kept finding the same hole in dynamic lanes: the validator called caller-defined code (operator overloads, getters, metamethods, `Inspect`/`toString`/`__repr__`, tied variables, an `equal?`/`==` receiver, `isinstance` on a lying subclass), so a hostile argument ran code inside the gate or changed the answer. Validation must snapshot with bound or raw primitives first (`type.__dict__` lookups, class-bound `equal?`, rawget, exact-class checks) and only then judge the copy. Error text must be fixed or built from our own integers, never from a caller's class name or description.

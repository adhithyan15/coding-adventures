## 0.1.3 (security: bounded error descriptions)

- Error messages never interpolate a caller value unboundedly: ints above 128 bits are
  reported as "an int with N bits" (no `str()` on huge ints, so the library exception
  is raised instead of the interpreter digit-limit ValueError).
- Caller type names go through `_safe_type_name`: exact-str `__qualname__` only,
  control/non-ASCII characters escaped, capped at 64 chars; no user dunder is called.
- Applied to `gates`, `combinational` and `sequential` error paths.
- Tests: LG02 section 9 type-confusion rows on all 11 gates (spy-counted dunders,
  messages printable and <= 256 chars) and pinning of exact-`list`-only containers.
- `mux4`, `mux8`: `sel`/`inputs` now go through `_require_bit_list` (exact `list`,
  validated snapshot, computation only from the copy); non-lists (tuple, range, dict,
  generator, lying list subclass) raise `TypeError` instead of `ValueError`; wrong length
  still raises `ValueError`. Closes a TOCTOU where a list subclass with alternating
  `__getitem__` bypassed validation (CWE-367/843). `mux2` sel pinned as exact int.
- Review follow-ups: `_safe_type_name` reads the name via `type.__dict__` so a metaclass
  `__getattribute__` never runs, and truncates on an escape boundary; `_safe_int_text` uses
  unbound `int.bit_length`/`int.__repr__`. mux4/mux8 behaviour change restated: a non-list
  or list-subclass `sel`/`inputs` (previously `ValueError`, or accepted for subclasses)
  now raises `TypeError`.

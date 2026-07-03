# coding-adventures-sir-runtime-regex

Regex runtime for **Semantic-IR-emitted Python**.

The Ruby→SIR frontend lowers a regex literal `/pat/flags` to
`BuiltinCall("regex", [StrLit pattern, StrLit flags])`. Emitted Python needs a
runtime landing point for that builtin — one that compiles the pattern with
Python's standard `re` engine while translating Ruby's flag spellings and
line-anchor conventions. That runtime is this package.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                             │ imports
                                                                             ▼
                                                       coding-adventures-sir-runtime-regex
```

The Python backend emits an import of this package only when a module uses a
regex; pure modules never gain the dependency.

## Why a translation layer — Ruby's dialect differs from Python `re`

Both engines are Perl-compatible, so the *syntax* (`\d`, `[a-z]`, `(?:...)`,
`a|b`) is shared. They diverge in two places:

| Ruby flag char | Python `re` flag | Meaning |
|---|---|---|
| `i` | `re.IGNORECASE` | Case-insensitive matching. |
| `m` | `re.DOTALL` | Ruby `/m` ("multiline") makes `.` match a newline — this is Python's *DOTALL*, **not** Python's `re.MULTILINE`. The shared letter is a foot-gun. |
| `x` | `re.VERBOSE` | "Extended" mode — unescaped whitespace and `#` comments ignored. |
| (any other) | — | Unknown flag characters are silently dropped. |

**The `^` / `$` nuance.** In Ruby, `^` and `$` *always* anchor to the start/end
of a **line** — the whole-string anchors are `\A` and `\z`/`\Z`. In Python the
default is the opposite (`^`/`$` anchor the whole string unless `re.MULTILINE`).
So this package **always ORs in `re.MULTILINE`** to make Ruby patterns behave
faithfully; `\A` / `\z` carry over unchanged for whole-string anchoring.

**Match semantics.** Ruby's `=~` / `String#match?` perform an *unanchored
search*, not a full-string match. `is_match` / `match_data` therefore use
`re.Pattern.search`, never `fullmatch`.

## API

| Export | Purpose |
|---|---|
| `compile(pattern, flags="") -> re.Pattern[str]` | Compile a Ruby-dialect pattern under Ruby-dialect flags (always line-anchored). |
| `is_match(pattern, string) -> bool` | True iff an unanchored search matches (Ruby `=~`/`match?`). Accepts a compiled pattern or a raw string. |
| `match_data(pattern, string) -> str \| None` | The matched substring (group 0), or `None` on no match (minimal `String#match`). |
| `Val` | The universal SIR value type alias (`Any`) at this boundary. |

`compile` intentionally shadows `builtins.compile` / `re.compile`: `regex` is the
SIR builtin's name and emitted code addresses this package's `compile` by
qualified name, so the shadow is harmless.

## Usage

```python
from coding_adventures_sir_runtime_regex import compile, is_match, match_data

pat = compile(r"\d+", "i")        # Ruby /\d+/i
is_match(pat, "abc 42")           # True   (unanchored search, like Ruby =~)
is_match(r"\d+", "no digits")     # False  (raw string also accepted)
match_data(r"\d+", "abc 42 xyz")  # "42"   (group 0)
match_data(r"\d+", "none")        # None   (no match)
```

## Development

```bash
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT

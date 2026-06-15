# @coding-adventures/sir-runtime-regex

Regex runtime for **Semantic-IR-emitted TypeScript/JavaScript**.

The Ruby→SIR frontend lowers a regex literal `/pat/flags` to
`BuiltinCall("regex", [StrLit pattern, StrLit flags])`. Emitted TypeScript needs
a runtime landing point for that builtin — one that builds a native `RegExp`
while translating Ruby's flag spellings and line-anchor conventions. That
runtime is this package.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-typescript ─▶ .ts
                                                                                  │ imports
                                                                                  ▼
                                                              @coding-adventures/sir-runtime-regex
```

The TypeScript backend emits an import of this package only when a module uses a
regex; pure modules never gain the dependency.

## Why a translation layer — Ruby's dialect differs from JS `RegExp`

Both engines are Perl-compatible, so most *syntax* (`\d`, `[a-z]`, `(?:...)`,
`a|b`) is shared. They diverge in three places:

| Ruby flag char | JS flag | Meaning |
|---|---|---|
| `i` | `i` | Case-insensitive matching. |
| `m` | `s` | Ruby `/m` ("multiline") makes `.` match a newline — this is JS's *dotAll* (`s`), **not** JS's `m`. The shared letter is a foot-gun. |
| `x` | (none) | "Extended" mode — ignore unescaped whitespace and `#` comments. JS has no equivalent flag, so the pattern text is preprocessed with `stripExtended` (best-effort subset). |
| (any other) | — | Unknown flag characters are silently dropped. |

**The `^` / `$` nuance.** In Ruby, `^` and `$` *always* anchor to the start/end
of a **line** — the whole-string anchors are `\A` and `\z`/`\Z`. In JS the
default is the opposite (`^`/`$` anchor the whole string unless `m` is set). So
this package **always includes the JS `m` flag** to make Ruby patterns behave
faithfully.

**Match semantics.** Ruby's `=~` / `String#match?` perform an *unanchored
search*, not a full-string match. `isMatch` / `matchData` therefore use
`.test` / `.exec` on a fresh non-global RegExp (so a stale `lastIndex` from a
global/sticky regex never leaks across calls).

## API

| Export | Purpose |
|---|---|
| `compile(pattern, flags=""): RegExp` | Build a Ruby-dialect `RegExp` under Ruby-dialect flags (always line-anchored; `x` strips ignorable whitespace/comments). |
| `isMatch(pattern, s): boolean` | True iff an unanchored search matches (Ruby `=~`/`match?`). Accepts a `RegExp` or a raw string. |
| `matchData(pattern, s): string \| null` | The matched substring (`match[0]`), or `null` on no match (minimal `String#match`). |
| `stripExtended(pattern): string` | Best-effort `/x` subset: strip unescaped whitespace and `#` comments. |
| `Val` | The universal SIR value type alias (`any`) at this boundary. |

## Usage

```ts
import { compile, isMatch, matchData } from "@coding-adventures/sir-runtime-regex";

const re = compile("\\d+", "i");     // Ruby /\d+/i
isMatch(re, "abc 42");               // true   (unanchored search, like Ruby =~)
isMatch("\\d+", "no digits");        // false  (raw string also accepted)
matchData("\\d+", "abc 42 xyz");     // "42"   (group 0)
matchData("\\d+", "none");           // null   (no match)
```

## Development

```bash
npm ci
npx tsc --noEmit      # strict typecheck
npx vitest run --coverage
```

## License

MIT

---
category: Rust
---

# A line-oriented reader of a nested format loses the state that separates a key from its section

The capability analyzer's first `Cargo.toml` reader looked at one line at a time
and remembered only "the current `[section]`". A security review bypassed its
`path` / `build` / `proc-macro` rules six ways: top-level inline tables
(`lib = { path = ".." }`), arrays of inline tables, quoted table names
(`["lib"]`), multi-line `'''` strings whose body looks like a header, and
multi-line arrays whose `["x"],` lines look like headers and reset the section.
Every bypass was state that TOML carries *between* lines.

The fix was to tokenize the whole file, flatten each value to a full key path
(`bin`, `[]`, `path`) and apply one rule to the path, however it was spelled;
anything the reader cannot parse is a finding rather than a skipped line.

Do differently: when a security rule is about *where* a key lives, parse the
nesting, do not approximate it with lines. Write the rule over the normalised
path, enumerate the spellings that reach it (dotted, quoted, inline, array of
tables) in the regression corpus, and make "I could not parse this" fail closed.
The same review found a quadratic expansion (checked its cap after building the
product), so check resource caps inside the loop that produces the growth.

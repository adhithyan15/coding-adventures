# PIF: the grammar of the Proof Interchange Format

`pif.tokens` and `pif.grammar` describe the concrete syntax of PIF version 1, defined
in [VER32](../../specs/VER32-proof-interchange-format.md) section 4 (the language), 4.1
(why the tokens carry their own spacing) and 5.3 (the reader's rule list). A PIF file is
the exact byte string these two files accept: a header, records, and an `end <count>`
trailer, ASCII only, LF line ends, one space between fields.

```text
pif 1
logic 1
profile native "pif-case-split\u{20}0.1"
features
nstr 1 0 "Bool"
econst 2 1 []
end 2
```

The pair is read by the generic `GrammarLexer` and `GrammarParser`; there is no
hand-written lexer or parser in the path. Tests live in
[`rust/pif-grammar`](../../packages/rust/pif-grammar) (parse tests, the worked example,
and the lexical negative cases).

## What the grammar enforces (lexical and structural)

Single separating spaces (no leading, trailing or doubled space, no tab, CR or blank
line), byte-exact brackets, ASCII-only bytes, canonical decimal numerals, the string
shape (the escapes `\"`, `\\`, `\u{h}` with no leading zero, no raw space), digests
(`sha256:` and 64 lowercase hex digits), the four-line header, every record's field
shape, inductive-block nesting, `\u{0}` only in an `estr` literal, the four binder words
never being identifiers, and an `end` line followed by end of input.

## Reader-only rules (not grammar rules)

VER32 5.3 assigns these to the trusted reader. The grammar accepts them, and the tests
pin that so tightening it is deliberate:

| Reader rule | Not checked by the grammar |
|---|---|
| R-LEX-2 | a `\u{h}` escape denotes a Unicode scalar (surrogates, above 10FFFF) |
| R-LEX-3 | an escape is used only for a code point with no direct spelling (`\u{41}`) |
| R-LEX-4 | numbers fit their bounds (indices, `nnum`, heights, `nat` digit count) |
| R-VER-*, R-FEA-1 | versions, feature names, feature order, record-needs-feature |
| R-IDX-* | dense indices from 1, backward references only |
| R-CNT-1, R-CNT-2 | `inductive`/`itype` counts, `end k` equals the record-line count |
| R-NAM-*, R-GOL-3 | name component classes, claim id charset |
| R-PRM-*, R-AX-*, R-DER-1, R-GOL-*, R-DEC-1, R-IMP-*, R-LIM-1 | everything semantic |

## Differences from the VER32 4.2 and 4.3 sketches

The files describe the same language as the spec. Four engine facts forced a different
spelling; each is commented where it occurs.

1. **A `skip:` section that skips nothing.** `GrammarLexer` silently skips space, tab
   and CR when a grammar has no `skip:` section, which would admit doubled spaces and
   tabs. `pif.tokens` declares a skip pattern that can never match (`NEVER`), which turns
   the built-in rule off. The "no skip section" of 4.2 is realised this way.
2. **`FIDENT_LONG`.** The lexer takes the first matching definition, not the longest
   match. With the literal binder tokens listed first, `instance` would lex as `inst`
   plus `ance`. `FIDENT_LONG`, listed before them and aliased to `FIDENT`, keeps longer
   identifiers whole; the exact words `default`, `implicit`, `strict`, `inst` still tie
   with `FIDENT` and the literal wins, as 4.2 says.
3. **`estr FNAT ( FSTRING | FLSTRING )`.** 4.3 names only `FLSTRING`, but `FSTRING` is
   listed first and so is the token for every string without `\u{0}`; `estr "abc"` would
   never parse. The rule accepts either token; names and goals still accept only
   `FSTRING`.
4. **`trailer = "end" FNAT NEWLINE EOF`.** `GrammarParser` silently skips NEWLINE tokens
   left after the entry rule, so `end 0\n\n` would parse. Naming `EOF` makes "nothing
   follows the end line" part of the grammar.

The `keywords:` section also lists one word per line (the file format's rule), where 4.2
shows several per line.

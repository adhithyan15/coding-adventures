# LANG51 — String Literals

## Motivation

A self-hosted Twig compiler (TW05) needs to write string constants in its source
code: keyword names (`"define"`, `"lambda"`), error messages, token strings.
Without string literal syntax, every compile-time string must be built at runtime
from character-code arithmetic — a recipe for illegible, unmaintainable code.

LANG51 adds double-quoted string literal syntax end-to-end: lexer token, grammar
rule, parser AST node, IIR lowering, and type-checker annotation.

## Syntax

```scheme
"hello"          ; string literal — type str
""               ; empty string
"say \"hi\""     ; escape: \" → "
"line1\nline2"   ; escape: \n → newline
"\t"             ; escape: \t → tab
"\\"             ; escape: \\ → backslash
```

## Escape sequences (v1)

| Source | Decoded |
|--------|---------|
| `\\`   | `\` |
| `\"`   | `"` |
| `\n`   | U+000A newline |
| `\t`   | U+0009 tab |
| `\r`   | U+000D carriage return |

Unknown sequences `\x` are passed through as just `x` (the GrammarLexer's
`process_escapes` silently absorbs them; stricter validation can be added in a
future LANG without a grammar change).

## Type hint

String literals carry `type_hint = "str"` on the emitted `const` instruction.
`compile_typed_source` propagates this to `KindDecl::Str → "str"` automatically.

## IIR emission

A string literal `"hello"` lowers to a single `const` instruction:

```
%str0 = const(Str("hello")) : "str"
```

`Operand::Str` was introduced in LANG32 specifically for compile-time string
constants.  The VM's `exec_const` already materialises `Operand::Str(text)` as a
`LangString` heap object (LANG47), so no VM changes are needed.

## Runtime representation

`LangString` (heap class `CLASS_STRING = 3`) — an immutable `Box<[u8]>` of UTF-8
bytes.  Already fully implemented in `lispy-runtime/src/heap.rs`.  All string
builtins (`string-length`, `string-ref`, `substring`, `string-append`,
`string=?`, etc.) operate on `LangString` values and work unchanged.

## Files changed

| File | Change |
|------|--------|
| `code/grammars/twig.tokens` | Add `STRING = /"([^"\\]|\\.)*"/` before INTEGER |
| `code/grammars/twig.grammar` | Add `STRING` to `atom` rule |
| `twig-parser/src/ast_nodes.rs` | Add `StrLit` struct + `Expr::StrLit` variant |
| `twig-parser/src/ast_extract.rs` | Add `"STRING"` case in `extract_atom`; uses `tok.value` directly (GrammarLexer pre-decodes escapes) |
| `lexer/src/grammar_lexer.rs` | Add `\r` and `\'` to `process_escapes` |
| `twig-ir-compiler/src/compiler.rs` | Add `Expr::StrLit` arm → `const(Operand::Str(value))` : `"str"` |
| `twig-type-checker/src/profile.rs` | Add `"STRING"` → `KindDecl::Str` in `literal_kind` |

## No changes needed

- `lispy-runtime/src/heap.rs` — `LangString` + `alloc_string` already exist (LANG47)
- `twig-vm/src/dispatch.rs` — `exec_const` already handles `Operand::Str` (LANG47)

## Backward compatibility

All existing Twig programs continue to parse and compile unchanged.

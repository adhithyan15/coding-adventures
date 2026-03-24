# Parser Grammar File Format (`.grammar`)

## Overview

A `.grammar` file defines the syntactic grammar of a language using an EBNF-like
notation. It is loaded at runtime by the `grammar-tools` package, parsed into a
`ParserGrammar` data structure, and passed to the `GrammarParser` constructor.
The parser implements a **recursive descent PEG parser with packrat memoization**,
interpreting grammar rules at runtime rather than generating code.

`.grammar` files are language-agnostic — the same `sql.grammar` is loaded by
Go, Python, Ruby, TypeScript, Rust, and Elixir parser wrappers identically.

---

## File Encoding

UTF-8. Line endings: LF (`\n`).

---

## Comments

Lines whose first non-whitespace character is `#` are comments:

```
# This is a comment
value = object | array | STRING | NUMBER ;   # inline comments NOT supported
```

---

## Magic Comments

Magic comments (`# @key value`) provide configuration metadata. They are parsed
before the grammar rules and stored in the `ParserGrammar` struct.

### `# @version N`

```
# @version 1
```

Pins the grammar to format version `N`. See `tokens-format.md` for the full
version semantics. Default when missing: `0` (current/latest).

### Position

By convention, magic comments appear at the top of the file. They are valid
anywhere.

---

## Rule Definitions

The body of a `.grammar` file is a sequence of rule definitions:

```
rule_name = body ;
```

- `rule_name` — lowercase identifier matching `[a-z_][a-z0-9_]*`.
- `body` — a grammar expression (see below).
- Rules are terminated by `;`.
- Whitespace and newlines within a rule body are insignificant.

### Entry Point

**The first rule** in the file is the grammar's entry point. The parser starts
by trying to match this rule, and expects the entire token stream to be consumed
after a successful parse (ignoring trailing NEWLINE and EOF tokens).

---

## Grammar Elements

### Token References (UPPERCASE)

An all-caps identifier references a **token type** from the companion `.tokens`
file. The parser matches the current token if its `TypeName` equals the
identifier:

```
value = STRING | NUMBER | NAME ;
```

Special built-in token types always available:

| Name | Meaning |
|------|---------|
| `NAME` | Identifier token |
| `NUMBER` | Numeric literal |
| `STRING` | String literal (or alias target) |
| `KEYWORD` | Any keyword token |
| `NEWLINE` | Newline token |
| `EOF` | End of input |

Any token name defined in the `.tokens` file (e.g., `NOT_EQUALS`, `STAR`,
`SEMICOLON`) can be referenced here by its TypeName or alias target.

### Rule References (lowercase)

A lowercase identifier references **another grammar rule**. The parser recursively
tries to match that rule at the current position:

```
expr = term { PLUS term } ;
```

Forward references are allowed — rules can be referenced before they are defined.
Mutual recursion is supported (e.g., `expr` references `term` which references `expr`).

### Literal Strings (`"value"`)

A double-quoted string matches the **value** of the current token (regardless of
token type). This is how specific keywords are matched:

```
select_stmt = "SELECT" select_list "FROM" table_ref ;
```

When the grammar uses `# @case_insensitive true` in its companion `.tokens` file,
keyword values are normalized to uppercase by the lexer. Grammar literals should
use uppercase to match (`"SELECT"`, not `"select"`).

---

## Quantifiers and Grouping

### Sequence (adjacency)

Elements listed one after another must all match in order:

```
pair = STRING ":" value ;
```

### Alternation (`|`)

Try each alternative in order; use the first that matches (ordered choice /
PEG semantics — NOT ambiguous BNF alternation):

```
value = object | array | STRING | NUMBER | "true" | "false" | "null" ;
```

**Order matters**: put longer/more-specific alternatives before shorter ones.

### Zero-or-more (`{ x }`)

Match the element zero or more times (greedy):

```
program = { statement } ;
```

### Optional (`[ x ]`)

Match the element zero or one times:

```
select_stmt = "SELECT" [ "DISTINCT" ] select_list ;
```

### Grouping (`( x )`)

Group sub-expressions for clarity or to apply a quantifier to multiple elements:

```
join_type = "INNER" | ( "LEFT" [ "OUTER" ] ) | "CROSS" ;
```

Grouping does not affect the AST — it is purely for expression structure.

---

## Operator Precedence

Grammar operators bind in this order (tightest to loosest):

1. Atoms: token refs, rule refs, literals, `( x )`
2. Quantifiers: `{ x }`, `[ x ]`
3. Sequence (adjacency)
4. `|` (alternation)

Example showing precedence:

```
# This:
expr = a b | c d ;
# Means: (a b) | (c d) — NOT a (b | c) d
```

When in doubt, use explicit grouping `( )`.

---

## AST Structure

The parser produces an `ASTNode` tree where:

- Each node has a `RuleName` (the matched rule's name) and a `Children` list.
- Children are either `ASTNode` pointers (sub-rule matches) or `Token` values
  (terminal matches).
- A **leaf node** has exactly one child that is a `Token`.

The grammar does not allow explicit AST shape customization — the tree structure
mirrors the grammar rule structure. Post-processing is done by consumers of the
AST.

---

## Packrat Memoization

The parser caches `(rule_index, token_position) → result` so that each rule is
tried at most once per position. This guarantees **linear time** parsing of
PEG grammars, avoiding exponential blowup from backtracking.

Implication: grammars with left-recursion are NOT supported. Write:

```
# WRONG — left-recursive:
expr = expr "+" term | term ;

# CORRECT — right-iterative:
expr = term { "+" term } ;
```

---

## Error Reporting

The parser tracks the **furthest position reached** during parsing. When a parse
fails, it reports the expected token(s) at that furthest position — this gives
better error messages than reporting the failure at the start of the input.

Example error: `Parse error at 3:7: Expected ")" or "," , got "WHERE"`

---

## NEWLINE Token Handling

By default, NEWLINE tokens are **insignificant** — the parser skips them when
matching any element. This is the correct behavior for most grammars where
statements can span multiple lines.

**Exception:** If any rule explicitly references `NEWLINE` (e.g., Python/Starlark
with `mode: indentation`), the parser enters **newline-significant mode** and
does NOT skip NEWLINE tokens automatically. The grammar must explicitly handle
them.

The parser auto-detects newline significance by scanning all rules for `NEWLINE`
references.

---

## Complete Example: `sql.grammar`

```
# Parser grammar for SQL (ANSI SQL subset)
# @version 1
#
# UPPERCASE identifiers reference token types from sql.tokens.
# Quoted strings match keyword values (normalized to uppercase by the lexer
# because sql.tokens uses @case_insensitive true).

program           = statement { ";" statement } [ ";" ] ;

statement         = select_stmt | insert_stmt | update_stmt
                  | delete_stmt | create_table_stmt | drop_table_stmt ;

select_stmt       = "SELECT" [ "DISTINCT" | "ALL" ] select_list
                    "FROM" table_ref { join_clause }
                    [ where_clause ] [ group_clause ] [ having_clause ]
                    [ order_clause ] [ limit_clause ] ;

select_list       = STAR | select_item { "," select_item } ;
select_item       = expr [ "AS" NAME ] ;
...
```

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `03-parser.md` | High-level parser concepts and motivation |
| `tokens-format.md` | The companion `.tokens` file format |
| `F04-lexer-pattern-groups.md` | Context-sensitive lexing with groups |
| `sql.md` | SQL grammar design decisions and subset scope |

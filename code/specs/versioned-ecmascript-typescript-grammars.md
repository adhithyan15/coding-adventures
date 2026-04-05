# Versioned ECMAScript & TypeScript Grammar System

## Overview

This spec defines a comprehensive grammar system covering every major ECMAScript version
(ES1 through ES2025) and TypeScript version (1.0 through 5.8). Each version gets its own
self-contained `.tokens` and `.grammar` file pair. No file inherits from or extends another —
each is a complete, standalone definition of the language at that point in time.

The grammar files use the existing `.tokens` and `.grammar` formats documented in
`tokens-format.md` and `grammar-format.md`, plus extensions from `lexer-parser-extensions.md`
(syntactic predicates, context keywords, token lookbehind, bracket depth tracking).

### Why version-per-file?

JavaScript has evolved more than almost any other language. Code written for ES1 in 1997 lives
in a fundamentally different universe than ES2025 code with decorators and import attributes.
By giving each version its own grammar, we can:

1. **Parse historically** — feed a 1999-era script through the ES3 grammar and get an accurate AST
2. **Teach evolution** — read the files in order to see exactly what each version added
3. **Test precisely** — verify that `?.` is a syntax error in ES2019 but valid in ES2020
4. **Build version-aware tooling** — linters that know which features your target supports

### Relationship to existing grammars

The current `javascript.tokens`, `javascript.grammar`, `typescript.tokens`, and
`typescript.grammar` in `code/grammars/` are toy subsets (variable declarations and basic
arithmetic). This spec supersedes them. The new versioned files live in subdirectories.

---

## 1. File Naming Convention

### ECMAScript Files

| Version | Year | Tokens File | Grammar File | Notes |
|---------|------|-------------|--------------|-------|
| ES1 | 1997 | `es1.tokens` | `es1.grammar` | ECMA-262 1st edition. The foundation. |
| ES2 | 1998 | — | — | Editorial changes only. Alias to ES1. |
| ES3 | 1999 | `es3.tokens` | `es3.grammar` | Regex, try/catch, `===`/`!==`. |
| ES5 | 2009 | `es5.tokens` | `es5.grammar` | Strict mode, getters/setters. ES4 abandoned. |
| ES5.1 | 2011 | — | — | Minor corrections. Alias to ES5. |
| ES2015 | 2015 | `es2015.tokens` | `es2015.grammar` | The big one: classes, arrows, modules, etc. |
| ES2016 | 2016 | `es2016.tokens` | `es2016.grammar` | Exponentiation operator. |
| ES2017 | 2017 | `es2017.tokens` | `es2017.grammar` | async/await. |
| ES2018 | 2018 | `es2018.tokens` | `es2018.grammar` | Async iteration, rest/spread properties. |
| ES2019 | 2019 | `es2019.tokens` | `es2019.grammar` | Optional catch binding. |
| ES2020 | 2020 | `es2020.tokens` | `es2020.grammar` | `?.`, `??`, BigInt, dynamic import. |
| ES2021 | 2021 | `es2021.tokens` | `es2021.grammar` | Logical assignment, numeric separators. |
| ES2022 | 2022 | `es2022.tokens` | `es2022.grammar` | Class fields, private `#names`, static blocks. |
| ES2023 | 2023 | `es2023.tokens` | `es2023.grammar` | Hashbang comments. |
| ES2024 | 2024 | `es2024.tokens` | `es2024.grammar` | Regex `/v` flag. |
| ES2025 | 2025 | `es2025.tokens` | `es2025.grammar` | Decorators, import attributes, `using`. |

ES2 and ES5.1 are aliases (symlinks or comments pointing to ES1/ES5). No separate files needed.

### TypeScript Files

| Version | Year | Tokens File | Grammar File | ES Baseline |
|---------|------|-------------|--------------|-------------|
| TS 1.0 | 2014 | `ts1.0.tokens` | `ts1.0.grammar` | ES5 |
| TS 2.0 | 2016 | `ts2.0.tokens` | `ts2.0.grammar` | ES2015 |
| TS 3.0 | 2018 | `ts3.0.tokens` | `ts3.0.grammar` | ES2018 |
| TS 4.0 | 2020 | `ts4.0.tokens` | `ts4.0.grammar` | ES2020 |
| TS 5.0 | 2023 | `ts5.0.tokens` | `ts5.0.grammar` | ES2022 |
| TS 5.8 | 2025 | `ts5.8.tokens` | `ts5.8.grammar` | ES2025 |

Each TypeScript file is self-contained: it includes the full ES baseline grammar plus
all TypeScript-specific syntax for that version.

### Directory Structure

```
code/grammars/
  ecmascript/
    es1.tokens
    es1.grammar
    es3.tokens
    es3.grammar
    es5.tokens
    es5.grammar
    es2015.tokens
    es2015.grammar
    es2016.tokens
    es2016.grammar
    es2017.tokens
    es2017.grammar
    es2018.tokens
    es2018.grammar
    es2019.tokens
    es2019.grammar
    es2020.tokens
    es2020.grammar
    es2021.tokens
    es2021.grammar
    es2022.tokens
    es2022.grammar
    es2023.tokens
    es2023.grammar
    es2024.tokens
    es2024.grammar
    es2025.tokens
    es2025.grammar
  typescript/
    ts1.0.tokens
    ts1.0.grammar
    ts2.0.tokens
    ts2.0.grammar
    ts3.0.tokens
    ts3.0.grammar
    ts4.0.tokens
    ts4.0.grammar
    ts5.0.tokens
    ts5.0.grammar
    ts5.8.tokens
    ts5.8.grammar
```

### Magic Comments

Every file includes version metadata:

```
# ECMAScript 2020 lexical grammar
# @version 1
# @ecmascript_edition 2020
```

```
# TypeScript 5.0 parser grammar
# @version 1
# @typescript_version 5.0
# @ecmascript_baseline 2022
```

---

## 2. ECMAScript Version Feature Inventory

### ES1 (1997) — The Foundation

ES1 is the original ECMA-262 specification. It formalized the core of what Brendan Eich built
for Netscape Navigator in 1995. Everything that follows builds on this.

#### Tokens

**Identifiers:**

```
NAME = /[a-zA-Z_$][a-zA-Z0-9_$]*/
```

The `$` in identifiers is a JavaScript signature — it was included for generated code
(like Java's `$` in inner class names) and later became jQuery's calling card.

**Numbers:**

```
NUMBER = /0[xX][0-9a-fA-F]+/            # hex integer: 0xFF
       | /[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?/  # decimal: 42, 3.14, 1e10
       | /\.[0-9]+([eE][+-]?[0-9]+)?/         # leading dot: .5, .5e2
```

No binary literals, no octal prefix (`0o`), no BigInt. Octal was implementation-defined
via leading zero (e.g., `077`) but not formally specified.

**Strings:**

```
STRING_DQ = /"([^"\\]|\\.)*"/ -> STRING
STRING_SQ = /'([^'\\]|\\.)*'/ -> STRING
```

Both single and double quotes. Backslash escapes: `\\`, `\"`, `\'`, `\n`, `\r`, `\t`,
`\b`, `\f`, `\0`, `\xHH` (hex), `\uHHHH` (Unicode BMP). No template literals.

**Operators (46 total):**

Arithmetic: `+`, `-`, `*`, `/`, `%`
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
Bitwise: `&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`
Bitwise assignment: `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
Logical: `&&`, `||`, `!`
Increment/decrement: `++`, `--`
Ternary: `?` (paired with `:`)
Comma: `,`

Note: NO `===` or `!==` in ES1. Strict equality was added in ES3.

**Punctuation:**

```
( ) { } [ ] ; , . :
```

**Keywords (26):**

```
keywords:
  break case continue default delete do else
  for function if in new return switch this
  typeof var void while with
```

**Future reserved words (7):**

```
reserved:
  class const enum export extends import super
```

These cause a lex error if used as identifiers, reserving them for future versions.

**Skip patterns:**

```
skip:
  WHITESPACE    = /[ \t\r\n]+/
  LINE_COMMENT  = /\/\/[^\n]*/
  BLOCK_COMMENT = /\/\*([^*]|\*[^\/])*\*\//
```

**Error tokens:**

```
errors:
  BAD_STRING = /"[^"]*$/     # unclosed double-quoted string
  BAD_STRING = /'[^']*$/     # unclosed single-quoted string
```

#### Grammar Rules

```
program = { source_element } ;

source_element = function_declaration
               | statement ;

# --- Declarations ---

function_declaration = "function" NAME LPAREN [ formal_parameter_list ] RPAREN
                       LBRACE { source_element } RBRACE ;

formal_parameter_list = NAME { COMMA NAME } ;

# --- Statements ---

statement = block
          | variable_statement
          | empty_statement
          | expression_statement
          | if_statement
          | while_statement
          | for_statement
          | for_in_statement
          | do_while_statement
          | continue_statement
          | break_statement
          | return_statement
          | with_statement
          | switch_statement
          | labelled_statement ;

block = LBRACE { statement } RBRACE ;

variable_statement = "var" variable_declaration_list SEMICOLON ;

variable_declaration_list = variable_declaration { COMMA variable_declaration } ;

variable_declaration = NAME [ EQUALS assignment_expression ] ;

empty_statement = SEMICOLON ;

expression_statement = expression SEMICOLON ;

if_statement = "if" LPAREN expression RPAREN statement [ "else" statement ] ;

while_statement = "while" LPAREN expression RPAREN statement ;

do_while_statement = "do" statement "while" LPAREN expression RPAREN SEMICOLON ;

for_statement = "for" LPAREN
                  [ expression | "var" variable_declaration_list ] SEMICOLON
                  [ expression ] SEMICOLON
                  [ expression ]
                RPAREN statement ;

for_in_statement = "for" LPAREN
                     ( NAME | "var" variable_declaration ) "in" expression
                   RPAREN statement ;

continue_statement = "continue" [ NAME ] SEMICOLON ;

break_statement = "break" [ NAME ] SEMICOLON ;

return_statement = "return" [ expression ] SEMICOLON ;

with_statement = "with" LPAREN expression RPAREN statement ;

switch_statement = "switch" LPAREN expression RPAREN
                   LBRACE { case_clause } [ default_clause { case_clause } ] RBRACE ;

case_clause = "case" expression COLON { statement } ;

default_clause = "default" COLON { statement } ;

labelled_statement = NAME COLON statement ;

# --- Expressions ---
#
# Operator precedence is encoded by rule nesting, from lowest (comma) to
# highest (primary). Each level consumes operators at that precedence and
# delegates to the next-higher level for operands.
#
#   comma < assignment < conditional < logical_or < logical_and
#   < bitwise_or < bitwise_xor < bitwise_and < equality
#   < relational < shift < additive < multiplicative
#   < unary < postfix < call/member < primary

expression = assignment_expression { COMMA assignment_expression } ;

assignment_expression = conditional_expression
                      | left_hand_side_expression assignment_operator assignment_expression ;

assignment_operator = EQUALS | PLUS_EQUALS | MINUS_EQUALS | STAR_EQUALS
                    | SLASH_EQUALS | PERCENT_EQUALS | AMPERSAND_EQUALS
                    | PIPE_EQUALS | CARET_EQUALS | LEFT_SHIFT_EQUALS
                    | RIGHT_SHIFT_EQUALS | UNSIGNED_RIGHT_SHIFT_EQUALS ;

conditional_expression = logical_or_expression
                         [ QUESTION assignment_expression COLON assignment_expression ] ;

logical_or_expression = logical_and_expression { OR_OR logical_and_expression } ;

logical_and_expression = bitwise_or_expression { AND_AND bitwise_or_expression } ;

bitwise_or_expression = bitwise_xor_expression { PIPE bitwise_xor_expression } ;

bitwise_xor_expression = bitwise_and_expression { CARET bitwise_and_expression } ;

bitwise_and_expression = equality_expression { AMPERSAND equality_expression } ;

equality_expression = relational_expression { ( EQUALS_EQUALS | NOT_EQUALS ) relational_expression } ;

relational_expression = shift_expression { ( LESS_THAN | GREATER_THAN | LESS_EQUALS | GREATER_EQUALS | "in" | "instanceof" ) shift_expression } ;

shift_expression = additive_expression { ( LEFT_SHIFT | RIGHT_SHIFT | UNSIGNED_RIGHT_SHIFT ) additive_expression } ;

additive_expression = multiplicative_expression { ( PLUS | MINUS ) multiplicative_expression } ;

multiplicative_expression = unary_expression { ( STAR | SLASH | PERCENT ) unary_expression } ;

unary_expression = postfix_expression
                 | "delete" unary_expression
                 | "void" unary_expression
                 | "typeof" unary_expression
                 | PLUS_PLUS unary_expression
                 | MINUS_MINUS unary_expression
                 | PLUS unary_expression
                 | MINUS unary_expression
                 | TILDE unary_expression
                 | BANG unary_expression ;

postfix_expression = left_hand_side_expression [ PLUS_PLUS | MINUS_MINUS ] ;

left_hand_side_expression = call_expression | new_expression ;

call_expression = member_expression arguments { arguments | DOT NAME | LBRACKET expression RBRACKET } ;

new_expression = member_expression
               | "new" new_expression ;

member_expression = primary_expression { DOT NAME | LBRACKET expression RBRACKET }
                  | "new" member_expression arguments ;

arguments = LPAREN [ assignment_expression { COMMA assignment_expression } ] RPAREN ;

primary_expression = "this"
                   | NAME
                   | NUMBER
                   | STRING
                   | array_literal
                   | object_literal
                   | LPAREN expression RPAREN ;

array_literal = LBRACKET [ element_list ] RBRACKET ;

element_list = [ assignment_expression ] { COMMA [ assignment_expression ] } ;

object_literal = LBRACE [ property_assignment { COMMA property_assignment } [ COMMA ] ] RBRACE ;

property_assignment = property_name COLON assignment_expression ;

property_name = NAME | STRING | NUMBER ;

function_expression = "function" [ NAME ] LPAREN [ formal_parameter_list ] RPAREN
                      LBRACE { source_element } RBRACE ;
```

**What ES1 does NOT have:**

- No `===`/`!==` (strict equality)
- No `try`/`catch`/`finally`/`throw`
- No regex literals (implementation-defined)
- No `instanceof` keyword
- No labeled `break`/`continue` targets (wait — actually ES1 does have labels)
- No formal regex syntax in the grammar

---

### ES3 (1999) — Regularization

ES3 was the version that made JavaScript a real, complete language. It added error handling,
formal regex literals, and strict equality.

#### Tokens Added (delta from ES1)

**New operators:**

```
STRICT_EQUALS     = "==="
STRICT_NOT_EQUALS = "!=="
```

These must appear BEFORE `==` and `!=` in the token file (first-match-wins).

**Regex literal:**

```
REGEX = /\/([^\/\\\n]|\\.)+\/[gimsuy]*/
```

Regex literals require context-sensitive lexing. The `/` character is ambiguous: it could
start a regex or be a division operator. Resolution uses `previousToken()` (Extension 1):

- `/` is REGEX_START after: `= ( [ { , ; : ? ! & | ^ ~ + - * / % < > return typeof void delete new in instanceof case throw`
- `/` is SLASH (division) after: `) ] NAME NUMBER STRING ++ --`

**New keywords:**

```
catch finally instanceof throw try
```

**Expanded future reserved:**

```
reserved:
  abstract boolean byte char class const debugger double
  enum export extends final float goto implements import
  int interface long native package private protected public
  short static super synchronized throws transient volatile
```

#### Grammar Rules Added

```
# Error handling
try_statement = "try" block ( catch_clause [ finally_clause ] | finally_clause ) ;
catch_clause = "catch" LPAREN NAME RPAREN block ;
finally_clause = "finally" block ;
throw_statement = "throw" expression SEMICOLON ;  # no newline between throw and expression (ASI restricted)
```

#### Grammar Rules Changed

- `equality_expression` gains `===` and `!==`
- `primary_expression` gains `REGEX` as an alternative
- `relational_expression` gains `"instanceof"`
- `statement` gains `try_statement` and `throw_statement`

---

### ES5 (2009) — Strict Mode & JSON

ES5 landed a decade after ES3 (ES4 was famously abandoned). The syntactic changes are
modest — the real additions were strict mode semantics and built-in JSON.

#### Tokens Added (delta from ES3)

**Keywords promoted from future-reserved:**

```
debugger
```

**String continuation (lexer change):** A backslash immediately before a newline continues
the string to the next line. This is a lexer-level change to the STRING pattern.

#### Grammar Rules Added

```
# Getters and setters in object literals
getter_property = "get" property_name LPAREN RPAREN LBRACE { source_element } RBRACE ;
setter_property = "set" property_name LPAREN NAME RPAREN LBRACE { source_element } RBRACE ;

# Debugger statement
debugger_statement = "debugger" SEMICOLON ;
```

#### Grammar Rules Changed

- `property_assignment` gains `getter_property` and `setter_property` alternatives
- `property_name` now allows string literals and numeric literals (not just identifiers)
- `statement` gains `debugger_statement`

**Strict mode:** Handled semantically, not syntactically. The parser recognizes `"use strict"`
as a directive prologue (an expression statement containing a string literal at the start of
a function or program body), but the grammar itself doesn't change — restrictions are applied
as post-parse validation.

---

### ES2015 (ES6) — The Big One

ES2015 is the largest single expansion of JavaScript. It introduced classes, modules, arrow
functions, template literals, destructuring, generators, symbols, iterators, `let`/`const`,
default/rest parameters, computed properties, shorthand methods, `for-of`, and more.

This is where the grammar goes from "fits on one page" to "needs an index."

#### Tokens Added (delta from ES5)

**New operators:**

```
ARROW       = "=>"
ELLIPSIS    = "..."
```

**Template literal tokens:** Require pattern groups (Extension F04).

```
groups:
  template:
    TEMPLATE_MIDDLE = /\}([^`\\$]|\\.|\$[^{])*\$\{/
    TEMPLATE_TAIL   = /\}([^`\\$]|\\.|\$[^{])*`/

TEMPLATE_HEAD   = /`([^`\\$]|\\.|\$[^{])*\$\{/
TEMPLATE_NO_SUB = /`([^`\\$]|\\.|\$[^{])*`/
```

Backtick triggers group push via on-token callback. `${` pops to default group for the
expression. `}` at brace-depth 0 pushes template group back. This requires bracket depth
tracking (Extension 2).

**Keywords promoted from reserved:**

```
class const export extends import super let yield
```

**Context keywords** (identifiers that act as keywords in certain positions):

```
context_keywords:
  as from of get set static
```

**Numeric literals expanded:**

```
BINARY_NUMBER = /0[bB][01]+/
OCTAL_NUMBER  = /0[oO][0-7]+/
```

**Unicode escape expanded:** `\u{HHHHHH}` syntax (1-6 hex digits, supporting all of Unicode).

**Regex flag:** `u` (Unicode mode).

#### Grammar Rules Added (major categories)

**Classes:**

```
class_declaration = "class" NAME [ class_heritage ] class_body ;
class_expression = "class" [ NAME ] [ class_heritage ] class_body ;
class_heritage = "extends" left_hand_side_expression ;
class_body = LBRACE { class_element } RBRACE ;
class_element = [ "static" ] method_definition | SEMICOLON ;
method_definition = property_name LPAREN [ formal_parameters ] RPAREN LBRACE { source_element } RBRACE
                  | "get" property_name LPAREN RPAREN LBRACE { source_element } RBRACE
                  | "set" property_name LPAREN NAME RPAREN LBRACE { source_element } RBRACE
                  | STAR property_name LPAREN [ formal_parameters ] RPAREN LBRACE { source_element } RBRACE ;
```

**Modules:**

```
import_declaration = "import" import_clause from_clause SEMICOLON
                   | "import" module_specifier SEMICOLON ;
import_clause = default_import [ COMMA named_imports ]
              | named_imports
              | namespace_import ;
default_import = NAME ;
named_imports = LBRACE [ import_specifier { COMMA import_specifier } [ COMMA ] ] RBRACE ;
import_specifier = NAME [ "as" NAME ] ;
namespace_import = STAR "as" NAME ;
from_clause = "from" STRING ;
module_specifier = STRING ;

export_declaration = "export" "default" ( function_declaration | class_declaration | assignment_expression SEMICOLON )
                   | "export" ( function_declaration | class_declaration | lexical_declaration | variable_statement )
                   | "export" named_exports [ from_clause ] SEMICOLON ;
named_exports = LBRACE [ export_specifier { COMMA export_specifier } [ COMMA ] ] RBRACE ;
export_specifier = NAME [ "as" NAME ] ;
```

**Arrow functions:**

```
arrow_function = arrow_parameters ARROW concise_body ;
arrow_parameters = NAME | LPAREN [ formal_parameters ] RPAREN ;
concise_body = assignment_expression | LBRACE { source_element } RBRACE ;
```

Uses cover grammar / `&ARROW` lookahead to disambiguate from parenthesized expressions:

```
primary_expression = ... | arrow_function &ARROW | LPAREN expression RPAREN | ... ;
```

**Destructuring:**

```
binding_pattern = object_binding_pattern | array_binding_pattern ;
object_binding_pattern = LBRACE [ binding_property { COMMA binding_property } [ COMMA ] ] RBRACE ;
array_binding_pattern = LBRACKET [ binding_element { COMMA binding_element } [ COMMA ] ] RBRACKET ;
binding_property = property_name COLON binding_element | NAME [ initializer ] ;
binding_element = binding_pattern [ initializer ] | NAME [ initializer ] ;
rest_element = ELLIPSIS NAME ;
initializer = EQUALS assignment_expression ;
```

**Generators:**

```
generator_declaration = "function" STAR NAME LPAREN [ formal_parameters ] RPAREN LBRACE { source_element } RBRACE ;
generator_expression = "function" STAR [ NAME ] LPAREN [ formal_parameters ] RPAREN LBRACE { source_element } RBRACE ;
yield_expression = "yield" [ STAR ] assignment_expression ;
```

**Iterators:**

```
for_of_statement = "for" LPAREN ( "var" | "let" | "const" ) binding_element "of" expression RPAREN statement ;
```

**Template literals:**

```
template_literal = TEMPLATE_NO_SUB
                 | TEMPLATE_HEAD { template_span } TEMPLATE_TAIL ;
template_span = expression TEMPLATE_MIDDLE ;
tagged_template = member_expression template_literal ;
```

**Let/Const:**

```
lexical_declaration = ( "let" | "const" ) binding_list SEMICOLON ;
binding_list = lexical_binding { COMMA lexical_binding } ;
lexical_binding = ( NAME | binding_pattern ) [ initializer ] ;
```

**Other additions:**
- Computed property names: `[ expression ]` as `property_name` alternative
- Shorthand properties: `{ x }` as `property_assignment` alternative
- Shorthand methods: `{ method() {} }` in object literals
- Spread element: `ELLIPSIS assignment_expression` in arrays and call arguments
- Rest parameter: `ELLIPSIS NAME` as last formal parameter
- Default parameters: `NAME EQUALS assignment_expression` in formal parameter list
- `super` expressions in methods: `"super" DOT NAME` and `"super" arguments`
- `new.target` meta-property

---

### ES2016

The smallest annual release. One new operator.

#### Tokens Added

```
STAR_STAR        = "**"
STAR_STAR_EQUALS = "**="
```

Must appear before `STAR` and `STAR_EQUALS` in token file (first-match-wins).

#### Grammar Rules Added

```
# Exponentiation: right-associative, between unary and multiplicative
exponentiation_expression = unary_expression [ STAR_STAR exponentiation_expression ] ;
```

#### Grammar Rules Changed

- `multiplicative_expression` now delegates to `exponentiation_expression` instead of `unary_expression`
- `assignment_operator` gains `STAR_STAR_EQUALS`

---

### ES2017

#### Context Keywords Promoted

`async` and `await` become context keywords (they act as keywords in certain positions
but can still be used as identifiers in non-async contexts for backward compatibility).

#### Grammar Rules Added

```
async_function_declaration = "async" "function" NAME LPAREN [ formal_parameters ] RPAREN
                             LBRACE { source_element } RBRACE ;
async_function_expression = "async" "function" [ NAME ] LPAREN [ formal_parameters ] RPAREN
                            LBRACE { source_element } RBRACE ;
async_arrow_function = "async" arrow_parameters ARROW concise_body ;
async_method = "async" [ STAR ] property_name LPAREN [ formal_parameters ] RPAREN
               LBRACE { source_element } RBRACE ;
await_expression = "await" unary_expression ;
```

#### Grammar Rules Changed

- Trailing commas now allowed in function parameter lists and call arguments
  (already allowed in array/object literals since ES1)
- `class_element` gains `async_method`
- `unary_expression` gains `await_expression` (only valid inside async functions — semantic check)

---

### ES2018

#### Grammar Rules Added

```
# Async generators
async_generator_declaration = "async" "function" STAR NAME LPAREN [ formal_parameters ] RPAREN
                              LBRACE { source_element } RBRACE ;
async_generator_expression = "async" "function" STAR [ NAME ] LPAREN [ formal_parameters ] RPAREN
                             LBRACE { source_element } RBRACE ;

# for-await-of
for_await_of_statement = "for" "await" LPAREN ( "var" | "let" | "const" ) binding_element "of"
                         expression RPAREN statement ;

# Object rest/spread
object_rest_property = ELLIPSIS NAME ;           # in destructuring
object_spread_property = ELLIPSIS assignment_expression ;  # in object literals
```

#### Regex Improvements (lexer-internal)

These are changes to what the REGEX token pattern accepts, not new token types:

- `s` flag (dotAll — `.` matches newlines)
- Named capture groups: `(?<name>...)`
- Lookbehind assertions: `(?<=...)` and `(?<!...)`
- Unicode property escapes: `\p{Script=Greek}`

---

### ES2019

#### Grammar Rules Changed

```
# Catch binding is now optional
catch_clause = "catch" [ LPAREN NAME RPAREN ] block ;
```

#### Token Changes

- String literals now allow U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH SEPARATOR)
  directly. This is a lexer-level change — the STRING regex must be updated to permit
  these characters.

---

### ES2020

#### Tokens Added

```
OPTIONAL_CHAIN    = "?."
NULLISH_COALESCE  = "??"
```

`?.` must appear before `?` in the token file. `??` must appear before `?` as well.

#### Grammar Rules Added

```
# Optional chaining
optional_chain_expression = member_expression OPTIONAL_CHAIN NAME
                          | member_expression OPTIONAL_CHAIN LBRACKET expression RBRACKET
                          | member_expression OPTIONAL_CHAIN arguments ;

# Nullish coalescing
nullish_coalescing_expression = logical_or_expression { NULLISH_COALESCE logical_or_expression } ;
```

Note: `??` cannot be mixed with `&&` or `||` without parentheses — this is a semantic/syntax
error, enforced as a post-parse check.

```
# BigInt literal
BIGINT = /[0-9]+n/          # decimal BigInt
       | /0[xX][0-9a-fA-F]+n/  # hex BigInt
       | /0[bB][01]+n/         # binary BigInt
       | /0[oO][0-7]+n/        # octal BigInt
```

BIGINT tokens must appear before NUMBER tokens (first-match-wins, the `n` suffix
distinguishes them).

```
# Dynamic import
dynamic_import = "import" LPAREN assignment_expression RPAREN ;

# import.meta
import_meta = "import" DOT "meta" ;
```

#### Grammar Rules Changed

- `primary_expression` gains `BIGINT`, `dynamic_import`, `import_meta`
- Expression precedence chain gains `nullish_coalescing_expression` and `optional_chain_expression`

---

### ES2021

#### Tokens Added

```
OR_OR_EQUALS      = "||="
AND_AND_EQUALS    = "&&="
NULLISH_COALESCE_EQUALS = "??="
```

#### Token Changes

Numeric literals now allow `_` as a separator between digits:

```
NUMBER = /[0-9]([0-9_]*[0-9])?(\.[0-9]([0-9_]*[0-9])?)?([eE][+-]?[0-9]([0-9_]*[0-9])?)?/
```

Rules: No leading `_`, no trailing `_`, no consecutive `__`, no `_` adjacent to `.` or `e/E`.

#### Grammar Rules Changed

- `assignment_operator` gains `||=`, `&&=`, `??=`

---

### ES2022

#### Tokens Added

```
PRIVATE_NAME = /#[a-zA-Z_$][a-zA-Z0-9_$]*/
```

`PRIVATE_NAME` is context-sensitive: `#name` is a single token inside class bodies. The
`#` prefix is consumed as part of the identifier, not as a separate operator.

**Regex flag added:** `d` (hasIndices — provides start/end indices for captures).

#### Grammar Rules Added

```
# Class fields
class_field_declaration = [ "static" ] ( property_name | PRIVATE_NAME ) [ initializer ] SEMICOLON ;

# Private methods
private_method_definition = [ "static" ] PRIVATE_NAME LPAREN [ formal_parameters ] RPAREN
                            LBRACE { source_element } RBRACE ;

# Private member access
private_member_expression = member_expression DOT PRIVATE_NAME ;

# Static initialization blocks
static_block = "static" LBRACE { statement } RBRACE ;
```

#### Grammar Rules Changed

- `class_element` gains `class_field_declaration`, `private_method_definition`, `static_block`
- `member_expression` gains `DOT PRIVATE_NAME` alternative
- Top-level `await` is valid at module scope (semantic change — `await_expression` can appear
  outside async functions when the source is parsed as a module)

---

### ES2023

#### Tokens Added

```
HASHBANG = /^#![^\n]*/
```

The hashbang (`#!`) is only valid at byte position 0 of the source. Best handled as a
pre-tokenize hook or special first-token check.

#### Grammar Rules Changed

```
program = [ HASHBANG ] { source_element } ;
```

No new operators or expression forms. The runtime additions (findLast, change-array-by-copy)
are API-only, not syntax.

---

### ES2024

#### Token Changes

**Regex flag added:** `v` (unicodeSets — extended character class syntax with set operations
like `[A--B]` for difference and `[A&&B]` for intersection).

This is internal to the REGEX token pattern. No new grammar productions.

---

### ES2025

#### Tokens Added

```
AT = "@"   # decorator prefix
```

#### Context Keywords Added

```
context_keywords:
  ... (all previous) ...
  using
```

#### Grammar Rules Added

```
# Decorators
decorator = AT left_hand_side_expression ;
decorated_class_declaration = { decorator } class_declaration ;
decorated_class_element = { decorator } class_element ;

# Import attributes
import_attributes = "with" LBRACE attribute_list RBRACE ;
attribute_list = import_attribute { COMMA import_attribute } [ COMMA ] ;
import_attribute = ( NAME | STRING ) COLON STRING ;

# Explicit resource management
using_declaration = "using" binding_list SEMICOLON ;
await_using_declaration = "await" "using" binding_list SEMICOLON ;
```

#### Grammar Rules Changed

- `import_declaration` gains optional `import_attributes` at the end
- `export_declaration` (re-exports) gains optional `import_attributes`
- `class_declaration` can be preceded by decorators
- `class_element` can be preceded by decorators
- `statement` gains `using_declaration` and `await_using_declaration`
- `for_of_statement` accepts `"using"` and `"await" "using"` as declaration heads

---

## 3. TypeScript Version Feature Inventory

Each TypeScript grammar is self-contained: full ES baseline grammar plus all TS-specific
syntax for that version.

### TS 1.0 (2014) — ES5 + Type System

**ES Baseline:** ES5

TypeScript 1.0 introduced a structural type system on top of ES5. The core innovation is
that type annotations are syntactically optional — every valid JavaScript program is also
a valid TypeScript program.

#### Keywords Added (on top of ES5)

```
keywords:
  ... (all ES5 keywords) ...
  interface type enum namespace module declare readonly
  implements extends any void never unknown
  number string boolean object symbol
  public private protected abstract
  as is keyof
```

#### Grammar Rules Added

**Type annotations:**

```
type_annotation = COLON type ;

type = union_type ;
union_type = intersection_type { PIPE intersection_type } ;
intersection_type = primary_type { AMPERSAND primary_type } ;
primary_type = type_reference
             | array_type
             | tuple_type
             | function_type
             | constructor_type
             | object_type
             | parenthesized_type
             | type_literal
             | type_query
             | "void" | "any" | "never" | "unknown"
             | "number" | "string" | "boolean" | "object" | "symbol"
             | "null" | "undefined" ;

type_reference = NAME [ type_arguments ] ;
array_type = primary_type LBRACKET RBRACKET ;
tuple_type = LBRACKET [ type { COMMA type } ] RBRACKET ;
function_type = LPAREN [ parameter_list ] RPAREN ARROW type ;
constructor_type = "new" LPAREN [ parameter_list ] RPAREN ARROW type ;
object_type = LBRACE [ type_member { ( SEMICOLON | COMMA ) type_member } ] RBRACE ;
parenthesized_type = LPAREN type RPAREN ;
type_literal = STRING | NUMBER | "true" | "false" ;
type_query = "typeof" NAME ;
```

**Generics:**

```
type_parameters = LESS_THAN type_parameter { COMMA type_parameter } GREATER_THAN ;
type_parameter = NAME [ "extends" type ] [ EQUALS type ] ;
type_arguments = LESS_THAN type { COMMA type } GREATER_THAN ;
```

Generics vs comparison disambiguation: `<` after a type name or function name is generic;
otherwise comparison. Uses `previousToken()` heuristic.

**Interfaces:**

```
interface_declaration = "interface" NAME [ type_parameters ] [ interface_extends ] object_type ;
interface_extends = "extends" type_reference { COMMA type_reference } ;
```

**Enums:**

```
enum_declaration = [ "const" ] "enum" NAME LBRACE [ enum_member { COMMA enum_member } [ COMMA ] ] RBRACE ;
enum_member = property_name [ EQUALS assignment_expression ] ;
```

**Type aliases:**

```
type_alias_declaration = "type" NAME [ type_parameters ] EQUALS type SEMICOLON ;
```

**Ambient declarations:**

```
declare_function = "declare" function_declaration ;
declare_variable = "declare" variable_statement ;
declare_class = "declare" class_declaration ;
declare_module = "declare" "module" ( NAME | STRING ) LBRACE { source_element } RBRACE ;
declare_enum = "declare" enum_declaration ;
```

**Type assertions:**

```
angle_bracket_assertion = LESS_THAN type GREATER_THAN unary_expression ;
as_expression = expression "as" type ;
```

**Access modifiers on parameters and members:**

```
parameter_property = ( "public" | "private" | "protected" ) [ "readonly" ] NAME [ type_annotation ] ;
```

**Optional properties and parameters:**

```
optional_property = NAME QUESTION type_annotation ;
optional_parameter = NAME QUESTION [ type_annotation ] ;
```

---

### TS 2.0 (2016) — ES2015 + Advanced Types

**ES Baseline:** ES2015

#### Keywords Added

```
never readonly
```

(These may already be in TS 1.0's keyword list depending on when they were formalized.)

#### Grammar Rules Added

```
# Non-null assertion
non_null_expression = left_hand_side_expression BANG ;

# Type guards
type_predicate = NAME "is" type ;  # as function return type

# Abstract classes and methods
abstract_class_declaration = "abstract" "class" NAME [ class_heritage ] class_body ;
abstract_method = "abstract" method_signature SEMICOLON ;

# String/number literal types (already partially in 1.0, now formalized)
string_literal_type = STRING ;
numeric_literal_type = NUMBER ;

# this type
this_type = "this" ;  # as a type reference
```

---

### TS 3.0 (2018) — ES2018 + Conditional Types

**ES Baseline:** ES2018

#### Keywords Added

```
unknown infer
```

#### Grammar Rules Added

```
# Conditional types
conditional_type = type "extends" type QUESTION type COLON type ;

# infer keyword in conditional types
infer_type = "infer" NAME ;

# Tuple rest and optional elements
tuple_rest_element = ELLIPSIS type ;
tuple_optional_element = type QUESTION ;

# Mapped type modifiers
mapped_type = LBRACE [ "readonly" | PLUS "readonly" | MINUS "readonly" ]
              LBRACKET NAME "in" type RBRACKET [ QUESTION | PLUS QUESTION | MINUS QUESTION ]
              COLON type RBRACE ;
```

---

### TS 4.0 (2020) — ES2020 + Variadic Tuples

**ES Baseline:** ES2020

#### Grammar Rules Added

```
# Variadic tuple elements
variadic_tuple_element = ELLIPSIS type ;  # ...T where T is a tuple type

# Labeled tuple elements
labeled_tuple_element = NAME COLON type ;
labeled_optional_tuple = NAME QUESTION COLON type ;
labeled_rest_tuple = ELLIPSIS NAME COLON type ;

# Template literal types
template_literal_type = TEMPLATE_NO_SUB
                      | TEMPLATE_HEAD { type TEMPLATE_MIDDLE } type TEMPLATE_TAIL ;
```

---

### TS 5.0 (2023) — ES2022 + Decorators + satisfies

**ES Baseline:** ES2022

#### Context Keywords Added

```
satisfies
```

#### Grammar Rules Added

```
# ES-standard decorators (same as ES2025 but available earlier in TS)
decorator = AT left_hand_side_expression ;
decorated_class_declaration = { decorator } class_declaration ;
decorated_class_element = { decorator } class_element ;

# satisfies operator
satisfies_expression = expression "satisfies" type ;

# const type parameters
const_type_parameter = "const" NAME [ "extends" type ] ;

# auto-accessor fields
accessor_field = [ "static" ] "accessor" ( NAME | PRIVATE_NAME ) [ type_annotation ] [ initializer ] SEMICOLON ;
```

---

### TS 5.8 (2025) — ES2025 + Latest

**ES Baseline:** ES2025

Incorporates all ES2025 additions (decorators, import attributes, `using`/`await using`)
plus TypeScript's own latest features.

#### Grammar Rules Added

```
# All ES2025 rules (import_attributes, using_declaration, await_using_declaration)
# are included in the full grammar.

# No major new TS-specific syntax beyond what's in TS 5.0 + ES2025.
# Focus is on refinements to existing features.
```

---

## 4. Token Categories Summary Table

| Category | ES1 | ES3 | ES5 | ES2015 | ES2020 | ES2022 | ES2025 |
|----------|-----|-----|-----|--------|--------|--------|--------|
| Identifiers | `[a-zA-Z_$][\w$]*` | same | same | + `\u{...}` | same | same | same |
| Numbers | dec, hex, float | same | same | + `0b`, `0o` | + BigInt `n` | + separators `_` (ES2021) | same |
| Strings | `"..."` `'...'` | same | + line cont. | + template `` ` `` | same | same | same |
| Regex | none (impl-defined) | formal `/pattern/flags` | same | + `u` flag | same | + `d` flag | + `v` flag |
| Operators | 46 | + `===` `!==` | same | + `=>` `...` | + `?.` `??` | same | + `@` |
| Exponentiation | none | none | none | none | none (ES2016) | same | same |
| Logical assign | none | none | none | none | none | `\|\|=` `&&=` `??=` (ES2021) | same |
| Keywords | 26 | + 5 | + `debugger` | + 8 (class, let, etc.) | same | same | same |
| Context kw | none | none | none | as, from, of, get, set, static | same | same | + using |
| Template | none | none | none | HEAD, MIDDLE, TAIL, NO_SUB | same | same | same |
| Private names | none | none | none | none | none | `#name` | same |
| Hashbang | none | none | none | none | none | none (ES2023) | same |

---

## 5. Grammar Rule Categories Summary Table

| Category | ES1 | ES3 | ES5 | ES2015 | ES2017 | ES2020 | ES2022 | ES2025 |
|----------|-----|-----|-----|--------|--------|--------|--------|--------|
| Program | source elements | same | same | + module goal | same | same | + top-level await | + hashbang (ES2023) |
| Declarations | var, function | same | same | + let, const, class, generator, import, export | + async function | same | same | + using, decorated class |
| Statements | if, while, for, for-in, do-while, switch, with, labeled, break, continue, return | + try/catch/throw | + debugger | + for-of | same | same | same | same |
| Expressions | assignment, conditional, binary, unary, call, member, new, primary | + regex literal | + getters/setters | + arrow, template, yield, spread, destructuring | + await | + `?.`, `??`, dynamic import, import.meta, BigInt | + private member | + decorator |
| Classes | n/a | n/a | n/a | full | + async methods | same | + fields, static blocks, private | + decorators |
| Modules | n/a | n/a | n/a | import, export | same | + dynamic import() | same | + import attributes |
| Error handling | n/a | try/catch/finally/throw | same | same | same | same | same | same |

---

## 6. Lexer Implementation Notes

### Context-Sensitive Lexing

**Regex vs Division (ES3+):**

The `/` character is the most famous ambiguity in JavaScript lexing. It requires
`previousToken()` (Extension 1 from `lexer-parser-extensions.md`):

```
# After these tokens, / starts a regex:
REGEX_PREDECESSOR = { EQUALS, LPAREN, LBRACKET, LBRACE, COMMA, SEMICOLON,
                      COLON, QUESTION, BANG, AMPERSAND, PIPE, CARET, TILDE,
                      PLUS, MINUS, STAR, SLASH, PERCENT, LESS_THAN,
                      GREATER_THAN, RETURN, TYPEOF, VOID, DELETE, NEW,
                      IN, INSTANCEOF, CASE, THROW }

# After these tokens, / is division:
DIVISION_PREDECESSOR = { RPAREN, RBRACKET, NAME, NUMBER, STRING,
                         PLUS_PLUS, MINUS_MINUS, TEMPLATE_TAIL,
                         TEMPLATE_NO_SUB }
```

**Template Literals (ES2015+):**

Require pattern group switching and bracket depth tracking:

1. Backtick (`` ` ``) → push `template` group
2. `${` inside template → pop to `default` group, increment brace depth
3. `}` at brace-depth 0 → push `template` group back
4. Closing backtick → pop `template` group

This is specified in `F04-lexer-pattern-groups.md`.

**Generics vs Comparison (TypeScript):**

`<` after a type name starts a generic type argument list; otherwise it's a comparison
operator. The heuristic uses `previousToken()`:

- Generic after: `NAME`, `GREATER_THAN` (closing `>`), `COMMA` (inside `<>`)
- Comparison otherwise

More sophisticated approaches use cover grammars or backtracking.

**Private Names (ES2022+):**

`#name` is tokenized as a single `PRIVATE_NAME` token. The `#` is not a separate operator.

**Hashbang (ES2023+):**

`#!` is only valid at byte offset 0. Implement as a pre-tokenize hook that checks the
first two bytes. If `#!`, consume until newline and emit `HASHBANG`.

### Automatic Semicolon Insertion (ASI)

ASI is one of JavaScript's most complex features. It applies to all versions from ES1 onward.

**Rules:**
1. When the parser encounters a token that is not allowed by any production, AND the
   offending token is separated from the previous token by at least one newline, THEN
   a semicolon is automatically inserted before the offending token.
2. When the end of input is reached and the parser cannot parse the input as a complete
   program, a semicolon is automatically inserted at the end.
3. Certain tokens are "restricted" — no newline is allowed between them and the following
   token. If a newline occurs, a semicolon is inserted.

**Restricted productions:**
```
postfix_expression:  expr [no newline] ++/--
return_statement:    return [no newline] expression
throw_statement:     throw [no newline] expression    (ES3+)
continue_statement:  continue [no newline] label
break_statement:     break [no newline] label
yield_expression:    yield [no newline] expression    (ES2015+)
arrow_function:      params [no newline] =>           (ES2015+)
```

In the grammar, these use `!NEWLINE` negative lookahead (Extension 4):

```
postfix_expression = left_hand_side_expression [ !NEWLINE ( PLUS_PLUS | MINUS_MINUS ) ] ;
return_statement = "return" [ !NEWLINE expression ] SEMICOLON ;
```

### Skip Patterns

All versions share the same skip patterns:

```
skip:
  WHITESPACE    = /[ \t\r\n\v\f\u00A0\uFEFF]+/
  LINE_COMMENT  = /\/\/[^\n]*/
  BLOCK_COMMENT = /\/\*[\s\S]*?\*\//
```

(In ES2019+, the whitespace pattern should also include U+2028 and U+2029 explicitly if
they are no longer treated as line terminators in string contexts.)

---

## 7. Parser Implementation Notes

### No Left Recursion

The parser uses PEG semantics with packrat memoization (see `grammar-format.md`). Left
recursion is forbidden. Operator precedence uses iterative form:

```
# Instead of:  expr = expr "+" term ;          (LEFT RECURSIVE - FORBIDDEN)
# Use:         expr = term { "+" term } ;      (ITERATIVE - OK)

additive_expression = multiplicative_expression { ( PLUS | MINUS ) multiplicative_expression } ;
```

### Cover Grammars

Arrow functions use positive lookahead to disambiguate from parenthesized expressions:

```
# The parser tries arrow_function first; if it sees ARROW after the
# parameter list, it commits. Otherwise it falls back to parenthesized_expression.
primary_expression = arrow_function
                   | LPAREN expression RPAREN
                   | ... ;
```

With PEG ordered choice (`|`), the arrow alternative is tried first. If the `ARROW` token
does not appear, PEG backtracks and tries the next alternative.

### ASI as Negative Lookahead

Restricted productions use `!NEWLINE` to express "no newline here":

```
postfix_expression = left_hand_side_expression [ !NEWLINE ( PLUS_PLUS | MINUS_MINUS ) ] ;
```

This means: "optionally match `++` or `--`, but only if there is no newline between
the previous token and the operator." The `!NEWLINE` predicate checks the
`TOKEN_PRECEDED_BY_NEWLINE` flag (Extension 3).

### Expression Precedence Chain

The full ES2025 precedence chain, from lowest to highest:

```
comma_expression
  assignment_expression            (= += -= *= /= %= **= <<= >>= >>>= &= |= ^= &&= ||= ??=)
    conditional_expression         (? :)
      nullish_coalescing_expression  (??)
        logical_or_expression      (||)
          logical_and_expression   (&&)
            bitwise_or_expression  (|)
              bitwise_xor_expression (^)
                bitwise_and_expression (&)
                  equality_expression (== != === !==)
                    relational_expression (< > <= >= instanceof in)
                      shift_expression (<< >> >>>)
                        additive_expression (+ -)
                          multiplicative_expression (* / %)
                            exponentiation_expression (**)   [ES2016+, right-associative]
                              unary_expression (delete void typeof + - ~ ! await)
                                postfix_expression (++ --)
                                  optional_chain_expression (?.)   [ES2020+]
                                    call_expression
                                      member_expression (. [] `` )
                                        new_expression
                                          primary_expression
```

---

## 8. Verification Strategy

### Per-Version Test Fixtures

```
code/fixtures/ecmascript/
  es1/
    accept/       # Programs that MUST parse successfully
      variables.js
      functions.js
      expressions.js
      control_flow.js
    reject/       # Programs that MUST fail (features from later versions)
      strict_equality.js        # === is ES3
      try_catch.js              # try/catch is ES3
      class_declaration.js      # class is ES2015
      arrow_function.js         # => is ES2015
      optional_chaining.js      # ?. is ES2020
  es3/
    accept/
      strict_equality.js        # now valid
      try_catch.js              # now valid
      regex_literal.js
    reject/
      class_declaration.js      # still ES2015
      arrow_function.js         # still ES2015
  ...
```

Similarly for TypeScript under `code/fixtures/typescript/`.

### Test Categories

| Category | Description |
|----------|-------------|
| **Keyword reservation** | `class` is a valid identifier in ES1, reserved in ES3, keyword in ES2015 |
| **Operator parsing** | `a ** b * c` parses as `(a ** b) * c` in ES2016+ |
| **Statement parsing** | `for (const x of y) {}` valid in ES2015+ only |
| **Expression parsing** | `a?.b?.c` chains correctly in ES2020+ only |
| **Cross-version forward** | Accept files from version N also parse with all versions > N |
| **Cross-version backward** | Reject files for version N+1 fail on version N and earlier |
| **Grammar validation** | `grammar-tools validate` passes for every `.tokens`/`.grammar` pair |
| **AST snapshot** | AST output captured and compared against baselines |
| **Differential testing** | Compare parse results against Babel/Acorn/tsc for reference conformance |

### Grammar Validation

Every `.tokens`/`.grammar` pair must pass `grammar_tools` cross-validation:

```python
from grammar_tools import parse_token_grammar, parse_parser_grammar, cross_validate

tokens = parse_token_grammar(open("es2020.tokens").read())
grammar = parse_parser_grammar(open("es2020.grammar").read())
cross_validate(tokens, grammar)  # raises on mismatch
```

---

## 9. Implementation Sequence

### Phase 1 — Foundation (ES1, ES3, ES5)

Build the three versions that defined "classic JavaScript." These establish the base grammar
patterns that all later versions build on.

- ES1: The minimal core — 26 keywords, no regex, no try/catch
- ES3: Adds regex, error handling, strict equality — this is "real" JavaScript
- ES5: Adds getters/setters, debugger, strict mode hooks

### Phase 2 — ES2015

The largest single grammar. Template literals require pattern group support. Arrow functions
require cover grammar / lookahead. Modules introduce a second parsing goal. Destructuring
adds significant grammar complexity.

### Phase 3 — ES2016 through ES2020

Incremental additions per year. Each grammar is a copy of the previous with additions:

- ES2016: One operator (`**`)
- ES2017: async/await
- ES2018: Async generators, rest/spread properties
- ES2019: Optional catch binding
- ES2020: `?.`, `??`, BigInt, dynamic import

### Phase 4 — ES2021 through ES2025

- ES2021: Logical assignment, numeric separators
- ES2022: Class fields, private names, static blocks
- ES2023: Hashbang
- ES2024: Regex `/v` flag
- ES2025: Decorators, import attributes, `using`

### Phase 5 — TypeScript

Each TS version incorporates its ES baseline and adds type system syntax:

- TS 1.0: Type annotations, interfaces, enums, generics
- TS 2.0: Non-null assertion, abstract classes, type guards
- TS 3.0: Conditional types, `unknown`, `infer`
- TS 4.0: Variadic tuples, template literal types
- TS 5.0: Decorators, `satisfies`, const type params
- TS 5.8: ES2025 integration

### Phase 6 — Integration

- Compiled grammar generation for all versions
- Version selection API: `create_lexer("es2020")`, `create_parser("ts5.0")`
- Differential testing against reference implementations
- Documentation and examples

---

## 10. Relationship to Existing Specs

| Spec | Relationship |
|------|-------------|
| `tokens-format.md` | Defines the `.tokens` file format used by all grammar files |
| `grammar-format.md` | Defines the `.grammar` file format (PEG/EBNF, no left recursion) |
| `lexer-parser-extensions.md` | Extensions needed for JS/TS: lookahead, lookbehind, context keywords, bracket depth |
| `F04-lexer-pattern-groups.md` | Pattern groups and callbacks needed for template literal lexing |
| `compiled-grammars.md` | How grammars are compiled to language-native source |
| `browser-compatible-grammars.md` | Browser-friendly compiled grammar imports |
| `lexer-parser-hooks.md` | Pre/post transform hooks for ASI preprocessing |

---

## 11. Open Design Questions

1. **ES2 and ES5.1:** Symlinks to ES1/ES5, omitted entirely, or separate files with
   equivalence comments?
2. **Strict mode:** Separate grammar files (`es5-strict.tokens`) or purely semantic handling?
3. **Module vs Script goal:** Separate grammar files (`es2015-module.grammar`) or single file
   with parameterized entry rule (`program` vs `module`)?
4. **Regex internal grammar:** Sub-grammar for regex pattern syntax, or opaque single token?
5. **JSX:** Separate grammar set (`es2015-jsx.tokens`, `es2015-jsx.grammar`) or out of scope?
6. **TypeScript experimental decorators:** Reflect TS's legacy decorator syntax in TS 1.0-4.0,
   or only include ES-standard decorators starting in TS 5.0?
7. **TypeScript minor versions:** Cover only major (1.0, 2.0, ...) or include significant minors
   (4.1 for template literal types, 4.9 for `satisfies`)?

# Lattice — A CSS Superset Transpiler

## Overview

Lattice is a CSS superset language that compiles to plain CSS. Any valid CSS is
valid Lattice — the language adds variables, mixins, control flow, functions,
and modules on top of standard CSS syntax.

**Primary goal:** Stress-test the AST transformation pipeline. The lexer and
parser infrastructure has been battle-tested across 11 grammars and 6 languages,
but no package has ever transformed an AST back into source text. Lattice
exercises the full pipeline: Source → Lex → Parse → AST Transform → CSS Emit.

**Design principle:** Grammar-first, not syntax-sugar-first. Every construct is
either standard CSS or an unambiguous extension that a context-free grammar can
parse without backtracking. This is what Sass and Less got wrong — they were
designed for developer ergonomics first, with no thought for formal parseability.

**What problems does Lattice solve?**
- **No variables in CSS** (pre custom properties) — repeat colors, sizes everywhere
- **No reuse** — copy-paste patterns across rules (vendor prefixes, clearfix, etc.)
- **No logic** — can't conditionally include rules based on a theme or breakpoint
- **No functions** — can't compute derived values (spacing scales, column widths)
- **No modularity** — monolithic stylesheets with no encapsulation

## Language Design

### Why This Grammar Is Context-Free

Every Lattice construct starts with an unambiguous leading token:

| Construct | Leading Token(s) | Collision with CSS? |
|-----------|------------------|---------------------|
| `$var: value;` | `VARIABLE` (`$`) | No — CSS never uses `$` |
| `@mixin name(...)` | `AT_KEYWORD("@mixin")` | No — unknown at-rules are valid CSS |
| `@include name(...)` | `AT_KEYWORD("@include")` | No — same reason |
| `@if expr { }` | `AT_KEYWORD("@if")` | No — same reason |
| `@for $i from...` | `AT_KEYWORD("@for")` | No — same reason |
| `@each $v in...` | `AT_KEYWORD("@each")` | No — same reason |
| `@function name(...)` | `AT_KEYWORD("@function")` | No — same reason |
| `@return expr;` | `AT_KEYWORD("@return")` | No — same reason |
| `@use "path";` | `AT_KEYWORD("@use")` | No — same reason |

CSS's at-rule syntax (`@ IDENT prelude (block | ;)`) is extensible by design.
The CSS spec says unknown at-rules should be parsed using the general at-rule
grammar and ignored. Lattice gives specific at-keywords specific semantics.

Literal matching on AT_KEYWORD text (e.g., `"@mixin"` matches an `AT_KEYWORD`
token whose `.value` is `@mixin`) is already proven by `BANG "important"` in
the CSS grammar. The grammar parser tries `lattice_rule` before `at_rule` in
alternation, so Lattice at-keywords are consumed before falling through to
generic CSS at-rules.

### No Division Operator

CSS uses `/` as a separator in shorthand properties (`font: 12px/1.5`). Sass
originally overloaded `/` for division, causing years of ambiguity bugs and
eventually a breaking change to `math.div()`. Lattice avoids this entirely:
- Use `calc()` for arithmetic in value positions: `calc(100% / 3)`
- Use arithmetic expressions in `@if`, `@for`, `@return` contexts where
  `/` has no CSS meaning (future consideration)
- Use a `math-div()` function for explicit division

### No String Interpolation

Sass uses `#{$var}` to interpolate variables inside strings and selectors.
This requires modal lexing (switch token rules mid-string) and makes the
grammar context-sensitive. Lattice avoids this:
- Variables in value positions: `color: $brand-color;` — the variable
  reference is a discrete token, not interpolated into a string
- For dynamic selectors: use `@each` to generate rules, not interpolation

## Token Design (`lattice.tokens`)

The Lattice token file is a standalone copy of all CSS token definitions plus
5 new tokens. The existing `css.tokens` is not modified.

### New Tokens

| Token | Pattern | Purpose | Position |
|-------|---------|---------|----------|
| `VARIABLE` | `/\$[a-zA-Z_][a-zA-Z0-9_-]*/` | Variable references (`$color`) | After STRING, before DIMENSION |
| `EQUALS_EQUALS` | `"=="` | Equality comparison | Before single-char operators |
| `NOT_EQUALS` | `"!="` | Inequality comparison | Before single-char operators |
| `GREATER_EQUALS` | `">="` | Greater-or-equal comparison | Before single-char operators |
| `LESS_EQUALS` | `"<="` | Less-or-equal comparison | Before single-char operators |

### Priority Ordering for VARIABLE

`VARIABLE` (`/\$[a-zA-Z_].../`) must come before `DOLLAR_EQUALS` (`$=`) in
the token file. Both start with `$`, but:
- `$color` starts with `$` + letter → matches VARIABLE
- `$=` starts with `$` + `=` → does NOT match VARIABLE (requires letter/underscore)

Since VARIABLE is listed first and the lexer uses first-match-wins, `$color`
is tokenized as VARIABLE. `$=` fails the VARIABLE regex (no letter after `$`)
and falls through to DOLLAR_EQUALS. No ambiguity.

### Priority Ordering for Comparison Operators

`EQUALS_EQUALS` (`==`) must come before `EQUALS` (`=`). `NOT_EQUALS` (`!=`)
must come before `BANG` (`!`). `GREATER_EQUALS` (`>=`) must come before
`GREATER` (`>`). `LESS_EQUALS` (`<=`) must come before `EQUALS` (`=`).
This follows the same pattern as `COLON_COLON` before `COLON`.

## Grammar Design (`lattice.grammar`)

The Lattice grammar is a standalone copy of all CSS grammar rules plus ~20
new rules. The existing `css.grammar` is not modified.

### Structural Changes to CSS Rules

Three CSS rules gain Lattice alternatives (tried first via alternation):

```
rule       = lattice_rule | at_rule | qualified_rule ;
block_item = lattice_block_item | at_rule | declaration_or_nested ;
value      = ... | VARIABLE | ... ;  (VARIABLE added to alternatives)
```

### New Grammar Rules

#### Variables

```ebnf
variable_declaration = VARIABLE COLON value_list SEMICOLON ;
```

Variables appear in value positions as `VARIABLE` tokens (added to `value`
and `function_arg` alternatives).

#### Mixins

```ebnf
mixin_definition  = "@mixin" IDENT LPAREN [ mixin_params ] RPAREN block ;
mixin_params      = mixin_param { COMMA mixin_param } ;
mixin_param       = VARIABLE [ COLON value_list ] ;
include_directive = "@include" IDENT [ LPAREN include_args RPAREN ] ( SEMICOLON | block ) ;
include_args      = value_list { COMMA value_list } ;
```

Mixins with no parameters: `@mixin clearfix() { ... }`
Mixins with defaults: `@mixin button($bg, $fg: white) { ... }`
Include with content block: `@include responsive() { font-size: 18px; }`

#### Control Flow

```ebnf
lattice_control  = if_directive | for_directive | each_directive ;
if_directive     = "@if" lattice_expression block
                   { "@else" "if" lattice_expression block }
                   [ "@else" block ] ;
for_directive    = "@for" VARIABLE "from" lattice_expression
                   ( "through" | "to" ) lattice_expression block ;
each_directive   = "@each" VARIABLE { COMMA VARIABLE } "in"
                   each_list block ;
each_list        = value { COMMA value } ;
```

`@else if` is two tokens: `AT_KEYWORD("@else")` then `IDENT("if")`. The
grammar uses two literal matches in sequence.

`"through"` is inclusive (`1 through 3` → 1, 2, 3). `"to"` is exclusive
(`1 to 3` → 1, 2).

#### Expressions

Used in `@if` conditions, `@for` bounds, and `@return` values:

```ebnf
lattice_expression      = lattice_or_expr ;
lattice_or_expr         = lattice_and_expr { "or" lattice_and_expr } ;
lattice_and_expr        = lattice_comparison { "and" lattice_comparison } ;
lattice_comparison      = lattice_additive [ comparison_op lattice_additive ] ;
comparison_op           = EQUALS_EQUALS | NOT_EQUALS | GREATER
                        | GREATER_EQUALS | LESS_EQUALS ;
lattice_additive        = lattice_multiplicative
                          { ( PLUS | MINUS ) lattice_multiplicative } ;
lattice_multiplicative  = lattice_unary { STAR lattice_unary } ;
lattice_unary           = MINUS lattice_unary | lattice_primary ;
lattice_primary         = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
                        | STRING | IDENT | HASH
                        | "true" | "false" | "null"
                        | function_call
                        | LPAREN lattice_expression RPAREN ;
```

Operator precedence (tightest to loosest):
1. Unary minus (`-`)
2. Multiplication (`*`)
3. Addition/subtraction (`+`, `-`)
4. Comparison (`==`, `!=`, `>`, `>=`, `<=`)
5. Logical AND (`and`)
6. Logical OR (`or`)

#### Functions

```ebnf
function_definition = "@function" IDENT LPAREN [ mixin_params ] RPAREN
                      function_body ;
function_body       = LBRACE { function_body_item } RBRACE ;
function_body_item  = variable_declaration | return_directive | lattice_control ;
return_directive    = "@return" lattice_expression SEMICOLON ;
```

Functions share `mixin_params` for parameter syntax (both use `$param` with
optional `: default`).

#### Modules

```ebnf
use_directive = "@use" STRING [ "as" IDENT ] SEMICOLON ;
```

`@use "colors"` loads `./colors.lattice` and makes its exports available.
`@use "colors" as c` namespaces them under `c`.

## Transpiler Pipeline

```
Source Text (.lattice)
    │
    ▼
┌─────────┐    lattice.tokens
│  Lexer   │◄──────────────────
└────┬────┘
     │ Token stream
     ▼
┌─────────┐    lattice.grammar
│  Parser  │◄──────────────────
└────┬────┘
     │ AST (ASTNode tree with Lattice + CSS nodes)
     ▼
┌─────────────┐
│ Transformer │   Pass 1: Module resolution (@use)
│  (3 passes) │   Pass 2: Symbol collection (variables, mixins, functions)
│             │   Pass 3: Expansion (substitute, expand, evaluate)
└──────┬──────┘
       │ Clean CSS AST (no Lattice nodes remain)
       ▼
┌─────────┐
│ Emitter │   Walks AST, reconstructs CSS text
└────┬────┘
     │
     ▼
CSS Text (.css)
```

### Pass 1: Module Resolution

Walk top-level rules. For each `use_directive`:
1. Resolve the module path (relative to source file, `.lattice` extension)
2. Parse the module (recursively resolving its own `@use` directives)
3. Collect the module's exported variables, mixins, and functions
4. Register them in a module registry keyed by namespace
5. Remove the `use_directive` node from the AST

Cycle detection: track the set of files currently being loaded. If a file
appears twice, raise `LatticeError("Circular @use: a.lattice → b.lattice → a.lattice")`.

### Pass 2: Symbol Collection

Walk top-level rules (and block-level items in Pass 3). Collect:
- `variable_declaration` → `{name: value_list_node}`
- `mixin_definition` → `{name: (params, defaults, body_node)}`
- `function_definition` → `{name: (params, defaults, body_node)}`

Remove definition nodes from the AST (they produce no CSS output).

### Pass 3: Expansion

Recursively walk remaining AST nodes with a scope chain:

**Variable substitution**: When encountering a `VARIABLE` token in a
`value_list` or `function_arg`, look up the variable name in the scope chain.
Replace the VARIABLE token with the resolved value's tokens.

**`@include` expansion**:
1. Look up mixin name in the registry
2. Match actual arguments to formal parameters (use defaults for missing args)
3. Create a child scope with parameters bound
4. Deep-clone the mixin body AST
5. Recursively process the cloned body in the child scope
6. Splice the processed body nodes in place of the `include_directive`

Cycle detection: maintain a call stack set. If a mixin name appears twice,
raise `LatticeError("Circular mixin: a → b → a")`.

**Control flow evaluation**:
- `@if`: Evaluate the expression. If truthy, process the block. Otherwise
  try `@else if` branches, then `@else`.
- `@for`: Evaluate bounds. Loop from start to end (inclusive for `through`,
  exclusive for `to`), binding the loop variable in each iteration. Expand
  the body for each iteration.
- `@each`: Iterate over the value list, binding the variable(s) per item.
  Expand the body for each item.

**Function evaluation**: When a `function_call` node references a Lattice
function (not a CSS function like `rgb()`, `calc()`, `var()`):
1. Look up function in registry
2. Bind parameters in an isolated scope (parent = definition-site globals)
3. Evaluate function body statements
4. Return the `@return` expression's evaluated value
5. Replace the function_call node with the returned value

### Scoping Model

Variables are block-scoped with lexical lookup:

```
$color: red;              // global scope
.parent {
  $color: blue;           // child scope, shadows global
  color: $color;          // → blue
  .child {
    color: $color;        // → blue (inherits from parent scope)
  }
}
.sibling {
  color: $color;          // → red (global scope, not affected by .parent)
}
```

Implementation: `ScopeChain` class with `bindings`, `parent`, and methods:
- `get(name)` — walk up the chain until found
- `set(name, value)` — bind in the current scope
- `child()` — create a new scope with `self` as parent

Mixin expansion creates a child scope (inherits caller's scope).
Function evaluation creates an isolated scope (parent = definition-site
globals only, not caller's scope). This prevents functions from accidentally
depending on where they're called from.

### Expression Evaluator

Evaluates Lattice expressions at compile time. Handles:

**Arithmetic on numbers**: `2 + 3` → `5`, `10 * 2` → `20`

**Arithmetic on dimensions**: `10px + 5px` → `15px` (same units). When units
differ, emit a `calc()` expression: `10px + 2em` → `calc(10px + 2em)`.

**Comparisons**: `$size > 10px` → `true`/`false`. Compares numeric values
when both operands have the same unit. String equality for identifiers.

**Boolean logic**: `$a and $b`, `$a or $b`, `not $a`. Values are truthy
unless they are `false`, `null`, or `0`.

**Type rules**:
- Number + Number → Number
- Dimension + Dimension (same unit) → Dimension
- Dimension + Dimension (different units) → calc() expression
- Number * Dimension → Dimension (scales the value)
- String == String → Boolean
- Everything else → type error

## CSS Emitter

Reconstructs CSS text from a clean AST (no Lattice nodes remain after
transformation). Dispatches on `rule_name`:

```python
class CSSEmitter:
    def emit(self, ast: ASTNode) -> str
```

Handler methods:
- `_emit_stylesheet` — join rules with blank lines
- `_emit_qualified_rule` — selector list + block
- `_emit_at_rule` — @keyword prelude { block } or ;
- `_emit_selector_list` — comma-separated selectors
- `_emit_complex_selector` — compound selectors with combinators
- `_emit_block` — `{ declarations }` with indentation
- `_emit_declaration` — `property: value_list;`
- `_emit_value_list` — space-separated values
- `_emit_function_call` — `name(args)`
- `_emit_priority` — `!important`
- `_emit_default` — recurse children, emit tokens as-is

Supports two modes:
- **Pretty-print** (default): 2-space indentation, newlines after declarations,
  blank lines between rules
- **Minified**: no unnecessary whitespace

## Error Reporting

| Error | When | Example Message |
|-------|------|-----------------|
| Undefined variable | Pass 3, variable lookup | `Undefined variable '$foo' at line 12` |
| Undefined mixin | Pass 3, @include | `Undefined mixin 'bar' at line 15` |
| Undefined function | Pass 3, function call | `Undefined function 'baz' at line 20` |
| Wrong arity | Pass 3, argument matching | `Mixin 'bar' expects 3 args, got 1 at line 15` |
| Circular mixin | Pass 3, cycle detection | `Circular mixin: a → b → a at line 8` |
| Circular function | Pass 3, cycle detection | `Circular function: f → g → f at line 30` |
| Type error | Pass 3, expression eval | `Cannot add '10px' and 'red' at line 5` |
| Unit mismatch | Pass 3, arithmetic | `Cannot add '10px' and '5s' at line 7` |
| Module not found | Pass 1, @use | `Module 'colors' not found at line 1` |
| Missing @return | Pass 3, function eval | `Function 'double' has no @return at line 25` |
| @return outside function | Pass 2, collection | `@return outside @function at line 10` |

All errors carry line and column from the originating token.

## Python Packages

The implementation uses 4 small, composable packages rather than a
monolithic transpiler. Each package has a single responsibility:

### 1. `lattice-lexer`
Thin wrapper around `GrammarLexer` — loads `lattice.tokens`, provides
`tokenize_lattice()` and `create_lattice_lexer()`.

### 2. `lattice-parser`
Thin wrapper around `GrammarParser` — loads `lattice.grammar`, provides
`parse_lattice()` and `create_lattice_parser()`.

### 3. `lattice-ast-to-css`
The core compiler. Contains:
- `errors.py` — LatticeError hierarchy (10 error types)
- `scope.py` — ScopeChain for lexical scoping
- `evaluator.py` — Compile-time expression evaluator
- `emitter.py` — CSS source text emitter
- `transformer.py` — Multi-pass AST transformer

### 4. `lattice-transpiler`
Pipeline package — single `transpile_lattice()` entry point that wires
parse → transform → emit.

### Dependency Chain

```
lattice-transpiler
  └── lattice-ast-to-css
  └── lattice-parser
        └── lattice-lexer
              └── lexer, grammar-tools
        └── parser (lang_parser)
```

Does NOT depend on `css-lexer` or `css-parser` — Lattice has its own
standalone grammar files.

## Stress Test Scenarios

These test cases exercise feature interactions across the full pipeline:

### Test 1: Variables in Mixin Args Inside @media

```lattice
$breakpoint: 768px;
$color: blue;

@mixin responsive($prop-value) {
  @media (min-width: $breakpoint) {
    color: $prop-value;
  }
}

.header {
  @include responsive($color);
}
```

Expected CSS:
```css
.header {
}

@media (min-width: 768px) {
  .header {
    color: blue;
  }
}
```

Tests: scope chain through mixin expansion + at-rule nesting.

### Test 2: Scope Shadowing

```lattice
$color: red;

.outer {
  $color: blue;
  color: $color;

  .inner {
    $color: green;
    color: $color;
  }
}

.sibling {
  color: $color;
}
```

Expected CSS:
```css
.outer {
  color: blue;
}

.outer .inner {
  color: green;
}

.sibling {
  color: red;
}
```

Tests: block scoping with shadowing, sibling isolation.

### Test 3: @each Producing Multiple Rules

```lattice
@each $color in red, green, blue {
  .text-$color {
    color: $color;
  }
}
```

Note: since we don't support string interpolation in selectors, this test
would need the selector to use the variable differently. Alternative:

```lattice
@each $size in 8px, 16px, 24px {
  .card {
    padding: $size;
  }
}
```

Expected CSS (3 rules from one construct):
```css
.card {
  padding: 8px;
}

.card {
  padding: 16px;
}

.card {
  padding: 24px;
}
```

### Test 4: @if Inside @each

```lattice
$theme: dark;

@each $value in 10px, 20px {
  @if $theme == dark {
    .box {
      padding: $value;
      background: black;
    }
  } @else {
    .box {
      padding: $value;
      background: white;
    }
  }
}
```

Tests: control flow nesting with outer loop variable and global variable.

### Test 5: Function Calls in Value Positions

```lattice
@function spacing($multiplier) {
  @return $multiplier * 8px;
}

.card {
  padding: spacing(2);
  margin: spacing(3);
}
```

Expected CSS:
```css
.card {
  padding: 16px;
  margin: 24px;
}
```

### Test 6: Recursive Mixin with Depth Guard

```lattice
@mixin nested-box($depth) {
  padding: $depth * 10px;

  @if $depth > 0 {
    .inner {
      @include nested-box($depth - 1);
    }
  }
}

.box {
  @include nested-box(2);
}
```

Tests: recursive mixin expansion with @if termination condition.

### Test 7: Deeply Nested Feature Interaction

```lattice
@use "tokens" as t;

$base: 16px;

@function scale($value, $factor) {
  @return $value * $factor;
}

@mixin responsive-type($factor) {
  font-size: scale($base, $factor);

  @media (min-width: 768px) {
    @if $factor > 1 {
      font-size: scale($base, $factor * 1.2);
    }
  }
}

h1 {
  @include responsive-type(2);
}
```

Tests: @use + variable + function + mixin + @media + @if, all interacting.

### Test 8: Plain CSS Passthrough

```lattice
h1 {
  color: red;
  font-size: 2em;
}

@media (max-width: 768px) {
  h1 {
    font-size: 1.5em;
  }
}
```

Expected: identical CSS output. Tests that the transpiler is a true superset.

## Verification

```bash
# Grammar validation
grammar_tools validate lattice.tokens lattice.grammar

# Unit tests with coverage
cd code/packages/python/lattice-transpiler
mise exec python -- python -m pytest tests/ -v --cov=lattice_transpiler --cov-fail-under=90

# Integration: compile fixtures, compare to expected output
mise exec python -- python -m pytest tests/test_integration.py -v

# Plain CSS passthrough
mise exec python -- python -c "
from lattice_transpiler import transpile_lattice
css = 'h1 { color: red; }'
assert transpile_lattice(css).strip() == css
"

# Build tool discovery
./build-tool -dry-run | grep lattice
```

# Lattice v2

## Overview

Lattice v1 implements a CSS superset with variables, mixins, control flow (@if/@for/@each),
functions, and modules. It covers the most common Sass use cases, but several features are
missing that real-world Sass codebases rely on daily:

- **@while loops** for iteration that depends on a condition rather than a range
- **Variables in selectors** for generating families of rules (`.col-1`, `.col-2`, ...)
- **@content blocks** for wrapping arbitrary CSS in a mixin
- **!default and !global flags** for library-style variable management
- **Property nesting** for grouping related properties (`font: { size: ...; weight: ...; }`)
- **@at-root** for escaping nesting context
- **@extend and %placeholder selectors** for selector inheritance
- **Maps** as a first-class value type for structured data
- **Built-in color, list, math, and type functions** that every Sass codebase uses

Lattice v2 closes these gaps while preserving the core design constraint: **the grammar
remains context-free**. Every new construct starts with an unambiguous leading token or
keyword. No lookahead beyond one token is required. The grammar can be parsed by any
LL(1)-compatible parser, including the GrammarParser infrastructure already in place.

### Design approach

Features are split into two tiers:

- **Tier 1 (Grammar + Transformer Only)** — features 1-8 require changes to the grammar
  and transformer but no new value types. The existing token infrastructure and expression
  evaluator handle them.

- **Tier 2 (New Value Types)** — features 9-11 introduce maps as a value type and a
  library of built-in functions. These require changes to the value representation and
  the function call mechanism.

---

## New Tokens

Three new tokens are added to `lattice.tokens`:

```
# Placeholder selector prefix (NEW — Lattice v2)
# %placeholder selectors are used with @extend. The % character never
# appears in valid CSS value positions, making this unambiguous.
PLACEHOLDER = /%[a-zA-Z_][a-zA-Z0-9_-]*/

# !default flag (NEW — Lattice v2)
# Must come before BANG in token ordering. Only valid after a value in
# a variable declaration.
BANG_DEFAULT = "!default"

# !global flag (NEW — Lattice v2)
# Must come before BANG in token ordering. Only valid after a value in
# a variable declaration.
BANG_GLOBAL = "!global"
```

### Token ordering

In `lattice.tokens`, the new tokens must be placed as follows:

- `PLACEHOLDER` goes after `VARIABLE` and before `DIMENSION`. The `%` character would
  otherwise match the start of `PERCENTAGE`, but `PLACEHOLDER` requires a letter or
  underscore after `%`, while `PERCENTAGE` requires a digit before `%`. No ambiguity.

- `BANG_DEFAULT` and `BANG_GLOBAL` go in a new section between the multi-character
  operators and single-character delimiters, before `BANG`. The lexer tries multi-character
  patterns first, so `!default` matches `BANG_DEFAULT` rather than `BANG` + `IDENT`.

---

## Grammar Changes

This section shows only the rules that change or are added. All existing rules not
mentioned here remain exactly as defined in `lattice.grammar`.

### Modified rules

```
# Top-level rule list — add while_directive and extend_directive
lattice_control = if_directive | for_directive | each_directive | while_directive ;

# Variable declaration — add optional !default and !global flags
# Flags can appear in either order: $var: 10px !default !global;
variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;

variable_flag = BANG_DEFAULT | BANG_GLOBAL ;

# Lattice block items — add @content, @at-root, @extend
lattice_block_item = variable_declaration
                   | include_directive
                   | lattice_control
                   | content_directive
                   | at_root_directive
                   | extend_directive ;

# Include directive — add optional content block
# @include mixin-name($arg) { content passed to @content }
include_directive = "@include" FUNCTION [ include_args ] RPAREN
                      ( SEMICOLON | block )
                  | "@include" IDENT ( SEMICOLON | block ) ;

# Selectors — allow VARIABLE in compound selectors
# This enables .col-$i, $tag-name, etc.
simple_selector = IDENT | STAR | AMPERSAND | VARIABLE ;

# Class selector — allow VARIABLE after DOT
# .col-$i tokenizes as DOT IDENT("-col") VARIABLE("$i") — but since
# the lexer doesn't split mid-token, the practical form is:
# .$var   → DOT VARIABLE
# .name   → DOT IDENT (existing)
class_selector = DOT ( IDENT | VARIABLE ) ;

# Add placeholder selector to subclass_selector
subclass_selector = class_selector | id_selector
                  | attribute_selector | pseudo_class
                  | pseudo_element | placeholder_selector ;

# Value rule — add PLACEHOLDER to valid value tokens
value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH
      | CUSTOM_PROPERTY | UNICODE_RANGE | function_call
      | VARIABLE | PLACEHOLDER
      | SLASH | COMMA | PLUS | MINUS ;

# Block contents — add at_root and extend at block level
block_item = lattice_block_item | at_rule | declaration_or_nested ;

# Property nesting: a declaration whose value is a block
# font: { size: 14px; weight: bold; }
# This is handled by modifying declaration_or_nested to also try
# property_nesting before falling through to declaration.
declaration_or_nested = property_nesting | declaration | qualified_rule ;

# Lattice primary — add map literal and list literal
lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
                | STRING | IDENT | HASH
                | "true" | "false" | "null"
                | function_call
                | map_literal
                | LPAREN lattice_expression RPAREN ;
```

### New rules

```
# ============================================================================
# Lattice v2: @while loops
# ============================================================================
#
# @while loops execute their body as long as the condition is truthy.
# A max-iteration guard (1000 iterations) prevents infinite loops.
#
#   $i: 1;
#   @while $i <= 12 {
#     .col-#{$i} { width: calc(100% / 12 * $i); }
#     $i: $i + 1;
#   }

while_directive = "@while" lattice_expression block ;

# ============================================================================
# Lattice v2: @content blocks
# ============================================================================
#
# Inside a mixin body, @content is replaced with the block passed to
# the @include call site. If no content block was passed, @content
# produces nothing (silently omitted).
#
#   @mixin responsive($bp) {
#     @media (min-width: $bp) {
#       @content;
#     }
#   }
#
#   .sidebar {
#     @include responsive(768px) {
#       float: left;
#       width: 300px;
#     }
#   }

content_directive = "@content" SEMICOLON ;

# ============================================================================
# Lattice v2: @at-root
# ============================================================================
#
# Emits its contents at the stylesheet root, escaping any nesting context.
# Two forms:
#
#   @at-root {
#     .root-level { color: red; }
#   }
#
#   @at-root .root-level {
#     color: red;
#   }

at_root_directive = "@at-root" ( block | selector_list block ) ;

# ============================================================================
# Lattice v2: @extend and %placeholder selectors
# ============================================================================
#
# @extend appends the current selector to another rule's selector list.
# Works with class selectors, placeholder selectors, and other simple selectors.
#
#   %message-shared {
#     border: 1px solid #ccc;
#     padding: 10px;
#     color: #333;
#   }
#
#   .success { @extend %message-shared; color: green; }
#   .error   { @extend %message-shared; color: red; }
#
# Placeholder selectors (%name) are removed from output — they only
# exist to be extended.

extend_directive = "@extend" extend_target SEMICOLON ;

extend_target = PLACEHOLDER | DOT IDENT | IDENT ;

placeholder_selector = PLACEHOLDER ;

# ============================================================================
# Lattice v2: Property nesting
# ============================================================================
#
# Groups related properties under a shared prefix:
#
#   font: {
#     size: 14px;
#     weight: bold;
#     family: sans-serif;
#   }
#
# Produces:
#   font-size: 14px;
#   font-weight: bold;
#   font-family: sans-serif;
#
# The parent property name is prepended with a hyphen to each child
# property name.

property_nesting = property COLON block ;

# ============================================================================
# Lattice v2: Maps
# ============================================================================
#
# Maps are ordered key-value stores written as parenthesized pairs:
#
#   $theme: (
#     primary: #4a90d9,
#     secondary: #7b68ee,
#     background: #ffffff,
#   );
#
# Keys are identifiers or strings. Values are any value_list.
# Maps are valid in variable assignments and function arguments.
# Trailing comma is allowed.

map_literal = LPAREN map_entry COMMA { map_entry COMMA } [ map_entry ] RPAREN ;

map_entry = ( IDENT | STRING ) COLON value_list ;
```

---

## Feature Reference

### 1. @while loops

**Syntax:**
```scss
@while <expression> {
  <body>
}
```

**Semantics:**
1. Evaluate `<expression>` as a Lattice expression (same rules as @if conditions).
2. If the result is truthy (not `false`, not `null`, not `0`, not empty string), execute
   `<body>`.
3. After executing the body, evaluate the expression again. Repeat.
4. If the iteration count exceeds the **max-iteration guard** (1000), halt with a
   `MaxIterationError`.
5. Variable mutations inside the body (including re-assignment to the loop variable) are
   visible in subsequent iterations and after the loop exits.

**Truthy/falsy rules** (same as @if):
- Falsy: `false`, `null`, the number `0`, the empty string `""`
- Truthy: everything else (including the string `"false"`, non-zero numbers, all colors)

**Examples:**

```scss
// Generate column classes
$i: 1;
@while $i <= 12 {
  .col-#{$i} {
    width: calc(100% / 12 * $i);
  }
  $i: $i + 1;
}

// Halving a value
$size: 256px;
@while $size >= 16px {
  .icon-#{$size} { width: $size; height: $size; }
  $size: $size / 2;
}
```

**Error conditions:**
- `MaxIterationError`: loop body executed more than 1000 times. Message:
  `"@while loop exceeded maximum iteration count (1000)"`
- Condition expression evaluation errors propagate normally (type errors, undefined
  variables, etc.)

**Edge cases:**
- `@while false { }` — body never executes, produces no output.
- `@while true { }` with no variable mutation — hits max-iteration guard.
- Variables declared inside the loop body are scoped to the loop. Variables from outer
  scope that are reassigned inside the loop retain their new value after the loop.

---

### 2. $var in selectors

**Syntax:**
```scss
.$variable { ... }
$variable { ... }
.prefix-$variable { ... }
```

**Semantics:**
1. The lexer tokenizes `$var` as a `VARIABLE` token wherever it appears, including
   selector positions.
2. The grammar allows `VARIABLE` in `simple_selector` and `class_selector` positions.
3. During transformation, the transformer resolves the variable to its value (which must
   be a string, identifier, or number) and concatenates it with any adjacent literal
   tokens to form the final selector string.
4. Concatenation is purely textual: `.col-` + `3` = `.col-3`.

**Token-level behavior:**
```
.col-$i   tokenizes as:  DOT IDENT("col-") VARIABLE("$i")
                          ^^^ class_selector uses IDENT
                          Note: "col-" includes the hyphen because
                          IDENT = /-?[a-zA-Z_][a-zA-Z0-9_-]*/

.$theme   tokenizes as:  DOT VARIABLE("$theme")
                          ^^^ class_selector uses VARIABLE

$tag      tokenizes as:  VARIABLE("$tag")
                          ^^^ simple_selector uses VARIABLE
```

**Examples:**

```scss
// Dynamic class generation in @for
@for $i from 1 through 5 {
  .mt-$i {
    margin-top: $i * 4px;
  }
}
// Output: .mt-1 { margin-top: 4px; } .mt-2 { margin-top: 8px; } ...

// Dynamic element selector
$heading: h2;
$heading {
  font-size: 24px;
}
// Output: h2 { font-size: 24px; }
```

**Error conditions:**
- Variable used in selector position is undefined: `UndefinedVariableError`
- Variable value cannot be converted to a valid selector string (e.g., a map):
  `TypeError: "Cannot use map as selector"`

**Edge cases:**
- Variable resolves to a number: `.col-$i` where `$i` is `3` produces `.col-3`. The
  number is stringified.
- Variable resolves to a string with spaces: this is a transformer-level error. Selectors
  cannot contain unquoted spaces.
- Nested variable references in selectors: each variable is resolved independently and
  concatenated.

---

### 3. @content blocks

**Syntax:**

Inside a mixin definition:
```scss
@mixin name($params) {
  // ... declarations ...
  @content;
  // ... more declarations ...
}
```

At the include call site:
```scss
@include name($args) {
  // This block replaces @content in the mixin body
  .child { color: red; }
}
```

**Semantics:**
1. When a mixin body contains `@content;`, it marks the insertion point for caller-
   provided content.
2. When `@include` is followed by a block (not a semicolon), that block is the content
   block.
3. During transformation, `@content;` is replaced with the content block's children.
4. The content block is evaluated in the **caller's scope**, not the mixin's scope. This
   means variables from the mixin's parameters are not visible inside the content block
   unless they were defined in an outer scope.
5. If `@content;` appears but no content block was passed, it produces nothing (no error).
6. If a content block is passed but the mixin has no `@content;`, the block is silently
   ignored (no error).
7. Multiple `@content;` statements in the same mixin each emit the content block (the
   block is duplicated at each insertion point).

**Examples:**

```scss
// Media query wrapper mixin
@mixin respond-to($breakpoint) {
  @media (min-width: $breakpoint) {
    @content;
  }
}

.sidebar {
  width: 100%;
  @include respond-to(768px) {
    width: 300px;
    float: left;
  }
}
// Output:
// .sidebar { width: 100%; }
// @media (min-width: 768px) { .sidebar { width: 300px; float: left; } }

// Mixin with no @content — block ignored
@mixin simple { color: red; }
.foo { @include simple { this: is-ignored; } }
// Output: .foo { color: red; }

// @content with no block — silent no-op
@mixin with-content { @content; color: blue; }
.bar { @include with-content; }
// Output: .bar { color: blue; }
```

**Error conditions:**
- `@content` used outside a mixin body: `SyntaxError: "@content is only valid inside a mixin definition"`

**Edge cases:**
- Nested mixins with @content: each mixin tracks its own content block independently.
  An inner mixin's @content refers to the inner include's block, not the outer one.
- @content inside @if/@for/@each inside a mixin: the content block is still available
  and is inserted when @content is reached.

---

### 4. !default flag

**Syntax:**
```scss
$variable: value !default;
```

**Semantics:**
1. Evaluate whether `$variable` is already defined in the current scope chain (current
   scope, then each parent scope up to global).
2. If the variable **is not defined** anywhere in the scope chain, assign `value` to
   `$variable` in the current scope.
3. If the variable **is already defined** (even if its value is `null`), do nothing —
   the existing value is preserved.

**Use case:** Library authors use `!default` to provide configurable defaults:
```scss
// _library.scss
$primary-color: blue !default;
$border-radius: 4px !default;

// user's main.scss
$primary-color: #ff6600;
@use "library";
// $primary-color is #ff6600 (user's value wins)
// $border-radius is 4px (library default used)
```

**Examples:**

```scss
$color: red;
$color: blue !default;
// $color is still red — !default does not overwrite

$size: 16px !default;
// $size is 16px — variable was not previously defined

$undefined: null;
$undefined: 10px !default;
// $undefined is still null — the variable IS defined (even though its value is null)
```

**Error conditions:**
- `!default` on a non-variable-declaration statement: syntax error (grammar rejects it).
- `!default` combined with `!global`: both flags can appear on the same declaration.
  `!default` check happens first (in the scope determined by `!global`), then assignment
  happens with `!global` scope rules.

**Edge cases:**
- `$var: value !default !global;` — checks if `$var` is defined in the global scope.
  If not, sets it globally. If it is, does nothing.
- `!default` inside a mixin: checks the mixin's local scope chain (local scope + caller
  scopes up to global).

---

### 5. !global flag

**Syntax:**
```scss
$variable: value !global;
```

**Semantics:**
1. Assign `value` to `$variable` in the **root (global) scope**, regardless of how deeply
   nested the current execution context is.
2. If `$variable` already exists in the global scope, it is overwritten.
3. If `$variable` does not exist in the global scope, it is created there.
4. The variable is NOT created in the local scope — only in the global scope.

**Examples:**

```scss
$theme: light;

@mixin set-dark {
  $theme: dark !global;
}

.app {
  @include set-dark;
  // $theme is now "dark" globally
  background: if($theme == dark, #1a1a1a, white);
}
```

```scss
@for $i from 1 through 3 {
  $last-index: $i !global;
}
// $last-index is 3 in the global scope
```

**Error conditions:**
- `!global` on a non-variable-declaration statement: syntax error (grammar rejects it).

**Edge cases:**
- `!global` inside a function: the variable is set globally and persists after the
  function returns. This is intentional but considered bad practice.
- `!global` without `!default`: always sets the variable, even if it already exists.
- `!global` with `!default`: only sets the variable globally if it is not already defined
  in the global scope.

---

### 6. Property nesting

**Syntax:**
```scss
<property>: {
  <child-property>: <value>;
  <child-property>: <value>;
}
```

**Semantics:**
1. The parser recognizes a property followed by a colon and a block (instead of a value
   list) as a property nesting construct.
2. During transformation, each child declaration's property name is prefixed with the
   parent property name and a hyphen.
3. `font: { size: 14px; }` becomes `font-size: 14px;`.
4. Property nesting can be nested arbitrarily: `border: { style: { top: solid; } }` becomes
   `border-style-top: solid;`. (This is technically valid CSS, though unusual.)

**Examples:**

```scss
.card {
  font: {
    family: Helvetica, sans-serif;
    size: 14px;
    weight: bold;
  }
  margin: {
    top: 10px;
    bottom: 20px;
  }
  border: {
    width: 1px;
    style: solid;
    color: #ccc;
    radius: 4px;
  }
}
// Output:
// .card {
//   font-family: Helvetica, sans-serif;
//   font-size: 14px;
//   font-weight: bold;
//   margin-top: 10px;
//   margin-bottom: 20px;
//   border-width: 1px;
//   border-style: solid;
//   border-color: #ccc;
//   border-radius: 4px;
// }
```

**Error conditions:**
- Empty property nesting block: `font: { }` — produces no output (valid, not an error).
- Nested rules inside a property nesting block: not supported. Only declarations (and
  further property nesting) are valid children. Other constructs produce a
  `SyntaxError: "Nested rules are not allowed inside property nesting"`.

**Edge cases:**
- Property nesting with variables: `$prop: font; $prop: { size: 14px; }` — the property
  name is the literal text in the declaration, not a variable reference. Variable
  interpolation in property names is not supported in v2.
- Custom properties inside nesting: `--custom: { }` is NOT property nesting. Custom
  properties (starting with `--`) use blocks as values in CSS. The parser distinguishes
  by checking if the property starts with `--`.

---

### 7. @at-root

**Syntax:**
```scss
// Block form — multiple rules at root
@at-root {
  <rules>
}

// Inline form — single rule at root
@at-root <selector> {
  <declarations>
}
```

**Semantics:**
1. Rules inside `@at-root` are emitted at the stylesheet root level, regardless of how
   deeply nested the @at-root directive appears.
2. The nesting context (parent selectors) is discarded. The `&` selector inside @at-root
   refers to nothing (or produces an error — see edge cases).
3. @at-root only escapes **rule nesting**, not **at-rule nesting** (e.g., @media). Rules
   inside `@at-root` that are also inside `@media` remain inside the @media block.

**Examples:**

```scss
.parent {
  color: blue;

  @at-root .sibling {
    color: red;
  }

  @at-root {
    .another { color: green; }
    .more { color: yellow; }
  }
}
// Output:
// .parent { color: blue; }
// .sibling { color: red; }
// .another { color: green; }
// .more { color: yellow; }
```

```scss
.parent {
  @media (min-width: 768px) {
    @at-root .tablet-only {
      display: block;
    }
  }
}
// Output:
// @media (min-width: 768px) { .tablet-only { display: block; } }
// (The @media is preserved, but .parent nesting is escaped)
```

**Error conditions:**
- None specific to @at-root. If the content inside @at-root is invalid, normal syntax
  errors apply.

**Edge cases:**
- `&` inside @at-root: the parent selector is empty. Using `&` produces the literal
  text of the @at-root's own selector (if inline form) or is undefined (if block form).
  Transformer should emit a warning: `"& used inside @at-root with no parent selector"`.
- @at-root at the top level: no-op. Rules are already at root, so @at-root simply
  unwraps its block.
- @at-root inside a mixin: the rules are emitted at the stylesheet root, not at the
  mixin call site's nesting level.

---

### 8. @extend and %placeholder selectors

**Syntax:**
```scss
// Extend a class
@extend .class-name;

// Extend a placeholder
@extend %placeholder-name;

// Define a placeholder (used like a regular qualified rule)
%placeholder-name {
  <declarations>
}
```

**Semantics:**

**@extend:**
1. `@extend .target;` inside a rule tells the transformer: "everywhere `.target` appears
   as a selector in the stylesheet, also add the current rule's selector."
2. Selector merging is additive. If `.target { color: red; }` exists and `.success` extends
   `.target`, the output becomes `.target, .success { color: red; }`.
3. Extends are resolved after the entire stylesheet is parsed and transformed. This is a
   **post-processing pass**, not inline expansion.
4. An extend target that does not exist in the stylesheet produces an
   `ExtendTargetNotFoundError`, unless the extend uses the `!optional` flag (not
   implemented in v2 — noted for future work).

**%placeholder selectors:**
1. Placeholder selectors start with `%` and work exactly like class selectors in terms
   of matching and extension.
2. The key difference: placeholder rules are **removed from the output**. They exist
   only to be extended.
3. If a placeholder is defined but never extended, it is silently removed (no warning).

**Selector merging algorithm:**
1. Collect all `@extend` directives from the stylesheet into a map:
   `{target_selector -> [extending_selectors]}`.
2. For each rule in the output, check if its selector (or any part of its selector list)
   matches an extend target.
3. If it does, append the extending selectors to the rule's selector list.
4. Remove all rules whose selector is exclusively a placeholder (`%name`).
5. If a selector list contains both placeholder and non-placeholder selectors, only the
   placeholder portions are removed.

**Examples:**

```scss
%message-shared {
  border: 1px solid #ccc;
  padding: 10px;
  color: #333;
}

.success {
  @extend %message-shared;
  border-color: green;
}

.error {
  @extend %message-shared;
  border-color: red;
}
// Output (note: %message-shared itself is removed):
// .success, .error {
//   border: 1px solid #ccc;
//   padding: 10px;
//   color: #333;
// }
// .success { border-color: green; }
// .error { border-color: red; }
```

```scss
// Extending a regular class
.btn {
  padding: 8px 16px;
  border: none;
}

.btn-primary {
  @extend .btn;
  background: blue;
  color: white;
}
// Output:
// .btn, .btn-primary { padding: 8px 16px; border: none; }
// .btn-primary { background: blue; color: white; }
```

**Error conditions:**
- `ExtendTargetNotFoundError`: `@extend .nonexistent;` where `.nonexistent` is never
  defined as a selector in the stylesheet. Message:
  `"@extend target '.nonexistent' was not found in the stylesheet"`
- `@extend` used outside a rule block: syntax error.

**Edge cases:**
- Chained extends: `.a` extends `.b`, `.b` extends `.c`. All three share `.c`'s
  declarations. The extend resolver must handle transitive extension.
- Extending within @media: extends are scoped to the @media block. An @extend inside
  `@media print` only matches selectors within that same @media block.
- Multiple extends in one rule: each is processed independently.
- Extending a selector that is itself a selector list: the extending selector is added
  to the list.

---

### 9. Maps

**Syntax:**
```scss
$map: (
  key1: value1,
  key2: value2,
);
```

**Semantics:**
1. A map is an ordered collection of key-value pairs.
2. Keys must be strings or identifiers (identifiers are treated as strings for lookup).
3. Values can be any Lattice value: numbers, strings, colors, lists, or even other maps.
4. Maps are immutable. Functions like `map-merge()` return new maps.
5. Duplicate keys: the last value wins (no error).
6. Maps are truthy (even empty maps).
7. Trailing comma after the last entry is allowed.

**Disambiguation from grouped expressions:**

A parenthesized expression `($a + $b)` is a grouped expression, not a map. The parser
distinguishes them as follows:
- If the content inside parentheses contains a bare `COLON` separating an identifier/string
  from a value (i.e., matches the `map_entry` rule), it is a map literal.
- Otherwise, it is a grouped expression.
- Empty parentheses `()` represent an empty list, not an empty map. An empty map is not
  expressible as a literal (use `map-remove()` to empty a map, or simply never create one).

**Map access** is done exclusively through built-in functions:
```scss
$theme: (primary: #4a90d9, secondary: #7b68ee);

color: map-get($theme, primary);    // #4a90d9
$keys: map-keys($theme);            // primary, secondary
$has: map-has-key($theme, primary); // true
```

**Destructuring in @each:**
```scss
$sizes: (sm: 576px, md: 768px, lg: 992px);

@each $name, $width in $sizes {
  @media (min-width: $width) {
    .container-$name { max-width: $width; }
  }
}
```

When `@each` receives two variables and iterates over a map, the first variable gets
the key and the second gets the value for each entry.

**Error conditions:**
- Using a non-string/non-identifier as a map key: `TypeError: "Map keys must be strings or identifiers"`
- Attempting to use a map directly as a CSS value (e.g., `color: $map;`):
  `TypeError: "Maps cannot be used as CSS values"`

**Edge cases:**
- Single-entry map vs. parenthesized value: `(color: red)` is a map with one entry.
  `(red)` is a grouped expression. `(color: red, )` is also a map (trailing comma).
- Nested maps: `(outer: (inner: value))` is valid. `map-get()` returns the inner map.
- Map equality: two maps are equal if they have the same keys in the same order with
  equal values.

---

### 10. Built-in color functions

All color functions operate on hex color values (e.g., `#4a90d9`) and return hex colors.
Colors are internally represented as RGBA tuples with values 0-255 for channels and
0-1 for alpha.

**HSL-based functions** convert the input hex to HSL, apply the adjustment, and convert
back to hex.

| Function | Signature | Description |
|----------|-----------|-------------|
| `lighten` | `lighten($color, $amount)` | Increase lightness by `$amount` (a percentage, 0-100%) |
| `darken` | `darken($color, $amount)` | Decrease lightness by `$amount` |
| `saturate` | `saturate($color, $amount)` | Increase saturation by `$amount` |
| `desaturate` | `desaturate($color, $amount)` | Decrease saturation by `$amount` |
| `adjust-hue` | `adjust-hue($color, $degrees)` | Rotate hue by `$degrees` (positive or negative) |
| `complement` | `complement($color)` | Rotate hue by 180 degrees |
| `mix` | `mix($color1, $color2, $weight: 50%)` | Blend two colors. `$weight` is the proportion of `$color1` |
| `rgba` | `rgba($color, $alpha)` or `rgba($r, $g, $b, $a)` | Set alpha channel or construct from components |
| `red` | `red($color)` | Extract red channel (0-255) |
| `green` | `green($color)` | Extract green channel (0-255) |
| `blue` | `blue($color)` | Extract blue channel (0-255) |
| `hue` | `hue($color)` | Extract hue component (0-360deg) |
| `saturation` | `saturation($color)` | Extract saturation (0-100%) |
| `lightness` | `lightness($color)` | Extract lightness (0-100%) |

**Examples:**

```scss
$primary: #4a90d9;

.lighten   { color: lighten($primary, 20%); }    // #93bce8
.darken    { color: darken($primary, 20%); }     // #2b6cb3
.complement { color: complement($primary); }     // #d9934a
.mixed     { color: mix(#ff0000, #0000ff, 50%); } // #800080
.alpha     { color: rgba($primary, 0.5); }       // rgba(74, 144, 217, 0.5)
.channel   { --red: red($primary); }             // 74
```

**Error conditions:**
- First argument is not a color: `TypeError: "Expected a color, got <type>"`
- Amount is out of range (< 0% or > 100% for lighten/darken/saturate/desaturate):
  `RangeError: "Amount must be between 0% and 100%"`
- `mix()` weight out of range: `RangeError: "Weight must be between 0% and 100%"`

---

### 11. Built-in list, type, and math functions

#### List functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `nth` | `nth($list, $n)` | Get the nth item (1-indexed) |
| `length` | `length($list)` | Number of items in a list or map |
| `join` | `join($list1, $list2, $separator: auto)` | Concatenate two lists |
| `append` | `append($list, $val, $separator: auto)` | Add a value to the end of a list |
| `index` | `index($list, $value)` | Find the position of a value (1-indexed), or `null` if not found |

Lists in Lattice are space-separated or comma-separated value sequences. The `$separator`
parameter in `join()` and `append()` can be `space`, `comma`, or `auto` (inherits from
the first list).

#### Type introspection functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `type-of` | `type-of($value)` | Returns the type as a string: `"number"`, `"string"`, `"color"`, `"list"`, `"map"`, `"bool"`, `"null"` |
| `unit` | `unit($number)` | Returns the unit as a string: `"px"`, `"em"`, `"%"`, `""` |
| `unitless` | `unitless($number)` | Returns `true` if the number has no unit |
| `comparable` | `comparable($n1, $n2)` | Returns `true` if two numbers can be compared (same unit type or unitless) |

#### Math functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `math.div` | `math.div($a, $b)` | Division (replaces the `/` operator which is ambiguous with CSS shorthand) |
| `math.floor` | `math.floor($n)` | Round down to nearest integer |
| `math.ceil` | `math.ceil($n)` | Round up to nearest integer |
| `math.round` | `math.round($n)` | Round to nearest integer |
| `math.abs` | `math.abs($n)` | Absolute value |
| `math.min` | `math.min($numbers...)` | Minimum of all arguments |
| `math.max` | `math.max($numbers...)` | Maximum of all arguments |

**The `math.` prefix:** Math functions use a module-style prefix to avoid name collisions
with CSS functions (e.g., CSS `min()` and `max()` exist natively). At the token level,
`math.div(` is tokenized as `FUNCTION("math.div(")` — the dot is part of the function
name. No special grammar support is needed.

**Map functions** (see Feature 9 for map context):

| Function | Signature | Description |
|----------|-----------|-------------|
| `map-get` | `map-get($map, $key)` | Get value for key, or `null` if not found |
| `map-keys` | `map-keys($map)` | Return all keys as a list |
| `map-values` | `map-values($map)` | Return all values as a list |
| `map-has-key` | `map-has-key($map, $key)` | Return `true` if key exists |
| `map-merge` | `map-merge($map1, $map2)` | Merge two maps (second wins on conflicts) |
| `map-remove` | `map-remove($map, $keys...)` | Return a new map without the specified keys |

**Examples:**

```scss
$list: 10px 20px 30px;
$first: nth($list, 1);           // 10px
$len: length($list);             // 3
$type: type-of($first);          // "number"
$u: unit($first);                // "px"
$half: math.div(100%, 3);        // 33.33333%
$rounded: math.round(3.7px);    // 4px
$smallest: math.min(10px, 5px, 20px); // 5px
```

**Error conditions:**
- `nth()` index out of bounds: `RangeError: "Index 5 out of bounds for list of length 3"`
- `nth()` index is zero or negative: `RangeError: "List index must be 1 or greater"`
- `math.div()` division by zero: `ZeroDivisionError: "Division by zero"`
- `unit()` on non-number: `TypeError: "Expected a number, got <type>"`
- `map-get()` on non-map: `TypeError: "Expected a map, got <type>"`
- `math.min()`/`math.max()` with incomparable units: `TypeError: "Cannot compare px and em"`

---

## Built-in Functions Reference

Complete table of all built-in functions by category.

### Color functions

| Function | Signature | Returns | Example |
|----------|-----------|---------|---------|
| `lighten` | `($color, $amount)` | color | `lighten(#333, 20%)` |
| `darken` | `($color, $amount)` | color | `darken(#fff, 10%)` |
| `saturate` | `($color, $amount)` | color | `saturate(#888, 30%)` |
| `desaturate` | `($color, $amount)` | color | `desaturate(#f00, 50%)` |
| `adjust-hue` | `($color, $degrees)` | color | `adjust-hue(#f00, 120)` |
| `complement` | `($color)` | color | `complement(#f00)` |
| `mix` | `($c1, $c2, $weight: 50%)` | color | `mix(#f00, #00f)` |
| `rgba` | `($color, $alpha)` | color | `rgba(#333, 0.5)` |
| `red` | `($color)` | number | `red(#4a90d9)` → `74` |
| `green` | `($color)` | number | `green(#4a90d9)` → `144` |
| `blue` | `($color)` | number | `blue(#4a90d9)` → `217` |
| `hue` | `($color)` | number | `hue(#4a90d9)` → `210deg` |
| `saturation` | `($color)` | number | `saturation(#4a90d9)` → `60%` |
| `lightness` | `($color)` | number | `lightness(#4a90d9)` → `57%` |

### List functions

| Function | Signature | Returns | Example |
|----------|-----------|---------|---------|
| `nth` | `($list, $n)` | value | `nth(10px 20px, 1)` → `10px` |
| `length` | `($list)` | number | `length(a b c)` → `3` |
| `join` | `($l1, $l2, $sep: auto)` | list | `join(a b, c d)` → `a b c d` |
| `append` | `($list, $val, $sep: auto)` | list | `append(a b, c)` → `a b c` |
| `index` | `($list, $value)` | number/null | `index(a b c, b)` → `2` |

### Map functions

| Function | Signature | Returns | Example |
|----------|-----------|---------|---------|
| `map-get` | `($map, $key)` | value/null | `map-get((a: 1), a)` → `1` |
| `map-keys` | `($map)` | list | `map-keys((a: 1, b: 2))` → `a, b` |
| `map-values` | `($map)` | list | `map-values((a: 1, b: 2))` → `1, 2` |
| `map-has-key` | `($map, $key)` | bool | `map-has-key((a: 1), b)` → `false` |
| `map-merge` | `($map1, $map2)` | map | `map-merge((a: 1), (b: 2))` → `(a: 1, b: 2)` |
| `map-remove` | `($map, $keys...)` | map | `map-remove((a: 1, b: 2), a)` → `(b: 2)` |

### Type functions

| Function | Signature | Returns | Example |
|----------|-----------|---------|---------|
| `type-of` | `($value)` | string | `type-of(14px)` → `"number"` |
| `unit` | `($number)` | string | `unit(14px)` → `"px"` |
| `unitless` | `($number)` | bool | `unitless(14)` → `true` |
| `comparable` | `($n1, $n2)` | bool | `comparable(1px, 2px)` → `true` |

### Math functions

| Function | Signature | Returns | Example |
|----------|-----------|---------|---------|
| `math.div` | `($a, $b)` | number | `math.div(100px, 3)` → `33.33333px` |
| `math.floor` | `($n)` | number | `math.floor(3.7px)` → `3px` |
| `math.ceil` | `($n)` | number | `math.ceil(3.2px)` → `4px` |
| `math.round` | `($n)` | number | `math.round(3.5px)` → `4px` |
| `math.abs` | `($n)` | number | `math.abs(-5px)` → `5px` |
| `math.min` | `($numbers...)` | number | `math.min(1px, 2px, 3px)` → `1px` |
| `math.max` | `($numbers...)` | number | `math.max(1px, 2px, 3px)` → `3px` |

---

## Value Types

Updated table of all Lattice value types including the new LatticeMap type.

| Type | Representation | Truthy? | Example |
|------|---------------|---------|---------|
| Number | float64 + optional unit string | `0` is falsy, all else truthy | `14px`, `3.5em`, `100%`, `42` |
| String | UTF-8 text | `""` is falsy, all else truthy | `"Helvetica"`, `sans-serif` |
| Color | RGBA (0-255, 0-255, 0-255, 0-1) | always truthy | `#4a90d9`, `rgba(0,0,0,0.5)` |
| Boolean | true/false | `false` is falsy | `true`, `false` |
| Null | singleton | always falsy | `null` |
| List | ordered sequence of values | always truthy (even empty) | `10px 20px 30px`, `a, b, c` |
| **LatticeMap** | ordered key-value pairs | always truthy (even empty) | `(primary: #4a90d9, secondary: #7b68ee)` |

### Type coercion rules

- **String context** (selectors, concatenation): all types are stringified. Numbers include
  their unit. Colors use hex notation. Booleans become `"true"`/`"false"`. Null becomes
  `""`. Maps cannot be stringified (TypeError).
- **Numeric context** (arithmetic): only numbers participate. Strings, colors, bools, nulls,
  lists, and maps produce a TypeError.
- **Boolean context** (@if, @while conditions): falsy values are `false`, `null`, `0`, `""`.
  Everything else is truthy.
- **Equality** (`==`, `!=`): same-type comparison. Different types are never equal (no
  coercion). Exception: identifier strings and quoted strings compare by text content
  (`red == "red"` is `true`).

---

## Error Types

New errors introduced by Lattice v2 features.

| Error | Raised by | Message template |
|-------|-----------|-----------------|
| `MaxIterationError` | @while | `"@while loop exceeded maximum iteration count (1000)"` |
| `ExtendTargetNotFoundError` | @extend | `"@extend target '<selector>' was not found in the stylesheet"` |
| `TypeError` | various | `"Expected a <expected>, got <actual>"`, `"Maps cannot be used as CSS values"`, `"Cannot use map as selector"`, `"Cannot compare <unit1> and <unit2>"`, `"Map keys must be strings or identifiers"` |
| `RangeError` | nth(), color functions | `"Index <n> out of bounds for list of length <len>"`, `"Amount must be between 0% and 100%"`, `"List index must be 1 or greater"` |
| `ZeroDivisionError` | math.div() | `"Division by zero"` |
| `SyntaxError` | @content outside mixin, nested rules in property nesting | `"@content is only valid inside a mixin definition"`, `"Nested rules are not allowed inside property nesting"` |

All errors include the source file path (if available) and line number of the offending
construct.

---

## Implementation Notes

### Per-language considerations

**All languages:**
- The max-iteration guard for @while (1000) should be a configurable constant, not a
  hardcoded magic number. Define it as `MAX_WHILE_ITERATIONS = 1000` (or equivalent).
- @extend resolution is a post-processing pass over the entire transformed AST. It runs
  after all other transformations (variable resolution, mixin expansion, control flow
  evaluation).
- Map equality must compare keys in insertion order (ordered map semantics).
- Built-in functions are registered in a function table at transformer initialization.
  User-defined functions (@function) shadow built-ins with the same name.

**Python:**
- Use `dict` for LatticeMap (Python 3.7+ dicts preserve insertion order).
- Color HSL conversion: implement manually or use `colorsys` stdlib module.
- Math functions: use `math` stdlib for floor/ceil/round/abs.

**Go:**
- Use a `[]MapEntry` slice (not `map[string]Value`) for LatticeMap to preserve insertion
  order. Provide O(1) lookup via a parallel `map[string]int` index.
- Color HSL conversion: implement manually (no stdlib support).
- Math functions: use `math` stdlib.

**Ruby:**
- Use Ruby's ordered `Hash` for LatticeMap (Ruby hashes preserve insertion order since 1.9).
- Color functions: implement manually or use a gem (prefer manual for zero dependencies).

**TypeScript:**
- Use `Map<string, Value>` for LatticeMap (ES6 Maps preserve insertion order).
- Color HSL conversion: implement manually.
- `math.div(` tokenizes as `FUNCTION("math.div(")` — the dot is part of the FUNCTION
  regex `/-?[a-zA-Z_][a-zA-Z0-9_-]*\(/`. Verify: `math.div(` does NOT match because the
  dot is not in the character class. **Fix needed**: update the FUNCTION token regex to
  include dots: `/-?[a-zA-Z_][a-zA-Z0-9_.-]*\(/`. This allows `math.div(`, `math.floor(`,
  etc. without affecting existing function names (no CSS function name contains a dot).

**Rust:**
- Use `IndexMap<String, Value>` from the `indexmap` crate for LatticeMap (preserves
  insertion order with O(1) lookup).
- Color HSL: implement manually.
- Math: use `f64` methods.

**Elixir:**
- Use a keyword list `[{key, value}]` or a custom ordered map for LatticeMap. Elixir's
  `Map` does not guarantee insertion order.
- Color functions: implement manually.

### Token regex update

The `FUNCTION` token regex must be updated to support dotted names:

```
# Current
FUNCTION = /-?[a-zA-Z_][a-zA-Z0-9_-]*\(/

# Updated (v2)
FUNCTION = /-?[a-zA-Z_][a-zA-Z0-9_.-]*\(/
```

This change is backward-compatible. No existing CSS or Lattice v1 function name contains
a dot.

### Grammar context-freeness

All new features maintain the context-free property:

- `@while` is introduced by the unambiguous `"@while"` keyword (AT_KEYWORD with text
  `@while`).
- `@content` is introduced by `"@content"`.
- `@at-root` is introduced by `"@at-root"`.
- `@extend` is introduced by `"@extend"`.
- `!default` and `!global` are distinct tokens (`BANG_DEFAULT`, `BANG_GLOBAL`), not
  contextual keywords.
- `PLACEHOLDER` (`%name`) is unambiguous because `%` followed by a letter never appears
  in CSS.
- Property nesting is distinguished from regular declarations by the presence of LBRACE
  after COLON (instead of a value token). This is a one-token lookahead decision.
- Map literals are distinguished from grouped expressions by the presence of COLON inside
  parentheses.

---

## Test Strategy

### Tier 1 features

**@while loops:**
- Basic loop: counter from 1 to 5, verify output contains 5 rules
- Condition with comparison operators: `<=`, `>=`, `!=`, `==`
- Variable mutation: verify variable value after loop exit
- Max-iteration guard: `@while true { }` hits 1000 and raises `MaxIterationError`
- Zero iterations: `@while false { }` produces no output
- Nested @while inside @for, and vice versa
- @while at top level and inside a rule block

**$var in selectors:**
- Variable as class name: `.$var` resolves correctly
- Variable as element name: `$tag { }` resolves correctly
- Variable adjacent to literal: `.col-$i` concatenates to `.col-3`
- Variable in @for loop generating selector families
- Undefined variable in selector: `UndefinedVariableError`
- Variable resolving to number in selector position
- Variable resolving to a value with spaces: TypeError

**@content blocks:**
- Basic @content with include block: content appears in output
- @content with no include block: no output, no error
- Include block with no @content in mixin: block silently ignored
- Multiple @content in one mixin: block duplicated
- @content with nested mixin includes
- Content block evaluated in caller's scope (not mixin scope)
- @content outside mixin: SyntaxError

**!default flag:**
- Variable not defined: !default sets it
- Variable already defined: !default is a no-op
- Variable defined as null: !default is still a no-op (variable exists)
- !default inside mixin/function scope
- !default combined with !global

**!global flag:**
- Variable set globally from inside a mixin
- Variable set globally from inside a @for loop
- Variable overwrites existing global
- Variable creates new global
- !global combined with !default

**Property nesting:**
- Basic font nesting: `font: { size: 14px; }` → `font-size: 14px;`
- Multiple children in one nesting block
- Nested property nesting: `border: { style: { top: solid; } }`
- Empty nesting block: no output
- Custom property (--var) with block: NOT treated as property nesting

**@at-root:**
- @at-root inline form: rule emitted at root
- @at-root block form: multiple rules at root
- @at-root inside deep nesting: all nesting escaped
- @at-root preserves @media context
- @at-root at top level: no-op unwrap
- & inside @at-root: warning emitted

**@extend and %placeholder:**
- Basic @extend with class selector
- @extend with %placeholder: placeholder removed from output
- Multiple rules extending same target: all selectors merged
- Chained/transitive extends: A extends B extends C
- ExtendTargetNotFoundError for missing target
- Placeholder defined but never extended: silently removed
- @extend inside @media: scoped to that @media block
- @extend with selector list targets

### Tier 2 features

**Maps:**
- Map literal parsing: `(a: 1, b: 2)`
- Trailing comma: `(a: 1, b: 2, )`
- Nested maps: `(outer: (inner: value))`
- Duplicate keys: last value wins
- Disambiguation: `(a: 1)` is a map, `(1 + 2)` is a grouped expression
- @each destructuring with map: `@each $k, $v in $map`
- Map as CSS value: TypeError
- All map-* functions: get, keys, values, has-key, merge, remove

**Color functions:**
- lighten/darken by various percentages
- saturate/desaturate
- adjust-hue with positive and negative degrees
- complement (verify 180-degree rotation)
- mix with default weight and custom weight
- rgba with color+alpha form and r,g,b,a form
- Channel extraction: red(), green(), blue(), hue(), saturation(), lightness()
- Invalid input types: TypeError
- Out-of-range amounts: RangeError
- Edge colors: pure black, pure white, fully saturated

**List/type/math functions:**
- nth: first, last, middle, out-of-bounds, zero index
- length: empty list, single item, multi-item, map (returns number of entries)
- join and append with space and comma separators
- index: found, not found (null)
- type-of: all seven types
- unit/unitless/comparable: various unit combinations
- math.div: normal division, division by zero, unit handling
- math.floor/ceil/round: positive, negative, already-integer
- math.abs: positive, negative, zero
- math.min/math.max: single arg, multiple args, incomparable units

### Cross-cutting tests

- All new features inside @if/@for/@each (control flow interaction)
- All new features inside mixins and functions
- All new features with @use (module interaction)
- Combination tests: @while with maps, @extend with @content, property nesting with
  variables
- Error recovery: verify that errors include file path and line number
- Round-trip: parse → transform → emit CSS for each feature, verify output

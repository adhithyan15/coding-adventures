# coding-adventures-css-parser (Lua)

A grammar-driven CSS3 parser that builds an Abstract Syntax Tree from CSS stylesheets. Uses the shared `css.grammar` grammar file and the `GrammarParser` from `coding-adventures-parser`.

## What it does

Given the input `h1 { color: red; }`, the parser produces:

```
stylesheet
└── rule
    └── qualified_rule
        ├── selector_list
        │   └── complex_selector
        │       └── compound_selector
        │           └── simple_selector → IDENT "h1"
        └── block
            ├── LBRACE "{"
            ├── block_contents
            │   └── block_item
            │       └── declaration_or_nested
            │           └── declaration
            │               ├── property → IDENT "color"
            │               ├── COLON ":"
            │               ├── value_list
            │               │   └── value → IDENT "red"
            │               └── SEMICOLON ";"
            └── RBRACE "}"
```

## Supported CSS features

### Selectors
- Type selectors: `h1`, `div`, `p`
- Class selectors: `.active`, `.btn-primary`
- ID selectors: `#header`, `#nav`
- Attribute selectors: `[disabled]`, `[type="text"]`, `[class~="warning"]`
- Pseudo-classes: `:hover`, `:nth-child(2n+1)`, `:not(.class)`
- Pseudo-elements: `::before`, `::after`
- Combinators: `>` (child), `+` (adjacent sibling), `~` (general sibling)
- CSS Nesting: `&`, `& .child`
- Universal: `*`
- Comma lists: `h1, h2, h3`

### At-rules
- `@import "file.css";`
- `@charset "UTF-8";`
- `@media screen { }` — with nested rules
- `@keyframes name { }`
- `@font-face { }`
- `@supports (...)` — with nested rules

### Declaration values
- Dimensions: `16px`, `1.5em`, `100vh`
- Percentages: `50%`, `100%`
- Colors: `#333`, `#ff0000`, `rgba(255, 0, 0)`
- Functions: `calc(100% - 20px)`, `linear-gradient(...)`, `var(--name)`
- URLs: `url(./image.png)`
- Strings: `"sans-serif"`, `'Arial'`
- Custom properties: `--main-color`, `--bg`
- `!important` priority

## Usage

```lua
local css_parser = require("coding_adventures.css_parser")

local ast = css_parser.parse("h1 { color: red; }")
print(ast.rule_name)  -- "stylesheet"

-- Inspect the tree
local function walk(node, depth)
    local indent = string.rep("  ", depth)
    if node.is_leaf then
        print(indent .. "Token(" .. node.token.type .. " " .. node.token.value .. ")")
    else
        print(indent .. node.rule_name)
        for _, child in ipairs(node.children) do
            walk(child, depth + 1)
        end
    end
end
walk(ast, 0)
```

## How it fits in the stack

```
css.grammar  (code/grammars/)
    ↓  parsed by grammar_tools
ParserGrammar
    ↓  drives
GrammarParser  (coding-adventures-parser)
    ↓  fed tokens from
css_lexer  (coding-adventures-css-lexer)
    ↓  combined by
css_parser  ← you are here
```

## Dependencies

- `coding-adventures-css-lexer` — tokenizes CSS source
- `coding-adventures-parser` — provides `GrammarParser`
- `coding-adventures-grammar-tools` — parses `css.grammar`
- `coding-adventures-lexer` — used internally by css-lexer
- `coding-adventures-state-machine` — used internally
- `coding-adventures-directed-graph` — used internally

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```

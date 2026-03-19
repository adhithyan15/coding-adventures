# @coding-adventures/grammar-tools

Define and validate `.tokens` and `.grammar` file formats for grammar-driven lexer and parser generation.

## What is this?

This package provides parsers and validators for two declarative file formats used to describe programming language syntax:

- **`.tokens` files** define the lexical grammar (what tokens exist)
- **`.grammar` files** define the syntactic grammar in EBNF (how tokens combine into valid programs)

Together, these files provide a complete, language-agnostic description of a programming language's surface syntax that can be used to generate lexers and parsers for any target language.

## Where it fits in the stack

```
.tokens file  -->  parseTokenGrammar()   -->  TokenGrammar
                                                   |
                                            crossValidate()
                                                   |
.grammar file -->  parseParserGrammar()  -->  ParserGrammar
```

The grammar-tools package is the foundation layer for the grammar-driven compiler pipeline. It reads and validates the specification files that describe a language, producing structured data that downstream tools (lexer generators, parser generators) consume.

## Usage

```typescript
import {
  parseTokenGrammar,
  parseParserGrammar,
  crossValidate,
  validateTokenGrammar,
  validateParserGrammar,
  tokenNames,
} from "@coding-adventures/grammar-tools";

// Parse a .tokens file
const tokenGrammar = parseTokenGrammar(`
  NUMBER = /[0-9]+/
  PLUS   = "+"
  NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/

  keywords:
    if
    else
    while
`);

// Parse a .grammar file
const parserGrammar = parseParserGrammar(`
  expression = term { PLUS term } ;
  term       = NUMBER | NAME ;
`);

// Validate each individually
const tokenIssues = validateTokenGrammar(tokenGrammar);
const parserIssues = validateParserGrammar(
  parserGrammar,
  tokenNames(tokenGrammar)
);

// Cross-validate: check that grammars are consistent with each other
const crossIssues = crossValidate(tokenGrammar, parserGrammar);
```

## API

### Token Grammar

- `parseTokenGrammar(source: string): TokenGrammar` -- Parse a `.tokens` file
- `validateTokenGrammar(grammar: TokenGrammar): string[]` -- Lint a parsed token grammar
- `tokenNames(grammar: TokenGrammar): Set<string>` -- Get all defined token names

### Parser Grammar

- `parseParserGrammar(source: string): ParserGrammar` -- Parse a `.grammar` file
- `validateParserGrammar(grammar, tokenNames?): string[]` -- Lint a parsed parser grammar
- `ruleNames(grammar: ParserGrammar): Set<string>` -- Get all defined rule names
- `grammarTokenReferences(grammar): Set<string>` -- Get all referenced token names
- `grammarRuleReferences(grammar): Set<string>` -- Get all referenced rule names

### Cross-Validation

- `crossValidate(tokenGrammar, parserGrammar): string[]` -- Check consistency between grammars

## Grammar Element Types

The parser grammar AST uses TypeScript discriminated unions. Each node has a `type` field:

| Type | Description |
|------|-------------|
| `rule_reference` | Reference to another grammar rule (lowercase name) |
| `token_reference` | Reference to a token type (UPPERCASE name) |
| `literal` | A literal string match `"..."` |
| `sequence` | Elements that must appear in order: `A B C` |
| `alternation` | Choice between alternatives: `A \| B \| C` |
| `repetition` | Zero-or-more: `{ x }` |
| `optional` | Optional: `[ x ]` |
| `group` | Explicit grouping: `( x )` |

## Development

```bash
npm install
npm test
npm run test:coverage
```

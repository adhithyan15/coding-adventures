# Excel Formula Grammar

## Summary

Excel formulas do have a published grammar. Microsoft documents SpreadsheetML
formula syntax in the `MS-XLSX` open specification, with cell formulas defined
as a restricted form of the grammar in section `2.2.2`.

This implementation now targets a broad Excel-formula grammar surface:

- Optional leading `=`
- Numbers, strings, booleans, and Excel error constants
- Array constants
- Function calls
- Arithmetic operators `+`, `-`, `*`, `/`, `^`
- Concatenation with `&`
- Comparison operators `=`, `<>`, `<`, `<=`, `>`, `>=`
- Postfix percent
- Parenthesized expressions
- A1-style references
- Sheet-qualified and workbook-qualified prefixes
- 3-D style prefixes
- Reference operators for range, union, and intersection
- Bang references and bang names
- Structured table references

## Source Notes

Microsoft Learn currently states that:

- `MS-XLSX` section `2.2.2.1 Cell Formulas` defines cell formulas as formulas
  that follow the grammar in section `2.2.2`
- Cell formulas must not use the `bang-reference` or `bang-name` production
  rules

Microsoft also acknowledged in a 2024 Microsoft Q&A thread that parts of the
published ABNF for structured references needed correction. This repository's
grammar includes structured references, but that area remains one of the most
fragile parts of the upstream specification.

## Scope

The goal of this package pair is to push the grammar-driven lexer/parser stack
close to the edge of what its current file formats can express.

Known limitations of the current architecture:

- The lexer cannot always distinguish bare names from bare column references at
  tokenization time, so some reference forms are modeled in parser context
  rather than with unique token types
- Bare row references overlap with numeric literals in the same way
- The official Excel grammar has context-sensitive cases around commas, spaces,
  names, and references that are difficult to represent exactly with the
  repository's current first-match token grammar plus PEG parser
- R1C1 references and newer niche/dynamic-array forms are still not modeled

## Parsing Model

The parser uses the repository's generic grammar-driven parser. The grammar is
layered by precedence:

1. Comparison
2. Concatenation
3. Addition and subtraction
4. Multiplication and division
5. Exponentiation
6. Prefix unary operators
7. Postfix percent
8. Primary expressions

That precedence structure is close to practical Excel behavior while remaining
compatible with the repository's current grammar tooling.

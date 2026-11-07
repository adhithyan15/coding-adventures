# VER04c — guarded Warrant source parser (P006b3)

Parent: [VER04 §2, §11](VER04-warrant-language.md), [VER04a](VER04a-warrant-source-admission.md), and [VER04b](VER04b-warrant-layout-capture.md).
This slice completes the protected Warrant frontend intake in `rust/warrant-grammar`.
It consumes the significant-layout token stream from VER04b and constructs a typed,
bounded Abstract Syntax Tree (AST) conforming to `warrant.grammar`.
It does not perform semantic type checking, solver proofs, code generation, or
package release authorization. Type checking and bindings follow in VER-P007.

## Parser architecture and consumption contract

The parser operates directly on the `&[LayoutToken]` sequence emitted by
`layout::capture_layout`, which in turn operates on an admitted `SourceDocument`.

```
Raw Bytes ──► [VER04a: SourceDocument::admit] ──► Admitted Document
                     │ (normalized CRLF, original byte mappings)
                     ▼
              [VER04b: layout::capture_layout] ──► LayoutToken stream
                     │ (INDENT / DEDENT / NEWLINE, zero-width origins)
                     ▼
              [VER04c: parser::parse_document] ──► WarrantAst
```

### Parsing Discipline
1. **Grammar Conformance:** Parses all constructs defined in the canonical `warrant.grammar`:
   - `file`: `warrant` header followed by indented declaration blocks.
   - Declarations: `model`, `error`, `op`, `example`, `claim`, `assume`, `labels`, `input`, `bound`.
   - Expressions: quantified (`forall`/`exists`), conditional (`if-then-else`), match expressions,
     implications, conjunctions, disjunctions, comparisons, arithmetic, unary operations, and primaries.
2. **Span Preservation:** Every AST node preserves its source span (either real normalized range or
   synthetic zero-width boundary), allowing exact mapping to original source coordinates via the
   underlying `SourceDocument`.

## Budgets, recursion guards and failure policy

Parsing untrusted or adversarial specifications requires strict structural bounds:
1. **Node Budget (`max_nodes`):** Default 65,536 nodes; hard ceiling 1,048,576. Every AST node
   creation increments an accounted node counter. Exceeding this budget rejects immediately with
   `ParserErrorKind::NodeBudget`.
2. **Recursion Depth Guard (`max_depth`):** Default 64; hard ceiling 128. Every recursive call
   decrements the available depth. Reaching 0 rejects immediately with `ParserErrorKind::RecursionDepth`.
   This provably prevents call-stack exhaustion (CWE-674) regardless of grammar nesting.
3. **Parse Work Budget (`max_parse_work`):** Default 3 * token_count; hard ceiling 3,145,728.
   Every step advances a work counter. Exceeding work rejects with `ParserErrorKind::Work`.

### Closed Error Model
Every parsing failure returns stable code `W0001`, a closed `ParserErrorKind` enum, and an
optional half-open raw byte interval:
- `InvalidLimits`: Configuration exceeds ceilings or is zero.
- `Work`: Scan/parse work units exceeded.
- `NodeBudget`: AST node allocation quota exceeded.
- `RecursionDepth`: Call stack recursion budget exceeded (nesting > `max_depth`).
- `UnexpectedToken`: Found unexpected token or syntax violation.
- `UnexpectedEof`: Premature end of token stream.
- `Allocation`: Memory reservation failure.

Neither `Display` nor `Debug` echoes raw token text or source contents.

## Acceptance and verification

Authored test cases must cover:
1. Complete validation of all 57 cases in `warrant-syntax-v1/cases.tfv`.
2. Strict enforcement of `max_depth` rejecting nested expressions exceeding configured depth.
3. Strict enforcement of `max_nodes` rejecting oversized AST constructions.
4. Parsing of all declaration shapes: models, operations, claims, examples, assumptions, labels, inputs, and bounds.
5. Error diagnostics asserting opaque strings (zero source leakage) and stable `W0001` codes.
6. Minimum **≥90% line coverage and ≥90% branch coverage**.

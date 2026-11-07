# VER04b — bounded Warrant significant-layout capture (P006b2)

Parent: [VER04 §2](VER04-warrant-language.md#2-source-and-identity) and [VER04a](VER04a-warrant-source-admission.md).
This slice supplies exact significant-layout capture, indentation tracking and
synthetic token origin mapping in `rust/warrant-grammar`. It operates directly on
an admitted `source::SourceDocument` (from VER04a). It does not perform full AST
parsing, type checking, proof generation or package release authorization.
The guarded parser and recursive node budgets remain P006b3.

## Layout model and token origins

Warrant source uses significant indentation (Python/Starlark style) to delimit
declaration blocks (`assume:`, `claim:`, `example:`, `scenario:`, `match`, etc.).
Standard grammar lexer range capture (`tokenize_with_ranges` in `code/packages/rust/lexer`)
deliberately returns `Err(LexicalError::Unsupported("indentation/layout synthesis"))`
because synthetic tokens (`INDENT`, `DEDENT`, `NEWLINE`) do not correspond to
contiguous non-empty raw lexeme spans in the source text.

P006b2 establishes the origin contract for layout-bearing tokens:

1. **Real tokens:** Tokens matched against source text (`NAME`, `KEYWORD`, literals,
   punctuation) possess a non-empty half-open normalized byte range `[start, end)`.
   Their original coordinates and raw byte slice are retrieved directly via
   `SourceDocument::original_span(start, end)`.
2. **Synthetic layout tokens:** `INDENT`, `DEDENT` and synthetic trailing `NEWLINE`
   tokens are transitions emitted when indentation changes or blocks close. They
   possess an exact **zero-width origin boundary** `at` in the normalized buffer.
   Their `original_span(at, at)` produces a zero-width span where start and end
   `OriginalPosition` coincide exactly at that boundary.
3. **No fictitious positive-width spans:** A synthetic token must never be assigned
   a positive-width byte range spanning indentation spaces, newlines or comments.
   Indentation whitespace is trivia preceding the line's first token.

## Indentation rules and stack discipline

1. **Spaces only:** Leading indentation must consist entirely of ASCII spaces (`0x20`).
   Tabs (`\t`) in indentation are strictly forbidden and trigger immediate rejection.
2. **Initial indentation:** The document starts at indentation level 0. The stack
   maintains the sequence of active indentation column depths.
3. **Indentation depth limit:** The indentation stack is bounded by `max_indent_depth`
   (default 64, hard ceiling 256). Exceeding this depth rejects with a budget error.
4. **Line classification:**
   - Blank lines (only whitespace) and comment-only lines (lines where the first
     non-whitespace characters are `--`) are ignored for indentation tracking.
     They do not alter the indentation stack or emit `NEWLINE`, `INDENT` or `DEDENT` tokens.
   - Non-blank lines are measured by the number of leading ASCII spaces before the
     first real token or comment.
5. **Indentation comparison:**
   - If line indentation is greater than the current top of the stack, push the new
     indentation column and emit an `INDENT` token located at the boundary of the line's
     first real token.
   - If line indentation is equal to the top of the stack, emit a `NEWLINE` token
     separating statements (if not at document start).
   - If line indentation is less than the top of the stack, pop levels until a matching
     outer indentation column is found. For each popped level, emit a `DEDENT` token
     located at the boundary of the line's first token. If no matching level exists,
     reject immediately as an inconsistent dedent.
6. **End-of-file closure:**
   - At normalized EOF, if the indentation stack has depth greater than 1 (level 0),
     pop all remaining levels and emit one `DEDENT` token per level at normalized EOF.
   - If the token stream does not end with `NEWLINE` and non-layout tokens were emitted,
     emit a final `NEWLINE` at EOF.

## Limits, budgets and failure policy

`LayoutLimits` configures bounded execution:
- `max_tokens`: Default 65,536 tokens, hard ceiling 1,048,576.
- `max_indent_depth`: Default 64, hard ceiling 256.
- `max_scan_work`: Default 3 * normalized_bytes.

Before layout capture begins, conservative memory and work reservations are made.
Every failure uses stable code `W0001`, a closed `LayoutErrorKind` enum and an
optional half-open raw byte interval:
- `InvalidLimits`: Configuration exceeds hard ceilings or has invalid combinations.
- `Work`: Scan work units exceeded.
- `TokenBudget`: Emitted tokens exceeded configured cap.
- `IndentDepth`: Indentation nesting exceeded `max_indent_depth`.
- `TabInIndentation`: Tab character found in leading indentation.
- `InconsistentDedent`: Dedent level does not match any enclosing block level.
- `Allocation`: Allocation failure when reserving token or stack buffers.

Diagnostics, `Display` and `Debug` implementations must never echo source text or
token values. Failure is an explicit layout-stage refusal, never a partial token stream.

## Acceptance and verification

Authored test cases must cover:
1. Flat documents (level 0 only).
2. Deeply nested blocks up to `max_indent_depth` and rejection at depth + 1.
3. Blank lines, comment-only lines, and interleaved trailing comments.
4. Single-level and multi-level dedents.
5. Rejection of tabs in leading indentation.
6. Rejection of inconsistent dedent columns.
7. Correct mapping of both real token spans and zero-width synthetic token origins
   to original byte coordinates across LF and CRLF sources.
8. Nonreproducing diagnostics and Debug outputs under all failure modes.

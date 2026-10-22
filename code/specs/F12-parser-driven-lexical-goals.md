# F12 — Parser-driven lexical goals

**Status:** proposed 2026-10-21. Phase 0 is queued. Claude settled the open
questions provisionally on 2026-10-22 (see *Decisions* at the end); the owner
has not confirmed them yet. Builds on [F04](F04-lexer-pattern-groups.md)
(pattern groups) and [F10](F10-declarative-lexer-modes.md) (declarative modes),
and resolves F10's "Residual Hard Cases". It replaces the heuristic context
stack tried on the local branch `claude/closurec-regex-after-brace`, which was
never pushed.

## The problem

ECMAScript does not have one lexical grammar. It has several *goal symbols*,
and the syntactic grammar picks one at each point (ECMA-262 §12):

```text
InputElementDiv                   / and /= are operators
InputElementRegExp                / starts a regular-expression literal
InputElementRegExpOrTemplateTail  … or } continues a template
InputElementTemplateTail          } continues a template, / divides
InputElementHashbangOrRegExp      start of a Script or Module
```

The spec puts it plainly: the goal is "whichever is permitted by the syntactic
grammar at that point". Only the parser knows that.

Today closurec lexes the whole file before parsing. The ES2025 `.tokens` file
guesses the goal with F10's flat token-to-mode table. The guess is right after
almost every token, but wrong after `}` and after `if (…)`. A wrong guess either
rejects a valid program or silently compiles it to a different one:

```text
{ a }⏎/foo/g.exec(s)          → SIMPLE: ({a})/foo/g.exec(s)     (miscompile)
function f(){ return 1 }⏎/x/g.test(s)  → parse error             (rejection)
```

### Why not a smarter lexer

Branch `claude/closurec-regex-after-brace` (fix B) ported Acorn's
token-context stack into `javascript-lexer` and went through three
security-review rounds. Each round found a new desync, and each fix needed
more of the parser's knowledge inside the lexer:

| Round | Desync | Knowledge the lexer had to grow |
|---|---|---|
| 1 | `o = {}⏎{…}⏎/re/`, `static {`, `catch {`, `export default {` | ASI, class bodies, operand keywords |
| 2 | `Array.of⏎{…}⏎/re/`, `var await⏎{…}` | for-of heads, async function bodies |
| 3 | `class A { async() { await⏎{…} } }` | method-name position vs `async` modifier |

The rounds did not converge, because the lexer was slowly re-deriving the
parser. Some cases it cannot decide without the parser at all: computed
`async` method keys, expression-bodied async arrows, and module top-level
`await`. The decision belongs to the parser.

## How Closure does it

These names were checked against the oracle jar (v20260915) with `javap`:

- `Scanner.nextToken()` always scans `/` and `/=` as the `SLASH` and
  `SLASH_EQUAL` operators. The scanner never guesses.
- `Parser.parsePrimaryExpression` reaches `parseRegularExpressionLiteral()` on
  `SLASH`/`SLASH_EQUAL`. That calls `Scanner.nextRegularExpressionLiteralToken()`,
  which runs `clearTokenLookahead()` and rescans from the slash as a regex body
  (`skipRegularExpressionBody`, …).
- Template continuation works the same way. After a substitution, the parser
  calls `nextTemplateLiteralToken()` to rescan the `}`.
- ASI is inline: `eatPossiblyImplicitSemiColon()`.

## Design: relex on mismatch

closurec's parser is the shared packrat `GrammarParser`, not a hand-written
one, so this design keeps the grammar in charge. When the grammar expects
terminal **T** at token index *i*, finds token **U** there, and the grammar
declares that **U** may be re-read as **T** in lexer mode **M**:

1. The parser asks its token source to re-lex from the byte offset where
   token *i* starts, forcing the first token into mode **M**.
2. If the result is a **T**, the match succeeds and the parse goes on.
3. Tokens after *i* are re-lexed lazily, as the parser reaches them.

The engine already does this for one case. `split_angle_bracket_run` splits a
`>>` token when a generic closer `>` is expected. It records an undo entry,
invalidates the affected memo entries, and is undone on backtrack. F12
generalizes that mechanism from "split a token" to "re-lex from here".

### 1. Declaring relex rules (grammar data, not code)

A new `relex:` section in `.grammar` files:

```text
relex:
  REGEX           from SLASH | SLASH_EQUALS  in mode default
  SLASH           from REGEX                 in mode div
  SLASH_EQUALS    from REGEX                 in mode div
  TEMPLATE_MIDDLE from RBRACE                in mode template
  TEMPLATE_TAIL   from RBRACE                in mode template
```

The modes are the F10 groups that `es2025.tokens` already declares. grammar-tools
validates the section: terminals and modes must exist, and the F10 security
caps apply. A grammar without the section behaves exactly as today.

Rules go in both directions, so the F10 table remains a **first guess**. It is
right almost everywhere, so a typical file needs no re-lexing at all, but it
is no longer trusted. The parser is the authority.

### 2. Resumable lexing (`lexer` crate)

- `GrammarLexer::snapshot() -> LexerState` and `restore(LexerState)`. The
  state is the byte position, line and column, group stack, bracket depths,
  template entry depths, last emitted token and skip flag.
- `next_token_in_mode(Option<&str>)` lexes exactly one token. A mode, if
  given, overrides the active group for that token only.
- **Errors are deferred.** A wrong guess can run into text that does not lex.
  For example, `/'/` read as division opens a string that never closes. The
  lazy source turns that into an error token that only fails the parse if the
  parser actually needs it. Today the whole tokenize aborts before the parser
  can correct anything.

### 3. A token source behind `GrammarParser` (`parser` crate)

- `tokens: Vec<Token>` becomes a buffer, filled through an optional
  `Box<dyn TokenSource>`. Vec-built parsers (every other grammar) are
  unchanged.
- `TokenSource::fill_to(i)` and `relex_at(i, mode) -> Option<Token>`.
  `relex_at` truncates the buffer at *i*, restores the snapshot for token *i*,
  and lexes one token in `mode`.
- Snapshots are stored only for tokens of a relex `from` kind (`SLASH`,
  `SLASH_EQUALS`, `REGEX`, `RBRACE`), which keeps memory bounded by those
  tokens.
- **Undo.** `split_undo_log` becomes a general `token_undo_log` that also
  holds `Relex { pos, previous_mode }`. `restore_to` undoes an entry by
  re-lexing at `pos` in the previous mode. Lexing is deterministic, so the old
  suffix never has to be stored.
- **Memo.** Use `memo.retain(|_, e| e.end_pos < pos)`, the same rule as
  `set_token_and_invalidate_memo`. `furthest_pos` and the expected set are
  reset when they lie at or past `pos`.
- Positions stay token indices. AST positions come from token line and column
  as today.

### 4. ASI

`asi.rs` retries the parse after inserting a synthetic `;` *before a token
index*. Re-lexing can shift indices, so insertions are keyed by the **byte
offset** of the following token instead. The source injects them on every
(re)lex. The restricted-production pre-pass is keyed by byte offset in the
same way.

Precedence follows §12.10 automatically. Re-lexing happens *during* a parse
attempt, and ASI only after one fails. So `a⏎/b/g` stays a division, because
the `/` legally continues the expression. That is exactly the case where
§12.10 forbids inserting a semicolon.

### 5. Prerequisite: the ExpressionStatement lookahead

ECMA-262 §14.5 says an `ExpressionStatement` may not start with `{`,
`function`, `class`, `let [` or `async function`. The ES grammars do not
enforce this, so the parser accepts `{a}/foo/g.exec(s)` as the expression
`({a})/foo/g.exec(s)`. If both readings parse, the parser has no mismatch to
trigger a re-lex. The restriction (a negative lookahead in `.grammar`) must
land first.

On its own it turns the silent-miscompile class into fail-closed refusals at
SIMPLE and ADVANCED. Several fix-B review findings had this as their
amplifier.

### 6. Token consumers outside the parser

closurec tokenizes directly in `whitespace_only.rs`, `refusals.rs`,
`defines.rs`, `print_tree.rs` and `run.rs`. They would keep seeing the guessed
stream, so they move to the **resolved** stream: the buffer exactly as the
successful parse consumed it. The new entry point is
`javascript_parser::parse_javascript_resolved(source, version) -> (ast, tokens)`.

WHITESPACE_ONLY then needs a successful parse, and fails closed if there isn't
one. Closure's WHITESPACE_ONLY parses too, so this is parity. It also removes
the amplifier behind #17939, where a token-only emitter turns any lexer desync
into code.

### 7. Cost and denial of service

- One re-lex costs O(1) plus lazy re-lexing of whatever suffix the parser goes
  on to read. A memo invalidation is O(memo size).
- Backtracking can re-lex the same index more than once. Cap the total at
  `4 × (relexable tokens) + 64` re-lexes. Going over the cap fails closed with
  a parse error, never a guess.
- With the F10 guess in place, real files need re-lexing only after
  mispredicted `}`/`)` slashes, which is rare.

## What changes for existing pieces

| Piece | After F12 |
|---|---|
| F10 regex/div table in `es2025.tokens` | Kept, as the first guess |
| F10 Residual Hard Cases 1–3 | Resolved by the parser |
| Template brace-depth override in `GrammarLexer` | Kept as the guess; the relex rules correct `${ /}/ }` |
| Fix B's `brace_context.rs` and `LexerContext::set_mode` | Not needed; the branch stays unmerged |
| `asi.rs` | Insertions keyed by byte offset |

## Phases

One PR each, every one under about 1,500 lines:

0. **Grammar:** the ExpressionStatement lookahead (§5). Regression fixtures:
   `{ a }⏎/foo/g.exec(s)` must fail to parse rather than miscompile. Lands
   independently and immediately closes the silent-miscompile class.
1. **lexer:** `snapshot`/`restore`, `next_token_in_mode`, deferred errors. No
   behaviour change.
2. **parser and grammar-tools:** the `relex:` section (parse, validate, compile
   into the Rust compiled grammars), the `TokenSource`, the undo log and memo
   invalidation. Tested on a toy grammar.
3. **javascript:** the relex rules in `es2025.grammar`, the lazy source in
   `javascript-parser`, and byte-offset ASI. Every input from fix B's tests
   and its three review rounds becomes a parser fixture, checked against the
   Closure jar and node, and the test262 grow-only list must not regress.
4. **closurec:** token consumers use the resolved stream; WHITESPACE_ONLY
   parses.
5. **Optional:** template relex rules; the other editions (ES5–ES2024 have no
   `div` mode; give them one, or let them rely on re-lexing).

## Decisions

These were open questions when the spec was proposed. On 2026-10-22, Claude
settled them provisionally while restarting the Closure loop, using the
spec's own recommendations. The owner has not reviewed them. They follow the
owner's standing direction to match Closure's observable behavior (CCR-075).
The owner can overrule any of them before phase 1 starts.

1. **Keep the F10 guess as the first guess.** The parser corrects it by
   re-lexing. Always scanning `/` as division, as Closure does, is simpler,
   but every regex would then need a re-lex, and any consumer still reading
   raw tokens would be wrong on every regex.
2. **WHITESPACE_ONLY fails closed on input that does not parse.** Closure's
   WHITESPACE_ONLY parses too, so this is parity.
3. **Rust goes first.** Other grammar-tools ports ignore the `relex:` section
   and keep today's behavior until they implement it.
4. **The module goal stays out of scope** (top-level `await`, `export`). The
   parser reads scripts.

# CV04 — exact lexical source intervals

Tracking: [CCR-065 source foundation #16970](https://github.com/adhithyan15/coding-adventures/issues/16970), within [provenance epic #15830](https://github.com/adhithyan15/coding-adventures/issues/15830).

Status: specification first; acceptance is divided into CV04a (capture and domain
validation) and CV04b (compiler ingestion/leaf reuse and serialized source
inventory). Neither sub-slice closes the full provenance or compiler epic.

## Evidence and route audit

On `c0c7b8bc04d972e07348d02143b6f8b867eee1ae`, JavaScript's generated
`_grammar::token_grammar` returns ordinary `TokenGrammar` data. `GrammarLexer`
compiles it to anchored regex patterns; its standard matcher is the JavaScript
route. `tokenizer_dfa` and the handwritten tokenizer are distinct APIs, not
alternate execution branches inside this route. Standard named groups,
declarative mode transitions, keyword promotion, escape processing and template
substitution depth all use this same cursor. Indentation/layout dispatch and
user callbacks/pre/post hooks have separate synthesis or transformation behavior.

The standard matcher owns `matched` before quote stripping or value processing,
then advances its UTF8 byte cursor. Skip matches and fallback whitespace advance
the same cursor. EOF bypasses callbacks. Therefore callbacks alone cannot certify
coverage. Existing Token line/column are 1-based scalar counters which advance
lines only on LF; they must not be reinterpreted as ECMAScript coordinates.

CLOC27 already stamps lexer CV IDs onto parser tokens and bridge leaves. However,
`run.rs` derives ingestion token IDs from an input root, then SIMPLE/ADVANCED
`parse_javascript_typed_with_cv` tokenizes again and creates independent origins.
It currently reports `lexeme_byte_len = token.value.len()`: quotes have been
removed, and the JavaScript grammar preserves escape spelling. The existing
native raw-span observation shows `"A\nB"` has six raw bytes but four value
bytes; a lambda/emoji literal has eight raw bytes and six value bytes. Existing
maps have empty mappings. Source identity is currently the SHA256 of the exact
UTF8 string returned by `read_to_string`, not a proof about earlier decoded or
transformed buffers. This run path has no input decoding/normalization adapter;
future adapters must provide an explicit new buffer domain and relation.

## CV04a: additive capture API and owned types

Add `lexer::lexical` with repository-owned position, range, piece, limit, error,
and snapshot types; no new dependency and no required field on `Token`.
`GrammarLexer::tokenize_source_with_ranges(source, grammar, limits)` admits source
and limits before constructing the lexer's character vector. An instance method
`tokenize_with_ranges(limits)` admits additional range ownership before lexing;
the ordinary constructor has already created its existing character vector.
Both share the actual standard tokenizer. A snapshot borrows its exact source
buffer and privately owns unchanged tokens and ordered range descriptors. Expose
read-only source/tokens/pieces access and consuming token extraction.

This mode requires a fresh lexer, standard dispatch, borrowed unmodified source,
and no on-token/pre-tokenize/post-tokenize callbacks. Reject unsupported modes,
hooks, callbacks and reuse before executing hooks/callbacks or advancing the
cursor. Declarative grammar transitions remain supported because they change
matching modes without synthesizing source bytes. Legacy tokenize APIs retain
their existing behavior, callback dispatch, line/column, values, CVs and flags.
The handwritten and standalone DFA APIs make no new raw-span claim.

Each successful standard match produces a Token piece referring to the exact
emitted token ordinal, before quote/escape/case transformations. A default LF
Newline token is also a Token piece. Each accepted skip match or fallback skipped
character produces a Trivia piece. Do not guess comments versus whitespace from
their bytes, merge pieces, or call skipped trivia deleted optimizer nodes. The
last piece is Eof referencing the last token: one zero-width point at the source
length, including empty source. Every preceding piece has positive byte length;
pieces exactly partition [0, source.len()), with no gap, overlap or reordering.
The token ordinal sequence is contiguous and includes the EOF token exactly once.
Capture occurs at acceptance in the actual cursor, not by searching token values
back through source. Zero-length regex/skip matches reject in capture mode, so a
malformed trusted grammar cannot create a non-advancing capture loop.

## Coordinates and source-bound domain validation

`SourcePosition` has `byte_offset`, `scalar_offset`, `utf16_offset`, `line`,
`scalar_column`, `utf16_column`, all zero-based nonnegative `usize` values within
admitted bounds. Byte offsets address UTF8 source; scalar offsets count Unicode
scalar values; UTF16 offsets count code units, so an astral scalar advances by
two. Positions are boundaries, not byte values. `SourceRange` is half-open
[start,end); every byte endpoint must be a UTF8 boundary.

The independent coordinate cursor recognizes LF, bare CR, U+2028 and U+2029.
CRLF increments the line once: immediately after CR the position is the next
line at column zero, and after the following LF it remains that line/column.
Absolute scalar/UTF16/byte offsets count both CR and LF. Other scalars increment
both columns by their scalar/code-unit lengths. This cursor must not change the
legacy Token location or ASI flags.

The coordinate choice follows [ECMA-426 terms](https://tc39.es/ecma426/#sec-terms-and-definitions)
(UTF16 columns) and [ECMA-262 line terminators](https://tc39.es/ecma262/multipage/ecmascript-language-lexical-grammar.html#sec-line-terminators)
(LF/CR/LS/PS, CRLF as one sequence), read 2026-10-07. This is a source-coordinate
contract; real map generation and range-to-output joins are later work.

`validate_lexical_partition(source, token_count, pieces, limits)` validates
untrusted detached descriptors against the supplied UTF8 bytes before returning
success. It must reject invalid limits, budget excess, gaps/overlaps, reversed
or non-boundary ranges, incorrect start/end coordinates, repeated/skipped token
ordinals, absent/multiple/nonterminal/incorrect EOF, and incomplete byte coverage.
Walk source and descriptors monotonically in O(bytes + pieces), with bounded
additional state; do not rescan prefixes for each token. It validates partition
and coordinate structure. It does not prove that the grammar recognized a Token
versus Trivia, authenticate a file/path/hash, or validate generic CV metadata.
Private successful snapshots are the stronger actual-matcher evidence. An
external claimant needs both domain validation and a source-content identity.

## Resource and failure contract

`LexicalLimits` contains finite source-byte, piece-count, retained-descriptor-byte,
and work-unit caps. Defaults: 8 MiB source, 262144 pieces, 64 MiB retained range
descriptors and 32 Mi work units. Hard configuration ceilings: 128 MiB source,
1048576 pieces, 256 MiB descriptors and 512 Mi work. Lower caps, including zero,
are legal; requests needing more fail. Configuration values above ceilings reject.

Capture reserves source byte count as a conservative bound on coordinate cursor
work before iteration; one additional work unit is reserved before each piece.
Standalone validation charges source byte count plus piece count before scanning.
All additions, growth and byte-size multiplication are checked. Reserve a piece
slot and bounded geometric vector capacity before allocating a matched value or
emitting a token; account capacity * size_of(piece), not merely length. Grow only
within both count and descriptor caps using fallible reservation. These are
logical bounds on descriptors/cursor work; regex engine cost, existing Token
values, grammar compilation and allocator bookkeeping are separate, source-
bounded existing lexer costs, not falsely included in the descriptor counter.
No source clone or per-token raw-source copy is added. Cursor arithmetic is safe
under the source ceilings and must never wrap. No unsafe code is introduced.

Capture errors return no successful snapshot; discard the local descriptor
recorder and prohibit lexer reuse. Lexical malformed input remains an ordinary
lexer error; unsupported or resource errors are distinguishable. Domain
validation is read-only and cannot mutate the descriptor/source input. CV04a
changes no graph/clock state or compiler publication behavior.

## CV04b: required follow-up before #16970 acceptance

Before implementing CV04b, commit the precise serialized namespace, scalar
encoding and consumer admission specification. Compiler ingestion must capture
before source-root/token ownership, bind intervals to the actual inspected
buffer identity, record the complete lexical inventory including trivia/EOF,
and declare lexical unavailability on recoverable lex failure. Resource/invalid
recording failures must fail closed before publication, preserving the complete
old output set. Tracing off must retain ordinary behavior.

Carry the exact ingestion token CVs through the identical typed/ASI parse into
existing CLOC27 leaves. Do not remint leaf origins, relabel synthetic ASI tokens
as consumed bytes or change optimizer semantics. Generic graph/journal validation
must remain distinct from a source-domain validator that requires the source
bytes and verifies the complete declared namespace/inventory/root relation.
This follow-up needs exact raw serialization/import/domain-negative tests and
actual compiler subprocess neutrality for JS/map/manifest and connected primitive
fold lineage; it is deliberately not claimed by an additive unused range API.

## Test and review acceptance

Commit this spec before red tests and source changes. CV04a tests must run actual
standard match/skip acceptance and compare tokens/flags/CVs with legacy output:
quotes, escapes, case normalization, astral/BMP scalars, CRLF/bare CR/LS/PS,
blank lines, trailing trivia, multiline templates, regex/division modes, nested
template substitutions, default whitespace/Newline, EOF and empty source.
Exercise every supported ECMAScript version and named/declarative modes. Reject
zero-length matches, hooks/callbacks/modes/reuse before side effects. Boundary
controls cover exact and one-less source/piece/descriptor/work limits,
configuration ceilings, allocation failure handling where deterministically
injectable, and malformed detached descriptors including huge offsets/counts.

Run the affected consumer suites and strict all-target lint. Update package
README/changelog and durable backlog/issues. Independent security subagent review
is required before push. Keep one active shared-stack PR and require exact-head
terminal acceptable conflict-free/review-clean native CI evidence on all three
platforms, then expected-head squash merge and fetched-main blob/mode verification
before archival. CV04a acceptance selects CV04b; #16970 remains open until both
contracts pass. Composite/occurrence identities, remaining fold families,
inline/rename/motion/deletion, output joins and full compiler parity remain open.

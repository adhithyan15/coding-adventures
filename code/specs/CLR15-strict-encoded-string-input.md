# CLR15: strict encoded string input

Status: implementation contract, committed before production changes.

## Scope and representation

Extend the opt-in strict encoded CLR path with the zero-argument `input_str`
builtin. Reserve MemberRef row 8 immediately after CLR13's integer-input and
peek rows. The simulator represents a managed string as a copyable arena handle
distinct from object-array references and integer values; the arena owns the
exact input bytes. Expose a checked read-only byte view so tests and callers can
observe returned strings without exposing mutable arena storage.

The strict lowerer accepts `str` in locals, parameters, returns, `mov`, direct
calls, and `call_builtin input_str`. It emits CLR metadata type `string` and the
exact reserved MemberRef. String constants, arithmetic, comparisons, branches,
general string operations, and every other string builtin remain fail-closed.
Existing i32/i64/bool validation and CLR13 builtins retain their exact types.

## Input contract

`input_str` consumes one line from the simulator-owned byte input configured by
`set_input`. LF and CRLF delimiters are consumed and excluded; a final
unterminated line is returned intact. Spaces, tabs, non-ASCII bytes, embedded
carriage returns not immediately before LF, and all other content bytes are
preserved exactly. EOF returns an allocated empty string. Repeated EOF reads
remain empty, and `input_more` continues to peek without consuming.

Loading a program clears execution and value arenas but does not clear or rewind
the input stream. Replacing input resets only the input cursor; already-returned
strings remain valid until the next program load. Unknown MemberRefs continue
to refuse before changing execution or input state.

## Validation

Use independently assembled literal call bytes to prove LF, CRLF, final-line,
empty-line, non-ASCII and repeated-EOF behavior; sequential mixed
`input_more`/integer/string reads; exact-token refusal; and string-arena
lifecycle. Prove strict lowering emits row 8 with `string` metadata, transports
strings through locals and direct calls, executes an artifact in the simulator,
and refuses malformed or out-of-scope string shapes. Run the full simulator and
encoded-backend suites, focused lang-aot execution, strict Clippy, and diff
checks.

## Exclusions

Default `lower_iir_to_cil` and source routing retain their existing input and
int32-model gates. String literals and operations, nullable CoreCLR EOF parity,
PE metadata/host packaging, byte-oriented Brainfuck input, and general callback
registration remain separate work.

# Reuse one standard lexer cursor

Add an opaque zero-value `TokenCursor` for callers that repeatedly tokenize with
one compiled grammar and already own their token buffer. The cursor reuses
per-run lexer state, clears its references after success and lexical errors, and
keeps indentation/layout grammars on the owning path. `Compiled` remains
immutable and shareable across independent cursors.

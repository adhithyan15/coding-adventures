# Expose bounded line tokenization with a reusable lexer cursor

Add `TokenizeLineMaxIntoWithCursor`, which preserves the existing compile,
length, and newline checks before reusing caller-owned lexer state and token
storage. The owning and existing caller-buffer entry points keep their behavior.

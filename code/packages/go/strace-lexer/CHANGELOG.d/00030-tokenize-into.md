# Add caller-buffer line tokenization

Added `TokenizeLineMaxInto`, preserving byte-length and newline refusal order
while using the shared compiled lexer's caller-buffer API. Existing
`TokenizeLine` and `TokenizeLineMax` remain owned-output entry points.

# Validate projected metadata before dropping it

Some report readers return a core analysis object and intentionally omit
producer metadata. Validate each present metadata field at the input boundary
before constructing that reduced object; otherwise malformed input can pass
replay merely because the public result does not retain the field.

For a text-based JSON reader, validate the decoded workload identifier as a
Unicode scalar string. Reject lone UTF-16 surrogate escapes, while accepting
valid surrogate pairs, U+FFFD, and literal text that only resembles an escape.
Define blankness using the contract's exact character set instead of a broad
Unicode trim operation.

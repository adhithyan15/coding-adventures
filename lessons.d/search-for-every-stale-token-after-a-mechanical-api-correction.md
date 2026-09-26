# Search for every stale token after a mechanical API correction

After replacing a nonexistent helper name, one generic call site remained and
the next compile failed on the same typo. Follow mechanical API corrections
with an exact-token search from the correct repository root, confirm zero
matches, and only then rerun the compiler.

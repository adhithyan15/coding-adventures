---
category: Cross-platform & Windows BUILD_windows
---

# Emitted-C conformance helpers must omit Unix libm on Windows

Six actual `c-to-semantic-ir` consumer tests failed during CV04a's broad lexer
validation. Both emitted-C compiler helpers passed `-lm` unconditionally, while
the reference-C helper already omitted it on Windows. Initial suspicion was a
missing native linker environment, but a complete native probe disproved that:
Clang compiles/links the same C program successfully without `-lm`, and adding
only that flag returns 1181 with `LNK1181: cannot open input file m.lib`.
Windows CRT already supplies the required math functions; Unix needs libm.

Apply the existing reference-leg platform rule consistently to both emitted
legs. Preserve all original generated-C and Ruby runtime equivalence assertions;
do not hide the compiler failure or skip conformance. Commit the repair spec
before test-helper changes and rerun the full original package suite on Windows.
Keep full raw probe diagnostics outside the source tree. Evidence: local
`CV04a-clang-link-probe.log` and `CV04a-clang-link-libm-probe.log`.

The first lesson CLI path was shortened from its full title, and a writer
mistakenly guessed the longer filename. Use the exact returned path, resolve it
inside `lessons.d`, and consolidate only agent-created duplicate scaffolds after
preserving their content in the correct shard.

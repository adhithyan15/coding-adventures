- The 23 generated `book/book.tex` entrypoints are no longer tracked. The book
  generator retains their exact in-memory projection, rejects any returned root
  resurrected in the curriculum tree, and materializes roots only inside the
  caller-created empty compile-input directory.
- Book loading now recognizes authored `frontmatter.tex`, `backmatter.tex`, and
  chapter sources without requiring the projected root on disk. The shared
  compiler, CI publication path, local verification scripts, and Spanish build
  wrappers all consume the isolated entrypoint.
- The legacy parallel warning reporter now delegates compilation to the shared
  hardened gate and retains warning failures in the parent shell instead of
  losing them through a pipeline subshell.

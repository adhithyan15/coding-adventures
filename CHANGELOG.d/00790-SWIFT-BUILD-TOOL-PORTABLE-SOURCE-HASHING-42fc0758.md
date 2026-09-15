### Swift build-tool portable source hashing

- Completed the primary/reference source and package-metadata registry for
  every established language lane plus OCaml. Strict declared-source mode now retains root
  variable-name manifests such as `.gemspec`, `.rockspec`, `.cabal`, `.opam`,
  `.csproj`, and `.fsproj`, while applying the complete case-sensitive
  26-component generated-directory registry before selection.
- Replaced content-digest concatenation and the Windows FNV fallback with the
  repository-local cross-platform SHA-256 implementation. Package digests now
  frame every sorted portable repository-relative UTF-8 path and exact raw file
  body with unsigned 64-bit big-endian lengths.
- Made missing, unreadable, linked, reparse-backed, or unstable source inputs
  fail closed with CLI exit `2` and one root-redacted, Unicode-control-safe
  `HASH_PACKAGE_FAILED` diagnostic. Reads stay bound to verified no-follow
  handles while retained ancestor identities prevent plan paths from escaping
  through a symbolic link or Windows junction.


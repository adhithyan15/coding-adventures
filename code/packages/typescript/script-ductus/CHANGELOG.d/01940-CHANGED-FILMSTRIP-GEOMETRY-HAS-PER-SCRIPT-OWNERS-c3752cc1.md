### Changed — Filmstrip geometry has per-script owners

- The generated filmstrip geometry ledger is now split into deterministic
  per-script owners, so unrelated writing-system changes no longer rewrite a
  three-megabyte shared file.

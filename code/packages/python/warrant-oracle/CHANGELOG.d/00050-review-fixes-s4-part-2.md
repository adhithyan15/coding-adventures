## 0.2.1

- `verify-v2` review fixes: a FIFO (or any non-regular file) is refused after `lstat`, before it
  is opened, so it cannot hang the gate; `spec` must equal the repo-relative path of `--warrant`
  (`--repo-root` or the nearest ancestor with `code/`); `generator` strings are restricted to
  `[A-Za-z0-9._+-]{1,64}`; `corpus.profile` integers above 2^64 - 1 are refused; a symlinked or
  junction `chunks/` is reported before anything is read through it; the spec size cap is the
  lexer's 1 MiB.

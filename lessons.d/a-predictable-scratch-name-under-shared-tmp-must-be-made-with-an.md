---
category: Security boundaries
---

# A predictable scratch name under shared /tmp must be made with an exclusive mkdir, and cleaned by a Drop guard, not by code at the end

The runtime-jail harness named its scratch directory `<name>-<pid>` and made
it with `create_dir_all`. Under shared `/tmp` that name is guessable and
`create_dir_all` follows a pre-planted symlink, so another local user could
redirect strace output. It also removed the directory with a line at the end of
`parent_main`, so every early `NOT-LIVE` return, panic and kill left it behind.

- Make a directory you intend to own with a non-recursive, exclusive `mkdir`
  (mode 0700); an existing symlink, file or directory is a refusal, never reuse.
- Add unpredictable bits to the name; the pid is not a secret.
- Own cleanup with a `Drop` guard created right after the directory, not a
  statement at the end of the function. Only SIGKILL escapes it.

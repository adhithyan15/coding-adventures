## 0.2.0 - security review fixes, round two (VER06f)

- **Security fix (CWE-400).** The corpus runner charges a `mutate_sequence` call that
  hits its work cap (it throws `WorkExhausted` carrying the work done), zeroes the
  aggregate, and fails every later case without running it; before, each failing case
  was handed the full budget again. The initial buffer copy is charged too.
- **Security fix.** After the target returns or throws, `runTarget` checks the handle:
  a swallowed budget sentinel followed by a return is a timeout/OOM (was a pass), and
  followed by an ordinary exception is a timeout/OOM (was a crash).

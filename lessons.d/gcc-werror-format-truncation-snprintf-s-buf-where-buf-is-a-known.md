# gcc `-Werror=format-truncation`: `snprintf("...%s", buf)` where `buf` is a known-size array can overflow the destination

**Symptom:** A pure-ISO C package built clean on macOS (Apple clang) but failed
on ubuntu CI (real gcc) with:
`error: '%s' directive output may be truncated writing up to 127 bytes into a
region of size 101 [-Werror=format-truncation=]`
for `snprintf(err->message, sizeof err->message, "CompileError: Parse error: %s", perr.message);`
where both `err->message` and `perr.message` are `char[128]`.

**Why:** When the `%s` argument is a *fixed-size array* (e.g. `char[128]`), gcc
knows its maximum length (127) and can prove that `prefix + 127` may exceed the
destination buffer, so `-Werror=format-truncation` fires. It does NOT fire when
the argument is a bare `const char *` of unknown length (gcc can't bound it) —
which is why sibling `snprintf(... "%s", value)` calls with `char *` args were
not flagged. Apple clang doesn't implement this warning, so it only surfaces on
ubuntu CI.

**Fix:** Bound the embedded string with a precision so the total provably fits:
`"CompileError: Parse error: %.100s"` (128 − 27-char prefix − NUL = 100 usable).
Truncating an over-long nested message inside a fixed error buffer is correct
behaviour anyway. Do NOT silence the warning; cap the field.

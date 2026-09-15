# `grep -c "test result: FAILED"` is 0 for a crate that FAILED TO COMPILE (CLOC12.173 PR1)

When verifying "do all these crates pass?", a **compile error emits NO `test result:` line at all** —
so `cargo test -p X 2>&1 | grep -c "test result: FAILED"` returns `0` for a crate that never
even built. I read that `0` as "green" and concluded only 2 of ~10 pass crates needed a new
`ClassExpression` match arm; CI's per-crate build-tool then failed 6+ passes with `E0004:
non-exhaustive patterns`. A second flawed check — `cargo build -p X 2>&1 | grep -B1 "not covered"`
piped into a regex that didn't match rustc's `-->` location format — returned empty and I again
read empty as "no errors". LESSON: never infer compile success from the ABSENCE of a grepped
error string. Check the command's EXIT CODE (`if cargo build -p X >/dev/null 2>&1; then OK else
FAIL`), or grep for the POSITIVE signal (`Finished`/`test result: ok`) and require it. A new
`Expression`/exhaustive-enum variant breaks EVERY crate that matches it without a catch-all —
enumerate them by exit code, not by a fragile error-string grep.

# A quoted `set "VAR=value" && command` in `BUILD_windows` can silently fail to set the variable — and the failure is invisible unless something downstream crashes

`chief-of-staff-smart-home-tools`' `BUILD_windows` used
`set "RUST_MIN_STACK=33554432" && cargo test -p ... && cargo clippy ...` (the
form the existing Windows env-var lesson, line 35, recommends for defensive
quoting). CI still overflowed the stack at the exact same test, with the exact
same crash, as before the fix was written. Direct evidence the variable was
never applied: grepping the full job log for the literal string
`RUST_MIN_STACK` — anywhere in the log, not just this package's section —
returned zero matches, and cargo doesn't otherwise announce which env vars a
subprocess inherited.

**Suspected mechanism** (not independently confirmed on a real Windows box,
but consistent with every observed symptom): the build tool's Go executor
(`code/programs/go/build-tool/internal/executor/executor.go`) runs
`exec.Command("cmd", "/C", command)` — the ENTIRE `BUILD_windows` line as one
argument. Because that line contains spaces, Go's own Windows argument-escaping
wraps it in an outer pair of `"..."` and backslash-escapes any quotes already
inside it (standard CRT-argv escaping rules) before handing it to `CreateProcess`.
`cmd.exe`'s `/C` parsing has its own, different, long-documented quirk: when
the string after `/C` starts with a `"`, cmd applies special stripping logic
and does NOT understand `\"` as an escaped quote the way CRT argv parsing
does. The net effect: the literal backslash-quote sequences Go inserted around
`"RUST_MIN_STACK=33554432"` can survive into the variable name/value `set`
actually parses, silently setting a garbage-named variable instead of
`RUST_MIN_STACK` — no error, no warning, just a `set` that ran and did
something other than what the line says.

**Fix:** drop the quotes when the value has no spaces or shell metacharacters
that need protecting — `set RUST_MIN_STACK=33554432 && cargo test ...` — which
sidesteps the whole Go-escaping/cmd-`/C`-quirk interaction rather than
depending on it working correctly. The existing "defensive quoting" lesson
(line 35) is not wrong for values that DO contain `&|()`/`%CD%`/spaces — where
quoting is genuinely necessary and presumably already proven by the Lua/Perl
`BUILD_windows` files that use it for such values — but it's an unnecessary
risk for a value that doesn't need it, and this PR shipped one real,
CI-confirmed case of the quoted form silently failing.

**Generalized lesson:** an env-var-setting line in `BUILD_windows` that "did
nothing" fails silently — there's no error, just whatever downstream default
behavior kicks in. If a fix that sets an env var to work around a platform
failure doesn't change the failure at all on the next CI run, suspect the
`set` line itself before re-deriving the original bug's root cause a second
time. Grep the job log for the variable name as a first, cheap diagnostic —
its total absence from the log (build tools rarely echo inherited env) doesn't
prove failure by itself, but combined with an unchanged crash, it's strong
evidence the assignment never reached the process that needed it.

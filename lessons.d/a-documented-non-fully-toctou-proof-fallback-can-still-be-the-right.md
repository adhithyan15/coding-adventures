# A documented, non-fully-TOCTOU-proof fallback can still be the right call — say why in the code, not just that it's imperfect

`ir-to-jvm-class-file`'s POSIX symlink defense (`os.open(..., dir_fd=...)`
chained through every path component) has no Windows equivalent — `os.open()`
can't even open a bare directory there. The Windows fallback
(`Path.is_symlink()` checks before `mkdir`/write) has a real TOCTOU gap: a
symlink swapped in between the check and the use isn't caught, unlike the
atomic dir_fd chain.

Two things made this an acceptable ship, not a deferred vulnerability:

1. **Close the cheap half of the gap.** The final file write went through a
   temp file + `os.replace()` instead of opening the checked path directly —
   `os.replace()`/`rename(2)` replace the directory entry itself rather than
   following a symlink placed there afterward, so that specific race is fully
   closed for near-zero extra code. The intermediate-directory `mkdir` race is
   harder to close without a dir_fd equivalent and was left as documented,
   accepted risk rather than force-fixed.
2. **State the actual threat model in the comment, not just "not fully safe."**
   `class_filename` is compiler-internal input already validated (no absolute
   paths, no `.`/`..`) before either write path runs, so the residual race
   requires an attacker who already has write access to the output tree,
   racing a local build — not a remote/network-facing threat. A reviewer (or a
   future security audit) reading "not fully TOCTOU-proof" alone has to
   re-derive this; reading it inline saves that re-derivation and makes the
   accepted-risk decision auditable instead of just asserted.

**Correction, one CI round later:** point 2's premise — "already validated,
no absolute paths" — was itself broken on Windows by the SAME `Path.is_absolute()`
platform-dependence bug documented two lessons up, just in a fourth location
this time: `_validated_output_relative_path()` used `Path(class_filename).is_absolute()`
to reject `class_name=".Escape"` → `class_filename="/Escape.class"`. That
check passes (correctly rejects) on POSIX and silently **passed the file
through unrejected on Windows** (`PureWindowsPath("/Escape.class").is_absolute()`
is `False` — no drive/UNC prefix). `class_filename` is always POSIX-separated
(built via `class_name.replace(".", "/")`, see `JVMClassArtifact.class_filename`),
so the fix is the same pattern as the Rust cases: check `class_filename.startswith("/")`
directly instead of asking the platform-native `Path` what it thinks
"absolute" means. **The generalized lesson from three lessons up undersold its
own stakes**: this wasn't just "pick the right fix for the code's intent" in
the abstract — a stale platform-dependent validator can invalidate a security
argument that was written assuming the validator worked. When accepting a
residual risk *because* an earlier check already narrows the threat model,
verify that check cross-platform before relying on it, not just on the
platform you happen to be testing on.

**Second correction, before push this time (security review, not CI):** the
`class_filename.startswith("/")` fix above was itself an incomplete swap, not
a complete one — it closed the reported gap (POSIX-style `/...` under-rejected
on Windows) but reopened a *different* one the original `Path.is_absolute()`
had actually caught when the validator happened to run in Windows CI:
`class_name` is adversarial, unrestricted, attacker-supplied input (the test
suite constructs it deliberately), so nothing stops it from containing `:`
or `\` — neither of which `.replace(".", "/")` touches. `startswith("/")`
alone does not reject `"C:\\evil.class"`, `"C:/evil.class"`, or
`"\\\\server\\share\\evil.class"` on ANY host platform (not just Windows —
these were never rejected when the validator ran on POSIX either, since
POSIX's native `Path.is_absolute()` doesn't recognize Windows drive/UNC
syntax at all; the original code only caught them by accident, when the
validator happened to execute on an actual Windows host). **Correct fix:**
check both conventions explicitly and platform-independently — `PurePosixPath(x).is_absolute()
or PureWindowsPath(x).is_absolute()` — plus reject any `":"` outright to catch
drive-relative paths (`"C:evil.class"`, absolute under neither convention,
also never caught by the original code). `PurePosixPath`/`PureWindowsPath`
(unlike bare `Path`) are always available and always mean the same thing
regardless of host OS — exactly the tool this whole bug class was missing.
**Lesson: when a security check depends on "is this path absolute," check
it means "absolute under every convention the input could plausibly use,"
not "absolute under the one convention this function's `Path` happens to
resolve to on whichever host runs it."** A narrow platform-specific patch for
a reported failure is exactly the moment to ask what the *general* property
being checked for is, not just what makes the one failing test pass.

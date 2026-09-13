# The SAME "`Path::is_absolute()`/`is_symlink()` is platform-dependent" bug class recurs across unrelated crates — check by INTENT, not by pattern-matching the previous fix

Three different Windows CI failures in one PR rescue all trace back to
`std::path::Path`'s platform-dependent behavior, but needed **three different
fixes** because the code's *intent* differed each time:

1. **`sql-parser`** (`grammar_path.read_text()`): platform default *encoding*
   (cp1252 on Windows) silently misreads UTF-8 bytes. Fix: pin
   `encoding="utf-8"` explicitly — the file's encoding is a property of the
   file, never the host platform.
2. **`chief-of-staff-daemon-service-files`** (`validate_unix_path`): the
   function validates paths for a launchd/systemd config being rendered for
   **macOS/Linux, regardless of the host compiling and testing this crate**.
   `Path::is_absolute()` uses the *build host's* semantics, which is wrong
   here on principle, not just on Windows — the fix (`value.starts_with('/')`)
   hardcodes POSIX semantics because the target is always POSIX.
3. **`chief-of-staff-daemon-config`** (`ConfigPath::resolve`'s `home` param):
   this validates the **actual runtime host's** home directory — a real
   Windows deployment legitimately passes a `C:\Users\...` path. Here
   `Path::is_absolute()` is *correct as written* (there's already a
   `#[cfg(windows)] fn absolute_home()` test helper proving the crate expects
   platform-native paths); the bug was three tests hardcoding a POSIX path
   directly instead of calling that helper, like every other test in the file
   already did.

**Before reaching for the fix pattern that worked last time on `is_absolute()`
or `is_symlink()`, ask: is this code validating "a path meaningful on THIS host
right now" (use platform-native `Path` semantics — case 3) or "a path meaningful
on some OTHER, possibly-fixed target regardless of build host" (use an explicit,
target-specific string check — case 2)?** Getting this backwards either breaks
real Windows deployments (over-hardcoding POSIX) or reintroduces the original
bug (using native semantics for a fixed-target validator). Grep for an existing
`#[cfg(windows)]` test helper near the failing assertion before writing a new
fix — case 3's `absolute_home()` was sitting three functions away and would
have caught the real bug (a test authoring slip) immediately instead of
requiring a `Path::is_absolute()` investigation that turned out to be a red
herring.

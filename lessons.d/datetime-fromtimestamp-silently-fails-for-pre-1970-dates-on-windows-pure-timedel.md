# `datetime.fromtimestamp()` silently fails for pre-1970 dates on Windows — pure `timedelta` arithmetic doesn't

`sql-vm`'s `unixepoch` date handling used `datetime.fromtimestamp(seconds,
tz=UTC)` to convert Unix-epoch seconds to a date. `SELECT date('-1234567890',
'unixepoch')` — a pre-1970 date — returned `NULL` on Windows only; the
correct answer (`'1930-11-18'`) came back fine on Linux/macOS, and the code
already had `except (ValueError, OSError, OverflowError): return None`
around the call, which is precisely what swallowed the platform-specific
failure into silent NULL instead of surfacing it.

**Cause:** `datetime.fromtimestamp()` converts through the platform C
library's time functions. glibc's `gmtime` handles negative `time_t` (dates
before 1970-01-01) without complaint; the Windows CRT's `_gmtime64`/
`_localtime64` explicitly reject negative `time_t` with an error — a
documented, longstanding Windows CRT limitation, not a Python bug.

**Fix:** compute the date by pure calendar arithmetic instead —
`datetime(1970, 1, 1, tzinfo=UTC) + timedelta(seconds=seconds)` — which
never touches the platform C library, so it behaves identically on every
host `datetime`/`timedelta` run on. Verified this doesn't trade the fixed
platform bug for a worse one: extreme inputs (`1e300`, `inf`, `nan`,
`-1e300`) still raise `OverflowError`/`ValueError` in microseconds, same
failure class as before, just now bounded by `datetime`'s actual year
1-9999 representable range instead of an incidental platform ceiling.
**Generalizable lesson:** any `datetime.fromtimestamp()` (or `time.gmtime()`/
`time.localtime()`, same underlying mechanism) call that might see a
pre-1970 value is a Windows-only landmine — prefer epoch-relative
`timedelta` arithmetic when the code needs to work the same way
cross-platform, not just "not crash."

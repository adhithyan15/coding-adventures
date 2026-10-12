---
category: Security boundaries
---

# Bounded diagnostics must not decide whether browser retirement failed

Security review of the concurrently pushed browser warm-up change found that
`discardWarmupAudit` decides whether Chrome retirement failed by searching a
1,024-character diagnostic. A long startup error puts the appended retirement
failure beyond that boundary. The exact helper then resolves and logs a dropped
warm-up instead of failing. Native kill/temporary-directory errors may not use
the word retirement at all.

Classify retirement failures with an internally created error identity before
formatting logs, wrapping all native cleanup failures as well as timeout errors.
Propagate that identity from warm-up and recorded samples so a possibly live
browser cannot contaminate later evidence. Retain bounded diagnostic output.

The warm-up also moved Chrome startup before the gate's bounded HTML reads and
static inspection. Preflight all targets once before any browser starts and
reuse the fallback reports. Regression tests need real oversized files and the
long-message/native-cleanup failures; unchanged file-symlink cases still require
the host's existing symlink privilege and must not be weakened to pass locally.

The repaired two-file suite passed 23 cases, including the new classification,
proxy, native kill/cleanup and real oversized-artifact regressions. Three unchanged
file-symlink cases stopped at this non-administrator Windows host's EPERM; they
remain enabled for CI. Focused TypeScript compilation passed with the site's
ES2022/NodeNext/strict settings. The new error identity uses private WeakSet
membership, which does not invoke properties or proxy traps on a thrown value.

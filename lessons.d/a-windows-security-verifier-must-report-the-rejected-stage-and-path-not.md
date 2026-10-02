---
category: CI & GitHub Actions
---

# A Windows security verifier must report the rejected stage and path, not only an exit code

The Forme Windows launcher deliberately collapses every runtime-root trust failure
to one fail-closed exit status. That is the correct API result, but the launcher
also emitted an empty stderr stream. When the hosted Python distribution was
rejected, the Windows-only CI log therefore showed only exit code 65 from the
TypeScript wrapper: it did not distinguish an open/share failure, an untrusted
owner or writer, a reparse target outside the root, a traversal bound, or an
ancestor that allowed replacement. Each hypothesis required a new Windows CI
round to test.

Keep the public result generic, but make a security verifier print one bounded,
local diagnostic naming the failed check and the path it actually inspected.
The path itself is attacker-controlled data: encode every UTF-16 unit into
bounded ASCII (including separators, control characters, bidi controls, and
invalid surrogates) before writing it to a log. For Win32 calls, capture
`GetLastError()` immediately and include its numeric value. For ACL policy
failures, distinguish owner, ACE type, writer, and tree-bound failures. Tests
for the verifier should capture stderr, assert that known rejections are
diagnostic and cannot inject a second line, and include stderr as the assertion
message for expected successes. This preserves fail-closed behavior without
making CI forensics depend on speculative patches or creating a log-injection
surface.

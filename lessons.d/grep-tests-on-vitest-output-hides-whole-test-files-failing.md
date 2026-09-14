# `grep "Tests "` on vitest output hides whole test FILES failing

Same incident. Verification was scripted as
`npx vitest run 2>&1 | grep -E "Tests "`, which prints:

    Tests  243 passed (243)

and looks clean. The line above it read `Test Files  7 failed | 27 passed (34)`.
Seven files had failed to *load* — so their tests never ran, never failed, and
never appeared in the count that was being read. A run reporting "243 passed"
was actually a broken run, and the real suite is 727.

**Rule:** grep `"Test Files|Tests "`, never `"Tests "` alone. A collection error
is invisible in the passing-test count by construction.

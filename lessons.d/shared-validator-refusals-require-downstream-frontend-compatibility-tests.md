---
category: Testing & coverage
---

# Shared validator refusals require downstream frontend compatibility tests

CLR01 backend and lang-aot consumer tests passed locally, but macOS CI found
COBOL's backend-compatibility test still expected encoded CLR to accept scale-12
constants. That acceptance had concealed truncation. Update the specific test to
assert the new refusal and preserve textual CoreCLR int64 coverage. When a shared
validator becomes stricter, run downstream frontend compatibility suites as well
as backend tests; do not weaken validation merely to retain unsafe acceptance.

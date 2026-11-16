---
category: Testing & coverage
---

# A path helper with an optional flavour argument tests a different code path from the one production calls

The Python `relative_inside` applied the device-namespace rule only when a flavour
was passed. The conformance fixture always passed one; `corpus_relative` and the
binding's source check never did, so the fixture exercised a path production does
not use. The same review found the rule applied to the literal prefix before
separators were normalised (`\/?\D:` slipped through) and `posixpath.normpath`
keeping exactly two leading slashes. Make the helper take its default from the host
(`HOST_FLAVOR`), normalise separators before any prefix rule, and add a test that
calls the production wrapper with the flavour patched. A second opinion in a
checker must be a different algorithm, not the same stdlib call.

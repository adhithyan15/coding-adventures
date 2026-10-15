---
category: Testing & coverage
---

# Canonicalize mocked output roots before comparing macOS temporary paths

The portable Haskell bootstrap success test passed locally but failed on macOS:
the helper resolves its output while TemporaryDirectory returned /var and the
kernel canonical path uses /private/var. The exact environment comparison failed
before real probes; other conditional child-config checks could skip entirely.

Preserve the hosted raw failure, reproduce differing path spellings locally with
a lexical parent segment, then canonicalize the mock root when acquired. Keep
exact environment/config assertions and every phase-refusal probe. Do not change
production paths or claim mock success as actual macOS linker acceptance.

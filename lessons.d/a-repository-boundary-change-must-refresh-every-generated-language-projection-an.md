---
category: Supply chain & CI pinning
---

# A repository boundary change must refresh every generated language projection and digest pin

The neutral `repository-source-input-boundary.json` is not the only checked
source of boundary data. C# embeds it in `SourceInputRegistries.Generated.cs`,
and Swift embeds it in `RepositorySourceInputBoundary.swift`. Both are generated
from the neutral registry and must remain semantically equal to it. The C# and
Swift hashers also pin the canonical digest in production code and in
conformance tests.

When the TypeScript workspace boundary gained `code/packages/typescript/nand-flash`,
the neutral fixture, Go and Python references, and case-level digest pins were
updated, but the C# and Swift generated projections and four C#/Swift digest
pins remained stale. The contract corpus still validated because those language
lanes were not run by that check.

For future boundary edits, regenerate both language projections from the neutral
JSON, recompute the framed SHA-256 digest, and search the repository for the old
digest. Update production constants and tests in every affected language. Verify
that decoding each projection yields the same JSON value as the fixture.

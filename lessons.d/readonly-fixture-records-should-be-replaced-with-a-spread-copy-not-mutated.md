---
category: TypeScript / JavaScript
---

# Readonly fixture records should be replaced with a spread copy, not mutated in place

The canonical fixture types are readonly, so a test that clones a case and then assigns to its
`id` does not compile. Preserve the fixture's immutable contract in negative controls by constructing
a replacement object with `{ ...caseCopy, id: replacement }`; this also makes it explicit that only
the copied record is being mutated. Keep the exact harness failure assertion on that copy.

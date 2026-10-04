---
category: Testing & coverage
---

# Authoring project fixtures must use canonical UUIDv7 identities

`createAuthoringProject` validates identities before it can serve as a fixture.
A convenient label such as `"project"` therefore makes every downstream test
fail at setup and hides the behavior the suite intended to exercise. Reuse an
obviously synthetic but canonical UUIDv7 value in authoring tests, and reserve
short labels for fields whose schema actually permits them.

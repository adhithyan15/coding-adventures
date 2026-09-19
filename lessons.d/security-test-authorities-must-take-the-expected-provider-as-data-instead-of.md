---
category: Testing & coverage
---

# Security test authorities must take the expected provider as data instead of hardcoding one profile

A shared identity-authority mock asserted one confidential fixture name inside
its verifier. Reusing it for a public-provider composition made a correct call
panic on the mock's unrelated assumption. Give reusable security mocks an
explicit expected provider (and derive the expected client from it), while
keeping convenience constructors only for tests that intentionally share a
single profile. This preserves binding assertions without coupling the mock to
one authentication method.

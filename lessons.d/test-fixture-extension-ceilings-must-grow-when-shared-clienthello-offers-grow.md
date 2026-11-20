---
category: Testing & coverage
---

# Test fixture extension ceilings must grow when shared ClientHello offers grow

Adding `signature_algorithms` to the shared retry ClientHello fixture increased
its extension count from four to five while the helper still decoded every
fixture under a ceiling of four. Every dependent test then failed at fixture
construction with `TooManyExtensions`, obscuring the feature under test. When
expanding a shared encoded-message fixture, update its matching structural
limits in the same change and run the focused package test immediately.

---
category: Testing & coverage
---

# Check an integration test module's existing limit constructors before naming a helper

An X.509 integration-test addition guessed that the module exposed an
`asn1_limits()` helper even though its existing decoder and policy calls use
`Asn1Limits::default()` directly. Search the test module for both helper
definitions and constructor call sites before drafting a new call. Reuse the
established constructor unless the test genuinely needs a named policy limit.

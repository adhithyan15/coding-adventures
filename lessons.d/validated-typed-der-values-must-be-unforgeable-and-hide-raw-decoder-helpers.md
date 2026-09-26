---
category: Cryptography & security review
---

# Validated typed DER values must be unforgeable and hide raw decoder helpers

The first typed-DER APIs exposed public construction paths in Dart, Python, and
Go, and Dart also exported a raw framing helper. Callers could forge arbitrary
depths or invalid primitive values and bypass shared depth and element budgets.
Keep Dart constructors and raw helpers library-private, use an internal factory
for Python's frozen element wrapper, and keep Go wrapper fields unexported with
read-only accessors so every public typed value originates from checked decode
paths and retains stable error behavior.

---
category: Testing & coverage
---

# Artificial X.509 path-order fixtures must assert projected endpoint identities before signature verification

A test used a stateful issuer-selection callback to force a longer constructed
path, but its candidate vector did not match the intended order. The first
symptom was an RSA block-header failure because the terminal link's actual
child was not the certificate the test supplied as child policy.

Immediately after constructing an artificial path, assert the projected
terminal child and issuer pointer identities before building algorithm policy
or invoking signature verification. That localizes ordering mistakes and keeps
cryptographic failures meaningful.

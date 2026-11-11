---
category: Rust
---

# X.509 path construction relation callbacks receive issuer and subject names, not certificate objects

The constructed-path API exposes complete certificates to its terminal callback,
but deliberately narrows issuer selection to a callback over the child's issuer
name and each candidate's subject name. A test fixture tried to compare those
callback arguments with certificate object pointers and failed to compile.

Read the callback signature before designing a path-order fixture. When a test
needs an artificial order rather than name matching, use the documented `FnMut`
scan order explicitly and keep that policy local to the test. Production callers
must continue to supply their reviewed name comparison policy; the relation
callback is not an object-identity hook.

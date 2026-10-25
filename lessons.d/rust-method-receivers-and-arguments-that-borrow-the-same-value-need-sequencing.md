---
category: Rust
---

# Rust method receivers and arguments that borrow the same value need sequencing

An expression such as `self.update(self.read_value())` tries to hold the
mutable borrow for the method receiver while creating a second mutable borrow
to evaluate the argument. Rust rejects that overlap even when the operations
are conceptually sequential. Read the value into a local first, then call the
mutating method with that local; this makes the intended borrow boundary
explicit and keeps instruction-dispatch code easy to audit.

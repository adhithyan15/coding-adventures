---
category: Rust
---

# Box::leak(x) as *const T exposes a read-only tag, so writes through exposed provenance later are UB

`dynval-runtime::heap::alloc_cons` did `Box::leak(cell) as *const ConsCell` and then `expose_provenance()` on it (tagged-pointer `LispyValue`). The `&'static mut T -> *const T` cast reborrows through `&T` first, so under Stacked Borrows the only tag exposed for that allocation was **SharedReadOnly**. `set_field_unchecked` later rebuilt the pointer with `with_exposed_provenance` and wrote `car`/`cdr` through it. A wildcard pointer can only use permissions that were exposed, so the write was UB. Miri reported: `attempting a write access using <wildcard> ... but no exposed tags have suitable permission in the borrow stack`.

It went unnoticed for months for two reasons. dynval-runtime's blocking Miri suite had no test for `set_field_unchecked`. And the only test that reached the write (twig-vm `tw05d_smoke`) sat behind a stdin test that aborted the twig-vm Miri run before it got there.

Fix: `Box::into_raw(cell)`. It yields a `*mut` that keeps write permission, and casting `*mut` to `*const` does not retag.

Do differently:
- Anything that is ever written through a round-tripped tagged/exposed pointer must be exposed from a `*mut` obtained without a shared reborrow (`Box::into_raw`, `&raw mut`, `ptr::from_mut`), never `&mut as *const`.
- Test every `unsafe` write primitive in the crate that owns it, so the cheap, blocking Miri job covers it, instead of relying on a slow downstream integration suite.

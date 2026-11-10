---
category: Rust
---

# A composite binder's domain is materialised whole, so the 64 MiB byte budget bounds the product inside it even when the case budget is raised

The first WX-A generator tests assumed the WX-B behaviour for `U16 x U8 x U8`:
three *binders* multiply to 18.4 M cases, which the raised case budget (2^26)
admits, because the bounded-exhaustive runner iterates the Cartesian product of
the binders lazily and only materialises each binder's own domain. A single
*tuple* binder `(U16, U8, U8)` has the same 18.4 M elements, but they are one
binder's domain and `domain()` builds that whole vector (about 2.3 GB at 128
bytes per element). The byte budget (`DOMAIN_BYTES_BUDGET`, 64 MiB) therefore
refuses it at either case budget, and `(U16, U16)` demotes *both* components even
at 2^26.

That is correct, and the byte bound is what protects the machine; the mistake
was the expectation. Do differently when reasoning about a new domain shape:
ask which products are iterated and which are stored. A composite is stored, so
its size is bounded by `bytes / elem_cost`, not by the case budget; a list of
binders is iterated, so only the case budget bounds the product. Pin both
behaviours with a test that sits on each side of the boundary (VER04j section
14.1, item 20).

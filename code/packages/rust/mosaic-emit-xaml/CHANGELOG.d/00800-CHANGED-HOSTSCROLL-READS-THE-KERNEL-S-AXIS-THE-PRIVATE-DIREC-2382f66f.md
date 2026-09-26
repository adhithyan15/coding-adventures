### Changed — `HostScroll` reads the kernel's `axis`; the private `direction` prop is gone (UI61, #14854)

This emitter had already implemented the entire three-way axis, under a
backend-private prop named `direction` that no `.mll` in the repo ever wrote
and that the other seven backends could not see. The feature existed, worked,
was unreachable, and was invisible — which is precisely the defect UI61 exists
to fix.

It now reads the kernel's `axis`, and `direction` is deleted rather than kept
as an alias. Nothing authored it, so nothing breaks; keeping it would preserve
exactly the backend-private vocabulary this spec ends.


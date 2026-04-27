# jvm-class-file

`jvm-class-file` is the shared TypeScript infrastructure layer for the JVM
rollout.

It does three conservative things:

1. parse the small class-file subset our compiler lane emits
2. build a minimal one-method class file for low-level tests
3. resolve class, field, method, and loadable constant references from the
   constant pool

The package deliberately avoids trying to model the whole JVM specification at
once. It focuses on the boring subset the repo's compiler backends need first.

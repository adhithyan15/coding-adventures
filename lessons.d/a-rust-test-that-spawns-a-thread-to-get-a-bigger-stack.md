---
category: Rust
---

# A Rust test that spawns a thread to get a bigger stack trips the capability analyzer (CAP001 thread:spawn); make the recursion bound a parameter and lower it under test instead

The `warrant-corpus-gen` evaluator test for its recursion guard built a tree deeper than the
default test stack could unwind and ran it on `std::thread::Builder::new().stack_size(..)`.
The test passed, but `ca_capability_analyzer` scans test code too and reported
`CAP001: Undeclared capability 'thread:spawn:*'`; the package declares no capabilities, so
CI would have failed. The same test also showed the evaluator's debug-build frame was large
(every `match` arm's locals get their own slot), which is why heavy arms now live in
`#[inline(never)]` helpers.

Update (review): the real cause was a recursive evaluator whose debug frame (3 to 4 KiB per
level) overflowed a 2 MiB stack at 8 model frames, well inside the documented bounds (64
frames x 64 levels). A lowered limit hides that. The evaluator is now iterative (explicit heap
work stack) and a test runs the real bound on a default thread; do not shrink the limit to
make a recursion test pass, remove the recursion.

Original fix: the recursion bound is a field of the program (`with_max_depth`, `#[doc(hidden)]`), the
test lowers it to 32 and builds a 40-deep tree, so no big stack and no thread are needed.

Do differently: when a guard needs a pathological input to be tested, make the limit
injectable and test the guard at a small limit; run the capability analyzer
(`cargo run -q -p coding_adventures_ca_capability_analyzer -- --dir .`) before pushing any
crate whose BUILD runs it.

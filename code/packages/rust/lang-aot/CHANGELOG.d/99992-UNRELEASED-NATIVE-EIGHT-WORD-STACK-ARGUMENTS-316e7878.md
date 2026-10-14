## [Unreleased] — native eight-word stack arguments

Internal integer-word direct calls and function parameters now support
up to eight words on MsX64 and SysV. A fixed aligned outgoing tail follows
shadow space and stays below all negative RBP locals; callee extras spill from
RBP+16+shadow+8index. Frame/displacement arithmetic is checked; nine arguments
or formals refuse. Builtin physical-register marshalling is unchanged. Exact
both-ABI simulator controls check5/6/7/8 order, fullRAX, stackwords, shadow
clobbering, preserved locals, alignment/restoration, recursion and refusal.
Root metadata retainsp7 RBP-64 and actual external returnPC. Real Windows
six/eight-position and eight original captured/forwarded array rows execute
Exit42. Simulator evidence does not establish Linux/macOS hardware or forced
GC execution, and this bounded internal convention does not claim arbitrary
arity or native C floating argument interoperability.

Actual isolated source316e7878 validation completed: nine affected packages
1226PASS2FAIL1existingignore/81targets; only the obsolete five/seven-formal
refusal oracles failed. Full unfiltered LANG476/0/0, strict nine-package
alltargets, standalone Closure1093/0/0 across181targets and strict passed.
Spec-first test-only correction7126217d retains those accepted counts and eight,
adds exact ABI/got9/max8 plus public-refusal checks. New complete backend
85/0/0 over2 targets and strict nine-package alltargets passed.
Whole production prefix and every other Rust/instruction object remain exact;
unchanged runtime evidence is explicitly attributed to316e. These counts are
separate source receipts, not additive, and the original failed run stays failed.
The prior5ba run exhausted disk before tests; its failed receipts are preserved.
Retry disabled debug symbols/incremental compilation, preserving all inputs,
assertions and limits. Registry376 tuples remain exact. Miri
**DEFERRED/PENDING**, never passed; frozen run/input/target/receipts untouched.
General-hop32 resource debt, hosted CI/conflict/review/merge, full Closure parity
and output-source/full transformation lineage remain OPEN.

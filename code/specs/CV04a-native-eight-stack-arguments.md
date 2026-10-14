# CV04a — eight-word native stack arguments

Implement the bounded native scope already committed in
CV04a-captured-array-call-repair.md. Current baseline db041b91 includes the
reviewed JVM repair. Resume audit found main32d1f5e Japanese curriculum/TS
content changes, with no Rust compiler changes. Audit the complete delta and
retain it as pending producer reconciliation before publication; native work
proceeds on the current isolated owned branch and target. Do not alter the
frozen Miri checkout/run/input/target/receipts. Miri DEFERRED/PENDING.

Before production, add execution controls in the existing x86-simulator crate,
which already depends on this backend for tests. Compile real caller/callee
blobs and execute both ABIs without host imports. Six/eight distinct arguments
produce an exact weighted full-RAX result (not only an exit-byte match). Inspect
actual callee-entry stack arguments and16-byte call alignment; clobber the
allowed Windows shadow area to prove it is disjoint from arguments/locals.
Verify caller local values survive. Exercise terminating self-recursion with
eight arguments, check exact GC root offsets and both safepoint kinds, and
retain explicit nine-argument/formal refusal on both ABIs. No new dependencies.

Run native Windows emitted executables for exact six/eight-position checks,
plus all eight existing captured/forwarded array rows with Exit42. Other hosts
use their applicable native emitter; simulator evidence is not OS-native
acceptance. Diagnostic-only old refusal assertions become exact successful
prepared-function/object controls after the repair, retaining original red
receipts and unchanged fixtures. No arbitrary-arity/native-C-float-ABI claim.

Production keeps a fixed outgoing tail following MsX64shadow32 or SysVshadow0,
disjoint from negative RBP locals. Reserve the largest supported direct-call
tail, round the total frame to16, and reject overflow or unrepresentable signed
frame offsets. Callee extras spill from RBP+16+shadow+8index into deterministic
local slots. Caller extras load through RAX from existing RBP-relative values,
then store at RSP+shadow+8index; GPR args load afterward. Existing builtin ABI
dispatch is unchanged. Safepoints come from actual emitted return addresses;
reference roots retain their original negative RBP identities.

Run complete affected backend/simulator/Twig/LANG/compiler/downstream suites,
strict lint and exact-head independent security review before delivery. Required
CI/conflict/review/merge gates remain; no skipping existing failures or weakening
capacity/ownership checks. General-hop32 resource debt and full Closure parity/
byte-source/full lineage/pass-sweep/stableIDs/spans remain open. One active PR.

The first hardware control returned UNAVAILABLE because the shared test helper
uses a Linux/macOS-only approximation for NativeAot. The actual native runner
already uses native_linker_ok, including a genuine Windows linker probe.
Align the shared availability helper with that real execution gate, retain the
unavailable receipt as non-execution, and rerun the baseline hardware control.
An installed Windows linker must not silently suppress these controls.

Protect the two new LANG matrix controls with exact filters in its production
BUILD list, as required for every new matrix test. The simulator's existing
unfiltered cargo test command already executes the new integration target.
Only after the complete formerly red LANG target actually passes may its stale
exclusion notes and production test selection be reconciled.

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

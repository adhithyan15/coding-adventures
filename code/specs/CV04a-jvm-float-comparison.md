# CV04a — typed JVM float comparisons

The corrected float-array positive control at a1178ed8e4e now executes the
real verifier and fails: Main.main()I Register4 contains wrong type. Its f32
call result is stored correctly, but cmp_eq selects the integer iload path.
Preserve that actual failure. The earlier missing-UTF8 launcher ClassFormatError
is a harness failure and supplies no float-array descriptor evidence.

Extend the existing typed double comparison path to f32, loading with fload
and using fcmpl/fcmpg rather than integer comparisons. Keep the result as int
0/1. For both float widths, enforce ordered IEEE comparisons: unordered eq,
lt,le,gt,ge are false and ne true. Use cmpg for lt/le so NaN's +1 skips true;
cmpl for gt/ge so NaN's -1 skips true. Equality uses either NaN sign with eq/ne.
The old double path has the opposite unordered choice for ordered relations;
correct it and verify both widths rather than perpetuating the same error.

Commit exact typed-load/opcode/branch controls before implementation. Run a
real JVM table for all six relations over both widths, with 1 vs2, 2 vs1,
equal1, signed zeros and NaN in either operand. Generate NaN by 0/0 to avoid
unrelated arbitrary-f32-constant pool limitations; expected results come from
Rust's IEEE comparisons. The launcher checks every result and throws on a
mismatch, with verification enabled and a derived six-slot peak. Preserve the
float-array computation control and its descriptor-only negative verifier
control. Refuse mixed Float/Double or float/integer comparison operand types.

This extends the bounded float-array slice because its actual positive test
exposed an applicable compiler failure. It does not weaken or replace that
control. Run complete JVM, scoped real LANG/JVM controls, strict lint and
independent exact-head security review; full native/publication/merge gates
remain pending for the original native arity failures. General-hop work and
all Closure completion/provenance requirements remain open. Miri remains
DEFERRED/PENDING; frozen run/source/inputs/target/receipts untouched/unpolled.

The full candidate JVM suite exposed an old byte-opcode oracle requiring
dcmpg for greater-than. Real JVM table84 comparisons now pass, including NaN
on either side: greater-than must use dcmpl with ifle to keep unordered false.
Reconcile the existing assertion to that exact semantic opcode/branch pair;
retain its test and the original failing suite receipt rather than deleting it.

Actual revised c5b00f83 validation41518 completed EXIT0. Complete JVM200/3
(69 library,124 integration,7 docs), unchanged original8 real JVM array rows,
float-array positive/negative verifier controls, real IEEE84 checks and strict
JVM/LANG/Twig alltargets pass. Independent exact-source/control/terminal security
PASS SHA84f4fe59a3d0b50bda652540236e3a37ca6788d6c900d7211b7ae1b1f4410689.
The old2f complete-suite receipt retains the erroneous greater-than oracle
failure; it is not labeled all-pass. Latest full/native/hosted acceptance still
pending, with nine original native arity failures and generalhop32 resource
failure open. Miri DEFERRED/PENDING; full completion contract unchanged.

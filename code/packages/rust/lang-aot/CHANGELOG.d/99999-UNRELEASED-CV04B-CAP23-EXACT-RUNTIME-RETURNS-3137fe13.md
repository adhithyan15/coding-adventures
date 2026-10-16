## Unreleased - Exact cap23 runtime sources and typed return evidence

Append29 static full-source/output pairs without changing old saved19..21 or
shared Prog/Expect lifetimes. Actual3137 execution validates174 unique29x6 cells,
including26 exact Float return observations on VM/JIT. Two inert test-only hooks
capture execute-Ok values before integer status conversion; guarded source/
backend/count/type/value assertions reject stale evidence and PhantomData<Rc<()>>
encodes thread affinity. Rustc cross-thread rejection/same-thread control and
adversarial runtime controls pass; both compiler/LANG all-target strict0.
Keep source/result bytes and existing run/assert_cell semantics. Correct the
mistaken universal-zero contract using independently traced backend semantics;
preserve actual compiled red receipts. Native/LLVM typed-main ABI and CLR f64
entry conversion remain OPEN; absent CLR is uncounted. Focused completion/effect
acceptance is distinct from full affected/final/hosted validation and merge.
Miri DEFERRED/PENDING, never passed; preserve frozen run and rerun final source
only after implementation/delivery under the user's sequencing instruction.

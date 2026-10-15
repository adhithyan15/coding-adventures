# CV04a: select the declared Unix reactor boundary on Windows

Spec first. Published d400 build(windows-2025) job116367540028 failed in
rust/tcp-reactor after818 builds; direct logSHA42cfd96728b448644b44f277fe2d45f30a85b90ec020d884b4c55ed47667b62c.
The crate intentionally compile_errors on non-Unix and uses RawFd/as_raw_fd/
SourceRef::from_fd. Its generic BUILD unconditionally runs cargo test, so the
Windows plan selects an unsupported target. This is package platform selection,
not a demonstrated Closure compiler assertion/UB. Do not delete the compile
boundary, change its implementation or claim Windows socket support.

Add a BUILD_windows override with an explicit SKIP reason for unsupported Unix
file descriptors, following existing Apple-only package conventions. Preserve
original BUILD exact so Linux/macOS still run every native crate test. README
must declare this support boundary and distinguish package SKIP from testsPASS;
add the required changelog. No Rust/Cargo/dependency/CI-required-check changes.

Validation: run affected build-tool discovery/selection/validation tests and
vet, derive an actual fresh unforced build plan and inspect Windows versus
Linux/Darwin selected BUILD/commands for tcp-reactor using the real discovery
API. Existing direct hosted compile failure is the before evidence; unsupported
Windows has no native runtime test acceptance. Source and Unix BUILD hashes
must remain exact; no bespoke low-impact implementation-mirroring test needed.
Require genuine finalhead Windows hosted success before auto/merge; maintain
all Unix required jobs. This selection repair does not waive required CI.
Current macOS GHC9.4.8/SDK27 linker failure remains independently OPEN and needs
separate spec-first repair; native9/main/Vennpalette reconciliation remainsOPEN.

OnePR17288, autoOFF until exacthead conflictfree/reviewclean terminal required
checks. Verify fetched actual merge reachability before cleanup. Preserve
unrelated edits and all source/input inventories. Miri DEFERRED/PENDING, never
passed; frozen checkout/process/inputs/target/receipts untouched/unpolled.
Implementation delivery first, genuine final-source Miri afterward; full Closure
parity/output-byte-source/full transformation lineage/objectiveOPEN, heartbeatACTIVE.

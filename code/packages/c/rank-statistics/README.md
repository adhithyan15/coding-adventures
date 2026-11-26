# C rank statistics

An ISO C17 library providing finite-sample medians and exact median confidence
intervals. Arrays are borrowed and not modified; callers retain ownership.
Status enums distinguish invalid arguments, empty/nonfinite samples, configured
size limits, and allocation failure. Optional allocator callbacks let callers
inject and test the allocation-failure path.

The interval uses exact integer arithmetic and accepts up to 10,000 samples.
Build and test with `sh tools/run.sh` or `tools/run.ps1`. CMake installs the
public header, static library, and `coding_adventures::rank_statistics` target.
The POSIX test harness links `-lm` after its source inputs for the test's
`nextafter` boundary cases; the installed library itself does not add that
test-only link dependency.

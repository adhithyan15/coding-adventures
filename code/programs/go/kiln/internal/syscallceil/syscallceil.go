// Package syscallceil holds ONE number: the highest x86-64 system call number
// the sandbox's seccomp filter lets reach the kernel.
//
// It is a leaf package (it imports nothing) because two packages that must not
// import each other both need it:
//
//   - internal/sandbox builds the seccomp filter, and answers ENOSYS to every
//     number above it;
//   - internal/trace decides, from the filter's promise, that a dying call with
//     a number above it (KILN04 D47) can have done nothing.
//
// If each kept its own copy, bumping one without the other would silently make
// the trace accept calls the filter lets through.  One constant, and a test in
// internal/sandbox that pins the filter's behaviour to it, removes that hazard.
package syscallceil

// Max is the highest system call number the pinned strace (6.8, Linux 6.8's
// table) and the parser's table know: lsm_list_modules, 461.  The seccomp
// filter answers ENOSYS to every number ABOVE it (KILN04 section 4.7, L7).
//
// WHEN THE PINNED STRACE IS BUMPED, raise this to the highest number its table
// knows, after adding the new path-taking calls to the parser's table
// (KILN04 section 14).
const Max uint32 = 461

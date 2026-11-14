package sandbox

import (
	"fmt"
	"strings"
)

// The strace invocation of KILN04 section 4.3, in one place.
//
// The recorder runs `strace <StraceArgs> kiln sandbox-exec ...` and parses the
// trace with internal/trace.  The two must agree about which system calls are
// printed: a call strace does not print can never be refused by the parser, and
// a call the parser does not know is "unknown-syscall".  So the argument list
// is exported code, not a string copied into a test, and the tests of this
// package check it against the parser's closed table and the seccomp filter's
// refused list.

// straceClasses are strace's own call classes: every call that takes a
// pathname, every process-management call, every network call.
var straceClasses = []string{"%file", "%process", "%network"}

// straceNames are the calls named explicitly, in addition to the classes.
//
//   - getdents, getdents64 and fchdir take a descriptor, not a path, so no class
//     above holds them, yet they list and change directories (the parser needs
//     the -y annotation of their descriptor);
//   - the metadata writes through a descriptor (fchmod, fchown, fsetxattr,
//     fremovexattr) and the watch calls are likewise not path calls;
//   - then EVERY call the sandbox's filter refuses, and clone3.  A step that
//     tries one must leave a line in the trace for the parser to refuse
//     ("refused:<name>"), whatever class strace happens to put it in, so none is
//     left to the classes.  The first twenty are KILN04 section 4.4's; the
//     ten after them were added by the stage 3b review and are what an older
//     version of this list left out.
var straceNames = []string{
	"getdents", "getdents64", "fchdir",
	"io_uring_setup", "open_by_handle_at", "bpf", "setns", "unshare",
	"fsopen", "fsmount", "fsconfig", "fspick", "open_tree", "move_mount", "mount_setattr",
	"ptrace", "process_vm_readv", "pidfd_getfd",
	"inotify_add_watch", "fanotify_mark",
	"fchmod", "fchown", "fsetxattr", "fremovexattr",
	// The rest of the refused calls: from the classes (%file: name_to_handle_at,
	// mount, umount2, pivot_root, chroot; %process: clone3), listed so the set
	// names every one.
	"name_to_handle_at", "mount", "umount2", "pivot_root", "chroot", "clone3",
	// The ten the stage 3b review added to the filter.
	"io_uring_enter", "io_uring_register", "process_vm_writev", "userfaultfd",
	"kexec_load", "kexec_file_load", "perf_event_open", "add_key", "request_key", "keyctl",
}

// TraceSet is the value of strace's `-e trace=`: the classes, then the names.
func TraceSet() string {
	return strings.Join(append(append([]string{}, straceClasses...), straceNames...), ",")
}

// TraceNames returns the explicitly named calls of the trace set (a copy).
func TraceNames() []string { return append([]string{}, straceNames...) }

// StraceArgs returns the arguments of the strace that wraps `kiln
// sandbox-exec`, up to and including `-o <traceFile> --`; the caller appends the
// command to trace.
//
//   - -f follows every fork, vfork, clone and clone3;
//   - --quiet=attach hides attach and detach chatter but keeps the exit lines
//     the parser needs (-qq would remove them);
//   - -y annotates every descriptor with the kernel's resolved path;
//   - -s 4096 keeps long paths whole (a truncated path makes the trace
//     incomplete);
//   - --kill-on-exit kills the tracees if strace dies, instead of letting them
//     run on untraced;
//   - --pidns-translation prints a spawn's pid inside the sandbox's PID
//     namespace next to its host pid, which the parser requires;
//   - -e signal=all prints every signal line.  This is NOT cosmetic (KILN04
//     D19): strace prints "+++ killed by SIG +++" only for signals in the -e
//     signal set (print_signalled), so with signal=none ANY process killed by
//     a signal (SIGPIPE in `yes | head`, a harness killing its child) has no
//     exit line and the run is voided as missing-exit.  The "--- SIG ---"
//     delivery lines this adds are ignored by the parser;
//   - -o writes ONE file for every process, so events stay in the order the
//     kernel produced them.
//
// traceFile must be an absolute, clean path: it is the only variable argument,
// and "--" ends strace's own options so the command that follows cannot be read
// as one.
func StraceArgs(traceFile string) ([]string, error) {
	if err := checkPath("trace file", traceFile); err != nil {
		return nil, fmt.Errorf("strace: %v", err)
	}
	return []string{
		"-f", "--quiet=attach", "-y", "-s", "4096", "--kill-on-exit", "--pidns-translation",
		"-e", "signal=all",
		"-e", "trace=" + TraceSet(),
		"-o", traceFile,
		"--",
	}, nil
}

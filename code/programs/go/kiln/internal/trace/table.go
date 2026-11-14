package trace

import "sort"

// The closed table of KILN04 §4.4.
//
// strace's `-e trace=` filter decides what is printed; this table decides what
// each printed name MEANS.  Every name is in exactly one of three classes:
//
//   - handled: an observation or a write, with a Kind and the positions of its
//     path-typed arguments;
//   - ignored: carries no path (wait4, close, dup, ...);
//   - refused: reads files without a path the tracer sees, or changes what
//     paths mean (mount, chroot, io_uring_setup, ...).  Seeing one makes the
//     trace incomplete.
//
// A name in none of them is "unknown-syscall:<name>".  The strace version is
// pinned, so a call newly added to %file shows up here as unknown instead of
// being silently skipped: the table is a whitelist, not a blacklist.

// Kind is the table class of a handled system call.
type Kind string

// The handled kinds, matching the first column of the §4.4 table.
const (
	KindOpen      Kind = "open"
	KindMetadata  Kind = "metadata"
	KindListing   Kind = "listing"
	KindDirectory Kind = "directory"
	KindWrite     Kind = "write"
	KindProcess   Kind = "process"
	KindNetwork   Kind = "network"
)

type class int

const (
	classUnknown class = iota
	classHandled
	classIgnored
	classRefused
)

// role selects the special handling of a handled call.
type role int

const (
	rolePath     role = iota // generic: dirfd/path/path2 positions below
	roleCreat                // creat(path, mode): flags are implied
	roleFdPath               // fchmod(fd, ...): the object is the descriptor's -y path
	roleListing              // getdents(fd, ...): path is the fd annotation
	roleFchdir               // fchdir(fd): cwd becomes the fd annotation
	roleChdir                // chdir(path)
	roleSymlink              // symlink(target, linkpath)
	roleExec                 // execve / execveat
	roleFork                 // fork / vfork
	roleClone                // clone
	roleSocket               // socket(domain, type, protocol)
	roleSockaddr             // connect, bind, sendto: one sockaddr argument
	roleMsg                  // sendmsg, sendmmsg: sockaddrs nested in a struct
	roleListen               // listen(fd, backlog)
)

// entry describes one system call.  Argument positions are 1-based; 0 means
// "none".  min/max bound the number of printed arguments (a call with another
// count is not the call we think it is).
type entry struct {
	class      class
	kind       Kind
	role       role
	min, max   int
	dirfd, pth int
	dirfd2     int
	pth2       int
	flags      int
	target     int  // symlink target (data, not a path observation)
	nullPath   bool // utimensat(fd, NULL, ...) acts on the fd itself
	addr       int  // sockaddr argument
}

var table = buildTable()

func buildTable() map[string]entry {
	t := map[string]entry{}
	h := func(name string, e entry) {
		e.class = classHandled
		t[name] = e
	}
	// Open.
	h("open", entry{kind: KindOpen, min: 2, max: 3, pth: 1, flags: 2})
	h("openat", entry{kind: KindOpen, min: 3, max: 4, dirfd: 1, pth: 2, flags: 3})
	h("openat2", entry{kind: KindOpen, min: 4, max: 4, dirfd: 1, pth: 2, flags: 3})
	h("creat", entry{kind: KindOpen, role: roleCreat, min: 2, max: 2, pth: 1})

	// Metadata: an observation of the path.
	h("stat", entry{kind: KindMetadata, min: 2, max: 2, pth: 1})
	h("lstat", entry{kind: KindMetadata, min: 2, max: 2, pth: 1})
	h("newfstatat", entry{kind: KindMetadata, min: 4, max: 4, dirfd: 1, pth: 2, flags: 4})
	h("statx", entry{kind: KindMetadata, min: 5, max: 5, dirfd: 1, pth: 2, flags: 3})
	h("access", entry{kind: KindMetadata, min: 2, max: 2, pth: 1, flags: 2})
	h("faccessat", entry{kind: KindMetadata, min: 3, max: 3, dirfd: 1, pth: 2, flags: 3})
	h("faccessat2", entry{kind: KindMetadata, min: 4, max: 4, dirfd: 1, pth: 2, flags: 4})
	h("readlink", entry{kind: KindMetadata, min: 3, max: 3, pth: 1})
	h("readlinkat", entry{kind: KindMetadata, min: 4, max: 4, dirfd: 1, pth: 2})
	h("getxattr", entry{kind: KindMetadata, min: 4, max: 4, pth: 1})
	h("lgetxattr", entry{kind: KindMetadata, min: 4, max: 4, pth: 1})
	h("listxattr", entry{kind: KindMetadata, min: 3, max: 3, pth: 1})
	h("llistxattr", entry{kind: KindMetadata, min: 3, max: 3, pth: 1})
	h("statfs", entry{kind: KindMetadata, min: 2, max: 2, pth: 1})
	h("inotify_add_watch", entry{kind: KindMetadata, min: 3, max: 3, pth: 2, flags: 3})
	h("fanotify_mark", entry{kind: KindMetadata, min: 5, max: 5, dirfd: 4, pth: 5, flags: 2})

	// Listing: getdents(fd) lists the directory the descriptor names.
	h("getdents", entry{kind: KindListing, role: roleListing, min: 3, max: 3})
	h("getdents64", entry{kind: KindListing, role: roleListing, min: 3, max: 3})

	// Directory: changes the working directory.
	h("chdir", entry{kind: KindDirectory, role: roleChdir, min: 1, max: 1, pth: 1})
	h("fchdir", entry{kind: KindDirectory, role: roleFchdir, min: 1, max: 1})

	// Write.  rename/link: the source is an observation as well as a write
	// (decided in stage 1b); the event carries both paths.
	h("mkdir", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("mkdirat", entry{kind: KindWrite, min: 3, max: 3, dirfd: 1, pth: 2})
	h("rmdir", entry{kind: KindWrite, min: 1, max: 1, pth: 1})
	h("unlink", entry{kind: KindWrite, min: 1, max: 1, pth: 1})
	h("unlinkat", entry{kind: KindWrite, min: 3, max: 3, dirfd: 1, pth: 2, flags: 3})
	h("rename", entry{kind: KindWrite, min: 2, max: 2, pth: 1, pth2: 2})
	h("renameat", entry{kind: KindWrite, min: 4, max: 4, dirfd: 1, pth: 2, dirfd2: 3, pth2: 4})
	h("renameat2", entry{kind: KindWrite, min: 5, max: 5, dirfd: 1, pth: 2, dirfd2: 3, pth2: 4, flags: 5})
	h("link", entry{kind: KindWrite, min: 2, max: 2, pth: 1, pth2: 2})
	h("linkat", entry{kind: KindWrite, min: 5, max: 5, dirfd: 1, pth: 2, dirfd2: 3, pth2: 4, flags: 5})
	h("symlink", entry{kind: KindWrite, role: roleSymlink, min: 2, max: 2, target: 1, pth: 2})
	h("symlinkat", entry{kind: KindWrite, role: roleSymlink, min: 3, max: 3, target: 1, dirfd: 2, pth: 3})
	h("chmod", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("fchmodat", entry{kind: KindWrite, min: 3, max: 4, dirfd: 1, pth: 2})
	h("fchmodat2", entry{kind: KindWrite, min: 4, max: 4, dirfd: 1, pth: 2, flags: 4})
	h("chown", entry{kind: KindWrite, min: 3, max: 3, pth: 1})
	h("lchown", entry{kind: KindWrite, min: 3, max: 3, pth: 1})
	h("fchownat", entry{kind: KindWrite, min: 5, max: 5, dirfd: 1, pth: 2, flags: 5})
	h("truncate", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("utime", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("utimes", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("utimensat", entry{kind: KindWrite, min: 4, max: 4, dirfd: 1, pth: 2, flags: 4, nullPath: true})
	h("futimesat", entry{kind: KindWrite, min: 3, max: 3, dirfd: 1, pth: 2})
	h("setxattr", entry{kind: KindWrite, min: 5, max: 5, pth: 1})
	h("lsetxattr", entry{kind: KindWrite, min: 5, max: 5, pth: 1})
	h("removexattr", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	h("lremovexattr", entry{kind: KindWrite, min: 2, max: 2, pth: 1})
	// Metadata writes through a descriptor: the object is the -y path.
	h("fchmod", entry{kind: KindWrite, role: roleFdPath, min: 2, max: 2})
	h("fchown", entry{kind: KindWrite, role: roleFdPath, min: 3, max: 3})
	h("fsetxattr", entry{kind: KindWrite, role: roleFdPath, min: 5, max: 5})
	h("fremovexattr", entry{kind: KindWrite, role: roleFdPath, min: 2, max: 2})
	h("mknod", entry{kind: KindWrite, min: 3, max: 3, pth: 1})
	h("mknodat", entry{kind: KindWrite, min: 4, max: 4, dirfd: 1, pth: 2})

	// Process.
	h("execve", entry{kind: KindProcess, role: roleExec, min: 3, max: 3, pth: 1})
	h("execveat", entry{kind: KindProcess, role: roleExec, min: 5, max: 5, dirfd: 1, pth: 2, flags: 5})
	h("fork", entry{kind: KindProcess, role: roleFork, min: 0, max: 0})
	h("vfork", entry{kind: KindProcess, role: roleFork, min: 0, max: 0})
	h("clone", entry{kind: KindProcess, role: roleClone, min: 1, max: 8})

	// Network (§5.4).
	h("socket", entry{kind: KindNetwork, role: roleSocket, min: 3, max: 3, flags: 1})
	h("connect", entry{kind: KindNetwork, role: roleSockaddr, min: 3, max: 3, addr: 2})
	h("bind", entry{kind: KindNetwork, role: roleSockaddr, min: 3, max: 3, addr: 2})
	h("listen", entry{kind: KindNetwork, role: roleListen, min: 2, max: 2})
	h("sendto", entry{kind: KindNetwork, role: roleSockaddr, min: 6, max: 6, addr: 5})
	h("sendmsg", entry{kind: KindNetwork, role: roleMsg, min: 3, max: 3, addr: 2})
	h("sendmmsg", entry{kind: KindNetwork, role: roleMsg, min: 4, max: 4, addr: 2})

	// Ignored: no path.  This is the enumeration of the "rest of %process and
	// %network" that §4.4 defers to the implementation.
	for _, n := range []string{
		"wait4", "waitid", "waitpid", "exit", "exit_group", "kill", "tgkill", "tkill",
		"arch_prctl", "prctl", "pidfd_open", "pidfd_send_signal",
		"getsockname", "getpeername", "recvfrom", "recvmsg", "recvmmsg", "accept", "accept4",
		"shutdown", "setsockopt", "getsockopt", "socketpair",
		"getcwd", "pipe", "pipe2", "close", "close_range", "dup", "dup2", "dup3",
	} {
		t[n] = entry{class: classIgnored}
	}

	// Refused: they read files without a path we see, or change what paths mean.
	for _, n := range []string{
		"io_uring_setup", "open_by_handle_at", "name_to_handle_at", "mount", "umount2",
		"fsopen", "fsmount", "fsconfig", "fspick", "open_tree", "move_mount", "mount_setattr",
		"pivot_root", "chroot", "unshare", "setns", "ptrace", "process_vm_readv",
		"pidfd_getfd", "bpf",
		// The sandbox filter also refuses these (KILN04 section 4.7).
		"io_uring_enter", "io_uring_register", "process_vm_writev", "userfaultfd",
		"kexec_load", "kexec_file_load", "perf_event_open", "add_key", "request_key", "keyctl",
		// clone3's flags are not visible to the seccomp filter, so any clone3
		// that reaches a trace is refused (§4.4).
		"clone3",
	} {
		t[n] = entry{class: classRefused}
	}
	return t
}

func lookup(name string) entry { return table[name] }

// SyscallClass names the class of a system call in the closed table: "handled",
// "ignored" or "refused", or "" for a name the parser does not know (it would
// report such a name as "unknown-syscall").  It lets the recorder's strace
// argument list (package sandbox) be checked against this table.
func SyscallClass(name string) string {
	switch table[name].class {
	case classHandled:
		return "handled"
	case classIgnored:
		return "ignored"
	case classRefused:
		return "refused"
	}
	return ""
}

// HandledNames lists the handled class, sorted.
func HandledNames() []string {
	var out []string
	for n, e := range table {
		if e.class == classHandled {
			out = append(out, n)
		}
	}
	sort.Strings(out)
	return out
}

// RefusedNames lists the refused class, sorted.
func RefusedNames() []string {
	var out []string
	for n, e := range table {
		if e.class == classRefused {
			out = append(out, n)
		}
	}
	sort.Strings(out)
	return out
}

// classifyName returns the error for an unknown or refused name.
func classifyName(name string, line int) *Error {
	switch lookup(name).class {
	case classUnknown:
		return incomplete(line, "unknown-syscall:"+name)
	case classRefused:
		return incomplete(line, "refused:"+name)
	}
	return nil
}

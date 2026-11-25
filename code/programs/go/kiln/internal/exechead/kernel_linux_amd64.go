//go:build linux && amd64

package exechead

// The kernel's side of the supervisor: raw system calls and the structures they
// take, in one place so supervisor_linux_amd64.go reads as logic.
//
// Everything here uses package syscall: Kiln depends on in-repo modules only
// (TestNoThirdPartyDependencies), so golang.org/x/sys is not available, and the
// calls this file needs (openat2, pidfd_open, process_vm_readv, the seccomp
// ioctls) are plain syscall numbers.  No uintptr is ever turned back into an
// unsafe.Pointer; a remote address (the step's, in ITS address space) is carried
// as a uint64 inside a struct the kernel reads, never as a Go pointer.

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"strconv"
	"syscall"
	"unsafe"
)

// System call numbers of x86_64 that package syscall does not name.
const (
	sysOpenat2         = 437
	sysPidfdOpen       = 434
	sysPidfdSendSignal = 424
	sysProcessVMReadv  = 310 // syscall.SYS_PROCESS_VM_READV
)

// open(2) and openat2(2) flags and resolve flags.
const (
	oPath               = 0o10000000 // O_PATH: a handle on the name, no read, no write
	resolveNoMagiclinks = 0x02
	resolveBeneath      = 0x08
	resolveInRoot       = 0x10
	atFdcwd             = -100
	atEmptyPath         = 0x1000
	atSymlinkNofollow   = 0x100
)

// seccomp user notification (include/uapi/linux/seccomp.h).
const (
	secNotifFlagContinue = 1
	ioctlTypeSeccomp     = '!'
)

// ioc builds an ioctl request number: direction (1 write, 2 read, 3 both),
// argument size and command number, in the type letter '!' seccomp uses.
func ioc(dir, size, nr uintptr) uintptr { return dir<<30 | size<<16 | ioctlTypeSeccomp<<8 | nr }

type seccompData struct {
	Nr   int32
	Arch uint32
	IP   uint64
	Args [6]uint64
}

type seccompNotif struct {
	ID    uint64
	Pid   uint32
	Flags uint32
	Data  seccompData
}

type seccompNotifResp struct {
	ID    uint64
	Val   int64
	Error int32
	Flags uint32
}

var (
	ioctlNotifRecv = ioc(3, unsafe.Sizeof(seccompNotif{}), 0)
	ioctlNotifSend = ioc(3, unsafe.Sizeof(seccompNotifResp{}), 1)
	ioctlIDValid   = ioc(1, 8, 2)
)

// notifRecv waits for the next notification on the listener.  The structure
// must be zero: the kernel refuses a RECV into a dirty one.
func notifRecv(lfd int, n *seccompNotif) error {
	*n = seccompNotif{}
	for {
		_, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(lfd), ioctlNotifRecv, uintptr(unsafe.Pointer(n)))
		if e == syscall.EINTR {
			continue
		}
		if e != 0 {
			return e
		}
		return nil
	}
}

// notifContinue answers the notification with SECCOMP_USER_NOTIF_FLAG_CONTINUE:
// the kernel runs the call as if no filter had asked.  CONTINUE is explicitly
// not a security decision (the kernel re-reads the path from the step), which is
// why nothing about the pin gates the exec and the confirm is the decision.
func notifContinue(lfd int, id uint64) error {
	resp := seccompNotifResp{ID: id, Flags: secNotifFlagContinue}
	for {
		_, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(lfd), ioctlNotifSend, uintptr(unsafe.Pointer(&resp)))
		if e == syscall.EINTR {
			continue
		}
		if e != 0 {
			return e
		}
		return nil
	}
}

// notifValid says the notification is still waiting for an answer: its thread
// has not been killed or interrupted since the RECV.  ENOENT is the kernel's
// "no".  Any other error is returned.
func notifValid(lfd int, id uint64) (bool, error) {
	for {
		_, _, e := syscall.Syscall(syscall.SYS_IOCTL, uintptr(lfd), ioctlIDValid, uintptr(unsafe.Pointer(&id)))
		switch e {
		case 0:
			return true, nil
		case syscall.ENOENT:
			return false, nil
		case syscall.EINTR:
			continue
		}
		return false, e
	}
}

type pollfd struct {
	fd      int32
	events  int16
	revents int16
}

const (
	pollIn   = 0x1
	pollErr  = 0x8
	pollHup  = 0x10
	pollNval = 0x20
)

// pollOne polls one descriptor for POLLIN for at most ms milliseconds and
// returns the revents.
func pollOne(fd int, ms int) (int16, error) {
	p := pollfd{fd: int32(fd), events: pollIn}
	for {
		_, _, e := syscall.Syscall(syscall.SYS_POLL, uintptr(unsafe.Pointer(&p)), 1, uintptr(ms))
		if e == syscall.EINTR {
			continue
		}
		if e != 0 {
			return 0, e
		}
		return p.revents, nil
	}
}

// pidfdOpen returns a pidfd for the thread-group leader pid.  The descriptor is
// close-on-exec (the kernel makes every pidfd so).
func pidfdOpen(pid int) (int, error) {
	fd, _, e := syscall.Syscall(sysPidfdOpen, uintptr(pid), 0, 0)
	if e != 0 {
		return -1, e
	}
	return int(fd), nil
}

// pidfdExited says the process has exited: its pidfd is readable.  An error
// polling counts as "not exited" (the caller keeps its own deadline).
func pidfdExited(fd int) bool {
	rev, err := pollOne(fd, 0)
	return err == nil && rev&(pollIn|pollHup) != 0
}

// pidfdKill sends SIGKILL through the pidfd (never through a pid number, which
// could have been reused).
func pidfdKill(fd int) error {
	_, _, e := syscall.Syscall6(sysPidfdSendSignal, uintptr(fd), uintptr(syscall.SIGKILL), 0, 0, 0, 0)
	if e != 0 {
		return e
	}
	return nil
}

// openHow is struct open_how of openat2(2).
type openHow struct {
	flags, mode, resolve uint64
}

// openat2 opens path relative to dirfd with the given flags and RESOLVE_ flags.
func openat2(dirfd int, path string, flags, resolve uint64) (int, error) {
	p, err := syscall.BytePtrFromString(path)
	if err != nil {
		return -1, syscall.EINVAL
	}
	how := openHow{flags: flags, resolve: resolve}
	for {
		fd, _, e := syscall.Syscall6(sysOpenat2, uintptr(dirfd), uintptr(unsafe.Pointer(p)), uintptr(unsafe.Pointer(&how)), unsafe.Sizeof(how), 0, 0)
		if e == syscall.EINTR {
			continue
		}
		if e != 0 {
			return -1, e
		}
		return int(fd), nil
	}
}

// iovec is struct iovec with the address as a plain integer, so a pointer into
// ANOTHER process's memory is never a Go pointer.
type iovec struct {
	base, length uint64
}

// vmRead reads len(buf) bytes at addr of process pid's memory
// (process_vm_readv).  It returns how many bytes arrived; fewer than asked is a
// partial read, which the callers treat as a failure.
func vmRead(pid int, addr uint64, buf []byte) (int, error) {
	if len(buf) == 0 {
		return 0, nil
	}
	// The local side holds a real Go pointer (syscall.Iovec), so the compiler
	// knows buf is referenced from the structure the kernel reads and keeps it
	// where the garbage collector and the stack cannot move it.
	local := syscall.Iovec{Base: &buf[0], Len: uint64(len(buf))}
	remote := iovec{base: addr, length: uint64(len(buf))}
	for {
		n, _, e := syscall.Syscall6(sysProcessVMReadv, uintptr(pid), uintptr(unsafe.Pointer(&local)), 1, uintptr(unsafe.Pointer(&remote)), 1, 0)
		if e == syscall.EINTR {
			continue
		}
		if e != 0 {
			return 0, e
		}
		return int(n), nil
	}
}

// pathReadResult says how a string read from the step ended.
type pathReadResult int

const (
	pathOK         pathReadResult = iota
	pathUnreadable                // a partial read or an error: EFAULT, a vanished process
	pathTooLong                   // no NUL within PathMax bytes
)

const pageSize = 4096

// readCString reads a NUL-terminated string at addr of process pid's memory, at
// most PathMax bytes including the NUL.
//
// The read goes a page at a time and each chunk must arrive whole.  That is how
// a string that ends just before an unmapped page is read correctly (a single
// PathMax-byte read of it would be partial, and a partial read is refused), while
// a string that runs into an unmapped page without a NUL really is unreadable.
func readCString(pid int, addr uint64) (string, pathReadResult) {
	var out []byte
	buf := make([]byte, pageSize)
	for len(out) < PathMax {
		chunk := int(pageSize - addr%pageSize)
		if rest := PathMax - len(out); chunk > rest {
			chunk = rest
		}
		n, err := vmRead(pid, addr, buf[:chunk])
		if err != nil || n != chunk {
			return "", pathUnreadable
		}
		if i := bytes.IndexByte(buf[:n], 0); i >= 0 {
			return string(append(out, buf[:i]...)), pathOK
		}
		out = append(out, buf[:n]...)
		addr += uint64(n)
	}
	return "", pathTooLong
}

// readTgid reads the thread-group id of thread tid from /proc/<tid>/status.
func readTgid(tid int) (int, error) {
	data, err := os.ReadFile("/proc/" + strconv.Itoa(tid) + "/status")
	if err != nil {
		return 0, err
	}
	for _, line := range bytes.Split(data, []byte{'\n'}) {
		if rest, ok := bytes.CutPrefix(line, []byte("Tgid:")); ok {
			n, err := strconv.Atoi(string(bytes.TrimSpace(rest)))
			if err != nil || n <= 0 {
				return 0, fmt.Errorf("Tgid %q", rest)
			}
			return n, nil
		}
	}
	return 0, errors.New("no Tgid line")
}

// ident is the identity of a file: device and inode.
type ident struct{ dev, ino uint64 }

// statExe is the identity of what /proc/<tgid>/exe points at.
func statExe(tgid int) (ident, error) {
	var st syscall.Stat_t
	if err := syscall.Stat("/proc/"+strconv.Itoa(tgid)+"/exe", &st); err != nil {
		return ident{}, err
	}
	return ident{dev: uint64(st.Dev), ino: st.Ino}, nil
}

// syscallState is what /proc/<tid>/syscall says.
type syscallState int

const (
	// syscallUnknown: running, or unreadable.  NOT DECIDED: a thread inside
	// execve is "running" too, so this is never read as "outside execve".
	syscallUnknown syscallState = iota
	// syscallInExec: blocked in execve or execveat.
	syscallInExec
	// syscallOutsideExec: DEFINITELY not in execve: blocked in another call, or
	// blocked and not in a system call at all ("-1").
	syscallOutsideExec
)

// readSyscallState reads /proc/<tid>/syscall.  The file is "running", or "-1"
// then the stack pointer and program counter, or the call number then six
// arguments, sp and pc.
func readSyscallState(tid int) syscallState {
	data, err := os.ReadFile("/proc/" + strconv.Itoa(tid) + "/syscall")
	if err != nil {
		return syscallUnknown
	}
	fields := bytes.Fields(data)
	if len(fields) == 0 || string(fields[0]) == "running" {
		return syscallUnknown
	}
	nr, err := strconv.Atoi(string(fields[0]))
	if err != nil {
		return syscallUnknown
	}
	if nr == syscall.SYS_EXECVE || nr == 322 {
		return syscallInExec
	}
	return syscallOutsideExec
}

// openPath opens a name as an O_PATH directory handle (a base for openat2).
func openPathDir(path string) (int, error) {
	for {
		fd, err := syscall.Open(path, oPath|syscall.O_DIRECTORY|syscall.O_CLOEXEC, 0)
		if err == syscall.EINTR {
			continue
		}
		return fd, err
	}
}

// readLimited reads up to len(buf) bytes at offset 0 of fd with pread, looping
// over short reads, and returns the bytes it got (fewer at end of file).
func readLimited(fd int, buf []byte) ([]byte, error) {
	got := 0
	for got < len(buf) {
		n, err := syscall.Pread(fd, buf[got:], int64(got))
		if err == syscall.EINTR {
			continue
		}
		if err != nil {
			return nil, err
		}
		if n == 0 {
			break
		}
		got += n
	}
	return buf[:got], nil
}

// dupCloexec duplicates fd with F_DUPFD_CLOEXEC, or returns -1.
func dupCloexec(fd int) int {
	n, _, e := syscall.Syscall(syscall.SYS_FCNTL, uintptr(fd), 1030 /* F_DUPFD_CLOEXEC */, 3)
	if e != 0 {
		return -1
	}
	return int(n)
}

// notifSizes is struct seccomp_notif_sizes.
type notifSizes struct{ notif, resp, data uint16 }

// checkNotifSizes asks the kernel how big its seccomp notification structures
// are (seccomp(SECCOMP_GET_NOTIF_SIZES)) and refuses to run if they are not the
// ones this package hard-codes: the ioctl numbers embed the sizes, and a buffer of
// the wrong size is read wrongly.
func checkNotifSizes() error {
	var z notifSizes
	_, _, e := syscall.Syscall(317 /* seccomp */, 3 /* SECCOMP_GET_NOTIF_SIZES */, 0, uintptr(unsafe.Pointer(&z)))
	if e != 0 {
		return fmt.Errorf("exechead: seccomp(GET_NOTIF_SIZES): %v", e)
	}
	want := notifSizes{notif: uint16(unsafe.Sizeof(seccompNotif{})), resp: uint16(unsafe.Sizeof(seccompNotifResp{})), data: uint16(unsafe.Sizeof(seccompData{}))}
	if z != want {
		return fmt.Errorf("exechead: the kernel's seccomp notification sizes are %+v, this build hard-codes %+v", z, want)
	}
	return nil
}

// lastNSpid is the last entry of the NSpid line of /proc/<tid>/status: the
// thread's id in the innermost pid namespace it is in.  1 means it is that
// namespace's init.  0 when it cannot be read.
func lastNSpid(tid int) int {
	data, err := os.ReadFile("/proc/" + strconv.Itoa(tid) + "/status")
	if err != nil {
		return 0
	}
	for _, line := range bytes.Split(data, []byte{'\n'}) {
		if rest, ok := bytes.CutPrefix(line, []byte("NSpid:")); ok {
			f := bytes.Fields(rest)
			if len(f) == 0 {
				return 0
			}
			n, err := strconv.Atoi(string(f[len(f)-1]))
			if err != nil {
				return 0
			}
			return n
		}
	}
	return 0
}

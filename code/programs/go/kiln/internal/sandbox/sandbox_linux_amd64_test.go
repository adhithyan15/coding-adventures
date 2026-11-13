//go:build linux && amd64

package sandbox

import (
	"strings"
	"syscall"
	"testing"
)

// The filter's numbers are x86-64's; where package syscall names them, it must
// agree.  (These run only on linux/amd64, where the constants exist.)
func TestRefusedNumbersMatchPackageSyscall(t *testing.T) {
	known := map[string]uint32{
		"mount":      syscall.SYS_MOUNT,
		"umount2":    syscall.SYS_UMOUNT2,
		"pivot_root": syscall.SYS_PIVOT_ROOT,
		"chroot":     syscall.SYS_CHROOT,
		"ptrace":     syscall.SYS_PTRACE,
		"unshare":    syscall.SYS_UNSHARE,
		// Every other number package syscall names.  (It stopped adding
		// names around Linux 3.x: setns, bpf, process_vm_*, the handle calls
		// and io_uring_setup are checked against the literals in
		// TestFilterKnownNumbers.)
		"kexec_load":      syscall.SYS_KEXEC_LOAD,
		"perf_event_open": syscall.SYS_PERF_EVENT_OPEN,
		"add_key":         syscall.SYS_ADD_KEY,
		"request_key":     syscall.SYS_REQUEST_KEY,
		"keyctl":          syscall.SYS_KEYCTL,
	}
	checked := 0
	for _, s := range RefusedSyscalls {
		if want, ok := known[s.Name]; ok {
			checked++
			if s.Nr != want {
				t.Errorf("%s: table %d, kernel headers %d", s.Name, s.Nr, want)
			}
		}
	}
	if checked != len(known) {
		t.Errorf("checked %d of %d", checked, len(known))
	}
	if sysClone != syscall.SYS_CLONE {
		t.Error("clone")
	}
	if AuditArchX86_64 != 0xc000003e {
		t.Error("audit arch")
	}
	if !Supported() {
		t.Error("linux/amd64 must be supported")
	}
}

// The mount flags are the kernel's.
func TestMountFlagsMatchPackageSyscall(t *testing.T) {
	for name, pair := range map[string][2]uintptr{
		"MS_RDONLY": {MsRdonly, syscall.MS_RDONLY}, "MS_NOSUID": {MsNosuid, syscall.MS_NOSUID},
		"MS_NODEV": {MsNodev, syscall.MS_NODEV}, "MS_NOEXEC": {MsNoexec, syscall.MS_NOEXEC},
		"MS_BIND": {MsBind, syscall.MS_BIND}, "MS_REC": {MsRec, syscall.MS_REC}, "MS_PRIVATE": {MsPrivate, syscall.MS_PRIVATE},
		"CLONE_NEWNS": {uintptr(CloneNewNS), syscall.CLONE_NEWNS}, "CLONE_NEWUSER": {uintptr(CloneNewUser), syscall.CLONE_NEWUSER},
		"CLONE_NEWPID": {uintptr(CloneNewPID), syscall.CLONE_NEWPID}, "CLONE_NEWNET": {uintptr(CloneNewNet), syscall.CLONE_NEWNET},
		"CLONE_NEWIPC": {uintptr(CloneNewIPC), syscall.CLONE_NEWIPC}, "CLONE_NEWUTS": {uintptr(CloneNewUTS), syscall.CLONE_NEWUTS},
		"CLONE_FS": {cloneFS, syscall.CLONE_FS},
	} {
		if pair[0] != pair[1] {
			t.Errorf("%s: %#x vs %#x", name, pair[0], pair[1])
		}
	}
}

// The liveness check: open while the writer holds its end, an error once it is
// gone (the init then exits 125 instead of running an orphaned step).
func TestCheckPipeAlive(t *testing.T) {
	var p [2]int
	if err := syscall.Pipe2(p[:], syscall.O_CLOEXEC); err != nil {
		t.Fatal(err)
	}
	defer syscall.Close(p[0])
	if err := checkPipeAlive(p[0]); err != nil {
		t.Fatalf("writer alive: %v", err)
	}
	syscall.Close(p[1])
	if err := checkPipeAlive(p[0]); err == nil || !strings.Contains(err.Error(), "gone") {
		t.Fatalf("writer gone: %v", err)
	}
	if err := checkPipeAlive(1 << 20); err == nil {
		t.Fatal("a bad descriptor passed the check")
	}
}

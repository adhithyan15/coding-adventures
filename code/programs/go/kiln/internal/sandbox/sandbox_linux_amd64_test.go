//go:build linux && amd64

package sandbox

import (
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

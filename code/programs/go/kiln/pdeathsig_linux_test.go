//go:build linux

package main

import (
	"runtime"
	"syscall"
	"testing"
	"unsafe"
)

// armParentDeath asks for SIGKILL when the parent dies; PR_GET_PDEATHSIG (2)
// reads it back. The setting is per thread, so both calls are made on one.
func TestArmParentDeathSetsSIGKILL(t *testing.T) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	if err := armParentDeath(); err != nil {
		t.Fatal(err)
	}
	var sig int32
	if _, _, e := syscall.RawSyscall(syscall.SYS_PRCTL, 2, uintptr(unsafe.Pointer(&sig)), 0); e != 0 {
		t.Fatal(e)
	}
	if sig != int32(syscall.SIGKILL) {
		t.Fatalf("parent-death signal %d, want %d", sig, syscall.SIGKILL)
	}
}

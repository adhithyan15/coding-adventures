//go:build windows

package main

import (
	"os/exec"
	"syscall"
	"unsafe"
)

// processTree lets a service subject be stopped together with everything it started.
//
// A service command is run through the shell (`cmd /C go run server.go ...`), so killing
// only cmd.exe leaves the compiled server running, still holding the service log open,
// which also stops the log's directory from being deleted. The service is therefore put
// in a Job Object with JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, and the whole job is
// terminated. If the job cannot be set up the tree falls back to killing the process.
type processTree struct {
	cmd *exec.Cmd
	job syscall.Handle
}

const (
	jobObjectExtendedLimitInformation = 9
	jobObjectLimitKillOnJobClose      = 0x2000
	processSetQuota                   = 0x0100
	processTerminate                  = 0x0001
)

type ioCounters struct {
	ReadOperationCount, WriteOperationCount, OtherOperationCount uint64
	ReadTransferCount, WriteTransferCount, OtherTransferCount    uint64
}

type jobBasicLimit struct {
	PerProcessUserTimeLimit int64
	PerJobUserTimeLimit     int64
	LimitFlags              uint32
	MinimumWorkingSetSize   uintptr
	MaximumWorkingSetSize   uintptr
	ActiveProcessLimit      uint32
	Affinity                uintptr
	PriorityClass           uint32
	SchedulingClass         uint32
}

type jobExtendedLimit struct {
	Basic                 jobBasicLimit
	IoInfo                ioCounters
	ProcessMemoryLimit    uintptr
	JobMemoryLimit        uintptr
	PeakProcessMemoryUsed uintptr
	PeakJobMemoryUsed     uintptr
}

var (
	kernel32               = syscall.NewLazyDLL("kernel32.dll")
	procCreateJobObjectW   = kernel32.NewProc("CreateJobObjectW")
	procSetInformationJob  = kernel32.NewProc("SetInformationJobObject")
	procAssignProcessToJob = kernel32.NewProc("AssignProcessToJobObject")
	procTerminateJobObject = kernel32.NewProc("TerminateJobObject")
)

// configureProcessTree must be called before cmd.Start. Nothing is needed on Windows.
func configureProcessTree(cmd *exec.Cmd) {}

// attachProcessTree must be called right after cmd.Start succeeds. A failure to build
// the job is not fatal: the returned tree then kills just the process, as before.
func attachProcessTree(cmd *exec.Cmd) *processTree {
	tree := &processTree{cmd: cmd}
	job, _, _ := procCreateJobObjectW.Call(0, 0)
	if job == 0 {
		return tree
	}
	info := jobExtendedLimit{Basic: jobBasicLimit{LimitFlags: jobObjectLimitKillOnJobClose}}
	ok, _, _ := procSetInformationJob.Call(job, jobObjectExtendedLimitInformation,
		uintptr(unsafe.Pointer(&info)), unsafe.Sizeof(info))
	if ok == 0 {
		syscall.CloseHandle(syscall.Handle(job))
		return tree
	}
	process, err := syscall.OpenProcess(processSetQuota|processTerminate, false, uint32(cmd.Process.Pid))
	if err != nil {
		syscall.CloseHandle(syscall.Handle(job))
		return tree
	}
	defer syscall.CloseHandle(process)
	if ok, _, _ := procAssignProcessToJob.Call(job, uintptr(process)); ok == 0 {
		syscall.CloseHandle(syscall.Handle(job))
		return tree
	}
	tree.job = syscall.Handle(job)
	return tree
}

// Kill terminates the service and everything in its job.
func (t *processTree) Kill() error {
	if t == nil || t.cmd == nil || t.cmd.Process == nil {
		return nil
	}
	if t.job != 0 {
		if ok, _, _ := procTerminateJobObject.Call(uintptr(t.job), 1); ok != 0 {
			return nil
		}
	}
	return t.cmd.Process.Kill()
}

// Close releases the job handle. Closing it also kills anything still in the job.
func (t *processTree) Close() {
	if t != nil && t.job != 0 {
		syscall.CloseHandle(t.job)
		t.job = 0
	}
}
